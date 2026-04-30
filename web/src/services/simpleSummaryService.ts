import { invokeCommand } from "../tauri/invoke";
import { parseBoundaryObject, parseOptionalBoundaryObject } from "./serviceBoundarySchemas";

function normalizeText(value: unknown): string {
  return String(value ?? "").trim();
}

function normalizePositiveInteger(value: unknown): number {
  const next = Math.floor(Number(value));
  return Number.isFinite(next) && next > 0 ? next : 0;
}

function sanitizeDefaultSummaryName(value: unknown): string {
  const raw = normalizeText(value) || "vectorgrid-simple-summary.md";
  const safe = raw
    .replace(/[^a-z0-9._-]+/gi, "_")
    .replace(/^_+|_+$/g, "")
    .slice(0, 96);
  const next = safe || "vectorgrid-simple-summary.md";
  return /\.(md|markdown)$/i.test(next) ? next : `${next}.md`;
}

export interface SimpleSummarySaveResult {
  path: string;
  bytes: number;
}

export function buildSimpleSummaryDefaultName(datasetLabel: unknown): string {
  const label = normalizeText(datasetLabel) || "run";
  return sanitizeDefaultSummaryName(`vectorgrid-simple-summary-${label}`);
}

export async function saveSimpleSummaryMarkdown(
  markdown: string,
  defaultName?: string,
): Promise<SimpleSummarySaveResult | null> {
  const content = String(markdown ?? "").trim();
  if (!content) throw new Error("Simple summary is empty.");
  const raw = await invokeCommand("save_simple_summary_markdown", {
    markdown: content,
    defaultName: sanitizeDefaultSummaryName(defaultName),
  });
  const row = parseOptionalBoundaryObject(raw);
  if (!row) return null;
  const typed = parseBoundaryObject(row);
  return {
    path: normalizeText(typed.path),
    bytes: normalizePositiveInteger(typed.bytes),
  };
}
