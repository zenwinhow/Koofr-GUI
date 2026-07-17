import { render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { publicLinks } from '../../services/publicLinks'
import type { PublicLink, RemoteFile } from '../../types/backend'
import { CreateShareLinkDialog } from './CreateShareLinkDialog'

vi.mock('../../services/publicLinks', () => ({
  publicLinks: { create: vi.fn() },
}))

const FILE: RemoteFile = {
  name: 'report.pdf',
  path: '/Documents/report.pdf',
  entryType: 'file',
  modified: 0,
  size: 10,
  contentType: 'application/pdf',
  hash: '',
}

const CREATED: PublicLink = {
  id: 'link_1',
  name: FILE.name,
  path: FILE.path,
  counter: 0,
  url: 'https://app.koofr.net/links/link_1',
  shortUrl: 'https://k00.fr/report',
  hasPassword: false,
  kind: 'download',
}

describe('CreateShareLinkDialog', () => {
  beforeEach(() => {
    vi.mocked(publicLinks.create).mockResolvedValue(CREATED)
  })

  it('creates a download link from the selected file path', async () => {
    const user = userEvent.setup()
    render(<CreateShareLinkDialog mountId="mount_1" file={FILE} onClose={vi.fn()} />)

    await user.click(screen.getByRole('button', { name: '创建链接' }))

    expect(publicLinks.create).toHaveBeenCalledWith('mount_1', FILE.path, 'download')
    expect((await screen.findByRole('textbox', { name: '分享链接地址' })).getAttribute('value'))
      .toBe(CREATED.shortUrl)
  })

  it('offers a receive-files link only for a selected folder', async () => {
    const user = userEvent.setup()
    const folder = { ...FILE, name: 'Incoming', path: '/Incoming', entryType: 'dir' }
    vi.mocked(publicLinks.create).mockResolvedValue({ ...CREATED, kind: 'upload', path: folder.path })
    render(<CreateShareLinkDialog mountId="mount_1" file={folder} onClose={vi.fn()} />)

    await user.selectOptions(screen.getByRole('combobox', { name: '分享方式' }), 'upload')
    await user.click(screen.getByRole('button', { name: '创建链接' }))

    expect(publicLinks.create).toHaveBeenCalledWith('mount_1', folder.path, 'upload')
  })
})
