# Rust / Tauri Backend

当前后端实现普通 Koofr 文件管理的首个可测试切片：

- `src/file_ops/`：挂载点 ID、远程路径、远程名称和本地选择路径的校验。
- `src/koofr_api/`：认证、挂载点、目录列表、建夹、复制、移动、删除及内容请求。
- `src/transfer/`、`src/folder_download.rs`：流式上传/下载、单文件 Range 续传、持久化恢复检查点、递归目录清单、进度事件和取消。
- `src/commands.rs`：暴露给 WebView 的窄范围 Tauri 命令。
- `src/crypto/`、`src/vault_core/`：仍为空，Vault 尚未实现。
- `src/credential_manager.rs`：把用户明确选择保存的应用专用密码写入 Windows 凭据管理器；密码不会进入普通配置。
- `src/settings.rs`、`src/metadata_cache.rs`：保存非敏感设置，并为挂载点、目录、最近文件、共享和回收站提供账户隔离的 TTL 缓存。

## 会话与安全边界

- `connect_koofr` 将邮箱和 Koofr 应用密码发送到固定的
  `https://app.koofr.net/token`，只保留返回的会话令牌，并在断开或进程退出时清零
  令牌缓冲区。
- 重新连接或断开账户会取消当前传输，并清除尚未使用的本地文件授权。
- 远程路径必须是规范的绝对 Koofr 路径，拒绝 `.`、`..`、空段、NUL、超长名称和
  对根目录的破坏性操作。
- 上传路径只能由 Rust 打开的原生文件对话框授予。下载父目录可由用户手动填写或通过
  原生文件夹选择器获取；Rust 会验证它是现有的绝对目录且不是符号链接，只在其下使用
  清理后的远端名称创建新目标，并签发一次性、区分文件/文件夹的授权 ID。前端不能指定
  最终文件名或覆盖现有内容。上传同样拒绝符号链接。
- 下载不覆盖现有文件；单文件先写入由传输 ID 确定的 `.koofr-part-*` 文件，并把不含
  凭据的恢复元数据保存到当前 Windows 用户的应用数据目录，并绑定 Koofr 用户 ID（兼容服务缺失该字段时使用邮箱指纹）隔离账户。网络中断、退出或暂停后，
  Rust 会重新核对远端大小、修改时间和可用哈希，再发送 `Range` 请求从已落盘偏移继续；
  服务端忽略 Range 时会安全截断分片并从头下载。文件夹下载仍使用临时目录并在失败或
  取消时清理整个暂存树。Windows 非法名称会安全替换，同级清理后重名会稳定追加序号。
- 发给前端的错误只包含稳定错误码与安全消息，不包含令牌、本地路径、远程路径或
  服务端响应正文。

当前会话令牌仍只在内存中存在。用户勾选“保存密码”后，应用专用密码由 Windows Credential Manager 保护，并在下次启动时仅由 Rust 后端读取以重新认证。OAuth、公版应用注册与令牌刷新必须在后续认证里程碑确认后实现。

文件元数据缓存默认仅保存在内存中。用户可在设置中启用磁盘缓存；该缓存包含普通 Koofr 文件名和远程路径，保存在当前 Windows 用户的应用数据目录中，不包含密码、令牌或文件内容。切换到“不缓存”、清除缓存或更换账户会删除缓存条目。

## Tauri 命令

`connect_koofr`、`restore_saved_login`、`disconnect_koofr`、`koofr_session`、
`get_settings`、`update_settings`、`update_download_settings`、`clear_metadata_cache`、`forget_saved_login`、`select_upload_file`、
`select_download_location`、`select_download_folder`、`select_download_directory`、
`prepare_download_location`、`prepare_download_folder`、`list_mounts`、
`list_files`、`list_recent`、`list_shared`、`list_trash`、`restore_trash`、
`empty_trash`、`create_folder`、`rename_entry`、`move_entry`、`copy_entry`、
`delete_entry`、`upload_file`、`download_file`、`download_folder`、`cancel_transfer`、
`list_resumable_transfers`、`resume_transfer`、`discard_resumable_transfer`。

传输通过 `koofr://transfer-progress` 事件报告运行、暂停、完成、取消或失败状态；事件不包含
本地或远程文件名。对应 TypeScript 封装位于 `src/services/koofr.ts`。

Koofr 官方 Go 客户端公开了 `FilesGetRange`，因此下载可以进行真实的字节级续传。公开
上传协议及 rclone 的 Koofr 后端只有整文件 `FilesPut`，没有分块上传会话或已确认偏移；
应用会持久化中断上传并提供“重新上传”，但不会把整文件重传标记为字节续传。

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
