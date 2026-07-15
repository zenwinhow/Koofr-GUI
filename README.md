# Koofr-GUI

Windows 优先的 Koofr 桌面客户端。目前包含 React / TypeScript UI，以及首个
Rust / Tauri 普通文件后端切片。Vault 与持久化凭据管理尚未实现。

## 当前可用内容

- 可运行的 Tauri v2 + Vite + React + TypeScript 桌面工具链。
- “我的文件”主工作区：真实挂载点、目录导航、筛选、文件表格和传输面板。
- 新建文件夹、上传、下载、重命名、删除及可取消传输已接入 Rust 后端。
- 响应式桌面与窄屏布局。
- 五套可持久化的颜色主题，默认使用 Koofr 绿色。
- 登录页、启动会话检查、登录门禁与退出登录流程。
- Koofr 应用密码换取内存会话、挂载点与目录列表命令。
- 新建文件夹、重命名、移动、复制和删除命令。
- 可取消的流式上传/下载及脱敏进度事件。

登录页已接入 `src/services/koofr.ts`：应用密码只用于换取后端内存令牌，不会写入
应用配置或 WebView 存储，退出登录会清空会话。登录后会读取真实 Koofr 挂载点和目录；
文件名、容量、修改时间、大小及传输状态均来自后端。最近使用、共享、回收站、Vault、
移动和复制仍属于后续迭代。后端不会自动访问真实账户，只有用户显式登录或执行文件命令
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

基础文件后端的命令与安全边界见 `src-tauri/README.md`。Vault、OS 保护的凭据持久化
以及完整传输队列仍属于后续里程碑。
