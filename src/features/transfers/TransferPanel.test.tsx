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
        onResume={onResume}
        onDiscard={vi.fn()}
        onOpenFile={vi.fn()}
        onOpenFolder={vi.fn()}
        onClearFinished={vi.fn()}
      />,
    )

    await user.click(screen.getByRole('button', { name: '继续下载 large.iso' }))

    expect(onResume).toHaveBeenCalledWith('transfer-1')
    expect(screen.getByText('已暂停 · 64 B')).toBeTruthy()
  })
})
