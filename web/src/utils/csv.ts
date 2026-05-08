const FORMULA_PREFIX_PATTERN = /^\s*[=+\-@]|\t|\r/;

function shouldNeutralizeFormula(raw: string, original: unknown): boolean {
  if (typeof original === "number" && Number.isFinite(original)) return false;
  return FORMULA_PREFIX_PATTERN.test(raw);
}

export function escapeCsvCell(value: unknown): string {
  const raw = String(value ?? "");
  const safe = shouldNeutralizeFormula(raw, value) ? `'${raw}` : raw;
  return /[",\r\n]/.test(safe) ? `"${safe.replace(/"/g, '""')}"` : safe;
}

export function csvLine(values: readonly unknown[]): string {
  return values.map((value) => escapeCsvCell(value)).join(",");
}
