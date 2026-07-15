# Frontend (React / TypeScript)

- `components/`: 应用外壳和可复用 UI 组件。
- `features/auth/`: 登录表单、客户端校验与安全提示。
- `features/files/`: Koofr 挂载点、目录状态、文件展示与操作。
- `features/transfers/`: 真实上传/下载传输队列界面。
- `services/`: 类型化 Tauri 命令与传输事件封装层。
- `types/`: 前端领域类型。

账户连接、启动会话检查和退出登录已通过 `services/koofr.ts` 接入 Rust 后端。应用
专用密码会在每次提交完成后从表单状态清除；只有用户勾选“保存密码”时，Rust 后端才会把它写入 Windows 凭据管理器。文件工作区会读取真实挂载点
和目录，并已接入本地文件选择、上传、下载、新建、重命名和删除。最近的文件、已共享、
回收站列表与恢复/清空操作也通过类型化后端命令读取真实账户数据。Vault Safe Key 不会
进入前端状态。

`features/settings/` 提供缓存模式、缓存有效期、清除缓存和删除已保存登录信息的界面。前端只读取是否存在保存的账户及邮箱，不会取回已保存的密码。
