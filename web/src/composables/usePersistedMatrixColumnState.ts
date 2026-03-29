import { computed, ref, watch, type Ref } from "vue";
import {
  readStorageJson,
  removeStorageKey,
  writeStorageJson,
} from "../utils/storage";
import {
  resolveMatrixColumnMeta,
  type MatrixColumnLike,
} from "./useHeadlessMatrixTable";

interface PersistedMatrixColumnStateShape {
  visibleColumnKeys: string[];
  knownColumnKeys?: string[];
}

interface PersistedMatrixColumnOption {
  key: string;
  label: string;
}

function dedupeKeys(keys: readonly string[]): string[] {
  const next: string[] = [];
  const seen = new Set<string>();
  for (const key of keys) {
    const normalized = String(key || "").trim();
    if (!normalized || seen.has(normalized)) continue;
    seen.add(normalized);
    next.push(normalized);
  }
  return next;
}

function sanitizeVisibleColumnKeys(
  candidateKeys: readonly string[] | null | undefined,
  knownKeys: readonly string[] | null | undefined,
  allColumnKeys: readonly string[],
): string[] {
  if (!allColumnKeys.length) return [];
  const allowed = new Set(allColumnKeys);
  const stored = dedupeKeys(candidateKeys ?? []).filter((key) => allowed.has(key));
  const known = dedupeKeys(knownKeys ?? []).filter((key) => allowed.has(key));
  const visible = stored.length ? new Set(stored) : new Set(allColumnKeys);
  for (const key of allColumnKeys) {
    if (!known.includes(key)) visible.add(key);
  }
  if (!visible.size) visible.add(allColumnKeys[0]!);
  return allColumnKeys.filter((key) => visible.has(key));
}

function readPersistedColumnState(
  storageKey: string,
): PersistedMatrixColumnStateShape | null {
  const persisted = readStorageJson<PersistedMatrixColumnStateShape>(storageKey);
  if (!persisted || !Array.isArray(persisted.visibleColumnKeys)) return null;
  return {
    visibleColumnKeys: persisted.visibleColumnKeys.map((value) =>
      String(value || "").trim(),
    ),
    knownColumnKeys: Array.isArray(persisted.knownColumnKeys)
      ? persisted.knownColumnKeys.map((value) => String(value || "").trim())
      : [],
  };
}

export function usePersistedMatrixColumnState<TColumn extends MatrixColumnLike>(
  columns: Ref<TColumn[]>,
  storageKey: string,
) {
  const visibleColumnKeys = ref<string[]>([]);

  const allColumnOptions = computed<PersistedMatrixColumnOption[]>(() =>
    columns.value.map((column, columnIndex) => {
      const columnMeta = resolveMatrixColumnMeta(column, columnIndex);
      return { key: columnMeta.columnKey, label: columnMeta.columnLabel };
    }),
  );

  const allColumnKeys = computed(() =>
    allColumnOptions.value.map((column) => column.key),
  );

  function persistVisibleKeys(nextKeys: string[]): void {
    const normalized = sanitizeVisibleColumnKeys(
      nextKeys,
      allColumnKeys.value,
      allColumnKeys.value,
    );
    visibleColumnKeys.value = normalized;
    const matchesDefault =
      normalized.length === allColumnKeys.value.length &&
      normalized.every((key, index) => key === allColumnKeys.value[index]);
    if (matchesDefault) {
      removeStorageKey(storageKey);
      return;
    }
    writeStorageJson(storageKey, {
      visibleColumnKeys: normalized,
      knownColumnKeys: allColumnKeys.value,
    } satisfies PersistedMatrixColumnStateShape);
  }

  watch(
    allColumnKeys,
    (nextColumnKeys) => {
      const persisted = readPersistedColumnState(storageKey);
      visibleColumnKeys.value = sanitizeVisibleColumnKeys(
        visibleColumnKeys.value.length
          ? visibleColumnKeys.value
          : persisted?.visibleColumnKeys,
        persisted?.knownColumnKeys,
        nextColumnKeys,
      );
    },
    { immediate: true },
  );

  const visibleColumnKeySet = computed(() => new Set(visibleColumnKeys.value));
  const columnVisibility = computed<Record<string, boolean>>(() => {
    const hiddenEntries = allColumnKeys.value
      .filter((key) => !visibleColumnKeySet.value.has(key))
      .map((key) => [key, false] as const);
    return Object.fromEntries(hiddenEntries);
  });

  const columnOptions = computed(() =>
    allColumnOptions.value.map((column) => ({
      ...column,
      visible: visibleColumnKeySet.value.has(column.key),
    })),
  );

  const visibleCount = computed(() => visibleColumnKeys.value.length);
  const hiddenCount = computed(() =>
    Math.max(0, allColumnKeys.value.length - visibleColumnKeys.value.length),
  );
  const canRestoreDefaults = computed(
    () =>
      hiddenCount.value > 0 ||
      visibleColumnKeys.value.some(
        (key, index) => key !== allColumnKeys.value[index],
      ),
  );
  const leadVisibleColumnKey = computed(() => visibleColumnKeys.value[0] ?? "");

  function setColumnVisibility(columnKey: string, visible: boolean): void {
    const normalizedKey = String(columnKey || "").trim();
    if (!normalizedKey || !allColumnKeys.value.includes(normalizedKey)) return;
    const nextVisible = new Set(visibleColumnKeys.value);
    if (visible) nextVisible.add(normalizedKey);
    else nextVisible.delete(normalizedKey);
    persistVisibleKeys(Array.from(nextVisible));
  }

  function toggleColumnVisibility(columnKey: string): void {
    const normalizedKey = String(columnKey || "").trim();
    if (!normalizedKey) return;
    if (!visibleColumnKeySet.value.has(normalizedKey)) {
      setColumnVisibility(normalizedKey, true);
      return;
    }
    if (visibleCount.value <= 1) return;
    setColumnVisibility(normalizedKey, false);
  }

  function restoreDefaultColumns(): void {
    persistVisibleKeys(allColumnKeys.value);
  }

  function canHideColumn(columnKey: string): boolean {
    return visibleCount.value > 1 || !visibleColumnKeySet.value.has(columnKey);
  }

  return {
    columnOptions,
    columnVisibility,
    visibleColumnKeys,
    visibleCount,
    hiddenCount,
    canRestoreDefaults,
    leadVisibleColumnKey,
    toggleColumnVisibility,
    restoreDefaultColumns,
    canHideColumn,
  };
}
