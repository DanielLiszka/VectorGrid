import { computed, ref } from "vue";
import { describe, expect, it } from "vitest";
import { useHeadlessMatrixTable } from "./useHeadlessMatrixTable";

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
});
