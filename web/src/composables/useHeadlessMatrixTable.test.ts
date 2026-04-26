import { computed, nextTick, ref } from "vue";
import { describe, expect, it } from "vitest";
import {
  useHeadlessMatrixTable,
  type MatrixColumnMeta,
} from "./useHeadlessMatrixTable";

describe("useHeadlessMatrixTable", () => {
  it("builds stable header and row models from matrix-style input", () => {
    const columns = ref([
      { key: "rank", label: "#", alignLeft: true, sort: "asc" as const },
      { key: "pnl", label: "PnL", alignLeft: false, sort: "" as const },
    ]);
    const rows = ref([
      {
        selected: true,
        cells: [
          { text: "1", className: "mono left", title: "" },
          { text: "12.5", className: "mono", title: "12.5000" },
        ],
      },
    ]);

    const { headerGroups, rowModelRows } = useHeadlessMatrixTable(
      computed(() => columns.value),
      computed(() => rows.value),
    );

    expect(headerGroups.value).toHaveLength(1);
    expect(headerGroups.value[0]?.headers).toHaveLength(2);
    expect(rowModelRows.value).toHaveLength(1);
    expect(rowModelRows.value[0]?.getVisibleCells()).toHaveLength(2);
    expect(rowModelRows.value[0]?.original.selected).toBe(true);
    expect(
      rowModelRows.value[0]?.getVisibleCells()[1]?.getValue(),
    ).toMatchObject({
      text: "12.5",
      className: "mono",
      title: "12.5000",
    });
  });

  it("applies external column visibility state to headers and visible cells", () => {
    const columns = ref([
      { key: "rank", label: "#", alignLeft: true, sort: "asc" as const },
      { key: "name", label: "Name", alignLeft: true, sort: "" as const },
      { key: "pnl", label: "PnL", alignLeft: false, sort: "" as const },
    ]);
    const rows = ref([
      {
        selected: false,
        cells: [
          { text: "1", className: "mono left", title: "" },
          { text: "candidate_a", className: "left", title: "candidate_a" },
          { text: "12.5", className: "mono", title: "12.5000" },
        ],
      },
    ]);
    const columnVisibility = ref<Record<string, boolean>>({
      pnl: false,
    });

    const { headerGroups, rowModelRows, visibleLeafColumns } = useHeadlessMatrixTable(
      computed(() => columns.value),
      computed(() => rows.value),
      { columnVisibility: computed(() => columnVisibility.value) },
    );

    expect(headerGroups.value).toHaveLength(1);
    expect(headerGroups.value[0]?.headers).toHaveLength(2);
    expect(visibleLeafColumns.value.map((column) => column.id)).toEqual([
      "rank",
      "name",
    ]);
    expect(rowModelRows.value[0]?.getVisibleCells()).toHaveLength(2);
    expect(
      rowModelRows.value[0]?.getVisibleCells().map((cell) => cell.column.id),
    ).toEqual(["rank", "name"]);
  });

  it("keeps source cell access stable when display columns are reordered", () => {
    const columns = ref([
      { key: "pnl", label: "PnL", alignLeft: false, sort: "" as const, sourceColumnIndex: 2 },
      { key: "rank", label: "#", alignLeft: true, sort: "asc" as const, sourceColumnIndex: 0 },
      { key: "name", label: "Name", alignLeft: true, sort: "" as const, sourceColumnIndex: 1 },
    ]);
    const rows = ref([
      {
        selected: false,
        cells: [
          { text: "1", className: "mono left", title: "" },
          { text: "candidate_a", className: "left", title: "candidate_a" },
          { text: "12.5", className: "mono", title: "12.5000" },
        ],
      },
    ]);

    const { rowModelRows, visibleLeafColumns } = useHeadlessMatrixTable(
      computed(() => columns.value),
      computed(() => rows.value),
    );

    expect(visibleLeafColumns.value.map((column) => column.id)).toEqual(["pnl", "rank", "name"]);
    expect(rowModelRows.value[0]?.getVisibleCells().map((cell) => (cell.getValue() as { text: string }).text)).toEqual([
      "12.5",
      "1",
      "candidate_a",
    ]);
  });

  it("supports client-side sortable matrix columns when enabled", async () => {
    const columns = ref([
      { key: "name", label: "Name", alignLeft: true, sort: "asc" as const },
      { key: "pnl", label: "PnL", alignLeft: false, sort: "" as const },
    ]);
    const rows = ref([
      {
        rowId: "b",
        cells: [
          { text: "Beta", className: "left", title: "Beta" },
          { text: "12.5", className: "mono", title: "12.5000" },
        ],
      },
      {
        rowId: "a",
        cells: [
          { text: "Alpha", className: "left", title: "Alpha" },
          { text: "3.0", className: "mono", title: "3.0000" },
        ],
      },
    ]);

    const { getHeaderSortState, rowModelRows, toggleSortColumn } =
      useHeadlessMatrixTable(
        computed(() => columns.value),
        computed(() => rows.value),
        { enableClientSorting: true },
      );

    expect(getHeaderSortState("name")).toMatchObject({
      sortable: true,
      sort: "asc",
      ariaSort: "ascending",
      nextDirection: "desc",
    });
    expect(rowModelRows.value.map((row) => row.original.rowId)).toEqual([
      "a",
      "b",
    ]);

    toggleSortColumn("pnl");
    await nextTick();

    expect(getHeaderSortState("pnl")).toMatchObject({
      sort: "asc",
      ariaSort: "ascending",
      nextDirection: "desc",
    });
    expect(rowModelRows.value.map((row) => row.original.rowId)).toEqual([
      "a",
      "b",
    ]);

    toggleSortColumn("pnl");
    await nextTick();

    expect(getHeaderSortState("pnl")).toMatchObject({
      sort: "desc",
      ariaSort: "descending",
      nextDirection: "asc",
    });
    expect(rowModelRows.value.map((row) => row.original.rowId)).toEqual([
      "b",
      "a",
    ]);
  });

  it("carries persisted column widths into table column metadata", () => {
    const columns = ref([
      { key: "rank", label: "#", alignLeft: true, sort: "asc" as const },
      { key: "pnl", label: "PnL", alignLeft: false, sort: "" as const },
    ]);
    const rows = ref([
      {
        cells: [
          { text: "1", className: "mono left", title: "" },
          { text: "12.5", className: "mono", title: "12.5000" },
        ],
      },
    ]);
    const columnWidths = ref<Record<string, number>>({ pnl: 184 });

    const { headerGroups, visibleLeafColumns } = useHeadlessMatrixTable(
      computed(() => columns.value),
      computed(() => rows.value),
      { columnWidths: computed(() => columnWidths.value) },
    );

    const pnlHeader = headerGroups.value[0]?.headers.find(
      (header) => header.id === "pnl",
    );
    const pnlMeta = pnlHeader?.column.columnDef.meta as MatrixColumnMeta;
    expect(pnlMeta.widthPx).toBe(184);
    expect(visibleLeafColumns.value.find((column) => column.id === "pnl")?.getSize()).toBe(184);
  });
});
