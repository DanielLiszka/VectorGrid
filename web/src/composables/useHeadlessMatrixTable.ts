import { computed, ref, watch, type Ref } from "vue";
import {
  getCoreRowModel,
  useVueTable,
  type CellContext,
  type ColumnDef,
} from "@tanstack/vue-table";

export type MatrixSortDirection = "" | "asc" | "desc";

export interface MatrixColumnLike {
  label: string;
  alignLeft?: boolean;
  key?: string;
  sort?: MatrixSortDirection;
  sourceColumnIndex?: number;
  widthPx?: number;
}

export interface MatrixCellLike {
  text: string;
  className: string;
  title: string;
}

export interface MatrixRowLike {
  rowId?: string;
  selected?: boolean;
  cells: MatrixCellLike[];
}

interface MatrixCellPayload {
  text: string;
  className: string;
  title: string;
}

interface SortableCellKey {
  value: string;
  numericValue: number;
  numeric: boolean;
  missing: boolean;
}

export interface MatrixColumnMeta {
  alignLeft: boolean;
  columnKey: string;
  columnLabel: string;
  sort: MatrixSortDirection;
  sortable: boolean;
  widthPx: number | null;
}

export interface ResolvedMatrixColumnMeta extends MatrixColumnMeta {
  columnIndex: number;
  sourceColumnIndex: number;
}

export interface HeadlessMatrixTableOptions {
  columnVisibility?: Ref<Record<string, boolean>>;
  columnWidths?: Ref<Record<string, number>>;
  enableClientSorting?: boolean;
  onSortColumn?: (
    columnKey: string,
    nextDirection: Exclude<MatrixSortDirection, "">,
    currentDirection: MatrixSortDirection,
  ) => void;
}

export interface MatrixSortState {
  columnKey: string;
  direction: Exclude<MatrixSortDirection, "">;
}

export interface MatrixHeaderSortState {
  columnKey: string;
  columnLabel: string;
  sortable: boolean;
  sort: MatrixSortDirection;
  ariaSort: "ascending" | "descending" | "none";
  nextDirection: Exclude<MatrixSortDirection, "">;
}

export const MIN_MATRIX_COLUMN_WIDTH_PX = 48;
export const MAX_MATRIX_COLUMN_WIDTH_PX = 960;
const EMPTY_MATRIX_CELL: MatrixCellPayload = Object.freeze({
  text: "",
  className: "",
  title: "",
}) as MatrixCellPayload;

function fallbackCell(): MatrixCellPayload {
  return EMPTY_MATRIX_CELL;
}

export function normalizeMatrixColumnWidthPx(value: unknown): number | null {
  const numeric = Number(value);
  if (!Number.isFinite(numeric) || numeric <= 0) return null;
  return Math.min(
    MAX_MATRIX_COLUMN_WIDTH_PX,
    Math.max(MIN_MATRIX_COLUMN_WIDTH_PX, Math.round(numeric)),
  );
}

function normalizeMatrixSortDirection(value: unknown): MatrixSortDirection {
  return value === "asc" || value === "desc" ? value : "";
}

function resolveColumnKey(
  column: MatrixColumnLike,
  columnIndex: number,
): string {
  return String(column.key || "").trim() || `col_${columnIndex}`;
}

export function resolveMatrixColumnMeta(
  column: MatrixColumnLike,
  columnIndex: number,
  options?: { sortable?: boolean; widthPx?: number | null },
): ResolvedMatrixColumnMeta {
  const columnKey = resolveColumnKey(column, columnIndex);
  const columnLabel =
    String(column.label || "").trim() || `Column ${columnIndex + 1}`;
  const sort = normalizeMatrixSortDirection(column.sort);
  const sourceColumnIndex = Number.isInteger(column.sourceColumnIndex)
    ? Number(column.sourceColumnIndex)
    : columnIndex;
  const widthPx = normalizeMatrixColumnWidthPx(
    options?.widthPx ?? column.widthPx,
  );

  return {
    alignLeft: !!column.alignLeft,
    columnKey,
    columnLabel,
    columnIndex,
    sourceColumnIndex,
    sort,
    sortable: !!options?.sortable,
    widthPx,
  };
}

function firstColumnSortState(
  columns: readonly MatrixColumnLike[],
): MatrixSortState | null {
  for (let columnIndex = 0; columnIndex < columns.length; columnIndex += 1) {
    const column = columns[columnIndex];
    if (!column) continue;
    const direction = normalizeMatrixSortDirection(column.sort);
    if (!direction) continue;
    return {
      columnKey: resolveColumnKey(column, columnIndex),
      direction,
    };
  }
  return null;
}

function sortableCellKey(cell: MatrixCellPayload): SortableCellKey {
  const value = String(cell.title || cell.text || "").trim();
  if (!value) {
    return {
      value,
      numericValue: Number.NaN,
      numeric: false,
      missing: true,
    };
  }
  const numericValue = Number(value.replace(/[$,%]/g, ""));
  return {
    value,
    numericValue,
    numeric: Number.isFinite(numericValue),
    missing: false,
  };
}

function compareSortableCellKeys(
  left: SortableCellKey,
  right: SortableCellKey,
): number {
  if (left.missing && right.missing) return 0;
  if (left.missing) return 1;
  if (right.missing) return -1;
  if (left.numeric && right.numeric) {
    return left.numericValue - right.numericValue;
  }

  return left.value.localeCompare(right.value, undefined, {
    numeric: true,
    sensitivity: "base",
  });
}

export function useHeadlessMatrixTable<
  TColumn extends MatrixColumnLike,
  TRow extends MatrixRowLike,
>(
  columns: Ref<TColumn[]>,
  rows: Ref<TRow[]>,
  options?: HeadlessMatrixTableOptions,
) {
  const sortable = computed(
    () => !!options?.enableClientSorting || !!options?.onSortColumn,
  );
  const localSortState = computed(() => firstColumnSortState(columns.value));
  const clientSortOverride = ref<MatrixSortState | null>(null);

  const activeSortState = computed(
    () => clientSortOverride.value ?? localSortState.value,
  );
  const columnMetaByKey = computed(() => {
    const out = new Map<string, ResolvedMatrixColumnMeta>();
    for (
      let columnIndex = 0;
      columnIndex < columns.value.length;
      columnIndex += 1
    ) {
      const column = columns.value[columnIndex];
      if (!column) continue;
      const meta = resolveMatrixColumnMeta(column, columnIndex);
      out.set(meta.columnKey, meta);
    }
    return out;
  });

  const columnDefs = computed<ColumnDef<TRow, MatrixCellPayload>[]>(() =>
    columns.value.map((column, columnIndex) => {
      const columnKey = resolveColumnKey(column, columnIndex);
      const overrideSort = clientSortOverride.value;
      const persistedWidthPx = normalizeMatrixColumnWidthPx(
        options?.columnWidths?.value?.[columnKey],
      );
      const metaOptions: { sortable?: boolean; widthPx?: number | null } = {
        sortable: sortable.value,
      };
      if (persistedWidthPx != null) metaOptions.widthPx = persistedWidthPx;
      const resolvedMeta = resolveMatrixColumnMeta(
        column,
        columnIndex,
        metaOptions,
      );
      const columnMeta = {
        ...resolvedMeta,
        sort: overrideSort
          ? overrideSort.columnKey === columnKey
            ? overrideSort.direction
            : ""
          : resolvedMeta.sort,
      } satisfies ResolvedMatrixColumnMeta;

      const columnDef: ColumnDef<TRow, MatrixCellPayload> = {
        id: columnMeta.columnKey,
        header: columnMeta.columnLabel,
        accessorFn: (row) =>
          row.cells[columnMeta.sourceColumnIndex] ?? fallbackCell(),
        meta: columnMeta satisfies MatrixColumnMeta,
        enableSorting: columnMeta.sortable,
        minSize: MIN_MATRIX_COLUMN_WIDTH_PX,
        maxSize: MAX_MATRIX_COLUMN_WIDTH_PX,
        cell: (context: CellContext<TRow, MatrixCellPayload>) =>
          context.getValue() ?? fallbackCell(),
      };
      if (columnMeta.widthPx != null) columnDef.size = columnMeta.widthPx;
      return columnDef;
    }),
  );

  const sortedRows = computed(() => {
    if (!options?.enableClientSorting) return rows.value;
    const sortState = activeSortState.value;
    if (!sortState) return rows.value;
    const columnMeta = columnMetaByKey.value.get(sortState.columnKey);
    if (!columnMeta) return rows.value;
    const directionMultiplier = sortState.direction === "asc" ? 1 : -1;
    return rows.value
      .map((row, sourceIndex) => ({
        row,
        sourceIndex,
        sortKey: sortableCellKey(
          row.cells[columnMeta.sourceColumnIndex] ?? fallbackCell(),
        ),
      }))
      .sort((left, right) => {
        const comparedBase = compareSortableCellKeys(
          left.sortKey,
          right.sortKey,
        );
        const compared =
          left.sortKey.missing || right.sortKey.missing
            ? comparedBase
            : comparedBase * directionMultiplier;
        return compared || left.sourceIndex - right.sourceIndex;
      })
      .map((entry) => entry.row);
  });

  const table = useVueTable({
    get data() {
      return sortedRows.value;
    },
    get columns() {
      return columnDefs.value;
    },
    get state() {
      return {
        columnVisibility: options?.columnVisibility?.value ?? {},
        columnSizing: options?.columnWidths?.value ?? {},
      };
    },
    getCoreRowModel: getCoreRowModel(),
    manualSorting: true,
    enableColumnResizing: !!options?.columnWidths,
    columnResizeMode: "onChange",
    getRowId: (row, index) => {
      const explicit = String(row.rowId || "").trim();
      return explicit || `matrix_row_${index}`;
    },
  });

  const headerGroups = computed(() => table.getHeaderGroups());
  const rowModelRows = computed(() => table.getRowModel().rows);
  const visibleLeafColumns = computed(() => table.getVisibleLeafColumns());

  if (options?.columnVisibility) {
    watch(
      options.columnVisibility,
      (nextVisibility) => {
        table.setColumnVisibility(nextVisibility);
      },
      { immediate: true, deep: true },
    );
  }

  function columnExists(columnKey: string): boolean {
    return columnMetaByKey.value.has(columnKey);
  }

  function currentSortDirectionFor(columnKey: string): MatrixSortDirection {
    const overrideSort = clientSortOverride.value;
    if (overrideSort) {
      return overrideSort.columnKey === columnKey ? overrideSort.direction : "";
    }
    const columnMeta = columnMetaByKey.value.get(columnKey);
    return columnMeta?.sort ?? "";
  }

  function nextSortDirectionFor(
    columnKey: string,
  ): Exclude<MatrixSortDirection, ""> {
    const currentDirection = currentSortDirectionFor(columnKey);
    if (currentDirection === "asc") return "desc";
    return "asc";
  }

  function setSortColumn(
    columnKey: string,
    direction: Exclude<MatrixSortDirection, ""> = "asc",
  ): void {
    const normalizedKey = String(columnKey || "").trim();
    if (!normalizedKey || !columnExists(normalizedKey)) return;
    const nextDirection = direction === "desc" ? "desc" : "asc";
    const currentDirection = currentSortDirectionFor(normalizedKey);
    if (options?.enableClientSorting) {
      clientSortOverride.value = {
        columnKey: normalizedKey,
        direction: nextDirection,
      };
    }
    options?.onSortColumn?.(normalizedKey, nextDirection, currentDirection);
  }

  function toggleSortColumn(columnKey: string): void {
    const normalizedKey = String(columnKey || "").trim();
    if (!normalizedKey) return;
    setSortColumn(normalizedKey, nextSortDirectionFor(normalizedKey));
  }

  function getHeaderSortState(columnKey: string): MatrixHeaderSortState {
    const normalizedKey = String(columnKey || "").trim();
    const columnMeta = columnMetaByKey.value.get(normalizedKey);
    const active = currentSortDirectionFor(normalizedKey);
    return {
      columnKey: normalizedKey,
      columnLabel: columnMeta?.columnLabel ?? normalizedKey,
      sortable: sortable.value && !!columnMeta,
      sort: active,
      ariaSort:
        active === "asc"
          ? "ascending"
          : active === "desc"
            ? "descending"
            : "none",
      nextDirection: nextSortDirectionFor(normalizedKey),
    };
  }

  return {
    table,
    headerGroups,
    rowModelRows,
    visibleLeafColumns,
    getHeaderSortState,
    setSortColumn,
    toggleSortColumn,
  };
}
