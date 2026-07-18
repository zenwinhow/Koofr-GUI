import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  AppSettings,
  CacheMode,
  KoofrMount,
  KoofrSession,
  LocatedFile,
  LocalFileSelection,
  LoginBootstrap,
  RemoteFile,
  ResumableTransfer,
  TransferProgress,
  TransferResult,
  TrashItem,
  TrashList,
  CommandError,
} from '../types/backend'
import type { SplitUploadSettings } from '../types/files'

const TRANSFER_EVENT = 'koofr://transfer-progress'

export function isTauriRuntime() {
  return '__TAURI_INTERNALS__' in window
}

export function commandErrorMessage(error: unknown, fallback: string) {
  if (
    typeof error === 'object'
    && error !== null
    && typeof (error as Partial<CommandError>).code === 'string'
    && typeof (error as Partial<CommandError>).message === 'string'
  ) {
    return (error as CommandError).message
  }
  return fallback
}

export function commandErrorDiagnostic(error: unknown) {
  if (
    typeof error === 'object'
    && error !== null
    && typeof (error as Partial<CommandError>).diagnostic === 'string'
  ) {
    return (error as CommandError).diagnostic ?? ''
  }
  return ''
}

export function isCommandErrorCode(error: unknown, code: CommandError['code']) {
  return typeof error === 'object'
    && error !== null
    && (error as Partial<CommandError>).code === code
}

export const koofr = {
  connect(email: string, appPassword: string, rememberPassword: boolean) {
    return invoke<KoofrSession>('connect_koofr', { email, appPassword, rememberPassword })
  },

  restoreSavedLogin() {
    return invoke<LoginBootstrap>('restore_saved_login')
  },

  disconnect() {
    return invoke<void>('disconnect_koofr')
  },

  session() {
    return invoke<KoofrSession>('koofr_session')
  },

  getSettings() {
    return invoke<AppSettings>('get_settings')
  },

  updateSettings(cacheMode: CacheMode, cacheTtlMinutes: number) {
    return invoke<AppSettings>('update_settings', { cacheMode, cacheTtlMinutes })
  },

  updateDownloadSettings(downloadDirectory: string, askDownloadLocation: boolean) {
    return invoke<AppSettings>('update_download_settings', {
      downloadDirectory,
      askDownloadLocation,
    })
  },

  clearMetadataCache() {
    return invoke<AppSettings>('clear_metadata_cache')
  },

  forgetSavedLogin() {
    return invoke<AppSettings>('forget_saved_login')
  },

  listMounts(refresh = false) {
    return invoke<KoofrMount[]>('list_mounts', { refresh })
  },

  listFiles(mountId: string, path = '/', refresh = false) {
    return invoke<RemoteFile[]>('list_files', { mountId, path, refresh })
  },

  listRecent(refresh = false) {
    return invoke<LocatedFile[]>('list_recent', { refresh })
  },

  listShared(refresh = false) {
    return invoke<LocatedFile[]>('list_shared', { refresh })
  },

  listTrash(refresh = false) {
    return invoke<TrashList>('list_trash', { refresh })
  },

  restoreTrash(files: TrashItem[]) {
    return invoke<void>('restore_trash', {
      files: files.map(({ mountId, path }) => ({ mountId, path })),
    })
  },

  restoreAllTrash() {
    return invoke<void>('restore_trash', { files: [] })
  },

  emptyTrash(confirmation: string) {
    return invoke<void>('empty_trash', { confirmation })
  },

  selectUploadFile() {
    return invoke<LocalFileSelection | null>('select_upload_file')
  },

  selectDownloadLocation(suggestedName: string) {
    return invoke<LocalFileSelection | null>('select_download_location', { suggestedName })
  },

  selectDownloadFolder(suggestedName: string) {
    return invoke<LocalFileSelection | null>('select_download_folder', { suggestedName })
  },

  selectDownloadDirectory() {
    return invoke<string | null>('select_download_directory')
  },

  prepareDownloadLocation(suggestedName: string, downloadDirectory: string) {
    return invoke<LocalFileSelection>('prepare_download_location', {
      suggestedName,
      downloadDirectory,
    })
  },

  prepareDownloadFolder(suggestedName: string, downloadDirectory: string) {
    return invoke<LocalFileSelection>('prepare_download_folder', {
      suggestedName,
      downloadDirectory,
    })
  },

  createFolder(mountId: string, parentPath: string, name: string) {
    return invoke<void>('create_folder', { mountId, parentPath, name })
  },

  renameEntry(mountId: string, path: string, newName: string) {
    return invoke<void>('rename_entry', { mountId, path, newName })
  },

  moveEntry(
    mountId: string,
    path: string,
    destinationMountId: string,
    destinationDirectory: string,
  ) {
    return invoke<void>('move_entry', {
      mountId,
      path,
      destinationMountId,
      destinationDirectory,
    })
  },

  copyEntry(
    mountId: string,
    path: string,
    destinationMountId: string,
    destinationDirectory: string,
  ) {
    return invoke<void>('copy_entry', {
      mountId,
      path,
      destinationMountId,
      destinationDirectory,
    })
  },

  deleteEntry(mountId: string, path: string) {
    return invoke<void>('delete_entry', { mountId, path })
  },

  uploadFile(mountId: string, remoteDirectory: string, localPathGrant: string) {
    const transferId = crypto.randomUUID()
    return {
      transferId,
      result: invoke<TransferResult>('upload_file', {
        transferId,
        mountId,
        remoteDirectory,
        localPathGrant,
      }),
    }
  },

  uploadSplitFile(
    mountId: string,
    remoteDirectory: string,
    localPathGrant: string,
    settings: SplitUploadSettings,
  ) {
    const transferId = crypto.randomUUID()
    return {
      transferId,
      result: invoke<TransferResult>('upload_split_file', {
        request: {
          transferId,
          mountId,
          remoteDirectory,
          localPathGrant,
          packageName: settings.packageName,
          partBytes: settings.partBytes,
        },
      }),
    }
  },

  downloadFile(mountId: string, remotePath: string, localPathGrant: string) {
    const transferId = crypto.randomUUID()
    return {
      transferId,
      result: invoke<TransferResult>('download_file', {
        transferId,
        mountId,
        remotePath,
        localPathGrant,
      }),
    }
  },

  downloadFolder(mountId: string, remotePath: string, localPathGrant: string) {
    const transferId = crypto.randomUUID()
    return {
      transferId,
      result: invoke<TransferResult>('download_folder', {
        transferId,
        mountId,
        remotePath,
        localPathGrant,
      }),
    }
  },

  cancelTransfer(transferId: string) {
    return invoke<boolean>('cancel_transfer', { transferId })
  },

  pauseTransfer(transferId: string) {
    return invoke<boolean>('pause_transfer', { transferId })
  },

  listResumableTransfers() {
    return invoke<ResumableTransfer[]>('list_resumable_transfers')
  },

  resumeTransfer(transferId: string) {
    return invoke<TransferResult>('resume_transfer', { transferId })
  },

  discardResumableTransfer(transferId: string) {
    return invoke<boolean>('discard_resumable_transfer', { transferId })
  },

  openDownloadedFile(transferId: string) {
    return invoke<void>('open_downloaded_file', { transferId })
  },

  openDownloadedFolder(transferId: string) {
    return invoke<void>('open_downloaded_folder', { transferId })
  },

  onTransferProgress(listener: (progress: TransferProgress) => void): Promise<UnlistenFn> {
    return listen<TransferProgress>(TRANSFER_EVENT, (event) => listener(event.payload))
  },
}
