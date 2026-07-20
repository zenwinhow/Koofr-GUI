import {
  Ban,
  Bug,
  Database,
  Download,
  FolderOpen,
  HardDrive,
  KeyRound,
  MemoryStick,
  ShieldCheck,
  Trash2,
} from 'lucide-react'
import { useEffect, useState } from 'react'
import type { AppSettings, CacheMode, LogLevel } from '../../types/backend'
import { formatBytes } from '../files/filePresentation'

interface SettingsPanelProps {
  readonly settings: AppSettings | null
  readonly loading: boolean
  readonly busy: boolean
  readonly error: string
  readonly downloadError: string
  readonly onCacheModeChange: (mode: CacheMode) => void
  readonly onCacheTtlChange: (minutes: number) => void
  readonly onCacheDirectoryChange: (directory: string) => void
  readonly onLoggingSettingsChange: (settings: {
    logDirectory: string
    logLevel: LogLevel
    logRetentionDays: number
    logMaxFileSizeMb: number
  }) => void
  readonly onDownloadSettingsChange: (directory: string, askDownloadLocation: boolean) => void
  readonly onBrowseDownloadDirectory: () => Promise<string | null>
  readonly onBrowseSettingsDirectory: (kind: 'cache' | 'logs') => Promise<string | null>
  readonly onClearCache: () => void
  readonly onClearLogs: () => void
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
  onCacheDirectoryChange,
  onLoggingSettingsChange,
  onDownloadSettingsChange,
  onBrowseDownloadDirectory,
  onBrowseSettingsDirectory,
  onClearCache,
  onClearLogs,
  onForgetLogin,
}: SettingsPanelProps) {
  const [downloadDirectory, setDownloadDirectory] = useState(settings?.downloadDirectory ?? '')
  const [cacheDirectory, setCacheDirectory] = useState(settings?.cacheDirectory ?? '')
  const [logDirectory, setLogDirectory] = useState(settings?.logDirectory ?? '')
  const [logLevel, setLogLevel] = useState<LogLevel>(settings?.logLevel ?? 'info')
  const [logRetentionDays, setLogRetentionDays] = useState(settings?.logRetentionDays ?? 14)
  const [logMaxFileSizeMb, setLogMaxFileSizeMb] = useState(settings?.logMaxFileSizeMb ?? 10)
  const [browsing, setBrowsing] = useState<'download' | 'cache' | 'logs' | null>(null)

  useEffect(() => {
    if (!settings) return
    setDownloadDirectory(settings.downloadDirectory)
    setCacheDirectory(settings.cacheDirectory)
    setLogDirectory(settings.logDirectory)
    setLogLevel(settings.logLevel)
    setLogRetentionDays(settings.logRetentionDays)
    setLogMaxFileSizeMb(settings.logMaxFileSizeMb)
  }, [settings])

  if (loading || !settings) {
    return <div className="settings-loading"><span className="auth-spinner" />正在读取本地设置…</div>
  }

  const normalizedDownloadDirectory = downloadDirectory.trim()
  const normalizedCacheDirectory = cacheDirectory.trim()
  const normalizedLogDirectory = logDirectory.trim()
  const downloadDirectoryChanged = normalizedDownloadDirectory !== settings.downloadDirectory
  const cacheDirectoryChanged = normalizedCacheDirectory !== settings.cacheDirectory
  const loggingChanged = normalizedLogDirectory !== settings.logDirectory
    || logLevel !== settings.logLevel
    || logRetentionDays !== settings.logRetentionDays
    || logMaxFileSizeMb !== settings.logMaxFileSizeMb
  const browseDownloadDirectory = async () => {
    setBrowsing('download')
    try {
      const selected = await onBrowseDownloadDirectory()
      if (selected) {
        setDownloadDirectory(selected)
        onDownloadSettingsChange(selected, settings.askDownloadLocation)
      }
    } finally {
      setBrowsing(null)
    }
  }
  const browseStorageDirectory = async (kind: 'cache' | 'logs') => {
    setBrowsing(kind)
    try {
      const selected = await onBrowseSettingsDirectory(kind)
      if (selected) {
        if (kind === 'cache') setCacheDirectory(selected)
        else setLogDirectory(selected)
      }
    } finally {
      setBrowsing(null)
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
              disabled={busy || browsing !== null}
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
            aria-label="保存下载路径"
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

        <div className="settings-path-field settings-path-field--spaced">
          <label htmlFor="settings-cache-directory">缓存文件夹</label>
          <span className="path-field__control">
            <input
              id="settings-cache-directory"
              value={cacheDirectory}
              disabled={busy}
              onChange={(event) => setCacheDirectory(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && normalizedCacheDirectory) {
                  onCacheDirectoryChange(normalizedCacheDirectory)
                }
              }}
            />
            <button
              type="button"
              aria-label="选择缓存文件夹"
              title="选择文件夹"
              disabled={busy || browsing !== null}
              onClick={() => void browseStorageDirectory('cache')}
            >
              <FolderOpen aria-hidden="true" />
            </button>
          </span>
        </div>
        <div className="settings-path-actions">
          <small>仅磁盘模式会写入这里；切换位置时会安全迁移当前缓存。</small>
          <button
            type="button"
            aria-label="保存缓存路径"
            disabled={busy || !cacheDirectoryChanged || !normalizedCacheDirectory}
            onClick={() => onCacheDirectoryChange(normalizedCacheDirectory)}
          >
            保存路径
          </button>
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
          <Bug aria-hidden="true" />
          <div>
            <h3>诊断日志</h3>
            <p>用于定位上传、下载和网络错误；不会记录令牌、邮箱、文件名或完整路径。</p>
          </div>
        </header>

        <div className="settings-path-field">
          <label htmlFor="settings-log-directory">日志文件夹</label>
          <span className="path-field__control">
            <input
              id="settings-log-directory"
              value={logDirectory}
              disabled={busy}
              onChange={(event) => setLogDirectory(event.target.value)}
            />
            <button
              type="button"
              aria-label="选择日志文件夹"
              title="选择文件夹"
              disabled={busy || browsing !== null}
              onClick={() => void browseStorageDirectory('logs')}
            >
              <FolderOpen aria-hidden="true" />
            </button>
          </span>
        </div>

        <div className="settings-row settings-row--compact-grid">
          <label htmlFor="settings-log-level">
            <strong>记录级别</strong>
            <small>排查问题时可临时切换到“调试”</small>
          </label>
          <select
            id="settings-log-level"
            aria-label="记录级别"
            value={logLevel}
            disabled={busy}
            onChange={(event) => setLogLevel(event.target.value as LogLevel)}
          >
            <option value="error">仅错误</option>
            <option value="warn">警告及错误</option>
            <option value="info">常规信息</option>
            <option value="debug">调试信息</option>
          </select>
          <label htmlFor="settings-log-retention">
            <strong>保留时间</strong>
            <small>轮转日志超过此时间会自动删除</small>
          </label>
          <select
            id="settings-log-retention"
            aria-label="保留时间"
            value={logRetentionDays}
            disabled={busy}
            onChange={(event) => setLogRetentionDays(Number(event.target.value))}
          >
            <option value={3}>3 天</option>
            <option value={7}>7 天</option>
            <option value={14}>14 天</option>
            <option value={30}>30 天</option>
            <option value={90}>90 天</option>
          </select>
          <label htmlFor="settings-log-size">
            <strong>单文件上限</strong>
            <small>达到上限后创建新的日志文件</small>
          </label>
          <select
            id="settings-log-size"
            aria-label="单文件上限"
            value={logMaxFileSizeMb}
            disabled={busy}
            onChange={(event) => setLogMaxFileSizeMb(Number(event.target.value))}
          >
            <option value={1}>1 MB</option>
            <option value={5}>5 MB</option>
            <option value={10}>10 MB</option>
            <option value={25}>25 MB</option>
            <option value={50}>50 MB</option>
          </select>
        </div>

        <div className="settings-storage">
          <span>{settings.logFiles} 个日志文件</span>
          <span>磁盘占用 {formatBytes(settings.logDiskBytes)}</span>
          <button
            type="button"
            disabled={busy || !loggingChanged || !normalizedLogDirectory}
            onClick={() => onLoggingSettingsChange({
              logDirectory: normalizedLogDirectory,
              logLevel,
              logRetentionDays,
              logMaxFileSizeMb,
            })}
          >
            保存日志设置
          </button>
          <button type="button" disabled={busy || settings.logFiles === 0} onClick={onClearLogs}>
            <Trash2 aria-hidden="true" />清除日志
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
