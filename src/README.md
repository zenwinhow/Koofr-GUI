# Frontend (React / TypeScript)

- `components/`: 应用外壳和可复用 UI 组件。
- `features/auth/`: 登录表单、客户端校验与安全提示。
- `features/files/`: 文件工作区与演示数据。
- `features/transfers/`: 传输队列界面。
- `services/`: 类型化 Tauri 命令与传输事件封装层。
- `types/`: 前端领域类型。

账户连接、启动会话检查和退出登录已通过 `services/koofr.ts` 接入 Rust 后端。应用
专用密码不会持久化，并会在每次提交完成后从表单状态清除。文件工作区仍使用本地 React
演示数据；真实目录状态和本地文件选择 UI 尚未接线。Vault Safe Key 不会进入前端状态。
