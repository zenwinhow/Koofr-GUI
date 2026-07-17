import { Copy, FileText, Folder, LoaderCircle, Share2 } from 'lucide-react'
import { useState } from 'react'
import { Modal } from '../../components/Modal'
import { commandErrorMessage } from '../../services/koofr'
import { publicLinks } from '../../services/publicLinks'
import type { PublicLink, PublicLinkKind, RemoteFile } from '../../types/backend'
import { isDirectory } from '../files/filePresentation'

interface CreateShareLinkDialogProps {
  readonly mountId: string
  readonly file: RemoteFile
  readonly onClose: () => void
}

export function CreateShareLinkDialog({ mountId, file, onClose }: CreateShareLinkDialogProps) {
  const directory = isDirectory(file)
  const [kind, setKind] = useState<PublicLinkKind>('download')
  const [created, setCreated] = useState<PublicLink | null>(null)
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState(false)

  const createLink = async () => {
    setBusy(true)
    setError('')
    try {
      setCreated(await publicLinks.create(mountId, file.path, kind))
    } catch (createError) {
      setError(commandErrorMessage(createError, '无法为所选项目创建分享链接。'))
    } finally {
      setBusy(false)
    }
  }

  const copyLink = async () => {
    if (!created) return
    try {
      await navigator.clipboard.writeText(created.shortUrl || created.url)
      setCopied(true)
    } catch {
      setError('无法自动复制，请选中地址后手动复制。')
    }
  }

  return (
    <Modal
      title="分享所选项目"
      actionLabel={created ? '完成' : busy ? '正在创建…' : '创建链接'}
      actionDisabled={busy}
      onClose={onClose}
      onAction={created ? onClose : () => void createLink()}
    >
      <div className="create-share-dialog">
        <div className="create-share-target">
          <span aria-hidden="true">{directory ? <Folder /> : <FileText />}</span>
          <span>
            <strong>{file.name}</strong>
            <small>{file.path}</small>
          </span>
        </div>

        {!created ? (
          <label className="create-share-kind">
            分享方式
            {directory ? (
              <select value={kind} onChange={(event) => setKind(event.target.value as PublicLinkKind)} disabled={busy}>
                <option value="download">发送下载链接</option>
                <option value="upload">接收文件链接</option>
              </select>
            ) : (
              <span><Share2 aria-hidden="true" />发送下载链接</span>
            )}
          </label>
        ) : (
          <div className="create-share-result" role="status">
            <strong>链接已创建</strong>
            <small>{created.kind === 'upload' ? '其他人可以向此文件夹上传文件。' : '其他人可以通过此链接下载所选项目。'}</small>
            <input aria-label="分享链接地址" value={created.shortUrl || created.url} readOnly onFocus={(event) => event.currentTarget.select()} />
            <button className="secondary-button" type="button" onClick={() => void copyLink()}>
              <Copy aria-hidden="true" />{copied ? '已复制' : '复制链接'}
            </button>
          </div>
        )}

        {busy ? <p className="create-share-busy" role="status"><LoaderCircle className="spin" />正在请求 Koofr…</p> : null}
        {error ? <p className="settings-error" role="alert">{error}</p> : null}
      </div>
    </Modal>
  )
}
