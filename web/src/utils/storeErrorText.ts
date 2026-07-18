export function storeErrorText(error: unknown): string {
  if (error && typeof error === "object") {
    const row = error as { message?: unknown; name?: unknown };
    if (
      row.name === "TauriRuntimeError" &&
      typeof row.message === "string"
    ) {
      return row.message.trim();
    }
  }
  return String(error || "").trim();
}

