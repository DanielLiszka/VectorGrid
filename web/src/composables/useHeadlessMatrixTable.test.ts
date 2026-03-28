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
});
