import type { TransferDirection, TransferState } from './backend'

export type FileKind =
  | 'folder'
  | 'xlsx'
  | 'pdf'
  | 'docx'
  | 'image'
  | 'archive'
  | 'executable'
  | 'file'

export interface TransferItem {
  id: string
  name: string
  direction: TransferDirection
  state: TransferState
  bytesTransferred: number
  totalBytes: number | null
  localKind: 'file' | 'folder'
}
