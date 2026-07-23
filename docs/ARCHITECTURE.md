# 架构说明

Koofr-GUI 的整体架构、数据流、传输恢复设计和安全边界都在这里。适合想了解整体结构、评估安全性、或者准备贡献代码的人读。

## 目录

- [顶层结构](#顶层结构)
- [进程与线程模型](#进程与线程模型)
- [数据流](#数据流)
- [核心模块](#核心模块)
  - [前端 `src/`](#前端-src)
  - [Rust 后端 `src-tauri/src/`](#rust-后端-src-tauri-src)
- [关键设计](#关键设计)
  - [认证与会话](#认证与会话)
  - [路径与操作校验](#路径与操作校验)
  - [传输管线](#传输管线)
  - [单文件下载：字节级续传](#单文件下载字节级续传)
  - [分卷上传设计](#分卷上传设计)
  - [文件夹递归下载](#文件夹递归下载)
  - [元数据缓存](#元数据缓存)
  - [凭据存储](#凭据存储)
  - [错误模型](#错误模型)
- [事件与命令目录](#事件与命令目录)
- [本地数据布局](#本地数据布局)
- [未实现的部分](#未实现的部分)

## 顶层结构

```
┌──────────────────────────────────────────────────────┐
│  WebView2（Chromium 内核）                            │
│  ┌────────────────────────────────────────────────┐  │
│  │ React 19 + TypeScript                          │  │
│  │  - features/ 按功能划分                        │  │
│  │  - services/ 类型化 Tauri 命令封装             │  │
│  │  - 只调受限命令，不接触密码/令牌               │  │
│  └────────────────────────────────────────────────┘  │
└─────────────────────┬────────────────────────────────┘
                      │ Tauri IPC (typed commands + events)
                      ▼
┌──────────────────────────────────────────────────────┐
│  Rust 主进程 (tokio 多线程运行时)                    │
│  ┌────────────────────────────────────────────────┐  │
│  │ Tauri Builder + AppState                       │  │
│  │  - KoofrApi (reqwest / rustls)                 │  │
│  │  - TransferManager                             │  │
│  │  - TransferCheckpointStore                     │  │
│  │  - SettingsStore                               │  │
│  │  - MetadataCache                               │  │
│  │  - CredentialManager (Windows Credentials)     │  │
│  │  - LocalAccessManager                          │  │
│  └────────────────────────────────────────────────┘  │
└─────────────────────┬────────────────────────────────┘
                      │ HTTPS (rustls, cert 校验)
                      ▼
              ┌───────────────────┐
              │  Koofr REST API   │
              │  app.koofr.net    │
              └───────────────────┘
```

技术栈见 [CLAUDE.md](../CLAUDE.md#技术栈)。

## 进程与线程模型

- **单进程**：一个 `koofr-gui.exe`，包含 Tauri 主进程 + WebView2 子进程（由 WebView2 Runtime 管理）。
- **主线程**：Tauri 事件循环、UI 事件调度。
- **tokio runtime**：`rt-multi-thread`，异步 HTTP、异步磁盘 IO、传输任务、进度事件发射都在这里跑。
- **传输并发**：`TransferManager` 允许多个传输任务并行（每个任务一个 tokio task），共享同一个 `reqwest::Client`（连接池、rustls 会话复用）。

## 数据流

以"用户下载一个文件"为例：

```
用户点击"下载"
  │
  ▼
[前端] FileList → download button
  │  services/koofr.ts::selectDownloadLocation()
  │  services/koofr.ts::prepareDownloadLocation()  ← Rust 校验父目录、签发一次性 auth ID
  │  services/koofr.ts::downloadFile()             ← 传 mountId, remotePath, authId
  │
  ▼
[Rust] commands::download_file
  │  1. 校验 mountId + remotePath（file_ops/）
  │  2. 消费 authId（LocalAccessManager，一次性、区分文件/文件夹）
  │  3. 创建 TransferManager 任务，返回 transferId
  │
  ▼
[Rust tokio task] transfer::download
  │  1. HEAD → 获取 size / etag / modified
  │  2. 打开 .koofr-part-<transferId> 临时文件
  │  3. 写入 TransferCheckpointStore（transfer-checkpoints.json）
  │  4. 边下边写：reqwest stream → tokio::fs::File
  │  5. 每 N 字节 emit "koofr://transfer-progress" 事件（不含文件名）
  │  6. 完成：校验大小、rename 到最终名（同级重名加序号）
  │  7. remove checkpoint
  │
  ▼
[前端] onTransferProgress 监听器 → 更新进度条
```

普通上传、分卷上传、文件夹下载走类似的模式，差别在传输核心逻辑。

## 核心模块

### 前端 `src/`

见 [src/README.md](../src/README.md) 里的目录职责。要点：

- `App.tsx` 是唯一持有全局状态的组件（`authState`、`modalKind`、当前挂载点 / 目录、传输列表等）。React 19 + hooks，没有 Redux / Zustand 之类的外部状态库。
- `features/*/` 每个功能模块只导出 UI 组件和领域 hooks。跨模块通信通过 `App.tsx` 传 props 或事件回调，不允许 feature-to-feature 直接依赖。
- `services/koofr.ts` 和 `services/publicLinks.ts` 是**唯一**允许调 `@tauri-apps/api` 的地方，所有命令都在这里包一层类型安全接口。
- 错误处理：`commandErrorMessage(error, fallback)` 拿安全消息，`isCommandErrorCode(error, code)` 判断错误码。

### Rust 后端 `src-tauri/src/`

```
src-tauri/src/
├── main.rs                     入口，几乎为空
├── lib.rs                      Tauri Builder + AppState 初始化
├── error.rs                    AppError + 稳定错误码 + 安全消息
├── commands.rs                 核心 Tauri 命令注册
├── folder_commands.rs          文件夹下载命令
├── link_commands.rs            分享链接命令
├── split_commands.rs           分卷上传命令
├── transfer_commands.rs        传输列表/恢复/丢弃命令
├── credential_manager.rs       Windows 凭据封装
├── settings.rs                 settings.json 读写
├── metadata_cache.rs           内存 + 磁盘缓存
├── local_access.rs             一次性本地路径授权
├── local_open.rs               打开下载文件/文件夹
├── folder_download.rs          递归文件夹下载
├── file_ops/                   路径规范化 + 操作校验
├── koofr_api/                  Koofr REST 客户端
├── transfer/
│   ├── mod.rs                  TransferManager + 公共类型
│   ├── manager.rs              状态管理
│   ├── model.rs                传输模型
│   ├── download.rs             单文件下载 + Range 续传
│   ├── upload.rs               普通整文件上传
│   ├── split_upload.rs         分卷可续传上传
│   ├── split_package.rs        分卷包元数据、清单、恢复脚本
│   ├── checkpoint.rs           TransferCheckpointStore（持久化）
│   ├── checkpoint_snapshot.rs  从磁盘 checkpoint 重建可恢复项
│   └── part.rs                 .koofr-part-* 分片管理
├── crypto/                     预留（Vault，未实现）
└── vault_core/                 预留（Vault，未实现）
```

## 关键设计

### 认证与会话

- `connect_koofr(email, password, remember)` 把邮箱和 Koofr **应用专用密码**（不是主账户密码）POST 到 `https://app.koofr.net/token`。
- 成功后 Rust 内存保留 access token + refresh token。**永远不发到前端**。
- `remember=true` 时，密码通过 `keyring-core` + `windows-native-keyring-store` 写入 Windows 凭据管理器，作用域是当前用户。
- 下次启动时 `restore_saved_login()` 从凭据管理器读回密码，重新走一次 `/token`。
- `disconnect_koofr()` 主动断开：取消所有传输、清 refresh token 缓冲区（`zeroize`）、清 LocalAccessManager 里未消费的授权。
- 进程退出时同样 zeroize。

### 路径与操作校验

**远程路径**（Koofr 服务器上的路径）：

- 必须以 `/` 开头。
- 拒绝 `.`、`..`、空段、NUL、超长段。
- 对根目录 (`/`) 的删除 / 重命名等破坏性操作直接拒绝。

**本地路径**：

- **上传**：只能通过 `select_upload_file()` 打开的原生文件对话框获得，前端拿到的是不透明句柄，不能自己拼路径。
- **下载父目录**：用户可以手填也可以用 `select_download_folder()`。Rust 会校验：绝对路径、目录存在、不是符号链接。
- **最终下载文件名**：由 Rust 从远端名清理生成（Windows 非法字符替换、同级重名加序号）。**前端无权指定**。
- 上传拒绝符号链接。

**授权模型**：`LocalAccessManager` 签发**一次性、区分文件 / 文件夹**的授权 ID。前端调 `prepare_download_location()` 拿到 ID，只能用一次调 `download_file()` 或 `download_folder()`。用过或者进程重启后失效。

### 传输管线

`TransferManager` 维护一个 `HashMap<TransferId, TransferHandle>`，每个句柄包含：

- 当前状态：`Running | Paused | Completed | Cancelled | Failed`
- 已传输字节数、总字节数
- 取消 signal（`tokio_util::sync::CancellationToken`）
- 关联的 checkpoint（如果可恢复）

进度通过 `koofr://transfer-progress` 事件推送。事件 payload：

```typescript
{
  transferId: string,
  state: 'running' | 'paused' | 'completed' | 'cancelled' | 'failed',
  bytesTransferred: number,
  totalBytes: number,
  errorCode?: string,  // 仅 failed 状态
}
```

**payload 里没有文件名 / 路径 / 令牌**，前端自己维护 transferId → 文件名的映射。

### 单文件下载：字节级续传

`transfer::download` 实现步骤：

1. `HEAD` 请求拿 `Content-Length`、`ETag`、`Last-Modified`。
2. 检查同级是否已有目标文件：有就加序号（`file (1).ext`）。
3. 打开 `.koofr-part-<transferId>` 临时文件（O_CREAT | O_RDWR）。
4. 写入 `DownloadCheckpoint`（含 mountId、remotePath、expected_size、remote_hash、remote_modified、local_path、partial_path）到 `transfer-checkpoints.json`。
5. 用 `reqwest` 打 `GET` 请求，把 body 流写到临时文件。
6. **中断恢复**：应用重启后 `list_resumable_transfers()` 从 checkpoint 重建可恢复项。用户点"继续"时：
   - HEAD 再拉一次，比对 size / etag / modified。
   - 如果匹配：发 `Range: bytes=<offset>-` 请求，追加写。
   - 如果服务端忽略 Range（返回 200 而不是 206）：安全截断临时文件，从头重下。
   - 如果服务端文件变了（etag / modified 不匹配）：标记为 `Failed`，用户需要重开下载。
7. 完成后 rename `.koofr-part-*` → 最终文件名，删除 checkpoint。

支持文件：`transfer/download.rs`、`transfer/part.rs`、`transfer/checkpoint.rs`。

### 分卷上传设计

**为什么要分卷？** Koofr 公开上传 API 只有整文件 `PUT /content/api/v2/mounts/<mountId>/files/put`。没有分块会话、没有服务端已确认偏移。参考 rclone Koofr 后端和 Koofr 官方 Go 客户端确认了这点。所以普通上传一旦中断，除了整个重传别无办法。

分卷上传是明确选出来的互操作方案：

1. 用户在传输面板选"可续传大文件"，指定分卷大小（默认 100 MB，可配置）。
2. Rust 在远端建一个用户命名的文件夹（例如 `MyBigFile.split/`）。
3. 本地把源文件流式切成 `part-000.bin`、`part-001.bin`、……**不含专有文件头**，就是原始字节切片。
4. 每个 part 上传成功后：
   - 计算 SHA-256。
   - 追加到 `SHA256SUMS`。
   - 追加到 `manifest.json`（含 part 序号、大小、SHA-256、原文件总大小）。
   - 写入 `SplitUploadCheckpoint`（含 `completed_chunks: Vec<SplitPart>`）到 `transfer-checkpoints.json`。
5. 全部完成后再上传：
   - `README.txt`（中英双语，说明这个文件夹是什么、怎么还原）
   - `restore.bat`（Windows：`copy /b part-000.bin+part-001.bin+... target.bin`）
   - `restore.sh`（POSIX：`cat part-*.bin > target.bin`）
6. 删除 checkpoint。

**恢复**：中断后 `list_resumable_transfers()` 列出这个任务，用户点"继续"就从 `completed_chunks.len()` 之后开始传下一个 part。已确认完整的 part 不重传。

**远端可见性**：文件夹在 Koofr 网页 / 移动端表现为普通文件夹，用户可以自己下载、用 `copy /b` 或 `cat` 拼回原文件，**不依赖本客户端**。这是有意的互操作性保证。

支持文件：`transfer/split_upload.rs`、`transfer/split_package.rs`、`split_commands.rs`。

### 文件夹递归下载

`folder_download.rs` 实现：

1. 通过 Koofr API 递归列出目标文件夹的所有子项，构建清单。
2. 在下载父目录下建**临时目录** `.koofr-folder-<transferId>/`，暂存所有内容。
3. 每个文件走单文件下载路径（可以中途取消）。
4. 全部完成后 rename 临时目录 → 最终目录名（同级重名加序号）。
5. **失败或取消时清理整个临时目录**，不留半成品。

文件夹下载**不做**磁盘 checkpoint（重启后无法续传整个文件夹），因为 Koofr 目录清单可能已变；重新开始比"部分恢复导致混淆"更安全。已下载单个文件的字节 checkpoint 仍然保留。

### 网络错误自动恢复

设置中的 `autoRetryNetworkErrors` 只匹配 `AppError::Network`（前端错误码 `network_error`）。`networkRetryLimit` 表示初次请求失败后的最大重试次数，`null` 表示无限；`networkRetryIntervalSeconds` 是每次重试前使用的固定等待时间。等待期间会发出 `Retrying` 状态，暂停和取消仍由同一个 `CancellationToken` 控制。

- 单文件下载重新读取临时文件长度并使用 HTTP Range 从已落盘偏移继续。
- 分卷上传重新核对远端完整分卷，从最后一个已确认分卷继续。
- 普通上传受 Koofr 整文件 `FilesPut` 接口限制，每次重试必须从文件开头重新发送。
- 文件夹下载会清理本次失败的暂存树后，从头重建目录下载。
- 权限、冲突、本地 I/O、HTTP 状态和内容完整性错误不会触发该策略。
- 每次调度重试都会写入脱敏日志事件 `network_retry_scheduled`，仅包含传输 ID、方向、次数和延迟。

### 元数据缓存

`MetadataCache` 三种模式：

| 模式 | 存储位置 | TTL |
| --- | --- | --- |
| `Off` | 无缓存，每次都请求 Koofr | - |
| `Memory` | 进程内存 `RwLock<HashMap>` | 可配置（默认 15 分钟）|
| `Disk` | 用户在设置中指定的缓存文件夹 | 可配置 |

- 缓存 key = `(userId, mountId, remotePath)`，按账户完全隔离。
- 缓存 value = **仅**文件名 + 远程路径 + 大小 + 修改时间。**不缓存令牌、密码、文件内容**。
- 切换模式、清缓存、切换账户时自动失效。
- 缓存文件夹可在设置中更改；后端只接受已有的绝对目录并拒绝符号链接，切换时迁移缓存文件。

### 诊断日志

`AppLogger` 使用独立后台线程写入 JSONL，不阻塞 Tauri 命令或传输任务。设置页可以指定日志文件夹、记录级别、保留天数和单文件大小上限，也可以查看占用并一键清理。

- 活动文件为 `koofr-gui.jsonl`，达到大小上限后使用带时间戳和随机 ID 的文件名轮转。
- 轮转文件按保留天数自动清理；默认保留 14 天，单文件上限 10 MB。
- 传输日志记录 session ID、transfer ID、方向、终态、字节数、稳定错误码，以及脱敏后的 HTTP 状态 / I/O 类别 / 网络失败分类。
- **绝不记录**认证头、令牌、密码、邮箱、文件名、完整本地路径、远程路径或服务端响应正文。

### 凭据存储

- **会话 access / refresh token**：只在 Rust 进程内存，`zeroize` 清理。
- **应用专用密码**（用户勾了"记住我"时）：Windows Credential Manager，键格式 `KoofrGUI:<email>`，作用域当前 Windows 用户。
- **服务端返回的错误响应正文**：读一次拿到错误码后立刻丢弃，不写日志。

失败或用户勾"忘记我"时，从 Credential Manager 里删除对应记录。

### 错误模型

`src-tauri/src/error.rs`:

```rust
pub enum AppError {
    Network,
    Unauthorized,
    NotFound,
    Conflict,
    InvalidInput(&'static str),
    ServerError,
    LocalData,
    LocalIo,
    Cancelled,
    // ...
}
```

序列化给前端时只暴露：

```typescript
{ code: string, message: string }
```

- `code` 是稳定的英文标识（例如 `unauthorized`、`not_found`、`invalid_input:remote_path`），前端可以用来做逻辑判断（`isCommandErrorCode`）。
- `message` 是安全的国际化消息，**不含**具体路径、令牌、服务端响应正文。

## 事件与命令目录

**Tauri 命令**（前端 → Rust，由 `commands.rs` / `folder_commands.rs` / `link_commands.rs` / `split_commands.rs` / `transfer_commands.rs` 注册）：

认证 / 会话：`connect_koofr`, `restore_saved_login`, `disconnect_koofr`, `koofr_session`, `forget_saved_login`

设置：`get_settings`, `update_settings`, `update_download_settings`, `update_logging_settings`, `update_transfer_settings`, `clear_metadata_cache`, `clear_logs`, `select_settings_directory`

文件浏览：`list_mounts`, `list_files`, `list_recent`, `list_shared`, `list_trash`

文件操作：`create_folder`, `rename_entry`, `move_entry`, `copy_entry`, `delete_entry`, `restore_trash`, `empty_trash`

本地路径授权：`select_upload_file`, `select_download_location`, `select_download_folder`, `select_download_directory`, `prepare_download_location`, `prepare_download_folder`

传输：`upload_file`, `upload_split_file`, `download_file`, `download_folder`, `cancel_transfer`, `list_resumable_transfers`, `resume_transfer`, `discard_resumable_transfer`

分享链接：`list_public_links`, `create_public_link`, `delete_public_link`

**Tauri 事件**（Rust → 前端）：

- `koofr://transfer-progress`：传输进度 / 状态变化。

## 本地数据布局

```
%LOCALAPPDATA%\net.koofr.desktop.gui\
├─ settings.json                # AppSettings（下载、缓存和日志策略等）
├─ transfer-checkpoints.json    # TransferCheckpointStore（可恢复任务）
├─ download-history.json        # 按账户隔离的下载历史、目标位置与限量速度采样
├─ cache/metadata-cache.json    # 默认磁盘缓存位置（可配置）
└─ logs/koofr-gui*.jsonl       # 默认诊断日志位置（可配置）
```

**Windows 凭据管理器**（不在上面目录里）：

- `KoofrGUI:<email>` → 用户勾"记住我"时保存的应用专用密码。

**下载临时文件**（在用户配置的下载目录下）：

- `.koofr-part-<transferId>`：单文件下载暂存。
- `.koofr-folder-<transferId>/`：文件夹下载暂存目录。

清理 / 卸载应用时，这三处都不会自动删除，需要用户手动处理。

## 未实现的部分

### `crypto/` 和 `vault_core/`

Koofr Vault = Koofr 网页端一个基于 rclone crypt 的端到端加密卷。规划中的实现要点（不是承诺时间线）：

- 前端只见密文文件名 / 密文目录结构，Vault Safe Key **绝不进前端**。
- 解密 / 加密 / 文件名 EME 全部在 Rust 侧。
- 格式必须兼容 rclone crypt，能被 Koofr 网页端和 rclone 无损互操作。
- 密钥派生用 scrypt（rclone crypt 默认）。

### OAuth 和第三方存储管理

需要 Koofr 提供公版桌面客户端注册信息（client_id）和公开授权 API。在此之前：

- 应用能列出账户里已连接的 Google Drive / OneDrive / Dropbox 等存储，正常读写。
- 新增 / 移除 / 重新授权只能引导用户到 [Koofr 官方账户页面](https://app.koofr.net/app/admin/connections) 操作。

### 平台

macOS 和 Linux 支持排期未定。Tauri 本身跨平台，主要障碍是：Windows 凭据管理器要换成 `keyring` 后端；打包 / 分发要重做；对应平台的原生文件对话框行为差异需要测试。
