import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const sourceRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

const workbenchShell = fs.readFileSync(
  path.join(sourceRoot, "components", "workbench", "WorkbenchShell.vue"),
  "utf8",
);
const executionWorkspace = fs.readFileSync(
  path.join(sourceRoot, "components", "ExecutionWorkspacePanel.vue"),
  "utf8",
);
const priceChartPanel = fs.readFileSync(
  path.join(sourceRoot, "components", "visualizations", "PriceChartPanel.vue"),
  "utf8",
);

function expectRuleToContain(
  source: string,
  selector: string,
  declarations: string[],
): void {
  const marker = `${selector} {`;
  const lineMarker = `\n${marker}`;
  const rules: string[] = [];
  let searchFrom = 0;
  while (searchFrom < source.length) {
    const lineStart = source.indexOf(lineMarker, searchFrom);
    const ruleStart =
      lineStart >= 0
        ? lineStart + 1
        : searchFrom === 0 && source.startsWith(marker)
          ? 0
          : -1;
    if (ruleStart < 0) break;
    const ruleEnd = source.indexOf("}", ruleStart + marker.length);
    expect(ruleEnd, `Missing closing brace for ${selector}`).toBeGreaterThan(
      ruleStart,
    );
    rules.push(source.slice(ruleStart, ruleEnd + 1));
    searchFrom = ruleEnd + 1;
  }

  expect(rules.length, `Missing CSS rule ${selector}`).toBeGreaterThan(0);
  expect(
    rules.some((rule) =>
      declarations.every((declaration) => rule.includes(declaration)),
    ),
    `Expected one ${selector} rule to contain ${declarations.join(", ")}`,
  ).toBe(true);
}

describe("execution chart width contracts", () => {
  it("keeps the live workspace and teleported execution mount shrinkable", () => {
    expectRuleToContain(workbenchShell, "#liveShellGrid", [
      "grid-template-columns: minmax(0, 1fr)",
    ]);
    expectRuleToContain(workbenchShell, "#executionShellPane", [
      "min-width: 0",
      "max-width: 100%",
    ]);
    expectRuleToContain(workbenchShell, "#executionVueShellMount", [
      "min-width: 0",
      "max-width: 100%",
    ]);
  });

  it("prevents execution grid items from inheriting chart canvas intrinsic width", () => {
    expectRuleToContain(executionWorkspace, ".execPanelVueRoot", [
      "grid-template-columns: minmax(0, 1fr)",
      "min-width: 0",
      "max-width: 100%",
    ]);
    expectRuleToContain(executionWorkspace, ".pane", ["min-width: 0"]);
    expectRuleToContain(executionWorkspace, ".executionSubsection", [
      "grid-template-columns: minmax(0, 1fr)",
      "min-width: 0",
    ]);
    expectRuleToContain(executionWorkspace, ".chartBlock", [
      "min-width: 0",
      "max-width: 100%",
    ]);
    expectRuleToContain(executionWorkspace, ".statusGrid", [
      "grid-template-columns: repeat(2, minmax(0, 1fr))",
      "min-width: 0",
    ]);
    expectRuleToContain(executionWorkspace, ".statusGrid > div", [
      "min-width: 0",
    ]);
  });

  it("keeps the reusable price chart frame within its parent track", () => {
    expectRuleToContain(priceChartPanel, ".priceChartPanel", [
      "grid-template-columns: minmax(0, 1fr)",
      "min-width: 0",
      "max-width: 100%",
    ]);
    expectRuleToContain(priceChartPanel, ".tvChartFrame,\n.tvChartHost", [
      "min-width: 0",
      "max-width: 100%",
    ]);
  });
});
