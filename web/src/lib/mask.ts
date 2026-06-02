/**
 * 敏感信息脱敏工具 — 用于 Aliases 预览页。
 *
 * 关键设计：zsh 必须用明文 export（运行时需要真实 API Key），所以磁盘文件
 * `aliases.zsh` 保持不变。仅在 Web UI 预览层做掩码 + reveal 切换。
 */

/** 匹配 env var 的最后一段以 _KEY/_TOKEN/_SECRET/_PASSWORD/_CREDENTIAL(S) 结尾 */
const SENSITIVE_SUFFIX = /(?:^|_)(KEY|TOKEN|SECRET|PASSWORD|CREDENTIALS?)$/i;

/** 检查环境变量名是否是敏感（key/secret/token/password/credential）。 */
export function isSensitiveKey(name: string): boolean {
  return SENSITIVE_SUFFIX.test(name);
}

/**
 * 把 VALUE 替换为 `前3 + *** + 后4` 的形式。
 * 短字符串（≤ 8）一律 `***`，避免泄露任何字符。
 */
export function maskValue(value: string): string {
  if (value.length <= 8) return '***';
  const head = value.slice(0, 3);
  const tail = value.slice(-4);
  return `${head}***${tail}`;
}
