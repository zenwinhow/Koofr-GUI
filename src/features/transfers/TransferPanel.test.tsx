import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'

import { TransferPanel } from './TransferPanel'
import type { TransferItem } from '../../types/files'

function transferItem(overrides: Partial<TransferItem>): TransferItem {
  return {
    id: 'transfer-default',
    name: 'file.bin',
    direction: 'download',
    state: 'running',
    bytesTransferred: 0,
    totalBytes: null,
    localKind: 'file',
    recoveryKind: null,
    remotePath: '/file.bin',
    localPath: 'C:\\Downloads\\file.bin',
    startedAt: 1_700_000_000_000,
    finishedAt: null,
    speedSamples: [],
    ...overrides,
  }
}

describe('TransferPanel', () => {
  it('shows cached download details and speed information for the selected row', async () => {
    const user = userEvent.setup()
    render(
      <TransferPanel
        visible
        items={[
          transferItem({
            id: 'first',
            name: 'first.zip',
            state: 'completed',
            bytesTransferred: 1024,
            totalBytes: 1024,
            finishedAt: 1_700_000_002_000,
            speedSamples: [
              { recordedAt: 1_700_000_000_000, bytesTransferred: 0 },
              { recordedAt: 1_700_000_002_000, bytesTransferred: 1024 },
            ],
          }),
          transferItem({
            id: 'second',
            name: 'second.pdf',
            state: 'completed',
            remotePath: '/reports/second.pdf',
            localPath: 'C:\\Downloads\\second.pdf',
            startedAt: 1_700_000_003_000,
            finishedAt: 1_700_000_005_000,
            bytesTransferred: 2048,
            totalBytes: 2048,
          }),
        ]}
        onClose={vi.fn()}
        onCancel={vi.fn()}
        onPause={vi.fn()}
        onResume={vi.fn()}
        onDiscard={vi.fn()}
        onOpenFile={vi.fn()}
        onOpenFolder={vi.fn()}
        onClearFinished={vi.fn()}
      />,
    )

    await user.click(screen.getByRole('option', { name: /first\.zip/ }))

    expect(screen.getByText('Koofr · /file.bin')).toBeTruthy()
    expect(screen.getByText('C:\\Downloads\\file.bin')).toBeTruthy()
    expect(screen.getByText('平均速度')).toBeTruthy()
    expect(screen.getByRole('heading', { name: '传输' })).toBeTruthy()
    expect(screen.getByRole('img', { name: '最近 1 分钟下载速度' })).toBeTruthy()
    const chart = screen.getByRole('button', { name: /当前为折线样式/ })
    const linePoints = chart.querySelector('polyline')?.getAttribute('points') ?? ''
    expect(linePoints.startsWith('0.0,')).toBe(false)
    await user.click(chart)
    expect(screen.getByRole('button', { name: /当前为平滑样式/ })).toBeTruthy()
    expect(chart.querySelector('.transfer-speed-chart__curve')).toBeTruthy()
  })

  it('offers byte resume for an interrupted download', async () => {
    const user = userEvent.setup()
    const onResume = vi.fn()
    render(
      <TransferPanel
        visible
        items={[transferItem({
          id: 'transfer-1',
          name: 'large.iso',
          direction: 'download',
          state: 'paused',
          bytesTransferred: 64,
          totalBytes: 128,
          localKind: 'file',
          recoveryKind: 'byte_resume',
        })]}
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
    expect(screen.getAllByText('已暂停').length).toBeGreaterThan(0)
  })


  it('offers chunk resume for an interrupted large-file upload', async () => {
    // Given
    const user = userEvent.setup()
    const onResume = vi.fn()
    render(
      <TransferPanel
        visible
        items={[transferItem({
          id: 'transfer-2',
          name: 'archive.tar',
          direction: 'upload',
          state: 'paused',
          bytesTransferred: 64,
          totalBytes: 256,
          localKind: 'file',
          recoveryKind: 'chunk_resume',
        })]}
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
        items={[transferItem({
          id: 'transfer-3',
          name: 'failed.zip',
          direction: 'upload',
          state: 'failed',
          bytesTransferred: 32,
          totalBytes: 128,
          localKind: 'file',
          recoveryKind: 'restart',
        })]}
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

    expect(screen.getAllByText('失败').length).toBeGreaterThan(0)
    await user.click(screen.getByRole('button', { name: '重新上传 failed.zip' }))
    expect(onResume).toHaveBeenCalledWith('transfer-3')
  })

  it('continues a failed split upload from its committed chunks', async () => {
    const user = userEvent.setup()
    const onResume = vi.fn()
    render(
      <TransferPanel
        visible
        items={[transferItem({
          id: 'transfer-4',
          name: 'split-package',
          direction: 'upload',
          state: 'failed',
          bytesTransferred: 64,
          totalBytes: 256,
          localKind: 'file',
          recoveryKind: 'chunk_resume',
        })]}
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

  it('keeps pause and cancel available while waiting to retry', () => {
    render(
      <TransferPanel
        visible
        items={[transferItem({
          id: 'transfer-5',
          name: 'retrying.bin',
          direction: 'download',
          state: 'retrying',
          bytesTransferred: 64,
          totalBytes: 256,
          localKind: 'file',
          recoveryKind: 'byte_resume',
        })]}
        onClose={vi.fn()}
        onCancel={vi.fn()}
        onPause={vi.fn()}
        onResume={vi.fn()}
        onDiscard={vi.fn()}
        onOpenFile={vi.fn()}
        onOpenFolder={vi.fn()}
        onClearFinished={vi.fn()}
      />,
    )

    expect(screen.getAllByText('等待网络重试').length).toBeGreaterThan(0)
    expect(screen.getByRole('button', { name: '暂停 retrying.bin' })).toBeTruthy()
    expect(screen.getByRole('button', { name: '取消 retrying.bin' })).toBeTruthy()
  })
})
