import { computed, watch, type Ref } from "vue";
import {
  getCoreRowModel,
  useVueTable,
  type CellContext,
  type ColumnDef,
} from "@tanstack/vue-table";

export interface MatrixColumnLike {
  label: string;
  alignLeft?: boolean;
  key?: string;
  sort?: "" | "asc" | "desc";
  sourceColumnIndex?: number;
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

export interface MatrixColumnMeta {
  alignLeft: boolean;
  columnKey: string;
  columnLabel: string;
  sort: "" | "asc" | "desc";
}

export interface ResolvedMatrixColumnMeta extends MatrixColumnMeta {
  columnIndex: number;
  sourceColumnIndex: number;
}

export interface HeadlessMatrixTableOptions {
  columnVisibility?: Ref<Record<string, boolean>>;
}

function fallbackCell(): MatrixCellPayload {
  return { text: "", className: "", title: "" };
}

export function resolveMatrixColumnMeta(
  column: MatrixColumnLike,
  columnIndex: number,
): ResolvedMatrixColumnMeta {
  const columnKey =
    String(column.key || "").trim() || `col_${columnIndex}`;
  const columnLabel =
    String(column.label || "").trim() || `Column ${columnIndex + 1}`;
  const sort =
    column.sort === "asc" || column.sort === "desc" ? column.sort : "";
  const sourceColumnIndex = Number.isInteger(column.sourceColumnIndex)
    ? Number(column.sourceColumnIndex)
    : columnIndex;

  return {
    alignLeft: !!column.alignLeft,
    columnKey,
    columnLabel,
    columnIndex,
    sourceColumnIndex,
    sort,
  };
}

export function useHeadlessMatrixTable<
  TColumn extends MatrixColumnLike,
  TRow extends MatrixRowLike,
>(
  columns: Ref<TColumn[]>,
  rows: Ref<TRow[]>,
  options?: HeadlessMatrixTableOptions,
) {
  const columnDefs = computed<ColumnDef<TRow, MatrixCellPayload>[]>(() =>
    columns.value.map((column, columnIndex) => {
      const columnMeta = resolveMatrixColumnMeta(column, columnIndex);

      return {
        id: columnMeta.columnKey,
        header: columnMeta.columnLabel,
        accessorFn: (row) => row.cells[columnMeta.sourceColumnIndex] ?? fallbackCell(),
        meta: columnMeta satisfies MatrixColumnMeta,
        cell: (context: CellContext<TRow, MatrixCellPayload>) =>
          context.getValue() ?? fallbackCell(),
      } satisfies ColumnDef<TRow, MatrixCellPayload>;
    }),
  );

  const table = useVueTable({
    get data() {
      return rows.value;
    },
    get columns() {
      return columnDefs.value;
    },
    get state() {
      return {
        columnVisibility: options?.columnVisibility?.value ?? {},
      };
    },
    getCoreRowModel: getCoreRowModel(),
    manualSorting: true,
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

  return {
    table,
    headerGroups,
    rowModelRows,
    visibleLeafColumns,
  };
}
