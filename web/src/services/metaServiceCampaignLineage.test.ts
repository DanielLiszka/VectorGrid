import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { invokeCommand } from "../tauri/invoke";
import { bindRuntimePinia } from "../stores/runtimePinia";
import { useAnalysisStore } from "../stores/analysisStore";
import { LAST_BATCH_SUMMARY_KEY } from "../utils/core/storageKeys";
import { runMeta } from "./metaService";

vi.mock("../tauri/invoke", () => ({
  invokeCommand: vi.fn(async () => undefined),
}));

const invokeCommandMock = vi.mocked(invokeCommand);

function storageBackedBy(values: Map<string, string>): Storage {
  return {
    get length() {
      return values.size;
    },
    clear: () => values.clear(),
    getItem: (key: string) => values.get(key) ?? null,
    key: (index: number) => Array.from(values.keys())[index] ?? null,
    removeItem: (key: string) => values.delete(key),
    setItem: (key: string, value: string) => values.set(key, value),
  };
}

function vcetCampaignSummary(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    version: 2,
    kind: "vectorgrid_campaign_summary",
    ts: "2026-07-31T14:00:00Z",
    state: "completed_with_warnings",
    strategy_kind: "Vcet",
    timeframe: "1m",
    tags: ["vcet-universe"],
    holdout_pct: 20,
    holdout_tag: "oos",
    manifest: {
      manifest_hash: "sha256:abc123",
      candidate_manifest_hash: "sha256:candidates-123",
      execution_mode: "evaluate_candidates",
      strategy_kind: "Vcet",
    },
    request: {
      strategy_kind: "Vcet",
      timeframe_override: "1m",
      tags: ["vcet-universe"],
      holdout_pct: 20,
      holdout_tag: "oos",
      execution_mode: "evaluate_candidates",
    },
    counts: {
      total: 10,
      optimized: 0,
      validated: 10,
      skipped: 0,
      failed: 0,
      cancelled: false,
    },
    items: [
      {
        csv_path: "C:/data/AAPL.csv",
        status: "validated",
        artifact_path: "C:/runs/vectorgrid_run_aapl.json",
        artifact_sha256: "a".repeat(64),
      },
    ],
    fixed_candidate_evaluation: {
      schema_version: 4,
      candidate_manifest_hash: "sha256:candidates-123",
      require_gpu_kernel: true,
      requested_datasets: 10,
      evaluated_datasets: 10,
      gpu_kernel_datasets: 10,
      missing_or_failed_datasets: 0,
      selection_status: "selected",
      admitted_candidate_count: 1,
      admitted_candidate_ids: ["vcet_candidate_6_24"],
      selected_candidate_id: "vcet_candidate_6_24",
      candidates: [],
    },
    warnings: ["fixture warning"],
    runtime_ms: 10,
    ...overrides,
  };
}

function metaResponse(campaignSummaryPath = ""): Record<string, unknown> {
  return {
    ts: "2026-07-31T14:05:00Z",
    meta_json_path: "C:/runs/vcet-meta.json",
    cohort_manifest: campaignSummaryPath
      ? {
          campaign_lineage: {
            campaign_summary_path: campaignSummaryPath,
            manifest_hash: "sha256:abc123",
            candidate_manifest_hash: "sha256:candidates-123",
            expected_train_datasets: 8,
            expected_holdout_datasets: 2,
            matched_train_artifacts: 8,
            matched_holdout_artifacts: 2,
            excluded_out_of_scope_artifacts: 0,
          },
        }
      : null,
    result: {
      strategy_kind: "Vcet",
      timeframe: "1m",
      artifacts_used: 8,
      artifacts_considered: 8,
      candidates_considered: 4,
      runtime_ms: 12,
      groups: [],
      warnings: [],
    },
  };
}

function metaRequest(
  overrides: Record<string, unknown> = {},
): Record<string, unknown> {
  return {
    strategy_kind: "Vcet",
    timeframe: "1m",
    required_tag: "campaign-cohort:abc123",
    eval_holdout: true,
    holdout_tag: "OOS",
    min_samples: 5,
    top_k: 10,
    ...overrides,
  };
}

describe("Meta campaign cohort lineage", () => {
  let storageValues: Map<string, string>;
  let artifactsByPath: Map<string, Record<string, unknown>>;

  beforeEach(() => {
    vi.clearAllMocks();
    storageValues = new Map<string, string>();
    artifactsByPath = new Map<string, Record<string, unknown>>();
    vi.stubGlobal("localStorage", storageBackedBy(storageValues));

    const pinia = createPinia();
    setActivePinia(pinia);
    bindRuntimePinia(pinia);

    invokeCommandMock.mockImplementation(async (command, args) => {
      if (command === "load_artifact") {
        const path = String(args?.path ?? "");
        const artifact = artifactsByPath.get(path);
        if (!artifact) throw new Error(`missing fixture ${path}`);
        return artifact;
      }
      if (command === "run_meta_selection") {
        return metaResponse(String(args?.campaignSummaryPath ?? ""));
      }
      if (command === "list_artifact_library") {
        return { meta: [], stage5: [], policyEvals: [] };
      }
      return undefined;
    });
  });

  it("loads and attaches a compatible completed VCET evaluate-candidates cohort", async () => {
    const path = "C:/runs/vcet-cohort-compatible.json";
    storageValues.set(LAST_BATCH_SUMMARY_KEY, path);
    artifactsByPath.set(path, vcetCampaignSummary({ timeframe: "60s" }));

    await runMeta({ request: metaRequest() });

    const loadIndex = invokeCommandMock.mock.calls.findIndex(
      ([command]) => command === "load_artifact",
    );
    const metaIndex = invokeCommandMock.mock.calls.findIndex(
      ([command]) => command === "run_meta_selection",
    );
    expect(loadIndex).toBeGreaterThanOrEqual(0);
    expect(metaIndex).toBeGreaterThan(loadIndex);
    expect(invokeCommandMock.mock.calls[metaIndex]?.[1]).toMatchObject({
      req: metaRequest(),
      campaignSummaryPath: path,
    });

    const analysisStore = useAnalysisStore();
    expect(analysisStore.metaMeta).toContain("campaign_scope=8/8 train");
    expect(analysisStore.metaOutputPreview).toContain(
      "campaign cohort=vcet-cohort-compatible.json",
    );
  });

  it("does not attach a VCET cohort to a non-VCET Meta request", async () => {
    const path = "C:/runs/vcet-cohort-non-vcet-request.json";
    storageValues.set(LAST_BATCH_SUMMARY_KEY, path);
    artifactsByPath.set(path, vcetCampaignSummary());

    await runMeta({
      request: metaRequest({
        strategy_kind: "CompositeMo",
        required_tag: "vcet-universe",
        eval_holdout: false,
        holdout_tag: null,
      }),
    });

    const call = invokeCommandMock.mock.calls.find(
      ([command]) => command === "run_meta_selection",
    );
    expect(call?.[1]).not.toHaveProperty("campaignSummaryPath");
  });

  it("rejects stale timeframe, tag, holdout, unsealed artifacts, and incomplete campaign scopes", async () => {
    const cases = [
      {
        suffix: "timeframe",
        summary: vcetCampaignSummary({ timeframe: "5m" }),
        request: metaRequest(),
      },
      {
        suffix: "tag",
        summary: vcetCampaignSummary(),
        request: metaRequest({ required_tag: "another-cohort" }),
      },
      {
        suffix: "holdout",
        summary: vcetCampaignSummary(),
        request: metaRequest({ holdout_tag: "future" }),
      },
      {
        suffix: "running",
        summary: vcetCampaignSummary({ state: "running" }),
        request: metaRequest(),
      },
      {
        suffix: "unscoped-timeframe",
        summary: vcetCampaignSummary(),
        request: metaRequest({ timeframe: null }),
      },
      {
        suffix: "unscoped-tag",
        summary: vcetCampaignSummary(),
        request: metaRequest({ required_tag: null }),
      },
      {
        suffix: "broad-base-tag",
        summary: vcetCampaignSummary(),
        request: metaRequest({ required_tag: "vcet-universe" }),
      },
      {
        suffix: "non-campaign-kind",
        summary: vcetCampaignSummary({ kind: "vectorgrid_batch_summary" }),
        request: metaRequest(),
      },
      {
        suffix: "candidate-manifest",
        summary: vcetCampaignSummary({
          manifest: {
            manifest_hash: "sha256:abc123",
            candidate_manifest_hash: "sha256:other-candidates",
            execution_mode: "evaluate_candidates",
            strategy_kind: "Vcet",
          },
        }),
        request: metaRequest(),
      },
      {
        suffix: "legacy-admission-schema",
        summary: vcetCampaignSummary({
          fixed_candidate_evaluation: {
            ...(vcetCampaignSummary().fixed_candidate_evaluation as Record<
              string,
              unknown
            >),
            schema_version: 3,
          },
        }),
        request: metaRequest(),
      },
      {
        suffix: "missing-artifact-seal",
        summary: vcetCampaignSummary({
          items: [
            {
              csv_path: "C:/data/AAPL.csv",
              status: "validated",
              artifact_path: "C:/runs/vectorgrid_run_aapl.json",
            },
          ],
        }),
        request: metaRequest(),
      },
      {
        suffix: "malformed-artifact-seal",
        summary: vcetCampaignSummary({
          items: [
            {
              csv_path: "C:/data/AAPL.csv",
              status: "validated",
              artifact_path: "C:/runs/vectorgrid_run_aapl.json",
              artifact_sha256: "not-a-sha256",
            },
          ],
        }),
        request: metaRequest(),
      },
      {
        suffix: "no-admitted-candidate",
        summary: vcetCampaignSummary({
          fixed_candidate_evaluation: {
            ...(vcetCampaignSummary().fixed_candidate_evaluation as Record<
              string,
              unknown
            >),
            selection_status: "no_candidate_met_admission",
            admitted_candidate_count: 0,
            admitted_candidate_ids: [],
            selected_candidate_id: null,
          },
        }),
        request: metaRequest(),
      },
    ];

    for (const item of cases) {
      const path = `C:/runs/vcet-cohort-${item.suffix}.json`;
      storageValues.set(LAST_BATCH_SUMMARY_KEY, path);
      artifactsByPath.set(path, item.summary);
      const callsBefore = invokeCommandMock.mock.calls.length;

      await runMeta({ request: item.request });

      const call = invokeCommandMock.mock.calls
        .slice(callsBefore)
        .find(([command]) => command === "run_meta_selection");
      expect(call?.[1]?.campaignSummaryPath).toBeUndefined();
    }
  });

  it("does not reuse an in-memory cohort after its retained pointer is removed", async () => {
    const path = "C:/runs/vcet-cohort-cleared.json";
    storageValues.set(LAST_BATCH_SUMMARY_KEY, path);
    artifactsByPath.set(path, vcetCampaignSummary());
    await runMeta({ request: metaRequest() });

    storageValues.delete(LAST_BATCH_SUMMARY_KEY);
    const callsBefore = invokeCommandMock.mock.calls.length;
    await runMeta({ request: metaRequest() });

    const call = invokeCommandMock.mock.calls
      .slice(callsBefore)
      .find(([command]) => command === "run_meta_selection");
    expect(call?.[1]?.campaignSummaryPath).toBeUndefined();
  });

  it.each(["changed", "removed"])(
    "does not attach or restore a cohort whose pointer is %s during disk load",
    async (transition) => {
      const firstPath = `C:/runs/vcet-cohort-await-${transition}-first.json`;
      const nextPath = `C:/runs/vcet-cohort-await-${transition}-next.json`;
      storageValues.set(LAST_BATCH_SUMMARY_KEY, firstPath);
      let resolveArtifact: ((value: unknown) => void) | null = null;
      invokeCommandMock.mockImplementation(async (command) => {
        if (command === "load_artifact") {
          return new Promise((resolve) => {
            resolveArtifact = resolve;
          });
        }
        if (command === "run_meta_selection") return metaResponse();
        if (command === "list_artifact_library") {
          return { meta: [], stage5: [], policyEvals: [] };
        }
        return undefined;
      });

      const pending = runMeta({ request: metaRequest() });
      await vi.waitFor(() => expect(resolveArtifact).not.toBeNull());
      if (transition === "changed") {
        storageValues.set(LAST_BATCH_SUMMARY_KEY, nextPath);
      } else {
        storageValues.delete(LAST_BATCH_SUMMARY_KEY);
      }
      const resolver = resolveArtifact as ((value: unknown) => void) | null;
      if (!resolver) throw new Error("artifact resolver was not installed");
      resolver(vcetCampaignSummary());
      await pending;

      const call = invokeCommandMock.mock.calls.find(
        ([command]) => command === "run_meta_selection",
      );
      expect(call?.[1]?.campaignSummaryPath).toBeUndefined();
      expect(storageValues.get(LAST_BATCH_SUMMARY_KEY)).toBe(
        transition === "changed" ? nextPath : undefined,
      );
    },
  );

  it("fails closed on an unreadable changed pointer without replacing stored state", async () => {
    const firstPath = "C:/runs/vcet-cohort-first.json";
    storageValues.set(LAST_BATCH_SUMMARY_KEY, firstPath);
    artifactsByPath.set(firstPath, vcetCampaignSummary());
    await runMeta({ request: metaRequest() });

    const unreadablePath = "C:/runs/vcet-cohort-unreadable.json";
    storageValues.set(LAST_BATCH_SUMMARY_KEY, unreadablePath);
    const callsBefore = invokeCommandMock.mock.calls.length;
    await runMeta({ request: metaRequest() });

    const call = invokeCommandMock.mock.calls
      .slice(callsBefore)
      .find(([command]) => command === "run_meta_selection");
    expect(call?.[1]?.campaignSummaryPath).toBeUndefined();
    expect(storageValues.get(LAST_BATCH_SUMMARY_KEY)).toBe(unreadablePath);
  });
});
