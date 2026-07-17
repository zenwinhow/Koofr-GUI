import {
  ArrowDownToLine,
  Check,
  Clock3,
  FileText,
  FolderOpen,
  LayoutList,
  Palette,
  RefreshCw,
  RotateCcw,
  Search,
  Share2,
  Trash2,
} from 'lucide-react'
import { useDeferredValue, useMemo, useState } from 'react'
import { FileTypeIcon } from '../../components/FileTypeIcon'
import type { LocatedFile, RemoteFile, TrashItem } from '../../types/backend'
import type { CollectionView } from './useKoofrCollections'
import { fileKind, formatBytes, formatModified, isDirectory } from './filePresentation'

interface CollectionWorkspaceProps {
  view: CollectionView
  files: LocatedFile[]
  trash: TrashItem[]
  retentionDays: number
  loading: boolean
  error: string
  diagnostic: string
  lastSyncedAt: Date | null
  busy: boolean
  onRefresh: () => void
  onThemeOpen: () => void
  onOpenLocation: (file: LocatedFile) => void
  onDownload: (file: LocatedFile) => void
  onRestore: (items: TrashItem[]) => void
  onRestoreAll: () => void
  onEmptyTrash: () => void
}

const EMPTY_SELECTION = new Set<string>()

const viewDetails = {
  最近的文件: {
    icon: Clock3,
    description: '按最近修改时间展示账户中的文件和文件夹',
    empty: '还没有最近的文件',
  },
  已共享: {
    icon: Share2,
    description: '我已共享以及其他人共享给我的项目',
    empty: '还没有共享项目',
  },
  回收站: {
    icon: Trash2,
    description: '恢复误删项目，或永久清空回收站',
    empty: '回收站是空的',
  },
} satisfies Record<CollectionView, { icon: typeof Clock3; description: string; empty: string }>

const DELETED_DATE_FORMATTER = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hour12: false,
})

function locatedId(file: LocatedFile) {
  return `${file.mountId}:${file.path}`
}

function trashAsFile(item: TrashItem): RemoteFile {
  return {
    name: item.name,
    entryType: 'file',
    modified: 0,
    size: item.size,
    contentType: item.contentType,
    hash: '',
    path: item.path,
  }
}

function formatDeleted(value: string) {
  const normalized = value.trim()
  if (/^\d+$/.test(normalized)) {
    return formatModified(Number(normalized))
  }

  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '—'
  return DELETED_DATE_FORMATTER.format(date)
}

function parentPath(path: string) {
  const segments = path.split('/').filter(Boolean)
  if (segments.length <= 1) return '/'
  return `/${segments.slice(0, -1).join('/')}`
}

function locationLabel(file: LocatedFile) {
  const directory = isDirectory(file)
  const location = directory ? file.path : parentPath(file.path)
  return location === '/' ? file.mountName || '根目录' : `${file.mountName || '存储位置'}${location}`
}

export function CollectionWorkspace({
  view,
  files,
  trash,
  retentionDays,
  loading,
  error,
  diagnostic,
  lastSyncedAt,
  busy,
  onRefresh,
  onThemeOpen,
  onOpenLocation,
  onDownload,
  onRestore,
  onRestoreAll,
  onEmptyTrash,
}: CollectionWorkspaceProps) {
  const [query, setQuery] = useState('')
  const [selection, setSelection] = useState<{ scope: CollectionView; ids: Set<string> }>({
    scope: view,
    ids: EMPTY_SELECTION,
  })
  const deferredQuery = useDeferredValue(query)
  const selectedIds = selection.scope === view ? selection.ids : EMPTY_SELECTION
  const details = viewDetails[view]
  const HeadingIcon = details.icon
  const isTrash = view === '回收站'

  const visibleFiles = useMemo(() => {
    const normalized = deferredQuery.trim().toLocaleLowerCase('zh-CN')
    return normalized
      ? files.filter((file) => file.name.toLocaleLowerCase('zh-CN').includes(normalized))
      : files
  }, [deferredQuery, files])

  const visibleTrash = useMemo(() => {
    const normalized = deferredQuery.trim().toLocaleLowerCase('zh-CN')
    return normalized
      ? trash.filter((item) => item.name.toLocaleLowerCase('zh-CN').includes(normalized))
      : trash
  }, [deferredQuery, trash])

  const selectedFiles = useMemo(
    () => files.filter((file) => selectedIds.has(locatedId(file))),
    [files, selectedIds],
  )
  const selectedTrash = useMemo(
    () => trash.filter((item) => selectedIds.has(item.versionId)),
    [selectedIds, trash],
  )
  const visibleIds = isTrash
    ? visibleTrash.map((item) => item.versionId)
    : visibleFiles.map(locatedId)
  const allVisibleSelected = visibleIds.length > 0 && visibleIds.every((id) => selectedIds.has(id))

  const toggleSelection = (id: string) => {
    setSelection((current) => {
      const next = new Set(current.scope === view ? current.ids : EMPTY_SELECTION)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return { scope: view, ids: next }
    })
  }

  const toggleAll = () => {
    setSelection((current) => {
      const next = new Set(current.scope === view ? current.ids : EMPTY_SELECTION)
      visibleIds.forEach((id) => {
        if (allVisibleSelected) next.delete(id)
        else next.add(id)
      })
      return { scope: view, ids: next }
    })
  }

  const singleFile = selectedFiles.length === 1 ? selectedFiles[0] : null
  const canOpenLocation = singleFile
    && (isDirectory(singleFile) || singleFile.path !== '/' || singleFile.shareDirection === null)
  const itemCount = isTrash ? trash.length : files.length
  const syncedText = lastSyncedAt
    ? `同步于 ${lastSyncedAt.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}`
    : '尚未同步'

  return (
    <main className="workspace collection-workspace">
      <div className="workspace__topbar collection-workspace__topbar">
        <label className="search-box collection-search">
          <Search size={18} />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder={`筛选${view}`}
            aria-label={`筛选${view}`}
          />
        </label>
        <div className="top-actions">
          {isTrash && trash.length > 0 ? (
            <>
              <button className="secondary-button" type="button" disabled={busy || loading} onClick={onRestoreAll}>
                <RotateCcw size={17} />恢复全部
              </button>
              <button className="danger-button" type="button" disabled={busy || loading} onClick={onEmptyTrash}>
                <Trash2 size={17} />永久清空
              </button>
            </>
          ) : null}
          <button className="icon-button icon-button--bordered" type="button" aria-label={`刷新${view}`} disabled={loading || busy} onClick={onRefresh}>
            <RefreshCw className={loading ? 'icon-spin' : ''} size={19} />
          </button>
          <button className="icon-button icon-button--bordered theme-button-mobile" type="button" aria-label="外观主题" onClick={onThemeOpen}>
            <Palette size={19} />
          </button>
        </div>
      </div>

      <header className="workspace__heading collection-heading">
        <span className="collection-heading__icon"><HeadingIcon size={24} /></span>
        <div>
          <h1>{view}</h1>
          <p>
            {details.description} · {itemCount} 个项目 · {loading ? '正在同步…' : syncedText}
            {isTrash && retentionDays > 0 ? ` · 保留 ${retentionDays} 天` : ''}
          </p>
        </div>
      </header>

      {isTrash && selectedTrash.length > 0 ? (
        <div className="selection-toolbar" aria-label="所选回收站项目操作">
          <strong>{selectedTrash.length}</strong><span>已选中</span><i />
          <button type="button" disabled={busy} onClick={() => onRestore(selectedTrash)}>
            <RotateCcw size={18} />恢复所选
          </button>
        </div>
      ) : !isTrash && selectedFiles.length > 0 ? (
        <div className="selection-toolbar" aria-label="所选文件操作">
          <strong>{selectedFiles.length}</strong><span>已选中</span><i />
          <button type="button" disabled={!singleFile} onClick={() => singleFile && onDownload(singleFile)}>
            <ArrowDownToLine size={18} />下载
          </button>
          <button type="button" disabled={!canOpenLocation} onClick={() => singleFile && onOpenLocation(singleFile)}>
            <FolderOpen size={18} />打开位置
          </button>
        </div>
      ) : (
        <div className="selection-toolbar selection-toolbar--empty">
          <span>{isTrash ? '选择项目后可以恢复' : '选择项目以查看可用操作'}</span>
        </div>
      )}

      <section className="file-list collection-list" aria-label={`${view}列表`} aria-busy={loading}>
        <div className="file-row file-row--header collection-row" role="row">
          <button className={`checkbox${allVisibleSelected ? ' checkbox--checked' : ''}`} type="button" onClick={toggleAll} aria-label={`全选${view}`}>
            {allVisibleSelected ? <Check size={13} /> : null}
          </button>
          <span>名称</span>
          <span>位置</span>
          <span>{isTrash ? '删除时间' : '修改时间'}</span>
          <span>大小</span>
          <LayoutList size={18} />
        </div>

        {!isTrash ? visibleFiles.map((file) => {
          const id = locatedId(file)
          const selected = selectedIds.has(id)
          const directory = isDirectory(file)
          const direction = file.shareDirection === 'received' ? '共享给我' : file.shareDirection === 'outgoing' ? '我已共享' : null
          return (
            <div className={`file-row collection-row${selected ? ' file-row--selected' : ''}`} role="row" key={id}>
              <button className={`checkbox${selected ? ' checkbox--checked' : ''}`} type="button" onClick={() => toggleSelection(id)} aria-label={`${selected ? '取消选择' : '选择'} ${file.name}`}>
                {selected ? <Check size={13} /> : null}
              </button>
              <button className="file-name file-name--button" type="button" onClick={() => directory ? onOpenLocation(file) : toggleSelection(id)} onDoubleClick={() => directory ? onOpenLocation(file) : onDownload(file)}>
                <FileTypeIcon kind={fileKind(file)} />
                <strong>{file.name}</strong>
              </button>
              <span className="collection-location">{direction ? <em>{direction}</em> : null}{locationLabel(file)}</span>
              <span>{formatModified(file.modified)}</span>
              <span>{directory ? '—' : formatBytes(file.size)}</span>
              <button className="row-action" type="button" aria-label={directory ? `打开 ${file.name}` : `下载 ${file.name}`} onClick={() => directory ? onOpenLocation(file) : onDownload(file)}>
                {directory ? <FolderOpen size={18} /> : <ArrowDownToLine size={18} />}
              </button>
            </div>
          )
        }) : visibleTrash.map((item) => {
          const selected = selectedIds.has(item.versionId)
          const file = trashAsFile(item)
          const kind = fileKind(file)
          return (
            <div className={`file-row collection-row${selected ? ' file-row--selected' : ''}`} role="row" key={item.versionId}>
              <button className={`checkbox${selected ? ' checkbox--checked' : ''}`} type="button" onClick={() => toggleSelection(item.versionId)} aria-label={`${selected ? '取消选择' : '选择'} ${item.name}`}>
                {selected ? <Check size={13} /> : null}
              </button>
              <button className="file-name file-name--button" type="button" onClick={() => toggleSelection(item.versionId)}>
                <FileTypeIcon kind={kind} />
                <strong>{item.name}</strong>
              </button>
              <span className="collection-location">{item.mountName || '存储位置'}{parentPath(item.path)}</span>
              <span>{formatDeleted(item.deleted)}</span>
              <span>{formatBytes(item.size)}</span>
              <button className="row-action" type="button" aria-label={`恢复 ${item.name}`} disabled={busy} onClick={() => onRestore([item])}>
                <RotateCcw size={18} />
              </button>
            </div>
          )
        })}

        {loading ? (
          <div className="empty-state">
            <RefreshCw className="icon-spin" size={27} />
            <strong>正在读取{view}</strong>
            <span>内容将在 Koofr 响应后显示</span>
          </div>
        ) : null}

        {!loading && error ? (
          <div className="empty-state empty-state--error">
            <FileText size={27} />
            <strong>无法显示{view}</strong>
            <span>{error}</span>
            {diagnostic ? <code className="error-diagnostic">诊断：{diagnostic}</code> : null}
            <button className="secondary-button" type="button" onClick={onRefresh}>重试</button>
          </div>
        ) : null}

        {!loading && !error && (isTrash ? visibleTrash.length === 0 : visibleFiles.length === 0) ? (
          <div className="empty-state">
            {query ? <Search size={27} /> : <HeadingIcon size={27} />}
            <strong>{query ? '没有匹配的项目' : details.empty}</strong>
            <span>{query ? '试试筛选其他文件名' : isTrash ? '删除的项目会在这里显示' : '这里会显示来自 Koofr 的真实内容'}</span>
          </div>
        ) : null}
      </section>
    </main>
  )
}
