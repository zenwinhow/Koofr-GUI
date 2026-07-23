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
    | 'local_open_error'
    | 'local_data_error'
    | 'credential_store_error'
    | 'vault_locked'
    | 'vault_invalid_key'
    | 'vault_crypto_error'
    | 'vault_prompt_error'
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
export type LogLevel = 'error' | 'warn' | 'info' | 'debug'

export interface AppSettings {
  readonly cacheMode: CacheMode
  readonly cacheTtlMinutes: number
  readonly cachedItems: number
  readonly cacheDiskBytes: number
  readonly savedEmail: string | null
  readonly downloadDirectory: string
  readonly askDownloadLocation: boolean
  readonly cacheDirectory: string
  readonly logDirectory: string
  readonly logLevel: LogLevel
  readonly logRetentionDays: number
  readonly logMaxFileSizeMb: number
  readonly logFiles: number
  readonly logDiskBytes: number
  readonly autoRetryNetworkErrors: boolean
  readonly networkRetryLimit: number | null
  readonly networkRetryIntervalSeconds: number
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
export type TransferState = 'running' | 'retrying' | 'paused' | 'completed' | 'cancelled' | 'failed'
export type RecoveryKind = 'byte_resume' | 'chunk_resume' | 'restart'

export interface TransferProgress {
  transferId: string
  direction: TransferDirection
  state: TransferState
  bytesTransferred: number
  totalBytes: number | null
}

export interface DownloadSpeedSample {
  readonly recordedAt: number
  readonly bytesTransferred: number
}

export interface DownloadHistoryItem {
  readonly transferId: string
  readonly name: string
  readonly state: TransferState
  readonly bytesTransferred: number
  readonly totalBytes: number | null
  readonly localKind: 'file' | 'folder'
  readonly recoveryKind: RecoveryKind | null
  readonly remotePath: string
  readonly localPath: string
  readonly startedAt: number
  readonly finishedAt: number | null
  readonly speedSamples: DownloadSpeedSample[]
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
  readonly localPath: string | null
}

export interface VaultSummary {
  readonly id: string
  readonly name: string
  readonly locked: boolean
  readonly added: number
  readonly autoLockSeconds: number
}

export interface VaultBreadcrumb {
  readonly id: string
  readonly name: string
}

export interface VaultEntry {
  readonly id: string
  readonly name: string
  readonly entryType: 'file' | 'dir'
  readonly modified: number
  readonly size: number
  readonly contentType: string
}

export interface VaultDirectory {
  readonly repoId: string
  readonly repoName: string
  readonly directoryId: string
  readonly breadcrumbs: VaultBreadcrumb[]
  readonly entries: VaultEntry[]
}
