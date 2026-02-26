# AI-Box 问题修复记录

本文档记录项目开发过程中遇到的问题及解决方案，持续更新。

---

## #1 GitHub Copilot 集成：PAT 无法获取模型列表

**日期**：2026-02-26

**现象**：在设置中配置 GitHub PAT（Personal Access Token）后，无法获取 Copilot 订阅的模型列表（Claude、GPT-4o 等）。

**原因分析**：

- 最初使用 `https://api.github.com/copilot_internal/v2/token` 端点，用 PAT 换取 Copilot API token
- 该端点返回 404 —— 它是 GitHub 内部 API，**仅接受 OAuth token**（VS Code Copilot 扩展登录后生成的），不支持 PAT
- Copilot 订阅模型（Claude Sonnet/Opus、GPT-5 等）通过 `api.githubcopilot.com` 提供，必须使用 Copilot 内部 token 认证

**解决方案**：

实现 GitHub Device OAuth 登录流程，替代手动填写 PAT：

1. **Device Flow 认证**：调用 `https://github.com/login/device/code` 获取设备码，用户在浏览器授权后，轮询 `https://github.com/login/oauth/access_token` 获取 OAuth token
2. **Token 交换**：用 OAuth token 调用 `copilot_internal/v2/token` 换取短期 Copilot API token（自带过期时间，自动缓存刷新）
3. **模型和对话**：用 Copilot API token 访问 `api.githubcopilot.com/models` 和 `api.githubcopilot.com/chat/completions`

**涉及文件**：

| 文件 | 改动 |
|---|---|
| `src-tauri/src/llm/copilot.rs` | Device OAuth flow + token 交换 + Copilot API 调用 |
| `src-tauri/src/commands/settings.rs` | 新增登录/登出/状态检查命令 |
| `src/components/SettingsModal.tsx` | PAT 输入框改为 "Login with GitHub" 按钮 |

**关键点**：

- 使用的 OAuth client_id `Iv1.b507a08c87ecfe98` 是 Copilot IDE 集成的公开 client_id（copilot.vim 等项目使用）
- OAuth token 存储在本地 SQLite 数据库的 `settings` 表中（key: `copilot_oauth_token`）
- Copilot API token 有过期时间，代码中在过期前 120 秒自动刷新

---

## #2 OAuth 轮询响应慢

**日期**：2026-02-26

**现象**：用户在浏览器完成 GitHub 授权后，ai-box 内仍长时间显示 "Waiting for authorization..."。

**原因分析**：

- 使用 `setInterval` + async 回调，当网络请求耗时较长时，多个轮询请求会堆积
- 轮询间隔设为 5 秒，授权后最长需等待 5 秒才能检测到

**解决方案**：

1. 将 `setInterval` 改为递归 `setTimeout`，确保前一次请求完成后才发起下一次
2. 轮询间隔从 5 秒缩短为 3 秒（GitHub API 允许最小间隔通常为 3-5 秒）

**涉及文件**：`src/components/SettingsModal.tsx`
