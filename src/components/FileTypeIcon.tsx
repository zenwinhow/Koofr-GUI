import type { ReactNode } from 'react'
import type { FileKind } from '../types/files'

interface FileTypeIconProps {
  readonly kind: FileKind
}

function assertNever(value: never): never {
  throw new TypeError(`Unsupported file kind: ${value}`)
}

function artwork(kind: FileKind): ReactNode {
  switch (kind) {
    case 'folder':
      return (
        <>
          <path className="file-type-icon__folder-back" d="M3 10.5A3.5 3.5 0 0 1 6.5 7h6.4l3.2 3.4h13.4A3.5 3.5 0 0 1 33 14v16.5a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3Z" />
          <path className="file-type-icon__folder-tab" d="M4.5 10.3V8.9c0-.8.7-1.5 1.5-1.5h6.7l2.6 2.9Z" />
          <path className="file-type-icon__folder-front" d="M3 16h30v14.5a3 3 0 0 1-3 3H6a3 3 0 0 1-3-3Z" />
          <path className="file-type-icon__folder-highlight" d="M5.2 18.2h25.6" />
        </>
      )
    case 'xlsx':
      return (
        <>
          <DocumentShell />
          <rect className="file-type-icon__badge" x="3" y="12" width="20" height="19" rx="3" />
          <path className="file-type-icon__detail" d="M8 16.5h10M8 21.5h10M8 26.5h10M11.5 16.5v10M15 16.5v10" />
        </>
      )
    case 'pdf':
      return (
        <>
          <DocumentShell />
          <path className="file-type-icon__pdf-mark" d="M11.1 27.8c2.4-4.4 4.2-8.8 5.2-13.5.8 4 2.5 7.5 5.6 10.5-4.8-.5-8.5.4-11.8 2.7 5.7-1.4 10.2-1.3 14.3.2" />
        </>
      )
    case 'docx':
      return (
        <>
          <DocumentShell />
          <rect className="file-type-icon__badge" x="3" y="12" width="20" height="19" rx="3" />
          <path className="file-type-icon__detail" d="M8 17h10M8 21.5h8M8 26h10" />
        </>
      )
    case 'image':
      return (
        <>
          <DocumentShell />
          <rect className="file-type-icon__badge" x="3" y="12" width="21" height="19" rx="3" />
          <circle className="file-type-icon__detail-fill" cx="17.4" cy="17.6" r="2.1" />
          <path className="file-type-icon__detail" d="m6.6 27 4.4-5 3.2 3.2 2.3-2.5 4 4.3Z" />
        </>
      )
    case 'archive':
      return (
        <>
          <DocumentShell />
          <rect className="file-type-icon__badge" x="3" y="12" width="20" height="19" rx="3" />
          <path className="file-type-icon__detail" d="M13 15.5v12M10.5 17h5M10.5 20h5M10.5 23h5M10.5 26h5" />
        </>
      )
    case 'executable':
      return (
        <>
          <DocumentShell />
          <rect className="file-type-icon__badge" x="3" y="12" width="22" height="19" rx="3" />
          <path className="file-type-icon__detail" d="m7.5 18 3.5 3-3.5 3M13.5 25h6" />
        </>
      )
    case 'file':
      return (
        <>
          <DocumentShell />
          <circle className="file-type-icon__generic-dot" cx="10" cy="15" r="2" />
          <path className="file-type-icon__generic-line" d="M15 15h7M9 21h13M9 26h10" />
        </>
      )
    default:
      return assertNever(kind)
  }
}

function DocumentShell() {
  return (
    <>
      <path className="file-type-icon__paper" d="M8 3h13l8 8v21.5a2.5 2.5 0 0 1-2.5 2.5h-19A2.5 2.5 0 0 1 5 32.5v-27A2.5 2.5 0 0 1 7.5 3Z" />
      <path className="file-type-icon__fold" d="M21 3v6a2 2 0 0 0 2 2h6Z" />
      <path className="file-type-icon__sheen" d="M8 5.8v23.7" />
    </>
  )
}

export function FileTypeIcon({ kind }: FileTypeIconProps) {
  return (
    <span className={`file-type-icon file-type-icon--${kind}`} data-file-kind={kind} aria-hidden="true">
      <svg viewBox="0 0 36 38" focusable="false">
        {artwork(kind)}
      </svg>
    </span>
  )
}
