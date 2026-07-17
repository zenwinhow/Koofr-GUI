import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import type { RemoteFile } from '../../types/backend'
import { FileWorkspace } from './FileWorkspace'

const defaultCallbacks = {
  onMountChange: vi.fn(),
  onNavigate: vi.fn(),
  onRefresh: vi.fn(),
  onCreateFolder: vi.fn(),
  onThemeOpen: vi.fn(),
  onUpload: vi.fn(),
  onDownload: vi.fn(),
  onRename: vi.fn(),
  onDelete: vi.fn(),
}

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
        {...defaultCallbacks}
      />,
    )
    await user.click(screen.getByRole('button', { name: /新建/ }))
    expect(screen.getByText('新建文件夹')).not.toBeNull()

    // When
    await user.click(document.body)

    // Then
    expect(screen.queryByText('新建文件夹')).toBeNull()
  })

  it('renders a distinct visual glyph for every supported file kind', () => {
    // Given
    const files = [
      { name: 'Projects', entryType: 'dir', contentType: '', path: '/Projects' },
      { name: 'budget.xlsx', entryType: 'file', contentType: 'application/vnd.ms-excel', path: '/budget.xlsx' },
      { name: 'brief.pdf', entryType: 'file', contentType: 'application/pdf', path: '/brief.pdf' },
      { name: 'notes.docx', entryType: 'file', contentType: 'application/msword', path: '/notes.docx' },
      { name: 'cover.png', entryType: 'file', contentType: 'image/png', path: '/cover.png' },
      { name: 'source.zip', entryType: 'file', contentType: 'application/zip', path: '/source.zip' },
      { name: 'setup.exe', entryType: 'file', contentType: 'application/octet-stream', path: '/setup.exe' },
      { name: 'archive.bin', entryType: 'file', contentType: 'application/octet-stream', path: '/archive.bin' },
    ].map((file) => ({ ...file, modified: 0, size: 0, hash: '' })) satisfies RemoteFile[]

    // When
    const { container } = render(
      <FileWorkspace
        mounts={[]}
        activeMountId="mount-1"
        path="/"
        files={files}
        loading={false}
        error=""
        lastSyncedAt={null}
        {...defaultCallbacks}
      />,
    )

    // Then
    expect(new Set([...container.querySelectorAll('[data-file-kind]')].map((node) => node.getAttribute('data-file-kind'))))
      .toEqual(new Set(['folder', 'file', 'xlsx', 'image', 'pdf', 'docx', 'archive', 'executable']))
  })
})
