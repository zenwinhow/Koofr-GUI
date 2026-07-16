import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { DownloadDestinationDialog } from './DownloadDestinationDialog'

describe('DownloadDestinationDialog', () => {
  it('submits a trimmed manually entered directory', async () => {
    // Given
    const user = userEvent.setup()
    const onConfirm = vi.fn()
    render(
      <DownloadDestinationDialog
        fileName="年度报告.pdf"
        initialDirectory="C:\\Users\\Me\\Downloads"
        initialAskEveryTime={true}
        busy={false}
        error=""
        onBrowse={vi.fn()}
        onClose={vi.fn()}
        onConfirm={onConfirm}
      />,
    )

    // When
    const input = screen.getByLabelText('下载到')
    await user.clear(input)
    await user.type(input, '  D:\\Koofr 下载  ')
    await user.click(screen.getByRole('button', { name: '开始下载' }))

    // Then
    expect(onConfirm).toHaveBeenCalledWith('D:\\Koofr 下载', true)
  })

  it('uses the directory returned by the folder picker', async () => {
    // Given
    const user = userEvent.setup()
    const onBrowse = vi.fn().mockResolvedValue('D:\\Downloads')
    render(
      <DownloadDestinationDialog
        fileName="照片"
        initialDirectory="C:\\Users\\Me\\Downloads"
        initialAskEveryTime={true}
        busy={false}
        error=""
        onBrowse={onBrowse}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    )

    // When
    await user.click(screen.getByRole('button', { name: '选择文件夹' }))

    // Then
    const input = screen.getByLabelText('下载到')
    expect(input).toBeInstanceOf(HTMLInputElement)
    if (!(input instanceof HTMLInputElement)) return
    expect(input.value).toBe('D:\\Downloads')
  })

  it('shows a backend validation error without losing the entered path', () => {
    const invalidPath = String.raw`Z:\不存在`
    render(
      <DownloadDestinationDialog
        fileName="照片"
        initialDirectory={invalidPath}
        initialAskEveryTime={true}
        busy={false}
        error="请选择一个存在且可写入的文件夹。"
        onBrowse={vi.fn()}
        onClose={vi.fn()}
        onConfirm={vi.fn()}
      />,
    )

    // Then
    expect(screen.getByRole('alert').textContent).toContain('请选择一个存在且可写入的文件夹。')
    const input = screen.getByLabelText('下载到')
    expect(input).toBeInstanceOf(HTMLInputElement)
    if (!(input instanceof HTMLInputElement)) return
    expect(input.value).toBe(invalidPath)
  })

  it('lets the user disable future destination prompts before downloading', async () => {
    // Given
    const user = userEvent.setup()
    const onConfirm = vi.fn()
    const directory = String.raw`D:\Downloads`
    render(
      <DownloadDestinationDialog
        fileName="照片.zip"
        initialDirectory={directory}
        initialAskEveryTime={true}
        busy={false}
        error=""
        onBrowse={vi.fn()}
        onClose={vi.fn()}
        onConfirm={onConfirm}
      />,
    )

    // When
    await user.click(screen.getByRole('switch', { name: '每次下载前询问保存位置' }))
    await user.click(screen.getByRole('button', { name: '开始下载' }))

    // Then
    expect(onConfirm).toHaveBeenCalledWith(directory, false)
  })
})
