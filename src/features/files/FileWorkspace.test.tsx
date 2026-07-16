import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { FileWorkspace } from './FileWorkspace'

describe('FileWorkspace', () => {
  it('closes the new-item popup when the page background is clicked', async () => {
    // Given
    const user = userEvent.setup()
    render(
      <FileWorkspace
        mounts={[]}
        activeMountId="mount-1"
        path="/"
        files={[]}
        loading={false}
        error=""
        lastSyncedAt={null}
        onMountChange={vi.fn()}
        onNavigate={vi.fn()}
        onRefresh={vi.fn()}
        onCreateFolder={vi.fn()}
        onThemeOpen={vi.fn()}
        onUpload={vi.fn()}
        onDownload={vi.fn()}
        onRename={vi.fn()}
        onDelete={vi.fn()}
      />,
    )
    await user.click(screen.getByRole('button', { name: /新建/ }))
    expect(screen.getByText('新建文件夹')).not.toBeNull()

    // When
    await user.click(document.body)

    // Then
    expect(screen.queryByText('新建文件夹')).toBeNull()
  })
})
