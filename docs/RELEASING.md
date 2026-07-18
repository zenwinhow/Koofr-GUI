# 发布流程

Koofr-GUI 使用 GitHub Actions 在推送版本标签时构建并发布 Windows NSIS 安装程序。工作流定义在 `.github/workflows/release.yml`。

## 发布策略

公开 Release 不使用代码签名证书，也不需要 GitHub Actions secrets。用户从浏览器下载时，Windows 可能显示未知发布者或 SmartScreen 警告；只应从本仓库的 GitHub Release 页面获取安装程序，并自行确认仓库地址和发布标签。

## 发布一个版本

1. 将 `package.json`、`src-tauri/Cargo.toml` 与 `src-tauri/tauri.conf.json` 的版本统一改为相同的 SemVer 版本，例如 `1.0.1`。
2. 在 `CHANGELOG.md` 增加该版本的发布日期、变更与已知限制。
3. 在 Windows 环境执行 `npm ci` 与 `npm run verify:full`。
4. 提交版本变更，然后创建并推送完全匹配的标签：

   ```powershell
   git tag v1.0.1
   git push origin v1.0.1
   ```

5. 在 GitHub Actions 中等待 `Publish Windows release` 成功。它会校验标签和三个版本源一致，再构建未签名的 NSIS 安装程序并上传到同名 GitHub Release。

不要从第三方镜像或转发链接下载安装程序。若检查或构建失败，工作流不会上传发布资产。
