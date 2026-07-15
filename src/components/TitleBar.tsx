import { Minus, Square, X } from 'lucide-react'
import { BrandMark } from './BrandMark'

export function TitleBar() {
  return (
    <header className="titlebar">
      <BrandMark compact />
      <div className="window-controls" aria-label="窗口控件预览">
        <span><Minus size={16} /></span>
        <span><Square size={13} /></span>
        <span><X size={16} /></span>
      </div>
    </header>
  )
}
