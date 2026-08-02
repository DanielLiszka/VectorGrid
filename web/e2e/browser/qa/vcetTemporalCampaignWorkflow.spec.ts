import AxeBuilder from "@axe-core/playwright";
import { expect, type Page } from "@playwright/test";
import {
  emptyAppScenario,
  vcetTemporalEarlyFinalReportPoisonedScenario,
  vcetTemporalEarlySelectionReportPoisonedScenario,
  vcetTemporalFinalReportBlockedScenario,
  vcetTemporalBlockedRecoveryScenario,
  vcetTemporalFinalReportResumeScenario,
  vcetTemporalFinalShardResumeScenario,
  vcetTemporalFinalReductionPath,
  vcetTemporalFinalReportPath,
  vcetTemporalLateFinalBlockedScenario,
  vcetTemporalLatePreFinalBlockedScenario,
  vcetTemporalLifecycleReadyScenario,
  vcetTemporalMutationRecoveryScenario,
  vcetTemporalPreparationPath,
  vcetTemporalSelectionReportBlockedScenario,
  vcetTemporalSelectionReportResumeScenario,
  vcetTemporalSelectionReductionPath,
  vcetTemporalSelectionReportPath,
  vcetTemporalSelectionShardResumeScenario,
  vcetTemporalSourceSummaryPath,
  vcetTemporalStaleRecoveryScenario,
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

const TEMPORAL_STORAGE_KEY = "vectorgrid_vcet_temporal_campaign_v1";

async function openTemporalCampaignPanel({ page, action }: WorkflowContext) {
  await action("shell.workspace.aggregate").click();
  await action("aggregate.tab.batch").click();
  const panel = page.getByTestId("vcet-temporal-campaign-host");
  await expect(panel).toBeVisible();
  return panel;
}

async function expectTemporalPanelAccessible(page: Page): Promise<void> {
  const result = await new AxeBuilder({ page })
    .include('[data-testid="vcet-temporal-campaign-host"]')
    .withTags(["wcag2a", "wcag2aa", "wcag21aa", "wcag22aa"])
    .analyze();
  expect(
    result.violations.map((violation) => ({
      id: violation.id,
      impact: violation.impact,
      targets: violation.nodes.slice(0, 4).map((node) => node.target.join(" ")),
    })),
    "mocked-browser temporal panel accessibility violations",
  ).toEqual([]);
}

async function runSelectionResume(
  ctx: WorkflowContext,
  buttonLabel: string,
): Promise<void> {
  const { page, action } = ctx;
  const panel = await openTemporalCampaignPanel(ctx);
  await expect(panel.getByText(buttonLabel, { exact: true })).toBeVisible();
  await expect(
    action("aggregate.batch.vcetTemporal.runSelection"),
  ).toBeEnabled();
  await action("aggregate.batch.vcetTemporal.runSelection").click();
  await expect(
    panel.getByText("selection complete", { exact: true }).first(),
  ).toBeVisible();
  await expect(panel.getByText(/exact winner is locked/i)).toBeVisible();
  await expectNoStuckBusyState(page);
  await expectNoHorizontalDocumentOverflow(page);
}

async function runFinalResume(
  ctx: WorkflowContext,
  buttonLabel: string,
): Promise<void> {
  const { page, action } = ctx;
  const panel = await openTemporalCampaignPanel(ctx);
  await expect(panel.getByText(buttonLabel, { exact: true })).toBeVisible();
  await expect(
    action("aggregate.batch.vcetTemporal.openFinalTest"),
  ).toBeDisabled();
  await action("aggregate.batch.vcetTemporal.acceptFinalDisclosure").check();
  await action("aggregate.batch.vcetTemporal.confirmWinnerSuffix").fill(
    "dddddddd",
  );
  await expect(
    action("aggregate.batch.vcetTemporal.openFinalTest"),
  ).toBeEnabled();
  await action("aggregate.batch.vcetTemporal.openFinalTest").click();
  await expect(panel.getByText(/Final evidence is complete/i)).toBeVisible();
  await expectNoStuckBusyState(page);
  await expectNoHorizontalDocumentOverflow(page);
}

defineWorkflow({
  id: "aggregate.vcetTemporal.noSource",
  title:
    "keeps temporal preparation fail-closed without an eligible disk-backed VCET source",
  tags: ["@workflow", "@regression"],
  scenario: emptyAppScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);

    await expect(panel.locator('[data-source-eligible="false"]')).toBeVisible();
    await expect(
      panel.getByText("Not available", { exact: true }),
    ).toBeVisible();
    await expect(panel.getByText(/candidate-universe replay/i)).toBeVisible();
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expect(
      action("aggregate.batch.vcetTemporal.refreshStatus"),
    ).toBeDisabled();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);

    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).not.toContain("prepare_vcet_temporal_campaign");
    expect(invokes).not.toContain("run_vcet_temporal_campaign_selection");
    expect(invokes).not.toContain("open_vcet_temporal_campaign_final_test");
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.lifecycle",
  title:
    "drives the bounded temporal UI from preparation through an explicitly disclosed sealed final",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalLifecycleReadyScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);

    await expect(panel.locator('[data-source-eligible="true"]')).toContainText(
      "vcet-candidate-universe.json",
    );
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeEnabled();
    await expect(
      action("aggregate.batch.vcetTemporal.refreshStatus"),
    ).toBeDisabled();

    await page.locator("#vcetTemporal_trainBars").fill("4096");
    await page.locator("#vcetTemporalReportLabel").fill("Temporal browser QA");
    await action("aggregate.batch.vcetTemporal.prepare").click();

    await expect(page.getByTestId("vcet-temporal-active-source")).toHaveText(
      "vcet-candidate-universe.json",
    );
    await expect(page.getByTestId("vcet-temporal-active-plan")).toContainText(
      "4096 train",
    );
    await expect(
      panel.getByText("prepared", { exact: true }).first(),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toBeEnabled();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);

    const preparedLog = await readTauriInvokeLog(page);
    const prepareInvoke = preparedLog.find(
      (entry) => entry.command === "prepare_vcet_temporal_campaign",
    );
    expect(prepareInvoke?.args).toMatchObject({
      req: {
        source_campaign_summary_path: vcetTemporalSourceSummaryPath,
        report_label: "Temporal browser QA",
        plan: {
          train_bars: 4096,
          position_boundary_policy: "reset_flat_discard_open",
        },
      },
    });

    await action("aggregate.batch.vcetTemporal.refreshStatus").click();
    await expect(panel.getByText(/status refreshed/i)).toBeVisible();
    await expect
      .poll(
        async () =>
          (await readTauriInvokeLog(page)).filter(
            (entry) => entry.command === "get_vcet_temporal_campaign_status",
          ).length,
      )
      .toBe(1);

    await page.locator("#vcetTemporal_trainBars").fill("8192");
    await expect(page.getByTestId("vcet-temporal-drift-warning")).toContainText(
      /editable research draft differs/i,
    );
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await action("aggregate.batch.vcetTemporal.confirmReplacement").check();
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeEnabled();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toBeEnabled();

    await action("aggregate.batch.vcetTemporal.runSelection").click();
    await expect(
      panel.getByText("selection complete", { exact: true }).first(),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toBeDisabled();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toBeDisabled();
    await expect(
      panel.getByText("vcet-candidate-a", { exact: true }),
    ).toBeVisible();

    await action("aggregate.batch.vcetTemporal.revealPreparation").click();
    await action("aggregate.batch.vcetTemporal.revealSource").click();
    await action(
      "aggregate.batch.vcetTemporal.revealSelectionReduction",
    ).click();
    await action("aggregate.batch.vcetTemporal.openSelectionXlsx").click();

    await action("aggregate.batch.vcetTemporal.acceptFinalDisclosure").check();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toBeDisabled();
    await action("aggregate.batch.vcetTemporal.confirmWinnerSuffix").fill(
      "incorrect",
    );
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toBeDisabled();
    await action("aggregate.batch.vcetTemporal.confirmWinnerSuffix").fill(
      "dddddddd",
    );
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toBeEnabled();
    await action("aggregate.batch.vcetTemporal.openFinalTest").click();

    await expect(panel.getByText(/Final evidence is complete/i)).toBeVisible();
    await expect(panel.getByText(/no final rerank occurred/i)).toBeVisible();
    await action("aggregate.batch.vcetTemporal.revealFinalReduction").click();
    await action("aggregate.batch.vcetTemporal.openFinalXlsx").click();

    const invokeLog = await readTauriInvokeLog(page);
    const selectionInvoke = invokeLog.find(
      (entry) => entry.command === "run_vcet_temporal_campaign_selection",
    );
    expect(selectionInvoke?.args).toMatchObject({
      req: {
        preparation_artifact_path: vcetTemporalPreparationPath,
        expected_preparation_artifact_sha256: `sha256:${"a".repeat(64)}`,
      },
    });
    const finalInvoke = invokeLog.find(
      (entry) => entry.command === "open_vcet_temporal_campaign_final_test",
    );
    expect(finalInvoke?.args).toMatchObject({
      req: {
        preparation_artifact_path: vcetTemporalPreparationPath,
        expected_preparation_artifact_sha256: `sha256:${"a".repeat(64)}`,
        expected_winner_lock_sha256: `sha256:${"d".repeat(64)}`,
        confirm_final_test_access: true,
      },
    });

    const revealedPaths = invokeLog
      .filter((entry) => entry.command === "reveal_path")
      .map((entry) => entry.args.path);
    expect(revealedPaths).toEqual(
      expect.arrayContaining([
        vcetTemporalPreparationPath,
        vcetTemporalSourceSummaryPath,
        vcetTemporalSelectionReductionPath,
        vcetTemporalFinalReductionPath,
      ]),
    );
    const openedPaths = invokeLog
      .filter((entry) => entry.command === "open_path")
      .map((entry) => entry.args.path);
    expect(openedPaths).toEqual(
      expect.arrayContaining([
        vcetTemporalSelectionReportPath,
        vcetTemporalFinalReportPath,
      ]),
    );
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.staleRecovery",
  title:
    "forgets only a stale temporal local pointer after explicit recovery confirmation",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalStaleRecoveryScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);

    await expect(page.getByTestId("vcet-temporal-active-source")).toHaveText(
      "vcet-candidate-universe.json",
    );
    await expect(
      panel.getByText(/Saved preparation needs attention/i),
    ).toBeVisible();
    await expect(
      panel.getByText(/evidence files are never deleted/i),
    ).toBeVisible();
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toBeDisabled();

    await action("aggregate.batch.vcetTemporal.confirmForgetStale").check();
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toBeEnabled();
    await action("aggregate.batch.vcetTemporal.forgetStale").click();

    await expect(
      panel.getByText(/Forgot the stale local campaign pointer/i),
    ).toBeVisible();
    await expect(
      panel.getByText(/Saved preparation needs attention/i),
    ).toHaveCount(0);
    await expect(page.getByTestId("vcet-temporal-active-source")).toHaveCount(
      0,
    );
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeEnabled();

    const persisted = await page.evaluate(
      (key) => localStorage.getItem(key),
      TEMPORAL_STORAGE_KEY,
    );
    expect(JSON.parse(String(persisted))).toMatchObject({
      schema_version: 1,
      current_preparation: null,
    });
    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).not.toContain("reveal_path");
    expect(invokes).not.toContain("open_path");
    expect(invokes).not.toContain("prepare_vcet_temporal_campaign");
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.blockedRecovery",
  title:
    "blocks identical deterministic retry and unlocks replacement only after real draft drift",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalBlockedRecoveryScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);

    const blockedBoundary = page.getByTestId("vcet-temporal-execution-blocked");
    await expect(blockedBoundary).toHaveRole("alert");
    await expect(blockedBoundary).toContainText(/campaign cannot resume/i);
    await expect(blockedBoundary).toContainText(/pre-final reduction/i);
    await expect(blockedBoundary).toContainText(
      /evidence files are never deleted/i,
    );
    await expect(blockedBoundary).toContainText(/cannot be forgotten/i);
    await expect(panel.locator(".stageStateLabel")).toHaveText([
      "complete",
      "blocked",
      "sealed",
    ]);
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);

    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.confirmForgetStale"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toHaveCount(0);

    await page.locator("#vcetTemporal_trainBars").fill("4096");
    await expect(page.getByTestId("vcet-temporal-drift-warning")).toContainText(
      /editable research draft differs/i,
    );
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toBeVisible();
    await action("aggregate.batch.vcetTemporal.confirmReplacement").check();
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeEnabled();

    await expect(blockedBoundary).toBeVisible();
    await expect(page.getByTestId("vcet-temporal-active-source")).toBeVisible();
    const persisted = await page.evaluate(
      (key) => localStorage.getItem(key),
      TEMPORAL_STORAGE_KEY,
    );
    expect(JSON.parse(String(persisted))).toMatchObject({
      schema_version: 1,
      current_preparation: {
        preparation_artifact_path: vcetTemporalPreparationPath,
      },
    });
    const invokes = (await readTauriInvokeLog(page)).map(
      (entry) => entry.command,
    );
    expect(invokes).not.toContain("prepare_vcet_temporal_campaign");
    expect(invokes).not.toContain("run_vcet_temporal_campaign_selection");
    expect(invokes).not.toContain("open_vcet_temporal_campaign_final_test");
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.resumeSelectionShards",
  title: "resumes missing selection projections before report recovery",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalSelectionShardResumeScenario,
  run: (ctx) => runSelectionResume(ctx, "Resume sealed selection"),
});

defineWorkflow({
  id: "aggregate.vcetTemporal.resumeSelectionReport",
  title: "resumes a missing selection workbook after projections are complete",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalSelectionReportResumeScenario,
  run: (ctx) => runSelectionResume(ctx, "Resume selection report"),
});

defineWorkflow({
  id: "aggregate.vcetTemporal.resumeFinalShards",
  title: "requires disclosure before resuming final-test shards",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalFinalShardResumeScenario,
  run: (ctx) => runFinalResume(ctx, "Confirm and resume final test"),
});

defineWorkflow({
  id: "aggregate.vcetTemporal.resumeFinalReport",
  title: "requires disclosure before resuming the final workbook receipt",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalFinalReportResumeScenario,
  run: (ctx) => runFinalResume(ctx, "Confirm and resume final report"),
});

defineWorkflow({
  id: "aggregate.vcetTemporal.mutationRecovery",
  title:
    "keeps mocked-browser preparation, selection, and final failures safely retryable",
  tags: ["@workflow", "@regression"],
  scenario: vcetTemporalMutationRecoveryScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);

    await action("aggregate.batch.vcetTemporal.prepare").click();
    await expect(
      panel.getByText(/Mocked temporal preparation failure/i),
    ).toBeVisible();
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeEnabled();
    await action("aggregate.batch.vcetTemporal.prepare").click();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toBeEnabled();

    await action("aggregate.batch.vcetTemporal.runSelection").click();
    await expect(
      panel.getByText(/Mocked temporal selection failure/i),
    ).toBeVisible();
    await expect(panel.getByText(/Native status was refreshed/i)).toBeVisible();
    await expect(page.getByTestId("vcet-temporal-active-source")).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toBeEnabled();
    await action("aggregate.batch.vcetTemporal.runSelection").click();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toBeVisible();

    await action("aggregate.batch.vcetTemporal.acceptFinalDisclosure").check();
    await action("aggregate.batch.vcetTemporal.confirmWinnerSuffix").fill(
      "dddddddd",
    );
    await action("aggregate.batch.vcetTemporal.openFinalTest").click();
    await expect(
      panel.getByText(/Mocked temporal final-test failure/i),
    ).toBeVisible();
    await expect(panel.getByText(/Native status was refreshed/i)).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toBeEnabled();
    await action("aggregate.batch.vcetTemporal.openFinalTest").click();
    await expect(panel.getByText(/Final evidence is complete/i)).toBeVisible();

    const invokeLog = await readTauriInvokeLog(page);
    expect(
      invokeLog.filter(
        (entry) => entry.command === "prepare_vcet_temporal_campaign",
      ),
    ).toHaveLength(2);
    expect(
      invokeLog.filter(
        (entry) => entry.command === "run_vcet_temporal_campaign_selection",
      ),
    ).toHaveLength(2);
    expect(
      invokeLog.filter(
        (entry) => entry.command === "get_vcet_temporal_campaign_status",
      ),
    ).toHaveLength(2);
    expect(
      invokeLog.filter(
        (entry) => entry.command === "open_vcet_temporal_campaign_final_test",
      ),
    ).toHaveLength(2);
    await expectNoStuckBusyState(page);
    await expectNoHorizontalDocumentOverflow(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.lateFinalBlockedCompactA11y",
  title:
    "keeps a late final-shard block explicit, compact, and accessible in mocked-browser QA",
  tags: ["@workflow", "@regression", "@responsive", "@a11y"],
  viewport: { width: 390, height: 844 },
  scenario: vcetTemporalLateFinalBlockedScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);
    const blockedBoundary = page.getByTestId("vcet-temporal-execution-blocked");
    await expect(blockedBoundary).toContainText(/final-test shard 1/i);
    await expect(
      panel.getByText("Retained final evidence is complete."),
    ).toBeVisible();
    await expect(panel.locator(".stageStateLabel")).toHaveText([
      "complete",
      "complete",
      "blocked",
    ]);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.confirmForgetStale"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toHaveCount(0);
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expectTemporalPanelAccessible(page);
    await expectNoHorizontalDocumentOverflow(page);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.latePreFinalBlockedRetainsFinalEvidence",
  title:
    "keeps retained final evidence complete under a late pre-final reduction block",
  tags: ["@workflow", "@regression", "@a11y"],
  viewport: { width: 1024, height: 720 },
  scenario: vcetTemporalLatePreFinalBlockedScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);
    await expect(
      page.getByTestId("vcet-temporal-execution-blocked"),
    ).toContainText(/pre-final reduction/i);
    await expect(
      page.getByTestId("vcet-temporal-stage-selection"),
    ).toHaveAttribute("data-stage-state", "blocked");
    await expect(page.getByTestId("vcet-temporal-stage-final")).toHaveAttribute(
      "data-stage-state",
      "complete",
    );
    await expect(
      panel.getByText("Retained final evidence is complete."),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.revealFinalReduction"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalXlsx"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);
    await expectTemporalPanelAccessible(page);
    await expectNoHorizontalDocumentOverflow(page);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.selectionReportBlockedRetainsLateEvidence",
  title:
    "keeps late final evidence reviewable when the selection report receipt is terminally blocked",
  tags: ["@workflow", "@regression", "@a11y"],
  viewport: { width: 1024, height: 720 },
  scenario: vcetTemporalSelectionReportBlockedScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);
    const blocked = page.getByTestId("vcet-temporal-execution-blocked");
    await expect(blocked).toContainText(/selection report/i);
    await expect(blocked).toContainText(/receipt is invalid, foreign/i);
    await expect(
      page.getByTestId("vcet-temporal-stage-selection"),
    ).toHaveAttribute("data-stage-state", "blocked");
    await expect(page.getByTestId("vcet-temporal-stage-final")).toHaveAttribute(
      "data-stage-state",
      "complete",
    );
    await expect(
      panel.getByText("Retained final evidence is complete."),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.openSelectionXlsx"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.revealFinalReduction"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalXlsx"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toHaveCount(0);
    await expect(
      page.getByTestId("vcet-temporal-report-block-guidance"),
    ).toContainText(
      /change the report label.*source identity.*validation plan/i,
    );
    await page
      .getByTestId("vcet-temporal-report-label")
      .fill("Distinct replacement report");
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toBeVisible();
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expectTemporalPanelAccessible(page);
    await expectNoHorizontalDocumentOverflow(page);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.finalReportBlockedRetainsScientificReduction",
  title:
    "keeps the sealed final reduction reviewable when its workbook cannot be recovered exactly",
  tags: ["@workflow", "@regression", "@a11y"],
  viewport: { width: 1024, height: 720 },
  scenario: vcetTemporalFinalReportBlockedScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);
    const blocked = page.getByTestId("vcet-temporal-execution-blocked");
    await expect(blocked).toContainText(/final report/i);
    await expect(blocked).toContainText(/exact bytes and SHA-256/i);
    await expect(
      page.getByTestId("vcet-temporal-stage-selection"),
    ).toHaveAttribute("data-stage-state", "complete");
    await expect(page.getByTestId("vcet-temporal-stage-final")).toHaveAttribute(
      "data-stage-state",
      "blocked",
    );
    await expect(
      panel.getByText("Retained final scientific reduction is complete."),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.openSelectionXlsx"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.revealFinalReduction"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalXlsx"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toHaveCount(0);
    await expect(
      page.getByTestId("vcet-temporal-report-block-guidance"),
    ).toContainText(/identical retry is disabled.*cannot be forgotten/i);
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expectTemporalPanelAccessible(page);
    await expectNoHorizontalDocumentOverflow(page);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.earlySelectionReportPoisonClaimsNoScience",
  title:
    "blocks an early foreign selection-report receipt without claiming a winner or reduction",
  tags: ["@workflow", "@regression", "@a11y"],
  viewport: { width: 1024, height: 720 },
  scenario: vcetTemporalEarlySelectionReportPoisonedScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);
    const blocked = page.getByTestId("vcet-temporal-execution-blocked");
    await expect(blocked).toContainText(/selection report/i);
    await expect(blocked).toContainText(
      /before a locked winner or pre-final scientific reduction was retained/i,
    );
    await expect(blocked).toContainText(
      /no completed scientific selection evidence is claimed/i,
    );
    await expect(blocked).not.toContainText(/retained scientific artifact/i);
    await expect(
      page.getByTestId("vcet-temporal-stage-selection"),
    ).toHaveAttribute("data-stage-state", "blocked");
    await expect(page.getByTestId("vcet-temporal-stage-final")).toHaveAttribute(
      "data-stage-state",
      "sealed",
    );
    await expect(panel.getByText("Not locked", { exact: true })).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.revealSelectionReduction"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openSelectionXlsx"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.revealFinalReduction"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalXlsx"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toHaveCount(0);
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expect(
      page.getByTestId("vcet-temporal-report-block-guidance"),
    ).toContainText(/identical retry is disabled.*cannot be forgotten/i);
    await expectTemporalPanelAccessible(page);
    await expectNoHorizontalDocumentOverflow(page);
    await expectNoStuckBusyState(page);
  },
});

defineWorkflow({
  id: "aggregate.vcetTemporal.earlyFinalReportPoisonClaimsSelectionOnly",
  title:
    "keeps selection evidence but claims no final reduction after an early foreign final-report receipt",
  tags: ["@workflow", "@regression", "@a11y"],
  viewport: { width: 1024, height: 720 },
  scenario: vcetTemporalEarlyFinalReportPoisonedScenario,
  run: async (ctx) => {
    const { page, action } = ctx;
    const panel = await openTemporalCampaignPanel(ctx);
    const blocked = page.getByTestId("vcet-temporal-execution-blocked");
    await expect(blocked).toContainText(/final report/i);
    await expect(blocked).toContainText(
      /locked selection winner and pre-final scientific reduction remain complete/i,
    );
    await expect(blocked).toContainText(
      /no final scientific reduction was retained.*no completed final evidence is claimed/i,
    );
    await expect(
      page.getByTestId("vcet-temporal-stage-selection"),
    ).toHaveAttribute("data-stage-state", "complete");
    await expect(page.getByTestId("vcet-temporal-stage-final")).toHaveAttribute(
      "data-stage-state",
      "blocked",
    );
    await expect(
      action("aggregate.batch.vcetTemporal.revealSelectionReduction"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.openSelectionXlsx"),
    ).toBeVisible();
    await expect(
      action("aggregate.batch.vcetTemporal.revealFinalReduction"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalXlsx"),
    ).toHaveCount(0);
    await expect(panel.locator(".finalComplete")).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.runSelection"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.openFinalTest"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.forgetStale"),
    ).toHaveCount(0);
    await expect(
      action("aggregate.batch.vcetTemporal.confirmReplacement"),
    ).toHaveCount(0);
    await expect(action("aggregate.batch.vcetTemporal.prepare")).toBeDisabled();
    await expect(
      page.getByTestId("vcet-temporal-report-block-guidance"),
    ).toContainText(/identical retry is disabled.*cannot be forgotten/i);
    await expectTemporalPanelAccessible(page);
    await expectNoHorizontalDocumentOverflow(page);
    await expectNoStuckBusyState(page);
  },
});
