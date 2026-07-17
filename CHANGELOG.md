# 更新日志

本文件记录 Koofr-GUI 的所有重要变更。

格式遵循 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)，版本号遵循 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)。

## [1.1.0] - 2026-07-17

### 变更

- Add reusable inline SVG icons for folders and supported file types
- Detect archive and executable extensions in file listings
- Apply semantic icon tokens and document the new design variants
- Add coverage for all supported file kinds

## [1.0.0] - 2026-07-16

### 新增

- Koofr 应用专用密码登录、可选 Windows 凭据管理器存储、会话恢复和退出登录。
- 挂载点、目录、最近文件、共享内容和回收站浏览。
- 新建文件夹、上传、下载、重命名、移动、复制、删除和回收站恢复。
- 流式传输进度、取消、递归文件夹下载和未完成下载清理。
- 可配置元数据缓存和响应式 Windows 桌面界面。

### 安全

- 凭据始终保留在 Rust 后端；可选的持久化存储使用 Windows 凭据管理器。
- 发布构建在仓库质量检查通过后上传到 GitHub Release；当前发布包不使用代码签名证书。

### 已知限制

- 1.0.0 不包含 Koofr Vault 解锁、加密传输或 rclone-crypt 兼容性。
- 1.0.0 不包含传输重试和续传。
- 安装程序未签名，Windows 可能显示未知发布者或 SmartScreen 警告。
