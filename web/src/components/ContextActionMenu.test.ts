import { mount } from '@vue/test-utils'
import { nextTick } from 'vue'
import { afterEach, describe, expect, it } from 'vitest'
import ContextActionMenu from './ContextActionMenu.vue'

async function flushMenu(): Promise<void> {
  await nextTick()
  await nextTick()
}

describe('ContextActionMenu', () => {
  afterEach(() => {
    document.body.innerHTML = ''
  })

  it('opens, emits the selected action key, and returns focus to the trigger', async () => {
    const wrapper = mount(ContextActionMenu, {
      attachTo: document.body,
      props: {
        ariaLabel: 'Saved artifact actions',
        items: [
          { key: 'refresh', label: 'Refresh', disabled: false },
          { key: 'reveal', label: 'Reveal file', disabled: false },
          {
            key: 'delete',
            label: 'Delete',
            disabled: false,
            destructive: true,
            separatorBefore: true,
          },
        ],
      },
    })

    const trigger = wrapper.get('button.contextActionMenuTrigger')
    const triggerElement = trigger.element as HTMLButtonElement
    triggerElement.focus()
    expect(document.activeElement).toBe(triggerElement)

    await trigger.trigger('click')
    await flushMenu()

    const revealItem = Array.from(document.querySelectorAll<HTMLElement>('[role="menuitem"]')).find(
      (item) => String(item.textContent || '').trim() === 'Reveal file',
    )
    expect(revealItem).toBeTruthy()

    revealItem?.click()
    await flushMenu()

    expect(wrapper.emitted('select')).toEqual([['reveal']])
    expect(document.activeElement).toBe(triggerElement)
  })

  it('closes on escape and restores focus to the trigger', async () => {
    const wrapper = mount(ContextActionMenu, {
      attachTo: document.body,
      props: {
        ariaLabel: 'Saved artifact actions',
        items: [{ key: 'refresh', label: 'Refresh', disabled: false }],
      },
    })

    const trigger = wrapper.get('button.contextActionMenuTrigger')
    const triggerElement = trigger.element as HTMLButtonElement
    triggerElement.focus()

    await trigger.trigger('click')
    await flushMenu()

    const menu = document.querySelector<HTMLElement>('[role="menu"]')
    expect(menu).not.toBeNull()

    menu?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))
    await flushMenu()

    expect(document.querySelector('[role="menu"]')).toBeNull()
    expect(document.activeElement).toBe(triggerElement)
  })

  it('disables the trigger when every action is unavailable', async () => {
    const wrapper = mount(ContextActionMenu, {
      attachTo: document.body,
      props: {
        items: [
          { key: 'refresh', label: 'Refresh', disabled: true },
          { key: 'delete', label: 'Delete', disabled: true, destructive: true },
        ],
      },
    })

    expect(wrapper.get('button.contextActionMenuTrigger').attributes('disabled')).toBeDefined()
  })
})
