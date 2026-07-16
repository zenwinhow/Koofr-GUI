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
    onDownloadSettingsChange: vi.fn(),
    onBrowseDownloadDirectory: vi.fn(async () => null),
    onClearCache: vi.fn(),
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
    await user.click(screen.getByRole('button', { name: '保存路径' }))

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
