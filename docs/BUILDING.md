# 构建指南

Windows 上从源码构建 Koofr-GUI 的完整流程。所有命令从仓库根目录执行，示例用 PowerShell。

## 目录

- [1. 环境要求](#1-环境要求)
- [2. 拉代码和装依赖](#2-拉代码和装依赖)
- [3. 开发运行](#3-开发运行)
- [4. 构建](#4-构建)
- [5. 质量检查](#5-质量检查)
- [6. 清理构建产物](#6-清理构建产物)
- [7. 常见问题](#7-常见问题)

## 1. 环境要求

Koofr-GUI 基于 Tauri v2 + React + TypeScript + Rust，需要以下东西：

| 组件 | 版本 | 说明 |
| --- | --- | --- |
| 操作系统 | Windows 10 / 11（x64） | 目前只支持 Windows |
| Node.js | 24 LTS（推荐），22.12+ 22.x LTS 也行 | 从 [官方下载页](https://nodejs.org/en/download) 拿 LTS |
| npm | 10+ | 随 Node.js 一起装 |
| Rust | 1.88+，`x86_64-pc-windows-msvc` 工具链 | 用 [rustup](https://rustup.rs/) 装 |
| Visual Studio 2022 Build Tools | 含 "使用 C++ 的桌面开发" 工作负载 + Windows SDK | Tauri 编译原生代码需要 |
| Microsoft Edge WebView2 Runtime | 受支持的 Windows 通常自带 | 运行时依赖 |

完整安装方式以 [Tauri Windows 前置要求](https://v2.tauri.app/start/prerequisites/) 为准。

装完重启 PowerShell 检查：

```powershell
node --version
npm --version
rustc --version
cargo --version
rustup show active-toolchain
```

最后一条应该显示 MSVC 工具链，例如 `stable-x86_64-pc-windows-msvc`。如果不是：

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

## 2. 拉代码和装依赖

```powershell
git clone https://github.com/zenwinhow/Koofr-GUI.git
Set-Location Koofr-GUI
npm ci
```

用 `npm ci` 而不是 `npm install` —— 它严格按照已提交的 `package-lock.json` 装前端和 Tauri CLI 依赖，不会改锁文件。Rust 依赖在首次开发 / 检查 / 构建时根据 `src-tauri/Cargo.lock` 自动下载。

**永远不要提交** `node_modules/`、`dist/`、`src-tauri/gen/` 或 `src-tauri/target/`，这些都是能重新生成的。

## 3. 开发运行

### 只跑前端（浏览器）

```powershell
npm run dev
```

开发服务器固定 `http://127.0.0.1:1420/`。这个模式**测不了**依赖 Tauri 命令的桌面功能，登录、传输、文件操作等都无法工作。适合快速调 UI 布局或样式。

### 跑完整桌面应用

```powershell
npm run dev:desktop
```

同时启动 Vite 开发服务器和 Tauri 调试进程。第一次编译 Rust 会比较慢（几分钟），后续增量编译很快。

### 只前端热重载不重启 Rust

`npm run dev:desktop` 起来后，改前端代码走 Vite HMR，Rust 端**不会重启**。这在调 UI 又不想中断正在跑的传输时很有用。

## 4. 构建

### 4.1 前端静态资源

```powershell
npm run build
```

依次跑 TypeScript 项目构建和 Vite 生产构建，输出到 `dist/`。不生成 Windows 桌面程序。

### 4.2 Windows 桌面程序

建议先快速验证再构建：

```powershell
npm run verify:quick    # lint + 拆分上传测试 + 前端构建
npm run build:desktop
```

`npm run build:desktop` 通过 Tauri 自动触发前端生产构建，然后编译 Rust 发布版本，用 `--no-bundle` 只生成 exe。成功后文件在：

```
src-tauri/target/release/koofr-gui.exe
```

### 4.3 NSIS 安装包

```powershell
npm run build:installer
```

生成 NSIS 安装包到：

```
src-tauri/target/release/bundle/nsis/Koofr-GUI_<version>_x64-setup.exe
```

`tauri.conf.json` 里保留了 NSIS 配置。本地 `build:desktop` 不生成安装包是为了迭代快。正式发布由 GitHub Actions 负责，详见 [RELEASING.md](RELEASING.md)。

> ⚠️ **发布包不做代码签名**。Windows 可能显示未知发布者或 SmartScreen 警告。理由和权衡见 [SECURITY.md](../SECURITY.md#已知的非漏洞行为)。

## 5. 质量检查

### 完整检查（推荐 PR 前跑）

```powershell
npm run check
```

按顺序执行：

1. **ESLint**（`npm run lint`）
2. **Vitest**（`npm run test`）
3. **TypeScript + Vite 生产构建**（`npm run build`）
4. **`cargo fmt --check`**
5. **Clippy**，警告当错误（`-D warnings`）
6. **Rust 单元测试**（`cargo test`）

任何一步失败整个流程就会终止。

### 分开跑

```powershell
npm run lint            # 只跑 ESLint
npm run test            # 只跑前端测试
npm run build           # 只跑前端类型检查 + 构建
npm run check:rust      # 只跑 Rust 三件套（fmt + clippy + test）
```

### 快速验证

```powershell
npm run verify:quick    # lint + 拆分上传测试 + 前端构建
```

比 `npm run check` 快很多，适合小改动的自检。但正式提交前还是建议跑完整的 `npm run check`。

## 6. 清理构建产物

清理命令只删仓库里固定的可重新生成目录。跑之前**必须先退出**开发服务器和正在运行的 `koofr-gui.exe`，否则 Windows 因为文件占用可能删不掉 Rust 产物。

想预览清理范围但不删东西：

```powershell
node scripts/clean.mjs build --dry-run
```

### 6.1 常规清理

```powershell
npm run clean
```

删除：

- `dist/`：Vite 生产构建
- `node_modules/.tmp/`：TypeScript 增量信息
- `node_modules/.vite/`：Vite 依赖缓存
- `src-tauri/target/`：Rust 调试 / 发布 / 增量编译产物
- `src-tauri/gen/`：Tauri 自动生成的 schema

保留 `node_modules/` 里的依赖，之后可以直接重新构建。

### 6.2 分层清理

只清前端：

```powershell
npm run clean:frontend
```

只清 Rust / Tauri：

```powershell
npm run clean:rust
```

### 6.3 完全重置

```powershell
npm run clean:all
npm ci
```

`clean:all` 在常规清理基础上删掉整个 `node_modules/`。适合依赖装坏了、换了 Node.js 主版本或想释放磁盘空间的时候用。**重新开发或构建前必须再跑 `npm ci`**。

### 6.4 清理脚本不会动的东西

- 源码
- `package-lock.json`、`src-tauri/Cargo.lock`
- Git 内部文件
- Windows 凭据管理器里保存的 Koofr 凭据
- `%LOCALAPPDATA%\net.koofr.desktop.gui\` 里的应用设置和缓存
- 用户下载目录里的 `.koofr-part-*` 临时文件

要清理这些请手动操作，或者用应用内的"清缓存 / 忘记登录"功能。

## 7. 常见问题

### 找不到 `link.exe`、Windows SDK 或 C++ 工具

打开 Visual Studio Installer，给 Visual Studio Build Tools 加上"使用 C++ 的桌面开发"工作负载和 Windows SDK。装完重启终端。

### Rust 用了 GNU 工具链

本项目在 Windows 上需要 MSVC 工具链：

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

### 端口 1420 被占用

找到占用 `127.0.0.1:1420` 的进程杀掉。Vite 配置开了 `strictPort`，不会自动换端口（Tauri 开发配置依赖固定地址）。

```powershell
# 找占用进程
netstat -ano | Select-String ":1420"
# 结束进程（PID 从上一步拿）
taskkill /PID <PID> /F
```

### 清理时出现 `EPERM` 或"文件正在使用"

关掉 Vite、Tauri、`koofr-gui.exe` 和可能正在扫描构建目录的工具（IDE 索引、杀毒软件），再跑清理。不要手动删不认识的系统目录 —— 项目脚本只会操作仓库内的白名单路径。

### 首次编译 Rust 非常慢

正常。Tauri v2 依赖链不小，第一次全量编译在中端笔记本上可能要 3–8 分钟。之后走增量编译就快了。如果卡在某个 crate 一直不动，检查 `%CARGO_HOME%\.cargo\config.toml` 有没有配错误的镜像。

### `error: Microsoft Visual C++ 14.0 or greater is required`

Windows SDK 版本太老或没装。装 Visual Studio 2022 Build Tools 的 Windows 11 SDK（10.0.22621.0 或更新）。

### `webview2` 报错说找不到 Runtime

用户机器上没装 WebView2 Runtime。开发机通常自带；如果确实缺失，从 [Microsoft 官方](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) 装 Evergreen Bootstrapper。

### 确认干净构建

```powershell
npm run clean:all
npm ci
npm run check
npm run build:desktop
```

从锁文件重装 JavaScript 依赖，重新生成全部前端和 Rust 构建产物。这个流程也是发布前建议做一遍的。
