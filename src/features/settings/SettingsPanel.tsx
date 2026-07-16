import { Ban, Database, Download, FolderOpen, HardDrive, KeyRound, MemoryStick, ShieldCheck, Trash2 } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { AppSettings, CacheMode } from '../../types/backend'
import { formatBytes } from '../files/filePresentation'

interface SettingsPanelProps {
  readonly settings: AppSettings | null
  readonly loading: boolean
  readonly busy: boolean
  readonly error: string
  readonly downloadError: string
  readonly onCacheModeChange: (mode: CacheMode) => void
  readonly onCacheTtlChange: (minutes: number) => void
  readonly onDownloadSettingsChange: (directory: string, askDownloadLocation: boolean) => void
  readonly onBrowseDownloadDirectory: () => Promise<string | null>
  readonly onClearCache: () => void
  readonly onForgetLogin: () => void
}

const CACHE_OPTIONS = [
  {
    id: 'off',
    title: '不缓存',
    description: '每次打开目录都从 Koofr 获取最新数据。',
    icon: Ban,
  },
  {
    id: 'memory',
    title: '仅内存',
    description: '本次运行期间复用数据，关闭应用后自动清除。',
    icon: MemoryStick,
  },
  {
    id: 'disk',
    title: '本地磁盘',
    description: '重启应用后仍可复用文件元数据。',
    icon: HardDrive,
  },
] satisfies Array<{
  id: CacheMode
  title: string
  description: string
  icon: typeof Ban
}>

export function SettingsPanel({
  settings,
  loading,
  busy,
  error,
  downloadError,
  onCacheModeChange,
  onCacheTtlChange,
  onDownloadSettingsChange,
  onBrowseDownloadDirectory,
  onClearCache,
  onForgetLogin,
}: SettingsPanelProps) {
  const [downloadDirectory, setDownloadDirectory] = useState(settings?.downloadDirectory ?? '')
  const [browsing, setBrowsing] = useState(false)

  useEffect(() => {
    if (settings) setDownloadDirectory(settings.downloadDirectory)
  }, [settings])

  if (loading || !settings) {
    return <div className="settings-loading"><span className="auth-spinner" />正在读取本地设置…</div>
  }

  const normalizedDownloadDirectory = downloadDirectory.trim()
  const downloadDirectoryChanged = normalizedDownloadDirectory !== settings.downloadDirectory
  const browseDownloadDirectory = async () => {
    setBrowsing(true)
    try {
      const selected = await onBrowseDownloadDirectory()
      if (selected) {
        setDownloadDirectory(selected)
        onDownloadSettingsChange(selected, settings.askDownloadLocation)
      }
    } finally {
      setBrowsing(false)
    }
  }

  return (
    <div className="settings-panel">
      <section className="settings-section settings-section--download">
        <header className="settings-section__heading">
          <Download aria-hidden="true" />
          <div>
            <h3>下载</h3>
            <p>设置默认保存位置，以及下载前是否再次确认。</p>
          </div>
        </header>

        <div className="settings-path-field">
          <label htmlFor="settings-download-directory">默认下载位置</label>
          <span className="path-field__control">
            <input
              id="settings-download-directory"
              value={downloadDirectory}
              disabled={busy}
              aria-invalid={downloadError ? true : undefined}
              aria-describedby={downloadError ? 'settings-download-error' : 'settings-download-hint'}
              onChange={(event) => setDownloadDirectory(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && normalizedDownloadDirectory) {
                  onDownloadSettingsChange(normalizedDownloadDirectory, settings.askDownloadLocation)
                }
              }}
            />
            <button
              type="button"
              aria-label="选择默认下载文件夹"
              title="选择文件夹"
              disabled={busy || browsing}
              onClick={() => void browseDownloadDirectory()}
            >
              <FolderOpen aria-hidden="true" />
            </button>
          </span>
        </div>
        <div className="settings-path-actions">
          <small id="settings-download-hint">可直接填写完整路径，也可以使用右侧按钮选择。</small>
          <button
            type="button"
            disabled={busy || !downloadDirectoryChanged || !normalizedDownloadDirectory}
            onClick={() => onDownloadSettingsChange(
              normalizedDownloadDirectory,
              settings.askDownloadLocation,
            )}
          >
            保存路径
          </button>
        </div>
        {downloadError ? (
          <p className="field-message field-message--error" id="settings-download-error" role="alert">
            {downloadError}
          </p>
        ) : null}

        <div className="settings-switch-row">
          <span>
            <strong>每次下载前询问保存位置</strong>
            <small>关闭后将直接下载到上面的默认文件夹。</small>
          </span>
          <button
            className={`settings-switch${settings.askDownloadLocation ? ' settings-switch--on' : ''}`}
            type="button"
            role="switch"
            aria-checked={settings.askDownloadLocation}
            aria-label="每次下载前询问保存位置"
            disabled={busy || !normalizedDownloadDirectory}
            onClick={() => onDownloadSettingsChange(
              normalizedDownloadDirectory,
              !settings.askDownloadLocation,
            )}
          >
            <span aria-hidden="true" />
          </button>
        </div>
      </section>

      <section className="settings-section">
        <header className="settings-section__heading">
          <Database aria-hidden="true" />
          <div>
            <h3>文件信息缓存</h3>
            <p>减少重复网络请求；点击文件页面的刷新按钮仍会强制获取最新数据。</p>
          </div>
        </header>

        <div className="cache-mode-grid" role="radiogroup" aria-label="缓存位置">
          {CACHE_OPTIONS.map((option) => {
            const Icon = option.icon
            const selected = settings.cacheMode === option.id
            return (
              <button
                className={`cache-mode${selected ? ' cache-mode--selected' : ''}`}
                type="button"
                role="radio"
                aria-checked={selected}
                disabled={busy}
                key={option.id}
                onClick={() => onCacheModeChange(option.id)}
              >
                <Icon aria-hidden="true" />
                <span><strong>{option.title}</strong><small>{option.description}</small></span>
                <i aria-hidden="true" />
              </button>
            )
          })}
        </div>
        {settings.cacheMode === 'disk' ? (
          <p className="settings-note">磁盘缓存包含普通文件名和远程路径，但不包含密码、令牌或文件内容。</p>
        ) : null}

        <div className="settings-row">
          <label htmlFor="cache-ttl">
            <strong>缓存有效期</strong>
            <small>超过有效期后会自动重新获取</small>
          </label>
          <select
            id="cache-ttl"
            value={settings.cacheTtlMinutes}
            disabled={busy || settings.cacheMode === 'off'}
            onChange={(event) => onCacheTtlChange(Number(event.target.value))}
          >
            <option value={5}>5 分钟</option>
            <option value={15}>15 分钟</option>
            <option value={60}>1 小时</option>
            <option value={360}>6 小时</option>
            <option value={1440}>24 小时</option>
          </select>
        </div>

        <div className="settings-storage">
          <span>当前缓存 {settings.cachedItems} 项</span>
          <span>磁盘占用 {formatBytes(settings.cacheDiskBytes)}</span>
          <button type="button" disabled={busy || settings.cachedItems === 0} onClick={onClearCache}>
            <Trash2 aria-hidden="true" />清除缓存
          </button>
        </div>
      </section>

      <section className="settings-section">
        <header className="settings-section__heading">
          <KeyRound aria-hidden="true" />
          <div>
            <h3>保存的登录信息</h3>
            <p>应用专用密码仅保存在 Windows 凭据管理器中，不会写入设置文件。</p>
          </div>
        </header>
        {settings.savedEmail ? (
          <div className="saved-login">
            <span className="saved-login__icon"><ShieldCheck aria-hidden="true" /></span>
            <span><strong>{settings.savedEmail}</strong><small>下次启动时自动连接</small></span>
            <button type="button" disabled={busy} onClick={onForgetLogin}>删除</button>
          </div>
        ) : (
          <div className="saved-login saved-login--empty">
            登录时勾选“保存密码”即可启用自动连接。
          </div>
        )}
      </section>

      {error ? <div className="settings-error" role="alert">{error}</div> : null}
    </div>
  )
}
