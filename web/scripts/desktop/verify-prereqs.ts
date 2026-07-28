import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(scriptDir, "..", "..");
const repoRoot = path.resolve(webRoot, "..");

export type EdgeRuntimeKind = "webview2" | "edge";

export type EdgeRuntimeEvidence = {
  kind: EdgeRuntimeKind;
  executablePath: string;
  version: string;
  major: number;
};

type VersionEvidence = {
  version: string;
  components: number[];
  major: number;
};

type DesktopPrereqDependencies = {
  platform: NodeJS.Platform;
  env: NodeJS.ProcessEnv;
  commandExists: (command: string, explicitPath?: string) => string | null;
  versionOf: (executable: string, args?: string[]) => string;
  detectEdgeRuntimes: () => EdgeRuntimeEvidence[];
};

export type DesktopSmokePrereqs = {
  tauriDriverPath: string;
  edgeDriverPath: string;
  edgeDriverVersion: string;
  edgeRuntime: EdgeRuntimeEvidence | null;
  archiveToolPath: string | null;
  archiveToolVersion: string | null;
  summary: string;
};

export function commandExists(
  command: string,
  explicitPath?: string,
): string | null {
  if (explicitPath) {
    try {
      return fs.statSync(explicitPath).isFile()
        ? path.resolve(explicitPath)
        : null;
    } catch {
      return null;
    }
  }
  if (process.platform === "win32") {
    const resolved = spawnSync("where.exe", [command], { encoding: "utf8" });
    if (resolved.status === 0) {
      const candidate = String(resolved.stdout || "")
        .split(/\r?\n/)
        .map((line) => line.trim())
        .find((line) => line.length > 0);
      if (candidate) return candidate;
    }
  } else {
    const resolved = spawnSync("which", [command], { encoding: "utf8" });
    if (resolved.status === 0) {
      const candidate = String(resolved.stdout || "").trim();
      if (candidate) return candidate;
    }
  }
  return null;
}

function versionOf(executable: string, args = ["--version"]): string {
  const result = spawnSync(executable, args, { encoding: "utf8" });
  if (result.status !== 0) {
    return `unavailable (${String(result.stderr || result.stdout || "").trim() || "failed to execute"})`;
  }
  return String(result.stdout || result.stderr || "").trim() || "unknown";
}

export function parseProductVersion(raw: string): VersionEvidence | null {
  const match = String(raw || "").match(
    /(?:^|[^\d])(\d+)\.(\d+)\.(\d+)(?:\.(\d+))?/,
  );
  if (!match) return null;
  const components = match
    .slice(1)
    .filter((value): value is string => value !== undefined)
    .map(Number);
  if (components.some((value) => !Number.isSafeInteger(value) || value < 0))
    return null;
  return {
    version: components.join("."),
    components,
    major: components[0],
  };
}

function compareVersions(left: string, right: string): number {
  const leftParts = parseProductVersion(left)?.components ?? [];
  const rightParts = parseProductVersion(right)?.components ?? [];
  const count = Math.max(leftParts.length, rightParts.length);
  for (let index = 0; index < count; index += 1) {
    const delta = (leftParts[index] ?? 0) - (rightParts[index] ?? 0);
    if (delta !== 0) return delta;
  }
  return 0;
}

export function selectDesktopEdgeRuntime(
  runtimes: EdgeRuntimeEvidence[],
): EdgeRuntimeEvidence | null {
  for (const kind of ["webview2", "edge"] as const) {
    const newest = runtimes
      .filter((runtime) => runtime.kind === kind)
      .sort((left, right) => compareVersions(right.version, left.version))[0];
    if (newest) return newest;
  }
  return null;
}

function installedVersionDirectories(
  root: string,
  kind: EdgeRuntimeKind,
  executableName: string,
): EdgeRuntimeEvidence[] {
  if (!root || !fs.existsSync(root)) return [];
  let entries: fs.Dirent[];
  try {
    entries = fs.readdirSync(root, { withFileTypes: true });
  } catch {
    return [];
  }
  return entries.flatMap((entry) => {
    if (!entry.isDirectory()) return [];
    const parsed = parseProductVersion(entry.name);
    if (!parsed) return [];
    const executablePath = path.resolve(root, entry.name, executableName);
    if (!fs.existsSync(executablePath)) return [];
    return [
      {
        kind,
        executablePath,
        version: parsed.version,
        major: parsed.major,
      },
    ];
  });
}

export function detectInstalledEdgeRuntimes(
  env: NodeJS.ProcessEnv = process.env,
  platform: NodeJS.Platform = process.platform,
): EdgeRuntimeEvidence[] {
  if (platform !== "win32") return [];
  const programFilesX86 = String(env["ProgramFiles(x86)"] || "").trim();
  const programFiles = String(env.ProgramFiles || "").trim();
  const localAppData = String(env.LOCALAPPDATA || "").trim();
  const roots = [
    {
      kind: "webview2" as const,
      executable: "msedgewebview2.exe",
      values: [
        programFilesX86 &&
          path.resolve(
            programFilesX86,
            "Microsoft",
            "EdgeWebView",
            "Application",
          ),
        programFiles &&
          path.resolve(programFiles, "Microsoft", "EdgeWebView", "Application"),
        localAppData &&
          path.resolve(localAppData, "Microsoft", "EdgeWebView", "Application"),
      ],
    },
    {
      kind: "edge" as const,
      executable: "msedge.exe",
      values: [
        programFilesX86 &&
          path.resolve(programFilesX86, "Microsoft", "Edge", "Application"),
        programFiles &&
          path.resolve(programFiles, "Microsoft", "Edge", "Application"),
        localAppData &&
          path.resolve(localAppData, "Microsoft", "Edge", "Application"),
      ],
    },
  ];
  const seen = new Set<string>();
  return roots.flatMap(({ kind, executable, values }) =>
    values.flatMap((root) => {
      if (!root) return [];
      const normalized = path.normalize(root).toLocaleLowerCase("en-US");
      if (seen.has(normalized)) return [];
      seen.add(normalized);
      return installedVersionDirectories(root, kind, executable);
    }),
  );
}

function resolveArchiveTool(
  deps: DesktopPrereqDependencies,
): { path: string; version: string } | null {
  const archiveVersion = (executable: string): string | null => {
    const raw = deps.versionOf(executable, []);
    const version =
      raw
        .split(/\r?\n/)
        .map((line) => line.trim())
        .find((line) => line.length > 0) ?? "unknown";
    return /^(?:unavailable\b|unknown$)/i.test(version) ? null : version;
  };
  const programFiles = String(deps.env.ProgramFiles || "").trim();
  const programFilesX86 = String(deps.env["ProgramFiles(x86)"] || "").trim();
  const fallbackPaths = [
    programFiles && path.resolve(programFiles, "7-Zip", "7z.exe"),
    programFilesX86 && path.resolve(programFilesX86, "7-Zip", "7z.exe"),
  ].filter(Boolean);
  for (const fallback of fallbackPaths) {
    const resolved = deps.commandExists("7z.exe", fallback);
    const version = resolved ? archiveVersion(resolved) : null;
    if (resolved && version) return { path: resolved, version };
  }
  for (const command of deps.platform === "win32"
    ? ["7z.exe", "7za.exe"]
    : ["7z", "7za"]) {
    const resolved = deps.commandExists(command);
    const version = resolved ? archiveVersion(resolved) : null;
    if (resolved && version) return { path: resolved, version };
  }
  return null;
}

export function verifyDesktopSmokePrereqs(
  injected: Partial<DesktopPrereqDependencies> = {},
): DesktopSmokePrereqs {
  const deps: DesktopPrereqDependencies = {
    platform: injected.platform ?? process.platform,
    env: injected.env ?? process.env,
    commandExists: injected.commandExists ?? commandExists,
    versionOf: injected.versionOf ?? versionOf,
    detectEdgeRuntimes:
      injected.detectEdgeRuntimes ??
      (() =>
        detectInstalledEdgeRuntimes(
          injected.env ?? process.env,
          injected.platform ?? process.platform,
        )),
  };
  const edgeDriverOverride = String(
    deps.env.VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER || "",
  ).trim();
  const tauriDriverFallback = path.resolve(
    os.homedir(),
    ".cargo",
    "bin",
    deps.platform === "win32" ? "tauri-driver.exe" : "tauri-driver",
  );
  const edgeDriverName =
    deps.platform === "win32" ? "msedgedriver.exe" : "msedgedriver";
  const tauriDriverPath = deps.commandExists(
    "tauri-driver",
    tauriDriverFallback,
  ) ?? deps.commandExists("tauri-driver");
  let edgeDriverPath: string | null = null;
  if (edgeDriverOverride) {
    edgeDriverPath = deps.commandExists(edgeDriverName, edgeDriverOverride);
    if (!edgeDriverPath) {
      throw new Error(
        `VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER does not resolve to a file: ${edgeDriverOverride}`,
      );
    }
  } else {
    const fallbacks = [
      path.resolve(repoRoot, edgeDriverName),
      path.resolve(webRoot, edgeDriverName),
    ];
    edgeDriverPath =
      fallbacks
        .map((candidate) => deps.commandExists(edgeDriverName, candidate))
        .find((candidate): candidate is string => Boolean(candidate)) ??
      deps.commandExists(edgeDriverName);
  }

  if (!tauriDriverPath) {
    throw new Error(
      'Desktop smoke requires tauri-driver. Install it with "cargo install tauri-driver --locked".',
    );
  }
  if (!edgeDriverPath) {
    throw new Error(
      "Desktop smoke requires msedgedriver.exe. Install a driver that matches the local WebView2 runtime and place it on PATH, in the repo root, or set VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER.",
    );
  }

  const edgeDriverVersionOutput = deps.versionOf(edgeDriverPath);
  const edgeDriverVersion = /^(?:unavailable\b|unknown$)/i.test(
    edgeDriverVersionOutput.trim(),
  )
    ? null
    : parseProductVersion(edgeDriverVersionOutput);
  if (deps.platform === "win32" && !edgeDriverVersion) {
    throw new Error(
      `Desktop smoke could not parse the EdgeDriver version from "${edgeDriverVersionOutput}".`,
    );
  }
  const runtimes = deps.detectEdgeRuntimes();
  const webView2Runtime = selectDesktopEdgeRuntime(
    runtimes.filter((runtime) => runtime.kind === "webview2"),
  );
  const edgeBrowser = selectDesktopEdgeRuntime(
    runtimes.filter((runtime) => runtime.kind === "edge"),
  );
  const edgeRuntime = selectDesktopEdgeRuntime(runtimes);
  if (deps.platform === "win32" && !edgeRuntime) {
    throw new Error(
      "Desktop smoke could not resolve an installed WebView2 runtime or Microsoft Edge version, so EdgeDriver compatibility cannot be proven.",
    );
  }
  if (
    deps.platform === "win32" &&
    edgeDriverVersion &&
    edgeRuntime &&
    edgeDriverVersion.major !== edgeRuntime.major
  ) {
    throw new Error(
      `EdgeDriver major ${String(edgeDriverVersion.major)} (${edgeDriverVersion.version}) does not match the installed ${edgeRuntime.kind === "webview2" ? "WebView2 runtime" : "Microsoft Edge fallback"} major ${String(edgeRuntime.major)} (${edgeRuntime.version}). Set VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER to a compatible driver.`,
    );
  }

  const archiveTool = resolveArchiveTool(deps);
  const exactPackage = deps.env.VECTORGRID_DESKTOP_SMOKE_EXACT_PACKAGE === "1";
  if (exactPackage && !archiveTool) {
    throw new Error(
      "Exact-package desktop smoke requires 7z or 7za to inspect the staged NSIS payload.",
    );
  }

  const summaryLines = [
    `repoRoot: ${repoRoot}`,
    `webRoot: ${webRoot}`,
    `platform: ${deps.platform}`,
    `node: ${process.version}`,
    `cargo: ${deps.versionOf("cargo")}`,
    `tauri-driver-path: ${tauriDriverPath}`,
    `tauri-driver-version: ${deps.versionOf(tauriDriverPath, ["--help"])}`,
    `msedgedriver-path: ${edgeDriverPath}`,
    `msedgedriver-version: ${edgeDriverVersion?.version ?? edgeDriverVersionOutput}`,
    `msedgedriver-override: ${edgeDriverOverride ? "configured" : "not-configured"}`,
    `webview2-runtime: ${webView2Runtime ? `${webView2Runtime.version} (${webView2Runtime.executablePath})` : "unavailable"}`,
    `edge-browser: ${edgeBrowser ? `${edgeBrowser.version} (${edgeBrowser.executablePath})` : "unavailable"}`,
    `compatibility-target: ${edgeRuntime ? `${edgeRuntime.kind} ${edgeRuntime.version}` : "not-applicable"}`,
    `edge-major-compatible: ${deps.platform === "win32" ? "true" : "not-applicable"}`,
    `archive-tool-path: ${archiveTool?.path ?? "unavailable"}`,
    `archive-tool-version: ${archiveTool?.version ?? "unavailable"}`,
    `exact-package-archive-required: ${String(exactPackage)}`,
  ];

  return {
    tauriDriverPath,
    edgeDriverPath,
    edgeDriverVersion: edgeDriverVersion?.version ?? edgeDriverVersionOutput,
    edgeRuntime,
    archiveToolPath: archiveTool?.path ?? null,
    archiveToolVersion: archiveTool?.version ?? null,
    summary: summaryLines.join("\n"),
  };
}

if (
  process.argv[1] &&
  import.meta.url ===
    new URL(`file://${process.argv[1].replace(/\\/g, "/")}`).href
) {
  const { summary } = verifyDesktopSmokePrereqs();
  process.stdout.write(`${summary}\n`);
}
