import assert from "node:assert/strict";
import test from "node:test";
import {
  commandExists,
  parseProductVersion,
  selectDesktopEdgeRuntime,
  verifyDesktopSmokePrereqs,
  type EdgeRuntimeEvidence,
} from "./verify-prereqs.ts";

test("an explicit command path never falls back to PATH", () => {
  const command = process.platform === "win32" ? "node.exe" : "node";
  assert.equal(
    commandExists(command, "__vectorgrid_missing_explicit_command__"),
    null,
  );
  assert.ok(commandExists(command), `${command} should be available on PATH`);
});

const webView2Runtime: EdgeRuntimeEvidence = {
  kind: "webview2",
  executablePath:
    "C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\Application\\150.0.4078.99\\msedgewebview2.exe",
  version: "150.0.4078.99",
  major: 150,
};

function compatibleDependencies(
  overrides: Parameters<typeof verifyDesktopSmokePrereqs>[0] = {},
): Parameters<typeof verifyDesktopSmokePrereqs>[0] {
  return {
    platform: "win32",
    env: {},
    commandExists(command, explicitPath) {
      if (explicitPath?.includes("tauri-driver"))
        return "C:\\tools\\tauri-driver.exe";
      if (command === "msedgedriver.exe") return "C:\\repo\\msedgedriver.exe";
      if (command === "7z.exe") return "C:\\tools\\7z.exe";
      return null;
    },
    versionOf(executable) {
      if (executable.includes("msedgedriver")) {
        return "Microsoft Edge WebDriver 150.0.4078.99 (fixture)";
      }
      if (executable.includes("7z")) return "7-Zip 25.01 (fixture)";
      return "fixture-version";
    },
    detectEdgeRuntimes: () => [webView2Runtime],
    ...overrides,
  };
}

test("parseProductVersion extracts dotted product versions without accepting prose numbers", () => {
  assert.deepEqual(
    parseProductVersion("Microsoft Edge WebDriver 150.0.4078.99 (fixture)"),
    {
      version: "150.0.4078.99",
      components: [150, 0, 4078, 99],
      major: 150,
    },
  );
  assert.equal(
    parseProductVersion("Microsoft Edge WebDriver unavailable"),
    null,
  );
  assert.equal(parseProductVersion("build 150"), null);
});

test("selectDesktopEdgeRuntime prefers newest WebView2 over Edge browser fallback", () => {
  assert.deepEqual(
    selectDesktopEdgeRuntime([
      {
        kind: "edge",
        executablePath: "C:\\edge\\151\\msedge.exe",
        version: "151.0.1.0",
        major: 151,
      },
      {
        ...webView2Runtime,
        executablePath: "C:\\webview\\150.0.4078.83\\msedgewebview2.exe",
        version: "150.0.4078.83",
      },
      webView2Runtime,
    ]),
    webView2Runtime,
  );
});

test("explicit EdgeDriver override is authoritative and version evidence is reported", () => {
  const overridePath = "D:\\drivers\\150\\msedgedriver.exe";
  const calls: Array<{ command: string; explicitPath?: string }> = [];
  const result = verifyDesktopSmokePrereqs(
    compatibleDependencies({
      env: {
        VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER: overridePath,
        VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE: "1",
      },
      commandExists(command, explicitPath) {
        calls.push({ command, explicitPath });
        if (explicitPath?.includes("tauri-driver"))
          return "C:\\tools\\tauri-driver.exe";
        if (explicitPath === overridePath) return overridePath;
        if (command === "7z.exe") return "C:\\tools\\7z.exe";
        return null;
      },
    }),
  );

  assert.equal(result.edgeDriverPath, overridePath);
  assert.equal(result.edgeDriverVersion, "150.0.4078.99");
  assert.equal(result.archiveToolPath, "C:\\tools\\7z.exe");
  assert.match(result.summary, /webview2-runtime: 150\.0\.4078\.99/);
  assert.match(result.summary, /edge-major-compatible: true/);
  assert.match(result.summary, /archive-tool-version: 7-Zip 25\.01/);
  assert.equal(
    calls.some(
      ({ explicitPath }) => explicitPath === "C:\\repo\\msedgedriver.exe",
    ),
    false,
  );
});

test("invalid explicit EdgeDriver override fails closed without falling back", () => {
  assert.throws(
    () =>
      verifyDesktopSmokePrereqs(
        compatibleDependencies({
          env: {
            VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER:
              "D:\\missing\\msedgedriver.exe",
          },
          commandExists(command, explicitPath) {
            if (explicitPath?.includes("tauri-driver"))
              return "C:\\tools\\tauri-driver.exe";
            if (!explicitPath && command === "msedgedriver.exe")
              return "C:\\repo\\msedgedriver.exe";
            if (command === "7z.exe") return "C:\\tools\\7z.exe";
            return null;
          },
        }),
      ),
    /VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER does not resolve to a file/,
  );
});

test("EdgeDriver major mismatch fails before desktop launch", () => {
  assert.throws(
    () =>
      verifyDesktopSmokePrereqs(
        compatibleDependencies({
          versionOf(executable) {
            if (executable.includes("msedgedriver")) {
              return "Microsoft Edge WebDriver 148.0.3967.54";
            }
            return "fixture-version";
          },
        }),
      ),
    /EdgeDriver major 148 .* does not match .*WebView2 runtime.* major 150/,
  );
});

test("unverifiable installed WebView2 or Edge version fails closed on Windows", () => {
  assert.throws(
    () =>
      verifyDesktopSmokePrereqs(
        compatibleDependencies({
          detectEdgeRuntimes: () => [],
        }),
      ),
    /compatibility cannot be proven/,
  );
});

test("unexecutable EdgeDriver and archive tools do not count as available", () => {
  assert.throws(
    () =>
      verifyDesktopSmokePrereqs(
        compatibleDependencies({
          versionOf(executable) {
            if (executable.includes("msedgedriver")) {
              return "unavailable (process failed to execute)";
            }
            return "fixture-version";
          },
        }),
      ),
    /could not parse the EdgeDriver version/,
  );

  assert.throws(
    () =>
      verifyDesktopSmokePrereqs(
        compatibleDependencies({
          env: { VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE: "1" },
          versionOf(executable) {
            if (executable.includes("msedgedriver")) {
              return "Microsoft Edge WebDriver 150.0.4078.99";
            }
            if (executable.includes("7z")) {
              return "unavailable (process failed to execute)";
            }
            return "fixture-version";
          },
        }),
      ),
    /requires 7z or 7za/,
  );
});

test("exact-package smoke requires 7z or 7za while non-package smoke records absence", () => {
  const withoutArchive = compatibleDependencies({
    env: {},
    commandExists(command, explicitPath) {
      if (explicitPath?.includes("tauri-driver"))
        return "C:\\tools\\tauri-driver.exe";
      if (command === "msedgedriver.exe") return "C:\\repo\\msedgedriver.exe";
      return null;
    },
  });
  const nonPackage = verifyDesktopSmokePrereqs(withoutArchive);
  assert.equal(nonPackage.archiveToolPath, null);
  assert.match(nonPackage.summary, /archive-tool-path: unavailable/);

  assert.throws(
    () =>
      verifyDesktopSmokePrereqs({
        ...withoutArchive,
        env: { VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE: "1" },
      }),
    /requires 7z or 7za/,
  );
});
