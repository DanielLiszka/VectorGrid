<script setup lang="ts">
import { computed, ref, useSlots, watch } from "vue";
import { copyTextToDesktopClipboard } from "../services/desktopClipboard";
import { fileBasename } from "../utils/core/path";
import AppButton from "./ui/AppButton.vue";

const props = withDefaults(
  defineProps<{
    label: string;
    value: string;
    kind: "path" | "hash";
    showCopy?: boolean;
  }>(),
  {
    showCopy: true,
  },
);

const slots = useSlots();
const copyState = ref<"idle" | "copied" | "failed">("idle");
let copyRequest = 0;

const fullValue = computed(() => {
  const value = String(props.value || "").trim();
  return value === "—" || value === "-" ? "" : value;
});
const displayValue = computed(() => {
  const value = fullValue.value;
  if (!value) return "—";
  if (props.kind === "hash") {
    return value.length > 20
      ? `${value.slice(0, 10)}…${value.slice(-6)}`
      : value;
  }

  return fileBasename(value) || value;
});
const accessibleLabel = computed(() =>
  fullValue.value ? `${props.label}: ${fullValue.value}` : `${props.label}: unavailable`,
);
const copyStatus = computed(() => {
  if (copyState.value === "copied") return `Copied full ${props.label}.`;
  if (copyState.value === "failed") return `Could not copy full ${props.label}.`;
  return "";
});
const hasActions = computed(
  () => (props.showCopy && !!fullValue.value) || !!slots.actions,
);

watch(
  () => props.value,
  () => {
    copyRequest += 1;
    copyState.value = "idle";
  },
);

async function copyFullValue(): Promise<void> {
  const value = fullValue.value;
  if (!value) return;
  const request = ++copyRequest;
  try {
    const result = await copyTextToDesktopClipboard(value);
    if (request !== copyRequest) return;
    copyState.value = result.ok ? "copied" : "failed";
  } catch {
    if (request !== copyRequest) return;
    copyState.value = "failed";
  }
}
</script>

<template>
  <div class="compactEvidenceValue">
    <span
      class="compactEvidenceText mono"
      :title="fullValue || undefined"
      :aria-label="accessibleLabel"
    >
      {{ displayValue }}
    </span>
    <div v-if="hasActions" class="compactEvidenceActions">
      <AppButton
        v-if="showCopy && fullValue"
        size="sm"
        variant="subtle"
        class="compactEvidenceAction"
        data-qa-action="evidence.copy"
        :aria-label="`Copy full ${label}`"
        @click.stop="copyFullValue"
      >
        {{ copyState === "copied" ? "Copied" : copyState === "failed" ? "Retry copy" : "Copy" }}
      </AppButton>
      <slot name="actions" />
    </div>
    <span class="srOnly" role="status" aria-live="polite" aria-atomic="true">
      {{ copyStatus }}
    </span>
  </div>
</template>

<style scoped>
.compactEvidenceValue {
  min-width: 0;
  display: grid;
  gap: 0.3rem;
}

.compactEvidenceText {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.compactEvidenceActions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.35rem;
}

.compactEvidenceActions :deep(.compactEvidenceAction) {
  min-height: 1.8rem;
  padding-block: 0.25rem;
}

.srOnly {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border: 0;
}
</style>
