# 更新日志

格式按 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，版本号按 [SemVer](https://semver.org/spec/v2.0.0.html)。

## [Unreleased]

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
