import { isDirectory } from '../files/filePresentation'
import { koofr } from '../../services/koofr'
import type { RemoteFile, TransferResult } from '../../types/backend'

interface StartedDownload {
  readonly transferId: string
  readonly result: Promise<TransferResult>
  readonly localKind: 'file' | 'folder'
}

export async function beginDownload(
  file: RemoteFile,
  mountId: string,
  downloadDirectory: string,
): Promise<StartedDownload> {
  if (isDirectory(file)) {
    const selection = await koofr.prepareDownloadFolder(file.name, downloadDirectory)
    return {
      ...koofr.downloadFolder(mountId, file.path, selection.grantId),
      localKind: 'folder',
    }
  }

  const selection = await koofr.prepareDownloadLocation(file.name, downloadDirectory)
  return {
    ...koofr.downloadFile(mountId, file.path, selection.grantId),
    localKind: 'file',
  }
}
