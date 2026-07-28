import { expect, type Page } from "@playwright/test";
import {
  candidateDetailReadyScenario,
  aggregateMetaReadyScenario,
  campaignBatchReadyScenario,
  compositeRouteSummaryReadyScenario,
  dataLoadFailureScenario,
  dataLoadSuccessScenario,
  gpuStatusFailureScenario,
  liveSafetyReadyScenario,
  liveStreamStopReadyScenario,
  mlSignalBundleStatusFailureScenario,
  mlSignalDatasetExportFailureScenario,
  mlSignalPolicySweepFailureScenario,
  mlSignalScoreCacheFailureScenario,
  mlSignalUnavailableScenario,
  mlSignalValidationFailureScenario,
  optimizationFailureScenario,
  optimizationSuccessScenario,
  optionsValidationInputFailureScenario,
  optionsValidationPreviewFailureScenario,
  optionsValidationReadyScenario,
  savedRunOpenScenario,
  savedRunRepairScenario,
  savedRunRepairWriteScenario,
  settingsArtifactCatalogFailureScenario,
  settingsLocalDataInventoryFailureScenario,
  settingsStorageDiagnosticsScenario,
  taskCenterProgressReadyScenario,
  validationDatasetMismatchScenario,
  validationRejectedScenario,
  validationReviewScenario,
  validationSuccessScenario,
} from "../fixtures";
import {
  defineWorkflow,
  readTauriInvokeLog,
  type WorkflowContext,
} from "../support/workflowRunner";
import {
  expectNoHorizontalDocumentOverflow,
  expectNoStuckBusyState,
} from "../support/qaAssertions";

type ActionLookup = WorkflowContext["action"];

async function loadFixtureSampleData(
  page: import("@playwright/test").Page,
  action: ActionLookup,
): Promise<void> {
  await action("shell.workflowStep.data").click();
  await action("data.sample.load").first().click();
  await expect
    .poll(async () =>
      (await readTauriInvokeLog(page)).some(
        (entry) => entry.command === "load_price_data",
      ),
    )
    .toBe(true);
}

async function openMlSignalPanel(action: ActionLookup): Promise<void> {
  await action("shell.mode.mlSignal").click();
  await action("shell.workflowStep.optimize").click();
  await expect(action("mlSignal.bundles.refresh")).toBeVisible();
}

async function expectFailedTaskRawError(
  page: Page,
  action: ActionLookup,
  expected: RegExp,
): Promise<void> {
  await action("shell.taskCenter.open").first().click();
  const taskCenter = page.getByRole("dialog", { name: "Task center" });
  await expect(taskCenter).toBeVisible();
  const details = taskCenter.locator(".taskErrorDetails").filter({ hasText: expected }).first();
  await expect(details).toBeVisible();
  await details.locator("summary").click();
  await expect(details.locator(".taskErrorText")).toContainText(expected);
}

async function openSavedOptionsArtifact(
  page: import("@playwright/test").Page,
  action: ActionLookup,
): Promise<void> {
  await action("shell.workspace.library").click();
  await action("library.savedRun.refresh").click();
  await expect(
    page.getByRole("button", { name: /DoubleMa/i }).first(),
  ).toBeVisible();
  await page
    .getByRole("button", { name: /DoubleMa/i })
    .first()
    .click();

  await expect(
    page.locator('[aria-label="Options expression follow-up"]'),
  ).toBeVisible();
  await expect(
    page.getByText(/Saved options expression artifacts/i),
  ).toBeVisible();
  await action("options.artifacts.refresh").click();
  await expect(action("options.artifacts.load")).toBeEnabled();
  await action("options.artifacts.load").click();
  await expect(
    page.getByText(/Loaded saved options expression artifact/i).first(),
  ).toBeVisible();
}

defineWorkflow({
  id: "data.sample.load.success",
  title: "loads sample data and projects dataset readiness",
  tags: ["@workflow", "@regression"],
  scenario: dataLoadSuccessScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);

    await expect(page.getByText(/Loaded \(binary cache\)/i).first()).toBeVisible();
    await expect(page.getByText(/Loaded ID:\s*fixture_aapl_1m/i).first()).toBeVisible();
    await action("shell.workflowStep.optimize").click();
    await expect(action("optimize.run")).toBeEnabled();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "validation.success.selected",
  title:
    "runs conventional validation successfully from a loaded matching dataset",
  tags: ["@workflow", "@regression"],
  scenario: validationSuccessScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.data").click();
    await action("data.sample.load").first().click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_price_data",
        ),
      )
      .toBe(true);

    await action("shell.workflowStep.validate").click();
    await expect(action("validation.run")).toBeEnabled();
    await action("validation.run").click();

    await expect(
      page.getByText(/Validation completed in/i).first(),
    ).toBeVisible();
    await expect(
      page.locator(".validationPanelVueRoot").getByText(/Validation complete/i).first(),
    ).toBeVisible();
    await expect(
      page.locator(".validationTableVue tbody tr").first(),
    ).toBeVisible();
    const robustHeader = page
      .locator(".validationTableVue th", {
        has: page.getByRole("button", { name: /^Robust$/ }),
      })
      .first();
    await robustHeader.getByRole("button", { name: /^Robust$/ }).click();
    await expect(robustHeader).toHaveAttribute("aria-sort", "ascending");
    await robustHeader
      .getByRole("button", { name: /^Resize Robust column$/ })
      .focus();
    await page.keyboard.press("ArrowRight");
    await expect
      .poll(async () =>
        page.evaluate(
          () =>
            window.localStorage.getItem(
              "vta_validation_results_table_state_v1",
            ) || "",
        ),
      )
      .toContain('"robust":144');
    const validationTableShell = page.locator(
      '[aria-label="Validation results table shell"]',
    );
    await expect(validationTableShell).toHaveAttribute(
      "data-table-density",
      "comfortable",
    );
    await page
      .getByRole("group", { name: "Validation density" })
      .getByRole("button", { name: "compact" })
      .click();
    await expect(validationTableShell).toHaveAttribute(
      "data-table-density",
      "compact",
    );
    await page.locator('[data-qa="validation.table.view.gates"]').click();
    await expect(
      page.locator('[data-qa-panel="validation.rowDetail"]'),
    ).toContainText(/Row 1 of 1/);
    await expect(
      page.locator('[data-qa-state="validation.rowDetail.gate"]'),
    ).toBeVisible();
    await expect(action("validation.export.shortlistCsv")).toBeEnabled();
    await expect(action("validation.export.validationJson")).toBeEnabled();
    await expect(action("validation.export.reportJson")).toBeEnabled();
    await expect(action("validation.export.reportXlsx")).toBeEnabled();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "validation.review.evidence",
  title: "surfaces validation review evidence without hiding exports",
  tags: ["@workflow", "@regression"],
  scenario: validationReviewScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);

    await action("shell.workflowStep.validate").click();
    await expect(action("validation.run")).toBeEnabled();
    await action("validation.run").click();

    await expect(
      page.locator('[data-qa-panel="validation.overfitEvidence"]'),
    ).toHaveAttribute("data-qa-state", "attention");
    await expect(
      page.locator('[data-qa-panel="validation.overfitEvidence"]'),
    ).toContainText(/Needs review/i);
    await expect(page.getByText(/rec=approved_with_warnings/i).first()).toBeVisible();
    await expect(
      page.locator(".validationTableVue tbody tr").first(),
    ).toBeVisible();
    await expect(action("validation.export.reportJson")).toBeEnabled();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "validation.rejected.evidence",
  title: "surfaces rejected validation evidence without losing row inspection",
  tags: ["@workflow", "@regression"],
  scenario: validationRejectedScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);

    await action("shell.workflowStep.validate").click();
    await expect(action("validation.run")).toBeEnabled();
    await action("validation.run").click();

    await expect(
      page.locator('[data-qa-panel="validation.overfitEvidence"]'),
    ).toHaveAttribute("data-qa-state", "attention");
    await expect(page.getByText(/rec=rejected/i).first()).toBeVisible();
    await expect(page.getByText(/approved=0/i).first()).toBeVisible();
    await expect(
      page.locator(".validationTableVue tbody tr").first(),
    ).toBeVisible();
    await page.locator('[data-qa="validation.table.view.gates"]').click();
    await expect(
      page.locator('[data-qa-panel="validation.rowDetail"]'),
    ).toContainText(/Row 1 of 1/);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "optimize.loaded.success",
  title: "runs optimization successfully after loading a fixture dataset",
  tags: ["@workflow", "@regression"],
  scenario: optimizationSuccessScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.data").click();
    await action("data.sample.load").first().click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_price_data",
        ),
      )
      .toBe(true);

    await action("shell.workflowStep.optimize").click();
    await expect(action("optimize.run")).toBeEnabled();
    await action("optimize.run").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "run_double_ma_optimization",
        ),
      )
      .toBe(true);

    await action("shell.resultsTab.top").click();
    await expect(page.locator("#results_panel_top")).toBeVisible();
    await expect(page.locator('.topTableVue [data-top-row="1"]').first()).toBeVisible();
    await expect(action("top.selected.openCandidateDetail")).toBeEnabled();
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "optimize.loaded.failure",
  title: "surfaces optimization command failure without losing loaded data state",
  tags: ["@workflow", "@regression"],
  scenario: optimizationFailureScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.data").click();
    await action("data.sample.load").first().click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_price_data",
        ),
      )
      .toBe(true);

    await action("shell.workflowStep.optimize").click();
    await expect(action("optimize.run")).toBeEnabled();
    await action("optimize.run").click();
    await expect(
      page.getByText(/Optimization failed:.*Optimization fixture failed/i).first(),
    ).toBeVisible();
    await expect(action("optimize.run")).toBeEnabled();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "optimize.composite.routeSummary",
  title: "shows composite profile route summaries for CPU, hybrid, and kernel paths",
  tags: ["@workflow", "@regression"],
  scenario: compositeRouteSummaryReadyScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.optimize").click();
    const summary = page.getByLabel("Optimization summary");
    const strategySelect = page.locator("#vueStrategyKind");
    const backendSelect = page.locator("#vueBackend");

    await expect(strategySelect).toBeVisible();
    await strategySelect.selectOption("RegimeHybrid");
    await backendSelect.selectOption("GpuKernel");
    await expect(summary).toContainText(/Route: GPU hybrid/i);
    await expect(
      page.getByText(/Composite route preview: current inputs resolve to the hybrid GPU route/i).first(),
    ).toBeVisible();
    await expect(
      page.getByText(/exact-kernel shortlist controls stay disabled on hybrid execution/i).first(),
    ).toBeVisible();

    await strategySelect.selectOption("BreakoutMomentum");
    await backendSelect.selectOption("GpuKernel");
    await expect(summary).toContainText(/Route: GPU kernel/i);
    await expect(
      page.getByText(/Composite route preview: current inputs resolve to the GPU kernel route/i).first(),
    ).toBeVisible();
    await expect(
      page.getByText(/Export-all and exact-kernel shortlist controls stay available on this route/i).first(),
    ).toBeVisible();

    await strategySelect.selectOption("CompositeTrend");
    await backendSelect.selectOption("CpuOnly");
    await expect(summary).toContainText(/Route: CPU/i);
    await expect(
      page.getByText(/Composite route preview: current inputs resolve to the CPU route/i).first(),
    ).toBeVisible();

    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "taskCenter.progress.lifecycle",
  title: "shows running optimization progress and completed outcome in Task Center",
  tags: ["@workflow", "@regression"],
  scenario: taskCenterProgressReadyScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.data").click();
    await action("data.sample.load").first().click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_price_data",
        ),
      )
      .toBe(true);

    await action("shell.workflowStep.optimize").click();
    await expect(action("optimize.run")).toBeEnabled();
    await action("optimize.run").click();
    await action("shell.taskCenter.open").click();

    const taskCenter = page.getByRole("dialog", { name: "Task center" });
    const activePosture = taskCenter.locator(
      '[aria-label="Task center active posture"]',
    );
    const recentOutcomes = taskCenter.locator(
      '[aria-label="Task center recent outcomes"]',
    );
    const events = taskCenter.locator('[aria-label="Task center events"]');
    const timeline = taskCenter.locator('[aria-label="Task center timeline"]');

    await expect(taskCenter).toBeVisible();
    await expect(activePosture).toContainText(/Double Ma Optimization/i);
    await expect(activePosture).toContainText(/Running/i);
    await expect(activePosture.locator(".bar.thin .fill").first()).toBeVisible();
    await expect(activePosture).toContainText(/scoring candidates|finalizing|building grid/i);

    await expect(recentOutcomes).toContainText(/Double Ma Optimization/i, {
      timeout: 3_000,
    });
    await expect(recentOutcomes).toContainText(/Completed/i);
    await expect(events).toContainText(/completed|Phase:/i);
    await expect(timeline).toContainText(/completed|Phase changed/i);
    await page.keyboard.press("Escape");
    await expect(taskCenter).toBeHidden();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "aggregate.meta.runOpenReveal",
  title: "runs Meta Selection and verifies report, reveal, saved replay, and palette actions",
  tags: ["@workflow", "@regression"],
  scenario: aggregateMetaReadyScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.library").click();
    await action("library.savedRun.refresh").click();
    await expect(
      page.getByRole("button", { name: /CompositeMo/i }).first(),
    ).toBeVisible();

    await action("shell.workspace.aggregate").click();
    await expect(page.locator("#results_panel_meta")).toBeVisible();
    await expect(page.locator("#metaAggregateSurface")).toBeVisible();
    await expect(action("aggregate.meta.run")).toBeEnabled();

    await action("aggregate.meta.run").click();
    await expect(page.locator(".metaTableVue tbody tr").first()).toBeVisible();
    await expect(page.getByText(/groups=1/i).first()).toBeVisible();
    await expect(action("aggregate.ml.refresh")).toBeEnabled();
    await action("aggregate.ml.refresh").click();
    await expect(
      page.locator('[role="status"]').filter({
        hasText: /Refreshed ML confidence status/i,
      }).last(),
    ).toBeVisible();
    await expect(action("aggregate.ml.train")).toBeEnabled();
    await expect(action("aggregate.ml.apply")).toBeEnabled();
    await expect(action("aggregate.ml.delete")).toBeEnabled();
    await expect(action("aggregate.stage5.run")).toBeEnabled();
    await expect(action("aggregate.policy.evaluate")).toBeEnabled();
    await expect(action("aggregate.stage5.reveal")).toBeDisabled();

    await expect(action("aggregate.meta.openReport")).toBeEnabled();
    const openPathBefore = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "open_path",
    ).length;
    await action("aggregate.meta.openReport").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "open_path",
        ).length,
      )
      .toBeGreaterThan(openPathBefore);

    await expect(action("aggregate.meta.revealFiles")).toBeEnabled();
    const revealPathBefore = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "reveal_path",
    ).length;
    await action("aggregate.meta.revealFiles").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "reveal_path",
        ).length,
      )
      .toBeGreaterThan(revealPathBefore);

    await action("aggregate.tab.saved").click();
    await expect(page.locator("#vueMetaSavedSelect")).toBeVisible();
    await page.locator("#vueMetaSavedTags").fill("qa, reviewed");
    const updateTagsBefore = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "update_artifact_tags",
    ).length;
    await action("aggregate.saved.saveTags").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "update_artifact_tags",
        ).length,
      )
      .toBeGreaterThan(updateTagsBefore);
    await expect(action("aggregate.saved.load")).toBeEnabled();
    const loadMetaReplayBefore = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "load_meta_artifact_replay",
    ).length;
    await action("aggregate.saved.load").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "load_meta_artifact_replay",
        ).length,
      )
      .toBeGreaterThan(loadMetaReplayBefore);

    const paletteOpenPathBefore = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "open_path",
    ).length;
    await action("shell.commandPalette.open").click();
    await expect(page.locator(".cmdPaletteDialog")).toBeVisible();
    await page.getByLabel("Command search").fill("meta report");
    await expect(action("commandPalette.command.openLatestMetaReport")).toBeVisible();
    await action("commandPalette.command.openLatestMetaReport").click();
    await expect(page.locator(".cmdPaletteDialog")).toBeHidden();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "open_path",
        ).length,
      )
      .toBeGreaterThan(paletteOpenPathBefore);

    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).toContain("run_meta_selection");
    expect(invokes).toContain("load_meta_artifact_replay");
    expect(invokes).toContain("reveal_path");
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "aggregate.batch.campaignLifecycle",
  title: "runs aggregate batch campaign controls with refresh, stop, retry, resume, and reveal coverage",
  tags: ["@workflow", "@regression"],
  scenario: campaignBatchReadyScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);

    await action("shell.workspace.library").click();
    await action("library.savedRun.refresh").click();
    await expect(
      page.getByRole("button", { name: /DoubleMa/i }).first(),
    ).toBeVisible();

    await action("shell.workspace.aggregate").click();
    await action("aggregate.tab.batch").click();
    const batchPanel = page.locator("#meta_aggregate_panel_batch");
    await expect(batchPanel).toBeVisible();

    await action("aggregate.batch.refresh").click();
    const datasetGroup = page.getByRole("group", {
      name: "Batch generation datasets",
    });
    await expect(datasetGroup).toContainText(/AAPL 1m/i);
    await expect(datasetGroup).toContainText(/MSFT 1m/i);

    await expect(action("aggregate.batch.selectAll")).toBeEnabled();
    await action("aggregate.batch.selectAll").click();
    await expect(page.getByText(/Selected 2\/2/i).first()).toBeVisible();

    await expect(action("aggregate.batch.selectNone")).toBeEnabled();
    await action("aggregate.batch.selectNone").click();
    await expect(page.getByText(/Selected 0\/2/i).first()).toBeVisible();

    await action("aggregate.batch.selectAll").click();
    await expect(page.getByText(/Selected 2\/2/i).first()).toBeVisible();

    await expect(action("aggregate.batch.run")).toBeEnabled();
    const firstRunCount = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "run_research_campaign",
    ).length;
    await action("aggregate.batch.run").click();
    await expect(action("aggregate.batch.stop")).toBeEnabled();
    await action("aggregate.batch.stop").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "cancel_current_run",
        ),
      )
      .toBe(true);

    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "run_research_campaign",
        ).length,
      )
      .toBeGreaterThan(firstRunCount);
    const campaignDoneStatus = page.locator('[role="status"]').filter({
      hasText: /Campaign done: validated=1\/2/i,
    }).last();
    await expect(campaignDoneStatus).toBeVisible();
    await expect(campaignDoneStatus).toContainText(/failed=1/i);

    await expect(action("aggregate.batch.revealSummary")).toBeEnabled();
    const revealBefore = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "reveal_path",
    ).length;
    await action("aggregate.batch.revealSummary").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "reveal_path",
        ).length,
      )
      .toBeGreaterThan(revealBefore);

    await expect(action("aggregate.batch.retryFailed")).toBeEnabled();
    const retryRunCount = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "run_research_campaign",
    ).length;
    await action("aggregate.batch.retryFailed").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "run_research_campaign",
        ).length,
      )
      .toBeGreaterThan(retryRunCount);

    await expect(action("aggregate.batch.resumeIncomplete")).toBeEnabled({
      timeout: 3_000,
    });
    const resumeRunCount = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "run_research_campaign",
    ).length;
    await action("aggregate.batch.resumeIncomplete").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).filter(
          (entry) => entry.command === "run_research_campaign",
        ).length,
      )
      .toBeGreaterThan(resumeRunCount);

    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).toContain("polygon_get_cache_info");
    expect(invokes).toContain("list_repo_data_csvs");
    expect(invokes).toContain("run_research_campaign");
    expect(invokes).toContain("cancel_current_run");
    expect(invokes).toContain("reveal_path");
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "settings.diagnostics.storage",
  title: "reports settings storage diagnostics and refreshes local summaries",
  tags: ["@workflow", "@regression"],
  scenario: settingsStorageDiagnosticsScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.settings").click();
    await expect(
      page.getByRole("region", { name: "Desktop settings diagnostics" }),
    ).toBeVisible();

    await action("settings.artifactCatalog.refresh").click();
    await expect(page.getByText(/3 total · runs 1 · meta 1/i)).toBeVisible();

    await action("settings.artifactCatalog.rebuild").click();
    await expect(page.getByText(/Rebuilt artifact catalog for 3 artifacts/i)).toBeVisible();

    await action("settings.localDataInventory.refresh").click();
    await expect(page.getByText(/2 Polygon datasets · 3 artifacts/i)).toBeVisible();
    await expect(page.getByText(/keyring storage degraded/i)).toBeVisible();
    await expect(
      page.getByText(/Polygon credential storage is degraded/i),
    ).toBeVisible();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "settings.artifactCatalog.failure",
  title: "surfaces settings artifact catalog refresh failures",
  tags: ["@workflow", "@regression"],
  scenario: settingsArtifactCatalogFailureScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.settings").click();
    await action("settings.artifactCatalog.refresh").click();
    await expect(
      page.getByText(/list_artifact_library.*Artifact catalog fixture failure/i).first(),
    ).toBeVisible();
    await expect(action("settings.artifactCatalog.refresh")).toBeEnabled();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "settings.localDataInventory.failure",
  title: "surfaces settings local data inventory failures",
  tags: ["@workflow", "@regression"],
  scenario: settingsLocalDataInventoryFailureScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.settings").click();
    await action("settings.localDataInventory.refresh").click();
    await expect(
      page.getByText(/polygon_get_cache_info.*Polygon cache inventory fixture failure/i).first(),
    ).toBeVisible();
    await expect(action("settings.localDataInventory.refresh")).toBeEnabled();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "mlSignal.unavailable.emptyBundle",
  title: "keeps ML Signal mode inspectable with no model bundles",
  tags: ["@workflow", "@regression"],
  scenario: mlSignalUnavailableScenario,
  run: async ({ page, action }) => {
    await openMlSignalPanel(action);
    await action("mlSignal.bundles.refresh").click();
    await expect(page.locator("#mlSignalBundleId option")).toHaveCount(1);
    await expect(page.getByText(/Model bundle: optional/i)).toBeVisible();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "mlSignal.bundleStatus.failure",
  title: "surfaces ML Signal bundle status failures",
  tags: ["@workflow", "@regression"],
  scenario: mlSignalBundleStatusFailureScenario,
  run: async ({ page, action }) => {
    await openMlSignalPanel(action);
    await action("mlSignal.bundles.refresh").click();
    await expect(
      page.locator('#mlSignalBundleId option[value="qa-bundle"]'),
    ).toHaveCount(1);
    await page.locator("#mlSignalBundleId").selectOption("qa-bundle");
    await action("mlSignal.bundleStatus.refresh").click();
    await expect(
      page.getByText(/ML model bundle status refresh failed.*Bundle status fixture failure/i).first(),
    ).toBeVisible();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "mlSignal.datasetExport.failure",
  title: "surfaces ML Signal dataset export failures",
  tags: ["@workflow", "@regression"],
  scenario: mlSignalDatasetExportFailureScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);
    await openMlSignalPanel(action);
    await action("shell.workflowStep.data").click();
    await page.locator("#mlSignalDatasetExportPath").fill("C:/fixtures/ml_signal_dataset.csv");
    await expect(action("mlSignal.dataset.export")).toBeEnabled();
    await action("mlSignal.dataset.export").click();
    await expectNoStuckBusyState(page);
    await expectFailedTaskRawError(page, action, /ML dataset export fixture failure/i);
  },
});

defineWorkflow({
  id: "mlSignal.scoreCache.failure",
  title: "surfaces ML Signal score-cache generation failures",
  tags: ["@workflow", "@regression"],
  scenario: mlSignalScoreCacheFailureScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);
    await openMlSignalPanel(action);
    await page.locator("#mlSignalScoreInputPath").fill("C:/fixtures/ml_scores.csv");
    await page.locator("#mlSignalScoreCachePath").fill("C:/fixtures/ml_signal_score_cache.json");
    await expect(action("mlSignal.scoreCache.build")).toBeEnabled();
    await action("mlSignal.scoreCache.build").click();
    await expectNoStuckBusyState(page);
    await expectFailedTaskRawError(page, action, /ML score cache fixture failure/i);
  },
});

defineWorkflow({
  id: "mlSignal.validation.failure",
  title: "surfaces ML Signal validation failures",
  tags: ["@workflow", "@regression"],
  scenario: mlSignalValidationFailureScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);
    await openMlSignalPanel(action);
    await page.locator("#mlSignalScoreCachePath").fill("C:/fixtures/ml_signal_score_cache.json");
    await action("shell.workflowStep.validate").click();
    await expect(action("mlSignal.validation.run")).toBeEnabled();
    await action("mlSignal.validation.run").click();
    await expectNoStuckBusyState(page);
    await expectFailedTaskRawError(page, action, /ML validation fixture failure/i);
  },
});

defineWorkflow({
  id: "mlSignal.policySweep.failure",
  title: "surfaces ML Signal policy-sweep failures",
  tags: ["@workflow", "@regression"],
  scenario: mlSignalPolicySweepFailureScenario,
  run: async ({ page, action }) => {
    await loadFixtureSampleData(page, action);
    await openMlSignalPanel(action);
    await page.locator("#mlSignalScoreCachePath").fill("C:/fixtures/ml_signal_score_cache.json");
    await action("shell.workflowStep.analyze").click();
    await expect(action("mlSignal.policySweep.run")).toBeEnabled();
    await action("mlSignal.policySweep.run").click();
    await expectNoStuckBusyState(page);
    await expectFailedTaskRawError(page, action, /ML policy sweep fixture failure/i);
  },
});

defineWorkflow({
  id: "results.candidateDetail",
  title: "opens candidate detail from a selected Top result",
  tags: ["@workflow", "@regression"],
  scenario: candidateDetailReadyScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.data").click();
    await action("data.sample.load").first().click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_price_data",
        ),
      )
      .toBe(true);

    await action("shell.workflowStep.optimize").click();
    await expect(action("optimize.run")).toBeEnabled();
    await action("optimize.run").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "run_double_ma_optimization",
        ),
      )
      .toBe(true);

    await action("shell.resultsTab.top").click();
    await expect(action("top.selected.openCandidateDetail")).toBeEnabled();
    await action("top.selected.openCandidateDetail").click();
    await expect(page.locator("#results_panel_drill")).toBeVisible();
    await expect(action("candidateDetail.reviewTop").first()).toBeVisible();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "library.openRun",
  title: "loads, applies, and reruns a saved run from the Library panel",
  tags: ["@workflow", "@regression"],
  scenario: savedRunOpenScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.library").click();
    await action("library.savedRun.refresh").click();
    await expect(
      page.getByRole("button", { name: /DoubleMa/i }).first(),
    ).toBeVisible();
    await page
      .getByRole("button", { name: /DoubleMa/i })
      .first()
      .click();

    await expect(action("library.savedRun.load")).toBeEnabled();
    await action("library.savedRun.load").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_run_artifact_hydration",
        ),
      )
      .toBe(true);

    await action("shell.workspace.library").click();
    await expect(action("library.savedRun.apply")).toBeEnabled();
    await action("library.savedRun.apply").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_run_artifact_detail",
        ),
      )
      .toBe(true);

    await action("shell.workspace.library").click();
    await expect(action("library.savedRun.rerun")).toBeEnabled();
    await action("library.savedRun.rerun").click();
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "run_double_ma_optimization",
        ),
      )
      .toBe(true);

    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).toContain("load_run_artifact_hydration");
    expect(invokes).toContain("load_run_artifact_detail");
    expect(invokes).toContain("run_double_ma_optimization");
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "keyboard.core.validation",
  title: "runs the core data-to-validation path with keyboard activation only",
  tags: ["@workflow", "@regression"],
  scenario: validationSuccessScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.data").press("Enter");
    await expect(action("data.sample.load").first()).toBeVisible();
    await action("data.sample.load").first().press("Enter");
    await expect
      .poll(async () =>
        (await readTauriInvokeLog(page)).some(
          (entry) => entry.command === "load_price_data",
        ),
      )
      .toBe(true);

    await action("shell.workflowStep.validate").press("Enter");
    await expect(action("validation.run")).toBeEnabled();
    await action("validation.run").press("Enter");

    await expect(
      page.getByText(/Validation completed in/i).first(),
    ).toBeVisible();
    await expect(
      page.locator(".validationTableVue tbody tr").first(),
    ).toBeVisible();
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "validation.blocked.datasetMismatch",
  title: "blocks validation when the optimization dataset is not loaded",
  tags: ["@workflow", "@regression"],
  scenario: validationDatasetMismatchScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.validate").click();
    await expect(action("validation.run")).toBeVisible();
    await expect(action("validation.run")).toBeDisabled();
    await expect(
      page
        .getByText(/Validation artifact expects dataset .* Load that dataset before running validation\./)
        .first(),
    ).toBeVisible();
    const validationInvokes = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "run_double_ma_validation",
    );
    expect(validationInvokes).toHaveLength(0);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "data.load.failure",
  title:
    "surfaces sample CSV load failure after the user activates the sample action",
  tags: ["@workflow", "@regression"],
  scenario: dataLoadFailureScenario,
  run: async ({ page, action }) => {
    await action("data.sample.load").first().click();
    await expect(
      page.getByText(/Fixture CSV read failed/i).first(),
    ).toBeVisible();
    await expect(action("data.sample.load").first()).toBeEnabled();
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "optimize.gpuStatus.failure",
  title: "surfaces GPU status refresh failure without leaving optimize busy",
  tags: ["@workflow", "@regression"],
  scenario: gpuStatusFailureScenario,
  run: async ({ page, action }) => {
    await action("shell.workflowStep.optimize").click();
    const beforeRefreshCount = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "get_gpu_status",
    ).length;

    await action("optimize.gpu.refreshStatus").first().click();
    await expect(
      page
        .getByText(
          /GPU status refresh failed:.*GPU probe failed in browser QA fixture/i,
        )
        .first(),
    ).toBeVisible();

    const afterRefreshCount = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "get_gpu_status",
    ).length;
    expect(afterRefreshCount).toBeGreaterThan(beforeRefreshCount);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "options.validation.controls",
  title: "validates a saved options expression artifact from the Library panel",
  tags: ["@workflow", "@regression"],
  scenario: optionsValidationReadyScenario,
  run: async ({ page, action }) => {
    await openSavedOptionsArtifact(page, action);
    await expect(action("options.artifacts.revealActive")).toBeEnabled();

    await page.getByLabel("Tuple indexes").fill("0");
    await page.getByLabel("Validation plan").selectOption("date_range_holdout");
    await page.getByLabel("Holdout start").fill("2030-04-25");
    await page.getByLabel("Holdout end").fill("2030-04-26");
    await expect(action("options.validation.preview")).toBeEnabled();
    await expect(action("options.validation.run")).toBeEnabled();

    await action("options.validation.preview").click();
    await expect(page.getByText(/1 selected tuple/i).first()).toBeVisible();
    await expect(
      page
        .locator(".panelMuted.mono", { hasText: /Date-range holdout/i })
        .first(),
    ).toBeVisible();

    await action("options.validation.run").click();
    await expect(
      page.getByText(/Subsequent validation · accepted/i).first(),
    ).toBeVisible();
    await expect(
      page.locator(".gateBadge", { hasText: /Date-range holdout/i }).first(),
    ).toBeVisible();
    await expect(
      page
        .getByText(/Accepted with supported holdout and stress lanes/i)
        .first(),
    ).toBeVisible();
    await expect(action("options.validation.reveal")).toBeEnabled();
    await action("options.validation.reveal").click();
    await expect(
      page
        .getByText(
          /Revealed C:\/runs\/vectorgrid_options_parent_options_validation\.json/i,
        )
        .first(),
    ).toBeVisible();

    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).toContain(
      "analyze_options_expression_validation_feasibility",
    );
    expect(invokes).toContain("validate_options_expression_artifact");
    expect(invokes).toContain("reveal_path");
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "library.savedRun.repairCheck",
  title: "checks saved-run artifact repair from the Library panel",
  tags: ["@workflow", "@regression"],
  scenario: savedRunRepairScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.library").click();
    await action("library.savedRun.refresh").click();
    await expect(
      page.getByRole("button", { name: /DoubleMa/i }).first(),
    ).toBeVisible();
    await page
      .getByRole("button", { name: /DoubleMa/i })
      .first()
      .click();

    await expect(action("library.savedRun.repairCheck")).toBeEnabled();
    await action("library.savedRun.repairCheck").click();
    await page.locator("summary", { hasText: "Artifact repair" }).click();
    await expect(
      page.getByText(/Dry-run found repairable changes/i).first(),
    ).toBeVisible();
    await expect(
      page.getByText(/canonicalized request payload/i).first(),
    ).toBeVisible();

    const repairInvokes = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "repair_run_artifact",
    );
    expect(repairInvokes).toHaveLength(1);
    expect(repairInvokes[0]?.args).toMatchObject({
      path: "C:/runs/vectorgrid_options_parent.json",
      dry_run: true,
    });
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "library.savedRun.repairWrite",
  title: "writes a repaired saved-run artifact copy from the Library panel",
  tags: ["@workflow", "@regression"],
  scenario: savedRunRepairWriteScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.library").click();
    await action("library.savedRun.refresh").click();
    await expect(
      page.getByRole("button", { name: /DoubleMa/i }).first(),
    ).toBeVisible();
    await page
      .getByRole("button", { name: /DoubleMa/i })
      .first()
      .click();

    await expect(action("library.savedRun.repairWrite")).toBeEnabled();
    await action("library.savedRun.repairWrite").click();
    await page.locator("summary", { hasText: "Artifact repair" }).click();
    await expect(
      page.getByText(/Wrote repaired artifact copy/i).first(),
    ).toBeVisible();
    await expect(
      page.getByText(/vectorgrid_options_parent\.repaired\.json/i).first(),
    ).toBeVisible();

    const repairInvokes = (await readTauriInvokeLog(page)).filter(
      (entry) => entry.command === "repair_run_artifact",
    );
    expect(repairInvokes).toHaveLength(1);
    expect(repairInvokes[0]?.args).toMatchObject({
      path: "C:/runs/vectorgrid_options_parent.json",
      dry_run: false,
      write_target: "C:/runs/vectorgrid_options_parent.repaired.json",
    });
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "options.validation.inputFailure",
  title:
    "blocks options validation preview on invalid date-range input before invoking Tauri",
  tags: ["@workflow", "@regression"],
  scenario: optionsValidationInputFailureScenario,
  run: async ({ page, action }) => {
    await openSavedOptionsArtifact(page, action);

    await page.getByLabel("Validation plan").selectOption("date_range_holdout");
    await page.getByLabel("Holdout start").fill("2030-04-26");
    await page.getByLabel("Holdout end").fill("2030-04-25");
    const beforePreviewCount = (await readTauriInvokeLog(page)).filter(
      (entry) =>
        entry.command === "analyze_options_expression_validation_feasibility",
    ).length;

    await expect(action("options.validation.preview")).toBeDisabled();
    await expect(action("options.validation.run")).toBeDisabled();
    await expect(
      page
        .getByText(/Validation start date must be before validation end date/i)
        .first(),
    ).toBeVisible();

    const afterPreviewCount = (await readTauriInvokeLog(page)).filter(
      (entry) =>
        entry.command === "analyze_options_expression_validation_feasibility",
    ).length;
    expect(afterPreviewCount).toBe(beforePreviewCount);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "options.validation.previewFailure",
  title:
    "surfaces options validation feasibility preview failures from mocked Tauri",
  tags: ["@workflow", "@regression"],
  scenario: optionsValidationPreviewFailureScenario,
  run: async ({ page, action }) => {
    await openSavedOptionsArtifact(page, action);

    await page.getByLabel("Tuple indexes").fill("0");
    await page.getByLabel("Validation plan").selectOption("date_range_holdout");
    await page.getByLabel("Holdout start").fill("2030-04-25");
    await page.getByLabel("Holdout end").fill("2030-04-26");
    await action("options.validation.preview").click();

    await expect(
      page.getByText(/Options validation feasibility fixture failed/i).first(),
    ).toBeVisible();
    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).toContain(
      "analyze_options_expression_validation_feasibility",
    );
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "live.stream.stop",
  title: "starts and stops a local-paper live candle stream",
  tags: ["@workflow", "@regression"],
  scenario: liveStreamStopReadyScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.live").click();
    await page.getByLabel("Execution target").selectOption("local_paper");
    await page.getByLabel("Symbol").fill("AAPL");
    await page.getByLabel("Symbol").press("Tab");

    await expect(action("live.stream.start")).toBeEnabled();
    await action("live.stream.start").click();
    await page
      .locator(".confirmationDialog")
      .getByRole("button", { name: /^Start live feed$/ })
      .click();
    await expect(
      page.getByText(/Live candles started for AAPL/i).first(),
    ).toBeVisible();

    await expect(action("live.stream.stop")).toBeEnabled();
    await action("live.stream.stop").click();
    await expect(page.getByText(/Live candles stopped/i).first()).toBeVisible();

    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).toContain("polygon_start_live_candle_stream");
    expect(invokes).toContain("polygon_stop_live_candle_stream");
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "live.safety.confirmations",
  title: "keeps live safety actions behind explicit confirmation flows",
  tags: ["@workflow", "@regression"],
  scenario: liveSafetyReadyScenario,
  run: async ({ page, action }) => {
    await action("shell.workspace.live").click();
    await page.getByLabel("Execution target").selectOption("local_paper");
    await page.getByLabel("Symbol").fill("AAPL");
    await page.getByLabel("Symbol").press("Tab");

    await expect(action("live.account.refreshOrders")).toBeEnabled();
    await expect(action("live.safety.cancelOpenOrders")).toBeEnabled();
    await expect(action("live.safety.flattenPosition")).toBeEnabled();

    await expect(action("live.stream.start")).toBeEnabled();
    await action("live.stream.start").click();
    await expect(
      page.getByRole("alertdialog", { name: /Start live candle feed/i }),
    ).toBeVisible();
    await page
      .locator(".confirmationDialog")
      .getByRole("button", { name: /^Start live feed$/ })
      .click();
    await expect(
      page.getByText(/Live candles started for AAPL/i).first(),
    ).toBeVisible();
    await expect(action("live.safety.emergencyHalt")).toBeEnabled();

    await action("live.safety.cancelOpenOrders").click();
    await expect(
      page.getByRole("alertdialog", {
        name: /Cancel current-symbol open orders/i,
      }),
    ).toBeVisible();
    await page
      .locator(".confirmationDialog")
      .getByRole("button", { name: /^Cancel open orders$/ })
      .click();
    await expect(
      page.getByText(/Cancelled 1 open broker order for AAPL/i).first(),
    ).toBeVisible();

    await action("live.safety.flattenPosition").click();
    await expect(
      page.getByRole("alertdialog", {
        name: /Flatten the current broker position/i,
      }),
    ).toBeVisible();
    await page
      .locator(".confirmationDialog")
      .getByRole("button", { name: /^Flatten position$/ })
      .click();
    await expect(
      page
        .getByText(/Submitted SELL flatten order \(id=lp_flat_1\) for AAPL/i)
        .first(),
    ).toBeVisible();

    await action("live.safety.emergencyHalt").click();
    await expect(
      page.getByRole("alertdialog", {
        name: /Emergency halt live automation/i,
      }),
    ).toBeVisible();
    await page
      .locator(".confirmationDialog")
      .getByRole("button", { name: /^Emergency halt$/ })
      .click();
    await expect(
      page.getByText(/Emergency halt applied/i).first(),
    ).toBeVisible();
    await expect(action("live.safety.emergencyHalt")).toBeDisabled();

    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).toContain("polygon_start_live_candle_stream");
    expect(invokes).toContain("cancel_local_paper_open_orders");
    expect(invokes).toContain("flatten_local_paper_position");
    expect(invokes).toContain("emergency_halt_live_execution");
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});
