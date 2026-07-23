import type { RecoveryKind, TransferDirection, TransferState } from './backend'

export type FileKind =
  | 'folder'
  | 'xlsx'
  | 'pdf'
  | 'docx'
  | 'pptx'
  | 'image'
  | 'video'
  | 'audio'
  | 'archive'
  | 'executable'
  | 'code'
  | 'text'
  | 'font'
  | 'ebook'
  | 'disk'
  | 'database'
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
  readonly remotePath: string | null
  readonly localPath: string | null
  readonly startedAt: number | null
  readonly finishedAt: number | null
  readonly speedSamples: readonly TransferSpeedSample[]
}

export interface TransferSpeedSample {
  readonly recordedAt: number
  readonly bytesTransferred: number
}
