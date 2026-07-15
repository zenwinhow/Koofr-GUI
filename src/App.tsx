import { useState } from 'react'
import { AppSidebar } from './components/AppSidebar'
import { Modal } from './components/Modal'
import { ThemePicker } from './components/ThemePicker'
import { TitleBar } from './components/TitleBar'
import { FileWorkspace } from './features/files/FileWorkspace'
import { initialFiles } from './features/files/fileData'
import { TransferPanel } from './features/transfers/TransferPanel'
import { readStoredTheme, storeTheme, type ThemeId } from './theme'
import type { CloudFile } from './types/files'

type ModalKind = 'settings' | 'upload' | 'vault' | null

function App() {
  const [activeItem, setActiveItem] = useState('我的文件')
  const [files, setFiles] = useState<CloudFile[]>(initialFiles)
  const [modalKind, setModalKind] = useState<ModalKind>(null)
  const [transferVisible, setTransferVisible] = useState(true)
  const [notice, setNotice] = useState('')
  const [themeId, setThemeId] = useState<ThemeId>(readStoredTheme)
  const itemCount = 18 + files.length - initialFiles.length

  const selectTheme = (nextTheme: ThemeId) => {
    setThemeId(nextTheme)
    storeTheme(nextTheme)
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
      <div className="app-body">
        <AppSidebar
          activeItem={activeItem}
          onSelect={selectNavigation}
          onSettingsClick={() => setModalKind('settings')}
          onVaultClick={() => setModalKind('vault')}
        />
        <FileWorkspace
          files={files}
          itemCount={itemCount}
          onCreateFolder={createFolder}
          onThemeOpen={() => setModalKind('settings')}
          onUpload={() => setModalKind('upload')}
        />
        <TransferPanel visible={transferVisible} onClose={() => setTransferVisible(false)} />
        {transferVisible ? null : (
          <button className="transfer-reopen" type="button" onClick={() => setTransferVisible(true)}>
            传输 1
          </button>
        )}
      </div>

      {notice ? <div className="toast" role="status">{notice}</div> : null}

      {modalKind === 'upload' ? (
        <Modal title="上传文件" actionLabel="选择本地文件" onClose={() => setModalKind(null)}>
          <p>上传界面已经就绪。文件选择与流式传输将在 Tauri 后端命令接入后启用。</p>
        </Modal>
      ) : null}

      {modalKind === 'vault' ? (
        <Modal title="私人保险箱已锁定" actionLabel="知道了" onClose={() => setModalKind(null)}>
          <p>保险箱解锁需要由 Rust 后端安全处理。此 UI 不会在浏览器状态中读取或保存 Safe Key。</p>
        </Modal>
      ) : null}

      {modalKind === 'settings' ? (
        <Modal title="外观主题" actionLabel="完成" onClose={() => setModalKind(null)}>
          <ThemePicker value={themeId} onChange={selectTheme} />
        </Modal>
      ) : null}
    </div>
  )
}

export default App
