# 构建与清理指南

本文档面向希望在 Windows 上从源码构建 Koofr-GUI 的开发者。所有命令均从仓库根目录执行，示例使用 PowerShell。

## 1. 环境要求

Koofr-GUI 使用 Tauri v2、React、TypeScript 和 Rust。当前项目需要：

- Windows 10 或 Windows 11（x64）。
- Node.js 24 LTS（推荐）；项目也接受 Node.js 22.12 及以上的 22.x LTS。
- npm 10 或更高版本。
- Rust 1.88 或更高版本，并使用 `x86_64-pc-windows-msvc` 工具链。
- Visual Studio 2022 Build Tools 中的“使用 C++ 的桌面开发”工作负载和 Windows SDK。
- Microsoft Edge WebView2 Runtime。受支持的 Windows 10/11 通常已经安装。

安装方式和组件名称以 [Tauri Windows 前置要求](https://v2.tauri.app/start/prerequisites/) 为准。Node.js 建议从 [Node.js 官方下载页](https://nodejs.org/en/download) 获取 LTS 版本，Rust 建议通过 [rustup](https://rustup.rs/) 安装。

安装完成后重启 PowerShell，并检查版本：

```powershell
node --version
npm --version
rustc --version
cargo --version
rustup show active-toolchain
```

最后一条命令应显示 MSVC 工具链，例如 `stable-x86_64-pc-windows-msvc`。若不是，可执行：

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

## 2. 获取源码与安装依赖

```powershell
git clone <仓库地址>
Set-Location Koofr-GUI
npm ci
```

请优先使用 `npm ci`。它严格按照已提交的 `package-lock.json` 安装前端和 Tauri CLI 依赖，不会在安装过程中改写锁文件。Rust 依赖会在首次开发、检查或构建时根据 `src-tauri/Cargo.lock` 自动下载。

不要提交 `node_modules/`、`dist/`、`src-tauri/gen/` 或 `src-tauri/target/`。这些目录都是可重新生成的内容。

## 3. 开发运行

仅运行浏览器中的前端界面：

```powershell
npm run dev
```

开发服务器固定为 `http://127.0.0.1:1420/`。该模式不能完整验证依赖 Tauri 命令的桌面功能。

运行完整桌面应用：

```powershell
npm run dev:desktop
```

此命令会同时启动 Vite 开发服务器和 Tauri 调试进程。首次 Rust 编译耗时通常明显长于后续增量编译。

## 4. 构建

### 4.1 构建前端静态资源

```powershell
npm run build
```

该命令依次执行 TypeScript 项目构建和 Vite 生产构建，输出到 `dist/`。它不会生成 Windows 桌面程序。

### 4.2 构建 Windows 桌面程序

建议先执行完整检查，再构建发布版本：

```powershell
npm run check
npm run build:desktop
```

`npm run build:desktop` 会通过 Tauri 自动执行前端生产构建，然后编译 Rust 发布版本。成功后可执行文件位于：

```text
src-tauri/target/release/koofr-gui.exe
```

`src-tauri/tauri.conf.json` 启用了 NSIS 打包；构建会生成可执行文件和 `src-tauri/target/release/bundle/nsis/` 下的安装程序。本地构建不会导入发布证书，因此不要把未签名的本地安装程序当作正式发布包。正式发布由 GitHub Actions 构建并签名，具体步骤见 [发布流程](RELEASING.md)。

## 5. 质量检查

完整检查：

```powershell
npm run check
```

该命令会按顺序运行：

1. ESLint。
2. TypeScript 和 Vite 生产构建。
3. `cargo fmt --check`。
4. Clippy，并把警告作为错误。
5. Rust 单元测试。

也可以单独运行：

```powershell
npm run lint
npm run build
npm run check:rust
```

## 6. 清理构建产物

清理命令只删除仓库内固定的可再生成目录。执行前请先退出开发服务器和正在运行的 `koofr-gui.exe`，否则 Windows 可能因文件占用而无法删除 Rust 产物。

如需先预览常规清理范围而不删除任何内容，可运行：

```powershell
node scripts/clean.mjs build --dry-run
```

### 6.1 常规清理（推荐）

```powershell
npm run clean
```

删除：

- `dist/`：Vite 生产构建。
- `node_modules/.tmp/`：TypeScript 增量信息。
- `node_modules/.vite/`：Vite 依赖缓存。
- `src-tauri/target/`：Rust 调试、发布和增量编译产物。
- `src-tauri/gen/`：Tauri 自动生成的 schema。

保留 `node_modules/` 中已安装的依赖，后续可以直接重新构建。

### 6.2 分层清理

只清理前端产物和缓存：

```powershell
npm run clean:frontend
```

只清理 Rust/Tauri 产物：

```powershell
npm run clean:rust
```

### 6.3 完全重置本地依赖与构建产物

```powershell
npm run clean:all
npm ci
```

`clean:all` 在常规清理基础上删除整个 `node_modules/`，适合依赖安装损坏、切换 Node.js 主版本或需要释放磁盘空间时使用。重新开发或构建前必须再次执行 `npm ci`。

这些清理命令不会删除源码、`package-lock.json`、`src-tauri/Cargo.lock`、Git 文件、Windows Credential Manager 中保存的 Koofr 凭据，也不会清理应用在 Windows 用户目录中的设置或元数据缓存。

## 7. 常见问题

### 找不到 `link.exe`、Windows SDK 或 C++ 工具

打开 Visual Studio Installer，为 Visual Studio Build Tools 添加“使用 C++ 的桌面开发”工作负载及 Windows SDK，完成后重启终端。

### Rust 使用了 GNU 工具链

本项目在 Windows 上要求 MSVC 工具链：

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

### 端口 1420 被占用

结束占用 `127.0.0.1:1420` 的进程后重试。Vite 配置启用了 `strictPort`，不会自动换用其他端口，因为 Tauri 开发配置依赖固定地址。

### 清理时出现 `EPERM` 或“文件正在使用”

关闭 Vite、Tauri、`koofr-gui.exe` 以及可能正在扫描构建目录的工具，然后重新执行清理命令。不要手动删除来源不明的系统目录；项目脚本只会操作仓库内的白名单路径。

### 如何确认是干净构建

```powershell
npm run clean:all
npm ci
npm run check
npm run build:desktop
```

这会从锁文件重新安装 JavaScript 依赖，并重新生成全部前端和 Rust 构建产物。
