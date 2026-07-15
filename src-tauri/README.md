# Rust / Tauri Backend

当前后端实现普通 Koofr 文件管理的首个可测试切片：

- `src/file_ops/`：挂载点 ID、远程路径、远程名称和本地选择路径的校验。
- `src/koofr_api/`：认证、挂载点、目录列表、建夹、复制、移动、删除及内容请求。
- `src/transfer/`：流式上传/下载、进度事件、取消和下载临时文件清理。
- `src/commands.rs`：暴露给 WebView 的窄范围 Tauri 命令。
- `src/crypto/`、`src/vault_core/`：仍为空，Vault 尚未实现。
- `src/credential_manager/`：仍为空，不会把应用密码或令牌持久化到普通配置。

## 会话与安全边界

- `connect_koofr` 将邮箱和 Koofr 应用密码发送到固定的
  `https://app.koofr.net/token`，只保留返回的会话令牌，并在断开或进程退出时清零
  令牌缓冲区。
- 重新连接或断开账户会取消当前传输，并清除尚未使用的本地文件授权。
- 远程路径必须是规范的绝对 Koofr 路径，拒绝 `.`、`..`、空段、NUL、超长名称和
  对根目录的破坏性操作。
- 上传和下载路径只能由 Rust 打开的原生文件对话框授予；前端只拿到一次性、不透明、
  区分读写方向的授权 ID，不能自行指定任意本地路径。上传还会拒绝符号链接。
- 下载不覆盖现有文件；先写入目标目录中的唯一 `.koofr-part-*` 文件，成功同步后再
  原子改名，失败或取消时清理临时文件。
- 发给前端的错误只包含稳定错误码与安全消息，不包含令牌、本地路径、远程路径或
  服务端响应正文。

当前会话只在内存中存在。Windows Credential Manager 持久化、OAuth、公版应用注册
与令牌刷新必须在后续认证里程碑确认后实现。

## Tauri 命令

`connect_koofr`、`disconnect_koofr`、`koofr_session`、`select_upload_file`、
`select_download_location`、`list_mounts`、
`list_files`、`create_folder`、`rename_entry`、`move_entry`、`copy_entry`、
`delete_entry`、`upload_file`、`download_file`、`cancel_transfer`。

传输通过 `koofr://transfer-progress` 事件报告运行、完成、取消或失败状态；事件不包含
本地或远程文件名。对应 TypeScript 封装位于 `src/services/koofr.ts`。

## 检查

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

仓库根目录的 `npm run check` 会连同前端检查一起执行这些命令。

## 协议依据

请求路径和载荷以 Koofr 官方 [Go 客户端](https://github.com/koofr/go-koofrclient)
与 [Java SDK](https://github.com/koofr/java-koofr) 为准；命令和原生文件选择遵循
[Tauri v2 命令文档](https://v2.tauri.app/develop/calling-rust/) 与
[Dialog 插件文档](https://v2.tauri.app/plugin/dialog/)。
