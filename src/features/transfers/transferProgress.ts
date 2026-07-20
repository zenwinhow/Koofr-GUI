import type { TransferProgress } from '../../types/backend'
import type { TransferItem } from '../../types/files'

export function mergeTransferProgress(
  items: readonly TransferItem[],
  progress: TransferProgress,
): TransferItem[] {
  return items.map((transfer) => transfer.id === progress.transferId
    ? {
        ...transfer,
        state: progress.state,
        bytesTransferred: progress.bytesTransferred,
        totalBytes: progress.totalBytes ?? transfer.totalBytes,
      }
    : transfer)
}
