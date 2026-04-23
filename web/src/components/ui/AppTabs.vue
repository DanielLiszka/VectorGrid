<script setup lang="ts">
interface AppTabItem {
  key: string
  label: string
  disabled?: boolean
}

const props = withDefaults(
  defineProps<{
    idBase: string
    items: AppTabItem[]
    modelValue: string
    ariaLabel: string
  }>(),
  {
    items: () => [],
  },
)

const emit = defineEmits<{
  'update:modelValue': [value: string]
}>()

function selectTab(key: string): void {
  const item = props.items.find((candidate) => candidate.key === key)
  if (!item || item.disabled) return
  emit('update:modelValue', key)
}

function moveFocus(event: KeyboardEvent): void {
  const enabled = props.items.filter((item) => !item.disabled)
  if (!enabled.length) return
  const currentIndex = Math.max(0, enabled.findIndex((item) => item.key === props.modelValue))
  let nextIndex = currentIndex
  if (event.key === 'ArrowRight') nextIndex = Math.min(enabled.length - 1, currentIndex + 1)
  else if (event.key === 'ArrowLeft') nextIndex = Math.max(0, currentIndex - 1)
  else if (event.key === 'Home') nextIndex = 0
  else if (event.key === 'End') nextIndex = enabled.length - 1
  else return
  event.preventDefault()
  const next = enabled[nextIndex]
  if (!next) return
  selectTab(next.key)
  const focusNext = () => {
    document.getElementById(`${props.idBase}_${next.key}`)?.focus()
  }
  if (typeof requestAnimationFrame === 'function') requestAnimationFrame(focusNext)
  else setTimeout(focusNext, 0)
}
</script>

<template>
  <nav class="appTabs tabsRow" role="tablist" :aria-label="ariaLabel" @keydown="moveFocus">
    <button
      v-for="item in items"
      :id="`${idBase}_${item.key}`"
      :key="item.key"
      type="button"
      class="tabBtn"
      :class="{ active: modelValue === item.key }"
      role="tab"
      :aria-selected="modelValue === item.key"
      :tabindex="modelValue === item.key ? 0 : -1"
      :disabled="item.disabled"
      @click="selectTab(item.key)"
    >
      {{ item.label }}
    </button>
  </nav>
</template>

<style scoped>
.appTabs {
  min-width: 0;
}
</style>
