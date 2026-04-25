import { mount } from '@vue/test-utils'
import { describe, expect, it } from 'vitest'
import AppVisualIcon from './AppVisualIcon.vue'
import { runtimeReadyIcon } from '../utils/appSvgAssets'
import { taskLevelIcon } from '../utils/iconography'

describe('AppVisualIcon', () => {
  it('renders asset icons as decorative images by default', () => {
    const wrapper = mount(AppVisualIcon, {
      props: {
        icon: runtimeReadyIcon,
      },
    })

    const image = wrapper.get('img')
    expect(image.attributes('src')).toBe(runtimeReadyIcon.src)
    expect(image.attributes('alt')).toBe('')
    expect(image.attributes('aria-hidden')).toBe('true')
  })

  it('renders non-decorative icons with accessible naming for both asset and component paths', () => {
    const assetWrapper = mount(AppVisualIcon, {
      props: {
        icon: runtimeReadyIcon,
        decorative: false,
      },
    })
    expect(assetWrapper.get('img').attributes('alt')).toBe(runtimeReadyIcon.label)
    expect(assetWrapper.get('img').attributes('aria-hidden')).toBeUndefined()

    const componentWrapper = mount(AppVisualIcon, {
      props: {
        icon: taskLevelIcon('info'),
        decorative: false,
        alt: 'Info status',
      },
    })

    expect(componentWrapper.find('svg').exists()).toBe(true)
    expect(componentWrapper.find('svg').attributes('aria-hidden')).toBeUndefined()
    expect(componentWrapper.find('svg').attributes('aria-label')).toBe('Info status')
    expect(componentWrapper.find('svg').attributes('role')).toBe('img')
  })
})
