import {
  ArrowDownToLine,
  ArrowUp,
  Check,
  ChevronDown,
  Copy,
  FileImage,
  FileSpreadsheet,
  FileText,
  Folder,
  FolderInput,
  LayoutList,
  MoreHorizontal,
  Palette,
  Pencil,
  RefreshCw,
  Search,
  Share2,
  Trash2,
  UploadCloud,
} from 'lucide-react'
import { useDeferredValue, useMemo, useState } from 'react'
import type { CloudFile, FileKind } from '../../types/files'

interface FileWorkspaceProps {
  files: CloudFile[]
  itemCount: number
  onCreateFolder: () => void
  onThemeOpen: () => void
  onUpload: () => void
}

const fileGlyphs: Record<FileKind, typeof Folder> = {
  folder: Folder,
  xlsx: FileSpreadsheet,
  pdf: FileText,
  docx: FileText,
  image: FileImage,
}

export function FileWorkspace({ files, itemCount, onCreateFolder, onThemeOpen, onUpload }: FileWorkspaceProps) {
  const [query, setQuery] = useState('')
  const [selectedIds, setSelectedIds] = useState<Set<string>>(() => new Set(['budget']))
  const [newMenuOpen, setNewMenuOpen] = useState(false)
  const deferredQuery = useDeferredValue(query)

  const visibleFiles = useMemo(() => {
    const normalizedQuery = deferredQuery.trim().toLocaleLowerCase('zh-CN')
    return normalizedQuery
      ? files.filter((file) => file.name.toLocaleLowerCase('zh-CN').includes(normalizedQuery))
      : files
  }, [deferredQuery, files])

  const toggleSelection = (id: string) => {
    setSelectedIds((current) => {
      const next = new Set(current)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const allVisibleSelected = visibleFiles.length > 0 && visibleFiles.every((file) => selectedIds.has(file.id))
  const toggleAll = () => {
    setSelectedIds((current) => {
      const next = new Set(current)
      visibleFiles.forEach((file) => allVisibleSelected ? next.delete(file.id) : next.add(file.id))
      return next
    })
  }

  return (
    <main className="workspace">
      <div className="workspace__topbar">
        <div className="breadcrumb"><span>Koofr</span><span>/</span><strong>我的文件</strong></div>
        <label className="search-box">
          <Search size={18} />
          <input
            type="search"
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            placeholder="搜索文件和文件夹"
            aria-label="搜索文件和文件夹"
          />
        </label>
        <div className="top-actions">
          <button className="primary-button" type="button" onClick={onUpload}>
            <UploadCloud size={18} />上传
          </button>
          <div className="menu-anchor">
            <button className="secondary-button" type="button" onClick={() => setNewMenuOpen((open) => !open)}>
              新建<ChevronDown size={16} />
            </button>
            {newMenuOpen ? (
              <div className="new-menu">
                <button type="button" onClick={() => { onCreateFolder(); setNewMenuOpen(false) }}>
                  <Folder size={17} />新建文件夹
                </button>
              </div>
            ) : null}
          </div>
          <button className="icon-button icon-button--bordered" type="button" aria-label="更多操作">
            <MoreHorizontal size={20} />
          </button>
          <button className="icon-button icon-button--bordered theme-button-mobile" type="button" aria-label="外观主题" onClick={onThemeOpen}>
            <Palette size={19} />
          </button>
        </div>
      </div>

      <header className="workspace__heading">
        <h1>我的文件</h1>
        <p>{itemCount} 个项目 · 刚刚同步 <RefreshCw size={14} aria-hidden="true" /></p>
      </header>

      {selectedIds.size > 0 ? (
        <div className="selection-toolbar" aria-label="所选文件操作">
          <strong>{selectedIds.size}</strong><span>已选中</span><i />
          <button type="button"><Share2 size={17} />分享</button>
          <button type="button"><ArrowDownToLine size={18} />下载</button>
          <button type="button"><FolderInput size={18} />移动</button>
          <button type="button"><Copy size={17} />复制</button>
          <button type="button"><Pencil size={17} />重命名</button>
          <button type="button"><Trash2 size={17} />删除</button>
          <button type="button" aria-label="更多所选文件操作"><MoreHorizontal size={19} /></button>
        </div>
      ) : <div className="selection-toolbar selection-toolbar--empty"><span>选择项目以查看可用操作</span></div>}

      <section className="file-list" aria-label="文件列表">
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

        {visibleFiles.map((file) => {
          const Icon = fileGlyphs[file.kind]
          const selected = selectedIds.has(file.id)
          return (
            <div className={`file-row${selected ? ' file-row--selected' : ''}`} role="row" key={file.id}>
              <button
                className={`checkbox${selected ? ' checkbox--checked' : ''}`}
                type="button"
                onClick={() => toggleSelection(file.id)}
                aria-label={`${selected ? '取消选择' : '选择'} ${file.name}`}
              >
                {selected ? <Check size={13} /> : null}
              </button>
              <div className="file-name">
                <span className={`file-glyph file-glyph--${file.kind}`}><Icon size={22} strokeWidth={1.8} /></span>
                <strong>{file.name}</strong>
              </div>
              <span>{file.owner}</span>
              <span>{file.modifiedAt}</span>
              <span>{file.size}</span>
              <button className="row-action" type="button" aria-label={`${file.name} 的更多操作`}><MoreHorizontal size={19} /></button>
            </div>
          )
        })}

        {visibleFiles.length === 0 ? (
          <div className="empty-state">
            <Search size={27} />
            <strong>没有匹配的项目</strong>
            <span>试试搜索其他文件名</span>
          </div>
        ) : null}
      </section>
    </main>
  )
}
