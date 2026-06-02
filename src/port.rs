use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tokio::net::TcpListener;

/// 尝试绑定 `addr`，若失败就在 `addr.port()+1..addr.port()+1+max_attempts` 范围内找。
/// 返回 (实际 listener, 实际 port)。
pub async fn try_bind(
    start: u16,
    max_attempts: u16,
) -> Result<(TcpListener, u16), String> {
    for offset in 0..max_attempts {
        let port = start.saturating_add(offset);
        let addr: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
        match TcpListener::bind(addr).await {
            Ok(l) => return Ok((l, port)),
            Err(_) => continue,
        }
    }
    Err(format!(
        "failed to bind any port in {start}..{}",
        start.saturating_add(max_attempts)
    ))
}

/// 读 cached port 文件，返回 Some(port) if 文件存在且内容是合法 u16。
pub fn read_cached_port(path: &Path) -> Option<u16> {
    let content = std::fs::read_to_string(path).ok()?;
    content.trim().parse::<u16>().ok()
}

/// 写 port 到文件
pub fn write_port_file(path: &Path, port: u16) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, port.to_string())
}

/// 删 port 文件（graceful shutdown 时）
pub fn clear_port_file(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// 等待 Ctrl-C / SIGTERM，返回后调用 cleanup 闭包
pub async fn wait_for_shutdown<F>(cleanup: F)
where
    F: FnOnce(),
{
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl-C"),
        _ = terminate => tracing::info!("received SIGTERM"),
    }

    // 给 server 0.3s 优雅关闭
    tokio::time::sleep(Duration::from_millis(300)).await;
    cleanup();
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_read_cached_port_returns_none_when_missing() {
        let dir = TempDir::new().unwrap();
        assert_eq!(read_cached_port(&dir.path().join("port")), None);
    }

    #[test]
    fn test_read_cached_port_parses_valid_value() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("port");
        std::fs::write(&path, "7480").unwrap();
        assert_eq!(read_cached_port(&path), Some(7480));
    }

    #[test]
    fn test_read_cached_port_rejects_garbage() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("port");
        std::fs::write(&path, "not a number\n").unwrap();
        assert_eq!(read_cached_port(&path), None);
    }

    #[test]
    fn test_write_and_clear_port_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("port");
        write_port_file(&path, 8123).unwrap();
        assert!(path.exists());
        assert_eq!(read_cached_port(&path), Some(8123));
        clear_port_file(&path);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn test_try_bind_finds_free_port() {
        // 用一个随机高端口，确保不被占用
        let start: u16 = 50000 + (rand_port_offset() as u16) % 10000;
        let (listener, port) = try_bind(start, 10).await.expect("should bind");
        assert!(port >= start && port < start + 10);
        drop(listener);
    }

    #[tokio::test]
    async fn test_try_bind_skips_occupied_port() {
        // 先占一个端口
        let (occupied_listener, occupied_port) = try_bind(51000, 1).await.unwrap();

        // 再试从 51000 开始，应该跳过 occupied_port
        let (listener, port) = try_bind(occupied_port, 10).await.expect("should bind next free");
        assert_ne!(port, occupied_port);
        drop(listener);
        drop(occupied_listener);
    }

    fn rand_port_offset() -> u32 {
        use std::time::SystemTime;
        let nanos = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        nanos % 65535
    }
}
