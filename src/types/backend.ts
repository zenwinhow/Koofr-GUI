export interface CommandError {
  code:
    | 'authentication_failed'
    | 'not_authenticated'
    | 'invalid_input'
    | 'conflict'
    | 'not_found'
    | 'forbidden'
    | 'cancelled'
    | 'incomplete_transfer'
    | 'remote_error'
    | 'network_error'
    | 'local_io_error'
    | 'invalid_response'
    | 'initialization_error'
    | 'dialog_error'
  message: string
}

export interface KoofrSession {
  authenticated: boolean
  userId: string | null
}

export interface KoofrMount {
  id: string
  name: string
  mountType: string
  spaceTotal: number
  spaceUsed: number
  online: boolean
  isPrimary: boolean
  isShared: boolean
}

export interface RemoteFile {
  name: string
  entryType: string
  modified: number
  size: number
  contentType: string
  hash: string
  path: string
}

export type TransferDirection = 'upload' | 'download'
export type TransferState = 'running' | 'completed' | 'cancelled' | 'failed'

export interface TransferProgress {
  transferId: string
  direction: TransferDirection
  state: TransferState
  bytesTransferred: number
  totalBytes: number | null
}

export interface TransferResult {
  transferId: string
  bytesTransferred: number
  file: RemoteFile | null
}

export interface LocalFileSelection {
  grantId: string
  fileName: string
}
