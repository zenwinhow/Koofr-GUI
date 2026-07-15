import type { RemoteFile } from '../../types/backend'
import type { FileKind } from '../../types/files'

const FILE_SIZE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const

export function isDirectory(file: RemoteFile) {
  return file.entryType === 'dir' || file.entryType === 'folder'
}

export function fileKind(file: RemoteFile): FileKind {
  if (isDirectory(file)) return 'folder'
  if (file.contentType.startsWith('image/')) return 'image'

  const extension = file.name.split('.').pop()?.toLocaleLowerCase('en-US')
  if (extension === 'xlsx' || extension === 'xls' || extension === 'ods') return 'xlsx'
  if (extension === 'pdf') return 'pdf'
  if (extension === 'docx' || extension === 'doc' || extension === 'odt') return 'docx'
  return 'file'
}

export function formatBytes(value: number | null, empty = '—') {
  if (value === null || !Number.isFinite(value) || value < 0) return empty
  if (value === 0) return '0 B'

  const unitIndex = Math.min(
    Math.floor(Math.log(value) / Math.log(1024)),
    FILE_SIZE_UNITS.length - 1,
  )
  const amount = value / (1024 ** unitIndex)
  const digits = unitIndex === 0 || amount >= 100 ? 0 : amount >= 10 ? 1 : 2
  return `${amount.toFixed(digits)} ${FILE_SIZE_UNITS[unitIndex]}`
}

// Koofr mount quota fields use MiB, while file and transfer sizes use bytes.
export function formatStorageMegabytes(value: number | null, empty = '—') {
  if (value === null || !Number.isFinite(value) || value < 0) return empty
  return formatBytes(value * (1024 ** 2), empty)
}

export function formatModified(timestamp: number) {
  if (!Number.isFinite(timestamp) || timestamp <= 0) return '—'
  const milliseconds = timestamp < 1_000_000_000_000 ? timestamp * 1000 : timestamp
  const date = new Date(milliseconds)
  if (Number.isNaN(date.getTime())) return '—'
  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: '2-digit',
    day: '2-digit',
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  }).format(date)
}

export interface PathCrumb {
  label: string
  path: string
}

export function pathCrumbs(path: string): PathCrumb[] {
  const segments = path.split('/').filter(Boolean)
  return segments.map((segment, index) => ({
    label: segment,
    path: `/${segments.slice(0, index + 1).join('/')}`,
  }))
}
