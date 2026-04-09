<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    idBase: string;
    title: string;
    body: string;
    confirmLabel: string;
    confirmDisabled?: boolean;
    cancelLabel?: string;
    danger?: boolean;
  }>(),
  {
    confirmDisabled: false,
    cancelLabel: "Cancel",
    danger: false,
  },
);

const emit = defineEmits<{
  (event: "cancel"): void;
  (event: "confirm"): void;
}>();

const dialogRef = ref<HTMLElement | null>(null);
const cancelButtonRef = ref<HTMLButtonElement | null>(null);
const confirmButtonRef = ref<HTMLButtonElement | null>(null);
const lastFocusedElement = ref<HTMLElement | null>(null);

const titleId = computed(() => `${props.idBase}_title`);
const bodyId = computed(() => `${props.idBase}_body`);

function dialogFocusTargets(root: HTMLElement): HTMLElement[] {
  const selectors =
    'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';
  return Array.from(root.querySelectorAll<HTMLElement>(selectors)).filter(
    (el) => !el.hasAttribute("disabled") && el.getAttribute("aria-hidden") !== "true",
  );
}

function onDialogKeydown(event: KeyboardEvent): void {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    emit("cancel");
    return;
  }
  if (event.key !== "Tab") return;
  const root = dialogRef.value;
  if (!root) return;
  const targets = dialogFocusTargets(root);
  if (!targets.length) {
    event.preventDefault();
    root.focus();
    return;
  }
  const first = targets[0];
  const last = targets[targets.length - 1];
  const active =
    document.activeElement instanceof HTMLElement ? document.activeElement : null;
  if (event.shiftKey) {
    if (active === first || active === root) {
      event.preventDefault();
      last?.focus();
    }
    return;
  }
  if (active === last) {
    event.preventDefault();
    first?.focus();
  }
}

watch(
  () => props.open,
  async (isOpen, wasOpen) => {
    if (isOpen) {
      lastFocusedElement.value =
        document.activeElement instanceof HTMLElement
          ? document.activeElement
          : null;
      await nextTick();
      const preferredTarget =
        (!props.confirmDisabled && confirmButtonRef.value) ||
        cancelButtonRef.value ||
        dialogRef.value;
      preferredTarget?.focus();
      return;
    }
    if (!wasOpen) return;
    await nextTick();
    lastFocusedElement.value?.focus();
  },
  { flush: "post", immediate: true },
);

onBeforeUnmount(() => {
  lastFocusedElement.value = null;
});
</script>

<template>
  <div
    v-if="open"
    class="modal open"
    @keydown.capture="onDialogKeydown"
  >
    <div class="modalBackdrop" @click="emit('cancel')"></div>
    <section
      ref="dialogRef"
      class="modalDialog confirmationDialog"
      role="alertdialog"
      aria-modal="true"
      :aria-labelledby="titleId"
      :aria-describedby="bodyId"
      tabindex="-1"
    >
      <div class="modalHeader">
        <div class="confirmationDialogHeader">
          <div class="panelEyebrow">Confirmation</div>
          <h3 :id="titleId" class="panelPaneTitle">{{ title }}</h3>
        </div>
      </div>
      <div class="modalBody confirmationDialogBody">
        <p :id="bodyId" class="panelMuted confirmationDialogCopy">
          {{ body }}
        </p>
        <div class="confirmationDialogActions">
          <button ref="cancelButtonRef" type="button" @click="emit('cancel')">
            {{ cancelLabel }}
          </button>
          <button
            ref="confirmButtonRef"
            type="button"
            :class="danger ? 'danger-btn' : 'primary'"
            :disabled="confirmDisabled"
            @click="emit('confirm')"
          >
            {{ confirmLabel }}
          </button>
        </div>
      </div>
    </section>
  </div>
</template>

<style scoped>
.confirmationDialog {
  width: min(440px, 100%);
}

.confirmationDialogHeader {
  display: grid;
  gap: 0.24rem;
  padding-bottom: 0.2rem;
  border-bottom: 1px solid color-mix(in oklab, var(--border) 72%, transparent);
}

.confirmationDialogBody {
  display: grid;
  gap: 0.85rem;
}

.confirmationDialogCopy {
  margin: 0;
  white-space: pre-line;
}

.confirmationDialogActions {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: 0.55rem;
  padding-top: 0.1rem;
}
</style>
