import { render, screen, waitFor } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'

import { VaultDialog } from './VaultDialog'

const mocks = vi.hoisted(() => ({
  listVaults: vi.fn(),
  unlockVault: vi.fn(),
  listVaultFiles: vi.fn(),
}))

vi.mock('../../services/koofr', () => ({
  commandErrorMessage: (_error: unknown, fallback: string) => fallback,
  isCommandErrorCode: () => false,
  koofr: {
    listVaults: mocks.listVaults,
    unlockVault: mocks.unlockVault,
    listVaultFiles: mocks.listVaultFiles,
  },
}))

const lockedVault = {
  id: 'repo-opaque-id',
  name: '工作资料',
  locked: true,
  added: 0,
  autoLockSeconds: 3600,
}

describe('VaultDialog', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    mocks.listVaults.mockResolvedValue([lockedVault])
    mocks.unlockVault.mockResolvedValue({ ...lockedVault, locked: false })
    mocks.listVaultFiles.mockResolvedValue({
      repoId: lockedVault.id,
      repoName: lockedVault.name,
      directoryId: 'root',
      breadcrumbs: [{ id: 'root', name: lockedVault.name }],
      entries: [],
    })
  })

  it('never renders a Safe Key input in the WebView', async () => {
    render(
      <VaultDialog
        mountId="mount-1"
        parentPath="/"
        onClose={vi.fn()}
        onNotice={vi.fn()}
        onTransferStarted={vi.fn()}
      />,
    )

    expect(await screen.findByRole('button', { name: /解锁/ })).not.toBeNull()
    expect(document.querySelector('input[type="password"]')).toBeNull()
    expect(screen.queryByRole('textbox', { name: /Safe Key/i })).toBeNull()
    expect(screen.getByText(/Windows 原生安全窗口/)).not.toBeNull()
  })

  it('requests native unlock and opens the opaque root handle', async () => {
    const user = userEvent.setup()
    render(
      <VaultDialog
        mountId="mount-1"
        parentPath="/"
        onClose={vi.fn()}
        onNotice={vi.fn()}
        onTransferStarted={vi.fn()}
      />,
    )

    await user.click(await screen.findByRole('button', { name: /解锁/ }))

    await waitFor(() => {
      expect(mocks.unlockVault).toHaveBeenCalledWith(lockedVault.id)
      expect(mocks.listVaultFiles).toHaveBeenCalledWith(lockedVault.id, 'root')
    })
    expect(screen.getByText('此加密文件夹为空')).not.toBeNull()
  })
})
