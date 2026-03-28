import { computed, type Ref } from "vue";
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
}

export interface MatrixCellLike {
  text: string;
  className: string;
  title: string;
}

export interface MatrixRowLike {
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

function fallbackCell(): MatrixCellPayload {
  return { text: "", className: "", title: "" };
}

export function useHeadlessMatrixTable<
  TColumn extends MatrixColumnLike,
  TRow extends MatrixRowLike,
>(columns: Ref<TColumn[]>, rows: Ref<TRow[]>) {
  const columnDefs = computed<ColumnDef<TRow, MatrixCellPayload>[]>(() =>
    columns.value.map((column, columnIndex) => {
      const columnKey =
        String(column.key || "").trim() || `col_${columnIndex}`;
      const columnLabel =
        String(column.label || "").trim() || `Column ${columnIndex + 1}`;
      const sort = column.sort === "asc" || column.sort === "desc" ? column.sort : "";

      return {
        id: columnKey,
        header: columnLabel,
        accessorFn: (row) => row.cells[columnIndex] ?? fallbackCell(),
        meta: {
          alignLeft: !!column.alignLeft,
          columnKey,
          columnLabel,
          sort,
        } satisfies MatrixColumnMeta,
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
    getCoreRowModel: getCoreRowModel(),
    manualSorting: true,
    getRowId: (_row, index) => `matrix_row_${index}`,
  });

  const headerGroups = computed(() => table.getHeaderGroups());
  const rowModelRows = computed(() => table.getRowModel().rows);

  return {
    table,
    headerGroups,
    rowModelRows,
  };
}
