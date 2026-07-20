<div align="center">

# Koofr-GUI

**Windows 优先的 Koofr 桌面文件管理客户端**

用 Tauri v2 + React + TypeScript + Rust 构建。想做成原生文件管理器那种感觉，而不是套个网页壳子完事。

[![Release](https://img.shields.io/github/v/release/zenwinhow/Koofr-GUI?include_prereleases&sort=semver)](https://github.com/zenwinhow/Koofr-GUI/releases)
[![CI](https://img.shields.io/github/actions/workflow/status/zenwinhow/Koofr-GUI/release.yml?branch=main&label=release)](https://github.com/zenwinhow/Koofr-GUI/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6?logo=windows)](#系统要求)
[![Tauri](https://img.shields.io/badge/Tauri-v2-24C8DB?logo=tauri)](https://v2.tauri.app/)

[English](README.en.md) · [下载安装包](https://github.com/zenwinhow/Koofr-GUI/releases) · [构建指南](docs/BUILDING.md) · [架构说明](docs/ARCHITECTURE.md) · [更新日志](CHANGELOG.md)

</div>

---

## 目录

- [Koofr-GUI](#koofr-gui)
  - [目录](#目录)
  - [简介](#简介)
  - [截图](#截图)
  - [功能](#功能)
    - [已经能用](#已经能用)
    - [还没做](#还没做)
  - [下载与安装](#下载与安装)
  - [系统要求](#系统要求)
    - [运行](#运行)
    - [从源码构建](#从源码构建)
  - [从源码构建](#从源码构建-1)
  - [常用命令](#常用命令)
  - [架构与安全边界](#架构与安全边界)
  - [数据存储](#数据存储)
  - [路线图](#路线图)
  - [参与贡献](#参与贡献)
  - [报告安全问题](#报告安全问题)
  - [许可证](#许可证)
  - [致谢](#致谢)

## 简介

[Koofr](https://koofr.eu) 是一家欧洲的云存储服务商，支持把 Google Drive、OneDrive、Dropbox 等第三方存储聚合到同一个账户。官方网页端在中国大陆网络下体验不够顺畅，也没有正式的 Windows 桌面客户端。

Koofr-GUI 尝试解决这个问题：一个体积小、原生感强、支持大文件续传、不用把令牌塞进 WebView 的桌面客户端。当前版本 **1.3.3** 已实现普通 Koofr 文件管理、可续传传输、分享链接管理等主要功能。Koofr Vault（端到端加密）尚未实现。

## 截图

> 主界面截图待补。当前设计目标：1280×720，最小 980×640，符合 [DESIGN.md](DESIGN.md) 里定义的设计系统。

## 功能

### 已经能用

- **登录与凭据**
  - 用 Koofr 应用专用密码登录，下次启动自动恢复会话，也能一键退出登录。
  - 密码可以选择性地存到 Windows 凭据管理器里，不写到普通配置文件，也不塞进 WebView 存储。

- **文件浏览与操作**
  - 浏览挂载点、目录、最近文件、共享内容和回收站。
  - 识别账户里的 Koofr、Google Drive、OneDrive、Dropbox 等已连接存储。
  - 新建文件夹、上传、下载、重命名、移动、复制、删除、从回收站恢复。
  - 分享链接：查、建、撤下载链接和接收文件链接，撤销时二次确认。

- **传输**
  - 传输队列面板，实时进度、可取消、可暂停。
  - **单文件下载**：HTTP Range + 磁盘检查点，字节级续传。
  - **分卷上传**：把大文件切成用户可配置大小的 `part-*.bin`，断点后从最后一个已确认完整的分卷继续。远端表现为一个用户命名的普通文件夹，可以直接用 Windows `copy /b` 或 POSIX `cat` 拼回原文件。
  - **递归文件夹下载**：使用临时目录暂存，失败或取消时清理整个暂存树。

- **设置**
  - 默认下载文件夹可配置，每次下载可选“询问位置”。
  - 可选择在 `network_error` 后自动恢复传输；最大重试次数和固定重试间隔均可设置，次数可设为无限，等待期间仍可暂停或取消。
  - 元数据缓存可选内存 / 磁盘 / 关闭，磁盘缓存文件夹可自定义。
  - 诊断日志文件夹、级别、保留时间和单文件大小可配置，支持查看占用与一键清理。
  - 5 套主题（koofr / ocean / iris / coral / berry），只覆盖 accent 色。

### 还没做

- Koofr Vault 解锁、加解密、Vault 传输（`src-tauri/src/crypto/`、`vault_core/` 目录预留）。
- OAuth 登录和第三方存储的新增 / 移除 / 重授权。当前只能引导用户到 Koofr 官方账户页面管理，等 Koofr 提供公版桌面客户端注册信息和公开授权 API。
- macOS / Linux 支持。

> **为什么普通上传不能续传？** Koofr 公开上传接口只有整文件 `FilesPut`，不提供分块会话或服务端偏移确认，所以普通上传断了必须重传整个文件。分卷上传是明确选出来的互操作方案，详见 [ARCHITECTURE.md](docs/ARCHITECTURE.md#分卷上传设计)。

## 下载与安装

推荐从 [Releases 页面](https://github.com/zenwinhow/Koofr-GUI/releases) 下载最新的 NSIS 安装包（`Koofr-GUI_x.y.z_x64-setup.exe`）。

> ⚠️ **未签名警告**：安装包目前不做代码签名。Windows SmartScreen 可能显示"未知发布者"警告。**请只从本仓库的 Releases 页面下载**，不要从第三方镜像获取。校验方式详见 [RELEASING.md](docs/RELEASING.md)。

首次运行需要 Microsoft Edge WebView2 Runtime。受支持的 Windows 10 / 11 通常自带；如果缺失，安装包会引导下载。

## 系统要求

### 运行

- Windows 10 或 11（x64）
- Microsoft Edge WebView2 Runtime（通常自带）
- Koofr 账户 + [应用专用密码](https://app.koofr.net/app-password)

### 从源码构建

- Node.js **24 LTS**（推荐；22.12 及以上 22.x LTS 也可以）
- npm **10+**
- Rust **1.88+**，`x86_64-pc-windows-msvc` 工具链
- Visual Studio 2022 Build Tools 的"使用 C++ 的桌面开发"工作负载 + Windows SDK

完整安装步骤见 [BUILDING.md](docs/BUILDING.md#1-环境要求)。

## 从源码构建

```powershell
git clone https://github.com/zenwinhow/Koofr-GUI.git
Set-Location Koofr-GUI
npm ci
npm run dev:desktop          # 开发模式：完整桌面应用
```

只跑前端开发服务器（浏览器里看，测不了 Tauri 命令）：

```powershell
npm run dev
```

发布构建：

```powershell
npm run check                # 全套质量检查：lint + 测试 + Rust fmt/clippy
npm run build:desktop        # 生成 src-tauri/target/release/koofr-gui.exe
npm run build:installer      # 显式生成 NSIS 安装包
```

详细说明和常见问题见 [BUILDING.md](docs/BUILDING.md)。

## 常用命令

| 命令 | 用途 |
| --- | --- |
| `npm ci` | 按 `package-lock.json` 装依赖，可复现 |
| `npm run dev` | 启动 Vite 前端开发服务器（仅浏览器） |
| `npm run dev:desktop` | 启动完整 Tauri 桌面开发环境 |
| `npm run build` | 类型检查 + 生成 `dist/` 前端资源 |
| `npm run build:desktop` | 构建 Windows 发布版 exe（不生成安装包） |
| `npm run build:installer` | 显式构建 NSIS 安装包（发布流程用） |
| `npm run verify:quick` | 快速验证：lint + 拆分上传测试 + 前端构建 |
| `npm run check` | 完整验证：lint + 测试 + 前端构建 + Rust fmt + clippy + Rust 测试 |
| `npm run clean` | 清理构建产物，保留 `node_modules/` |
| `npm run clean:all` | 清理构建产物 + `node_modules/`（需要重新 `npm ci`） |

## 架构与安全边界

```
┌──────────────────────────────────────────────┐
│ React + TypeScript UI (src/)                 │
│ - 只调用受限的 Tauri 命令                    │
│ - 不接触密码、令牌、Vault Safe Key           │
└──────────────┬───────────────────────────────┘
               │ typed Tauri commands + events
               ▼
┌──────────────────────────────────────────────┐
│ Rust + Tauri core (src-tauri/src/)           │
│ ├─ file_ops/           路径与操作校验         │
│ ├─ transfer/           上传/下载/续传/进度    │
│ ├─ koofr_api/          Koofr REST 客户端      │
│ ├─ credential_manager  Windows 凭据管理器     │
│ ├─ metadata_cache      内存/磁盘缓存          │
│ ├─ crypto/             预留（Vault，未实现）  │
│ └─ vault_core/         预留（Vault，未实现）  │
└──────────────────────────────────────────────┘
```

关键规则：

- **凭据不出 Rust 边界**：会话令牌只在内存里；应用专用密码可选存入 Windows 凭据管理器。
- **路径必须规范**：远程路径拒绝 `.`、`..`、NUL、超长；本地下载目录必须绝对、存在、非符号链接。
- **不覆盖现有文件**：单文件下载先写入 `.koofr-part-*` 临时文件，同级重名自动加序号。
- **错误消息不泄露路径 / 令牌 / 响应正文**，只返回稳定错误码 + 安全消息。

完整设计和数据流见 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。

## 数据存储

应用数据保存在 Windows 用户目录下（`identifier = net.koofr.desktop.gui`）：

```
%LOCALAPPDATA%\net.koofr.desktop.gui\
├─ settings.json                # 应用设置
├─ transfer-checkpoints.json    # 传输恢复检查点
├─ cache/metadata-cache.json    # 默认元数据缓存位置（可配置）
└─ logs/koofr-gui*.jsonl       # 默认脱敏诊断日志位置（可配置）
```

**凭据不在这里**——Koofr 应用密码通过 Windows 凭据管理器保存，重装 / 重构建应用后仍然存在。

## 路线图

- [ ] Koofr Vault 解锁、加解密、Vault 传输（兼容 rclone crypt）
- [ ] OAuth 登录 & 第三方存储管理（等待 Koofr 公版桌面客户端 API）
- [ ] 代码签名 / SmartScreen 白名单
- [ ] macOS 支持
- [ ] Linux 支持

## 参与贡献

欢迎 Issue 和 PR。提交前请：

1. 读一下 [CONTRIBUTING.md](CONTRIBUTING.md)。
2. 跑一遍 `npm run check`，确保 lint、测试、Rust fmt / clippy 全部过。
3. 新增 Tauri 命令时，Rust 侧必须校验路径、标识符和操作范围。

## 报告安全问题

**不要**通过公开 Issue 上报漏洞。请参照 [SECURITY.md](SECURITY.md) 里的私密渠道。

## 许可证

[MIT](LICENSE) © 2026 Koofr-GUI 贡献者

本项目与 Koofr d.o.o. 无关联。"Koofr" 是 [Koofr d.o.o.](https://koofr.eu) 的商标。

## 致谢

- [Koofr](https://koofr.eu) 提供的云存储服务和 [Go 客户端](https://github.com/koofr/go-koofrclient) / [Java SDK](https://github.com/koofr/java-koofr) 参考实现。
- [Tauri](https://tauri.app) 团队的桌面框架。
- [rclone](https://rclone.org) 的 Koofr 后端和 crypt 格式，为分卷上传和后续 Vault 兼容性提供参考。
- [Lucide](https://lucide.dev) 的图标集。
