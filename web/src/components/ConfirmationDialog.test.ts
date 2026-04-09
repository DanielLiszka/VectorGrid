// @vitest-environment jsdom
import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { beforeEach, describe, expect, it } from 'vitest'
import ConfirmationDialog from './ConfirmationDialog.vue'

async function flushDialog(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('ConfirmationDialog', () => {
  beforeEach(() => {
    document.body.innerHTML = ''
  })

  it('traps focus within the dialog and restores focus when closed', async () => {
    const previousFocus = document.createElement('button')
    previousFocus.textContent = 'Previous focus'
    document.body.appendChild(previousFocus)
    previousFocus.focus()

    const wrapper = mount(ConfirmationDialog, {
      attachTo: document.body,
      props: {
        open: true,
        idBase: 'confirm_dialog',
        title: 'Confirm destructive action',
        body: 'This cannot be undone.',
        confirmLabel: 'Delete',
      },
    })

    await flushDialog()

    const buttons = wrapper.findAll('button')
    const cancelButton = buttons[0]?.element as HTMLButtonElement
    const confirmButton = buttons[1]?.element as HTMLButtonElement
    const modal = wrapper.get('.modal').element as HTMLElement

    expect(document.activeElement).toBe(confirmButton)

    modal.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', bubbles: true }))
    await flushDialog()
    expect(document.activeElement).toBe(cancelButton)

    modal.dispatchEvent(new KeyboardEvent('keydown', { key: 'Tab', shiftKey: true, bubbles: true }))
    await flushDialog()
    expect(document.activeElement).toBe(confirmButton)

    await wrapper.setProps({ open: false })
    await flushDialog()

    expect(document.activeElement).toBe(previousFocus)
  })

  it('emits cancel on Escape and keeps alertdialog semantics', async () => {
    const wrapper = mount(ConfirmationDialog, {
      attachTo: document.body,
      props: {
        open: true,
        idBase: 'confirm_dialog',
        title: 'Confirm close',
        body: 'Unsaved work will be lost.',
        confirmLabel: 'Close anyway',
      },
    })

    await flushDialog()

    const modal = wrapper.get('.modal').element as HTMLElement
    modal.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await flushDialog()

    expect(wrapper.get('[role="alertdialog"]').attributes('aria-modal')).toBe('true')
    expect(wrapper.emitted('cancel')).toHaveLength(1)
  })
})
