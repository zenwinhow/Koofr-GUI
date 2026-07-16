import { isDirectory } from '../files/filePresentation'
import { koofr } from '../../services/koofr'
import type { RemoteFile, TransferResult } from '../../types/backend'

interface StartedDownload {
  transferId: string
  result: Promise<TransferResult>
  localKind: 'file' | 'folder'
}

export async function beginDownload(
  file: RemoteFile,
  mountId: string,
): Promise<StartedDownload | null> {
  if (isDirectory(file)) {
    const selection = await koofr.selectDownloadFolder(file.name)
    if (!selection) return null
    return {
      ...koofr.downloadFolder(mountId, file.path, selection.grantId),
      localKind: 'folder',
    }
  }

  const selection = await koofr.selectDownloadLocation(file.name)
  if (!selection) return null
  return {
    ...koofr.downloadFile(mountId, file.path, selection.grantId),
    localKind: 'file',
  }
}
