# 安全策略

## 目录

- [支持的版本](#支持的版本)
- [报告漏洞](#报告漏洞)
- [响应时间](#响应时间)
- [披露策略](#披露策略)
- [安全设计边界](#安全设计边界)
- [已知的非漏洞行为](#已知的非漏洞行为)
- [English version](#english-version)

## 支持的版本

只对最新的稳定版本提供安全修复：

| 版本 | 是否支持 |
| --- | --- |
| 1.3.x | ✅ |
| < 1.3.0 | ❌ 请升级到最新版 |

发布包**未做代码签名**。请只从 [官方 Releases 页面](https://github.com/zenwinhow/Koofr-GUI/releases) 下载。

## 报告漏洞

**请不要通过公开 Issue 报告漏洞。**

请通过以下任一私密渠道联系：

- **GitHub Security Advisory（推荐）**：<https://github.com/zenwinhow/Koofr-GUI/security/advisories/new>
- **邮件**：[zenwinhow@users.noreply.github.com](mailto:zenwinhow@users.noreply.github.com)（可选 GPG 加密，公钥指纹在 GitHub Profile）

报告请尽量包含：

- 影响的 Koofr-GUI 版本 / commit
- 受影响的组件（前端 / Rust / 打包 / 分发）
- 重现步骤（最好带最小复现 demo）
- 潜在影响（本地权限提升、数据泄露、传输被劫持等）
- 建议的修复方向（可选）

**请不要**在报告里包含：

- 你的真实 Koofr 账号密码或令牌
- 他人的 PII
- 生产环境中的敏感文件

## 响应时间

因为这是个业余维护的开源项目，响应时间只能尽力而为：

| 阶段 | 目标 |
| --- | --- |
| 收到确认 | 5 个工作日内 |
| 初步评估（是否属于漏洞、严重程度） | 10 个工作日内 |
| 修复 / 缓解 | 视严重程度，Critical / High 尽量在 30 天内发布补丁 |

如果超过预期时间没有回复，请通过备用渠道再联系一次。

## 披露策略

采用 **协调披露**（Coordinated Disclosure）：

1. 收到报告后我们私下确认和评估。
2. 如果确认是漏洞，一起商议披露时间线，一般 **90 天** 内发布修复。
3. 修复发布后，在 CHANGELOG 和 GitHub Security Advisory 里公开细节，如果报告者同意会致谢。
4. 报告者请在**修复发布前**不要公开细节。

## 安全设计边界

Koofr-GUI 的信任模型：

```
┌─────────────────────────────────────────────────────┐
│ 用户（拥有 Koofr 账号 + 本机 Windows 用户会话）      │
└────────────────────┬────────────────────────────────┘
                     │ WebView2 UI
                     ▼
┌─────────────────────────────────────────────────────┐
│ React + TypeScript (受信任但只调受限命令)            │
│ - 无密码、无令牌、无 Vault Safe Key                  │
│ - 无 XHR / fetch 直连 koofr.net                     │
└────────────────────┬────────────────────────────────┘
                     │ typed Tauri IPC
                     ▼
┌─────────────────────────────────────────────────────┐
│ Rust core (信任边界)                                 │
│ - Koofr HTTP 请求、凭据管理、路径校验                │
│ - Windows Credential Manager                        │
│ - 磁盘 IO                                            │
└─────────────────────────────────────────────────────┘
```

我们**在意**这些威胁：

- **前端逃逸**：WebView / 前端注入代码尝试读取密码、令牌或直接调用 Koofr API。
- **路径穿越**：前端传入恶意路径试图读写沙箱外的文件。
- **凭据泄露**：日志、错误消息、事件、崩溃报告里意外携带密码 / 令牌 / 本地路径 / 远程路径 / 服务端响应正文。
- **传输覆盖**：下载覆盖用户已有文件。
- **不完整传输被当成完整**：分卷上传 / 断点续传的校验被绕过。
- **本机其他账户读取凭据**：Windows Credential Manager 的作用域是当前用户，跨用户读取需要提权，但我们不主动扩大权限。

我们**目前不在意**这些（本机 Windows 用户已被视为可信）：

- 同一 Windows 用户下运行的其他程序读取 `%LOCALAPPDATA%\net.koofr.desktop.gui\` 里的**非凭据**文件（设置、检查点、缓存）。这些文件里没有密码、令牌或文件内容。
- 有本机管理员权限的攻击者读取 Windows Credential Manager 里的密码。这个是操作系统层面的信任模型，超出应用范围。
- 用户主动分享自己账户的 refresh_token 或 session token。

## 已知的非漏洞行为

以下情况**不视为漏洞**，请不要提交为安全报告：

- **安装包未签名 / SmartScreen 警告**：已在 README 和 RELEASING.md 明确记录。不签名是有意的成本 / 收益权衡。
- **`.koofr-part-*` 临时文件被本机同用户程序读取**：临时文件在用户下载目录里，Windows 文件权限本身已足够；下载完成后重命名到最终文件。
- **Koofr 会话令牌保存在 Rust 进程内存里**：进程内存对同一 Windows 用户的调试器可见，这是操作系统信任模型。
- **前端可以看到当前挂载点 / 目录列表 / 传输进度**：这些是 UI 必需的、前端本来就有权看的信息。
- **应用不做证书固定（cert pinning）**：依赖系统 CA 存储 + rustls 的默认校验。如果 Koofr 更换证书链、或用户系统 CA 被本人操作污染，均属预期行为。

## English version

**Do not open public issues for security vulnerabilities.** Use one of the private channels:

- **GitHub Security Advisory (preferred)**: <https://github.com/zenwinhow/Koofr-GUI/security/advisories/new>
- **Email**: [zenwinhow@users.noreply.github.com](mailto:zenwinhow@users.noreply.github.com)

Supported: 1.3.x only. Older versions must upgrade to receive fixes.

Response targets (best-effort, hobby-maintained project):

- Acknowledgement within 5 business days
- Triage within 10 business days
- Critical / High fixes within 30 days when feasible

Coordinated disclosure with a 90-day default embargo. Please keep details private until the fix ships. Credit given on request in CHANGELOG and GitHub Security Advisory.

**In scope**: frontend escape, path traversal, credential leaks in logs / error messages / events / crash reports, overwrite of user files, bypassed integrity checks in split upload or byte resume.

**Out of scope**: unsigned binaries (documented), same-user local processes reading non-credential state, admin-level attackers on the same machine, user willingly sharing their own tokens, no certificate pinning.

Full policy above (in Chinese).
