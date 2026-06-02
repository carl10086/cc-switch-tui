use axum::{
    body::Body,
    http::{StatusCode, Uri, header},
    response::Response,
};
use include_dir::{Dir, include_dir};

/// 嵌入的 web-dist/ 目录。包含 Vite 编译产物（index.html + assets/）。
pub static WEB_DIST: Dir = include_dir!("$CARGO_MANIFEST_DIR/web-dist");

/// SPA fallback handler：
/// - 如果请求路径在 web-dist/ 中能找到文件（如 /assets/index-xxx.js），返回该文件
/// - 否则返回 web-dist/index.html（让 React Router 处理客户端路由）
pub async fn spa_fallback(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if let Some(file) = WEB_DIST.get_file(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(file.contents().to_vec()))
            .expect("building static response");
    }

    // 任何非 /api 的请求都 fallback 到 index.html（SPA 路由）
    let index = WEB_DIST
        .get_file("index.html")
        .expect("web-dist/index.html must exist (tracked in git)");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(index.contents().to_vec()))
        .expect("building index response")
}
