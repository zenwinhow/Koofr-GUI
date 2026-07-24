import {
  Ban,
  Bug,
  Database,
  Download,
  FolderOpen,
  HardDrive,
  KeyRound,
  MemoryStick,
  RefreshCw,
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
  readonly onLoggingSettingsChange: (settings: {
    logLevel: LogLevel
    logRetentionDays: number
    logMaxFileSizeMb: number
  }) => void
  readonly onTransferSettingsChange: (settings: {
    autoRetryNetworkErrors: boolean
    networkRetryLimit: number | null
    networkRetryIntervalSeconds: number
  }) => void
  readonly onDownloadSettingsChange: (directory: string, askDownloadLocation: boolean) => void
  readonly onBrowseDownloadDirectory: () => Promise<string | null>
  readonly onBrowseWorkDirectory: () => Promise<string | null>
  readonly onWorkDirectoryChange: (directory: string, moveExisting: boolean) => void
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
  onLoggingSettingsChange,
  onTransferSettingsChange,
  onDownloadSettingsChange,
  onBrowseDownloadDirectory,
  onBrowseWorkDirectory,
  onWorkDirectoryChange,
  onClearCache,
  onClearLogs,
  onForgetLogin,
}: SettingsPanelProps) {
  const [downloadDirectory, setDownloadDirectory] = useState(settings?.downloadDirectory ?? '')
  const [workDirectory, setWorkDirectory] = useState(
    settings?.pendingWorkDirectory ?? settings?.workDirectory ?? '',
  )
  const [moveExisting, setMoveExisting] = useState(settings?.pendingWorkDirectoryMove ?? true)
  const [logLevel, setLogLevel] = useState<LogLevel>(settings?.logLevel ?? 'info')
  const [logRetentionDays, setLogRetentionDays] = useState(settings?.logRetentionDays ?? 14)
  const [logMaxFileSizeMb, setLogMaxFileSizeMb] = useState(settings?.logMaxFileSizeMb ?? 10)
  const [browsing, setBrowsing] = useState<'download' | 'work' | null>(null)

  useEffect(() => {
    if (!settings) return
    setDownloadDirectory(settings.downloadDirectory)
    setWorkDirectory(settings.pendingWorkDirectory ?? settings.workDirectory)
    setMoveExisting(settings.pendingWorkDirectoryMove || settings.pendingWorkDirectory === null)
    setLogLevel(settings.logLevel)
    setLogRetentionDays(settings.logRetentionDays)
    setLogMaxFileSizeMb(settings.logMaxFileSizeMb)
  }, [settings])

  if (loading || !settings) {
    return <div className="settings-loading"><span className="auth-spinner" />正在读取本地设置…</div>
  }

  const normalizedDownloadDirectory = downloadDirectory.trim()
  const normalizedWorkDirectory = workDirectory.trim()
  const downloadDirectoryChanged = normalizedDownloadDirectory !== settings.downloadDirectory
  const configuredWorkDirectory = settings.pendingWorkDirectory ?? settings.workDirectory
  const workDirectoryChanged = normalizedWorkDirectory !== configuredWorkDirectory
    || (
      settings.pendingWorkDirectory !== null
      && moveExisting !== settings.pendingWorkDirectoryMove
    )
  const loggingChanged = logLevel !== settings.logLevel
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
  const browseWorkDirectory = async () => {
    setBrowsing('work')
    try {
      const selected = await onBrowseWorkDirectory()
      if (selected) setWorkDirectory(selected)
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
          <RefreshCw aria-hidden="true" />
          <div>
            <h3>传输恢复</h3>
            <p>控制网络中断后的自动恢复行为，其他类型错误仍会立即停止。</p>
          </div>
        </header>
        <div className="settings-switch-row settings-switch-row--standalone">
          <span>
            <strong>遇到网络错误时自动继续</strong>
            <small>只处理 network_error。下载和分卷上传从断点继续，普通上传会从头重试。</small>
          </span>
          <button
            className={`settings-switch${settings.autoRetryNetworkErrors ? ' settings-switch--on' : ''}`}
            type="button"
            role="switch"
            aria-checked={settings.autoRetryNetworkErrors}
            aria-label="遇到网络错误时自动继续"
            disabled={busy}
            onClick={() => onTransferSettingsChange({
              autoRetryNetworkErrors: !settings.autoRetryNetworkErrors,
              networkRetryLimit: settings.networkRetryLimit,
              networkRetryIntervalSeconds: settings.networkRetryIntervalSeconds,
            })}
          >
            <span aria-hidden="true" />
          </button>
        </div>
        <div className="settings-row settings-row--compact-grid">
          <label htmlFor="settings-network-retry-limit">
            <strong>最大重试次数</strong>
            <small>不包括最初的传输请求；可选择一直重试</small>
          </label>
          <select
            id="settings-network-retry-limit"
            aria-label="最大重试次数"
            value={settings.networkRetryLimit ?? 'unlimited'}
            disabled={busy || !settings.autoRetryNetworkErrors}
            onChange={(event) => onTransferSettingsChange({
              autoRetryNetworkErrors: settings.autoRetryNetworkErrors,
              networkRetryLimit: event.target.value === 'unlimited'
                ? null
                : Number(event.target.value),
              networkRetryIntervalSeconds: settings.networkRetryIntervalSeconds,
            })}
          >
            <option value={3}>3 次</option>
            <option value={5}>5 次</option>
            <option value={8}>8 次</option>
            <option value={10}>10 次</option>
            <option value={20}>20 次</option>
            <option value={50}>50 次</option>
            <option value="unlimited">无限重试</option>
          </select>
          <label htmlFor="settings-network-retry-interval">
            <strong>重试间隔</strong>
            <small>每次网络错误后等待相同时间再继续</small>
          </label>
          <select
            id="settings-network-retry-interval"
            aria-label="重试间隔"
            value={settings.networkRetryIntervalSeconds}
            disabled={busy || !settings.autoRetryNetworkErrors}
            onChange={(event) => onTransferSettingsChange({
              autoRetryNetworkErrors: settings.autoRetryNetworkErrors,
              networkRetryLimit: settings.networkRetryLimit,
              networkRetryIntervalSeconds: Number(event.target.value),
            })}
          >
            <option value={1}>1 秒</option>
            <option value={3}>3 秒</option>
            <option value={5}>5 秒</option>
            <option value={10}>10 秒</option>
            <option value={15}>15 秒</option>
            <option value={30}>30 秒</option>
            <option value={60}>1 分钟</option>
          </select>
        </div>
      </section>

      <section className="settings-section">
        <header className="settings-section__heading">
          <HardDrive aria-hidden="true" />
          <div>
            <h3>应用工作目录</h3>
            <p>统一保存设置、传输检查点、下载历史、磁盘缓存和诊断日志。</p>
          </div>
        </header>

        <div className="settings-path-field">
          <label htmlFor="settings-work-directory">工作配置目录</label>
          <span className="path-field__control">
            <input
              id="settings-work-directory"
              value={workDirectory}
              disabled={busy}
              onChange={(event) => setWorkDirectory(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter' && normalizedWorkDirectory && workDirectoryChanged) {
                  onWorkDirectoryChange(normalizedWorkDirectory, moveExisting)
                }
              }}
            />
            <button
              type="button"
              aria-label="选择应用工作目录"
              title="选择文件夹"
              disabled={busy || browsing !== null}
              onClick={() => void browseWorkDirectory()}
            >
              <FolderOpen aria-hidden="true" />
            </button>
          </span>
        </div>

        <label className="settings-migration-option">
          <input
            type="checkbox"
            checked={moveExisting}
            disabled={busy}
            onChange={(event) => setMoveExisting(event.target.checked)}
          />
          <span>
            <strong>移动当前工作目录中的全部文件</strong>
            <small>取消勾选会从空目录开始，原目录及其中的数据保持不变。</small>
          </span>
        </label>

        {settings.pendingWorkDirectory ? (
          <p className={`settings-note${settings.workDirectoryMigrationFailed ? ' settings-note--error' : ''}`}>
            {settings.workDirectoryMigrationFailed
              ? '上次迁移未完成；应用仍保留可用数据，并会在下次启动时重试。'
              : '目录更改已安排，将在下次启动、打开任何工作文件之前应用。'}
          </p>
        ) : null}

        <div className="settings-path-actions">
          <small>目标必须是空的本地文件夹，且不能位于当前工作目录内部或外层。</small>
          <button
            type="button"
            aria-label="保存工作目录"
            disabled={busy || !workDirectoryChanged || !normalizedWorkDirectory}
            onClick={() => onWorkDirectoryChange(normalizedWorkDirectory, moveExisting)}
          >
            保存并在重启后应用
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
          <Bug aria-hidden="true" />
          <div>
            <h3>诊断日志</h3>
            <p>用于定位上传、下载和网络错误；不会记录令牌、邮箱、文件名或完整路径。</p>
          </div>
        </header>

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
            disabled={busy || !loggingChanged}
            onClick={() => onLoggingSettingsChange({
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
