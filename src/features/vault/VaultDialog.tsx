import {
  ArrowLeft,
  Copy,
  Download,
  File,
  FileKey2,
  Folder,
  FolderInput,
  FolderPlus,
  KeyRound,
  Lock,
  Pencil,
  RefreshCw,
  ShieldPlus,
  Trash2,
  Upload,
} from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Modal } from '../../components/Modal'
import { commandErrorMessage, isCommandErrorCode, koofr } from '../../services/koofr'
import type { VaultDirectory, VaultEntry, VaultSummary } from '../../types/backend'
import type { VaultTransferSpec } from '../../types/vault'

interface VaultDialogProps {
  readonly mountId: string
  readonly parentPath: string
  readonly onClose: () => void
  readonly onNotice: (message: string) => void
  readonly onTransferStarted: (transfer: VaultTransferSpec) => void
}

interface Relocation {
  readonly entry: VaultEntry
  readonly isMove: boolean
}

const DATE_FORMAT = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
})

function formatBytes(value: number) {
  if (!Number.isFinite(value) || value < 0) return '—'
  if (value === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const index = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1)
  const amount = value / (1024 ** index)
  return `${amount.toFixed(index === 0 || amount >= 100 ? 0 : amount >= 10 ? 1 : 2)} ${units[index]}`
}

function formatModified(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '—'
  const milliseconds = value < 1_000_000_000_000 ? value * 1000 : value
  const date = new Date(milliseconds)
  return Number.isNaN(date.getTime()) ? '—' : DATE_FORMAT.format(date)
}

export function VaultDialog({
  mountId,
  parentPath,
  onClose,
  onNotice,
  onTransferStarted,
}: VaultDialogProps) {
  const [vaults, setVaults] = useState<VaultSummary[]>([])
  const [directory, setDirectory] = useState<VaultDirectory | null>(null)
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set())
  const [loading, setLoading] = useState(true)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [createName, setCreateName] = useState('')
  const [folderName, setFolderName] = useState('')
  const [renameName, setRenameName] = useState('')
  const [removeRepoId, setRemoveRepoId] = useState<string | null>(null)
  const [removeConfirmation, setRemoveConfirmation] = useState('')
  const [relocation, setRelocation] = useState<Relocation | null>(null)
  const directoryRef = useRef<VaultDirectory | null>(null)

  useEffect(() => {
    directoryRef.current = directory
  }, [directory])

  const reportError = (cause: unknown, fallback: string) => {
    if (!isCommandErrorCode(cause, 'cancelled')) {
      setError(commandErrorMessage(cause, fallback))
    }
  }

  const loadVaults = async () => {
    setLoading(true)
    setError('')
    try {
      const next = await koofr.listVaults()
      setVaults(next)
      if (directory && !next.some((vault) => vault.id === directory.repoId && !vault.locked)) {
        setDirectory(null)
      }
    } catch (cause) {
      reportError(cause, '无法读取保险箱列表。')
    } finally {
      setLoading(false)
    }
  }

  useEffect(() => {
    void loadVaults()
    // Initial load only. Later refreshes are explicit to avoid replacing active opaque handles.
  }, [])

  const openDirectory = async (repoId: string, directoryId: string) => {
    setLoading(true)
    setError('')
    try {
      const next = await koofr.listVaultFiles(repoId, directoryId)
      setDirectory(next)
      setSelectedIds(new Set())
    } catch (cause) {
      if (isCommandErrorCode(cause, 'vault_locked')) {
        setDirectory(null)
        await loadVaults()
      }
      reportError(cause, '无法读取保险箱内容。')
    } finally {
      setLoading(false)
    }
  }

  const run = async (operation: () => Promise<void>, fallback: string) => {
    setBusy(true)
    setError('')
    try {
      await operation()
    } catch (cause) {
      reportError(cause, fallback)
    } finally {
      setBusy(false)
    }
  }

  const unlock = (repoId: string) => run(async () => {
    await koofr.unlockVault(repoId)
    await openDirectory(repoId, 'root')
    setVaults(await koofr.listVaults())
  }, '无法解锁保险箱。')

  const lock = (repoId: string) => run(async () => {
    setVaults(await koofr.lockVault(repoId))
    setDirectory(null)
    setRelocation(null)
  }, '无法锁定保险箱。')

  const createVault = () => run(async () => {
    const name = createName.trim()
    if (!mountId || !name) return
    const created = await koofr.createVault(mountId, parentPath, name)
    setCreateName('')
    setVaults(await koofr.listVaults())
    await openDirectory(created.id, 'root')
  }, '无法创建保险箱。')

  const importVault = () => run(async () => {
    setVaults(await koofr.importVaultRcloneConfig())
    onNotice('rclone 配置已导入；Safe Key 和 salt 未经过前端。')
  }, '无法导入 rclone crypt 配置。')

  const exportVault = (repoId: string) => run(async () => {
    const exported = await koofr.exportVaultRcloneConfig(repoId)
    if (exported) onNotice('rclone 配置已导出到所选位置。请按敏感凭据妥善保管。')
  }, '无法导出 rclone 配置。')

  const removeVault = (repoId: string) => run(async () => {
    const next = await koofr.removeVault(repoId, removeConfirmation)
    setVaults(next)
    setRemoveRepoId(null)
    setRemoveConfirmation('')
    if (directory?.repoId === repoId) setDirectory(null)
    onNotice('已移除保险箱注册；远端加密目录及其中数据仍然保留。')
  }, '无法移除保险箱注册。')

  const refreshDirectory = () => {
    if (directory) void openDirectory(directory.repoId, directory.directoryId)
    else void loadVaults()
  }

  const selected = directory?.entries.filter((entry) => selectedIds.has(entry.id)) ?? []
  const singleSelected = selected.length === 1 ? selected[0] : null

  const createFolder = () => run(async () => {
    if (!directory || !folderName.trim()) return
    await koofr.createVaultFolder(directory.repoId, directory.directoryId, folderName.trim())
    setFolderName('')
    await openDirectory(directory.repoId, directory.directoryId)
  }, '无法创建加密文件夹。')

  const renameEntry = () => run(async () => {
    if (!directory || !singleSelected || !renameName.trim()) return
    await koofr.renameVaultEntry(directory.repoId, singleSelected.id, renameName.trim())
    setRenameName('')
    await openDirectory(directory.repoId, directory.directoryId)
  }, '无法重命名加密项目。')

  const deleteEntries = () => run(async () => {
    if (!directory || selected.length === 0) return
    if (!window.confirm(`确定删除所选 ${selected.length} 个加密项目吗？`)) return
    await koofr.deleteVaultEntries(directory.repoId, selected.map((entry) => entry.id))
    await openDirectory(directory.repoId, directory.directoryId)
  }, '无法删除加密项目。')

  const completeRelocation = () => run(async () => {
    if (!directory || !relocation) return
    await koofr.relocateVaultEntry(
      directory.repoId,
      relocation.entry.id,
      directory.directoryId,
      relocation.isMove,
    )
    const source = relocation
    setRelocation(null)
    await openDirectory(directory.repoId, directory.directoryId)
    onNotice(`${source.entry.name} 已${source.isMove ? '移动' : '复制'}到目标文件夹。`)
  }, relocation?.isMove ? '无法移动加密项目。' : '无法复制加密项目。')

  const upload = () => run(async () => {
    if (!directory) return
    const selection = await koofr.selectUploadFile()
    if (!selection) return
    const transfer = koofr.uploadVaultFile(directory.repoId, directory.directoryId, selection.grantId)
    const sourceRepo = directory.repoId
    const sourceDirectory = directory.directoryId
    const result = transfer.result.then(async (value) => {
      const current = directoryRef.current
      if (current?.repoId === sourceRepo && current.directoryId === sourceDirectory) {
        await openDirectory(sourceRepo, sourceDirectory)
      }
      return value
    })
    onTransferStarted({
      transferId: transfer.transferId,
      name: selection.fileName,
      direction: 'upload',
      totalBytes: null,
      recoveryKind: 'restart',
      localPath: selection.localPath,
      result,
    })
  }, '无法开始加密上传。')

  const downloadEntry = (entry: VaultEntry) => run(async () => {
    if (!directory || entry.entryType !== 'file') return
    const selection = await koofr.selectDownloadLocation(entry.name)
    if (!selection) return
    const transfer = koofr.downloadVaultFile(
      directory.repoId,
      entry.id,
      entry.name,
      selection.grantId,
    )
    onTransferStarted({
      transferId: transfer.transferId,
      name: entry.name,
      direction: 'download',
      totalBytes: entry.size,
      recoveryKind: 'byte_resume',
      localPath: selection.localPath,
      result: transfer.result,
    })
  }, '无法开始加密下载。')

  return (
    <Modal title="Koofr Vault" actionLabel="关闭" extraWide onClose={onClose}>
      <div className="vault-workspace">
        <div className="vault-security-note">
          <FileKey2 size={18} />
          <span>Safe Key 只在 Windows 原生安全窗口中输入，并始终留在 Rust 后端内存。</span>
        </div>

        {error ? <div className="vault-error" role="alert">{error}</div> : null}

        {!directory ? (
          <>
            <div className="vault-toolbar">
              <button className="secondary-button" type="button" disabled={busy} onClick={() => void loadVaults()}>
                <RefreshCw size={15} />刷新
              </button>
              <button className="secondary-button" type="button" disabled={busy} onClick={() => void importVault()}>
                <FolderInput size={15} />导入 rclone 配置
              </button>
            </div>

            <div className="vault-create">
              <label>
                <span>在当前 Koofr 文件夹创建保险箱</span>
                <input
                  value={createName}
                  maxLength={255}
                  placeholder="保险箱名称"
                  onChange={(event) => setCreateName(event.target.value)}
                />
              </label>
              <button
                className="primary-button"
                type="button"
                disabled={busy || !mountId || !createName.trim()}
                onClick={() => void createVault()}
              >
                <ShieldPlus size={15} />创建
              </button>
            </div>

            <div className="vault-list">
              {loading ? <p className="vault-muted">正在读取保险箱…</p> : null}
              {!loading && vaults.length === 0 ? (
                <div className="vault-empty">
                  <Lock size={28} />
                  <strong>尚未注册保险箱</strong>
                  <span>可在当前目录创建，或导入本客户端导出的 rclone crypt 配置。</span>
                </div>
              ) : null}
              {vaults.map((vault) => (
                <article className="vault-card" key={vault.id}>
                  <div>
                    <strong>{vault.name}</strong>
                    <span>{vault.locked ? '已锁定' : `已解锁 · ${Math.round(vault.autoLockSeconds / 60)} 分钟无操作后自动锁定`}</span>
                  </div>
                  <div className="vault-card__actions">
                    {vault.locked ? (
                      <button className="primary-button" type="button" disabled={busy} onClick={() => void unlock(vault.id)}>
                        <KeyRound size={14} />解锁
                      </button>
                    ) : (
                      <>
                        <button className="primary-button" type="button" disabled={busy} onClick={() => void openDirectory(vault.id, 'root')}>
                          <Folder size={14} />打开
                        </button>
                        <button className="secondary-button" type="button" disabled={busy} onClick={() => void lock(vault.id)}>
                          <Lock size={14} />锁定
                        </button>
                      </>
                    )}
                    <button className="secondary-button" type="button" disabled={busy} onClick={() => void exportVault(vault.id)}>
                      导出配置
                    </button>
                    <button
                      className="danger-button"
                      type="button"
                      disabled={busy}
                      onClick={() => {
                        setRemoveRepoId(vault.id)
                        setRemoveConfirmation('')
                      }}
                    >
                      移除
                    </button>
                  </div>
                  {removeRepoId === vault.id ? (
                    <div className="vault-remove-confirm">
                      <span>仅移除注册，不删除密文。输入“移除保险箱”并再次验证 Safe Key：</span>
                      <input value={removeConfirmation} onChange={(event) => setRemoveConfirmation(event.target.value)} />
                      <button
                        className="danger-button"
                        type="button"
                        disabled={busy || removeConfirmation !== '移除保险箱'}
                        onClick={() => void removeVault(vault.id)}
                      >
                        确认移除
                      </button>
                    </div>
                  ) : null}
                </article>
              ))}
            </div>
          </>
        ) : (
          <>
            <div className="vault-toolbar vault-toolbar--browser">
              <button className="secondary-button" type="button" disabled={busy} onClick={() => setDirectory(null)}>
                <ArrowLeft size={15} />保险箱列表
              </button>
              <nav className="vault-breadcrumb" aria-label="保险箱路径">
                {directory.breadcrumbs.map((crumb, index) => (
                  <span key={crumb.id}>
                    {index > 0 ? <i>/</i> : null}
                    <button type="button" disabled={busy || crumb.id === directory.directoryId} onClick={() => void openDirectory(directory.repoId, crumb.id)}>
                      {crumb.name}
                    </button>
                  </span>
                ))}
              </nav>
              <button className="secondary-button" type="button" disabled={busy} onClick={refreshDirectory}>
                <RefreshCw size={15} />
                <span className="sr-only">刷新</span>
              </button>
              <button className="secondary-button" type="button" disabled={busy} onClick={() => void lock(directory.repoId)}>
                <Lock size={15} />锁定
              </button>
            </div>

            {relocation ? (
              <div className="vault-relocation">
                <span>选择目标文件夹：{relocation.entry.name}</span>
                <button className="primary-button" type="button" disabled={busy} onClick={() => void completeRelocation()}>
                  {relocation.isMove ? <FolderInput size={15} /> : <Copy size={15} />}
                  {relocation.isMove ? '移动到此处' : '复制到此处'}
                </button>
                <button className="secondary-button" type="button" onClick={() => setRelocation(null)}>取消</button>
              </div>
            ) : (
              <>
                <div className="vault-toolbar">
                  <button className="secondary-button" type="button" disabled={busy} onClick={() => void upload()}>
                    <Upload size={15} />加密上传
                  </button>
                  <button className="secondary-button" type="button" disabled={busy || !singleSelected || singleSelected.entryType !== 'file'} onClick={() => singleSelected && void downloadEntry(singleSelected)}>
                    <Download size={15} />解密下载
                  </button>
                  <button className="secondary-button" type="button" disabled={busy || !singleSelected} onClick={() => {
                    if (singleSelected) {
                      setRenameName(singleSelected.name)
                    }
                  }}>
                    <Pencil size={15} />重命名
                  </button>
                  <button className="secondary-button" type="button" disabled={busy || !singleSelected} onClick={() => singleSelected && setRelocation({ entry: singleSelected, isMove: true })}>
                    <FolderInput size={15} />移动
                  </button>
                  <button className="secondary-button" type="button" disabled={busy || !singleSelected} onClick={() => singleSelected && setRelocation({ entry: singleSelected, isMove: false })}>
                    <Copy size={15} />复制
                  </button>
                  <button className="danger-button" type="button" disabled={busy || selected.length === 0} onClick={() => void deleteEntries()}>
                    <Trash2 size={15} />删除
                  </button>
                </div>

                <div className="vault-inline-forms">
                  <label>
                    <span>新建文件夹</span>
                    <input value={folderName} placeholder="文件夹名称" onChange={(event) => setFolderName(event.target.value)} />
                    <button className="secondary-button" type="button" disabled={busy || !folderName.trim()} onClick={() => void createFolder()}>
                      <FolderPlus size={15} />新建
                    </button>
                  </label>
                  {renameName ? (
                    <label>
                      <span>新名称</span>
                      <input autoFocus value={renameName} onChange={(event) => setRenameName(event.target.value)} />
                      <button className="primary-button" type="button" disabled={busy || !renameName.trim()} onClick={() => void renameEntry()}>
                        保存
                      </button>
                    </label>
                  ) : null}
                </div>
              </>
            )}

            <div className="vault-file-list" role="list" aria-label={`${directory.repoName} 内容`}>
              {loading ? <p className="vault-muted">正在解密目录元数据…</p> : null}
              {!loading && directory.entries.length === 0 ? (
                <div className="vault-empty">
                  <Folder size={28} />
                  <strong>此加密文件夹为空</strong>
                </div>
              ) : null}
              {directory.entries.map((entry) => {
                const selected = selectedIds.has(entry.id)
                return (
                  <button
                    className={`vault-file-row${selected ? ' vault-file-row--selected' : ''}`}
                    type="button"
                    role="listitem"
                    key={entry.id}
                    onClick={(event) => {
                      if (event.ctrlKey || event.metaKey) {
                        setSelectedIds((current) => {
                          const next = new Set(current)
                          if (next.has(entry.id)) next.delete(entry.id)
                          else next.add(entry.id)
                          return next
                        })
                      } else {
                        setSelectedIds(new Set([entry.id]))
                      }
                    }}
                    onDoubleClick={() => {
                      if (entry.entryType === 'dir') void openDirectory(directory.repoId, entry.id)
                      else void downloadEntry(entry)
                    }}
                  >
                    {entry.entryType === 'dir' ? <Folder size={22} /> : <File size={22} />}
                    <span className="vault-file-row__name">{entry.name}</span>
                    <span>{entry.entryType === 'dir' ? '文件夹' : formatBytes(entry.size)}</span>
                    <span>{formatModified(entry.modified)}</span>
                  </button>
                )
              })}
            </div>
          </>
        )}

        {busy ? <p className="vault-muted">正在处理…</p> : null}
      </div>
    </Modal>
  )
}
