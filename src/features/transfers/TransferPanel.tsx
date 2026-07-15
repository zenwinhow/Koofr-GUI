import { ChevronUp, FileText, Pause, Trash2, X } from 'lucide-react'

interface TransferPanelProps {
  visible: boolean
  onClose: () => void
}

export function TransferPanel({ visible, onClose }: TransferPanelProps) {
  return (
    <aside className={`transfer-panel${visible ? '' : ' transfer-panel--hidden'}`} aria-label="传输队列">
      <div className="transfer-panel__header">
        <h2>传输</h2>
        <div className="transfer-panel__header-actions">
          <button className="icon-button" type="button" aria-label="收起传输面板" onClick={onClose}>
            <ChevronUp size={18} />
          </button>
          <button className="icon-button" type="button" aria-label="关闭传输面板" onClick={onClose}>
            <X size={19} />
          </button>
        </div>
      </div>
      <div className="transfer-tabs" role="tablist" aria-label="传输状态">
        <button className="transfer-tab transfer-tab--active" type="button" role="tab" aria-selected="true">
          进行中 <span>1</span>
        </button>
        <button className="transfer-tab" type="button" role="tab" aria-selected="false">已完成</button>
      </div>
      <div className="transfer-item">
        <span className="file-glyph file-glyph--pdf file-glyph--small"><FileText size={21} /></span>
        <div className="transfer-item__content">
          <strong>品牌指南.pdf</strong>
          <div className="transfer-item__meta">
            <span>正在上传 · 3.2 MB / 5.0 MB</span>
            <span className="transfer-item__percent">64%</span>
          </div>
          <div className="progress-track"><span /></div>
        </div>
      </div>
      <div className="transfer-panel__footer">
        <button type="button"><Pause size={17} />全部暂停</button>
        <button type="button"><Trash2 size={17} />清除已完成</button>
      </div>
    </aside>
  )
}
