import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { AppSettings } from '../../types/backend'
import { SettingsPanel } from './SettingsPanel'

const SETTINGS: AppSettings = {
  cacheMode: 'memory',
  cacheTtlMinutes: 15,
  cachedItems: 0,
  cacheDiskBytes: 0,
  savedEmail: null,
  downloadDirectory: 'C:\\Users\\Test\\Downloads',
  askDownloadLocation: true,
  workDirectory: 'C:\\Users\\Test\\AppData\\Koofr-GUI',
  pendingWorkDirectory: null,
  pendingWorkDirectoryMove: false,
  workDirectoryMigrationFailed: false,
  logLevel: 'info',
  logRetentionDays: 14,
  logMaxFileSizeMb: 10,
  logFiles: 2,
  logDiskBytes: 2048,
  autoRetryNetworkErrors: false,
  networkRetryLimit: 8,
  networkRetryIntervalSeconds: 5,
}

function renderSettings(overrides: Partial<Parameters<typeof SettingsPanel>[0]> = {}) {
  const props: Parameters<typeof SettingsPanel>[0] = {
    settings: SETTINGS,
    loading: false,
    busy: false,
    error: '',
    downloadError: '',
    onCacheModeChange: vi.fn(),
    onCacheTtlChange: vi.fn(),
    onLoggingSettingsChange: vi.fn(),
    onTransferSettingsChange: vi.fn(),
    onDownloadSettingsChange: vi.fn(),
    onBrowseDownloadDirectory: vi.fn(async () => null),
    onBrowseWorkDirectory: vi.fn(async () => null),
    onWorkDirectoryChange: vi.fn(),
    onClearCache: vi.fn(),
    onClearLogs: vi.fn(),
    onForgetLogin: vi.fn(),
    ...overrides,
  }
  render(<SettingsPanel {...props} />)
  return props
}

describe('SettingsPanel download preferences', () => {
  it('saves a manually entered default directory', async () => {
    const user = userEvent.setup()
    const props = renderSettings()
    const input = screen.getByRole('textbox', { name: '默认下载位置' })

    await user.clear(input)
    await user.type(input, 'D:\\Koofr Downloads')
    await user.click(screen.getByRole('button', { name: '保存下载路径' }))

    expect(props.onDownloadSettingsChange).toHaveBeenCalledWith('D:\\Koofr Downloads', true)
  })

  it('saves the directory returned by the folder picker', async () => {
    const user = userEvent.setup()
    const onDownloadSettingsChange = vi.fn()
    renderSettings({
      onBrowseDownloadDirectory: vi.fn(async () => 'D:\\Selected Downloads'),
      onDownloadSettingsChange,
    })

    await user.click(screen.getByRole('button', { name: '选择默认下载文件夹' }))

    expect(onDownloadSettingsChange).toHaveBeenCalledWith('D:\\Selected Downloads', true)
  })

  it('associates a download path error with the path input', () => {
    renderSettings({ downloadError: '请选择一个已有的文件夹。' })

    const input = screen.getByRole('textbox', { name: '默认下载位置' })
    expect(input.getAttribute('aria-invalid')).toBe('true')
    expect(input.getAttribute('aria-describedby')).toBe('settings-download-error')
    expect(screen.getByRole('alert').textContent).toContain('请选择一个已有的文件夹。')
  })
})

describe('SettingsPanel storage and diagnostics', () => {
  it('schedules a selected work directory with a complete migration', async () => {
    const user = userEvent.setup()
    const onWorkDirectoryChange = vi.fn()
    renderSettings({
      onBrowseWorkDirectory: vi.fn(async () => 'D:\\Koofr Work'),
      onWorkDirectoryChange,
    })

    await user.click(screen.getByRole('button', { name: '选择应用工作目录' }))
    await user.click(screen.getByRole('button', { name: '保存工作目录' }))

    expect(onWorkDirectoryChange).toHaveBeenCalledWith('D:\\Koofr Work', true)
  })

  it('can leave the previous work directory in place', async () => {
    const user = userEvent.setup()
    const onWorkDirectoryChange = vi.fn()
    renderSettings({
      onBrowseWorkDirectory: vi.fn(async () => 'D:\\Fresh Koofr Work'),
      onWorkDirectoryChange,
    })

    await user.click(screen.getByRole('button', { name: '选择应用工作目录' }))
    await user.click(screen.getByRole('checkbox', { name: /移动当前工作目录/ }))
    await user.click(screen.getByRole('button', { name: '保存工作目录' }))

    expect(onWorkDirectoryChange).toHaveBeenCalledWith('D:\\Fresh Koofr Work', false)
  })

  it('saves logging location, level, retention and rotation together', async () => {
    const user = userEvent.setup()
    const onLoggingSettingsChange = vi.fn()
    renderSettings({ onLoggingSettingsChange })

    await user.selectOptions(screen.getByRole('combobox', { name: '记录级别' }), 'debug')
    await user.selectOptions(screen.getByRole('combobox', { name: '保留时间' }), '30')
    await user.selectOptions(screen.getByRole('combobox', { name: '单文件上限' }), '25')
    await user.click(screen.getByRole('button', { name: '保存日志设置' }))

    expect(onLoggingSettingsChange).toHaveBeenCalledWith({
      logLevel: 'debug',
      logRetentionDays: 30,
      logMaxFileSizeMb: 25,
    })
  })
})

describe('SettingsPanel transfer recovery', () => {
  it('enables automatic continuation for network errors', async () => {
    const user = userEvent.setup()
    const onTransferSettingsChange = vi.fn()
    renderSettings({ onTransferSettingsChange })

    await user.click(screen.getByRole('switch', { name: '遇到网络错误时自动继续' }))

    expect(onTransferSettingsChange).toHaveBeenCalledWith({
      autoRetryNetworkErrors: true,
      networkRetryLimit: 8,
      networkRetryIntervalSeconds: 5,
    })
  })
})
