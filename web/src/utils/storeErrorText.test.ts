import { describe, expect, it } from "vitest";
import { TauriRuntimeError } from "../tauri/runtimeError";
import { storeErrorText } from "./storeErrorText";

describe("storeErrorText", () => {
  it("keeps ordinary Error prefixes for existing store status behavior", () => {
    expect(storeErrorText(new Error("backend failed"))).toBe(
      "Error: backend failed",
    );
  });

  it("strips the TauriRuntimeError class prefix from user-facing status copy", () => {
    const error = new TauriRuntimeError({
      code: "TAURI_INVOKE_FAILED",
      operation: "polygon_get_cache_info",
      message:
        "Desktop runtime unavailable for polygon get cache info. Open this action in the Tauri desktop app.",
    });

    expect(storeErrorText(error)).toBe(
      "Desktop runtime unavailable for polygon get cache info. Open this action in the Tauri desktop app.",
    );
  });
});

