import type { RemoteFile } from '../../types/backend'
import type { FileKind } from '../../types/files'

const FILE_SIZE_UNITS = ['B', 'KB', 'MB', 'GB', 'TB'] as const
const ARCHIVE_EXTENSIONS: ReadonlySet<string> = new Set([
  'zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz', 'zst', 'zstd', 'lz', 'lzma', 'lz4',
  'tgz', 'tbz', 'tbz2', 'txz', 'tzst', 'z', 'cab', 'arj', 'ace', 'sit', 'sitx',
])
const EXECUTABLE_EXTENSIONS: ReadonlySet<string> = new Set([
  'exe', 'msi', 'msix', 'appx', 'appxbundle', 'bat', 'cmd', 'com', 'ps1',
  'apk', 'aab', 'ipa', 'app', 'deb', 'rpm', 'pkg', 'dmg', 'snap', 'flatpak', 'appimage',
])
const VIDEO_EXTENSIONS: ReadonlySet<string> = new Set([
  'mp4', 'm4v', 'mov', 'avi', 'wmv', 'flv', 'mkv', 'webm', 'mpeg', 'mpg', 'mpe',
  'mts', 'm2ts', 'ts', '3gp', '3g2', 'vob', 'ogv', 'rm', 'rmvb', 'asf', 'f4v',
])
const AUDIO_EXTENSIONS: ReadonlySet<string> = new Set([
  'mp3', 'wav', 'flac', 'aac', 'm4a', 'm4b', 'ogg', 'oga', 'opus', 'wma',
  'aiff', 'aif', 'ape', 'alac', 'amr', 'mid', 'midi', 'dsd', 'dsf', 'dff',
])
const IMAGE_EXTENSIONS: ReadonlySet<string> = new Set([
  'png', 'jpg', 'jpeg', 'jfif', 'gif', 'webp', 'bmp', 'tif', 'tiff', 'ico',
  'svg', 'heic', 'heif', 'avif', 'raw', 'cr2', 'nef', 'arw', 'dng', 'orf', 'raf', 'psd', 'ai',
])
const CODE_EXTENSIONS: ReadonlySet<string> = new Set([
  'js', 'jsx', 'mjs', 'cjs', 'tsx', 'json', 'jsonc', 'json5',
  'html', 'htm', 'xhtml', 'xml', 'xsl', 'xslt', 'css', 'scss', 'sass', 'less', 'styl',
  'py', 'pyw', 'ipynb', 'rb', 'php', 'go', 'rs', 'toml',
  'java', 'kt', 'kts', 'scala', 'groovy', 'gradle',
  'c', 'h', 'cc', 'cpp', 'cxx', 'hh', 'hpp', 'hxx', 'm', 'mm',
  'cs', 'fs', 'fsx', 'vb', 'swift', 'dart',
  'sh', 'bash', 'zsh', 'fish', 'sql',
  'lua', 'pl', 'pm', 'r', 'jl', 'ex', 'exs', 'erl', 'elm', 'clj', 'cljs', 'hs',
  'yaml', 'yml', 'ini', 'cfg', 'conf', 'env', 'properties',
  'vue', 'svelte', 'astro',
])
const TEXT_EXTENSIONS: ReadonlySet<string> = new Set([
  'txt', 'md', 'markdown', 'rst', 'rtf', 'log', 'csv', 'tsv',
  'tex', 'bib', 'org', 'adoc', 'asciidoc',
])
const FONT_EXTENSIONS: ReadonlySet<string> = new Set([
  'ttf', 'otf', 'woff', 'woff2', 'eot', 'ttc', 'pfa', 'pfb',
])
const EBOOK_EXTENSIONS: ReadonlySet<string> = new Set([
  'epub', 'mobi', 'azw', 'azw3', 'kfx', 'fb2', 'lit', 'lrf', 'ibooks',
])
const DISK_EXTENSIONS: ReadonlySet<string> = new Set([
  'iso', 'img', 'vhd', 'vhdx', 'vmdk', 'vdi', 'qcow2', 'cue', 'nrg', 'mds', 'mdf',
])
const DATABASE_EXTENSIONS: ReadonlySet<string> = new Set([
  'db', 'sqlite', 'sqlite3', 'mdb', 'accdb', 'dbf', 'parquet', 'orc', 'avro', 'arrow', 'feather',
])
const SPREADSHEET_EXTENSIONS: ReadonlySet<string> = new Set([
  'xlsx', 'xlsm', 'xlsb', 'xls', 'xltx', 'xltm', 'ods', 'numbers',
])
const DOCUMENT_EXTENSIONS: ReadonlySet<string> = new Set([
  'docx', 'docm', 'doc', 'dotx', 'dotm', 'odt', 'pages', 'wps',
])
const PRESENTATION_EXTENSIONS: ReadonlySet<string> = new Set([
  'pptx', 'pptm', 'ppt', 'potx', 'potm', 'ppsx', 'pps', 'odp', 'key',
])
const MODIFIED_DATE_FORMATTER = new Intl.DateTimeFormat('zh-CN', {
  year: 'numeric',
  month: '2-digit',
  day: '2-digit',
  hour: '2-digit',
  minute: '2-digit',
  hour12: false,
})

export function isDirectory(file: RemoteFile) {
  return file.entryType === 'dir' || file.entryType === 'folder'
}

export function fileKind(file: RemoteFile): FileKind {
  if (isDirectory(file)) return 'folder'
  const contentType = file.contentType.toLowerCase()
  if (contentType.startsWith('image/')) return 'image'
  if (contentType.startsWith('video/')) return 'video'
  if (contentType.startsWith('audio/')) return 'audio'
  return fileKindByName(file.name)
}

export function fileKindByName(name: string): FileKind {
  const extension = name.split('.').pop()?.toLocaleLowerCase('en-US') ?? ''
  if (IMAGE_EXTENSIONS.has(extension)) return 'image'
  if (VIDEO_EXTENSIONS.has(extension)) return 'video'
  if (AUDIO_EXTENSIONS.has(extension)) return 'audio'
  if (ARCHIVE_EXTENSIONS.has(extension)) return 'archive'
  if (EXECUTABLE_EXTENSIONS.has(extension)) return 'executable'
  if (SPREADSHEET_EXTENSIONS.has(extension)) return 'xlsx'
  if (extension === 'pdf') return 'pdf'
  if (DOCUMENT_EXTENSIONS.has(extension)) return 'docx'
  if (PRESENTATION_EXTENSIONS.has(extension)) return 'pptx'
  if (CODE_EXTENSIONS.has(extension)) return 'code'
  if (TEXT_EXTENSIONS.has(extension)) return 'text'
  if (FONT_EXTENSIONS.has(extension)) return 'font'
  if (EBOOK_EXTENSIONS.has(extension)) return 'ebook'
  if (DISK_EXTENSIONS.has(extension)) return 'disk'
  if (DATABASE_EXTENSIONS.has(extension)) return 'database'
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
  return MODIFIED_DATE_FORMATTER.format(date)
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
