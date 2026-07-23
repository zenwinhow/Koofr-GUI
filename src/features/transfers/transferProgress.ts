import type { TransferProgress } from '../../types/backend'
import type { TransferItem } from '../../types/files'

const MAX_SPEED_SAMPLES = 300
const MIN_SPEED_SAMPLE_INTERVAL_MS = 1000

export function countActiveDownloads(items: readonly TransferItem[]) {
  return items.filter((transfer) => (
    transfer.direction === 'download'
    && (transfer.state === 'running' || transfer.state === 'retrying')
  )).length
}

export function mergeTransferProgress(
  items: readonly TransferItem[],
  progress: TransferProgress,
): TransferItem[] {
  const recordedAt = Date.now()
  return items.map((transfer) => transfer.id === progress.transferId
    ? (() => {
        const previousSample = transfer.speedSamples.at(-1)
        const terminal = progress.state === 'completed'
          || progress.state === 'cancelled'
          || progress.state === 'failed'
        const shouldSample = progress.bytesTransferred !== transfer.bytesTransferred
          && (
            previousSample === undefined
            || recordedAt - previousSample.recordedAt >= MIN_SPEED_SAMPLE_INTERVAL_MS
            || terminal
          )
        const speedSamples = shouldSample
          ? [
              ...transfer.speedSamples,
              { recordedAt, bytesTransferred: progress.bytesTransferred },
            ].slice(-MAX_SPEED_SAMPLES)
          : transfer.speedSamples
        return {
          ...transfer,
          state: progress.state,
          bytesTransferred: progress.bytesTransferred,
          totalBytes: progress.totalBytes ?? transfer.totalBytes,
          finishedAt: terminal ? transfer.finishedAt ?? recordedAt : transfer.finishedAt,
          speedSamples,
        }
      })()
    : transfer)
}
