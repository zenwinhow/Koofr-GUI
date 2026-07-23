import type {
  RecoveryKind,
  TransferDirection,
  TransferResult,
} from './backend'

export interface VaultTransferSpec {
  readonly transferId: string
  readonly name: string
  readonly direction: TransferDirection
  readonly totalBytes: number | null
  readonly recoveryKind: RecoveryKind | null
  readonly localPath: string | null
  readonly result: Promise<TransferResult>
}
