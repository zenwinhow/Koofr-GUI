# 发布流程

Koofr-GUI 使用 GitHub Actions 在推送版本标签时构建、签名并发布 Windows NSIS 安装程序。工作流定义在 `.github/workflows/release.yml`。

## 一次性配置

在 GitHub 仓库的 Actions secrets 中创建以下机密：

- `WINDOWS_CERTIFICATE`：Base64 编码的 Windows 代码签名 `.pfx` 证书。
- `WINDOWS_CERTIFICATE_PASSWORD`：导出该 `.pfx` 时设置的密码。

工作流会把证书仅导入临时 GitHub Actions Runner 的当前用户证书存储，构建结束后删除临时 `.pfx` 文件。未配置这两个机密时，工作流会在发布前失败，绝不会上传未签名的正式安装程序。

## 发布一个版本

1. 将 `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 的版本统一改为相同的 SemVer 版本，例如 `1.0.1`。
2. 在 `CHANGELOG.md` 增加该版本的发布日期、变更与已知限制。
3. 在 Windows 环境执行 `npm ci` 与 `npm run check`。
4. 提交版本变更，然后创建并推送完全匹配的标签：

   ```powershell
   git tag v1.0.1
   git push origin v1.0.1
   ```

5. 在 GitHub Actions 中等待 `Publish Windows release` 成功。它会校验标签和三个版本源一致，再构建、签名 NSIS 安装程序并上传到同名 GitHub Release。

不要手动把未签名的构建产物上传到正式 Release。若签名、检查或构建任一环节失败，工作流不会创建发布资产。
