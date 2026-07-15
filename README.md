# Koofr-GUI

Windows 优先的 Koofr 桌面客户端。目前已开始实现 React / TypeScript UI，
Koofr API、Vault、凭据管理、文件操作与传输后端仍未接入。

## 当前可用内容

- 可运行的 Vite + React + TypeScript 前端工具链。
- “我的文件”主工作区：导航、搜索、文件表格、选择工具栏、传输面板。
- 本地演示交互：文件筛选、行选择、新建文件夹、上传边界提示、Vault 锁定提示。
- 响应式桌面与窄屏布局。
- 五套可持久化的颜色主题，默认使用 Koofr 绿色。

演示数据与交互只存在于浏览器内存中，不会访问真实 Koofr 账户或本地文件。

## 本地运行

```powershell
npm install
npm run dev
```

默认地址为 `http://127.0.0.1:1420/`。

## 检查

```powershell
npm run check
```

该命令运行 ESLint、TypeScript 构建检查和 Vite 生产构建。

## 计划架构

```text
React + TypeScript UI (`src/`)
        |
        | typed, narrowly scoped Tauri commands/events
        v
Rust + Tauri core (`src-tauri/src/`)
        |-- file_ops/
        |-- transfer/
        |-- koofr_api/
        |-- vault_core/
        |-- crypto/
        `-- credential_manager/
```

`src-tauri/` 目前仍是目录骨架，没有 Rust 入口、Cargo 清单或 Tauri 配置。
