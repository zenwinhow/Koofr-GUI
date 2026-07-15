import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import type {
  KoofrMount,
  KoofrSession,
  LocalFileSelection,
  RemoteFile,
  TransferProgress,
  TransferResult,
  CommandError,
} from '../types/backend'

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

export const koofr = {
  connect(email: string, appPassword: string) {
    return invoke<KoofrSession>('connect_koofr', { email, appPassword })
  },

  disconnect() {
    return invoke<void>('disconnect_koofr')
  },

  session() {
    return invoke<KoofrSession>('koofr_session')
  },

  listMounts() {
    return invoke<KoofrMount[]>('list_mounts')
  },

  listFiles(mountId: string, path = '/') {
    return invoke<RemoteFile[]>('list_files', { mountId, path })
  },

  selectUploadFile() {
    return invoke<LocalFileSelection | null>('select_upload_file')
  },

  selectDownloadLocation(suggestedName: string) {
    return invoke<LocalFileSelection | null>('select_download_location', { suggestedName })
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

  cancelTransfer(transferId: string) {
    return invoke<boolean>('cancel_transfer', { transferId })
  },

  onTransferProgress(listener: (progress: TransferProgress) => void): Promise<UnlistenFn> {
    return listen<TransferProgress>(TRANSFER_EVENT, (event) => listener(event.payload))
  },
}
