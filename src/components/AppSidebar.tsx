import {
  Clock3,
  Folder,
  HelpCircle,
  LockKeyhole,
  LogOut,
  Palette,
  Settings,
  Share2,
  Trash2,
} from 'lucide-react'
import { formatBytes } from '../features/files/filePresentation'
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
  onThemeClick: () => void
  onVaultClick: () => void
  onLogoutClick: () => void
  storageName: string
  storageUsed: number | null
  storageTotal: number | null
}

export function AppSidebar({
  activeItem,
  onSelect,
  onSettingsClick,
  onThemeClick,
  onVaultClick,
  onLogoutClick,
  storageName,
  storageUsed,
  storageTotal,
}: AppSidebarProps) {
  const storagePercent = storageUsed !== null && storageTotal !== null && storageTotal > 0
    ? Math.min(100, Math.max(0, (storageUsed / storageTotal) * 100))
    : 0

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
          <span>{storageName || '存储空间'}</span>
          <small>
            {storageTotal !== null && storageTotal > 0
              ? `已使用 ${formatBytes(storageUsed)} / ${formatBytes(storageTotal)}`
              : '容量信息不可用'}
          </small>
          <div className="storage__track" aria-label={`已使用 ${storagePercent.toFixed(1)}% 存储空间`}>
            <span style={{ width: `${storagePercent}%` }} />
          </div>
        </div>
        <button className="footer-link" type="button" onClick={onThemeClick}>
          <Palette size={17} />
          皮肤
        </button>
        <button className="footer-link" type="button" onClick={onSettingsClick}>
          <Settings size={17} />
          设置
        </button>
        <button className="footer-link" type="button" onClick={onLogoutClick}>
          <LogOut size={17} />
          退出登录
        </button>
        <button className="footer-link" type="button">
          <HelpCircle size={17} />
          帮助与反馈
        </button>
      </div>
    </aside>
  )
}
