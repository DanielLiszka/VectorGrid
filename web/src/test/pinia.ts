import { createTestingPinia, type TestingOptions } from '@pinia/testing'
import { setActivePinia, type Pinia } from 'pinia'
import { vi } from 'vitest'
import { bindRuntimePinia } from '../stores/runtimePinia'

type TestPiniaOptions = Omit<TestingOptions, 'createSpy'> & {
  bindRuntime?: boolean
}

export function createTestPinia(options: TestPiniaOptions = {}): Pinia {
  const { bindRuntime = false, ...testingOptions } = options
  const pinia = createTestingPinia({
    createSpy: vi.fn,
    stubActions: false,
    ...testingOptions,
  })

  setActivePinia(pinia)
  if (bindRuntime) {
    bindRuntimePinia(pinia)
  }

  return pinia
}
