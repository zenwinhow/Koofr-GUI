import { CheckCircle2, Cloud, Copy, Database, ExternalLink, HardDrive } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Modal } from '../../components/Modal'
import { formatStorageMegabytes } from '../files/filePresentation'
import type { KoofrMount } from '../../types/backend'
import { identifyMountProvider } from './mountProviders'

const ACCOUNT_URL = 'https://app.koofr.net'

interface ConnectedPlacesDialogProps {
  readonly mounts: readonly KoofrMount[]
  readonly onClose: () => void
}

export function ConnectedPlacesDialog({ mounts, onClose }: ConnectedPlacesDialogProps) {
  const [copied, setCopied] = useState(false)
  const connected = useMemo(() => mounts.map((mount) => ({
    mount,
    provider: identifyMountProvider(mount),
  })), [mounts])

  const copyAccountAddress = async () => {
    try {
      await navigator.clipboard.writeText(ACCOUNT_URL)
      setCopied(true)
    } catch {
      setCopied(false)
    }
  }

  return (
    <Modal title="已连接的存储" actionLabel="完成" onClose={onClose} wide>
      <div className="places-dialog">
        <p className="places-dialog__intro">
          这些位置已由 Koofr 账户授权，可直接在“我的文件”的存储选择器中浏览和管理文件。
        </p>

        <div className="places-grid" aria-label="已连接存储列表">
          {connected.map(({ mount, provider }) => (
            <article className="place-card" data-provider={provider.id} key={mount.id}>
              <span className="place-card__icon" aria-hidden="true">
                {provider.id === 'koofr' ? <Database /> : provider.id === 'other' ? <HardDrive /> : <Cloud />}
              </span>
              <span className="place-card__body">
                <span className="place-card__title">
                  <strong>{mount.name || provider.label}</strong>
                  <small>{provider.label}</small>
                </span>
                <span className={`place-card__status${mount.online ? ' place-card__status--online' : ''}`}>
                  <CheckCircle2 aria-hidden="true" />
                  {mount.online ? '在线' : '暂时离线'}
                </span>
                <small>
                  {mount.spaceTotal > 0
                    ? `${formatStorageMegabytes(mount.spaceUsed)} / ${formatStorageMegabytes(mount.spaceTotal)}`
                    : '容量由服务提供方管理'}
                </small>
              </span>
            </article>
          ))}
        </div>

        <section className="places-management">
          <ExternalLink aria-hidden="true" />
          <span>
            <strong>连接、移除或重新授权</strong>
            <small>Koofr 暂未公开这些授权操作所需的桌面 API。登录后请打开“账户 → 位置”。</small>
          </span>
          <label>
            官方页面地址
            <input value={ACCOUNT_URL} readOnly onFocus={(event) => event.currentTarget.select()} />
          </label>
          <button className="secondary-button" type="button" onClick={() => void copyAccountAddress()}>
            <Copy aria-hidden="true" />
            {copied ? '已复制' : '复制地址'}
          </button>
        </section>
      </div>
    </Modal>
  )
}
