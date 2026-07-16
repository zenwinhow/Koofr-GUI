# Koofr-GUI

Koofr-GUI 是一个 Windows 优先的 Koofr 桌面文件管理客户端，采用 Tauri v2、React、TypeScript 和 Rust 构建。项目目标是提供接近原生文件管理器的体验，而不是简单封装 Koofr 网页。

> 1.0.0 作为首个正式 Windows 发行版本，提供普通 Koofr 文件管理功能；Koofr Vault 兼容层尚未实现，因此 Vault 工作流不属于此版本范围。

## 当前功能

- Koofr 应用专用密码登录、启动时恢复会话和退出登录。
- 可选使用 Windows Credential Manager 保存登录凭据；密码不会写入普通配置或 WebView 存储。
- 浏览挂载点、目录、最近文件、共享内容和回收站。
- 新建文件夹、上传、单文件与递归文件夹下载、重命名、移动、复制、删除和回收站恢复。
- 流式传输、目录累计进度、取消操作，以及未完成文件和文件夹下载的临时内容清理。
- 可配置默认下载文件夹；可选择每次下载前询问位置，支持手填路径或原生文件夹选择器，并自动为重名下载追加序号而不覆盖已有内容。
- 可配置的内存/磁盘元数据缓存和响应式桌面界面。

尚未实现：Koofr Vault 解锁与加解密、Vault 传输，以及完整的传输重试/续传。

## 快速开始

构建环境需要 Node.js 24 LTS、npm 10+、Rust 1.88+ MSVC 工具链、Visual Studio C++ Build Tools、Windows SDK 和 WebView2。

```powershell
git clone <仓库地址>
Set-Location Koofr-GUI
npm ci
npm run dev:desktop
```

仅运行前端开发服务器：

```powershell
npm run dev
```

完整的环境安装、检查、发布构建、产物位置、故障排查和清理说明请阅读 [构建与清理指南](docs/BUILDING.md)。

## 构建与检查

```powershell
# 运行 ESLint、前端生产构建、Rust 格式检查、Clippy 和单元测试
npm run check

# 构建 Windows 发布版可执行文件
npm run build:desktop
```

发布版可执行文件输出到 `src-tauri/target/release/koofr-gui.exe`，NSIS 安装程序输出到 `src-tauri/target/release/bundle/nsis/`。

## 清理

```powershell
# 删除前端、Rust 和 Tauri 的可再生成构建产物，保留已安装依赖
npm run clean

# 同时删除 node_modules；之后需要重新执行 npm ci
npm run clean:all
```

清理脚本只操作仓库内的固定目录，不删除源码、锁文件、Windows 凭据或应用用户数据。各命令的准确删除范围见 [构建与清理指南](docs/BUILDING.md#6-清理构建产物)。

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `npm ci` | 按 `package-lock.json` 可复现地安装依赖 |
| `npm run dev` | 启动 Vite 前端开发服务器 |
| `npm run dev:desktop` | 启动完整 Tauri 桌面开发环境 |
| `npm run build` | 类型检查并生成 `dist/` 前端资源 |
| `npm run build:desktop` | 构建 Windows 发布版可执行文件 |
| `npm run check` | 运行全部前端和 Rust 检查 |
| `npm run clean` | 清理构建产物并保留依赖 |
| `npm run clean:all` | 清理构建产物和 `node_modules/` |

## 目录结构

```text
Koofr-GUI/
|-- docs/                 项目文档和设计资料
|-- public/               前端静态资源
|-- scripts/              项目维护脚本
|-- src/                  React / TypeScript 界面与 Tauri 调用封装
|-- src-tauri/            Rust / Tauri 后端、权限和桌面配置
|-- package.json          npm 命令与前端依赖
|-- package-lock.json     锁定的 JavaScript 依赖
`-- src-tauri/Cargo.lock  锁定的 Rust 依赖
```

前端目录职责见 [src/README.md](src/README.md)，Rust/Tauri 边界和命令说明见 [src-tauri/README.md](src-tauri/README.md)。

## 架构与安全边界

```text
React + TypeScript UI (`src/`)
        |
        | typed, narrowly scoped Tauri commands/events
        v
Rust + Tauri core (`src-tauri/src/`)
        |-- file_ops/            路径与文件操作校验
        |-- transfer/            上传、下载、进度与取消
        |-- koofr_api/           Koofr REST API
        |-- credential_manager/  Windows 安全凭据存储
        |-- vault_core/          计划中的 Vault 兼容层
        `-- crypto/              计划中的加密支持
```

凭据、文件系统访问、网络请求和未来的 Vault 密钥处理均保留在 Rust 边界内。前端只调用受限且类型化的 Tauri 命令，不应保存密码、令牌或 Vault Safe Key。

## 开发约定

- 使用 `npm ci` 和已提交的两个锁文件，不随意升级依赖。
- 编辑器遵循根目录 `.editorconfig`；文本文件统一使用 UTF-8 和 LF 换行。
- 提交前运行 `npm run check`。
- 不提交构建产物、依赖目录、账号凭据、访问令牌或真实 Secret 形式的测试数据。
- 新增 Tauri 命令时必须在 Rust 侧验证路径、远程标识符和操作范围。
- 下载目录可以由用户在界面中明确填写，但前端不能拼接最终文件名；Rust 必须验证父目录、清理远端名称、避免覆盖并签发一次性下载授权。
- Vault 功能必须兼容 Koofr/rclone crypt，不能创建自定义加密格式。

推送与版本号完全匹配的 `v*` 标签会触发 GitHub Actions：执行质量检查、导入代码签名证书、构建 NSIS 安装程序并创建同名 GitHub Release。发布所需机密、版本规则和操作步骤见 [发布流程](docs/RELEASING.md)。正式版不会上传未签名安装程序。
