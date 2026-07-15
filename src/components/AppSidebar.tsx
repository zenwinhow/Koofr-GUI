import {
  Cloud,
  Clock3,
  Folder,
  HelpCircle,
  LockKeyhole,
  Settings,
  Share2,
  Trash2,
} from 'lucide-react'
import { BrandMark } from './BrandMark'

const primaryItems = [
  { label: '我的文件', icon: Folder },
  { label: '最近使用', icon: Clock3 },
  { label: '与我共享', icon: Share2 },
  { label: '回收站', icon: Trash2 },
]

interface AppSidebarProps {
  activeItem: string
  onSelect: (label: string) => void
  onSettingsClick: () => void
  onVaultClick: () => void
}

export function AppSidebar({ activeItem, onSelect, onSettingsClick, onVaultClick }: AppSidebarProps) {
  return (
    <aside className="sidebar" aria-label="主导航">
      <div className="sidebar__brand">
        <BrandMark />
      </div>

      <nav className="sidebar__nav">
        {primaryItems.map(({ label, icon: Icon }) => (
          <button
            className={`nav-item${activeItem === label ? ' nav-item--active' : ''}`}
            key={label}
            type="button"
            onClick={() => onSelect(label)}
            aria-current={activeItem === label ? 'page' : undefined}
          >
            <Icon size={20} strokeWidth={1.9} />
            <span>{label}</span>
          </button>
        ))}
      </nav>

      <div className="sidebar__section">
        <p className="sidebar__section-label">保险箱</p>
        <button className="nav-item nav-item--vault" type="button" onClick={onVaultClick}>
          <LockKeyhole size={19} strokeWidth={1.9} />
          <span>私人保险箱</span>
          <span className="vault-state">已锁定</span>
        </button>
      </div>

      <div className="sidebar__footer">
        <div className="storage">
          <span>存储空间</span>
          <small>已使用&nbsp; 12.4 GB / 50 GB</small>
          <div className="storage__track" aria-label="已使用 24.8% 存储空间">
            <span />
          </div>
        </div>
        <button className="upgrade-button" type="button">
          <Cloud size={17} />
          升级存储空间
        </button>
        <button className="footer-link" type="button" onClick={onSettingsClick}>
          <Settings size={17} />
          设置
        </button>
        <button className="footer-link" type="button">
          <HelpCircle size={17} />
          帮助与反馈
        </button>
      </div>
    </aside>
  )
}
