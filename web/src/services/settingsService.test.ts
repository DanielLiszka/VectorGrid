import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { invokeCommand } from "../tauri/invoke";
import {
  clearArtifactRetentionSettings,
  buildLocalDataInventoryDryRun,
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

function artifactRetentionResponse() {
  return {
    defaults: {
      run_keep_newest: 200,
      meta_keep_newest: 100,
      stage5_keep_newest: 100,
      ml_policy_eval_keep_newest: 100,
      batch_summary_keep_newest: 100,
      campaign_summary_keep_newest: 100,
      options_expression_keep_newest: 100,
    },
    configured: {},
    env_overrides: {},
    effective: {
      run_keep_newest: 200,
      meta_keep_newest: 100,
      stage5_keep_newest: 100,
      ml_policy_eval_keep_newest: 100,
      batch_summary_keep_newest: 100,
      campaign_summary_keep_newest: 100,
      options_expression_keep_newest: 100,
    },
  };
}

describe("settings service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.useRealTimers();
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
    invokeCommandMock.mockResolvedValueOnce(artifactRetentionResponse());

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

  it("rejects invalid artifact-retention settings before invoking the backend", async () => {
    await expect(
      setArtifactRetentionSettings({
        run: -1,
        meta: null,
        stage5: null,
        mlPolicyEval: null,
        batchSummary: null,
        campaignSummary: null,
        optionsExpression: null,
      }),
    ).rejects.toThrow(
      "run artifact retention setting must be a positive integer.",
    );

    expect(invokeCommandMock).not.toHaveBeenCalled();
  });

  it("clears persisted artifact-retention settings", async () => {
    invokeCommandMock.mockResolvedValueOnce(artifactRetentionResponse());

    await clearArtifactRetentionSettings();

    expect(invokeCommandMock).toHaveBeenCalledWith(
      "clear_artifact_retention_settings",
    );
  });

  it("rejects malformed settings command responses instead of showing fabricated defaults", async () => {
    invokeCommandMock.mockResolvedValueOnce({
      runs: [],
      meta: [],
      stage5: [],
      policy_evals: "not-array",
      options_expression: [],
    });

    await expect(loadArtifactCatalogSummary()).rejects.toThrow(
      "artifact library response.policy_evals must be an array.",
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

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "list_artifact_library",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      2,
      "rebuild_artifact_catalog",
    );
  });

  it("builds a redacted local data inventory dry run from existing read-only commands", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-04-30T15:00:00.000Z"));
    invokeCommandMock
      .mockResolvedValueOnce({
        default_root_dir: "C:/Users/example/AppData/Local/vectorgrid",
        configured_root_dir: "D:/VectorGridData",
        effective_root_dir: "D:/VectorGridData",
        polygon_cache_dir: "D:/VectorGridData/polygon",
      })
      .mockResolvedValueOnce({
        cache_dir: "D:/VectorGridData/polygon",
        has_api_key: true,
        credential_status: {
          has_api_key: true,
          state: "keyring",
          storage_backend: "windows-credential-manager",
          degraded: false,
          message: "",
        },
        datasets: [{ csv_path: "D:/VectorGridData/polygon/a.csv" }, {}],
      })
      .mockResolvedValueOnce({
        defaults: {
          run_keep_newest: 200,
          meta_keep_newest: 100,
          stage5_keep_newest: 100,
          ml_policy_eval_keep_newest: 100,
          batch_summary_keep_newest: 100,
          campaign_summary_keep_newest: 100,
          options_expression_keep_newest: 100,
        },
        configured: { run_keep_newest: 150 },
        env_overrides: {},
        effective: {
          run_keep_newest: 150,
          meta_keep_newest: 100,
          stage5_keep_newest: 100,
          ml_policy_eval_keep_newest: 100,
          batch_summary_keep_newest: 100,
          campaign_summary_keep_newest: 100,
          options_expression_keep_newest: 100,
        },
      })
      .mockResolvedValueOnce({
        runs: [{}, {}],
        meta: [{}],
        stage5: [],
        policy_evals: [{}],
        options_expression: [{}],
      });

    await expect(buildLocalDataInventoryDryRun()).resolves.toEqual({
      generatedAtUnixMs: Date.parse("2026-04-30T15:00:00.000Z"),
      scanMode: "dry_run",
      rawMarketDataContentsScanned: false,
      pathSummary: {
        defaultRootDir: "C:/Users/example/AppData/Local/vectorgrid",
        configuredRootDir: "D:/VectorGridData",
        effectiveRootDir: "D:/VectorGridData",
        polygonCacheDir: "D:/VectorGridData/polygon",
      },
      credentialSummary: {
        polygonHasApiKey: true,
        polygonCredentialState: "keyring",
        polygonCredentialStorageBackend: "windows-credential-manager",
        polygonCredentialDegraded: false,
        polygonCredentialMessage: "",
      },
      polygonDatasetCount: 2,
      artifactCatalog: {
        runs: 2,
        meta: 1,
        stage5: 0,
        policyEvals: 1,
        optionsExpression: 1,
        total: 5,
      },
      artifactRetention: {
        defaults: {
          run: 200,
          meta: 100,
          stage5: 100,
          mlPolicyEval: 100,
          batchSummary: 100,
          campaignSummary: 100,
          optionsExpression: 100,
        },
        configured: {
          run: 150,
          meta: null,
          stage5: null,
          mlPolicyEval: null,
          batchSummary: null,
          campaignSummary: null,
          optionsExpression: null,
        },
        envOverrides: {
          run: null,
          meta: null,
          stage5: null,
          mlPolicyEval: null,
          batchSummary: null,
          campaignSummary: null,
          optionsExpression: null,
        },
        effective: {
          run: 150,
          meta: 100,
          stage5: 100,
          mlPolicyEval: 100,
          batchSummary: 100,
          campaignSummary: 100,
          optionsExpression: 100,
        },
      },
      warnings: [],
    });

    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      1,
      "get_market_data_dir_info",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      2,
      "polygon_get_cache_info",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      3,
      "get_artifact_retention_info",
    );
    expect(invokeCommandMock).toHaveBeenNthCalledWith(
      4,
      "list_artifact_library",
    );
  });
});
