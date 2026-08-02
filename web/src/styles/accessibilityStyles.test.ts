import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const stylesDirectory = path.dirname(fileURLToPath(import.meta.url));
const sourceRoot = path.resolve(stylesDirectory, "..");

const appStyles = fs.readFileSync(
  path.join(stylesDirectory, "app.css"),
  "utf8",
);
const runContextBar = fs.readFileSync(
  path.join(sourceRoot, "components", "RunContextBar.vue"),
  "utf8",
);
const historyWorkspace = fs.readFileSync(
  path.join(sourceRoot, "components", "HistoryWorkspacePanel.vue"),
  "utf8",
);

function extractMediaBlock(source: string, condition: string): string {
  const marker = `@media (${condition})`;
  const mediaStart = source.indexOf(marker);
  if (mediaStart < 0) {
    throw new Error(`Missing ${marker}`);
  }

  const blockStart = source.indexOf("{", mediaStart);
  if (blockStart < 0) {
    throw new Error(`Missing opening brace for ${marker}`);
  }

  let depth = 0;
  for (let index = blockStart; index < source.length; index += 1) {
    if (source[index] === "{") {
      depth += 1;
    } else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) {
        return source.slice(blockStart + 1, index);
      }
    }
  }

  throw new Error(`Missing closing brace for ${marker}`);
}

function expectBlockToContain(block: string, fragments: string[]): void {
  for (const fragment of fragments) {
    expect(block, `Expected media block to contain ${fragment}`).toContain(
      fragment,
    );
  }
}

describe("accessibility style contracts", () => {
  it("removes non-essential motion from global application interactions", () => {
    const block = extractMediaBlock(
      appStyles,
      "prefers-reduced-motion: reduce",
    );

    expectBlockToContain(block, [
      "button,",
      ".detailsCard summary::after",
      ".toast",
      "button:active:not(:disabled)",
      ".progressFill.indeterminate",
      "html.ui-transitions .tabPanel.active",
      ":root.ui-transitions::view-transition-old(root)",
      ":root.ui-transitions::view-transition-new(root)",
      "transition: none !important",
      "animation: none !important",
    ]);
  });

  it("removes non-essential motion from context and history controls", () => {
    const contextBlock = extractMediaBlock(
      runContextBar,
      "prefers-reduced-motion: reduce",
    );
    const historyBlock = extractMediaBlock(
      historyWorkspace,
      "prefers-reduced-motion: reduce",
    );

    expectBlockToContain(contextBlock, [
      ".contextCard",
      ".contextOverflowIcon",
      "transition: none !important",
    ]);
    expectBlockToContain(historyBlock, [
      ".runListButton",
      ".runListButton:hover:not(:disabled)",
      "transition: none !important",
      "transform: none",
    ]);
  });

  it("uses system colors and explicit focus treatment in forced-colors mode", () => {
    const block = extractMediaBlock(appStyles, "forced-colors: active");

    expectBlockToContain(block, [
      "Canvas",
      "CanvasText",
      "ButtonFace",
      "ButtonText",
      "Highlight",
      "HighlightText",
      "LinkText",
      "button:focus-visible",
      ".tabBtn.active",
      ".statusChip.ok",
      ".statusChip.danger",
      "tbody tr.selected",
    ]);
  });

  it("preserves context and history state cues in forced-colors mode", () => {
    const contextBlock = extractMediaBlock(
      runContextBar,
      "forced-colors: active",
    );
    const historyBlock = extractMediaBlock(
      historyWorkspace,
      "forced-colors: active",
    );

    expectBlockToContain(contextBlock, [
      ".contextCard.current",
      ".contextOverflowSummary:focus-visible",
      "HighlightText",
      "GrayText",
    ]);
    expectBlockToContain(historyBlock, [
      ".runListButton.active",
      ".runListButton:focus-visible",
      ".pill.good",
      ".pill.subtle",
      "HighlightText",
      "GrayText",
    ]);
  });
});
