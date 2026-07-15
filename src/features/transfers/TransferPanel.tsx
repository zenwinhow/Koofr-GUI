import { ArrowDownToLine, ArrowUpToLine, Trash2, X } from 'lucide-react'
import { formatBytes } from '../files/filePresentation'
import type { TransferItem } from '../../types/files'

interface TransferPanelProps {
  visible: boolean
  items: TransferItem[]
  onClose: () => void
  onCancel: (transferId: string) => void
  onClearFinished: () => void
}

const stateLabels = {
  running: '正在传输',
  completed: '已完成',
  cancelled: '已取消',
  failed: '失败',
} as const

export function TransferPanel({
  visible,
  items,
  onClose,
  onCancel,
  onClearFinished,
}: TransferPanelProps) {
  const runningCount = items.filter((item) => item.state === 'running').length
  const finishedCount = items.length - runningCount

  return (
    <aside className={`transfer-panel${visible ? '' : ' transfer-panel--hidden'}`} aria-label="传输队列">
      <div className="transfer-panel__header">
        <div>
          <h2>传输</h2>
          <span>{runningCount > 0 ? `${runningCount} 项进行中` : '当前没有传输'}</span>
        </div>
        <button className="icon-button" type="button" aria-label="关闭传输面板" onClick={onClose}>
          <X size={19} />
        </button>
      </div>

      <div className="transfer-list">
        {items.map((item) => {
          const percent = item.totalBytes && item.totalBytes > 0
            ? Math.min(100, (item.bytesTransferred / item.totalBytes) * 100)
            : item.state === 'completed' ? 100 : 0
          const DirectionIcon = item.direction === 'upload' ? ArrowUpToLine : ArrowDownToLine
          return (
            <div className="transfer-item" key={item.id}>
              <span className="file-glyph file-glyph--file file-glyph--small"><DirectionIcon size={20} /></span>
              <div className="transfer-item__content">
                <strong title={item.name}>{item.name}</strong>
                <div className="transfer-item__meta">
                  <span>{stateLabels[item.state]} · {formatBytes(item.bytesTransferred)}</span>
                  <span className="transfer-item__percent">{Math.round(percent)}%</span>
                </div>
                <div className="progress-track"><span style={{ width: `${percent}%` }} /></div>
              </div>
              {item.state === 'running' ? (
                <button className="row-action" type="button" aria-label={`取消 ${item.name}`} onClick={() => onCancel(item.id)}>
                  <X size={16} />
                </button>
              ) : null}
            </div>
          )
        })}

        {items.length === 0 ? (
          <div className="transfer-empty">
            <ArrowUpToLine size={25} />
            <strong>传输队列为空</strong>
            <span>上传和下载任务会显示在这里</span>
          </div>
        ) : null}
      </div>

      <div className="transfer-panel__footer">
        <span>{items.length} 个任务</span>
        <button type="button" disabled={finishedCount === 0} onClick={onClearFinished}>
          <Trash2 size={17} />清除已完成
        </button>
      </div>
    </aside>
  )
}
