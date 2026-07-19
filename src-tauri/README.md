# Rust / Tauri 后端

后端目前实现了普通 Koofr 文件管理的基本功能，算是第一个可测试的切片：

- `src/file_ops/`：校验挂载点 ID、远程路径、远程名称和本地选择路径。
- `src/koofr_api/`：认证、挂载点、目录列表、建文件夹、复制、移动、删除、内容请求和基础分享链接。
- `src/link_commands.rs`：下载链接和接收文件链接的查询、创建、撤销。
- `src/transfer/`、`src/folder_download.rs`：流式上传/下载、单文件 Range 续传、通用二进制分卷续传、持久化恢复检查点、递归目录清单、进度事件、取消、下载临时内容清理。
- `src/commands.rs`：暴露给 WebView 的 Tauri 命令，接口范围控制得很窄。
- `src/crypto/`、`src/vault_core/`：还是空的，Vault 没动。
- `src/credential_manager.rs`：用户选了保存密码的话，把应用专用密码写到 Windows 凭据管理器里。密码不会出现在普通配置中。
- `src/settings.rs`、`src/metadata_cache.rs`：保存非敏感设置，为挂载点、目录、最近文件、共享和回收站提供按账户隔离的 TTL 缓存。

## 会话和安全边界

说起来有点绕，但都是踩过的坑，记一下：

- `connect_koofr` 把邮箱和 Koofr 应用密码发到 `https://app.koofr.net/token`，只保留返回的会话令牌。断开连接或进程退出时把令牌缓冲区清零。
- 重新连接或断开账户会取消当前传输，清掉还没用的本地文件授权。
- 远程路径必须是规范的绝对 Koofr 路径。`.`、`..`、空段、NUL、超长名称、对根目录的破坏性操作全部拒绝。
- 上传路径只能由 Rust 打开的原生文件对话框授权。下载父目录用户可以手填，也可以用原生文件夹选择器选。Rust 会验证它是现有的绝对目录而且不是符号链接，只在这个目录下用清理后的远端名称创建新目标，签发一次性、区分文件/文件夹的授权 ID。前端不能指定最终文件名，也不能覆盖已有内容。上传也拒绝符号链接。
- 下载不覆盖现有文件。单文件先写入由传输 ID 确定的 `.koofr-part-*` 临时文件。不含凭据的恢复元数据保存到当前 Windows 用户的应用数据目录，用 Koofr 用户 ID 隔离账户（兼容服务缺失该字段时用邮箱指纹）。网络断了、退出或暂停后，Rust 会重新核对远端大小、修改时间和可用哈希，然后发 `Range` 请求从已落盘偏移继续。如果服务端忽略 Range，就安全截断分片从头下载。文件夹下载还是用临时目录，失败或取消时清理整个暂存树。Windows 非法名称做安全替换，同级清理后重名会稳定追加序号。
- 发给前端的错误只包含稳定错误码和安全消息，不包含令牌、本地路径、远程路径或服务端响应正文。

会话令牌只在内存里待着。用户勾了"保存密码"，应用专用密码由 Windows Credential Manager 保护，下次启动时由 Rust 后端读出来重新认证。OAuth、公版应用注册和令牌刷新等后续认证里程碑确认后再做。

文件元数据缓存默认只存在内存里。用户可以在设置里开磁盘缓存——缓存里只有普通 Koofr 文件名和远程路径，存在当前 Windows 用户的应用数据目录里，不包含密码、令牌或文件内容。切换到"不缓存"、清缓存或者换账户，缓存条目自动删掉。

## Tauri 命令

`connect_koofr`、`restore_saved_login`、`disconnect_koofr`、`koofr_session`、
`get_settings`、`update_settings`、`update_download_settings`、`clear_metadata_cache`、`forget_saved_login`、`select_upload_file`、
`select_download_location`、`select_download_folder`、`select_download_directory`、
`prepare_download_location`、`prepare_download_folder`、`list_mounts`、
`list_files`、`list_recent`、`list_shared`、`list_trash`、`restore_trash`、
`list_public_links`、`create_public_link`、`delete_public_link`、
`empty_trash`、`create_folder`、`rename_entry`、`move_entry`、`copy_entry`、
`delete_entry`、`upload_file`、`upload_split_file`、`download_file`、`download_folder`、`cancel_transfer`、
`list_resumable_transfers`、`resume_transfer`、`discard_resumable_transfer`。

传输通过 `koofr://transfer-progress` 事件上报运行、暂停、完成、取消或失败状态。事件里不包含本地或远程文件名。TypeScript 封装在 `src/services/koofr.ts`。

Koofr 官方 Go 客户端暴露了 `FilesGetRange`，所以下载能做真实的字节级续传。但公开上传协议和 rclone 的 Koofr 后端只有整文件 `FilesPut`，没有针对单个普通文件的分块上传会话或已确认偏移。普通上传会持久化中断任务并提供"重新上传"，不会把整文件重传标成字节续传。用户明确选了"可续传大文件"时，后端建一个独立远端文件夹，把原文件切成自定义大小的 `part-*.bin`，只从最后一个已确认完整的分卷继续。完成后写入通用恢复命令、分卷和整文件 SHA-256 和开放 JSON 清单。分卷可以直接用系统自带的 `copy /b` 或 `cat` 拼接，不需要本客户端。

## 检查

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

根目录的 `npm run check` 会连前端检查一起跑这些命令。

## 参考实现

请求路径和载荷以 Koofr 官方 [Go 客户端](https://github.com/koofr/go-koofrclient) 和 [Java SDK](https://github.com/koofr/java-koofr) 为准。命令和原生文件选择遵循 [Tauri v2 命令文档](https://v2.tauri.app/develop/calling-rust/) 和 [Dialog 插件文档](https://v2.tauri.app/plugin/dialog/)。