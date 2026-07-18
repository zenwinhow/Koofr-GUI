import { Archive, Files } from 'lucide-react'
import { useMemo, useState } from 'react'

import { Modal } from '../../components/Modal'
import type { SplitUploadSettings } from '../../types/files'

const MEBIBYTE = 1024 * 1024
const MIN_PART_MIB = 1
const MAX_PART_MIB = 4096
const PACKAGE_SUFFIX = '.parts'
const MAX_REMOTE_NAME_UNITS = 255

interface SplitUploadDialogProps {
  readonly fileName: string
  readonly onClose: () => void
  readonly onConfirm: (settings: SplitUploadSettings) => void
}

export function defaultSplitPackageName(fileName: string) {
  const available = MAX_REMOTE_NAME_UNITS - PACKAGE_SUFFIX.length
  let base = ''
  for (const character of fileName) {
    if (base.length + character.length > available) break
    base += character
  }
  return `${base || 'file'}${PACKAGE_SUFFIX}`
}

export function SplitUploadDialog({ fileName, onClose, onConfirm }: SplitUploadDialogProps) {
  const [packageName, setPackageName] = useState(() => defaultSplitPackageName(fileName))
  const [partMiB, setPartMiB] = useState('64')
  const parsedPartMiB = Number(partMiB)
  const validPartSize = Number.isInteger(parsedPartMiB)
    && parsedPartMiB >= MIN_PART_MIB
    && parsedPartMiB <= MAX_PART_MIB
  const partCountHint = useMemo(() => {
    if (!validPartSize) return '请输入 1 至 4096 之间的整数。'
    return `每个分卷最多 ${parsedPartMiB} MiB；最后一卷通常更小。`
  }, [parsedPartMiB, validPartSize])
  const trimmedPackageName = packageName.trim()
  const validPackageName = trimmedPackageName.length > 0
    && trimmedPackageName.length <= MAX_REMOTE_NAME_UNITS
    && !trimmedPackageName.includes('/')
    && !trimmedPackageName.includes('\0')
    && trimmedPackageName !== '.'
    && trimmedPackageName !== '..'

  return (
    <Modal
      title="设置可续传分卷"
      actionLabel="开始分卷上传"
      actionDisabled={!validPackageName || !validPartSize}
      onClose={onClose}
      onAction={() => onConfirm({
        packageName: trimmedPackageName,
        partBytes: parsedPartMiB * MEBIBYTE,
      })}
    >
      <div className="split-upload-dialog">
        <div className="split-upload-dialog__summary">
          <span><Archive aria-hidden="true" /></span>
          <span>
            <small>待上传文件</small>
            <strong>{fileName}</strong>
          </span>
        </div>
        <p>
          远端将创建一个文件夹，内部保存原始二进制分卷、恢复命令、SHA-256 校验和与开放格式清单。
        </p>
        <label className="modal-field">
          <span>远端文件夹名称</span>
          <input
            autoFocus
            aria-label="远端文件夹名称"
            value={packageName}
            maxLength={MAX_REMOTE_NAME_UNITS}
            aria-invalid={!validPackageName}
            onChange={(event) => setPackageName(event.target.value)}
          />
          <small>
            <Files aria-hidden="true" />
            {validPackageName ? '可自定义；同名文件夹存在时不会覆盖。' : '名称不能为空或包含“/”。'}
          </small>
        </label>
        <label className="modal-field modal-field--spaced">
          <span>每个分卷大小（MiB）</span>
          <input
            type="number"
            aria-label="每个分卷大小（MiB）"
            value={partMiB}
            min={MIN_PART_MIB}
            max={MAX_PART_MIB}
            step="1"
            aria-invalid={!validPartSize}
            onChange={(event) => setPartMiB(event.target.value)}
          />
          <small>{partCountHint}</small>
        </label>
      </div>
    </Modal>
  )
}
