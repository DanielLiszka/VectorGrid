import { computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useSettingsStore, type HelpMode } from '../stores/settingsStore'

export function useHelpMode() {
  const settingsStore = useSettingsStore()
  const { helpMode } = storeToRefs(settingsStore)

  const mode = computed<HelpMode>(() => {
    const next = String(helpMode.value || 'full').trim().toLowerCase()
    if (next === 'concise' || next === 'off') return next
    return 'full'
  })

  const isFull = computed(() => mode.value === 'full')
  const isConcise = computed(() => mode.value === 'concise')
  const isOff = computed(() => mode.value === 'off')

  return {
    mode,
    isFull,
    isConcise,
    isOff,
  }
}
