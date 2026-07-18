import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { SplitUploadDialog } from './SplitUploadDialog'

describe('SplitUploadDialog', () => {
  it('submits a custom package name and part size in bytes', async () => {
    const user = userEvent.setup()
    const onConfirm = vi.fn()
    render(
      <SplitUploadDialog fileName="movie.mkv" onClose={vi.fn()} onConfirm={onConfirm} />,
    )

    const packageName = screen.getByRole('textbox', { name: '远端文件夹名称' })
    const partSize = screen.getByRole('spinbutton', { name: '每个分卷大小（MiB）' })
    await user.clear(packageName)
    await user.type(packageName, '旅行视频分卷')
    await user.clear(partSize)
    await user.type(partSize, '128')
    await user.click(screen.getByRole('button', { name: '开始分卷上传' }))

    expect(onConfirm).toHaveBeenCalledWith({
      packageName: '旅行视频分卷',
      partBytes: 128 * 1024 * 1024,
    })
  })

  it('does not allow a part size outside the supported range', async () => {
    const user = userEvent.setup()
    render(
      <SplitUploadDialog fileName="archive.zip" onClose={vi.fn()} onConfirm={vi.fn()} />,
    )

    const partSize = screen.getByRole('spinbutton', { name: '每个分卷大小（MiB）' })
    await user.clear(partSize)
    await user.type(partSize, '0')

    expect(screen.getByRole('button', { name: '开始分卷上传' }).hasAttribute('disabled')).toBe(true)
  })

  it('rejects a remote path separator in the package name', async () => {
    const user = userEvent.setup()
    render(
      <SplitUploadDialog fileName="archive.zip" onClose={vi.fn()} onConfirm={vi.fn()} />,
    )

    const packageName = screen.getByRole('textbox', { name: '远端文件夹名称' })
    await user.clear(packageName)
    await user.type(packageName, 'archive/parts')

    expect(screen.getByRole('button', { name: '开始分卷上传' }).hasAttribute('disabled')).toBe(true)
  })

  it('rejects dot path segments as a remote folder name', async () => {
    const user = userEvent.setup()
    render(
      <SplitUploadDialog fileName="archive.zip" onClose={vi.fn()} onConfirm={vi.fn()} />,
    )

    const packageName = screen.getByRole('textbox', { name: '远端文件夹名称' })
    await user.clear(packageName)
    await user.type(packageName, '..')

    expect(screen.getByRole('button', { name: '开始分卷上传' }).hasAttribute('disabled')).toBe(true)
  })
})
