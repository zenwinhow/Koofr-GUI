# 发布流程

Koofr-GUI 用 GitHub Actions 在推送版本标签时构建和发布 Windows NSIS 安装包。工作流在 [`.github/workflows/release.yml`](../.github/workflows/release.yml)。

## 目录

- [发布策略](#发布策略)
- [版本号规则](#版本号规则)
- [发一个版本](#发一个版本)
- [手动本地发布](#手动本地发布)
- [用户如何校验安装包](#用户如何校验安装包)
- [紧急撤回一个版本](#紧急撤回一个版本)

## 发布策略

- **平台**：只发 Windows x64 NSIS 安装包。
- **签名**：**不做代码签名**。用户下载后 Windows SmartScreen 可能显示"未知发布者"警告。这是有意的成本 / 收益权衡，详见 [SECURITY.md](../SECURITY.md#已知的非漏洞行为)。
- **触发**：推送符合 `v*` 的 Git 标签自动触发工作流。
- **Secrets**：不需要任何 GitHub Actions secrets（不签名、不推包管理器）。
- **发布渠道**：**只**通过本仓库的 [GitHub Releases 页面](https://github.com/zenwinhow/Koofr-GUI/releases)。不使用镜像、CDN、第三方包管理器。

## 版本号规则

遵守 [SemVer 2.0](https://semver.org/spec/v2.0.0.html)：

- **MAJOR**：破坏性变更（API、命令签名、检查点格式、数据布局不兼容）。
- **MINOR**：新增功能，向后兼容。
- **PATCH**：Bug 修复，向后兼容。

**四个地方**的版本号必须完全一致：

| 位置 | 字段 |
| --- | --- |
| `package.json` | `version` |
| `src-tauri/Cargo.toml` | `[package].version` |
| `src-tauri/tauri.conf.json` | `version` |
| Git tag | `v<version>`（例如 `v1.3.3`） |

工作流会校验 tag 和前三处版本完全一致，不一致则拒绝构建。

## 发一个版本

### 1. 更新版本号

假设要发 `1.3.4`：

```powershell
# package.json
# src-tauri/Cargo.toml
# src-tauri/tauri.conf.json
# 三个文件的 version 全改成 1.3.4
```

（可以用编辑器全局搜索替换，注意别改到 `Cargo.lock` —— 那个 cargo 会自动更新。）

### 2. 更新 CHANGELOG

在 [`CHANGELOG.md`](../CHANGELOG.md) 里加一段：

```markdown
## [1.3.4] - 2026-07-20

### Changed
- xxx

### Fixed
- yyy

### Security
- zzz（有则加，无则删）
```

日期用 `YYYY-MM-DD`（发布日期，不是提交日期）。格式遵守 [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)。

### 3. 本地质量检查

```powershell
npm ci
npm run verify:full
```

（`verify:full` 等价于 `npm run check`。）

### 4. 提交 + 打标签

```powershell
git add package.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json CHANGELOG.md
git commit -m "Prepare 1.3.4 release"
git tag v1.3.4
git push origin main
git push origin v1.3.4
```

### 5. 观察 GitHub Actions

去 [Actions 页面](https://github.com/zenwinhow/Koofr-GUI/actions) 看 `Publish Windows release` 工作流。它会：

1. 校验 tag 和三个版本源一致。
2. 装依赖 + 跑全套质量检查（等同 `npm run check`）。
3. 构建未签名的 NSIS 安装包。
4. 传到同名的 GitHub Release 上。

如果检查或构建失败，工作流不会上传发布资产。修复问题后：

- **如果 tag 还没被下载过**：可以强制删 tag 重新打（`git tag -d v1.3.4 && git push origin :refs/tags/v1.3.4`）。但一般不推荐，因为破坏了不可变性。
- **建议**：直接发一个 patch 版本（`1.3.5`）。

### 6. 编辑 Release 说明

工作流会创建 Release 但不填详细说明。手动到 Release 页面：

- 把 CHANGELOG 里对应版本段落复制到 Release 描述里。
- 顶部加一段"下载校验方式"，指向 [用户如何校验安装包](#用户如何校验安装包)。
- 如果是 Pre-release，勾"Set as a pre-release"。

## 手动本地发布

一般不用。如果 CI 挂了、又想赶紧发一个内测包给自己或少数人测：

仓库里有一个 `scripts/release-local.ps1` 脚本（如果存在），会在本地跑一次干净构建、生成安装包，但**不提交、不打 tag、不推 Release**。产出物在 `src-tauri/target/release/bundle/nsis/`。

发出去前想清楚：本地构建的包和 CI 构建的可能不完全一致（rustc / Node 版本、依赖锁文件、环境变量）。正式版本必须走 CI。

## 用户如何校验安装包

因为不做代码签名，用户下载后至少应该：

1. **确认下载来源** = `https://github.com/zenwinhow/Koofr-GUI/releases`（注意仓库地址和 owner）。
2. **确认 tag 和文件名一致**：`Koofr-GUI_1.3.4_x64-setup.exe` 对应 tag `v1.3.4`。
3. **对照 CHANGELOG 里的版本段落**确认功能符合预期。
4. **可选**：校验 SHA-256。工作流在 Release Assets 里附带 `SHA256SUMS.txt`（如果没附带，说明工作流版本还没做这一步，未来会加）。用 PowerShell 校验：

   ```powershell
   Get-FileHash Koofr-GUI_1.3.4_x64-setup.exe -Algorithm SHA256
   ```

   把输出的 hash 和 `SHA256SUMS.txt` 里对应文件的 hash 比对。

如果任何一步不对，**不要运行安装包**，请到 Issues 里反馈。

## 紧急撤回一个版本

如果发出去的版本有严重问题（数据损坏、凭据泄露之类的）：

1. **立即**在对应 Release 顶部加显眼警告："⚠️ 已撤回，不要使用此版本"。
2. 在 Release Assets 上加 `.YANKED` 后缀 rename，或者直接删除 Assets。
3. 在 CHANGELOG 里对应版本下加 `### Yanked` 段落，写清楚撤回原因。
4. 尽快发一个修复版本（PATCH，或者必要时 MINOR）。
5. 如果是安全问题，走 [SECURITY.md](../SECURITY.md) 的 Coordinated Disclosure 流程发 Security Advisory。
6. **不要 `git tag -d` 已发布的 tag** —— tag 保持存在但不再是"当前推荐版本"。

被撤回的 Release 不删除，只做标记 —— 这样已经下载了的用户还能识别自己拿到的是哪个版本。
