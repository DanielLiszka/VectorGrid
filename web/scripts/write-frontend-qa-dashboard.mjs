#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import crypto from "node:crypto";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(webRoot, "..");
const outputDir = path.resolve(repoRoot, "output", "qa");
const jsonPath = path.resolve(outputDir, "frontend-qa-dashboard.json");
const markdownPath = path.resolve(outputDir, "frontend-qa-dashboard.md");

const existsCache = new Map();
const statCache = new Map();
const listAllFilesCache = new Map();

function fileExists(fullPath) {
  let exists = existsCache.get(fullPath);
  if (exists === undefined) {
    exists = fs.existsSync(fullPath);
    existsCache.set(fullPath, exists);
  }
  return exists;
}

function fileStat(fullPath) {
  let stat = statCache.get(fullPath);
  if (!stat) {
    stat = fs.statSync(fullPath);
    statCache.set(fullPath, stat);
  }
  return stat;
}

function readJson(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath);
  if (!fileExists(fullPath)) return null;
  return JSON.parse(fs.readFileSync(fullPath, "utf8"));
}

function safeGit(args) {
  try {
    return execFileSync("git", args, {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return "";
  }
}

function listAllFiles(dir) {
  const cached = listAllFilesCache.get(dir);
  if (cached) return cached;
  if (!fileExists(dir)) {
    listAllFilesCache.set(dir, []);
    return [];
  }
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listAllFiles(fullPath));
    else out.push(fullPath);
  }
  listAllFilesCache.set(dir, out);
  return out;
}

function listFiles(dir, predicate) {
  const files = listAllFiles(dir);
  if (!files.length) return files;
  return files.filter(predicate);
}

function newestMtimeMs(entries) {
  let newest = 0;
  for (const entry of entries) {
    const fullPath = path.resolve(repoRoot, entry);
    if (!fileExists(fullPath)) continue;
    const stat = fileStat(fullPath);
    if (stat.isDirectory()) {
      for (const file of listFiles(
        fullPath,
        (item) => /\.(json|mjs|ts|tsx|vue)$/.test(item),
      )) {
        newest = Math.max(newest, fileStat(file).mtimeMs);
      }
      continue;
    }
    newest = Math.max(newest, stat.mtimeMs);
  }
  return newest;
}

function reportFreshnessIssue(label, relativePath, sourceEntries, required = true) {
  const fullPath = path.resolve(repoRoot, relativePath);
  if (!fileExists(fullPath)) {
    return required ? `${label} report is missing: ${relativePath}` : null;
  }
  let generatedAtMs = 0;
  try {
    const report = JSON.parse(fs.readFileSync(fullPath, "utf8"));
    generatedAtMs = Date.parse(report.generatedAt || "");
  } catch {
    return `${label} report is not valid JSON: ${relativePath}`;
  }
  const reportMs = Number.isFinite(generatedAtMs)
    ? generatedAtMs
    : fileStat(fullPath).mtimeMs;
  const sourceMs = newestMtimeMs(sourceEntries);
  if (sourceMs > 0 && sourceMs - reportMs > 1000) {
    return `${label} report is stale: ${relativePath}`;
  }
  return null;
}

function collectFreshnessIssues() {
  return [
    reportFreshnessIssue("static action coverage", "output/qa/action-coverage-static.json", [
      "web/src",
      "web/e2e/browser/qa/actionCoverageManifest.ts",
      "web/e2e/browser/qa",
      "web/scripts/check-action-coverage-manifest.ts",
    ]),
    reportFreshnessIssue("artifact fixture", "output/qa/artifact-fixtures.json", [
      "web/e2e/browser/fixtures/artifacts",
      "web/scripts/check-e2e-artifact-fixtures.mjs",
    ]),
    reportFreshnessIssue("fixture matrix", "output/qa/e2e-fixture-matrix.json", [
      "web/e2e/browser/fixtures",
      "web/e2e/browser/qa",
      "web/scripts/check-e2e-fixture-matrix.ts",
    ]),
    reportFreshnessIssue("selector inventory", "output/qa/qa-selector-inventory.json", [
      "web/src",
      "web/scripts/check-qa-selector-inventory.mjs",
    ]),
    reportFreshnessIssue("Tauri capability", "output/qa/tauri-capabilities.json", [
      "src-tauri/capabilities",
      "src-tauri/tauri.conf.json",
      "web/scripts/check-tauri-capabilities.mjs",
    ]),
    reportFreshnessIssue("UI perf budget", "output/qa/ui-perf-budget.json", [
      "scripts/baselines/ui_capture/perf_metrics.json",
      "web/scripts/check-ui-perf-budget.mjs",
    ]),
    reportFreshnessIssue("local QA summary", "output/qa/local-qa-summary.json", [
      "output/qa/action-coverage-static.json",
      "output/qa/artifact-fixtures.json",
      "output/qa/e2e-fixture-matrix.json",
      "output/qa/qa-selector-inventory.json",
      "output/qa/tauri-capabilities.json",
      "output/qa/ui-perf-budget.json",
      "web/scripts/write-local-qa-summary.mjs",
    ]),
    reportFreshnessIssue("runtime action coverage", "output/playwright/action-coverage.json", [
      "web/src",
      "web/e2e/browser/qa",
    ], false),
    reportFreshnessIssue("runtime options action coverage", "output/playwright/action-coverage-options.json", [
      "web/src",
      "web/e2e/browser/qa",
    ], false),
  ].filter(Boolean);
}

function fileMeta(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath);
  if (!fileExists(fullPath)) return null;
  const stat = fileStat(fullPath);
  return {
    path: relativePath.replace(/\\/g, "/"),
    bytes: stat.size,
    modifiedAt: stat.mtime.toISOString(),
  };
}

function countWorkflowIds() {
  const qaDir = path.resolve(webRoot, "e2e", "browser", "qa");
  const ids = new Set();
  for (const file of listFiles(qaDir, (item) => item.endsWith(".ts"))) {
    const content = fs.readFileSync(file, "utf8");
    for (const match of content.matchAll(
      /\bdefineWorkflow\s*\(\s*\{\s*id:\s*['"]([^'"]+)['"]/g,
    )) {
      ids.add(match[1]);
    }
  }
  return [...ids].sort();
}

function existingArtifacts() {
  return [
    "output/qa/action-coverage-static.json",
    "output/qa/artifact-fixtures.json",
    "output/qa/e2e-fixture-matrix.json",
    "output/qa/qa-selector-inventory.json",
    "output/qa/tauri-capabilities.json",
    "output/qa/ui-perf-budget.json",
    "output/qa/local-qa-summary.json",
    "output/playwright/action-coverage.json",
    "output/playwright/action-coverage-options.json",
    "output/wdio/desktop-smoke-summary.json",
    "output/wdio/desktop-smoke-expanded-summary.json",
  ]
    .map(fileMeta)
    .filter(Boolean);
}

function directoryMeta(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath);
  if (!fileExists(fullPath)) {
    return {
      path: relativePath.replace(/\\/g, "/"),
      present: false,
      fileCount: 0,
      bytes: 0,
      modifiedAt: "",
    };
  }
  const files = listFiles(fullPath, () => true);
  const bytes = files.reduce((acc, file) => acc + fileStat(file).size, 0);
  const modifiedMs = files.reduce(
    (acc, file) => Math.max(acc, fileStat(file).mtimeMs),
    fileStat(fullPath).mtimeMs,
  );
  return {
    path: relativePath.replace(/\\/g, "/"),
    present: true,
    fileCount: files.length,
    bytes,
    modifiedAt: new Date(modifiedMs).toISOString(),
  };
}

function normalizeOptionalReport(report) {
  if (!report) return null;
  return {
    status: report.status || "unknown",
    generatedAt: report.generatedAt || "",
    scannedSurfaces: Array.isArray(report.scannedSurfaces)
      ? report.scannedSurfaces.map((surface) => ({
          name: surface.name || "",
          violationCount: surface.violationCount ?? null,
        }))
      : [],
    includeColorContrast: !!report.includeColorContrast,
    includeWcag22: !!report.includeWcag22,
  };
}

function fileSha256(fullPath) {
  return crypto.createHash("sha256").update(fs.readFileSync(fullPath)).digest("hex");
}

function collectVisualBaselineEvidence() {
  const specRelativePath = "web/e2e/browser/qa/visualBaselines.spec.ts";
  const snapshotDirRelativePath = "web/e2e/browser/qa/visualBaselines.spec.ts-snapshots";
  const specPath = path.resolve(repoRoot, specRelativePath);
  const snapshotDir = path.resolve(repoRoot, snapshotDirRelativePath);
  const projectName = "chromium";
  const platformName = process.platform;
  const specContent = fileExists(specPath) ? fs.readFileSync(specPath, "utf8") : "";
  const baselineNames = [
    ...new Set(
      [...specContent.matchAll(/\btoHaveScreenshot\s*\(\s*['"`]([^'"`]+\.png)['"`]/g)]
        .map((match) => String(match[1] || "").trim())
        .filter(Boolean),
    ),
  ].sort();
  const expectedFiles = baselineNames.map((name) => {
    const parsed = path.parse(name);
    return `${parsed.name}-${projectName}-${platformName}${parsed.ext || ".png"}`;
  });
  const actualFiles = listFiles(snapshotDir, (item) => item.endsWith(".png"))
    .map((file) => path.basename(file))
    .sort();
  const actualSet = new Set(actualFiles);
  const expectedSet = new Set(expectedFiles);
  const files = expectedFiles.map((file, index) => {
    const fullPath = path.resolve(snapshotDir, file);
    const exists = fileExists(fullPath);
    const stat = exists ? fileStat(fullPath) : null;
    return {
      baselineName: baselineNames[index],
      file,
      path: path.join(snapshotDirRelativePath, file).replace(/\\/g, "/"),
      present: exists,
      bytes: stat?.size ?? 0,
      modifiedAt: stat ? stat.mtime.toISOString() : "",
      sha256: exists ? fileSha256(fullPath) : "",
    };
  });
  const specMs = fileExists(specPath) ? fileStat(specPath).mtimeMs : 0;
  const stale = files
    .filter((file) => {
      if (!file.present || specMs <= 0) return false;
      const snapshotMs = Date.parse(file.modifiedAt);
      return Number.isFinite(snapshotMs) && specMs - snapshotMs > 1000;
    })
    .map((file) => file.file);
  return {
    specPath: specRelativePath,
    snapshotDir: snapshotDirRelativePath,
    projectName,
    platformName,
    expectedCount: expectedFiles.length,
    actualCount: actualFiles.length,
    snapshotCount: actualFiles.length,
    missing: expectedFiles.filter((file) => !actualSet.has(file)),
    extra: actualFiles.filter((file) => !expectedSet.has(file)),
    stale,
    files,
  };
}

function visualBaselineIssues(evidence) {
  const out = [];
  if (!evidence.expectedCount) {
    out.push("visual baseline spec has no toHaveScreenshot baselines");
  }
  out.push(...evidence.missing.map((file) => `visual baseline snapshot missing: ${file}`));
  out.push(...evidence.extra.map((file) => `visual baseline snapshot extra or stale: ${file}`));
  return out;
}

function normalizedDesktopSummary(summary, primarySummary = null) {
  if (!summary) return null;
  const generatedAt = summary.generatedAt || "";
  const primaryGeneratedAt = primarySummary?.generatedAt || "";
  const stale =
    primaryGeneratedAt &&
    generatedAt &&
    Date.parse(generatedAt) < Date.parse(primaryGeneratedAt) &&
    summary.status !== "not_run";
  return {
    status: stale ? "stale" : summary.status || "unknown",
    suite: summary.suite || "",
    specCount: summary.specCount ?? null,
    generatedAt,
    runId: summary.runId || "",
    stale,
  };
}

const localSummary = readJson("output/qa/local-qa-summary.json");
const actionStatic = readJson("output/qa/action-coverage-static.json");
const artifactFixtures = readJson("output/qa/artifact-fixtures.json");
const fixtureMatrix = readJson("output/qa/e2e-fixture-matrix.json");
const selectorInventory = readJson("output/qa/qa-selector-inventory.json");
const perfBudget = readJson("output/qa/ui-perf-budget.json");
const a11ySmoke = readJson("output/qa/a11y-smoke.json");
const desktopSmoke = readJson("output/wdio/desktop-smoke-summary.json");
const desktopSmokeExpanded = readJson("output/wdio/desktop-smoke-expanded-summary.json");
const workflowIds = countWorkflowIds();
const visualBaselineEvidence = collectVisualBaselineEvidence();
const dirtyFiles = safeGit(["status", "--porcelain"])
  .split(/\r?\n/)
  .map((line) => line.trim())
  .filter(Boolean);
const issues = [
  ...collectFreshnessIssues(),
  ...(localSummary?.issues || []).map((issue) => `local summary: ${issue}`),
  ...(actionStatic?.unknownCoverageReferences || []).map(
    (id) => `unknown workflow coverage reference: ${id}`,
  ),
  ...(fixtureMatrix?.issues || []).map((issue) => `fixture matrix: ${issue}`),
  ...visualBaselineIssues(visualBaselineEvidence),
];

const dashboard = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  status: issues.length > 0 ? "failed" : "passed",
  cicd: {
    "cicd.pipeline.name": process.env.GITHUB_WORKFLOW || "local-frontend-qa",
    "cicd.pipeline.run.id": process.env.GITHUB_RUN_ID || "",
    "cicd.pipeline.run.url.full":
      process.env.GITHUB_SERVER_URL &&
      process.env.GITHUB_REPOSITORY &&
      process.env.GITHUB_RUN_ID
        ? `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}`
        : "",
    "vcs.repository.url.full":
      process.env.GITHUB_SERVER_URL && process.env.GITHUB_REPOSITORY
        ? `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}`
        : safeGit(["config", "--get", "remote.origin.url"]),
    "vcs.ref.head.name":
      process.env.GITHUB_REF_NAME || safeGit(["branch", "--show-current"]),
    "vcs.ref.head.revision":
      process.env.GITHUB_SHA || safeGit(["rev-parse", "HEAD"]),
    "vcs.working_tree.dirty": dirtyFiles.length > 0,
    "vcs.working_tree.dirty_file_count": dirtyFiles.length,
  },
  workflows: {
    count: workflowIds.length,
    ids: workflowIds,
  },
  actions: actionStatic
    ? {
        sourceActionCount: actionStatic.sourceActionCount,
        manifestEntryCount: actionStatic.manifestEntryCount,
        requiredEntryCount: actionStatic.requiredEntryCount,
        requiredCoveredCount: actionStatic.requiredCoveredCount,
        waiverCount: actionStatic.waiverCount,
        unknownCoverageReferences: actionStatic.unknownCoverageReferences || [],
        priorityCounts: actionStatic.priorityCounts || {},
        surfaceCounts: actionStatic.surfaceCounts || {},
        requiredCoverageByPriority: actionStatic.requiredCoverageByPriority || {},
        requiredCoverageBySurface: actionStatic.requiredCoverageBySurface || {},
        nearExpiryWaivers: actionStatic.nearExpiryWaivers || [],
        expiredWaivers: actionStatic.expiredWaivers || [],
      }
    : null,
  selectors: selectorInventory
    ? {
        annotatedControlCount: selectorInventory.annotatedControlCount,
        controlCount: selectorInventory.controlCount,
        actionSelectorCount: selectorInventory.actionSelectorCount,
        panelSelectorCount: selectorInventory.panelSelectorCount,
      }
    : null,
  fixtures: {
    artifactPackCount: artifactFixtures?.reports?.length ?? 0,
    matrixStatus: fixtureMatrix?.status || "not present",
    scenarioCount: fixtureMatrix?.scenarioCount ?? 0,
    registeredWorkflowCount: fixtureMatrix?.registeredWorkflowCount ?? 0,
    localWorkflowScenarioCount: fixtureMatrix?.localWorkflowScenarioCount ?? 0,
    fixtureArtifactCount: fixtureMatrix?.artifactReports?.length ?? 0,
    requiredWorkflowBacklog: fixtureMatrix?.requiredWorkflowBacklog || [],
    families: fixtureMatrix?.families || {},
  },
  visualBaselines: {
    ...visualBaselineEvidence,
  },
  desktopSmoke: normalizedDesktopSummary(desktopSmoke),
  desktopSmokeExpanded: normalizedDesktopSummary(desktopSmokeExpanded, desktopSmoke),
  a11ySmoke: normalizeOptionalReport(a11ySmoke),
  playwrightArtifacts: [
    directoryMeta("output/playwright/browser-ux-artifacts"),
    directoryMeta("output/playwright/browser-ux-report"),
    directoryMeta("output/playwright/heatmap-visual"),
    directoryMeta("output/playwright/workbench-visual"),
  ],
  perfBudget: perfBudget
    ? {
        source: perfBudget.source,
        observations: perfBudget.observations,
      }
    : null,
  artifacts: existingArtifacts(),
  issues,
};

function formatCoverageBuckets(buckets) {
  const entries = Object.entries(buckets || {}).sort(([a], [b]) =>
    a.localeCompare(b),
  );
  if (!entries.length) return "- none";
  return entries
    .map(([key, value]) => {
      const required = value?.required ?? 0;
      const covered = value?.covered ?? 0;
      const waived = value?.waived ?? 0;
      return `- ${key}: ${covered}/${required} required covered (${waived} waived)`;
    })
    .join("\n");
}

function formatFixtureFamilies(families) {
  const entries = Object.entries(families || {}).sort(([a], [b]) =>
    a.localeCompare(b),
  );
  if (!entries.length) return "- none";
  return entries
    .map(([family, report]) => {
      const deferred = report?.deferred?.reason
        ? `; deferred: ${report.deferred.reason}`
        : "";
      return `- ${family}: ${report?.scenarioCount ?? 0} scenarios, ${report?.workflowCount ?? 0} workflows${deferred}`;
    })
    .join("\n");
}

function formatNearExpiryWaivers(waivers) {
  if (!waivers?.length) return "- none";
  return waivers
    .map(
      (waiver) =>
        `- ${waiver.actionId}: ${waiver.expires || "no expiry"} (${waiver.daysUntilExpiry ?? "n/a"} days, ${waiver.owner})`,
    )
    .join("\n");
}

function formatWorkflowBacklog(items) {
  if (!items?.length) return "- none";
  return items
    .map((item) => {
      const matched = item.matchedBy ? ` via ${item.matchedBy}` : "";
      const deferred = item.deferredReason ? `; ${item.deferredReason}` : "";
      return `- ${item.id}: ${item.status}${matched}${deferred}`;
    })
    .join("\n");
}

function formatPlaywrightArtifacts(artifacts) {
  if (!artifacts?.length) return "- none";
  return artifacts
    .map((artifact) =>
      artifact.present
        ? `- ${artifact.path}: ${artifact.fileCount} files, ${artifact.bytes} bytes`
        : `- ${artifact.path}: not present`,
    )
    .join("\n");
}

function formatVisualBaselines(visualBaselines) {
  if (!visualBaselines?.files?.length) return "- none";
  return visualBaselines.files
    .map((file) =>
      file.present
        ? `- ${file.baselineName}: ${file.file} (${file.bytes} bytes)`
        : `- ${file.baselineName}: missing ${file.file}`,
    )
    .join("\n");
}

const lines = [
  "# Frontend QA Dashboard",
  "",
  `Generated: ${dashboard.generatedAt}`,
  `Status: ${dashboard.status}`,
  `Revision: ${dashboard.cicd["vcs.ref.head.revision"] || "unknown"}`,
  `Dirty working tree: ${dashboard.cicd["vcs.working_tree.dirty"] ? `yes (${dashboard.cicd["vcs.working_tree.dirty_file_count"]} files)` : "no"}`,
  "",
  "## Coverage",
  "",
  `- Workflow ids: ${dashboard.workflows.count}`,
  `- Manifest entries: ${dashboard.actions?.manifestEntryCount ?? "n/a"}`,
  `- Required covered: ${dashboard.actions?.requiredCoveredCount ?? "n/a"} / ${dashboard.actions?.requiredEntryCount ?? "n/a"}`,
  `- Artifact fixture packs: ${dashboard.fixtures.artifactPackCount}`,
  `- Fixture matrix: ${dashboard.fixtures.matrixStatus}`,
  `- Registered scenarios: ${dashboard.fixtures.scenarioCount}`,
  `- Registered workflow scenario refs: ${dashboard.fixtures.registeredWorkflowCount}`,
  `- Local workflow scenario refs: ${dashboard.fixtures.localWorkflowScenarioCount}`,
  `- Fixture JSON artifacts checked: ${dashboard.fixtures.fixtureArtifactCount}`,
  `- Required workflow backlog covered: ${dashboard.fixtures.requiredWorkflowBacklog.filter((item) => item.status === "covered").length} / ${dashboard.fixtures.requiredWorkflowBacklog.length}`,
  `- Visual snapshots: ${dashboard.visualBaselines.actualCount} actual / ${dashboard.visualBaselines.expectedCount} expected`,
  `- Visual missing/extra: ${dashboard.visualBaselines.missing.length} missing / ${dashboard.visualBaselines.extra.length} extra`,
  `- Visual older-than-spec snapshots: ${dashboard.visualBaselines.stale.length}`,
  `- Desktop smoke summary: ${dashboard.desktopSmoke ? dashboard.desktopSmoke.status : "not present"}`,
  `- Desktop smoke expanded summary: ${dashboard.desktopSmokeExpanded ? dashboard.desktopSmokeExpanded.status : "not present"}`,
  `- Accessibility smoke: ${dashboard.a11ySmoke ? dashboard.a11ySmoke.status : "not present"}`,
  "",
  "## Fixture Families",
  "",
  formatFixtureFamilies(dashboard.fixtures.families),
  "",
  "## Required Workflow Backlog",
  "",
  formatWorkflowBacklog(dashboard.fixtures.requiredWorkflowBacklog),
  "",
  "## Required Actions By Priority",
  "",
  formatCoverageBuckets(dashboard.actions?.requiredCoverageByPriority),
  "",
  "## Required Actions By Surface",
  "",
  formatCoverageBuckets(dashboard.actions?.requiredCoverageBySurface),
  "",
  "## Near-Expiry Waivers",
  "",
  formatNearExpiryWaivers(dashboard.actions?.nearExpiryWaivers),
  "",
  "## Playwright Artifacts",
  "",
  formatPlaywrightArtifacts(dashboard.playwrightArtifacts),
  "",
  "## Visual Baselines",
  "",
  formatVisualBaselines(dashboard.visualBaselines),
  "",
  "## Artifacts",
  "",
  dashboard.artifacts.length
    ? dashboard.artifacts
        .map((artifact) => `- ${artifact.path} (${artifact.bytes} bytes)`)
        .join("\n")
    : "- none",
  "",
  "## Issues",
  "",
  dashboard.issues.length
    ? dashboard.issues.map((issue) => `- ${issue}`).join("\n")
    : "- none",
  "",
];

fs.mkdirSync(outputDir, { recursive: true });
fs.writeFileSync(jsonPath, `${JSON.stringify(dashboard, null, 2)}\n`, "utf8");
fs.writeFileSync(markdownPath, `${lines.join("\n")}\n`, "utf8");
console.log(
  `[qa-dashboard] ${dashboard.status}. Dashboard written to ${path.relative(webRoot, markdownPath).replace(/\\/g, "/")}`,
);
if (dashboard.status === "failed") {
  for (const issue of dashboard.issues) console.error(`[qa-dashboard] ${issue}`);
  process.exit(1);
}
