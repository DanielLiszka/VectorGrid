import { beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../tauri/invoke";
import {
  clearArtifactRetentionSettings,
  getArtifactRetentionInfo,
  loadArtifactCatalogSummary,
  openReportsDir,
  openRunsDir,
  rebuildArtifactCatalogSummary,
  setArtifactRetentionSettings,
  setWindowEffect,
} from "./settingsService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

const invokeCommandMock = vi.mocked(invokeCommand);

describe("settings service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("maps window effect payload for native command", async () => {
    await setWindowEffect("none", false);
    await setWindowEffect("mica", true);
    await setWindowEffect("blur", false);

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "set_window_effect", {
      effect: null,
      dark: false,
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "set_window_effect", {
      effect: "mica",
      dark: true,
    });
    expect(invokeCommandMock).toHaveBeenNthCalledWith(3, "set_window_effect", {
      effect: "blur",
      dark: false,
    });
  });

  it("opens runs and reports directories", async () => {
    await openRunsDir();
    await openReportsDir();

    expect(invokeCommandMock).toHaveBeenCalledWith("open_runs_dir");
    expect(invokeCommandMock).toHaveBeenCalledWith("open_reports_dir");
  });

  it("normalizes artifact-retention info into family-based camelCase summaries", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      defaults: {
        run_keep_newest: 200,
        meta_keep_newest: 100,
        stage5_keep_newest: 75,
        ml_policy_eval_keep_newest: 40,
        batch_summary_keep_newest: 30,
        campaign_summary_keep_newest: 20,
        options_expression_keep_newest: 10,
      },
      configured: {
        run_keep_newest: 160,
        stage5_keep_newest: 55,
        options_expression_keep_newest: 8,
      },
      env_overrides: {
        meta_keep_newest: 12,
        batch_summary_keep_newest: 6,
      },
      effective: {
        run_keep_newest: 160,
        meta_keep_newest: 12,
        stage5_keep_newest: 55,
        ml_policy_eval_keep_newest: 40,
        batch_summary_keep_newest: 6,
        campaign_summary_keep_newest: 20,
        options_expression_keep_newest: 8,
      },
    });

    await expect(getArtifactRetentionInfo()).resolves.toEqual({
      defaults: {
        run: 200,
        meta: 100,
        stage5: 75,
        mlPolicyEval: 40,
        batchSummary: 30,
        campaignSummary: 20,
        optionsExpression: 10,
      },
      configured: {
        run: 160,
        meta: null,
        stage5: 55,
        mlPolicyEval: null,
        batchSummary: null,
        campaignSummary: null,
        optionsExpression: 8,
      },
      envOverrides: {
        run: null,
        meta: 12,
        stage5: null,
        mlPolicyEval: null,
        batchSummary: 6,
        campaignSummary: null,
        optionsExpression: null,
      },
      effective: {
        run: 160,
        meta: 12,
        stage5: 55,
        mlPolicyEval: 40,
        batchSummary: 6,
        campaignSummary: 20,
        optionsExpression: 8,
      },
    });

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "get_artifact_retention_info",
    );
  });

  it("serializes artifact-retention settings with snake_case backend fields", async () => {
    await setArtifactRetentionSettings({
      run: 140,
      meta: null,
      stage5: 60,
      mlPolicyEval: null,
      batchSummary: 25,
      campaignSummary: null,
      optionsExpression: 9,
    });

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "set_artifact_retention_settings",
      {
        settings: {
          run_keep_newest: 140,
          stage5_keep_newest: 60,
          batch_summary_keep_newest: 25,
          options_expression_keep_newest: 9,
        },
      },
    );
  });

  it("clears persisted artifact-retention settings", async () => {
    await clearArtifactRetentionSettings();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "clear_artifact_retention_settings",
    );
  });

  it("loads and rebuilds artifact catalog count summaries", async () => {
    invokeCommandMock
      .mockResolvedValueOnce({
        runs: [{ path: "run.json" }, { path: "run2.json" }],
        meta: [{}],
        stage5: [],
        policyEvals: [{}],
        optionsExpression: [{ path: "options.json" }],
      })
      .mockResolvedValueOnce({
        runs: [{ path: "run.json" }],
        meta: [],
        stage5: [{}],
        policy_evals: [{}, {}],
        options_expression: [],
      });

    await expect(loadArtifactCatalogSummary()).resolves.toEqual({
      runs: 2,
      meta: 1,
      stage5: 0,
      policyEvals: 1,
      optionsExpression: 1,
      total: 5,
    });
    await expect(rebuildArtifactCatalogSummary()).resolves.toEqual({
      runs: 1,
      meta: 0,
      stage5: 1,
      policyEvals: 2,
      optionsExpression: 0,
      total: 4,
    });

    expect(invokeCommandMock).toHaveBeenNthCalledWith(1, "list_artifact_library");
    expect(invokeCommandMock).toHaveBeenNthCalledWith(2, "rebuild_artifact_catalog");
  });
});
