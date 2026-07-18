import type { RecoveryKind, TransferDirection, TransferState } from './backend'

export type FileKind =
  | 'folder'
  | 'xlsx'
  | 'pdf'
  | 'docx'
  | 'image'
  | 'archive'
  | 'executable'
  | 'file'

export type UploadMode = 'compatible' | 'split'

export interface SplitUploadSettings {
  readonly packageName: string
  readonly partBytes: number
}

export interface TransferItem {
  readonly id: string
  readonly name: string
  readonly direction: TransferDirection
  readonly state: TransferState
  readonly bytesTransferred: number
  readonly totalBytes: number | null
  readonly localKind: 'file' | 'folder'
  readonly recoveryKind: RecoveryKind | null
}
