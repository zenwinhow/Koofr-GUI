import {
  ArrowDownToLine,
  ArrowUp,
  Check,
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  LayoutList,
  Palette,
  Pencil,
  RefreshCw,
  Search,
  Share2,
  Trash2,
} from 'lucide-react'
import { useDeferredValue, useEffect, useMemo, useRef, useState } from 'react'
import { FileTypeIcon } from '../../components/FileTypeIcon'
import type { KoofrMount, RemoteFile } from '../../types/backend'
import type { UploadMode } from '../../types/files'
import { UploadModeMenu } from './UploadModeMenu'
import {
  fileKind,
  formatBytes,
  formatModified,
  isDirectory,
  pathCrumbs,
} from './filePresentation'

interface FileWorkspaceProps {
  mounts: KoofrMount[]
  activeMountId: string
  path: string
  files: RemoteFile[]
  loading: boolean
  error: string
  lastSyncedAt: Date | null
  onMountChange: (mountId: string) => void
  onNavigate: (path: string) => void
  onRefresh: () => void
  onCreateFolder: () => void
  onThemeOpen: () => void
  onUpload: (mode: UploadMode) => void
  onDownload: (file: RemoteFile) => void
  onShare: (file: RemoteFile) => void
  onRename: (file: RemoteFile) => void
  onDelete: (files: RemoteFile[]) => void
}

const EMPTY_SELECTION = new Set<string>()

function directoryHeading(path: string, activeMount?: KoofrMount) {
  if (path === '/') return activeMount?.name || '我的文件'
  return path.split('/').filter(Boolean).at(-1) ?? '我的文件'
}

function parentDirectory(path: string) {
  const segments = path.split('/').filter(Boolean)
  segments.pop()
  return segments.length > 0 ? `/${segments.join('/')}` : '/'
}

export function FileWorkspace({
  mounts,
  activeMountId,
  path,
  files,
  loading,
  error,
  lastSyncedAt,
  onMountChange,
  onNavigate,
  onRefresh,
  onCreateFolder,
  onThemeOpen,
  onUpload,
  onDownload,
  onShare,
  onRename,
  onDelete,
}: FileWorkspaceProps) {
  const [query, setQuery] = useState('')
  const [selection, setSelection] = useState<{ scope: string; ids: Set<string> }>({
    scope: '',
    ids: EMPTY_SELECTION,
  })
  const [newMenuOpen, setNewMenuOpen] = useState(false)
  const newMenuRef = useRef<HTMLDivElement>(null)
  const deferredQuery = useDeferredValue(query)
  const scope = `${activeMountId}:${path}`
  const selectedIds = selection.scope === scope ? selection.ids : EMPTY_SELECTION
  const activeMount = mounts.find((mount) => mount.id === activeMountId)

  useEffect(() => {
    if (!newMenuOpen) return
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (event.target instanceof Node && !newMenuRef.current?.contains(event.target)) {
        setNewMenuOpen(false)
      }
    }
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setNewMenuOpen(false)
    }
    document.addEventListener('pointerdown', closeOnOutsidePointer)
    document.addEventListener('keydown', closeOnEscape)
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePointer)
      document.removeEventListener('keydown', closeOnEscape)
    }
  }, [newMenuOpen])

  const visibleFiles = useMemo(() => {
    const normalizedQuery = deferredQuery.trim().toLocaleLowerCase('zh-CN')
    const matches = normalizedQuery
      ? files.filter((file) => file.name.toLocaleLowerCase('zh-CN').includes(normalizedQuery))
      : files
    return [...matches].sort((left, right) => {
      const directoryOrder = Number(isDirectory(right)) - Number(isDirectory(left))
      return directoryOrder || left.name.localeCompare(right.name, 'zh-CN', { numeric: true })
    })
  }, [deferredQuery, files])

  const selectedFiles = useMemo(
    () => files.filter((file) => selectedIds.has(file.path)),
    [files, selectedIds],
  )

  const toggleSelection = (id: string) => {
    setSelection((current) => {
      const next = new Set(current.scope === scope ? current.ids : EMPTY_SELECTION)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return { scope, ids: next }
    })
  }

  const allVisibleSelected = visibleFiles.length > 0
    && visibleFiles.every((file) => selectedIds.has(file.path))
  const toggleAll = () => {
    setSelection((current) => {
      const next = new Set(current.scope === scope ? current.ids : EMPTY_SELECTION)
      visibleFiles.forEach((file) => {
        if (allVisibleSelected) next.delete(file.path)
        else next.add(file.path)
      })
      return { scope, ids: next }
    })
  }

  const singleSelection = selectedFiles.length === 1 ? selectedFiles[0] : null
  const downloadableSelection = singleSelection
  const crumbs = pathCrumbs(path)
  const syncedText = lastSyncedAt
    ? `同步于 ${lastSyncedAt.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}`
    : '尚未同步'

  return (
    <main className="workspace">
      <div className="workspace__topbar">
        <nav className="breadcrumb" aria-label="当前文件夹路径">
          <select
            className="mount-select"
            value={activeMountId}
            aria-label="存储位置"
            disabled={loading || mounts.length === 0}
            onChange={(event) => onMountChange(event.target.value)}
          >
            {mounts.map((mount) => <option value={mount.id} key={mount.id}>{mount.name}</option>)}
          </select>
          <span>/</span>
          <button type="button" onClick={() => onNavigate('/')} disabled={path === '/'}>根目录</button>
          {crumbs.map((crumb) => (
            <span className="breadcrumb__segment" key={crumb.path}>
              <span>/</span>
              <button type="button" onClick={() => onNavigate(crumb.path)} disabled={crumb.path === path}>
                {crumb.label}
              </button>
            </span>
          ))}
        </nav>
        <label className="search-box">
          <Search size={18} />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="筛选当前文件夹"
            aria-label="筛选当前文件夹"
          />
        </label>
        <div className="top-actions">
          <UploadModeMenu disabled={!activeMountId || loading} onSelect={onUpload} />
          <div className="menu-anchor" ref={newMenuRef}>
            <button
              className="secondary-button"
              type="button"
              disabled={!activeMountId || loading}
              aria-haspopup="menu"
              aria-expanded={newMenuOpen}
              onClick={() => setNewMenuOpen((open) => !open)}
            >
              新建<ChevronDown size={16} />
            </button>
            {newMenuOpen ? (
              <div className="new-menu" role="menu">
                <button role="menuitem" type="button" onClick={() => { onCreateFolder(); setNewMenuOpen(false) }}>
                  <Folder size={17} />新建文件夹
                </button>
              </div>
            ) : null}
          </div>
          <button className="icon-button icon-button--bordered" type="button" aria-label="刷新当前文件夹" disabled={loading} onClick={onRefresh}>
            <RefreshCw className={loading ? 'icon-spin' : ''} size={19} />
          </button>
          <button className="icon-button icon-button--bordered theme-button-mobile" type="button" aria-label="外观主题" onClick={onThemeOpen}>
            <Palette size={19} />
          </button>
        </div>
      </div>

      <header className="workspace__heading">
        <h1>{directoryHeading(path, activeMount)}</h1>
        <p>{files.length} 个项目 · {loading ? '正在同步…' : syncedText}</p>
      </header>

      {selectedFiles.length > 0 ? (
        <div className="selection-toolbar" aria-label="所选文件操作">
          <strong>{selectedFiles.length}</strong><span>已选中</span><i />
          <button type="button" disabled={!downloadableSelection} onClick={() => downloadableSelection && onDownload(downloadableSelection)}>
            <ArrowDownToLine size={18} />下载
          </button>
          <button type="button" disabled={!singleSelection} onClick={() => singleSelection && onShare(singleSelection)}>
            <Share2 size={17} />分享
          </button>
          <button type="button" disabled={!singleSelection} onClick={() => singleSelection && onRename(singleSelection)}>
            <Pencil size={17} />重命名
          </button>
          <button className="selection-action--danger" type="button" onClick={() => onDelete(selectedFiles)}>
            <Trash2 size={17} />删除
          </button>
        </div>
      ) : <div className="selection-toolbar selection-toolbar--empty"><span>选择项目以查看可用操作</span></div>}

      <section className="file-list" aria-label="文件列表" aria-busy={loading}>
        <div className="file-row file-row--header" role="row">
          <button className={`checkbox${allVisibleSelected ? ' checkbox--checked' : ''}`} type="button" onClick={toggleAll} aria-label="全选当前文件">
            {allVisibleSelected ? <Check size={13} /> : null}
          </button>
          <button className="column-sort" type="button">名称<ArrowUp size={14} /></button>
          <span>所有者</span>
          <span>修改时间</span>
          <span>大小</span>
          <LayoutList size={18} />
        </div>

        {path !== '/' ? (
          <div className="file-row file-row--parent" role="row">
            <span aria-hidden="true" />
            <button className="file-name file-name--button" type="button" onClick={() => onNavigate(parentDirectory(path))}>
              <FileTypeIcon kind="folder" />
              <strong>..</strong>
            </button>
            <span>返回上一级目录</span>
            <span />
            <span>—</span>
            <button className="row-action" type="button" aria-label="返回上一级目录" onClick={() => onNavigate(parentDirectory(path))}>
              <ArrowUp size={18} />
            </button>
          </div>
        ) : null}

        {visibleFiles.map((file) => {
          const kind = fileKind(file)
          const selected = selectedIds.has(file.path)
          const directory = isDirectory(file)
          return (
            <div className={`file-row${selected ? ' file-row--selected' : ''}`} role="row" key={file.path}>
              <button
                className={`checkbox${selected ? ' checkbox--checked' : ''}`}
                type="button"
                onClick={() => toggleSelection(file.path)}
                aria-label={`${selected ? '取消选择' : '选择'} ${file.name}`}
              >
                {selected ? <Check size={13} /> : null}
              </button>
              <button
                className="file-name file-name--button"
                type="button"
                onDoubleClick={() => directory ? onNavigate(file.path) : onDownload(file)}
                onClick={() => directory ? onNavigate(file.path) : toggleSelection(file.path)}
              >
                <FileTypeIcon kind={kind} />
                <strong>{file.name}</strong>
              </button>
              <span>我</span>
              <span>{formatModified(file.modified)}</span>
              <span>{directory ? '—' : formatBytes(file.size)}</span>
              <button
                className="row-action"
                type="button"
                aria-label={directory ? `打开 ${file.name}` : `下载 ${file.name}`}
                onClick={() => directory ? onNavigate(file.path) : onDownload(file)}
              >
                {directory ? <ChevronRight size={19} /> : <ArrowDownToLine size={18} />}
              </button>
            </div>
          )
        })}

        {loading ? (
          <div className="empty-state">
            <RefreshCw className="icon-spin" size={27} />
            <strong>正在读取 Koofr 文件</strong>
            <span>目录内容将在连接完成后显示</span>
          </div>
        ) : null}

        {!loading && error ? (
          <div className="empty-state empty-state--error">
            <FileText size={27} />
            <strong>无法显示这个文件夹</strong>
            <span>{error}</span>
            <button className="secondary-button" type="button" onClick={onRefresh}>重试</button>
          </div>
        ) : null}

        {!loading && !error && visibleFiles.length === 0 ? (
          <div className="empty-state">
            {query ? <Search size={27} /> : <Folder size={27} />}
            <strong>{query ? '没有匹配的项目' : '这个文件夹是空的'}</strong>
            <span>{query ? '试试筛选其他文件名' : '上传文件或新建文件夹以开始使用'}</span>
          </div>
        ) : null}
      </section>
    </main>
  )
}
