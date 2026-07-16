import { fireEvent, render, screen } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { describe, expect, it, vi } from 'vitest'
import { Modal } from './Modal'

describe('Modal', () => {
  it('closes when the backdrop is clicked', async () => {
    // Given
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <Modal title="设置" actionLabel="完成" onClose={onClose}>
        <p>设置内容</p>
      </Modal>,
    )

    // When
    const backdrop = document.querySelector('.modal-dismiss-surface')
    expect(backdrop).toBeInstanceOf(HTMLElement)
    if (!(backdrop instanceof HTMLElement)) return
    await user.click(backdrop)

    // Then
    expect(onClose).toHaveBeenCalledOnce()
  })

  it('stays open when the dialog surface is clicked', async () => {
    // Given
    const user = userEvent.setup()
    const onClose = vi.fn()
    render(
      <Modal title="设置" actionLabel="完成" onClose={onClose}>
        <p>设置内容</p>
      </Modal>,
    )

    // When
    const surface = document.querySelector('.modal')
    expect(surface).toBeInstanceOf(HTMLElement)
    if (!(surface instanceof HTMLElement)) return
    await user.click(surface)

    // Then
    expect(onClose).not.toHaveBeenCalled()
  })

  it('closes when Escape is pressed', () => {
    // Given
    const onClose = vi.fn()
    render(
      <Modal title="设置" actionLabel="完成" onClose={onClose}>
        <p>设置内容</p>
      </Modal>,
    )

    // When
    fireEvent(screen.getByRole('dialog'), new Event('cancel', { cancelable: true }))

    // Then
    expect(onClose).toHaveBeenCalledOnce()
  })

  it('focuses the visible close button instead of the transparent backdrop', () => {
    render(
      <Modal title="设置" actionLabel="完成" onClose={vi.fn()}>
        <p>设置内容</p>
      </Modal>,
    )

    const visibleClose = screen.getByRole('button', { name: '关闭' })
    const backdrop = document.querySelector('.modal-dismiss-surface')
    expect(document.activeElement).toBe(visibleClose)
    expect(backdrop?.getAttribute('tabindex')).toBe('-1')
    expect(backdrop?.getAttribute('aria-hidden')).toBe('true')
  })
})
