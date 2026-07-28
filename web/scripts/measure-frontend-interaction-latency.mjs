#!/usr/bin/env node
import crypto from "node:crypto";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

import {
  FILE_SET_HASH_FRAMING,
  atomicWriteJson,
  computeCanonicalFileSetSnapshot,
  readBoundedFilePublication,
  readStableSingleLinkFile,
  resolveBoundedOutputPath,
  validateBuildAttestation,
  validateImmutableDistSnapshot,
} from "./frontend-interaction-build-attestation.mjs";
import { verifyAuthorityLease } from "./frontend-interaction-authority-lease.mjs";
import {
  EXPECTED_MEASUREMENT_HARNESS_PATHS,
  buildPathDistinctDetailFixture,
  buildLatencyReport,
  computeAuthorityScopedWorktreeState,
  validatePolicy,
} from "./frontend-interaction-latency-lib.mjs";
import {
  closeStaticSnapshotServer,
  startStaticServerFromSnapshot,
} from "./frontend-interaction-static-snapshot.mjs";
import {
  dismissStartupOverlayIfPresent,
  installMockedTauri,
  seedAdvancedUiMode,
} from "./support/mockedTauriHarness.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const webRoot = path.resolve(scriptDir, "..");
const repoRoot = path.resolve(webRoot, "..");
const policyPath = path.resolve(
  repoRoot,
  "scripts",
  "frontend_interaction_latency_policy.json",
);
const fixturePath = path.resolve(
  webRoot,
  "e2e",
  "browser",
  "fixtures",
  "artifacts",
  "saved-options-artifact-pack.json",
);
const measurementHarnessPaths = [...EXPECTED_MEASUREMENT_HARNESS_PATHS];

function parseArgs(argv) {
  const args = {
    authority: false,
    buildAttestationPath: "",
    distSnapshotPath: "",
    lockLeasePath: "",
    lockNonce: "",
    outPath: "",
    runId: crypto.randomUUID().replaceAll("-", ""),
    runStartedAt: new Date().toISOString(),
  };
  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];
    if (arg === "--authority") {
      args.authority = true;
      continue;
    }
    if (
      [
        "--build-attestation",
        "--dist-snapshot",
        "--lock-lease",
        "--lock-nonce",
        "--out",
        "--run-id",
        "--run-started-at",
      ].includes(arg)
    ) {
      index += 1;
      if (!argv[index]) throw new Error(`${arg} requires a value`);
      if (arg === "--build-attestation")
        args.buildAttestationPath = argv[index];
      if (arg === "--dist-snapshot") args.distSnapshotPath = argv[index];
      if (arg === "--lock-lease") args.lockLeasePath = argv[index];
      if (arg === "--lock-nonce") args.lockNonce = argv[index];
      if (arg === "--out") args.outPath = argv[index];
      if (arg === "--run-id") args.runId = argv[index];
      if (arg === "--run-started-at") args.runStartedAt = argv[index];
      continue;
    }
    throw new Error(`unknown argument: ${arg}`);
  }
  if (!args.outPath) throw new Error("--out is required");
  if (!args.buildAttestationPath)
    throw new Error("--build-attestation is required");
  if (!args.distSnapshotPath) throw new Error("--dist-snapshot is required");
  if (!args.lockLeasePath) throw new Error("--lock-lease is required");
  if (!/^[a-f0-9]{32}$/.test(args.lockNonce)) {
    throw new Error("--lock-nonce must be a 32-character lowercase hex token");
  }
  return args;
}

function sha256Bytes(bytes) {
  return crypto.createHash("sha256").update(bytes).digest("hex");
}

function gitOutput(args) {
  const result = spawnSync("git", args, {
    cwd: repoRoot,
    encoding: "utf8",
    windowsHide: true,
  });
  return result.status === 0 ? String(result.stdout || "").trim() : "";
}

async function waitForStablePaint(page, quietWindowMs) {
  await page.evaluate(async (quietMs) => {
    await new Promise((resolve) => {
      globalThis.setTimeout(resolve, quietMs);
    });
    await new Promise((resolve) => {
      requestAnimationFrame(() =>
        requestAnimationFrame(() => globalThis.setTimeout(resolve, 0)),
      );
    });
  }, quietWindowMs);
}

async function measureClickToStablePaint(
  page,
  {
    triggerSelector,
    triggerIdentity,
    readinessContractId,
    readinessTargetId,
    readyChecks,
    quietWindowMs,
    timeoutMs,
  },
) {
  await page.evaluate(
    ({
      trigger,
      triggerId,
      contractId,
      targetId,
      checks,
      quietMs,
      deadlineMs,
    }) => {
      const latencyWindow = window;
      if (latencyWindow.__VTA_FRONTEND_LATENCY_PENDING__) {
        throw new Error(
          "a frontend interaction latency measurement is already pending",
        );
      }

      latencyWindow.__VTA_FRONTEND_LATENCY_PENDING__ = new Promise(
        (resolve, reject) => {
          let settled = false;
          let observer = null;
          let timeoutHandle = 0;

          const cleanup = () => {
            document.removeEventListener("click", onClick, true);
            observer?.disconnect();
            if (timeoutHandle) globalThis.clearTimeout(timeoutHandle);
          };
          const fail = (message) => {
            if (settled) return;
            settled = true;
            cleanup();
            reject(new Error(message));
          };
          const visible = (element) => {
            if (!(element instanceof HTMLElement)) return false;
            const style = globalThis.getComputedStyle(element);
            const rect = element.getBoundingClientRect();
            return (
              style.display !== "none" &&
              style.visibility !== "hidden" &&
              Number(style.opacity || "1") > 0 &&
              rect.width > 0 &&
              rect.height > 0
            );
          };
          const ready = () =>
            checks.every((check) => {
              const element = document.querySelector(check.selector);
              if (!element || !visible(element)) return false;
              if (
                check.attribute &&
                element.getAttribute(check.attribute) !==
                  String(check.equals ?? "")
              ) {
                return false;
              }
              if (
                check.text_equals != null &&
                String(element.textContent || "").trim() !==
                  String(check.text_equals)
              ) {
                return false;
              }
              return true;
            });

          const onClick = (event) => {
            const eventTarget =
              event.target instanceof Element ? event.target : null;
            if (!eventTarget?.closest(trigger)) return;
            document.removeEventListener("click", onClick, true);

            const clickedAt = performance.now();
            let lastMutationAt = clickedAt;
            observer = new MutationObserver(() => {
              lastMutationAt = performance.now();
            });
            observer.observe(document.documentElement, {
              attributes: true,
              childList: true,
              characterData: true,
              subtree: true,
            });

            const checkAfterPaint = () => {
              if (settled) return;
              if (!ready() || performance.now() - lastMutationAt < quietMs) {
                requestAnimationFrame(checkAfterPaint);
                return;
              }
              requestAnimationFrame(() => {
                requestAnimationFrame(() => {
                  globalThis.setTimeout(() => {
                    const readyAtCompletion = ready();
                    if (
                      !readyAtCompletion ||
                      performance.now() - lastMutationAt < quietMs
                    ) {
                      requestAnimationFrame(checkAfterPaint);
                      return;
                    }
                    const paintedStableAt = performance.now();
                    settled = true;
                    cleanup();
                    resolve({
                      duration_ms: Number(
                        (paintedStableAt - clickedAt).toFixed(3),
                      ),
                      click_is_trusted: event.isTrusted === true,
                      quiet_window_observed_ms: Number(
                        (paintedStableAt - lastMutationAt).toFixed(3),
                      ),
                      ready_checks_satisfied: readyAtCompletion,
                      readiness_contract_id: contractId,
                      readiness_target_id: targetId,
                      trigger_identity: triggerId,
                    });
                  }, 0);
                });
              });
            };
            requestAnimationFrame(checkAfterPaint);
          };

          document.addEventListener("click", onClick, true);
          timeoutHandle = globalThis.setTimeout(
            () =>
              fail(
                `interaction did not reach a painted stable state within ${deadlineMs}ms`,
              ),
            deadlineMs,
          );
        },
      );
    },
    {
      trigger: triggerSelector,
      triggerId: triggerIdentity,
      contractId: readinessContractId,
      targetId: readinessTargetId,
      checks: readyChecks,
      quietMs: quietWindowMs,
      deadlineMs: timeoutMs,
    },
  );

  try {
    await page.locator(triggerSelector).click();
    return await page.evaluate(() => window.__VTA_FRONTEND_LATENCY_PENDING__);
  } finally {
    await page.evaluate(() => {
      delete window.__VTA_FRONTEND_LATENCY_PENDING__;
    });
  }
}

async function ensureWorkspace(page, workspace, quietWindowMs) {
  const tab = page.locator(`[data-qa-action="shell.workspace.${workspace}"]`);
  if ((await tab.getAttribute("aria-selected")) !== "true") {
    await tab.click();
  }
  await page.waitForSelector(
    `[data-qa-action="shell.workspace.${workspace}"][aria-selected="true"]`,
    { state: "visible" },
  );
  await waitForStablePaint(page, quietWindowMs);
}

async function prepareSavedRunList(page, quietWindowMs, expectedCount) {
  await ensureWorkspace(page, "library", quietWindowMs);
  await page
    .locator('[data-qa-action="library.savedRun.refresh"]')
    .first()
    .click();
  await page.waitForFunction((count) => {
    const text = String(
      document.querySelector(".historyPanelVueRoot")?.textContent || "",
    );
    return text.includes(`Saved runs`) && text.includes(String(count));
  }, expectedCount);
  await waitForStablePaint(page, quietWindowMs);
}

async function readDetailHydrationEvidence(page, artifactPath) {
  const entries = await page.evaluate(
    ({ pathValue }) =>
      (window.__VTA_TEST_INVOKE_LOG || [])
        .filter(
          (entry) =>
            entry?.command === "load_run_artifact_detail" &&
            String(entry?.args?.path || "") === pathValue,
        )
        .map((entry) => ({
          command: String(entry.command || ""),
          requested_path: String(entry.args?.path || ""),
          matched_argument_name: String(entry.matchedArgumentName || ""),
          matched_argument_value: String(entry.matchedArgumentValue || ""),
          delayed_ms: Number(entry.delayedMs),
          duration_ms: Number(entry.durationMs),
        })),
    { pathValue: artifactPath },
  );
  return entries;
}

async function validateDetailHydrationRunEvidence(
  page,
  identities,
  expectedDelayMs,
) {
  const entries = await page.evaluate(() =>
    (window.__VTA_TEST_INVOKE_LOG || [])
      .filter((entry) => entry?.command === "load_run_artifact_detail")
      .map((entry) => ({
        path: String(entry?.args?.path || ""),
        matched_argument_name: String(entry?.matchedArgumentName || ""),
        matched_argument_value: String(entry?.matchedArgumentValue || ""),
        delayed_ms: Number(entry?.delayedMs),
      })),
  );
  const expectedPaths = identities.map((identity) => identity.artifactPath);
  const observedPaths = entries.map((entry) => entry.path);
  const issues = [];
  if (entries.length !== expectedPaths.length) {
    issues.push(
      `detail hydration count ${entries.length} did not equal ${expectedPaths.length}`,
    );
  }
  if (
    new Set(observedPaths).size !== observedPaths.length ||
    expectedPaths.some(
      (expectedPath) =>
        observedPaths.filter((value) => value === expectedPath).length !== 1,
    ) ||
    observedPaths.some((observedPath) => !expectedPaths.includes(observedPath))
  ) {
    issues.push(
      "detail hydration paths were not an exact one-to-one match for the path-distinct fixture",
    );
  }
  if (
    entries.some(
      (entry) =>
        entry.matched_argument_name !== "path" ||
        entry.matched_argument_value !== entry.path ||
        entry.delayed_ms !== expectedDelayMs,
    )
  ) {
    issues.push(
      "detail hydration log contained unmatched arguments or an unexpected delay",
    );
  }
  return issues;
}

async function measureSavedRunDetail(page, policy, identity) {
  await ensureWorkspace(page, "library", policy.dom_quiet_window_ms);
  const search = page.locator("#vueSavedRunsSearch");
  await search.fill(identity.searchToken);
  await page.waitForFunction(
    ({ token, artifactPath }) => {
      const rows = [
        ...document.querySelectorAll(
          'ul[aria-label="Saved run artifacts"] button[data-saved-run="1"]',
        ),
      ];
      return (
        rows.length === 1 &&
        String(rows[0].textContent || "").includes(token) &&
        rows[0].getAttribute("aria-pressed") === "false" &&
        !String(
          document.querySelector(".historyPanelVueRoot .detailCard pre.mono")
            ?.textContent || "",
        ).includes(artifactPath)
      );
    },
    {
      token: identity.searchToken,
      artifactPath: identity.artifactPath,
    },
  );
  await page.evaluate(() => {
    const requestDetails = [
      ...document.querySelectorAll(".historyPanelVueRoot details.panelDetails"),
    ].find(
      (element) =>
        String(element.querySelector("summary")?.textContent || "").trim() ===
        "Request snapshot",
    );
    if (!(requestDetails instanceof HTMLDetailsElement)) {
      throw new Error("saved-run Request snapshot details were not found");
    }
    requestDetails.open = true;
    const readyField = requestDetails.querySelector("pre.mono");
    if (!(readyField instanceof HTMLElement)) {
      throw new Error("saved-run parameter ready field was not found");
    }
    readyField.setAttribute(
      "data-vta-latency-ready-field",
      "savedRunDetailParams",
    );
  });
  await waitForStablePaint(page, policy.dom_quiet_window_ms);

  const target = page
    .locator('ul[aria-label="Saved run artifacts"] button[data-saved-run="1"]')
    .first();
  await target.evaluate((element, artifactPath) => {
    element.setAttribute("data-vta-latency-target", "true");
    element.setAttribute("data-vta-latency-artifact-path", artifactPath);
  }, identity.artifactPath);
  const beforeInvokes = await readDetailHydrationEvidence(
    page,
    identity.artifactPath,
  );
  if (beforeInvokes.length !== 0) {
    throw new Error(
      `detail path was already hydrated before measurement: ${identity.artifactPath}`,
    );
  }

  try {
    const result = await measureClickToStablePaint(page, {
      triggerSelector:
        '[data-vta-latency-target="true"][data-vta-latency-artifact-path]',
      triggerIdentity: identity.artifactPath,
      readinessContractId: policy.cases.find(
        (entry) => entry.id === "saved-run-detail-open",
      ).readiness_contract_id,
      readinessTargetId: identity.artifactPath,
      readyChecks: [
        {
          selector: '[data-vta-latency-target="true"]',
          attribute: "aria-pressed",
          equals: "true",
        },
        {
          selector: '[data-vta-latency-ready-field="savedRunDetailParams"]',
          text_equals: identity.readyFieldValue,
        },
      ],
      quietWindowMs: policy.dom_quiet_window_ms,
      timeoutMs: policy.interaction_timeout_ms,
    });
    const hydrationEntries = await readDetailHydrationEvidence(
      page,
      identity.artifactPath,
    );
    const hydration = hydrationEntries[0] || {};
    return {
      ...result,
      ready_field_value: identity.readyFieldValue,
      async_detail_hydration: {
        command: hydration.command || "",
        requested_path: hydration.requested_path || "",
        matched_argument_name: hydration.matched_argument_name || "",
        matched_argument_value: hydration.matched_argument_value || "",
        argument_matched:
          hydration.matched_argument_name === "path" &&
          hydration.matched_argument_value === identity.artifactPath,
        invocation_count_for_path: hydrationEntries.length,
        delayed_ms: hydration.delayed_ms,
        duration_ms: hydration.duration_ms,
        ready_field_identity: "savedRunDetailParams",
        ready_field_value: identity.readyFieldValue,
      },
    };
  } finally {
    await page.evaluate(() => {
      for (const element of document.querySelectorAll(
        "[data-vta-latency-target], [data-vta-latency-artifact-path]",
      )) {
        element.removeAttribute("data-vta-latency-target");
        element.removeAttribute("data-vta-latency-artifact-path");
      }
      document
        .querySelector('[data-vta-latency-ready-field="savedRunDetailParams"]')
        ?.removeAttribute("data-vta-latency-ready-field");
    });
  }
}

async function measureWorkspaceNavigation(page, policy) {
  await ensureWorkspace(page, "workflow", policy.dom_quiet_window_ms);
  const casePolicy = policy.cases.find(
    (entry) => entry.id === "workspace-navigation",
  );
  const result = await measureClickToStablePaint(page, {
    triggerSelector: '[data-qa-action="shell.workspace.library"]',
    triggerIdentity: "shell.workspace.library",
    readinessContractId: casePolicy.readiness_contract_id,
    readinessTargetId: "workspace:library",
    readyChecks: [
      {
        selector: '[data-qa-action="shell.workspace.library"]',
        attribute: "aria-selected",
        equals: "true",
      },
      { selector: "#results_panel_history" },
      { selector: ".historyPanelVueRoot" },
    ],
    quietWindowMs: policy.dom_quiet_window_ms,
    timeoutMs: policy.interaction_timeout_ms,
  });
  await ensureWorkspace(page, "workflow", policy.dom_quiet_window_ms);
  return {
    ...result,
    ready_field_value: null,
    async_detail_hydration: null,
  };
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const outPath = resolveBoundedOutputPath({
    repoRoot,
    candidatePath: args.outPath,
    mustNotExist: true,
  });

  const policyPublication = readStableSingleLinkFile(
    policyPath,
    "interaction-latency policy",
  );
  const policyBytes = policyPublication.bytes;
  const policy = JSON.parse(policyBytes.toString("utf8"));
  const policyIssues = validatePolicy(policy);
  if (policyIssues.length > 0) {
    throw new Error(
      `invalid interaction-latency policy: ${policyIssues.join("; ")}`,
    );
  }
  const initialLockEvidence = verifyAuthorityLease({
    repoRoot,
    leasePath: args.lockLeasePath,
    expectedNonce: args.lockNonce,
    expectedRunId: args.runId,
    maxAgeMinutes: policy.report_freshness_max_minutes,
  });
  if (args.authority && !initialLockEvidence.verified) {
    throw new Error(
      `authority mode requires a child-verifiable live benchmark-lock lease: ${initialLockEvidence.issues.join("; ")}`,
    );
  }
  const attestationPublication = readBoundedFilePublication({
    repoRoot,
    candidatePath: args.buildAttestationPath,
    expectedPrefix: "output/qa/frontend-interaction-latency/",
  });
  const buildAttestationPath = attestationPublication.path;
  const attestationBytes = attestationPublication.bytes;
  const buildAttestation = JSON.parse(attestationBytes.toString("utf8"));
  const buildValidation = validateBuildAttestation({
    attestation: buildAttestation,
    webRoot,
    maxAgeMinutes: policy.build_attestation_max_age_minutes,
  });
  if (buildValidation.status !== "passed") {
    throw new Error(
      `build attestation failed: ${buildValidation.issues.join("; ")}`,
    );
  }
  const distSnapshot = validateImmutableDistSnapshot({
    repoRoot,
    snapshotRoot: args.distSnapshotPath,
    expectedDistSha256: buildAttestation.dist.sha256,
  });
  const measurementHarnessSnapshot = computeCanonicalFileSetSnapshot(
    repoRoot,
    measurementHarnessPaths,
  );

  const fixturePublication = readStableSingleLinkFile(
    fixturePath,
    "interaction-latency fixture",
  );
  const fixtureBytes = fixturePublication.bytes;
  const fixture = JSON.parse(fixtureBytes.toString("utf8"));
  const detailCount =
    policy.warmup_samples_per_case + policy.measured_samples_per_case;
  const detailFixture = buildPathDistinctDetailFixture(fixture, detailCount);
  const commandResults = {
    list_run_artifacts: detailFixture.summaries,
    load_run_artifact_detail: {
      __mockByArgument: {
        name: "path",
        values: detailFixture.detailsByPath,
      },
    },
  };
  const commandDelays = {
    load_run_artifact_detail: policy.detail_mock_delay_ms,
  };

  const runtimeIssues = [];
  const pageErrors = [];
  const consoleErrors = [];
  const measurements = Object.fromEntries(
    policy.cases.map((entry) => [
      entry.id,
      { warmup_samples: [], measured_samples: [] },
    ]),
  );
  let server = null;
  let browser = null;

  try {
    const serverInfo = await startStaticServerFromSnapshot(
      distSnapshot.entries,
    );
    server = serverInfo.server;
    browser = await chromium.launch({ headless: true });
    const context = await browser.newContext({
      viewport: policy.viewport,
      deviceScaleFactor: 1,
      locale: "en-US",
      timezoneId: "UTC",
      reducedMotion: "reduce",
    });
    await seedAdvancedUiMode(context);
    await installMockedTauri(context, {
      holdStartupOverlay: true,
      strictUnknownInvoke: true,
      commandResults,
      commandDelays,
    });
    const page = await context.newPage();
    page.on("pageerror", (error) => pageErrors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") consoleErrors.push(message.text());
    });
    await page.goto(serverInfo.url, { waitUntil: "networkidle" });
    await dismissStartupOverlayIfPresent(page);
    await page.waitForSelector('[data-qa-shell="workbench"]', {
      state: "visible",
    });
    await page.waitForSelector(
      '[data-qa-action="shell.workspace.workflow"][aria-selected="true"]',
      { state: "visible" },
    );
    await prepareSavedRunList(page, policy.dom_quiet_window_ms, detailCount);

    const runners = {
      "saved-run-detail-open": (absoluteIndex) =>
        measureSavedRunDetail(
          page,
          policy,
          detailFixture.identities[absoluteIndex],
        ),
      "workspace-navigation": () => measureWorkspaceNavigation(page, policy),
    };
    for (const casePolicy of policy.cases) {
      const runner = runners[casePolicy.id];
      if (!runner) throw new Error(`no runner registered for ${casePolicy.id}`);
      for (let index = 0; index < policy.warmup_samples_per_case; index += 1) {
        const observation = await runner(index);
        measurements[casePolicy.id].warmup_samples.push({
          ...observation,
          phase: "warmup",
          sequence: index,
        });
      }
      for (
        let index = 0;
        index < policy.measured_samples_per_case;
        index += 1
      ) {
        const observation = await runner(
          index + policy.warmup_samples_per_case,
        );
        measurements[casePolicy.id].measured_samples.push({
          ...observation,
          phase: "measured",
          sequence: index,
        });
      }
    }

    runtimeIssues.push(
      ...(await validateDetailHydrationRunEvidence(
        page,
        detailFixture.identities,
        policy.detail_mock_delay_ms,
      )),
    );
    if (pageErrors.length > 0) {
      runtimeIssues.push(...pageErrors.map((error) => `page error: ${error}`));
    }
    if (consoleErrors.length > 0) {
      runtimeIssues.push(
        ...consoleErrors.map((error) => `console error: ${error}`),
      );
    }
    if (
      JSON.stringify(
        computeCanonicalFileSetSnapshot(repoRoot, measurementHarnessPaths),
      ) !== JSON.stringify(measurementHarnessSnapshot)
    ) {
      runtimeIssues.push(
        "measurement harness sources changed during the authority run",
      );
    }
    try {
      const finalPolicy = readStableSingleLinkFile(
        policyPath,
        "interaction-latency policy",
      );
      if (finalPolicy.bytes.compare(policyBytes) !== 0) {
        runtimeIssues.push(
          "interaction policy changed during the authority run",
        );
      }
    } catch (error) {
      runtimeIssues.push(
        `interaction policy revalidation: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
    try {
      const finalFixture = readStableSingleLinkFile(
        fixturePath,
        "interaction-latency fixture",
      );
      if (finalFixture.bytes.compare(fixtureBytes) !== 0) {
        runtimeIssues.push("fixture source changed during the authority run");
      }
    } catch (error) {
      runtimeIssues.push(
        `fixture source revalidation: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
    try {
      readBoundedFilePublication({
        repoRoot,
        candidatePath: buildAttestationPath,
        expectedSha256: attestationPublication.sha256,
      });
    } catch (error) {
      runtimeIssues.push(
        `build attestation publication: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }
    const finalBuildValidation = validateBuildAttestation({
      attestation: buildAttestation,
      webRoot,
      maxAgeMinutes: policy.build_attestation_max_age_minutes,
    });
    if (finalBuildValidation.status !== "passed") {
      runtimeIssues.push(
        ...finalBuildValidation.issues.map(
          (issue) => `final build attestation: ${issue}`,
        ),
      );
    }
    try {
      const finalDistSnapshot = validateImmutableDistSnapshot({
        repoRoot,
        snapshotRoot: args.distSnapshotPath,
        expectedDistSha256: buildAttestation.dist.sha256,
      });
      if (
        finalDistSnapshot.manifest_sha256 !== distSnapshot.manifest_sha256 ||
        finalDistSnapshot.snapshot.sha256 !== distSnapshot.snapshot.sha256
      ) {
        runtimeIssues.push(
          "immutable served distribution identity changed during the authority run",
        );
      }
    } catch (error) {
      runtimeIssues.push(
        `final immutable distribution snapshot: ${
          error instanceof Error ? error.message : String(error)
        }`,
      );
    }

    const browserVersion = browser.version();
    const userAgent = await page.evaluate(() => navigator.userAgent);
    let scopedWorktreeState = null;
    try {
      scopedWorktreeState = computeAuthorityScopedWorktreeState(repoRoot);
    } catch (error) {
      runtimeIssues.push(
        error instanceof Error ? error.message : String(error),
      );
    }
    const finalLockEvidence = verifyAuthorityLease({
      repoRoot,
      leasePath: args.lockLeasePath,
      expectedNonce: args.lockNonce,
      expectedRunId: args.runId,
      maxAgeMinutes: policy.report_freshness_max_minutes,
    });
    if (!finalLockEvidence.verified) {
      runtimeIssues.push(
        ...finalLockEvidence.issues.map(
          (issue) => `final authority lease: ${issue}`,
        ),
      );
    }
    const lockEvidence = {
      ...finalLockEvidence,
      verified: initialLockEvidence.verified && finalLockEvidence.verified,
      verified_at_start: initialLockEvidence.verified,
      verified_before_publication: finalLockEvidence.verified,
      initial_lease_sha256: initialLockEvidence.lease_sha256,
    };
    const report = buildLatencyReport({
      policy,
      policyPath: path.relative(repoRoot, policyPath).replaceAll("\\", "/"),
      policySha256: sha256Bytes(policyBytes),
      authorityRequested: args.authority,
      run: {
        id: args.runId,
        started_at: args.runStartedAt,
      },
      source: {
        git_commit: gitOutput(["rev-parse", "HEAD"]) || null,
        scoped_worktree_clean: scopedWorktreeState?.clean ?? null,
        scoped_worktree_status_format: scopedWorktreeState?.format ?? null,
        scoped_worktree_paths: scopedWorktreeState?.paths ?? null,
        scoped_worktree_status_sha256: scopedWorktreeState?.sha256 ?? null,
        scoped_worktree_status_bytes: scopedWorktreeState?.byte_count ?? null,
        measurement_profile: policy.profile,
        mock_profile: policy.mock_profile,
        fixture_profile: policy.fixture_profile,
        fixture_path: path
          .relative(repoRoot, fixturePath)
          .replaceAll("\\", "/"),
        fixture_sha256: fixturePublication ? sha256Bytes(fixtureBytes) : null,
        effective_fixture_sha256: sha256Bytes(
          Buffer.from(JSON.stringify(detailFixture)),
        ),
        detail_fixture_count: detailFixture.identities.length,
        benchmark_lock: lockEvidence,
        measurement_harness_paths: measurementHarnessPaths,
        measurement_harness_sha256: measurementHarnessSnapshot.sha256,
        measurement_harness_path_manifest_sha256:
          measurementHarnessSnapshot.path_manifest_sha256,
        measurement_harness_file_count: measurementHarnessSnapshot.file_count,
        measurement_harness_total_bytes: measurementHarnessSnapshot.total_bytes,
        measurement_harness_hash_framing: FILE_SET_HASH_FRAMING,
        build_attestation_path: path
          .relative(repoRoot, buildAttestationPath)
          .replaceAll("\\", "/"),
        build_attestation_sha256: attestationPublication.sha256,
        build_attestation_verified: true,
        build_attestation_kind: buildAttestation.attestation_kind,
        build_attestation_generated_at: buildAttestation.generated_at,
        build_profile: buildAttestation.build_profile,
        attested_node_version: buildAttestation.toolchain.node_version,
        build_input_sha256: buildValidation.current_source_inputs.sha256,
        attested_build_input_sha256: buildAttestation.source_inputs.sha256,
        dist_sha256: distSnapshot.snapshot.sha256,
        attested_dist_sha256: buildAttestation.dist.sha256,
        served_dist_snapshot_path: distSnapshot.snapshot_root,
        served_dist_snapshot_manifest_path: distSnapshot.manifest_path,
        served_dist_snapshot_manifest_sha256: distSnapshot.manifest_sha256,
        served_dist_snapshot_sha256: distSnapshot.snapshot.sha256,
        served_dist_snapshot_path_manifest_sha256:
          distSnapshot.snapshot.path_manifest_sha256,
        served_dist_snapshot_file_count: distSnapshot.snapshot.file_count,
        served_dist_snapshot_total_bytes: distSnapshot.snapshot.total_bytes,
        served_from_memory_snapshot: true,
        server_snapshot_entry_count: serverInfo.served_entry_count,
        server_snapshot_total_bytes: serverInfo.served_total_bytes,
      },
      environment: {
        os_platform: process.platform,
        os_release: os.release(),
        cpu_model: os.cpus()[0]?.model ?? null,
        logical_cpu_count: os.cpus().length,
        node_version: process.version,
        browser_name: "chromium",
        browser_version: browserVersion,
        user_agent: userAgent,
        headless: true,
        device_scale_factor: 1,
        locale: "en-US",
        timezone: "UTC",
        reduced_motion: "reduce",
      },
      measurements,
      runtimeIssues,
    });

    const publication = atomicWriteJson(outPath, report, { repoRoot });
    const relativeOut = path.relative(repoRoot, outPath).replaceAll("\\", "/");
    console.log(`Scoped frontend interaction latency: ${report.status}`);
    for (const row of report.cases) {
      console.log(
        `- ${row.id}: p95=${row.stats.p95_ms}ms max=${row.stats.max_ms}ms ` +
          `(budgets ${row.budgets.p95_ms}/${row.budgets.max_ms}ms)`,
      );
    }
    console.log(`Evidence: ${relativeOut}`);
    console.log(`Evidence-SHA256: ${publication.sha256}`);
    if (args.authority && report.status !== "passed") process.exitCode = 1;
  } finally {
    try {
      await browser?.close();
    } finally {
      await closeStaticSnapshotServer(server);
    }
  }
}

main().catch((error) => {
  console.error(
    error instanceof Error ? error.stack || error.message : String(error),
  );
  process.exitCode = 1;
});
