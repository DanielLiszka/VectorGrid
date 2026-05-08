import { invokeCommand } from "../tauri/invoke";

function normalizePath(path: unknown): string {
  return String(path ?? "").trim();
}

function errorMessage(error: unknown): string {
  return String(error instanceof Error ? error.message : error).trim();
}

export type OpenOrRevealPathResult = {
  path: string;
  openedWith: "open_path" | "reveal_path";
};

export async function openPathWithRevealFallback(
  path: unknown,
): Promise<OpenOrRevealPathResult> {
  const nextPath = normalizePath(path);
  if (!nextPath) {
    throw new Error("Path is required.");
  }
  try {
    await invokeCommand("open_path", { path: nextPath });
    return { path: nextPath, openedWith: "open_path" };
  } catch (openError) {
    try {
      await invokeCommand("reveal_path", { path: nextPath });
      return { path: nextPath, openedWith: "reveal_path" };
    } catch (revealError) {
      const openMessage = errorMessage(openError) || "unknown open error";
      const revealMessage = errorMessage(revealError) || "unknown reveal error";
      throw new Error(
        `Failed to open or reveal path. open_path failed: ${openMessage}; reveal_path failed: ${revealMessage}`,
      );
    }
  }
}
