import { describe, expect, it, vi } from 'vitest'

import type { TransferProgress } from '../../types/backend'
import type { TransferItem } from '../../types/files'
import { countActiveDownloads, mergeTransferProgress } from './transferProgress'

describe('mergeTransferProgress', () => {
  it('preserves the known total size when a paused event omits it', () => {
    // Given
    const transfer: TransferItem = {
      id: 'transfer-1',
      name: 'large.iso',
      direction: 'download',
      state: 'running',
      bytesTransferred: 16,
      totalBytes: 128,
      localKind: 'file',
      recoveryKind: 'byte_resume',
      remotePath: '/large.iso',
      localPath: 'C:\\Downloads\\large.iso',
      startedAt: 1_700_000_000_000,
      finishedAt: null,
      speedSamples: [{ recordedAt: 1_700_000_000_000, bytesTransferred: 0 }],
    }
    const paused: TransferProgress = {
      transferId: 'transfer-1',
      direction: 'download',
      state: 'paused',
      bytesTransferred: 64,
      totalBytes: null,
    }

    // When
    const [updated] = mergeTransferProgress([transfer], paused)

    // Then
    expect(updated?.state).toBe('paused')
    expect(updated?.bytesTransferred).toBe(64)
    expect(updated?.totalBytes).toBe(128)
    expect(updated?.speedSamples.at(-1)?.bytesTransferred).toBe(64)
  })

  it('counts only running and retrying downloads for the badge', () => {
    const base: TransferItem = {
      id: 'transfer-1',
      name: 'large.iso',
      direction: 'download',
      state: 'running',
      bytesTransferred: 16,
      totalBytes: 128,
      localKind: 'file',
      recoveryKind: 'byte_resume',
      remotePath: '/large.iso',
      localPath: 'C:\\Downloads\\large.iso',
      startedAt: 1_700_000_000_000,
      finishedAt: null,
      speedSamples: [],
    }

    expect(countActiveDownloads([
      base,
      { ...base, id: 'completed', state: 'completed' },
      { ...base, id: 'paused', state: 'paused' },
      { ...base, id: 'upload', direction: 'upload', state: 'running' },
      { ...base, id: 'retrying', state: 'retrying' },
    ])).toBe(2)
  })

  it('records bounded speed samples for uploads as well as downloads', () => {
    vi.spyOn(Date, 'now').mockReturnValue(1_700_000_002_000)
    const upload: TransferItem = {
      id: 'upload-1',
      name: 'archive.zip',
      direction: 'upload',
      state: 'running',
      bytesTransferred: 0,
      totalBytes: 4096,
      localKind: 'file',
      recoveryKind: 'restart',
      remotePath: null,
      localPath: null,
      startedAt: 1_700_000_000_000,
      finishedAt: null,
      speedSamples: [{ recordedAt: 1_700_000_000_000, bytesTransferred: 0 }],
    }

    const [updated] = mergeTransferProgress([upload], {
      transferId: upload.id,
      direction: 'upload',
      state: 'running',
      bytesTransferred: 2048,
      totalBytes: 4096,
    })

    expect(updated?.speedSamples).toEqual([
      { recordedAt: 1_700_000_000_000, bytesTransferred: 0 },
      { recordedAt: 1_700_000_002_000, bytesTransferred: 2048 },
    ])
    vi.restoreAllMocks()
  })
})
