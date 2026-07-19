# Koofr-GUI

Windows 上用的 Koofr 桌面客户端，用 Tauri v2 + React + TypeScript + Rust 凑出来的。想做成原生文件管理器那种感觉，而不是套个网页壳子完事。

当前版本 1.3.2，普通 Koofr 文件管理基本能用了。Vault 还没动。

## 现在能干什么

- 用 Koofr 应用专用密码登录，下次启动能自动恢复会话，也能退出登录。
- 登录凭据可以存到 Windows 凭据管理器里——不会写在普通配置文件里，也不会塞进 WebView 存储。
- 浏览挂载点、目录、最近文件、共享内容和回收站。
- 能认出来账户里已有的 Koofr、Google Drive、OneDrive、Dropbox 这些存储位置。
- 查、建、撤下载链接和接收文件链接。撤销链接会弹二次确认。
- 新建文件夹、传文件、分卷续传大文件、下载单文件或递归下载整个文件夹、重命名、移动、复制、删除、从回收站恢复。
- 传输有进度显示，能取消。单文件下载用 HTTP Range 加磁盘检查点做字节级续传。分卷上传会建一个用户命名的远端文件夹，切成自定义大小的原始二进制分卷，断了之后从最后一个完整的分卷接着传。
- 默认下载文件夹可以配置。每次下载前可以选问我要位置，支持手填路径或者用原生文件夹选择器。重名文件自动加序号，不会覆盖已有的。
- 元数据缓存可选内存或磁盘，界面做了响应式布局。

Koofr 的公开上传接口不提供分块会话或服务端偏移确认，所以普通上传断了就得重传整个文件。分卷续传是明确选出来的互操作方案：远端显示为一个文件夹，里面的 `part-*.bin` 不含专有文件头，用 Windows 的 `copy /b` 或者 Linux/macOS 的 `cat` 就能拼回去。文件夹里还放了 `README.txt`、恢复脚本、`SHA256SUMS` 和 `manifest.json`。它不会假装成 Koofr 里的一个普通文件。

还没做的：Koofr Vault 解锁、加解密、Vault 传输。OAuth 登录和第三方存储的新增、移除、重授权得等 Koofr 提供公版桌面客户端注册信息和公开授权 API。当前界面能展示已有的连接，引导用户去官方账户页面管理授权——不会复用网页 Cookie 也不会内置其他客户端的密钥。

## 跑起来

需要 Node.js 24 LTS、npm 10+、Rust 1.88+ MSVC 工具链、Visual Studio C++ Build Tools、Windows SDK 和 WebView2。具体安装步骤看[构建指南](docs/BUILDING.md)。

```powershell
git clone <仓库地址>
Set-Location Koofr-GUI
npm ci
npm run dev:desktop
```

只跑前端开发服务器（浏览器里看）：

```powershell
npm run dev
```

## 构建和检查

```powershell
# 跑 ESLint、前端生产构建、Rust 格式检查、Clippy 和单元测试
npm run check

# 构建 Windows 发布版 exe
npm run build:desktop
```

发布版 exe 生成在 `src-tauri/target/release/koofr-gui.exe`，NSIS 安装包在 `src-tauri/target/release/bundle/nsis/`。

## 清理

```powershell
# 删前端、Rust 和 Tauri 的构建产物，保留已安装依赖
npm run clean

# 连 node_modules 一起删；之后得重新 npm ci
npm run clean:all
```

清理脚本只动仓库里的固定目录，不删源码、锁文件、Windows 凭据或应用用户数据。具体删什么见[构建指南](docs/BUILDING.md#6-清理构建产物)。

## 常用命令

| 命令 | 干什么用的 |
| --- | --- |
| `npm ci` | 按 `package-lock.json` 装依赖，可复现 |
| `npm run dev` | 启动 Vite 前端开发服务器 |
| `npm run dev:desktop` | 启动完整 Tauri 桌面开发环境 |
| `npm run build` | 类型检查 + 生成 `dist/` 前端资源 |
| `npm run build:desktop` | 构建 Windows 发布版 exe（不生成安装包） |
| `npm run build:installer` | 显式构建 NSIS 安装包（发布流程用） |
| `npm run verify:quick` | 快速验证：lint + 拆分上传测试 + 前端构建 |
| `npm run verify:full` | 完整验证，等同 `npm run check` |
| `npm run check` | 跑全部前端和 Rust 检查 |
| `npm run clean` | 清理构建产物，保留依赖 |
| `npm run clean:all` | 清理构建产物 + `node_modules/` |

## 目录结构

```
Koofr-GUI/
├── docs/                 文档和设计资料
├── public/               前端静态资源
├── scripts/              维护脚本
├── src/                  React / TypeScript 界面和 Tauri 调用封装
├── src-tauri/            Rust / Tauri 后端、权限、桌面配置
├── package.json          npm 命令和前端依赖
├── package-lock.json     锁定的 JavaScript 依赖
└── src-tauri/Cargo.lock  锁定的 Rust 依赖
```

前端目录职责看 [src/README.md](src/README.md)，Rust 后端命令说明看 [src-tauri/README.md](src-tauri/README.md)。

## 架构和安全边界

```
React + TypeScript UI (src/)
        |
        | typed Tauri 命令/事件，接口很窄
        v
Rust + Tauri core (src-tauri/src/)
├── file_ops/            路径与文件操作校验
├── transfer/            上传、下载、进度、取消
├── koofr_api/           Koofr REST API
├── credential_manager/  Windows 安全凭据存储
├── vault_core/          计划中的 Vault 兼容层
└── crypto/              计划中的加密支持
```

凭据、文件系统访问、网络请求这些不会放进前端。前端只调受限的 Tauri 命令，不碰密码、令牌或 Vault Safe Key。

## 开发约定

- 用 `npm ci` 和两个锁文件，别乱升级依赖。
- 编辑器配好 `.editorconfig`；文本文件一律 UTF-8 + LF。
- 提交前跑 `npm run check`。
- 不提交构建产物、依赖目录、账号凭据、访问令牌或长得像 secret 的测试数据。
- 加新 Tauri 命令时，Rust 侧必须验证路径、远程标识符和操作范围。
- 下载目录用户可以手填，但前端不能拼最终文件名。Rust 负责验证父目录、清理远端名称、避免覆盖、签发一次性下载授权。
- Vault 功能必须兼容 Koofr/rclone crypt，不能自己搞一套加密格式。

推送和版本号完全匹配的 `v*` 标签会触发 GitHub Actions：跑质量检查、构建 NSIS 安装程序、创建同名 GitHub Release。发布包目前不签名，Windows 可能显示未知发布者或 SmartScreen 警告。建议只从本仓库 Release 页面下载。版本规则和操作步骤见[发布流程](docs/RELEASING.md)。