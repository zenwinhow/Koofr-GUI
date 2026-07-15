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
    | 'local_data_error'
    | 'credential_store_error'
  message: string
  diagnostic?: string
}

export interface KoofrSession {
  authenticated: boolean
  userId: string | null
}

export interface LoginBootstrap {
  session: KoofrSession
  savedEmail: string | null
}

export type CacheMode = 'off' | 'memory' | 'disk'

export interface AppSettings {
  cacheMode: CacheMode
  cacheTtlMinutes: number
  cachedItems: number
  cacheDiskBytes: number
  savedEmail: string | null
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

export interface LocatedFile extends RemoteFile {
  mountId: string
  mountName: string
  shareDirection: 'outgoing' | 'received' | null
}

export interface TrashItem {
  versionId: string
  mountId: string
  mountName: string
  path: string
  name: string
  deleted: string
  size: number
  contentType: string
}

export interface TrashList {
  items: TrashItem[]
  retentionDays: number
}

export interface TrashRestoreTarget {
  mountId: string
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
