export interface CommandError {
  code:
    | 'authentication_failed'
    | 'not_authenticated'
    | 'account_identity_unavailable'
    | 'invalid_input'
    | 'conflict'
    | 'not_found'
    | 'forbidden'
    | 'cancelled'
    | 'transfer_paused'
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
  readonly cacheMode: CacheMode
  readonly cacheTtlMinutes: number
  readonly cachedItems: number
  readonly cacheDiskBytes: number
  readonly savedEmail: string | null
  readonly downloadDirectory: string
  readonly askDownloadLocation: boolean
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

export type PublicLinkKind = 'download' | 'upload'

export interface PublicLink {
  readonly id: string
  readonly name: string
  readonly path: string
  readonly counter: number
  readonly url: string
  readonly shortUrl: string
  readonly hasPassword: boolean
  readonly kind: PublicLinkKind
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
export type TransferState = 'running' | 'paused' | 'completed' | 'cancelled' | 'failed'
export type RecoveryKind = 'byte_resume' | 'chunk_resume' | 'restart'

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

export interface ResumableTransfer {
  readonly transferId: string
  readonly name: string
  readonly direction: TransferDirection
  readonly recoveryKind: RecoveryKind
  readonly bytesTransferred: number
  readonly totalBytes: number
}

export interface LocalFileSelection {
  readonly grantId: string
  readonly fileName: string
}
