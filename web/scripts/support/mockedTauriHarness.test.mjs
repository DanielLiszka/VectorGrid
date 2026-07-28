import assert from "node:assert/strict";
import test from "node:test";

import { installMockedTauri } from "./mockedTauriHarness.mjs";

function memoryStorage() {
  const values = new Map();
  return {
    getItem: (key) => values.get(String(key)) ?? null,
    removeItem: (key) => values.delete(String(key)),
    setItem: (key, value) => values.set(String(key), String(value)),
  };
}

test("argument-matched mocks return only the exact configured path and retain evidence", async () => {
  let initScript = null;
  let initArgument = null;
  const context = {
    async addInitScript(script, argument) {
      initScript = script;
      initArgument = argument;
    },
  };
  await installMockedTauri(context, {
    commandResults: {
      load_run_artifact_detail: {
        __mockByArgument: {
          name: "path",
          values: {
            "C:/runs/one.json": { paramsText: "READY::one" },
            "C:/runs/two.json": { paramsText: "READY::two" },
          },
        },
      },
    },
    commandDelays: {
      load_run_artifact_detail: 1,
    },
  });

  assert.equal(typeof initScript, "function");
  const priorWindow = globalThis.window;
  const priorSessionStorage = globalThis.sessionStorage;
  const priorLocalStorage = globalThis.localStorage;
  try {
    globalThis.window = {};
    globalThis.sessionStorage = memoryStorage();
    globalThis.localStorage = memoryStorage();
    initScript(initArgument);

    const result = await globalThis.window.__TAURI_INTERNALS__.invoke(
      "load_run_artifact_detail",
      { path: "C:/runs/two.json" },
    );
    assert.deepEqual(result, { paramsText: "READY::two" });
    const [entry] = globalThis.window.__VTA_TEST_INVOKE_LOG;
    assert.equal(entry.command, "load_run_artifact_detail");
    assert.equal(entry.args.path, "C:/runs/two.json");
    assert.equal(entry.matchedArgumentName, "path");
    assert.equal(entry.matchedArgumentValue, "C:/runs/two.json");
    assert.equal(entry.delayedMs, 1);
    assert.ok(entry.durationMs > 0);

    await assert.rejects(
      globalThis.window.__TAURI_INTERNALS__.invoke("load_run_artifact_detail", {
        path: "C:/runs/not-configured.json",
      }),
      /has no argument-matched response/,
    );
  } finally {
    if (priorWindow === undefined) delete globalThis.window;
    else globalThis.window = priorWindow;
    if (priorSessionStorage === undefined) delete globalThis.sessionStorage;
    else globalThis.sessionStorage = priorSessionStorage;
    if (priorLocalStorage === undefined) delete globalThis.localStorage;
    else globalThis.localStorage = priorLocalStorage;
  }
});
