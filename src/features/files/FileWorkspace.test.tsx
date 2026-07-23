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
  onShare: vi.fn(),
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
      { name: 'deck.pptx', entryType: 'file', contentType: 'application/vnd.ms-powerpoint', path: '/deck.pptx' },
      { name: 'cover.png', entryType: 'file', contentType: 'image/png', path: '/cover.png' },
      { name: 'clip.mp4', entryType: 'file', contentType: 'video/mp4', path: '/clip.mp4' },
      { name: 'track.mp3', entryType: 'file', contentType: 'audio/mpeg', path: '/track.mp3' },
      { name: 'source.zip', entryType: 'file', contentType: 'application/zip', path: '/source.zip' },
      { name: 'setup.exe', entryType: 'file', contentType: 'application/octet-stream', path: '/setup.exe' },
      { name: 'app.tsx', entryType: 'file', contentType: 'application/octet-stream', path: '/app.tsx' },
      { name: 'readme.md', entryType: 'file', contentType: 'text/markdown', path: '/readme.md' },
      { name: 'Inter.ttf', entryType: 'file', contentType: 'font/ttf', path: '/Inter.ttf' },
      { name: 'novel.epub', entryType: 'file', contentType: 'application/epub+zip', path: '/novel.epub' },
      { name: 'ubuntu.iso', entryType: 'file', contentType: 'application/octet-stream', path: '/ubuntu.iso' },
      { name: 'store.sqlite', entryType: 'file', contentType: 'application/octet-stream', path: '/store.sqlite' },
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
      .toEqual(new Set([
        'folder', 'file', 'xlsx', 'image', 'pdf', 'docx', 'pptx', 'video', 'audio',
        'archive', 'executable', 'code', 'text', 'font', 'ebook', 'disk', 'database',
      ]))
  })

  it('shares the selected file instead of asking for a path', async () => {
    const user = userEvent.setup()
    const file: RemoteFile = {
      name: 'report.pdf',
      entryType: 'file',
      modified: 0,
      size: 10,
      contentType: 'application/pdf',
      hash: '',
      path: '/Documents/report.pdf',
    }
    const onShare = vi.fn()
    render(
      <FileWorkspace
        mounts={[]}
        activeMountId="mount-1"
        path="/Documents"
        files={[file]}
        loading={false}
        error=""
        lastSyncedAt={null}
        {...defaultCallbacks}
        onShare={onShare}
      />,
    )

    await user.click(screen.getByRole('button', { name: '选择 report.pdf' }))
    await user.click(screen.getByRole('button', { name: '分享' }))

    expect(onShare).toHaveBeenCalledWith(file)
  })

  it('lets the user choose a compatible or resumable upload mode', async () => {
    // Given
    const user = userEvent.setup()
    const onUpload = vi.fn()
    render(
      <FileWorkspace
        mounts={[]}
        activeMountId="mount-1"
        path="/Videos"
        files={[]}
        loading={false}
        error=""
        lastSyncedAt={null}
        {...defaultCallbacks}
        onUpload={onUpload}
      />,
    )

    // When
    await user.click(screen.getByRole('button', { name: '选择上传方式' }))
    await user.click(screen.getByRole('menuitem', { name: /可续传大文件/ }))

    // Then
    expect(onUpload).toHaveBeenCalledWith('split')
  })
})
