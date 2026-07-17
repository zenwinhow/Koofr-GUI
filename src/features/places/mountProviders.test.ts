import { describe, expect, it } from 'vitest'
import type { KoofrMount } from '../../types/backend'
import { identifyMountProvider } from './mountProviders'

function mount(mountType: string, name: string): KoofrMount {
  return {
    id: `${mountType}-${name}`,
    name,
    mountType,
    spaceTotal: 0,
    spaceUsed: 0,
    online: true,
    isPrimary: false,
    isShared: false,
  }
}

describe('identifyMountProvider', () => {
  it.each([
    ['koofr', 'Koofr', 'koofr'],
    ['google_drive', '工作云盘', 'google-drive'],
    ['onedrive', 'OneDrive', 'onedrive'],
    ['dropbox', 'Dropbox', 'dropbox'],
    ['custom', 'Google Drive 备份', 'google-drive'],
  ] as const)('maps %s / %s to %s', (mountType, name, provider) => {
    expect(identifyMountProvider(mount(mountType, name)).id).toBe(provider)
  })
})
