import type { Pinia } from 'pinia'
import { useWorkflowStore } from './workflowStore'

let runtimePinia: Pinia | null = null

export function bindRuntimePinia(pinia: Pinia | null | undefined): Pinia | null {
  runtimePinia = pinia ?? null
  return runtimePinia
}

export function resolveRuntimePinia(preferred?: Pinia | null): Pinia | null {
  if (preferred) {
    runtimePinia = preferred
    return preferred
  }
  if (runtimePinia) return runtimePinia
  try {
    const active = (useWorkflowStore() as unknown as { $pinia: Pinia }).$pinia
    runtimePinia = active
    return active
  } catch {
    return null
  }
}
