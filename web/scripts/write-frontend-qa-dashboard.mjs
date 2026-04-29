#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(webRoot, "..");
const outputDir = path.resolve(repoRoot, "output", "qa");
const jsonPath = path.resolve(outputDir, "frontend-qa-dashboard.json");
const markdownPath = path.resolve(outputDir, "frontend-qa-dashboard.md");

function readJson(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath);
  if (!fs.existsSync(fullPath)) return null;
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

function listFiles(dir, predicate) {
  if (!fs.existsSync(dir)) return [];
  const out = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) out.push(...listFiles(fullPath, predicate));
    else if (predicate(fullPath)) out.push(fullPath);
  }
  return out;
}

function fileMeta(relativePath) {
  const fullPath = path.resolve(repoRoot, relativePath);
  if (!fs.existsSync(fullPath)) return null;
  const stat = fs.statSync(fullPath);
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
    "output/qa/qa-selector-inventory.json",
    "output/qa/tauri-capabilities.json",
    "output/qa/ui-perf-budget.json",
    "output/qa/local-qa-summary.json",
    "output/playwright/action-coverage.json",
    "output/playwright/action-coverage-options.json",
    "output/wdio/desktop-smoke-summary.json",
  ]
    .map(fileMeta)
    .filter(Boolean);
}

const localSummary = readJson("output/qa/local-qa-summary.json");
const actionStatic = readJson("output/qa/action-coverage-static.json");
const artifactFixtures = readJson("output/qa/artifact-fixtures.json");
const selectorInventory = readJson("output/qa/qa-selector-inventory.json");
const perfBudget = readJson("output/qa/ui-perf-budget.json");
const desktopSmoke = readJson("output/wdio/desktop-smoke-summary.json");
const workflowIds = countWorkflowIds();
const issues = [
  ...(localSummary?.issues || []).map((issue) => `local summary: ${issue}`),
  ...(actionStatic?.unknownCoverageReferences || []).map(
    (id) => `unknown workflow coverage reference: ${id}`,
  ),
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
  },
  visualBaselines: {
    snapshotCount: listFiles(
      path.resolve(
        webRoot,
        "e2e",
        "browser",
        "qa",
        "visualBaselines.spec.ts-snapshots",
      ),
      (item) => item.endsWith(".png"),
    ).length,
  },
  desktopSmoke: desktopSmoke
    ? {
        status: desktopSmoke.status || "unknown",
        specCount: desktopSmoke.specCount ?? null,
      }
    : null,
  perfBudget: perfBudget
    ? {
        source: perfBudget.source,
        observations: perfBudget.observations,
      }
    : null,
  artifacts: existingArtifacts(),
  issues,
};

const lines = [
  "# Frontend QA Dashboard",
  "",
  `Generated: ${dashboard.generatedAt}`,
  `Status: ${dashboard.status}`,
  `Revision: ${dashboard.cicd["vcs.ref.head.revision"] || "unknown"}`,
  "",
  "## Coverage",
  "",
  `- Workflow ids: ${dashboard.workflows.count}`,
  `- Manifest entries: ${dashboard.actions?.manifestEntryCount ?? "n/a"}`,
  `- Required covered: ${dashboard.actions?.requiredCoveredCount ?? "n/a"} / ${dashboard.actions?.requiredEntryCount ?? "n/a"}`,
  `- Artifact fixture packs: ${dashboard.fixtures.artifactPackCount}`,
  `- Visual snapshots: ${dashboard.visualBaselines.snapshotCount}`,
  `- Desktop smoke summary: ${dashboard.desktopSmoke ? dashboard.desktopSmoke.status : "not present"}`,
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
