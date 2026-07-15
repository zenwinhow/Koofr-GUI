import { useEffect, useState } from 'react'
import { AppSidebar } from './components/AppSidebar'
import { Modal } from './components/Modal'
import { ThemePicker } from './components/ThemePicker'
import { TitleBar } from './components/TitleBar'
import { LoginPage } from './features/auth/LoginPage'
import { FileWorkspace } from './features/files/FileWorkspace'
import { initialFiles } from './features/files/fileData'
import { TransferPanel } from './features/transfers/TransferPanel'
import { commandErrorMessage, isTauriRuntime, koofr } from './services/koofr'
import { readStoredTheme, storeTheme, type ThemeId } from './theme'
import type { CloudFile } from './types/files'

type ModalKind = 'settings' | 'theme' | 'upload' | 'vault' | null
type AuthState = 'checking' | 'signedOut' | 'signingIn' | 'signedIn'

function App() {
  const [activeItem, setActiveItem] = useState('我的文件')
  const [authState, setAuthState] = useState<AuthState>('checking')
  const [loginError, setLoginError] = useState('')
  const [files, setFiles] = useState<CloudFile[]>(initialFiles)
  const [modalKind, setModalKind] = useState<ModalKind>(null)
  const [transferVisible, setTransferVisible] = useState(true)
  const [notice, setNotice] = useState('')
  const [themeId, setThemeId] = useState<ThemeId>(readStoredTheme)
  const itemCount = 18 + files.length - initialFiles.length

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
      setAuthState('signedOut')
    } catch (error) {
      setNotice(commandErrorMessage(error, '暂时无法退出登录，请重试。'))
      window.setTimeout(() => setNotice(''), 2600)
    }
  }

  const createFolder = () => {
    setFiles((current) => [
      { id: `folder-${current.length + 1}`, name: '新建文件夹', kind: 'folder', owner: '我', modifiedAt: '刚刚', size: '—' },
      ...current,
    ])
    setNotice('已在演示列表中创建“新建文件夹”')
    window.setTimeout(() => setNotice(''), 2600)
  }

  const selectNavigation = (label: string) => {
    setActiveItem(label)
    if (label !== '我的文件') {
      setNotice(`${label} 页面将在后续迭代中接入`)
      window.setTimeout(() => setNotice(''), 2600)
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
          />
          <FileWorkspace
            files={files}
            itemCount={itemCount}
            onCreateFolder={createFolder}
            onThemeOpen={() => setModalKind('theme')}
            onUpload={() => setModalKind('upload')}
          />
          <TransferPanel visible={transferVisible} onClose={() => setTransferVisible(false)} />
          {transferVisible ? null : (
            <button className="transfer-reopen" type="button" onClick={() => setTransferVisible(true)}>
              传输 1
            </button>
          )}
        </div>
      ) : null}

      {notice ? <div className="toast" role="status">{notice}</div> : null}

      {authState === 'signedIn' && modalKind === 'upload' ? (
        <Modal title="上传文件" actionLabel="选择本地文件" onClose={() => setModalKind(null)}>
          <p>上传界面已经就绪。文件选择与流式传输将在文件工作区接入后启用。</p>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'vault' ? (
        <Modal title="私人保险箱已锁定" actionLabel="知道了" onClose={() => setModalKind(null)}>
          <p>保险箱解锁需要由 Rust 后端安全处理。此 UI 不会在浏览器状态中读取或保存 Safe Key。</p>
        </Modal>
      ) : null}

      {authState === 'signedIn' && modalKind === 'settings' ? (
        <Modal title="设置" actionLabel="知道了" onClose={() => setModalKind(null)}>
          <p>应用设置界面将在后续迭代中接入。</p>
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
