import { useEffect, useMemo, useState } from 'react'
import {
  ExternalLink,
  FolderOpen,
  Gauge,
  Pause,
  Play,
  RotateCcw,
  Search,
  Trash2,
  X,
} from 'lucide-react'
import { FileTypeIcon } from '../../components/FileTypeIcon'
import { formatBytes } from '../files/filePresentation'
import type { RecoveryKind } from '../../types/backend'
import type { FileKind, TransferItem } from '../../types/files'

interface TransferPanelProps {
  visible: boolean
  items: TransferItem[]
  onClose: () => void
  onCancel: (transferId: string) => void
  onPause: (transferId: string) => void
  onResume: (transferId: string) => void
  onDiscard: (transferId: string) => void
  onOpenFile: (transferId: string) => void
  onOpenFolder: (transferId: string) => void
  onClearFinished: () => void
}

const stateLabels = {
  running: '正在传输',
  retrying: '等待网络重试',
  paused: '已暂停',
  completed: '已完成',
  cancelled: '已取消',
  failed: '失败',
} as const

const recoveryActions = {
  byte_resume: { label: '继续下载', Icon: Play },
  chunk_resume: { label: '继续上传', Icon: Play },
  restart: { label: '重新上传', Icon: RotateCcw },
} satisfies Record<RecoveryKind, { readonly label: string; readonly Icon: typeof Play }>

const DATE_FORMATTER = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  second: '2-digit',
  hour12: false,
})
const CHART_WIDTH = 300
const CHART_TOP = 12
const CHART_BOTTOM = 76
const CHART_WINDOW_MS = 60 * 1000

interface SpeedPoint {
  readonly recordedAt: number
  readonly value: number
}

interface ChartPoint {
  readonly x: number
  readonly y: number
}

function fileKindForTransfer(item: TransferItem): FileKind {
  if (item.localKind === 'folder') return 'folder'
  const extension = item.name.split('.').pop()?.toLocaleLowerCase('en-US') ?? ''
  if (['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz'].includes(extension)) return 'archive'
  if (['exe', 'msi', 'msix', 'appx', 'bat', 'cmd', 'ps1'].includes(extension)) return 'executable'
  if (['xlsx', 'xls', 'ods'].includes(extension)) return 'xlsx'
  if (extension === 'pdf') return 'pdf'
  if (['docx', 'doc', 'odt', 'md', 'txt'].includes(extension)) return 'docx'
  if (['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'].includes(extension)) return 'image'
  return 'file'
}

function percentage(item: TransferItem) {
  if (item.totalBytes && item.totalBytes > 0) {
    return Math.min(100, (item.bytesTransferred / item.totalBytes) * 100)
  }
  return item.state === 'completed' ? 100 : 0
}

function formatDate(timestamp: number | null) {
  if (timestamp === null || !Number.isFinite(timestamp)) return '—'
  return DATE_FORMATTER.format(new Date(timestamp))
}

function formatRate(value: number) {
  if (!Number.isFinite(value) || value <= 0) return '0 B'
  if (value < 1) return `${value.toFixed(1)} B`
  return formatBytes(value)
}

function averageBytesPerSecond(item: TransferItem) {
  const first = item.speedSamples[0]
  const last = item.speedSamples.at(-1)
  if (first && last && last.recordedAt > first.recordedAt) {
    return Math.max(0, (last.bytesTransferred - first.bytesTransferred)
      / ((last.recordedAt - first.recordedAt) / 1000))
  }
  if (item.startedAt !== null) {
    const end = item.finishedAt ?? Date.now()
    if (end > item.startedAt) {
      return item.bytesTransferred / ((end - item.startedAt) / 1000)
    }
  }
  return 0
}

function currentBytesPerSecond(item: TransferItem) {
  const samples = item.speedSamples
  const current = samples.at(-1)
  const previous = samples.at(-2)
  if (!current || !previous || current.recordedAt <= previous.recordedAt) return 0
  return Math.max(0, (current.bytesTransferred - previous.bytesTransferred)
    / ((current.recordedAt - previous.recordedAt) / 1000))
}

function speedSeries(item: TransferItem) {
  return item.speedSamples.slice(1).map((sample, index) => {
    const previous = item.speedSamples[index]
    const elapsedSeconds = (sample.recordedAt - previous.recordedAt) / 1000
    return {
      recordedAt: sample.recordedAt,
      value: elapsedSeconds > 0
        ? Math.max(0, (sample.bytesTransferred - previous.bytesTransferred) / elapsedSeconds)
        : 0,
    }
  })
}

function chartPoints(item: TransferItem) {
  const series = speedSeries(item)
  const lastSampleAt = item.speedSamples.at(-1)?.recordedAt ?? item.startedAt ?? Date.now()
  const windowEnd = item.finishedAt ?? lastSampleAt
  const windowStart = windowEnd - CHART_WINDOW_MS
  const visible = series.filter((point) => point.recordedAt >= windowStart)
  const fallbackValue = averageBytesPerSecond(item)
  const firstValue = visible[0]?.value ?? fallbackValue
  const lastValue = visible.at(-1)?.value ?? fallbackValue
  const transferStart = Math.max(item.startedAt ?? windowStart, windowStart)
  const values: SpeedPoint[] = [
    { recordedAt: transferStart, value: firstValue },
    ...visible,
    { recordedAt: windowEnd, value: lastValue },
  ]
  const max = Math.max(...values.map((point) => point.value), 1)
  const points = values.map((point): ChartPoint => ({
    x: Math.min(CHART_WIDTH, Math.max(
      0,
      ((point.recordedAt - windowStart) / CHART_WINDOW_MS) * CHART_WIDTH,
    )),
    y: CHART_BOTTOM - (point.value / max) * (CHART_BOTTOM - CHART_TOP),
  }))
  return { points, max }
}

function smoothPath(points: readonly ChartPoint[]) {
  if (points.length === 0) return ''
  if (points.length === 1) return `M ${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`
  const clampY = (value: number) => Math.min(CHART_BOTTOM, Math.max(CHART_TOP, value))
  let path = `M ${points[0].x.toFixed(1)} ${points[0].y.toFixed(1)}`
  for (let index = 0; index < points.length - 1; index += 1) {
    const previous = points[Math.max(0, index - 1)]
    const current = points[index]
    const next = points[index + 1]
    const after = points[Math.min(points.length - 1, index + 2)]
    const controlOneX = current.x + (next.x - previous.x) / 6
    const controlOneY = clampY(current.y + (next.y - previous.y) / 6)
    const controlTwoX = next.x - (after.x - current.x) / 6
    const controlTwoY = clampY(next.y - (after.y - current.y) / 6)
    path += ` C ${controlOneX.toFixed(1)} ${controlOneY.toFixed(1)}, ${controlTwoX.toFixed(1)} ${controlTwoY.toFixed(1)}, ${next.x.toFixed(1)} ${next.y.toFixed(1)}`
  }
  return path
}

function SpeedChart({ item }: { readonly item: TransferItem }) {
  const [mode, setMode] = useState<'line' | 'smooth'>('line')
  const { points, max } = chartPoints(item)
  const pointList = points.map((point) => (
    `${point.x.toFixed(1)},${point.y.toFixed(1)}`
  )).join(' ')
  const direction = item.direction === 'upload' ? '上传' : '下载'
  const nextMode = mode === 'line' ? '平滑' : '折线'

  return (
    <button
      className="transfer-speed-chart"
      type="button"
      aria-label={`${direction}速度曲线，当前为${mode === 'line' ? '折线' : '平滑'}样式，点击切换为${nextMode}样式`}
      onClick={() => setMode((current) => current === 'line' ? 'smooth' : 'line')}
    >
      <svg viewBox="0 0 300 86" role="img" aria-label={`最近 1 分钟${direction}速度`}>
        <line x1="0" y1="16" x2="300" y2="16" />
        <line x1="0" y1="47" x2="300" y2="47" />
        <line x1="0" y1="78" x2="300" y2="78" />
        {mode === 'line' ? (
          <polyline points={pointList} />
        ) : (
          <path className="transfer-speed-chart__curve" d={smoothPath(points)} />
        )}
      </svg>
      <span className="transfer-speed-chart__peak">峰值 {formatRate(max)}/s</span>
      <span className="transfer-speed-chart__mode">{mode === 'line' ? '折线' : '平滑'}</span>
      <small><span>最近 1 分钟</span><span>现在</span></small>
    </button>
  )
}

export function TransferPanel({
  visible,
  items,
  onClose,
  onCancel,
  onPause,
  onResume,
  onDiscard,
  onOpenFile,
  onOpenFolder,
  onClearFinished,
}: TransferPanelProps) {
  const [query, setQuery] = useState('')
  const [selectedId, setSelectedId] = useState<string | null>(null)
  const orderedItems = useMemo(
    () => [...items].sort((left, right) => (right.startedAt ?? 0) - (left.startedAt ?? 0)),
    [items],
  )
  const filteredItems = useMemo(() => {
    const normalized = query.trim().toLocaleLowerCase()
    if (!normalized) return orderedItems
    return orderedItems.filter((item) => (
      item.name.toLocaleLowerCase().includes(normalized)
      || item.localPath?.toLocaleLowerCase().includes(normalized)
      || item.remotePath?.toLocaleLowerCase().includes(normalized)
    ))
  }, [orderedItems, query])

  useEffect(() => {
    if (filteredItems.length === 0) {
      setSelectedId(null)
      return
    }
    if (!filteredItems.some((item) => item.id === selectedId)) {
      setSelectedId(filteredItems[0].id)
    }
  }, [filteredItems, selectedId])

  const selected = filteredItems.find((item) => item.id === selectedId) ?? null
  const runningCount = items.filter((item) => (
    item.state === 'running' || item.state === 'retrying'
  )).length
  const finishedCount = items.filter((item) => (
    item.state === 'completed'
    || item.state === 'cancelled'
    || (item.state === 'failed' && item.recoveryKind === null)
  )).length

  return (
    <aside className={`transfer-panel${visible ? '' : ' transfer-panel--hidden'}`} aria-label="传输">
      <div className="transfer-panel__header">
        <div>
          <h2>传输</h2>
          <span>{runningCount > 0 ? `${runningCount} 项进行中` : '当前没有进行中的任务'}</span>
        </div>
        <button className="icon-button" type="button" aria-label="关闭传输面板" onClick={onClose}>
          <X size={19} />
        </button>
      </div>

      <div className="transfer-toolbar">
        <label className="transfer-search">
          <Search size={16} aria-hidden="true" />
          <input
            type="search"
            value={query}
            placeholder="搜索传输"
            aria-label="搜索传输"
            onChange={(event) => setQuery(event.target.value)}
          />
        </label>
        <button
          className="transfer-toolbar__button"
          type="button"
          disabled={finishedCount === 0}
          aria-label="清除已完成传输"
          title="清除已完成"
          onClick={onClearFinished}
        >
          <Trash2 size={16} />
        </button>
      </div>

      <div className="transfer-sort-label">
        <strong>按日期排序</strong>
        <span>{items.length} 项</span>
      </div>

      <div className="transfer-list" role="listbox" aria-label="传输列表">
        {filteredItems.map((item) => {
          const percent = percentage(item)
          const recovery = item.recoveryKind ? recoveryActions[item.recoveryKind] : null
          const active = item.state === 'running' || item.state === 'retrying'
          return (
            <div
              className={`transfer-item${selectedId === item.id ? ' transfer-item--selected' : ''}`}
              key={item.id}
              role="option"
              aria-selected={selectedId === item.id}
              tabIndex={0}
              onClick={() => setSelectedId(item.id)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  setSelectedId(item.id)
                }
              }}
            >
              <FileTypeIcon kind={fileKindForTransfer(item)} />
              <div className="transfer-item__content">
                <strong title={item.name}>{item.name}</strong>
                <div className="transfer-item__meta">
                  <span>{stateLabels[item.state]}</span>
                  <span>{formatBytes(item.totalBytes ?? item.bytesTransferred)}</span>
                </div>
                {active || item.state === 'paused' ? (
                  <div className="progress-track" aria-label={`${Math.round(percent)}%`}>
                    <span style={{ width: `${percent}%` }} />
                  </div>
                ) : null}
              </div>
              <div className="transfer-item__trailing">
                {active ? (
                  <button
                    className="row-action"
                    type="button"
                    aria-label={`暂停 ${item.name}`}
                    title="暂停"
                    onClick={(event) => {
                      event.stopPropagation()
                      onPause(item.id)
                    }}
                  >
                    <Pause size={16} />
                  </button>
                ) : null}
                {recovery && (item.state === 'paused' || item.state === 'failed') ? (
                  <button
                    className="row-action"
                    type="button"
                    aria-label={`${recovery.label} ${item.name}`}
                    title={recovery.label}
                    onClick={(event) => {
                      event.stopPropagation()
                      onResume(item.id)
                    }}
                  >
                    <recovery.Icon size={16} />
                  </button>
                ) : null}
                {item.direction === 'download' && item.state === 'completed' ? (
                  <button
                    className="row-action"
                    type="button"
                    aria-label={`打开 ${item.name} 所在文件夹`}
                    title="打开所在文件夹"
                    onClick={(event) => {
                      event.stopPropagation()
                      onOpenFolder(item.id)
                    }}
                  >
                    <FolderOpen size={16} />
                  </button>
                ) : null}
              </div>
            </div>
          )
        })}

        {filteredItems.length === 0 ? (
          <div className="transfer-empty">
            <Search size={25} />
            <strong>{items.length === 0 ? '传输列表为空' : '没有匹配的传输'}</strong>
            <span>{items.length === 0 ? '新的传输任务会显示在这里' : '试试其他文件名或路径'}</span>
          </div>
        ) : null}
      </div>

      {selected ? (
        <section className="transfer-detail" aria-label={`${selected.name} 详情`}>
          <div className="transfer-detail__title">
            <div>
              <strong title={selected.name}>{selected.name}</strong>
              <span>{stateLabels[selected.state]}</span>
            </div>
            <span>{Math.round(percentage(selected))}%</span>
          </div>

          <dl className="transfer-detail__paths">
            <div>
              <dt>来源</dt>
              <dd title={selected.remotePath ?? undefined}>{selected.remotePath ? `Koofr · ${selected.remotePath}` : 'Koofr'}</dd>
            </div>
            <div>
              <dt>文件位置</dt>
              <dd title={selected.localPath ?? undefined}>{selected.localPath ?? '完成后可用'}</dd>
            </div>
          </dl>

          <div className="transfer-detail__facts">
            <div><span>开始时间</span><strong>{formatDate(selected.startedAt)}</strong></div>
            <div><span>完成时间</span><strong>{formatDate(selected.finishedAt)}</strong></div>
            <div><span>文件大小</span><strong>{formatBytes(selected.totalBytes ?? selected.bytesTransferred)}</strong></div>
            <div><span>平均速度</span><strong><Gauge size={14} />{formatRate(averageBytesPerSecond(selected))}/s</strong></div>
          </div>

          <div className="transfer-detail__speed">
            <div>
              <span>速度曲线</span>
              <strong>{formatRate(currentBytesPerSecond(selected))}/s</strong>
            </div>
            <SpeedChart item={selected} />
          </div>

          <div className="transfer-detail__actions">
            {selected.direction === 'download' && selected.state === 'completed' ? (
              <>
                {selected.localKind === 'file' ? (
                  <button type="button" aria-label={`打开文件 ${selected.name}`} onClick={() => onOpenFile(selected.id)}>
                    <ExternalLink size={15} />打开文件
                  </button>
                ) : null}
                <button type="button" aria-label={`在资源管理器中显示 ${selected.name}`} onClick={() => onOpenFolder(selected.id)}>
                  <FolderOpen size={15} />在资源管理器中显示
                </button>
              </>
            ) : null}
            {(selected.state === 'paused' || selected.state === 'failed') && selected.recoveryKind ? (
              <button type="button" aria-label={`在详情中${recoveryActions[selected.recoveryKind].label} ${selected.name}`} onClick={() => onResume(selected.id)}>
                <Play size={15} />{recoveryActions[selected.recoveryKind].label}
              </button>
            ) : null}
            {selected.state === 'running' || selected.state === 'retrying' ? (
              <>
                <button type="button" aria-label={`在详情中暂停 ${selected.name}`} onClick={() => onPause(selected.id)}>
                  <Pause size={15} />暂停
                </button>
                <button type="button" aria-label={`取消 ${selected.name}`} onClick={() => onCancel(selected.id)}>
                  <X size={15} />取消
                </button>
              </>
            ) : null}
            {(selected.state === 'paused' || selected.state === 'failed') && selected.recoveryKind ? (
              <button type="button" aria-label={`在详情中放弃恢复 ${selected.name}`} onClick={() => onDiscard(selected.id)}>
                <Trash2 size={15} />放弃任务
              </button>
            ) : null}
          </div>
        </section>
      ) : null}
    </aside>
  )
}
