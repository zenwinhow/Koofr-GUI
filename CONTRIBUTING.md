# 贡献指南

感谢你愿意帮 Koofr-GUI 改进。本指南说明如何提交 Issue、发起 Pull Request，以及项目对代码、提交信息和测试的要求。

英文版：见文档末尾 [English version](#english-version)。

## 目录

- [行为准则](#行为准则)
- [先看这些](#先看这些)
- [报告 Bug](#报告-bug)
- [提出新功能](#提出新功能)
- [开发环境](#开发环境)
- [Pull Request 流程](#pull-request-流程)
- [代码风格](#代码风格)
- [测试](#测试)
- [提交信息约定](#提交信息约定)
- [文档](#文档)
- [不要提交的东西](#不要提交的东西)
- [License 与 DCO](#license-与-dco)
- [English version](#english-version)

## 行为准则

参与本项目请遵守常识：尊重他人、假定善意、聚焦技术。不接受骚扰、人身攻击、发布他人隐私。项目维护者保留删除评论、关闭 Issue / PR 或封禁账号的权利。

## 先看这些

在动手之前请确认：

1. 读过 [README.md](README.md) 和 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。
2. 搜过 [已有 Issue](https://github.com/zenwinhow/Koofr-GUI/issues) —— 你想报的问题可能已经在讨论了。
3. 涉及安全漏洞的**不要**开公开 Issue，走 [SECURITY.md](SECURITY.md) 的私密渠道。

## 报告 Bug

一个好的 Bug 报告至少包含：

- **Koofr-GUI 版本**（左下角 or `About` or 安装包文件名）
- **操作系统**（Windows 10 / 11，位数）
- **重现步骤**（越具体越好，最好能从"打开应用"开始）
- **实际结果 vs 期望结果**
- **错误消息 / 错误码**（应用返回的稳定错误码，不要贴含路径 / 令牌的完整日志）

如果涉及传输问题，附上：

- 传输类型（普通上传 / 分卷上传 / 单文件下载 / 文件夹下载）
- 文件大小量级（不需要精确数字）
- 网络中断 / 应用重启 / 磁盘满等触发条件

## 提出新功能

先开个 Issue 讨论，用 `[Feature]` 或 `[Proposal]` 前缀。说明：

- 要解决什么问题
- 至少一个你自己会用它的场景
- 有没有替代方案，为什么不选

**已经明确 Out-of-Scope**：

- 需要 Koofr 官方公开桌面客户端 API 才能做的东西（OAuth、第三方存储管理）
- 自定义加密格式（Vault 必须兼容 Koofr / rclone crypt）
- 网页 Cookie 复用、其他客户端的密钥内嵌

## 开发环境

见 [docs/BUILDING.md](docs/BUILDING.md)。快速版：

```powershell
git clone https://github.com/zenwinhow/Koofr-GUI.git
Set-Location Koofr-GUI
npm ci
npm run dev:desktop
```

## Pull Request 流程

1. **Fork + 建分支**。分支名用简短描述性名字，例如 `fix/split-upload-retry` 或 `feat/vault-unlock`。
2. **小步快跑**。一个 PR 只做一件事。多个逻辑独立的改动请拆成多个 PR。
3. **保持锁文件干净**。不要随手升级依赖 —— 有需要请单独开 PR 说明理由。
4. **本地跑通所有检查**：

   ```powershell
   npm run check
   ```

   这条命令依次跑 ESLint、Vitest、TypeScript + Vite 构建、`cargo fmt --check`、Clippy（警告当错误）、Rust 单元测试。必须**全绿**。

5. **写测试**。新增功能模块或修 Bug 时，尽量加对应的 `.test.ts` / `.test.tsx` 或 Rust `#[cfg(test)]`。
6. **更新文档**。改了用户可见的行为要更新 README；改了架构 / 安全约定要更新 [ARCHITECTURE.md](docs/ARCHITECTURE.md) 和 [CLAUDE.md](CLAUDE.md)。
7. **更新 CHANGELOG.md**。在 `## [Unreleased]` 段落下加一条，遵守 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) 格式。如果没有 `Unreleased` 段落，就新建一个。
8. **提 PR 到 `main`**。标题一行描述做了什么。描述里说明：
   - 动机 / 关联 Issue
   - 主要改动点
   - 手动测试过什么
   - 有没有破坏性变更

## 代码风格

### TypeScript / React

- **严格模式**（`tsconfig` 已启用）。不要用 `any`，需要时用 `unknown` + 类型收窄。
- **函数组件 + hooks**，不写 class 组件。
- **显式接口类型**放到组件文件顶部或 `types/` 里。
- 命名：组件 `PascalCase`，hooks `useXxx`，常量 `SCREAMING_SNAKE_CASE`，其余 `camelCase`。
- **错误处理走 helper**：`commandErrorMessage(error, fallback)` 取安全消息，`isCommandErrorCode(error, code)` 判错误码。不要直接打印或往 UI 里塞原始 error。
- ESLint 配置在 `eslint.config.js`，跑 `npm run lint`。

### Rust

- **edition 2024**，`rustfmt` 默认配置。
- **Clippy 警告当错误**（`-D warnings`）。
- **命名**：模块和文件 `snake_case`，类型 / trait `PascalCase`，函数 / 变量 `snake_case`。
- **错误类型统一走 `AppError`**（`src-tauri/src/error.rs`）。命令返回给前端的错误必须是稳定错误码 + 安全消息，**绝不能**包含路径、令牌、响应正文。
- **新增 Tauri 命令必须**：
  - 校验挂载点 ID、远程路径（拒绝 `.`、`..`、NUL、超长）
  - 校验本地路径（绝对、存在、非符号链接）
  - 限定操作范围（不能越权访问其他账户 / 目录）
  - 在 `commands.rs` 或对应模块里注册

### 提交前自检清单

- [ ] `npm run lint` 无错
- [ ] `npm run test` 全过
- [ ] `npm run check:rust` 全过
- [ ] 新增 / 修改的行为有测试覆盖
- [ ] 相关文档已同步更新
- [ ] CHANGELOG 已更新
- [ ] 没有引入 `console.log`、`dbg!`、临时调试代码

## 测试

- **前端**：Vitest + jsdom + Testing Library。测试文件与源文件同目录，命名 `*.test.ts` 或 `*.test.tsx`。
- **Rust**：单元测试用 `#[cfg(test)] mod tests`，或独立测试文件放在 `src-tauri/src/**/xxx_tests.rs`。
- 涉及网络请求的 Rust 测试尽量 mock；不要在测试里访问真实 Koofr 账户。
- 涉及本地文件系统的测试用 `tempfile` crate 之类的工具，别写死路径。

## 提交信息约定

我们不严格要求 Conventional Commits，但推荐用这个风格：

```
<type>: <subject>

<body>

<footer>
```

`type` 常用：`feat`、`fix`、`refactor`、`docs`、`test`、`chore`、`perf`、`build`。

例子：

```
fix(transfer): reset progress correctly after pause

Progress was previously reset to 0% when the user paused, because the
frontend read the pending checkpoint before the Rust side had time to
emit the paused progress event.

Closes #42
```

- 主语用英文或中文都可以，一致就行。
- 一行 subject 不超过 72 字符。
- 关联 Issue 用 `Closes #N` / `Refs #N`。

## 文档

哪些改动需要哪些文档同步：

| 改动 | 需要更新 |
| --- | --- |
| 用户可见的功能变化 | `README.md` + `README.en.md` + `CHANGELOG.md` |
| 新增 Tauri 命令 | `src-tauri/README.md` + `CLAUDE.md`（命令索引）|
| 架构、数据流、安全边界变化 | `docs/ARCHITECTURE.md` + `CLAUDE.md` |
| 构建流程变化 | `docs/BUILDING.md` |
| 发布流程变化 | `docs/RELEASING.md` |
| 设计 token / UI 约定变化 | `DESIGN.md` |

## 不要提交的东西

- `node_modules/`、`dist/`、`src-tauri/target/`、`src-tauri/gen/`、`*.tsbuildinfo`
- 真实的 Koofr 账号、密码、访问令牌、OAuth 密钥
- 本地路径、用户名、内网 IP 等 PII
- 未压缩的大二进制文件（截图请压缩）
- IDE 工作区文件（除非项目根 `.gitignore` 明确允许）

## License 与 DCO

- 本项目采用 [MIT License](LICENSE)。
- 提交 PR 即表示你同意你的贡献以同样的 MIT 协议发布。
- 你的代码必须是你自己写的、或者你有权以 MIT 授权。不要贴不明来源的代码。

---

## English version

Thanks for your interest in improving Koofr-GUI. Short version of the rules:

1. **Search issues first**, don't file duplicates.
2. **Security bugs go through [SECURITY.md](SECURITY.md), never public issues.**
3. **Fork → branch → PR to `main`.** One logical change per PR.
4. **`npm run check` must pass** before requesting review — it runs lint, Vitest, TypeScript + Vite build, `cargo fmt --check`, Clippy (warnings as errors), and Rust unit tests.
5. **Every new Tauri command must validate** mount IDs, remote paths (reject `.`, `..`, NUL, oversize), local paths (absolute, existing, non-symlink), and operation scope on the Rust side.
6. **Error messages returned to the frontend must not leak paths, tokens, or response bodies** — use `AppError` and stable error codes.
7. **Update docs** (README / ARCHITECTURE / CHANGELOG) for user-visible or architectural changes.
8. **No auto dep bumps** in feature PRs — open a separate PR for dependency updates.
9. **License**: contributions are released under [MIT](LICENSE). By opening a PR you certify you have the right to license your work under MIT.

Full rules above (in Chinese).
