# 更新日志

格式按 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，版本号按 [SemVer](https://semver.org/spec/v2.0.0.html)。

## [1.6.1] - 2026-07-24

### Added
- 应用工作目录选择，可在下次启动前选择全量迁移设置、传输状态、下载历史、缓存和日志

### Changed
- 缓存与日志固定收纳到应用工作目录；移除各自的路径选择，保留缓存策略和原有日志级别、保留期、轮转、占用与清理配置。

## [1.6.0] - 2026-07-23

### Added
- Koofr Vault 完整工作区：创建、导入/导出、原生 Safe Key 解锁、锁定、自动锁定、移除注册、解密浏览和文件操作
- 使用 [Koofr Vault](https://github.com/koofr/vault) 提供的 `vault-crypto` 实现 rclone crypt 兼容的 AES-EME 文件名与 XSalsa20-Poly1305 内容加密；加密上传可整文件恢复，密文下载支持 HTTP Range 字节续传
- Vault / rclone golden vector、Unicode + salt、块边界、篡改拒绝和 WebView 无 Safe Key 输入覆盖
### Security
- Vault Safe Key 只经过 Windows 原生凭据窗口和 Rust 内存；前端、日志、缓存、配置与传输检查点均不保存 Safe Key
- Vault 文件操作通过短期不透明句柄在 Rust 侧重新解析并限制到已注册根目录，密文路径和 validator 不暴露给 WebView

## [1.5.1] - 2026-07-23

### Added
- 文件图标大幅扩展覆盖面：新增视频（mp4、mkv、mov、avi、webm 等）、音频（mp3、flac、wav、ogg 等）、代码（ts/tsx、js、py、rs、go、java、c/cpp、sh、sql、yaml 等）、演示文稿（pptx/ppt/odp/key）、纯文本（txt、md、log、csv 等）、字体（ttf、otf、woff/woff2）、电子书（epub、mobi、azw3）、光盘映像（iso、img、vhd、vmdk）与数据库（sqlite、db、parquet）九类专属彩色图标
- 图片、压缩包、可执行文件、表格与文档的识别扩展名同步补全（heic、avif、zst、apk、deb、xlsm、docm 等）

### Changed
- 传输面板的文件类型识别改为复用 `fileKindByName`，与文件列表保持一致，不再维护两份扩展名表
- `.ts` 后缀按视频（MPEG-TS）识别，TypeScript 源码以 `.tsx` 识别为代码
- 传输面板只为具有恢复检查点的活动任务显示暂停按钮

## [1.5.0] - 2026-07-23

### Added
- 传输面板新增可选择的紧凑列表、文件来源与位置、开始/完成时间、平均速度和最近一分钟速度曲线
- 上传与下载速度曲线支持点击切换折线和平滑样式
- 下载历史按 Koofr 账户持久化，应用重启后仍可查看并打开已完成下载的位置

### Changed
- 下载入口角标只统计正在下载或等待网络重试的下载任务，不再统计历史记录、暂停任务或上传任务

## [1.4.0] - 2026-07-20

### Added
- 设置页新增“遇到网络错误时自动继续”开关，可设置最大重试次数（含无限）和固定重试间隔
- 设置页可配置缓存文件夹、日志文件夹、日志级别、保留天数和轮转大小，并可查看占用及清理日志
- 后台 JSONL 诊断日志，记录脱敏后的传输失败类别与关联 ID
- 顶层 `LICENSE`（MIT）、`CONTRIBUTING.md`、`SECURITY.md`
- `docs/ARCHITECTURE.md`：完整架构、数据流、传输恢复、安全边界
- `.github/ISSUE_TEMPLATE/`（bug / feature）与 `.github/PULL_REQUEST_TEMPLATE.md`
- 英文 README（`README.en.md`）

### Changed
- 网络自动恢复期间传输面板显示“等待网络重试”，且任务仍可暂停或取消
- 上传、下载和分卷上传仅在用户明确暂停时进入“已暂停”；网络、权限和本地 I/O 等真实错误现在显示为失败并保留重试入口
- 重写 `README.md`，按开源项目通用结构组织
- 精修 `docs/BUILDING.md` 与 `docs/RELEASING.md` 的措辞、目录与常见问题

## [1.3.3] - 2026-07-20

### Changes

- Fix transfer progress resetting to 0% when paused

## [1.3.2] - 2026-07-19

### Changes

- Fix Rust formatting required by the release workflow

## [1.3.1] - 2026-07-19

### Changes

- Add a pause button for active transfers
- Refactor the frontend structure

## [1.3.0] - 2026-07-19

### 变更

- 重构上传和下载文件后端
- 支持下载断点续传
- 支持上传时分卷拆分大文件并提供恢复方式

- Add sharing-link management for querying, creating, copying, and confirming revocation of download and file-receiving links
- Detect existing Koofr, Google Drive, OneDrive, Dropbox, and other connected storage mounts

## [1.1.0] - 2026-07-17

### 变更

- Add reusable inline SVG icons for folders and supported file types
- Detect archive and executable extensions in file listings
- Apply semantic icon tokens and document the new design variants
- Add coverage for all supported file kinds

## [1.0.0] - 2026-07-16

### 新增

- Koofr 应用专用密码登录，可选 Windows 凭据管理器存储，会话恢复，退出登录。
- 浏览挂载点、目录、最近文件、共享内容和回收站。
- 新建文件夹、上传、下载、重命名、移动、复制、删除、回收站恢复。
- 流式传输进度、取消、递归文件夹下载、未完成下载清理。
- 可配置元数据缓存，响应式 Windows 桌面界面。

### 安全

- 凭据始终留在 Rust 后端。可选的持久化存储用 Windows 凭据管理器。
- 发布构建在仓库质量检查通过后上传到 GitHub Release。当前不签名。

### 已知限制

- 1.0.0 不含 Koofr Vault 解锁、加密传输或 rclone-crypt 兼容性。
- 1.0.0 不含传输重试和续传。
- 安装包未签名，Windows 可能显示未知发布者或 SmartScreen 警告。
