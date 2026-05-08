import { computed, ref, watch, type Ref } from "vue";
import {
  readStorageJson,
  removeStorageKey,
  writeStorageJson,
} from "../utils/storage";
import {
  normalizeMatrixColumnWidthPx,
  resolveMatrixColumnMeta,
  type MatrixColumnLike,
} from "./useHeadlessMatrixTable";

interface PersistedMatrixColumnStateShape {
  visibleColumnKeys: string[];
  knownColumnKeys?: string[];
  columnOrderKeys?: string[];
  columnWidths?: Record<string, number>;
}

interface PersistedMatrixColumnOption {
  key: string;
  label: string;
  visible?: boolean;
}

export interface MatrixColumnViewPreset {
  visibleColumnKeys?: readonly string[];
  columnOrderKeys?: readonly string[];
}

type OrderedMatrixColumn<TColumn> = TColumn & {
  key: string;
  sourceColumnIndex: number;
};

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
  const knownSet = new Set(known);
  const visible = stored.length ? new Set(stored) : new Set(allColumnKeys);
  for (const key of allColumnKeys) {
    if (!knownSet.has(key)) visible.add(key);
  }
  if (!visible.size) visible.add(allColumnKeys[0]!);
  return allColumnKeys.filter((key) => visible.has(key));
}

function sanitizeColumnOrderKeys(
  candidateKeys: readonly string[] | null | undefined,
  allColumnKeys: readonly string[],
): string[] {
  if (!allColumnKeys.length) return [];
  const allowed = new Set(allColumnKeys);
  const ordered = dedupeKeys(candidateKeys ?? []).filter((key) => allowed.has(key));
  const orderedSet = new Set(ordered);
  const missing = allColumnKeys.filter((key) => !orderedSet.has(key));
  return [...ordered, ...missing];
}

function sanitizeColumnWidths(
  candidateWidths: Record<string, unknown> | null | undefined,
  allColumnKeys: readonly string[],
): Record<string, number> {
  if (!candidateWidths || !allColumnKeys.length) return {};
  const allowed = new Set(allColumnKeys);
  const out: Record<string, number> = {};
  for (const [rawKey, rawWidth] of Object.entries(candidateWidths)) {
    const key = String(rawKey || "").trim();
    if (!key || !allowed.has(key)) continue;
    const widthPx = normalizeMatrixColumnWidthPx(rawWidth);
    if (widthPx == null) continue;
    out[key] = widthPx;
  }
  return out;
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
    columnOrderKeys: Array.isArray(persisted.columnOrderKeys)
      ? persisted.columnOrderKeys.map((value) => String(value || "").trim())
      : [],
    columnWidths:
      persisted.columnWidths && typeof persisted.columnWidths === "object"
        ? persisted.columnWidths
        : {},
  };
}

export function usePersistedMatrixColumnState<TColumn extends MatrixColumnLike>(
  columns: Ref<TColumn[]>,
  storageKey: string,
) {
  const visibleColumnKeys = ref<string[]>([]);
  const columnOrderKeys = ref<string[]>([]);
  const columnWidths = ref<Record<string, number>>({});

  const allColumnOptions = computed<PersistedMatrixColumnOption[]>(() =>
    columns.value.map((column, columnIndex) => {
      const columnMeta = resolveMatrixColumnMeta(column, columnIndex);
      return { key: columnMeta.columnKey, label: columnMeta.columnLabel };
    }),
  );

  const allColumnKeys = computed(() =>
    allColumnOptions.value.map((column) => column.key),
  );

  function persistState(
    nextVisibleKeys: string[],
    nextColumnOrderKeys: string[],
    nextColumnWidths: Record<string, unknown> = columnWidths.value,
  ): void {
    const normalized = sanitizeVisibleColumnKeys(
      nextVisibleKeys,
      allColumnKeys.value,
      allColumnKeys.value,
    );
    const normalizedOrder = sanitizeColumnOrderKeys(nextColumnOrderKeys, allColumnKeys.value);
    const normalizedWidths = sanitizeColumnWidths(
      nextColumnWidths,
      allColumnKeys.value,
    );
    visibleColumnKeys.value = normalized;
    columnOrderKeys.value = normalizedOrder;
    columnWidths.value = normalizedWidths;
    const matchesDefault =
      normalized.length === allColumnKeys.value.length &&
      normalized.every((key, index) => key === allColumnKeys.value[index]) &&
      normalizedOrder.every((key, index) => key === allColumnKeys.value[index]) &&
      Object.keys(normalizedWidths).length === 0;
    if (matchesDefault) {
      removeStorageKey(storageKey);
      return;
    }
    const nextState: PersistedMatrixColumnStateShape = {
      visibleColumnKeys: normalized,
      knownColumnKeys: allColumnKeys.value,
      columnOrderKeys: normalizedOrder,
    };
    if (Object.keys(normalizedWidths).length > 0) {
      nextState.columnWidths = normalizedWidths;
    }
    writeStorageJson(storageKey, nextState);
  }

  function persistVisibleKeys(nextKeys: string[]): void {
    persistState(nextKeys, columnOrderKeys.value);
  }

  function persistColumnOrderKeys(nextKeys: string[]): void {
    persistState(visibleColumnKeys.value, nextKeys);
  }

  function persistColumnWidths(nextWidths: Record<string, unknown>): void {
    persistState(visibleColumnKeys.value, columnOrderKeys.value, nextWidths);
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
      columnOrderKeys.value = sanitizeColumnOrderKeys(
        columnOrderKeys.value.length
          ? columnOrderKeys.value
          : persisted?.columnOrderKeys,
        nextColumnKeys,
      );
      columnWidths.value = sanitizeColumnWidths(
        Object.keys(columnWidths.value).length
          ? columnWidths.value
          : persisted?.columnWidths,
        nextColumnKeys,
      );
    },
    { immediate: true },
  );

  const visibleColumnKeySet = computed(() => new Set(visibleColumnKeys.value));
  const columnVisibility = computed<Record<string, boolean>>(() => {
    const visibility: Record<string, boolean> = {};
    for (const key of allColumnKeys.value) {
      if (!visibleColumnKeySet.value.has(key)) visibility[key] = false;
    }
    return visibility;
  });

  const orderedColumnOptions = computed(() => {
    const byKey = new Map(allColumnOptions.value.map((column) => [column.key, column]));
    return columnOrderKeys.value
      .map((key) => byKey.get(key))
      .filter((column): column is PersistedMatrixColumnOption => !!column);
  });
  const columnOptions = computed(() =>
    orderedColumnOptions.value.map((column) => ({
      ...column,
      visible: visibleColumnKeySet.value.has(column.key),
    })),
  );
  const orderedVisibleColumnKeys = computed(() =>
    columnOrderKeys.value.filter((key) => visibleColumnKeySet.value.has(key)),
  );
  const sourceColumnIndexByKey = computed(
    () => new Map(allColumnKeys.value.map((key, index) => [key, index] as const)),
  );
  const orderedColumns = computed<OrderedMatrixColumn<TColumn>[]>(() => {
    const nextColumns: OrderedMatrixColumn<TColumn>[] = [];
    for (const key of columnOrderKeys.value) {
      const sourceColumnIndex = sourceColumnIndexByKey.value.get(key);
      if (sourceColumnIndex == null) continue;
      const column = columns.value[sourceColumnIndex];
      if (!column) continue;
      nextColumns.push({
        ...column,
        key,
        sourceColumnIndex,
      });
    }
    return nextColumns;
  });

  const visibleCount = computed(() => visibleColumnKeys.value.length);
  const hiddenCount = computed(() =>
    Math.max(0, allColumnKeys.value.length - visibleColumnKeys.value.length),
  );
  const canRestoreDefaults = computed(
    () =>
      hiddenCount.value > 0 ||
      Object.keys(columnWidths.value).length > 0 ||
      visibleColumnKeys.value.some(
        (key, index) => key !== allColumnKeys.value[index],
      ) ||
      columnOrderKeys.value.some(
        (key, index) => key !== allColumnKeys.value[index],
      ),
  );
  const leadVisibleColumnKey = computed(() => orderedVisibleColumnKeys.value[0] ?? "");

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
    persistState(allColumnKeys.value, allColumnKeys.value, {});
  }

  function setColumnOrder(nextColumnKeys: readonly string[]): void {
    persistColumnOrderKeys([...nextColumnKeys]);
  }

  function applyColumnViewPreset(preset: MatrixColumnViewPreset): void {
    persistState(
      [...(preset.visibleColumnKeys ?? allColumnKeys.value)],
      [...(preset.columnOrderKeys ?? allColumnKeys.value)],
    );
  }

  function setColumnWidth(
    columnKey: string,
    widthPx: number | null | undefined,
  ): void {
    const normalizedKey = String(columnKey || "").trim();
    if (!normalizedKey || !allColumnKeys.value.includes(normalizedKey)) return;
    const nextWidths = { ...columnWidths.value };
    const normalizedWidth = normalizeMatrixColumnWidthPx(widthPx);
    if (normalizedWidth == null) delete nextWidths[normalizedKey];
    else nextWidths[normalizedKey] = normalizedWidth;
    persistColumnWidths(nextWidths);
  }

  function resizeColumnWidth(columnKey: string, deltaPx: number): void {
    const normalizedKey = String(columnKey || "").trim();
    if (!normalizedKey || !allColumnKeys.value.includes(normalizedKey)) return;
    const currentWidth = columnWidths.value[normalizedKey] ?? 128;
    setColumnWidth(normalizedKey, currentWidth + Number(deltaPx || 0));
  }

  function clearColumnWidth(columnKey: string): void {
    setColumnWidth(columnKey, null);
  }

  function columnWidthStyle(columnKey: string): Record<string, string> {
    const normalizedKey = String(columnKey || "").trim();
    const widthPx = normalizedKey ? columnWidths.value[normalizedKey] : null;
    if (!widthPx) return {};
    return {
      width: `${widthPx}px`,
      minWidth: `${widthPx}px`,
      maxWidth: `${widthPx}px`,
    };
  }

  function canHideColumn(columnKey: string): boolean {
    return visibleCount.value > 1 || !visibleColumnKeySet.value.has(columnKey);
  }

  return {
    columnOptions,
    columnVisibility,
    allColumnKeys,
    orderedColumns,
    columnOrderKeys,
    columnWidths,
    visibleColumnKeys,
    orderedVisibleColumnKeys,
    visibleCount,
    hiddenCount,
    canRestoreDefaults,
    leadVisibleColumnKey,
    setColumnOrder,
    setColumnWidth,
    resizeColumnWidth,
    clearColumnWidth,
    columnWidthStyle,
    applyColumnViewPreset,
    toggleColumnVisibility,
    restoreDefaultColumns,
    canHideColumn,
  };
}
