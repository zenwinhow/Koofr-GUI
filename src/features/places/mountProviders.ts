import type { KoofrMount } from '../../types/backend'

export type MountProviderId = 'koofr' | 'google-drive' | 'onedrive' | 'dropbox' | 'other'

export interface MountProvider {
  readonly id: MountProviderId
  readonly label: string
}

const PROVIDERS: Record<MountProviderId, MountProvider> = {
  koofr: { id: 'koofr', label: 'Koofr' },
  'google-drive': { id: 'google-drive', label: 'Google Drive' },
  onedrive: { id: 'onedrive', label: 'OneDrive' },
  dropbox: { id: 'dropbox', label: 'Dropbox' },
  other: { id: 'other', label: '外部存储' },
}

export function identifyMountProvider(mount: KoofrMount): MountProvider {
  const signature = `${mount.mountType} ${mount.name}`.toLocaleLowerCase('en-US')
  if (signature.includes('google') || signature.includes('gdrive')) return PROVIDERS['google-drive']
  if (signature.includes('onedrive') || signature.includes('one drive') || signature.includes('skydrive')) {
    return PROVIDERS.onedrive
  }
  if (signature.includes('dropbox')) return PROVIDERS.dropbox
  if (mount.isPrimary || signature.includes('koofr')) return PROVIDERS.koofr
  return PROVIDERS.other
}
