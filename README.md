# Koofr-GUI
Koofr的GUI界面

## 项目骨架（仅结构，未实现功能代码）

```text
Koofr-GUI
├── src/                         # React / TypeScript：界面
│   ├── components/
│   ├── features/
│   ├── services/
│   └── types/
└── src-tauri/                   # Rust：本地文件、加密、下载上传
    └── src/
        ├── file_ops/            # 本地文件
        ├── crypto/              # 加密能力
        ├── transfer/            # 下载上传
        ├── koofr_api/           # Koofr REST API：普通文件访问
        ├── vault_core/          # vault-core：Vault 加解密
        └── credential_manager/  # Windows Credential Manager：保存凭据
```
