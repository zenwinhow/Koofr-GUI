# 构建与清理

写给想在 Windows 上从源码构建 Koofr-GUI 的人。所有命令从仓库根目录跑，示例用 PowerShell。

## 1. 环境要求

Tauri v2 + React + TypeScript + Rust，需要以下东西：

- Windows 10 或 11（x64）。
- Node.js 24 LTS（推荐），22.12 以上的 22.x LTS 也行。
- npm 10 或更高。
- Rust 1.88+，用 `x86_64-pc-windows-msvc` 工具链。
- Visual Studio 2022 Build Tools 的"使用 C++ 的桌面开发"工作负载和 Windows SDK。
- Microsoft Edge WebView2 Runtime。受支持的 Windows 10/11 通常自带。

安装方式以 [Tauri Windows 前置要求](https://v2.tauri.app/start/prerequisites/) 为准。Node.js 去 [官方下载页](https://nodejs.org/en/download) 拿 LTS 版本，Rust 用 [rustup](https://rustup.rs/) 装。

装完重启 PowerShell，检查一下：

```powershell
node --version
npm --version
rustc --version
cargo --version
rustup show active-toolchain
```

最后一条应该显示 MSVC 工具链，比如 `stable-x86_64-pc-windows-msvc`。如果不是，跑：

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

## 2. 拉代码和装依赖

```powershell
git clone <仓库地址>
Set-Location Koofr-GUI
npm ci
```

用 `npm ci` 而不是 `npm install`——它严格按照已提交的 `package-lock.json` 装前端和 Tauri CLI 依赖，不会改锁文件。Rust 依赖在首次开发、检查或构建时根据 `src-tauri/Cargo.lock` 自动下载。

别提交 `node_modules/`、`dist/`、`src-tauri/gen/` 或 `src-tauri/target/`。这些都是能重新生成的。

## 3. 开发运行

只在浏览器里跑前端：

```powershell
npm run dev
```

开发服务器固定 `http://127.0.0.1:1420/`。这个模式测不了依赖 Tauri 命令的桌面功能。

跑完整桌面应用：

```powershell
npm run dev:desktop
```

同时启动 Vite 开发服务器和 Tauri 调试进程。第一次编译 Rust 会比较慢，后面增量编译就快了。

## 4. 构建

### 4.1 前端静态资源

```powershell
npm run build
```

依次跑 TypeScript 项目构建和 Vite 生产构建，输出到 `dist/`。不生成 Windows 桌面程序。

### 4.2 Windows 桌面程序

建议先快速验证再构建发布版本：

```powershell
npm run verify:quick
npm run build:desktop
```

`npm run build:desktop` 通过 Tauri 自动执行前端生产构建，然后编译 Rust 发布版本，用 `--no-bundle` 只生成 exe。成功后文件在：

```
src-tauri/target/release/koofr-gui.exe
```

`src-tauri/tauri.conf.json` 里保留了 NSIS 打包配置，但本地 `build:desktop` 不会生成安装包。要显式构建 NSIS 安装包用 `npm run build:installer`。正式发布由 GitHub Actions 负责。发布包不签名，Windows 可能显示未知发布者或 SmartScreen 警告。具体步骤看[发布流程](RELEASING.md)。

## 5. 质量检查

完整检查：

```powershell
npm run check
```

按顺序跑：

1. ESLint。
2. TypeScript 和 Vite 生产构建。
3. `cargo fmt --check`。
4. Clippy（警告当错误）。
5. Rust 单元测试。

也可以单独跑：

```powershell
npm run lint
npm run build
npm run check:rust
```

## 6. 清理构建产物

清理命令只删仓库里固定的可重新生成目录。跑之前先退出开发服务器和正在运行的 `koofr-gui.exe`，不然 Windows 可能因为文件占用删不掉 Rust 产物。

想预览清理范围但不删东西：

```powershell
node scripts/clean.mjs build --dry-run
```

### 6.1 常规清理

```powershell
npm run clean
```

删除：

- `dist/`：Vite 生产构建。
- `node_modules/.tmp/`：TypeScript 增量信息。
- `node_modules/.vite/`：Vite 依赖缓存。
- `src-tauri/target/`：Rust 调试、发布和增量编译产物。
- `src-tauri/gen/`：Tauri 自动生成的 schema。

保留 `node_modules/` 里的依赖，之后可以直接重新构建。

### 6.2 分层清理

只清前端：

```powershell
npm run clean:frontend
```

只清 Rust/Tauri：

```powershell
npm run clean:rust
```

### 6.3 完全重置

```powershell
npm run clean:all
npm ci
```

`clean:all` 在常规清理基础上删掉整个 `node_modules/`。适合依赖装坏了、换了 Node.js 主版本或想释放磁盘空间的时候用。重新开发或构建前必须再跑 `npm ci`。

这些命令不会删源码、`package-lock.json`、`src-tauri/Cargo.lock`、Git 文件、Windows Credential Manager 里保存的 Koofr 凭据，也不会动 Windows 用户目录下的应用设置或元数据缓存。

## 7. 常见问题

### 找不到 `link.exe`、Windows SDK 或 C++ 工具

打开 Visual Studio Installer，给 Visual Studio Build Tools 加上"使用 C++ 的桌面开发"工作负载和 Windows SDK，装完重启终端。

### Rust 用了 GNU 工具链

本项目在 Windows 上需要 MSVC 工具链：

```powershell
rustup default stable-x86_64-pc-windows-msvc
```

### 端口 1420 被占用

找到占用 `127.0.0.1:1420` 的进程杀掉。Vite 配置开了 `strictPort`，不会自动换端口，因为 Tauri 开发配置依赖固定地址。

### 清理时出现 `EPERM` 或"文件正在使用"

关掉 Vite、Tauri、`koofr-gui.exe` 和可能正在扫描构建目录的工具，再跑清理。不要手动删不认识的系统目录——项目脚本只会操作仓库内的白名单路径。

### 确认干净构建

```powershell
npm run clean:all
npm ci
npm run check
npm run build:desktop
```

从锁文件重装 JavaScript 依赖，重新生成全部前端和 Rust 构建产物。