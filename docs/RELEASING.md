# 发布流程

Koofr-GUI 用 GitHub Actions 在推送版本标签时构建和发布 Windows NSIS 安装包。工作流在 `.github/workflows/release.yml`。

## 发布策略

公开 Release 不签名，也不需要 GitHub Actions secrets。用户从浏览器下载时，Windows 可能显示未知发布者或 SmartScreen 警告。只从本仓库的 GitHub Release 页面拿安装包，自己确认一下仓库地址和发布标签。

## 发一个版本

1. 把 `package.json`、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 里的版本改成同一个 SemVer，比如 `1.0.1`。
2. 在 `CHANGELOG.md` 里加上这个版本的发布日期、变更和已知限制。
3. 在 Windows 上跑 `npm ci` 和 `npm run verify:full`。
4. 提交版本变更，然后打标签推送：

   ```powershell
   git tag v1.0.1
   git push origin v1.0.1
   ```

5. 去 GitHub Actions 看 `Publish Windows release` 跑没跑成功。它会校验标签和三个版本源一致，然后构建未签名的 NSIS 安装包，传到同名的 GitHub Release 上。

不要从第三方镜像或转发链接下载安装包。如果检查或构建失败了，工作流不会上传发布资产。