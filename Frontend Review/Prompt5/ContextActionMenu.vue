<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'
import type { AppIcon } from '../utils/iconography'
import {
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuPortal,
  DropdownMenuRoot,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from 'reka-ui'

interface ContextActionMenuItem {
  key: string
  label: string
  icon?: AppIcon
  disabled?: boolean
  destructive?: boolean
  separatorBefore?: boolean
}

const props = withDefaults(
  defineProps<{
    items: ContextActionMenuItem[]
    label?: string
    ariaLabel?: string
    side?: 'top' | 'right' | 'bottom' | 'left'
    align?: 'start' | 'center' | 'end'
  }>(),
  {
    label: 'More actions',
    ariaLabel: 'More actions',
    side: 'bottom',
    align: 'end',
  },
)

const emit = defineEmits<{
  (event: 'select', key: string): void
}>()

const open = ref(false)
const triggerRef = ref<HTMLButtonElement | null>(null)

const renderedItems = computed(() =>
  Array.isArray(props.items)
    ? props.items.filter(
        (item) =>
          String(item?.key || '').trim().length > 0 &&
          String(item?.label || '').trim().length > 0,
      )
    : [],
)

const triggerDisabled = computed(
  () => renderedItems.value.length === 0 || renderedItems.value.every((item) => !!item.disabled),
)

function handleSelect(key: string): void {
  if (!String(key || '').trim()) return
  emit('select', key)
}

watch(
  open,
  async (isOpen, wasOpen) => {
    if (isOpen || !wasOpen) return
    await nextTick()
    triggerRef.value?.focus()
  },
  { flush: 'post' },
)
</script>

<template>
  <DropdownMenuRoot v-model:open="open">
    <DropdownMenuTrigger as-child>
      <button
        ref="triggerRef"
        type="button"
        class="contextActionMenuTrigger"
        :aria-label="ariaLabel"
        :disabled="triggerDisabled"
      >
        {{ label }}
      </button>
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent
        class="contextActionMenuContent"
        :side="side"
        :align="align"
        :side-offset="8"
        :collision-padding="12"
      >
        <template
          v-for="item in renderedItems"
          :key="`context_action_${item.key}`"
        >
          <DropdownMenuSeparator
            v-if="item.separatorBefore"
            class="contextActionMenuSeparator"
          />
          <DropdownMenuItem
            class="contextActionMenuItem"
            :class="{ destructive: item.destructive }"
            :disabled="!!item.disabled"
            @select="handleSelect(item.key)"
          >
            <component :is="item.icon" v-if="item.icon" class="contextActionMenuIcon" aria-hidden="true" />
            <span class="contextActionMenuLabel">{{ item.label }}</span>
          </DropdownMenuItem>
        </template>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>

<style>
.contextActionMenuTrigger {
  padding: 0.42rem 0.6rem;
  font-size: 0.76rem;
  white-space: nowrap;
}

.contextActionMenuContent {
  z-index: 4088;
  min-width: 11rem;
  border: 1px solid color-mix(in oklab, var(--panel-focus-edge) 68%, var(--border));
  border-radius: 12px;
  background: linear-gradient(
    145deg,
    color-mix(in oklab, var(--surface-focus) 94%, var(--panel-footer)),
    color-mix(in oklab, var(--panel-footer) 96%, var(--surface-working))
  );
  box-shadow: var(--shadow-focus);
  padding: 0.35rem;
  display: grid;
  gap: 0.15rem;
}

.contextActionMenuItem {
  border-radius: 9px;
  padding: 0.5rem 0.65rem;
  font-size: 0.78rem;
  line-height: 1.2;
  color: var(--text);
  outline: none;
  cursor: pointer;
  user-select: none;
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}

.contextActionMenuIcon {
  width: 0.9rem;
  height: 0.9rem;
  flex: 0 0 auto;
}

.contextActionMenuLabel {
  min-width: 0;
}

.contextActionMenuItem[data-highlighted],
.contextActionMenuItem:focus-visible {
  background: color-mix(in oklab, var(--selection-fill) 58%, var(--surface-focus));
  color: var(--selection-text);
}

.contextActionMenuItem[data-disabled] {
  opacity: 0.45;
  cursor: not-allowed;
}

.contextActionMenuItem.destructive {
  color: var(--danger);
}

.contextActionMenuItem.destructive[data-highlighted],
.contextActionMenuItem.destructive:focus-visible {
  background: color-mix(in oklab, var(--warning-soft) 45%, var(--surface-focus));
  color: var(--danger);
}

.contextActionMenuSeparator {
  height: 1px;
  margin: 0.2rem 0.1rem;
  background: color-mix(in oklab, var(--border) 82%, transparent);
}
</style>
