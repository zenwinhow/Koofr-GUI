import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { TransferPanel } from './TransferPanel'

describe('TransferPanel', () => {
  it('offers byte resume for an interrupted download', async () => {
    const user = userEvent.setup()
    const onResume = vi.fn()
    render(
      <TransferPanel
        visible
        items={[{
          id: 'transfer-1',
          name: 'large.iso',
          direction: 'download',
          state: 'paused',
          bytesTransferred: 64,
          totalBytes: 128,
          localKind: 'file',
          recoveryKind: 'byte_resume',
        }]}
        onClose={vi.fn()}
        onCancel={vi.fn()}
        onPause={vi.fn()}
        onResume={onResume}
        onDiscard={vi.fn()}
        onOpenFile={vi.fn()}
        onOpenFolder={vi.fn()}
        onClearFinished={vi.fn()}
      />,
    )

    await user.click(screen.getByRole('button', { name: '继续下载 large.iso' }))

    expect(onResume).toHaveBeenCalledWith('transfer-1')
    expect(screen.getByText('50%')).toBeTruthy()
    expect(screen.getByText('已暂停 · 64 B')).toBeTruthy()
  })


  it('offers chunk resume for an interrupted large-file upload', async () => {
    // Given
    const user = userEvent.setup()
    const onResume = vi.fn()
    render(
      <TransferPanel
        visible
        items={[{
          id: 'transfer-2',
          name: 'archive.tar',
          direction: 'upload',
          state: 'paused',
          bytesTransferred: 64,
          totalBytes: 256,
          localKind: 'file',
          recoveryKind: 'chunk_resume',
        }]}
        onClose={vi.fn()}
        onCancel={vi.fn()}
        onPause={vi.fn()}
        onResume={onResume}
        onDiscard={vi.fn()}
        onOpenFile={vi.fn()}
        onOpenFolder={vi.fn()}
        onClearFinished={vi.fn()}
      />,
    )

    // When
    await user.click(screen.getByRole('button', { name: '继续上传 archive.tar' }))

    // Then
    expect(onResume).toHaveBeenCalledWith('transfer-2')
  })

  it('keeps retry available when an upload reports a real failure', async () => {
    const user = userEvent.setup()
    const onResume = vi.fn()
    render(
      <TransferPanel
        visible
        items={[{
          id: 'transfer-3',
          name: 'failed.zip',
          direction: 'upload',
          state: 'failed',
          bytesTransferred: 32,
          totalBytes: 128,
          localKind: 'file',
          recoveryKind: 'restart',
        }]}
        onClose={vi.fn()}
        onCancel={vi.fn()}
        onPause={vi.fn()}
        onResume={onResume}
        onDiscard={vi.fn()}
        onOpenFile={vi.fn()}
        onOpenFolder={vi.fn()}
        onClearFinished={vi.fn()}
      />,
    )

    expect(screen.getByText('失败 · 32 B')).toBeTruthy()
    await user.click(screen.getByRole('button', { name: '重新上传 failed.zip' }))
    expect(onResume).toHaveBeenCalledWith('transfer-3')
  })

  it('continues a failed split upload from its committed chunks', async () => {
    const user = userEvent.setup()
    const onResume = vi.fn()
    render(
      <TransferPanel
        visible
        items={[{
          id: 'transfer-4',
          name: 'split-package',
          direction: 'upload',
          state: 'failed',
          bytesTransferred: 64,
          totalBytes: 256,
          localKind: 'file',
          recoveryKind: 'chunk_resume',
        }]}
        onClose={vi.fn()}
        onCancel={vi.fn()}
        onPause={vi.fn()}
        onResume={onResume}
        onDiscard={vi.fn()}
        onOpenFile={vi.fn()}
        onOpenFolder={vi.fn()}
        onClearFinished={vi.fn()}
      />,
    )

    await user.click(screen.getByRole('button', { name: '继续上传 split-package' }))
    expect(onResume).toHaveBeenCalledWith('transfer-4')
  })
})
