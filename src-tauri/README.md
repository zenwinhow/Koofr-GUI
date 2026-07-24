# Rust / Tauri 后端

后端目前实现了普通 Koofr 文件管理的基本功能，算是第一个可测试的切片：

- `src/file_ops/`：校验挂载点 ID、远程路径、远程名称和本地选择路径。
- `src/koofr_api/`：认证、挂载点、目录列表、建文件夹、复制、移动、删除、内容请求和基础分享链接。
- `src/link_commands.rs`：下载链接和接收文件链接的查询、创建、撤销。
- `src/transfer/`、`src/folder_download.rs`：流式上传/下载、单文件 Range 续传、通用二进制分卷续传、持久化恢复检查点、递归目录清单、进度事件、取消、下载临时内容清理。
- `src/commands.rs`、`src/vault_commands.rs`：暴露给 WebView 的窄接口；Vault 命令只接收注册 ID 和短期不透明句柄。
- `src/crypto/`、`src/vault_core/`：官方 `koofr/vault` 加密引擎边界、Vault 解锁会话、不透明路径句柄和自动锁定。
- `src/credential_manager.rs`：用户选了保存密码的话，把应用专用密码写到 Windows 凭据管理器里。密码不会出现在普通配置中。
- `src/settings.rs`、`src/metadata_cache.rs`：保存非敏感设置，为挂载点、目录、最近文件、共享和回收站提供按账户隔离的 TTL 缓存。
- `src/work_directory.rs`：保存工作目录定位记录，在启动早期执行可恢复的全量目录迁移。
- `src/logging.rs`：后台写入结构化 JSONL 诊断日志，支持级别过滤、大小轮转、保留期限和清理。

## 会话和安全边界

说起来有点绕，但都是踩过的坑，记一下：

- `connect_koofr` 把邮箱和 Koofr 应用密码发到 `https://app.koofr.net/token`，只保留返回的会话令牌。断开连接或进程退出时把令牌缓冲区清零。
- 重新连接或断开账户会取消当前传输，清掉还没用的本地文件授权。
- 远程路径必须是规范的绝对 Koofr 路径。`.`、`..`、空段、NUL、超长名称、对根目录的破坏性操作全部拒绝。
- 上传路径只能由 Rust 打开的原生文件对话框授权。下载父目录用户可以手填，也可以用原生文件夹选择器选。Rust 会验证它是现有的绝对目录而且不是符号链接，只在这个目录下用清理后的远端名称创建新目标，签发一次性、区分文件/文件夹的授权 ID。前端不能指定最终文件名，也不能覆盖已有内容。上传也拒绝符号链接。
- 下载不覆盖现有文件。单文件先写入由传输 ID 确定的 `.koofr-part-*` 临时文件。不含凭据的恢复元数据保存到当前 Windows 用户的应用数据目录，用 Koofr 用户 ID 隔离账户（兼容服务缺失该字段时用邮箱指纹）。网络断了、退出或暂停后，Rust 会重新核对远端大小、修改时间和可用哈希，然后发 `Range` 请求从已落盘偏移继续。如果服务端忽略 Range，就安全截断分片从头下载。文件夹下载还是用临时目录，失败或取消时清理整个暂存树。Windows 非法名称做安全替换，同级清理后重名会稳定追加序号。
- 发给前端的错误只包含稳定错误码和安全消息，不包含令牌、本地路径、远程路径或服务端响应正文。
- 诊断日志同样不记录令牌、邮箱、文件名或路径；传输失败只记录 transfer ID、错误类别、HTTP 状态或 I/O 类别等脱敏字段。
- Vault Safe Key 只由 Windows 原生凭据窗口交给 Rust，不经过 IPC。前端只见解密显示元数据和 UUID 句柄；密文路径、validator 和 rclone 配置留在后端。默认 60 分钟无操作自动锁定，账户切换和退出会立即清空解锁状态。

会话令牌只在内存里待着。用户勾了"保存密码"，应用专用密码由 Windows Credential Manager 保护，下次启动时由 Rust 后端读出来重新认证。OAuth、公版应用注册和令牌刷新等后续认证里程碑确认后再做。

文件元数据缓存默认只存在内存里。用户可以在设置里开启磁盘缓存，文件固定放在当前工作目录的 `cache/` 中——缓存里只有普通 Koofr 文件名和远程路径，不包含密码、令牌或文件内容。切换到"不缓存"、清缓存或者换账户，缓存条目自动删掉。

工作目录默认是 Tauri 的 `app_local_data_dir`，也可切换到用户选择的空目录。变更先写入默认数据目录旁的 `net.koofr.desktop.gui.work-directory.json` 定位记录，下次启动会在设置、检查点、历史、缓存和日志初始化前完成迁移。全量迁移使用独立暂存目录和迁移标记，成功激活新目录后才清理旧数据；失败时保留可用副本并在后续启动重试。根目录、符号链接、非空目标和互相嵌套的新旧目录都会被拒绝。Windows 凭据管理器中的应用专用密码不属于工作目录。

用户可选开启 `network_error` 自动恢复，并设置有限或无限的重试次数以及固定重试间隔。单文件下载从已落盘偏移继续，分卷上传从已确认分卷继续，普通上传由于 Koofr 只提供整文件上传接口而从头重试。等待期间仍接受暂停和取消，其他错误类别不会自动重试。

## Tauri 命令

`connect_koofr`、`restore_saved_login`、`disconnect_koofr`、`koofr_session`、
`get_settings`、`update_settings`、`update_download_settings`、`update_logging_settings`、`update_transfer_settings`、`update_work_directory`、`clear_metadata_cache`、`clear_logs`、`select_work_directory`、`forget_saved_login`、`select_upload_file`、
`select_download_location`、`select_download_folder`、`select_download_directory`、
`prepare_download_location`、`prepare_download_folder`、`list_mounts`、
`list_files`、`list_recent`、`list_shared`、`list_trash`、`restore_trash`、
`list_public_links`、`create_public_link`、`delete_public_link`、
`empty_trash`、`create_folder`、`rename_entry`、`move_entry`、`copy_entry`、
`delete_entry`、`upload_file`、`upload_split_file`、`download_file`、`download_folder`、`cancel_transfer`、
`list_resumable_transfers`、`resume_transfer`、`discard_resumable_transfer`。

Vault：`list_vaults`、`unlock_vault`、`lock_vault`、`list_vault_files`、
`create_vault_folder`、`rename_vault_entry`、`relocate_vault_entry`、
`delete_vault_entries`、`upload_vault_file`、`download_vault_file`、
`create_vault`、`remove_vault`、`export_vault_rclone_config`、
`import_vault_rclone_config`。

传输通过 `koofr://transfer-progress` 事件上报运行、暂停、完成、取消或失败状态。事件里不包含本地或远程文件名。TypeScript 封装在 `src/services/koofr.ts`。

Koofr 官方 Go 客户端暴露了 `FilesGetRange`，所以下载能做真实的字节级续传。但公开上传协议和 rclone 的 Koofr 后端只有整文件 `FilesPut`，没有针对单个普通文件的分块上传会话或已确认偏移。普通上传会持久化中断任务并提供"重新上传"，不会把整文件重传标成字节续传。用户明确选了"可续传大文件"时，后端建一个独立远端文件夹，把原文件切成自定义大小的 `part-*.bin`，只从最后一个已确认完整的分卷继续。完成后写入通用恢复命令、分卷和整文件 SHA-256 和开放 JSON 清单。分卷可以直接用系统自带的 `copy /b` 或 `cat` 拼接，不需要本客户端。

Vault 文件名使用 AES-EME，内容使用 rclone crypt 的认证分块格式，密钥派生参数与 Koofr Vault / rclone 一致。加密上传因 `FilesPut` 限制只能整文件安全重试；下载先对密文做 HTTP Range 续传，完整逐块认证和解密后才发布最终明文。互操作 golden vector 来自 rclone 与官方 `koofr/vault`。

## 检查

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

根目录的 `npm run check` 会连前端检查一起跑这些命令。

## 参考实现

请求路径和载荷以 Koofr 官方 [Go 客户端](https://github.com/koofr/go-koofrclient)、[Java SDK](https://github.com/koofr/java-koofr) 和 [Koofr Vault](https://github.com/koofr/vault) 为准。命令和原生文件选择遵循 [Tauri v2 命令文档](https://v2.tauri.app/develop/calling-rust/) 和 [Dialog 插件文档](https://v2.tauri.app/plugin/dialog/)。
