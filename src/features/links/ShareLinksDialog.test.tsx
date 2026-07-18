import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { KoofrMount, PublicLink } from '../../types/backend'
import { publicLinks } from '../../services/publicLinks'
import { ShareLinksDialog } from './ShareLinksDialog'

vi.mock('../../services/publicLinks', () => ({
  publicLinks: {
    list: vi.fn(),
    create: vi.fn(),
    remove: vi.fn(),
  },
}))

const MOUNTS: KoofrMount[] = [{
  id: 'mount_1',
  name: 'Koofr',
  mountType: 'koofr',
  spaceTotal: 1_000,
  spaceUsed: 100,
  online: true,
  isPrimary: true,
  isShared: false,
}]

const DOWNLOAD_LINK: PublicLink = {
  id: 'download_1',
  name: 'report.pdf',
  path: '/report.pdf',
  counter: 3,
  url: 'https://app.koofr.net/links/download_1',
  shortUrl: 'https://k00.fr/report',
  hasPassword: false,
  kind: 'download',
}

describe('ShareLinksDialog', () => {
  beforeEach(() => {
    vi.mocked(publicLinks.list).mockResolvedValue([DOWNLOAD_LINK])
    vi.mocked(publicLinks.remove).mockResolvedValue()
  })

  it('loads both kinds of links for the selected storage', async () => {
    render(<ShareLinksDialog mounts={MOUNTS} onClose={vi.fn()} />)

    expect((await screen.findByRole('textbox', { name: 'report.pdf 的链接地址' })).getAttribute('value'))
      .toBe(DOWNLOAD_LINK.shortUrl)
    expect(publicLinks.list).toHaveBeenCalledWith('mount_1')
  })

  it('requires a second click before revoking a link', async () => {
    const user = userEvent.setup()
    render(<ShareLinksDialog mounts={MOUNTS} onClose={vi.fn()} />)

    await screen.findByText('report.pdf')
    await user.click(screen.getByRole('button', { name: '撤销 report.pdf' }))
    expect(publicLinks.remove).not.toHaveBeenCalled()
    await user.click(screen.getByRole('button', { name: '确认撤销 report.pdf' }))

    await waitFor(() => expect(publicLinks.remove).toHaveBeenCalledWith('mount_1', 'download_1', 'download'))
    expect(screen.queryByText('report.pdf')).toBeNull()
  })
})
