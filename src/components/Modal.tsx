import { X } from 'lucide-react'
import type { ReactNode } from 'react'

interface ModalProps {
  title: string
  children: ReactNode
  actionLabel: string
  onClose: () => void
  onAction?: () => void
  actionDisabled?: boolean
}

export function Modal({ title, children, actionLabel, onClose, onAction, actionDisabled = false }: ModalProps) {
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className="modal"
        role="dialog"
        aria-modal="true"
        aria-labelledby="modal-title"
        onMouseDown={(event) => event.stopPropagation()}
      >
        <button className="icon-button modal__close" type="button" aria-label="关闭" onClick={onClose}>
          <X size={18} />
        </button>
        <h2 id="modal-title">{title}</h2>
        <div className="modal__content">{children}</div>
        <button className="primary-button modal__action" type="button" disabled={actionDisabled} onClick={onAction ?? onClose}>
          {actionLabel}
        </button>
      </section>
    </div>
  )
}
