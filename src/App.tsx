import { useEffect, useRef, useState } from 'react'
import { AppSidebar } from './components/AppSidebar'
import { Modal } from './components/Modal'
import { ThemePicker } from './components/ThemePicker'
import { TitleBar } from './components/TitleBar'
import { LoginPage } from './features/auth/LoginPage'
import { FileWorkspace } from './features/files/FileWorkspace'
import { isDirectory } from './features/files/filePresentation'
import { useKoofrWorkspace } from './features/files/useKoofrWorkspace'
import { TransferPanel } from './features/transfers/TransferPanel'
import {
  commandErrorMessage,
  isCommandErrorCode,
  isTauriRuntime,
  koofr,
} from './services/koofr'
import { readStoredTheme, storeTheme, type ThemeId } from './theme'
import type { RemoteFile, TransferProgress } from './types/backend'
import type { TransferItem } from './types/files'

type ModalKind = 'settings' | 'theme' | 'vault' | 'createFolder' | 'rename' | 'delete' | null
type AuthState = 'checking' | 'signedOut' | 'signingIn' | 'signedIn'

function App() {
  const [activeItem, setActiveItem] = useState('我的文件')
  const [authState, setAuthState] = useState<AuthState>('checking')
  const [loginError, setLoginError] = useState('')
  const [modalKind, setModalKind] = useState<ModalKind>(null)
  const [modalInput, setModalInput] = useState('')
  const [pendingFiles, setPendingFiles] = useState<RemoteFile[]>([])
  const [operationBusy, setOperationBusy] = useState(false)
  const [transferVisible, setTransferVisible] = useState(false)
  const [transfers, setTransfers] = useState<TransferItem[]>([])
  const [notice, setNotice] = useState('')
  const [themeId, setThemeId] = useState<ThemeId>(readStoredTheme)
  const workspace = useKoofrWorkspace(authState === 'signedIn')
  const workspaceLocation = useRef({ activeMountId: '', path: '/' })
  workspaceLocation.current = {
    activeMountId: workspace.activeMountId,
    path: workspace.path,
  }
  const activeMount = workspace.mounts.find((mount) => mount.id === workspace.activeMountId)
  const runningTransfers = transfers.filter((transfer) => transfer.state === 'running').length

  useEffect(() => {
    let active = true

    if (!isTauriRuntime()) {
      setAuthState('signedOut')
      return () => { active = false }
    }

    koofr.session()
      .then((session) => {
        if (active) setAuthState(session.authenticated ? 'signedIn' : 'signedOut')
      })
      .catch(() => {
        if (active) {
          setLoginError('无法读取本地会话，请重新启动应用后再试。')
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

  const showNotice = (message: string) => {
    setNotice(message)
    window.setTimeout(() => setNotice(''), 3200)
  }

  const selectTheme = (nextTheme: ThemeId) => {
    setThemeId(nextTheme)
    storeTheme(nextTheme)
  }

  const login = async (email: string, appPassword: string) => {
    if (!isTauriRuntime()) {
      setLoginError('登录功能需要在 Koofr 桌面应用中使用。')
      return
    }

    setLoginError('')
    setAuthState('signingIn')
    try {
      const session = await koofr.connect(email, appPassword)
      if (!session.authenticated) {
        setLoginError('Koofr 未能建立登录会话，请重试。')
        setAuthState('signedOut')
        return
      }
      setAuthState('signedIn')
    } catch (error) {
      setLoginError(commandErrorMessage(error, '登录失败，请稍后重试。'))
      setAuthState('signedOut')
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
          await workspace.loadDirectory(uploadMountId, uploadPath)
        }
      } catch (error) {
        setTransfers((current) => current.map((item) => item.id === transfer.transferId
          ? { ...item, state: isCommandErrorCode(error, 'cancelled') ? 'cancelled' : 'failed' }
          : item))
        showNotice(commandErrorMessage(error, '上传失败，请稍后重试。'))
      }
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法选择本地文件。'))
    }
  }

  const handleDownload = async (file: RemoteFile) => {
    if (!workspace.activeMountId || isDirectory(file)) return
    try {
      const selection = await koofr.selectDownloadLocation(file.name)
      if (!selection) return

      const transfer = koofr.downloadFile(
        workspace.activeMountId,
        file.path,
        selection.grantId,
      )
      setTransfers((current) => [{
        id: transfer.transferId,
        name: file.name,
        direction: 'download',
        state: 'running',
        bytesTransferred: 0,
        totalBytes: file.size > 0 ? file.size : null,
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
      } catch (error) {
        setTransfers((current) => current.map((item) => item.id === transfer.transferId
          ? { ...item, state: isCommandErrorCode(error, 'cancelled') ? 'cancelled' : 'failed' }
          : item))
        showNotice(commandErrorMessage(error, '下载失败，请稍后重试。'))
      }
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法选择保存位置。'))
    }
  }

  const cancelTransfer = async (transferId: string) => {
    try {
      await koofr.cancelTransfer(transferId)
    } catch (error) {
      showNotice(commandErrorMessage(error, '无法取消这个传输任务。'))
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
    let deletedCount = 0
    try {
      for (const file of pendingFiles) {
        await koofr.deleteEntry(workspace.activeMountId, file.path)
        deletedCount += 1
      }
      setModalKind(null)
      showNotice(`已删除 ${deletedCount} 个项目`)
    } catch (error) {
      setModalKind(null)
      const prefix = deletedCount > 0 ? `已删除 ${deletedCount} 个项目；` : ''
      showNotice(`${prefix}${commandErrorMessage(error, '其余项目删除失败。')}`)
    } finally {
      await workspace.refresh()
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
    showNotice(`${label} 页面将在后续迭代中接入`)
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
          onLogin={login}
          onThemeClick={() => setModalKind('theme')}
        />
      ) : null}

      {authState === 'signedIn' ? (
        <div className="app-body">
          <AppSidebar
            activeItem={activeItem}
            onSelect={selectNavigation}
            onSettingsClick={() => setModalKind('settings')}
            onThemeClick={() => setModalKind('theme')}
            onVaultClick={() => setModalKind('vault')}
            onLogoutClick={logout}
            storageName={activeMount?.name ?? ''}
            storageUsed={activeMount?.spaceUsed ?? null}
            storageTotal={activeMount?.spaceTotal ?? null}
          />
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
          <TransferPanel
            visible={transferVisible}
            items={transfers}
            onClose={() => setTransferVisible(false)}
            onCancel={(transferId) => void cancelTransfer(transferId)}
            onClearFinished={() => setTransfers((current) => current.filter((item) => item.state === 'running'))}
          />
          {!transferVisible && transfers.length > 0 ? (
            <button className="transfer-reopen" type="button" onClick={() => setTransferVisible(true)}>
              传输 {runningTransfers || transfers.length}
            </button>
          ) : null}
        </div>
      ) : null}

      {notice ? <div className="toast" role="status">{notice}</div> : null}

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

      {authState === 'signedIn' && modalKind === 'vault' ? (
        <Modal title="私人保险箱已锁定" actionLabel="知道了" onClose={() => setModalKind(null)}>
          <p>保险箱解锁需要由 Rust 后端安全处理。此 UI 不会在浏览器状态中读取或保存 Safe Key。</p>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'settings' ? (
        <Modal title="设置" actionLabel="知道了" onClose={() => setModalKind(null)}>
          <p>当前会话令牌只保存在内存中，退出登录或关闭应用后会清除。</p>
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
