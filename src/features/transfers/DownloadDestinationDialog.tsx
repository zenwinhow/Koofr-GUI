import { Download, FolderOpen } from 'lucide-react'
import { useEffect, useState } from 'react'
import { Modal } from '../../components/Modal'

interface DownloadDestinationDialogProps {
  readonly fileName: string
  readonly initialDirectory: string
  readonly initialAskEveryTime: boolean
  readonly busy: boolean
  readonly error: string
  readonly onBrowse: () => Promise<string | null>
  readonly onClose: () => void
  readonly onConfirm: (directory: string, askEveryTime: boolean) => void
}

export function DownloadDestinationDialog({
  fileName,
  initialDirectory,
  initialAskEveryTime,
  busy,
  error,
  onBrowse,
  onClose,
  onConfirm,
}: DownloadDestinationDialogProps) {
  const [directory, setDirectory] = useState(initialDirectory)
  const [askEveryTime, setAskEveryTime] = useState(initialAskEveryTime)
  const [browsing, setBrowsing] = useState(false)
  const [browseError, setBrowseError] = useState('')

  useEffect(() => {
    setDirectory(initialDirectory)
  }, [initialDirectory])

  useEffect(() => {
    setAskEveryTime(initialAskEveryTime)
  }, [initialAskEveryTime])

  const browse = async () => {
    setBrowsing(true)
    setBrowseError('')
    try {
      const selected = await onBrowse()
      if (selected) setDirectory(selected)
    } catch (browseFailure) {
      setBrowseError(browseFailure instanceof Error
        ? '无法打开文件夹选择器，请手动填写路径。'
        : '无法选择下载文件夹，请手动填写路径。')
    } finally {
      setBrowsing(false)
    }
  }

  const trimmedDirectory = directory.trim()
  const displayedError = error || browseError

  return (
    <Modal
      title="选择下载位置"
      actionLabel={busy ? '正在准备下载…' : '开始下载'}
      actionDisabled={busy || browsing || !trimmedDirectory}
      onClose={onClose}
      onAction={() => onConfirm(trimmedDirectory, askEveryTime)}
    >
      <div className="download-dialog__summary">
        <span className="download-dialog__icon"><Download aria-hidden="true" /></span>
        <span>
          <small>即将下载</small>
          <strong title={fileName}>{fileName}</strong>
        </span>
      </div>
      <div className="path-field modal-field--spaced">
        <label htmlFor="download-directory">下载到</label>
        <span className="path-field__control">
          <input
            id="download-directory"
            value={directory}
            disabled={busy}
            title={directory}
            aria-invalid={displayedError ? true : undefined}
            aria-describedby={displayedError ? 'download-directory-error' : 'download-directory-hint'}
            onChange={(event) => setDirectory(event.target.value)}
          />
          <button
            type="button"
            aria-label="选择文件夹"
            title="选择文件夹"
            disabled={busy || browsing}
            onClick={() => void browse()}
          >
            <FolderOpen aria-hidden="true" />
          </button>
        </span>
      </div>
      {displayedError ? (
        <p className="field-message field-message--error" id="download-directory-error" role="alert">
          {displayedError}
        </p>
      ) : (
        <p className="field-message" id="download-directory-hint">
          文件重名时会自动保留两份，不会覆盖已有内容。
        </p>
      )}
      <div className="download-dialog__preference">
        <span>
          <strong>每次下载前询问保存位置</strong>
          <small>关闭后会将本次选择的文件夹设为默认位置。</small>
        </span>
        <button
          className={`settings-switch${askEveryTime ? ' settings-switch--on' : ''}`}
          type="button"
          role="switch"
          aria-checked={askEveryTime}
          aria-label="每次下载前询问保存位置"
          disabled={busy}
          onClick={() => setAskEveryTime((current) => !current)}
        >
          <span aria-hidden="true" />
        </button>
      </div>
    </Modal>
  )
}
