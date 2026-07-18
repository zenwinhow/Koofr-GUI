import { useEffect, useRef, useState } from 'react'
import { ArrowDownToLine } from 'lucide-react'
import { AppSidebar } from './components/AppSidebar'
import { Modal } from './components/Modal'
import { ThemePicker } from './components/ThemePicker'
import { TitleBar } from './components/TitleBar'
import { LoginPage } from './features/auth/LoginPage'
import { CollectionWorkspace } from './features/files/CollectionWorkspace'
import { FileWorkspace } from './features/files/FileWorkspace'
import { isDirectory } from './features/files/filePresentation'
import { isCollectionView, useKoofrCollections } from './features/files/useKoofrCollections'
import { useKoofrWorkspace } from './features/files/useKoofrWorkspace'
import { SettingsPanel } from './features/settings/SettingsPanel'
import { DownloadDestinationDialog } from './features/transfers/DownloadDestinationDialog'
import { TransferPanel } from './features/transfers/TransferPanel'
import { beginDownload } from './features/transfers/beginDownload'
import {
  commandErrorMessage,
  isCommandErrorCode,
  isTauriRuntime,
  koofr,
} from './services/koofr'
import { readStoredTheme, storeTheme, type ThemeId } from './theme'
import type {
  AppSettings,
  CacheMode,
  LocatedFile,
  RemoteFile,
  TransferProgress,
  TrashItem,
} from './types/backend'
import type { TransferItem } from './types/files'

type ModalKind = 'settings' | 'theme' | 'vault' | 'download' | 'createFolder' | 'rename' | 'delete' | 'emptyTrash' | null
type AuthState = 'checking' | 'signedOut' | 'signingIn' | 'signedIn'

interface PendingDownload {
  readonly file: RemoteFile
  readonly mountId: string
}

function App() {
  const [activeItem, setActiveItem] = useState('我的文件')
  const [authState, setAuthState] = useState<AuthState>('checking')
  const [loginError, setLoginError] = useState('')
  const [savedEmail, setSavedEmail] = useState('')
  const [modalKind, setModalKind] = useState<ModalKind>(null)
  const [modalInput, setModalInput] = useState('')
  const [pendingFiles, setPendingFiles] = useState<RemoteFile[]>([])
  const [operationBusy, setOperationBusy] = useState(false)
  const [transferVisible, setTransferVisible] = useState(false)
  const [transfers, setTransfers] = useState<TransferItem[]>([])
  const [notice, setNotice] = useState('')
  const [themeId, setThemeId] = useState<ThemeId>(readStoredTheme)
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [settingsLoading, setSettingsLoading] = useState(false)
  const [settingsBusy, setSettingsBusy] = useState(false)
  const [settingsError, setSettingsError] = useState('')
  const [downloadSettingsError, setDownloadSettingsError] = useState('')
  const [pendingDownload, setPendingDownload] = useState<PendingDownload | null>(null)
  const [downloadDirectory, setDownloadDirectory] = useState('')
  const [downloadBusy, setDownloadBusy] = useState(false)
  const [downloadError, setDownloadError] = useState('')
  const workspace = useKoofrWorkspace(authState === 'signedIn')
  const collections = useKoofrCollections(authState === 'signedIn', activeItem)
  const workspaceLocation = useRef({ activeMountId: '', path: '/' })
  const activeMount = workspace.mounts.find((mount) => mount.id === workspace.activeMountId)
  const runningTransfers = transfers.filter((transfer) => transfer.state === 'running').length

  useEffect(() => {
    workspaceLocation.current = {
      activeMountId: workspace.activeMountId,
      path: workspace.path,
    }
  }, [workspace.activeMountId, workspace.path])

  useEffect(() => {
    let active = true

    if (!isTauriRuntime()) {
      setAuthState('signedOut')
      return () => { active = false }
    }

    koofr.restoreSavedLogin()
      .then((bootstrap) => {
        if (!active) return
        setSavedEmail(bootstrap.savedEmail ?? '')
        setAuthState(bootstrap.session.authenticated ? 'signedIn' : 'signedOut')
      })
      .catch(async (error) => {
        if (active) {
          setLoginError(commandErrorMessage(error, '无法恢复保存的登录信息，请手动登录。'))
          try {
            const localSettings = await koofr.getSettings()
            if (active) setSavedEmail(localSettings.savedEmail ?? '')
          } catch {
            // The login form remains usable even if optional local settings cannot be read.
          }
          setAuthState('signedOut')
        }
      })

    return () => { active = false }
  }, [])

  useEffect(() => {
    if (authState !== 'signedIn') return
    let disposed = false
    let unlisten: (() => void) | undefined

    void koofr.onTransferProgress((progress: TransferProgress) => {
      setTransfers((current) => current.map((transfer) => transfer.id === progress.transferId
        ? {
            ...transfer,
            state: progress.state,
            bytesTransferred: progress.bytesTransferred,
            totalBytes: progress.totalBytes,
          }
        : transfer))
    }).then((stopListening) => {
      if (disposed) stopListening()
      else unlisten = stopListening
    })

    return () => {
      disposed = true
      unlisten?.()
    }
  }, [authState])

  useEffect(() => {
    if (authState !== 'signedIn') return
    let active = true
    void koofr.listResumableTransfers()
      .then((resumable) => {
        if (!active) return
        setTransfers((current) => {
          const existingIds = new Set(current.map((item) => item.id))
          const restored = resumable
            .filter((item) => !existingIds.has(item.transferId))
            .map((item): TransferItem => ({
              id: item.transferId,
              name: item.name,
              direction: item.direction,
              state: 'paused',
              bytesTransferred: item.bytesTransferred,
              totalBytes: item.totalBytes,
              localKind: 'file',
              recoveryKind: item.recoveryKind,
            }))
          return [...restored, ...current]
        })
      })
      .catch((error) => {
        if (active) setNotice(commandErrorMessage(error, '无法读取中断的传输任务。'))
      })
    return () => { active = false }
  }, [authState])

  const showNotice = (message: string) => {
    setNotice(message)
    window.setTimeout(() => setNotice(''), 3200)
  }

  const selectTheme = (nextTheme: ThemeId) => {
    setThemeId(nextTheme)
    storeTheme(nextTheme)
  }

  const login = async (email: string, appPassword: string, rememberPassword: boolean) => {
    if (!isTauriRuntime()) {
      setLoginError('登录功能需要在 Koofr 桌面应用中使用。')
      return
    }

    setLoginError('')
    setAuthState('signingIn')
    try {
      const session = await koofr.connect(email, appPassword, rememberPassword)
      if (!session.authenticated) {
        setLoginError('Koofr 未能建立登录会话，请重试。')
        setAuthState('signedOut')
        return
      }
      setSavedEmail(rememberPassword ? email : '')
      setAuthState('signedIn')
    } catch (error) {
      setLoginError(commandErrorMessage(error, '登录失败，请稍后重试。'))
      setAuthState('signedOut')
    }
  }

  const openSettings = async () => {
    setModalKind('settings')
    setSettingsLoading(true)
    setSettingsError('')
    setDownloadSettingsError('')
    try {
      setSettings(await koofr.getSettings())
    } catch (error) {
      setSettingsError(commandErrorMessage(error, '无法读取本地设置。'))
    } finally {
      setSettingsLoading(false)
    }
  }

  const updateCacheSettings = async (cacheMode: CacheMode, cacheTtlMinutes: number) => {
    setSettingsBusy(true)
    setSettingsError('')
    try {
      setSettings(await koofr.updateSettings(cacheMode, cacheTtlMinutes))
    } catch (error) {
      setSettingsError(commandErrorMessage(error, '无法保存缓存设置。'))
    } finally {
      setSettingsBusy(false)
    }
  }

  const updateDownloadSettings = async (
    nextDownloadDirectory: string,
    askDownloadLocation: boolean,
  ) => {
    setSettingsBusy(true)
    setSettingsError('')
    setDownloadSettingsError('')
    try {
      const nextSettings = await koofr.updateDownloadSettings(
        nextDownloadDirectory,
        askDownloadLocation,
      )
      setSettings(nextSettings)
      showNotice('下载设置已保存')
    } catch (error) {
      setDownloadSettingsError(commandErrorMessage(error, '无法保存下载设置，请检查文件夹路径。'))
    } finally {
      setSettingsBusy(false)
    }
  }

  const browseSettingsDownloadDirectory = async () => {
    setDownloadSettingsError('')
    try {
      return await koofr.selectDownloadDirectory()
    } catch (error) {
      setDownloadSettingsError(commandErrorMessage(error, '无法打开下载文件夹选择器。'))
      return null
    }
  }

  const clearMetadataCache = async () => {
    setSettingsBusy(true)
    setSettingsError('')
    try {
      setSettings(await koofr.clearMetadataCache())
      showNotice('文件信息缓存已清除')
    } catch (error) {
      setSettingsError(commandErrorMessage(error, '无法清除文件信息缓存。'))
    } finally {
      setSettingsBusy(false)
    }
  }

  const forgetSavedLogin = async () => {
    setSettingsBusy(true)
    setSettingsError('')
    try {
      const next = await koofr.forgetSavedLogin()
      setSettings(next)
      setSavedEmail('')
      showNotice('已从 Windows 凭据管理器删除登录信息')
    } catch (error) {
      setSettingsError(commandErrorMessage(error, '无法删除保存的登录信息。'))
    } finally {
      setSettingsBusy(false)
    }
  }

  const logout = async () => {
    try {
      await koofr.disconnect()
      setModalKind(null)
      setLoginError('')
      setTransfers([])
      setAuthState('signedOut')
    } catch (error) {
      showNotice(commandErrorMessage(error, '暂时无法退出登录，请重试。'))
    }
  }

  const handleUpload = async () => {
    if (!workspace.activeMountId) return
    const uploadMountId = workspace.activeMountId
    const uploadPath = workspace.path
    try {
      const selection = await koofr.selectUploadFile()
      if (!selection) return

      const transfer = koofr.uploadFile(
        uploadMountId,
        uploadPath,
        selection.grantId,
      )
      setTransfers((current) => [{
        id: transfer.transferId,
        name: selection.fileName,
        direction: 'upload',
        state: 'running',
        bytesTransferred: 0,
        totalBytes: null,
        localKind: 'file',
        recoveryKind: 'restart',
      }, ...current])
      setTransferVisible(true)

      try {
        const result = await transfer.result
        setTransfers((current) => current.map((item) => item.id === transfer.transferId
          ? {
              ...item,
              state: 'completed',
              bytesTransferred: result.bytesTransferred,
              totalBytes: item.totalBytes ?? result.bytesTransferred,
            }
          : item))
        if (
          workspaceLocation.current.activeMountId === uploadMountId
          && workspaceLocation.current.path === uploadPath
        ) {
          await workspace.loadDirectory(uploadMountId, uploadPath, true)
        }
      } catch (error) {
        setTransfers((current) => current.map((item) => item.id === transfer.transferId
          ? {
              ...item,
              state: isCommandErrorCode(error, 'transfer_paused')
                ? 'paused'
                : isCommandErrorCode(error, 'cancelled') ? 'cancelled' : 'failed',
            }
          : item))
        showNotice(commandErrorMessage(error, '上传失败，请稍后重试。'))
      }
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法选择本地文件。'))
    }
  }

  const startDownload = async (
    file: RemoteFile,
    mountId: string,
    targetDirectory: string,
  ) => {
    const transfer = await beginDownload(file, mountId, targetDirectory)
    setTransfers((current) => [{
      id: transfer.transferId,
      name: file.name,
      direction: 'download',
      state: 'running',
      bytesTransferred: 0,
      totalBytes: transfer.localKind === 'file' && file.size > 0 ? file.size : null,
      localKind: transfer.localKind,
      recoveryKind: transfer.localKind === 'file' ? 'byte_resume' : null,
    }, ...current])
    setTransferVisible(true)

    const monitorDownload = async () => {
      try {
        const result = await transfer.result
        setTransfers((current) => current.map((item) => item.id === transfer.transferId
          ? {
              ...item,
              state: 'completed',
              bytesTransferred: result.bytesTransferred,
              totalBytes: item.totalBytes ?? result.bytesTransferred,
            }
          : item))
      } catch (error) {
        setTransfers((current) => current.map((item) => item.id === transfer.transferId
          ? {
              ...item,
              state: isCommandErrorCode(error, 'transfer_paused')
                ? 'paused'
                : isCommandErrorCode(error, 'cancelled') ? 'cancelled' : 'failed',
            }
          : item))
        showNotice(commandErrorMessage(error, '下载失败，请稍后重试。'))
      }
    }
    void monitorDownload()
  }

  const openDownloadDestination = (
    file: RemoteFile,
    mountId: string,
    initialDirectory: string,
    error = '',
  ) => {
    setPendingDownload({ file, mountId })
    setDownloadDirectory(initialDirectory)
    setDownloadError(error)
    setModalKind('download')
  }

  const handleDownload = async (file: RemoteFile, mountId = workspace.activeMountId) => {
    if (!mountId) return
    let downloadSettings = settings
    try {
      if (!downloadSettings) {
        downloadSettings = await koofr.getSettings()
        setSettings(downloadSettings)
      }
      if (downloadSettings.askDownloadLocation) {
        openDownloadDestination(file, mountId, downloadSettings.downloadDirectory)
        return
      }
      await startDownload(file, mountId, downloadSettings.downloadDirectory)
    } catch (error) {
      const fallbackDirectory = downloadSettings?.downloadDirectory ?? ''
      openDownloadDestination(
        file,
        mountId,
        fallbackDirectory,
        commandErrorMessage(error, '默认下载位置不可用，请选择其他文件夹。'),
      )
    }
  }

  const confirmDownload = async (targetDirectory: string, askEveryTime: boolean) => {
    if (!pendingDownload) return
    setDownloadBusy(true)
    setDownloadError('')
    try {
      if (!askEveryTime || settings?.askDownloadLocation !== askEveryTime) {
        setSettings(await koofr.updateDownloadSettings(targetDirectory, askEveryTime))
      }
      await startDownload(pendingDownload.file, pendingDownload.mountId, targetDirectory)
      setModalKind(null)
      setPendingDownload(null)
    } catch (error) {
      setDownloadError(commandErrorMessage(error, '无法使用这个下载位置，请检查路径后重试。'))
    } finally {
      setDownloadBusy(false)
    }
  }

  const browseDownloadDirectory = async () => {
    setDownloadError('')
    try {
      return await koofr.selectDownloadDirectory()
    } catch (error) {
      setDownloadError(commandErrorMessage(error, '无法打开下载文件夹选择器。'))
      return null
    }
  }

  const cancelTransfer = async (transferId: string) => {
    try {
      await koofr.cancelTransfer(transferId)
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法取消这个传输任务。'))
    }
  }

  const resumeTransfer = async (transferId: string) => {
    setTransfers((current) => current.map((item) => item.id === transferId
      ? { ...item, state: 'running' }
      : item))
    setTransferVisible(true)
    try {
      const result = await koofr.resumeTransfer(transferId)
      setTransfers((current) => current.map((item) => item.id === transferId
        ? {
            ...item,
            state: 'completed',
            bytesTransferred: result.bytesTransferred,
            totalBytes: item.totalBytes ?? result.bytesTransferred,
          }
        : item))
      await workspace.refresh()
    } catch (error) {
      setTransfers((current) => current.map((item) => item.id === transferId
        ? {
            ...item,
            state: isCommandErrorCode(error, 'transfer_paused') ? 'paused' : 'failed',
          }
        : item))
      showNotice(commandErrorMessage(error, '无法继续这个传输任务。'))
    }
  }

  const discardResumableTransfer = async (transferId: string) => {
    try {
      await koofr.discardResumableTransfer(transferId)
      setTransfers((current) => current.filter((item) => item.id !== transferId))
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法放弃这个中断任务。'))
    }
  }

  const clearFinishedTransfers = () => {
    setTransfers((current) => current.filter((item) => (
      item.state === 'running' || item.state === 'paused'
    )))
  }

  const openDownloadedFile = async (transferId: string) => {
    try {
      await koofr.openDownloadedFile(transferId)
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法打开下载的文件。'))
    }
  }

  const openDownloadedFolder = async (transferId: string) => {
    try {
      await koofr.openDownloadedFolder(transferId)
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法打开文件所在的文件夹。'))
    }
  }

  const openCreateFolder = () => {
    setModalInput('')
    setPendingFiles([])
    setModalKind('createFolder')
  }

  const createFolder = async () => {
    const name = modalInput.trim()
    if (!workspace.activeMountId || !name) return
    setOperationBusy(true)
    try {
      await koofr.createFolder(workspace.activeMountId, workspace.path, name)
      setModalKind(null)
      await workspace.refresh()
      showNotice(`已创建文件夹“${name}”`)
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法创建文件夹。'))
    } finally {
      setOperationBusy(false)
    }
  }

  const openRename = (file: RemoteFile) => {
    setModalInput(file.name)
    setPendingFiles([file])
    setModalKind('rename')
  }

  const renameFile = async () => {
    const file = pendingFiles[0]
    const name = modalInput.trim()
    if (!file || !workspace.activeMountId || !name || name === file.name) return
    setOperationBusy(true)
    try {
      await koofr.renameEntry(workspace.activeMountId, file.path, name)
      setModalKind(null)
      await workspace.refresh()
      showNotice(`已重命名为“${name}”`)
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法重命名这个项目。'))
    } finally {
      setOperationBusy(false)
    }
  }

  const openDelete = (files: RemoteFile[]) => {
    setPendingFiles(files)
    setModalKind('delete')
  }

  const deleteFiles = async () => {
    if (!workspace.activeMountId || pendingFiles.length === 0) return
    setOperationBusy(true)
    try {
      const results = await Promise.allSettled(
        pendingFiles.map((file) => koofr.deleteEntry(workspace.activeMountId, file.path)),
      )
      const deletedCount = results.filter((result) => result.status === 'fulfilled').length
      const failed = results.filter((result) => result.status === 'rejected')
      setModalKind(null)
      if (failed.length === 0) {
        showNotice(`已删除 ${deletedCount} 个项目`)
      } else {
        const prefix = deletedCount > 0 ? `已删除 ${deletedCount} 个项目；` : ''
        showNotice(`${prefix}${commandErrorMessage(failed[0]?.reason, '其余项目删除失败。')}`)
      }
    } finally {
      await workspace.refresh()
      setOperationBusy(false)
    }
  }

  const openLocatedFile = (file: LocatedFile) => {
    const segments = file.path.split('/').filter(Boolean)
    const targetPath = isDirectory(file)
      ? file.path
      : segments.length <= 1 ? '/' : `/${segments.slice(0, -1).join('/')}`
    setActiveItem('我的文件')
    void workspace.loadDirectory(file.mountId, targetPath)
  }

  const restoreTrash = async (items: TrashItem[]) => {
    if (items.length === 0) return
    setOperationBusy(true)
    try {
      await koofr.restoreTrash(items)
      await collections.load('回收站', true)
      showNotice(`已提交恢复 ${items.length} 个项目`)
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法恢复所选项目。'))
    } finally {
      setOperationBusy(false)
    }
  }

  const restoreAllTrash = async () => {
    setOperationBusy(true)
    try {
      await koofr.restoreAllTrash()
      await collections.load('回收站', true)
      showNotice('已提交恢复全部项目')
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法恢复回收站项目。'))
    } finally {
      setOperationBusy(false)
    }
  }

  const openEmptyTrash = () => {
    setModalInput('')
    setModalKind('emptyTrash')
  }

  const emptyTrash = async () => {
    if (modalInput !== '永久删除') return
    setOperationBusy(true)
    try {
      await koofr.emptyTrash(modalInput)
      setModalKind(null)
      await collections.load('回收站', true)
      showNotice('回收站已永久清空')
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法清空回收站。'))
    } finally {
      setOperationBusy(false)
    }
  }

  const selectNavigation = (label: string) => {
    setActiveItem(label)
    if (label === '我的文件') {
      if (workspace.activeMountId && workspace.path !== '/') {
        void workspace.loadDirectory(workspace.activeMountId, '/')
      }
      return
    }
  }

  return (
    <div className="app-shell" data-theme={themeId}>
      <TitleBar />

      {authState === 'checking' ? (
        <main className="auth-loading" aria-live="polite">
          <span className="auth-spinner" />
          正在检查登录状态…
        </main>
      ) : null}

      {authState === 'signedOut' || authState === 'signingIn' ? (
        <LoginPage
          busy={authState === 'signingIn'}
          error={loginError}
          initialEmail={savedEmail}
          onLogin={login}
          onThemeClick={() => setModalKind('theme')}
        />
      ) : null}

      {authState === 'signedIn' ? (
        <div className="app-body">
          <AppSidebar
            activeItem={activeItem}
            onSelect={selectNavigation}
            onSettingsClick={() => void openSettings()}
            onThemeClick={() => setModalKind('theme')}
            onVaultClick={() => setModalKind('vault')}
            onLogoutClick={logout}
            storageName={activeMount?.name ?? ''}
            storageUsed={activeMount?.spaceUsed ?? null}
            storageTotal={activeMount?.spaceTotal ?? null}
          />
          {isCollectionView(activeItem) ? (
            <CollectionWorkspace
              view={activeItem}
              files={collections.files}
              trash={collections.trash}
              retentionDays={collections.retentionDays}
              loading={collections.status === 'loading'}
              error={collections.error}
              diagnostic={collections.diagnostic}
              lastSyncedAt={collections.lastSyncedAt}
              busy={operationBusy}
              onRefresh={() => void collections.refresh()}
              onThemeOpen={() => setModalKind('theme')}
              onOpenLocation={openLocatedFile}
              onDownload={(file) => void handleDownload(file, file.mountId)}
              onRestore={(items) => void restoreTrash(items)}
              onRestoreAll={() => void restoreAllTrash()}
              onEmptyTrash={openEmptyTrash}
            />
          ) : (
            <FileWorkspace
              mounts={workspace.mounts}
              activeMountId={workspace.activeMountId}
              path={workspace.path}
              files={workspace.files}
              loading={workspace.status === 'loading'}
              error={workspace.error}
              lastSyncedAt={workspace.lastSyncedAt}
              onMountChange={(mountId) => void workspace.loadDirectory(mountId, '/')}
              onNavigate={(path) => void workspace.loadDirectory(workspace.activeMountId, path)}
              onRefresh={() => void (workspace.activeMountId ? workspace.refresh() : workspace.initialize())}
              onCreateFolder={openCreateFolder}
              onThemeOpen={() => setModalKind('theme')}
              onUpload={() => void handleUpload()}
              onDownload={(file) => void handleDownload(file)}
              onRename={openRename}
              onDelete={openDelete}
            />
          )}
          {transferVisible ? (
            <button
              className="transfer-panel-backdrop"
              type="button"
              aria-label="收起传输面板"
              onClick={() => setTransferVisible(false)}
            />
          ) : null}
          <TransferPanel
            visible={transferVisible}
            items={transfers}
            onClose={() => setTransferVisible(false)}
            onCancel={(transferId) => void cancelTransfer(transferId)}
            onResume={(transferId) => void resumeTransfer(transferId)}
            onDiscard={(transferId) => void discardResumableTransfer(transferId)}
            onOpenFile={(transferId) => void openDownloadedFile(transferId)}
            onOpenFolder={(transferId) => void openDownloadedFolder(transferId)}
            onClearFinished={clearFinishedTransfers}
          />
          {!transferVisible ? (
            <button
              className="transfer-reopen"
              type="button"
              aria-label={transfers.length > 0 ? `打开下载界面，共 ${transfers.length} 个任务` : '打开下载界面'}
              title="打开下载界面"
              onClick={() => setTransferVisible(true)}
            >
              <ArrowDownToLine size={23} />
              {transfers.length > 0 ? (
                <span>{runningTransfers || transfers.length}</span>
              ) : null}
            </button>
          ) : null}
        </div>
      ) : null}

      {notice ? <div className="toast" role="status">{notice}</div> : null}

      {authState === 'signedIn' && modalKind === 'download' && pendingDownload ? (
        <DownloadDestinationDialog
          fileName={pendingDownload.file.name}
          initialDirectory={downloadDirectory}
          initialAskEveryTime={settings?.askDownloadLocation ?? true}
          busy={downloadBusy}
          error={downloadError}
          onBrowse={browseDownloadDirectory}
          onClose={() => {
            setModalKind(null)
            setPendingDownload(null)
            setDownloadError('')
          }}
          onConfirm={(directory, askEveryTime) => void confirmDownload(directory, askEveryTime)}
        />
      ) : null}

      {authState === 'signedIn' && modalKind === 'createFolder' ? (
        <Modal
          title="新建文件夹"
          actionLabel={operationBusy ? '正在创建…' : '创建文件夹'}
          actionDisabled={operationBusy || !modalInput.trim()}
          onClose={() => setModalKind(null)}
          onAction={() => void createFolder()}
        >
          <label className="modal-field">
            <span>文件夹名称</span>
            <input autoFocus value={modalInput} maxLength={255} onChange={(event) => setModalInput(event.target.value)} />
          </label>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'rename' ? (
        <Modal
          title="重命名"
          actionLabel={operationBusy ? '正在保存…' : '保存名称'}
          actionDisabled={operationBusy || !modalInput.trim() || modalInput.trim() === pendingFiles[0]?.name}
          onClose={() => setModalKind(null)}
          onAction={() => void renameFile()}
        >
          <label className="modal-field">
            <span>新名称</span>
            <input autoFocus value={modalInput} maxLength={255} onChange={(event) => setModalInput(event.target.value)} />
          </label>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'delete' ? (
        <Modal
          title="确认删除"
          actionLabel={operationBusy ? '正在删除…' : `删除 ${pendingFiles.length} 个项目`}
          actionDisabled={operationBusy}
          onClose={() => setModalKind(null)}
          onAction={() => void deleteFiles()}
        >
          <p>删除后，所选文件和文件夹将从当前 Koofr 存储位置移除。此操作需要明确确认。</p>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'emptyTrash' ? (
        <Modal
          title="永久清空回收站"
          actionLabel={operationBusy ? '正在永久删除…' : '永久清空'}
          actionDisabled={operationBusy || modalInput !== '永久删除'}
          onClose={() => setModalKind(null)}
          onAction={() => void emptyTrash()}
        >
          <p>回收站中的所有项目将被永久删除，且无法恢复。请输入“永久删除”以确认。</p>
          <label className="modal-field modal-field--spaced">
            <span>确认文字</span>
            <input autoFocus value={modalInput} onChange={(event) => setModalInput(event.target.value)} />
          </label>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'vault' ? (
        <Modal title="私人保险箱已锁定" actionLabel="知道了" onClose={() => setModalKind(null)}>
          <p>保险箱解锁需要由 Rust 后端安全处理。此 UI 不会在浏览器状态中读取或保存 Safe Key。</p>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'settings' ? (
        <Modal title="设置" actionLabel="完成" wide onClose={() => setModalKind(null)}>
          <SettingsPanel
            settings={settings}
            loading={settingsLoading}
            busy={settingsBusy}
            error={settingsError}
            downloadError={downloadSettingsError}
            onCacheModeChange={(cacheMode) => {
              if (settings) void updateCacheSettings(cacheMode, settings.cacheTtlMinutes)
            }}
            onCacheTtlChange={(cacheTtlMinutes) => {
              if (settings) void updateCacheSettings(settings.cacheMode, cacheTtlMinutes)
            }}
            onDownloadSettingsChange={(directory, askDownloadLocation) => {
              void updateDownloadSettings(directory, askDownloadLocation)
            }}
            onBrowseDownloadDirectory={browseSettingsDownloadDirectory}
            onClearCache={() => void clearMetadataCache()}
            onForgetLogin={() => void forgetSavedLogin()}
          />
        </Modal>
      ) : null}

      {modalKind === 'theme' ? (
        <Modal title="外观主题" actionLabel="完成" onClose={() => setModalKind(null)}>
          <ThemePicker value={themeId} onChange={selectTheme} />
        </Modal>
      ) : null}
    </div>
  )
}

export default App
