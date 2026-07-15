# Koofr-GUI

Windows 优先的 Koofr 桌面客户端。目前包含 React / TypeScript UI，以及首个
Rust / Tauri 普通文件后端切片。Vault 尚未实现；应用专用密码可选择保存到 Windows 凭据管理器。

## 当前可用内容

- 可运行的 Tauri v2 + Vite + React + TypeScript 桌面工具链。
- “我的文件”主工作区：真实挂载点、目录导航、筛选、文件表格和传输面板。
- 新建文件夹、上传、下载、重命名、删除及可取消传输已接入 Rust 后端。
- 最近的文件、已共享和回收站均读取真实 Koofr 数据；回收站支持恢复、恢复全部和确认后永久清空。
- 响应式桌面与窄屏布局。
- 五套可持久化的颜色主题，默认使用 Koofr 绿色。
- 登录页、启动会话检查、登录门禁与退出登录流程。
- 可选的 Windows 凭据管理器密码保存与启动时自动连接；密码不会进入普通配置或 WebView 存储。
- 设置界面与带有效期的文件元数据缓存，支持关闭、仅内存和本地磁盘三种模式。
- Koofr 应用密码换取内存会话、挂载点与目录列表命令。
- 新建文件夹、重命名、移动、复制和删除命令。
- 可取消的流式上传/下载及脱敏进度事件。

登录页已接入 `src/services/koofr.ts`：应用密码只用于换取后端内存令牌；只有用户明确
勾选“保存密码”时才会写入 Windows 凭据管理器，不会写入应用配置或 WebView 存储。登录后会读取真实 Koofr 挂载点和目录；
文件名、容量、修改时间、大小及传输状态均来自后端。Vault 以及移动、复制的前端交互
仍属于后续迭代。后端不会自动访问真实账户，只有用户显式登录或执行文件命令
时才会发起请求。

## 本地运行

```powershell
npm install
npm run dev
```

默认地址为 `http://127.0.0.1:1420/`。

运行桌面应用：

```powershell
npm run dev:desktop
```

Windows 构建需要 Rust MSVC 工具链、Visual Studio C++ Build Tools、Windows SDK
和 WebView2。

## 检查

```powershell
npm run check
```

该命令运行 ESLint、TypeScript/Vite 构建、Rust 格式检查、Clippy 和 Rust 测试。

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

基础文件后端的命令与安全边界见 `src-tauri/README.md`。Vault 与完整传输队列仍属于后续里程碑。
