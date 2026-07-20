import { describe, expect, it } from 'vitest'

import type { TransferProgress } from '../../types/backend'
import type { TransferItem } from '../../types/files'
import { mergeTransferProgress } from './transferProgress'

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
  })
})
