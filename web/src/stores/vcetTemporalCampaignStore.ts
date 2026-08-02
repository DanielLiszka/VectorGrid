import { defineStore } from "pinia";
import {
  buildVcetTemporalValidationPlan,
  createDefaultVcetTemporalPlanDraft,
  getVcetTemporalCampaignStatus,
  openVcetTemporalCampaignFinalTest,
  openVcetTemporalCampaignWorkbook,
  prepareVcetTemporalCampaign,
  revealVcetTemporalCampaignPath,
  runVcetTemporalCampaignSelection,
  type VcetTemporalCampaignStatus,
  type VcetTemporalPlanDraft,
  type VcetTemporalSourceEligibilityIdentity,
  vcetTemporalNativePathKey,
} from "../services/vcetTemporalCampaignService";
import { storeErrorText } from "../utils/storeErrorText";
import { readStorageString, writeStorageJson } from "../utils/storage";

export const VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY =
  "vectorgrid_vcet_temporal_campaign_v1";

type BusyAction = "prepare" | "selection" | "final" | "refresh" | "path";

interface PersistedPreparationIdentity {
  source_campaign_summary_path: string;
  source_summary_content_fingerprint?: string;
  preparation_artifact_path: string;
  preparation_artifact_sha256: string;
  // Display-only comparison metadata. The hash-bound preparation artifact,
  // not localStorage, remains authoritative for the submitted plan.
  prepared_plan_draft_snapshot?: VcetTemporalPlanDraft;
}

interface VcetTemporalCampaignSnapshotV1 {
  schema_version: 1;
  plan_draft: VcetTemporalPlanDraft;
  current_preparation: PersistedPreparationIdentity | null;
}

interface VcetTemporalCampaignStoreState {
  hydrated: boolean;
  busyAction: BusyAction | null;
  error: string;
  notice: string;
  status: VcetTemporalCampaignStatus | null;
  eligibleSourcePath: string;
  eligibleSourceIdentity: VcetTemporalSourceEligibilityIdentity | null;
  eligibilityMessage: string;
  currentSourceCampaignSummaryPath: string;
  currentSourceSummaryContentFingerprint: string;
  currentPreparationArtifactPath: string;
  currentPreparationArtifactSha256: string;
  currentPreparationPlanDraft: VcetTemporalPlanDraft | null;
  planDraft: VcetTemporalPlanDraft;
}

const SHA256_PATTERN = /^(?:sha256:)?([0-9a-f]{64})$/iu;
const SOURCE_CONTENT_FINGERPRINT_PATTERN =
  /^campaign-summary-v2-fnv1a64-[0-9a-f]{8,16}$/iu;
const MAX_PATH_CHARS = 4096;
const MAX_PERSISTED_SNAPSHOT_CHARS = 32 * 1024;

function asRecord(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function safePath(value: unknown): string {
  if (typeof value !== "string") return "";
  return value.trim() &&
    value === value.trim() &&
    value.length <= MAX_PATH_CHARS &&
    !/\p{Cc}/u.test(value)
    ? value
    : "";
}

function safeHash(value: unknown): string {
  if (typeof value !== "string" || value !== value.trim()) return "";
  const match = value.match(SHA256_PATTERN);
  return match?.[1] ? `sha256:${match[1].toLowerCase()}` : "";
}

function safeSourceContentFingerprint(value: unknown): string {
  if (typeof value !== "string" || value !== value.trim()) return "";
  return SOURCE_CONTENT_FINGERPRINT_PATTERN.test(value)
    ? value.toLowerCase()
    : "";
}

function safeEligibilityIdentity(
  path: string,
  value: unknown,
): VcetTemporalSourceEligibilityIdentity | null {
  const row = asRecord(value);
  const nativePathKey = vcetTemporalNativePathKey(path);
  const summaryContentFingerprint = safeSourceContentFingerprint(
    row?.summaryContentFingerprint,
  );
  if (
    !nativePathKey ||
    !summaryContentFingerprint ||
    row?.nativePathKey !== nativePathKey
  ) {
    return null;
  }
  return { nativePathKey, summaryContentFingerprint };
}

function safeDraftText(
  value: unknown,
  fallback: string,
  pattern: RegExp,
  maximum: number,
): string {
  if (typeof value !== "string") return fallback;
  const text = value.trim();
  return text.length <= maximum && pattern.test(text) ? text : fallback;
}

function normalizePlanDraft(value: unknown): VcetTemporalPlanDraft {
  const defaults = createDefaultVcetTemporalPlanDraft();
  const row = asRecord(value);
  if (!row) return defaults;
  const integerDraft = (key: keyof VcetTemporalPlanDraft) =>
    safeDraftText(row[key], defaults[key], /^\d*$/u, 12);
  const dateDraft = (key: "startUtc" | "endUtc") =>
    safeDraftText(
      row[key],
      defaults[key],
      /^(?:|\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(?::\d{2})?)$/u,
      19,
    );
  const foldMode =
    row.foldMode === "rolling" || row.foldMode === "anchored"
      ? row.foldMode
      : defaults.foldMode;
  const positionBoundaryPolicy =
    row.positionBoundaryPolicy === "reset_flat_discard_open"
      ? (row.positionBoundaryPolicy as VcetTemporalPlanDraft["positionBoundaryPolicy"])
      : defaults.positionBoundaryPolicy;
  const reportLabelCandidate = safeDraftText(
    row.reportLabel,
    defaults.reportLabel,
    /^[^\p{Cc}]*$/u,
    128,
  );
  const reportLabel =
    new TextEncoder().encode(reportLabelCandidate).byteLength <= 128
      ? reportLabelCandidate
      : defaults.reportLabel;

  return {
    startUtc: dateDraft("startUtc"),
    endUtc: dateDraft("endUtc"),
    foldMode,
    trainBars: integerDraft("trainBars"),
    validationBars: integerDraft("validationBars"),
    finalTestBars: integerDraft("finalTestBars"),
    stepBars: integerDraft("stepBars"),
    purgeBars: integerDraft("purgeBars"),
    embargoBars: integerDraft("embargoBars"),
    warmupBars: integerDraft("warmupBars"),
    positionBoundaryPolicy,
    minFolds: integerDraft("minFolds"),
    minTrainBars: integerDraft("minTrainBars"),
    minValidationBars: integerDraft("minValidationBars"),
    minFinalTestBars: integerDraft("minFinalTestBars"),
    minTrainTradesPerFold: integerDraft("minTrainTradesPerFold"),
    minValidationTradesPerFold: integerDraft("minValidationTradesPerFold"),
    minFinalTestTrades: integerDraft("minFinalTestTrades"),
    reportLabel,
  };
}

const PLAN_DRAFT_KEYS: ReadonlyArray<keyof VcetTemporalPlanDraft> = [
  "startUtc",
  "endUtc",
  "foldMode",
  "trainBars",
  "validationBars",
  "finalTestBars",
  "stepBars",
  "purgeBars",
  "embargoBars",
  "warmupBars",
  "positionBoundaryPolicy",
  "minFolds",
  "minTrainBars",
  "minValidationBars",
  "minFinalTestBars",
  "minTrainTradesPerFold",
  "minValidationTradesPerFold",
  "minFinalTestTrades",
  "reportLabel",
];

function normalizeSealedPlanDraft(
  value: unknown,
): VcetTemporalPlanDraft | null {
  const row = asRecord(value);
  if (
    !row ||
    PLAN_DRAFT_KEYS.some(
      (key) =>
        !Object.prototype.hasOwnProperty.call(row, key) ||
        typeof row[key] !== "string",
    )
  ) {
    return null;
  }
  const normalized = normalizePlanDraft(row);
  if (PLAN_DRAFT_KEYS.some((key) => normalized[key] !== row[key])) {
    return null;
  }
  try {
    buildVcetTemporalValidationPlan(normalized);
    return normalized;
  } catch {
    return null;
  }
}

function planDraftsMatch(
  left: VcetTemporalPlanDraft,
  right: VcetTemporalPlanDraft,
): boolean {
  try {
    const leftBuilt = buildVcetTemporalValidationPlan(left);
    const rightBuilt = buildVcetTemporalValidationPlan(right);
    return (
      JSON.stringify(leftBuilt.plan) === JSON.stringify(rightBuilt.plan) &&
      (leftBuilt.reportLabel ?? "") === (rightBuilt.reportLabel ?? "")
    );
  } catch {
    return false;
  }
}

function readSnapshot(): VcetTemporalCampaignSnapshotV1 | null {
  const raw = readStorageString(VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY, "");
  if (!raw || raw.length > MAX_PERSISTED_SNAPSHOT_CHARS) return null;
  let decoded: unknown;
  try {
    decoded = JSON.parse(raw) as unknown;
  } catch {
    return null;
  }
  const row = asRecord(decoded);
  if (!row || row.schema_version !== 1) return null;
  const identity = asRecord(row.current_preparation);
  let currentPreparation: PersistedPreparationIdentity | null = null;
  if (identity) {
    const sourcePath = safePath(identity.source_campaign_summary_path);
    const sourceContentFingerprint = safeSourceContentFingerprint(
      identity.source_summary_content_fingerprint,
    );
    const preparationPath = safePath(identity.preparation_artifact_path);
    const preparationHash = safeHash(identity.preparation_artifact_sha256);
    if (sourcePath && preparationPath && preparationHash) {
      currentPreparation = {
        source_campaign_summary_path: sourcePath,
        ...(sourceContentFingerprint
          ? {
              source_summary_content_fingerprint: sourceContentFingerprint,
            }
          : {}),
        preparation_artifact_path: preparationPath,
        preparation_artifact_sha256: preparationHash,
      };
      const sealedPlanDraft = normalizeSealedPlanDraft(
        identity.prepared_plan_draft_snapshot,
      );
      if (sealedPlanDraft) {
        currentPreparation.prepared_plan_draft_snapshot = sealedPlanDraft;
      }
    }
  }
  return {
    schema_version: 1,
    plan_draft: normalizePlanDraft(row.plan_draft),
    current_preparation: currentPreparation,
  };
}

function errorText(error: unknown): string {
  return storeErrorText(error) || "VCET temporal campaign action failed.";
}

function hasRetainedSelectionScientificEvidence(
  status: VcetTemporalCampaignStatus,
): boolean {
  return !!(
    status.winnerCandidateId &&
    status.winnerLockSha256 &&
    status.preFinalReductionArtifactPath &&
    status.preFinalReductionArtifactSha256 &&
    status.completedSelectionShardCount === status.selectionShardCount
  );
}

function hasRetainedFinalScientificEvidence(
  status: VcetTemporalCampaignStatus,
): boolean {
  return !!(
    status.finalReductionArtifactPath &&
    status.finalReductionArtifactSha256 &&
    status.completedFinalTestShardCount === status.finalTestShardCount
  );
}

const DISTINCT_REPORT_REPLACEMENT_GUIDANCE =
  "Change the report label, source identity, or validation plan before starting a distinct replacement campaign.";

function refreshedReportBlockNotice(
  status: VcetTemporalCampaignStatus,
): string {
  if (hasRetainedFinalScientificEvidence(status)) {
    return `Status refreshed: retained final scientific evidence remains available, but report recovery is terminally blocked. ${DISTINCT_REPORT_REPLACEMENT_GUIDANCE}`;
  }
  if (hasRetainedSelectionScientificEvidence(status)) {
    return `Status refreshed: retained scientific selection evidence remains available, but no final scientific reduction was retained before report recovery became terminally blocked. ${DISTINCT_REPORT_REPLACEMENT_GUIDANCE}`;
  }
  return `Status refreshed: report recovery is terminally blocked before a scientific selection reduction was retained. ${DISTINCT_REPORT_REPLACEMENT_GUIDANCE}`;
}

function selectionReportBlockNotice(
  status: VcetTemporalCampaignStatus,
): string {
  const evidence = hasRetainedSelectionScientificEvidence(status)
    ? "The locked winner and scientific selection evidence remain retained, but the selection report cannot be trusted or recovered."
    : "The selection-report receipt became terminally blocked before a locked winner or scientific selection reduction was retained.";
  return `${evidence} ${DISTINCT_REPORT_REPLACEMENT_GUIDANCE}`;
}

function finalReportBlockNotice(status: VcetTemporalCampaignStatus): string {
  const evidence = hasRetainedFinalScientificEvidence(status)
    ? "The sealed final scientific reduction remains retained, but the final report cannot be trusted or recovered."
    : hasRetainedSelectionScientificEvidence(status)
      ? "The final-report receipt became terminally blocked before a final scientific reduction was retained; the locked selection evidence remains available."
      : "The final-report receipt became terminally blocked before scientific selection or final evidence was retained.";
  return `${evidence} ${DISTINCT_REPORT_REPLACEMENT_GUIDANCE}`;
}

function defaultState(): VcetTemporalCampaignStoreState {
  return {
    hydrated: false,
    busyAction: null,
    error: "",
    notice: "",
    status: null,
    eligibleSourcePath: "",
    eligibleSourceIdentity: null,
    eligibilityMessage:
      "Complete a structurally eligible VCET candidate-universe replay before preparing temporal evidence.",
    currentSourceCampaignSummaryPath: "",
    currentSourceSummaryContentFingerprint: "",
    currentPreparationArtifactPath: "",
    currentPreparationArtifactSha256: "",
    currentPreparationPlanDraft: null,
    planDraft: createDefaultVcetTemporalPlanDraft(),
  };
}

export const useVcetTemporalCampaignStore = defineStore(
  "vcetTemporalCampaign",
  {
    state: defaultState,

    getters: {
      busy: (state): boolean => state.busyAction !== null,
      hasCurrentPreparation: (state): boolean =>
        !!state.currentPreparationArtifactPath &&
        !!state.currentPreparationArtifactSha256,
      canRunSelection: (state): boolean =>
        state.busyAction === null &&
        state.status?.state === "prepared" &&
        !state.status.executionBlocked,
      canOpenFinalTest: (state): boolean =>
        state.busyAction === null &&
        state.status?.state === "selection_complete" &&
        !state.status.executionBlocked &&
        !!state.status.winnerLockSha256,
      winnerLockConfirmationSuffix: (state): string => {
        const hash = String(state.status?.winnerLockSha256 ?? "").replace(
          /^sha256:/iu,
          "",
        );
        return hash.length >= 8 ? hash.slice(-8) : "";
      },
      draftDiffersFromActive: (state): boolean =>
        !!state.currentPreparationPlanDraft &&
        !planDraftsMatch(state.currentPreparationPlanDraft, state.planDraft),
      activePlanComparisonKnown: (state): boolean =>
        !!state.currentPreparationPlanDraft,
      activeSourceComparisonKnown: (state): boolean => {
        const currentPathKey = vcetTemporalNativePathKey(
          state.currentSourceCampaignSummaryPath,
        );
        const eligiblePathKey =
          state.eligibleSourceIdentity?.nativePathKey ?? "";
        if (!currentPathKey || !eligiblePathKey) return false;
        if (currentPathKey !== eligiblePathKey) return true;
        return !!(
          state.currentSourceSummaryContentFingerprint &&
          state.eligibleSourceIdentity?.summaryContentFingerprint
        );
      },
      sourceDiffersFromActive: (state): boolean => {
        const currentPathKey = vcetTemporalNativePathKey(
          state.currentSourceCampaignSummaryPath,
        );
        const eligiblePathKey =
          state.eligibleSourceIdentity?.nativePathKey ?? "";
        if (
          currentPathKey &&
          eligiblePathKey &&
          currentPathKey !== eligiblePathKey
        ) {
          return true;
        }
        return !!(
          currentPathKey &&
          eligiblePathKey &&
          currentPathKey === eligiblePathKey &&
          state.currentSourceSummaryContentFingerprint &&
          state.eligibleSourceIdentity?.summaryContentFingerprint &&
          state.currentSourceSummaryContentFingerprint !==
            state.eligibleSourceIdentity.summaryContentFingerprint
        );
      },
      canForgetStalePreparation: (state): boolean =>
        state.busyAction === null &&
        state.status === null &&
        !!state.currentPreparationArtifactPath &&
        !!state.currentPreparationArtifactSha256,
      canForgetCurrentPreparation: (state): boolean =>
        state.busyAction === null &&
        state.status === null &&
        !!state.currentPreparationArtifactPath &&
        !!state.currentPreparationArtifactSha256,
    },

    actions: {
      persist(): boolean {
        const currentPreparation =
          this.currentSourceCampaignSummaryPath &&
          this.currentPreparationArtifactPath &&
          this.currentPreparationArtifactSha256
            ? {
                source_campaign_summary_path:
                  this.currentSourceCampaignSummaryPath,
                ...(this.currentSourceSummaryContentFingerprint
                  ? {
                      source_summary_content_fingerprint:
                        this.currentSourceSummaryContentFingerprint,
                    }
                  : {}),
                preparation_artifact_path: this.currentPreparationArtifactPath,
                preparation_artifact_sha256:
                  this.currentPreparationArtifactSha256,
                ...(this.currentPreparationPlanDraft
                  ? {
                      prepared_plan_draft_snapshot: normalizePlanDraft(
                        this.currentPreparationPlanDraft,
                      ),
                    }
                  : {}),
              }
            : null;
        return writeStorageJson(VCET_TEMPORAL_CAMPAIGN_STORAGE_KEY, {
          schema_version: 1,
          plan_draft: normalizePlanDraft(this.planDraft),
          current_preparation: currentPreparation,
        } satisfies VcetTemporalCampaignSnapshotV1);
      },

      hydrate(): void {
        if (this.hydrated) return;
        const snapshot = readSnapshot();
        if (snapshot) {
          this.planDraft = snapshot.plan_draft;
          const current = snapshot.current_preparation;
          this.currentSourceCampaignSummaryPath =
            current?.source_campaign_summary_path ?? "";
          this.currentSourceSummaryContentFingerprint =
            current?.source_summary_content_fingerprint ?? "";
          this.currentPreparationArtifactPath =
            current?.preparation_artifact_path ?? "";
          this.currentPreparationArtifactSha256 =
            current?.preparation_artifact_sha256 ?? "";
          this.currentPreparationPlanDraft =
            current?.prepared_plan_draft_snapshot ?? null;
        }
        this.hydrated = true;
      },

      setEligibleSource(
        path: string,
        message = "",
        identity: VcetTemporalSourceEligibilityIdentity | null = null,
      ): void {
        const nextPath = safePath(path);
        this.eligibleSourcePath = nextPath;
        this.eligibleSourceIdentity = nextPath
          ? safeEligibilityIdentity(nextPath, identity)
          : null;
        this.eligibilityMessage = nextPath
          ? String(
              message ||
                "Structurally eligible terminal VCET campaign loaded; exact sealing-time verification is still pending.",
            ).trim()
          : String(
              message ||
                "Complete a structurally eligible VCET candidate-universe replay before preparing temporal evidence.",
            ).trim();
      },

      setPlanField<K extends keyof VcetTemporalPlanDraft>(
        key: K,
        value: VcetTemporalPlanDraft[K],
      ): void {
        this.planDraft = normalizePlanDraft({
          ...this.planDraft,
          [key]: value,
        });
        this.persist();
      },

      applyStatus(
        status: VcetTemporalCampaignStatus,
        options: {
          preparedPlanDraftSnapshot?: VcetTemporalPlanDraft;
          sourceSummaryContentFingerprint?: string;
          persistPreparationIdentity?: boolean;
        } = {},
      ): boolean {
        const previousStatus = this.status;
        const previousSourcePath = this.currentSourceCampaignSummaryPath;
        const previousSourceContentFingerprint =
          this.currentSourceSummaryContentFingerprint;
        const previousPreparationPath = this.currentPreparationArtifactPath;
        const previousPreparationHash = this.currentPreparationArtifactSha256;
        const previousPlanDraft = this.currentPreparationPlanDraft;
        // Native commands may return a canonical path after receiving an
        // equivalent alias. The pinned artifact hash is the preparation
        // identity authority; the response path may safely advance to its
        // canonical spelling without discarding retained comparison metadata.
        const samePreparationIdentity =
          !!this.currentPreparationArtifactSha256 &&
          this.currentPreparationArtifactSha256 ===
            status.preparationArtifactSha256;
        this.status = status;
        this.currentSourceCampaignSummaryPath =
          status.sourceCampaignSummaryPath;
        if (options.sourceSummaryContentFingerprint) {
          this.currentSourceSummaryContentFingerprint =
            safeSourceContentFingerprint(
              options.sourceSummaryContentFingerprint,
            );
        } else if (!samePreparationIdentity) {
          this.currentSourceSummaryContentFingerprint = "";
        }
        this.currentPreparationArtifactPath = status.preparationArtifactPath;
        this.currentPreparationArtifactSha256 =
          status.preparationArtifactSha256;
        if (options.preparedPlanDraftSnapshot) {
          this.currentPreparationPlanDraft = normalizeSealedPlanDraft(
            normalizePlanDraft(options.preparedPlanDraftSnapshot),
          );
        } else if (!samePreparationIdentity) {
          this.currentPreparationPlanDraft = null;
        }
        if (options.persistPreparationIdentity && !this.persist()) {
          this.status = previousStatus;
          this.currentSourceCampaignSummaryPath = previousSourcePath;
          this.currentSourceSummaryContentFingerprint =
            previousSourceContentFingerprint;
          this.currentPreparationArtifactPath = previousPreparationPath;
          this.currentPreparationArtifactSha256 = previousPreparationHash;
          this.currentPreparationPlanDraft = previousPlanDraft;
          this.error =
            previousPreparationPath && previousPreparationHash
              ? "The native replacement preparation completed, but its local resume identity could not be saved. It was not adopted; the prior active campaign pointer and status were retained. Evidence files were not deleted."
              : "The native preparation completed, but its local resume identity could not be saved. It was not adopted as the active campaign, and no local preparation pointer or status was retained. Evidence files were not deleted.";
          this.notice = "";
          return false;
        }
        this.error = "";
        return true;
      },

      failClosed(error: unknown): void {
        this.status = null;
        this.error = errorText(error);
        this.notice = "";
      },

      async reconcileMutationFailure(
        error: unknown,
        actionLabel: string,
      ): Promise<void> {
        const mutationError = errorText(error);
        const previousStatus = this.status;
        const previousSourcePath = this.currentSourceCampaignSummaryPath;
        const previousSourceContentFingerprint =
          this.currentSourceSummaryContentFingerprint;
        const previousPreparationPath = this.currentPreparationArtifactPath;
        const previousPreparationHash = this.currentPreparationArtifactSha256;
        const previousPlanDraft = this.currentPreparationPlanDraft;
        try {
          const refreshed = await getVcetTemporalCampaignStatus({
            preparationArtifactPath: previousPreparationPath,
            expectedPreparationArtifactSha256: previousPreparationHash,
          });
          this.applyStatus(refreshed);
          this.error = `${mutationError} Native status was refreshed after the unsuccessful ${actionLabel}; the active sealed identity and its verified stage state remain available.`;
          this.notice = "";
        } catch {
          this.status = previousStatus;
          this.currentSourceCampaignSummaryPath = previousSourcePath;
          this.currentSourceSummaryContentFingerprint =
            previousSourceContentFingerprint;
          this.currentPreparationArtifactPath = previousPreparationPath;
          this.currentPreparationArtifactSha256 = previousPreparationHash;
          this.currentPreparationPlanDraft = previousPlanDraft;
          this.error = `${mutationError} The last verified campaign state was retained because the follow-up status refresh also failed; use Refresh status before relying on newer evidence.`;
          this.notice = "";
        }
      },

      async prepare(
        input: { replaceActivePreparation?: boolean } = {},
      ): Promise<boolean> {
        if (this.busy) return false;
        const hasVerifiedActivePreparation =
          this.hasCurrentPreparation && !!this.status;
        if (!this.eligibleSourcePath) {
          const error = new Error(
            "No eligible latest VCET evaluate_candidates campaign is available.",
          );
          if (hasVerifiedActivePreparation) {
            this.error = errorText(error);
            this.notice = "The active sealed campaign remains unchanged.";
          } else {
            this.failClosed(error);
          }
          return false;
        }
        if (this.hasCurrentPreparation && !this.status) {
          this.failClosed(
            new Error(
              "Refresh or clear the stale saved preparation identity before starting another campaign.",
            ),
          );
          return false;
        }
        if (this.hasCurrentPreparation && !input.replaceActivePreparation) {
          this.error =
            "Starting another temporal campaign requires explicit confirmation that the active local campaign pointer will be replaced.";
          this.notice = "";
          return false;
        }
        if (
          this.hasCurrentPreparation &&
          input.replaceActivePreparation &&
          !this.draftDiffersFromActive &&
          !this.sourceDiffersFromActive
        ) {
          this.error = this.status?.executionBlocked
            ? "This blocked campaign has the same deterministic source and research-plan identity. Change the plan or load a structurally eligible source with a different native/content identity before starting a replacement."
            : "This source and research plan have no confirmed identity drift from the active sealed campaign. Continue that campaign, or change the plan/source identity before replacing it.";
          this.notice = "";
          return false;
        }
        const replacingActivePreparation = this.hasCurrentPreparation;
        try {
          const preparedPlanDraftSnapshot = normalizePlanDraft(this.planDraft);
          buildVcetTemporalValidationPlan(preparedPlanDraftSnapshot);
          const sourceCampaignSummaryPath = this.eligibleSourcePath;
          const sourceSummaryContentFingerprint =
            this.eligibleSourceIdentity?.summaryContentFingerprint ?? "";
          this.busyAction = "prepare";
          this.error = "";
          this.notice = "Preparing and sealing the bounded temporal campaign…";
          const status = await prepareVcetTemporalCampaign({
            sourceCampaignSummaryPath,
            planDraft: preparedPlanDraftSnapshot,
          });
          const adopted = this.applyStatus(status, {
            preparedPlanDraftSnapshot,
            sourceSummaryContentFingerprint,
            persistPreparationIdentity: true,
          });
          if (!adopted) return false;
          this.notice =
            "Temporal campaign prepared. Final-test data remains sealed.";
          return true;
        } catch (error) {
          if (replacingActivePreparation) {
            this.error = errorText(error);
            this.notice =
              "The active sealed campaign remains unchanged after the unsuccessful replacement attempt.";
          } else {
            this.failClosed(error);
          }
          return false;
        } finally {
          this.busyAction = null;
        }
      },

      async refreshStatus(): Promise<boolean> {
        if (this.busy || !this.hasCurrentPreparation) return false;
        try {
          this.busyAction = "refresh";
          this.error = "";
          const status = await getVcetTemporalCampaignStatus({
            preparationArtifactPath: this.currentPreparationArtifactPath,
            expectedPreparationArtifactSha256:
              this.currentPreparationArtifactSha256,
          });
          this.applyStatus(status);
          this.notice = status.executionBlocked
            ? status.blockedStage === "selection_report" ||
              status.blockedStage === "final_report"
              ? refreshedReportBlockNotice(status)
              : "Status refreshed: this campaign cannot resume and requires a new campaign."
            : "Temporal campaign status refreshed.";
          return true;
        } catch (error) {
          this.failClosed(error);
          return false;
        } finally {
          this.busyAction = null;
        }
      },

      async resumeAfterHydration(): Promise<boolean> {
        this.hydrate();
        return this.hasCurrentPreparation ? this.refreshStatus() : false;
      },

      forgetCurrentPreparation(input: { confirmed: boolean }): boolean {
        if (!this.canForgetCurrentPreparation) return false;
        if (!input.confirmed) {
          this.error =
            "Confirm that you want to forget this local preparation identity.";
          this.notice = "";
          return false;
        }
        const previousStatus = this.status;
        const previousSourcePath = this.currentSourceCampaignSummaryPath;
        const previousSourceContentFingerprint =
          this.currentSourceSummaryContentFingerprint;
        const previousPreparationPath = this.currentPreparationArtifactPath;
        const previousPreparationHash = this.currentPreparationArtifactSha256;
        const previousPlanDraft = this.currentPreparationPlanDraft;
        this.status = null;
        this.currentSourceCampaignSummaryPath = "";
        this.currentSourceSummaryContentFingerprint = "";
        this.currentPreparationArtifactPath = "";
        this.currentPreparationArtifactSha256 = "";
        this.currentPreparationPlanDraft = null;
        if (!this.persist()) {
          this.status = previousStatus;
          this.currentSourceCampaignSummaryPath = previousSourcePath;
          this.currentSourceSummaryContentFingerprint =
            previousSourceContentFingerprint;
          this.currentPreparationArtifactPath = previousPreparationPath;
          this.currentPreparationArtifactSha256 = previousPreparationHash;
          this.currentPreparationPlanDraft = previousPlanDraft;
          this.error =
            "The local preparation identity could not be cleared from app storage. No campaign state was changed.";
          this.notice = "";
          return false;
        }
        this.error = "";
        this.notice =
          "Forgot the stale local campaign pointer. Evidence files were not deleted.";
        return true;
      },

      forgetStalePreparation(input: { confirmed: boolean }): boolean {
        if (!this.canForgetStalePreparation) return false;
        return this.forgetCurrentPreparation(input);
      },

      async runSelection(): Promise<boolean> {
        if (!this.canRunSelection || !this.hasCurrentPreparation) return false;
        try {
          this.busyAction = "selection";
          this.error = "";
          this.notice = "Running sealed train/validation selection shards…";
          const status = await runVcetTemporalCampaignSelection({
            preparationArtifactPath: this.currentPreparationArtifactPath,
            expectedPreparationArtifactSha256:
              this.currentPreparationArtifactSha256,
          });
          this.applyStatus(status);
          if (status.executionBlocked) {
            this.notice =
              status.blockedStage === "selection_report"
                ? selectionReportBlockNotice(status)
                : "Selection cannot resume because durable pre-final evidence is terminally blocked. Start a new campaign.";
            return false;
          }
          this.notice =
            "Selection complete. The exact winner is locked; final-test data remains sealed.";
          return true;
        } catch (error) {
          await this.reconcileMutationFailure(error, "selection attempt");
          return false;
        } finally {
          this.busyAction = null;
        }
      },

      async openFinalTest(input: {
        disclosureAccepted: boolean;
        winnerLockSuffix: string;
      }): Promise<boolean> {
        if (!this.canOpenFinalTest || !this.hasCurrentPreparation) return false;
        const expectedSuffix = this.winnerLockConfirmationSuffix.toLowerCase();
        const suppliedSuffix = String(input.winnerLockSuffix ?? "")
          .trim()
          .toLowerCase();
        if (
          !input.disclosureAccepted ||
          !expectedSuffix ||
          suppliedSuffix !== expectedSuffix
        ) {
          this.error =
            "Final-test access requires the disclosure checkbox and the exact winner-lock suffix.";
          return false;
        }
        const winnerLockSha256 = this.status?.winnerLockSha256;
        if (!winnerLockSha256) return false;
        try {
          this.busyAction = "final";
          this.error = "";
          this.notice =
            "Opening the sealed final test for the locked winner only…";
          const status = await openVcetTemporalCampaignFinalTest({
            preparationArtifactPath: this.currentPreparationArtifactPath,
            expectedPreparationArtifactSha256:
              this.currentPreparationArtifactSha256,
            expectedWinnerLockSha256: winnerLockSha256,
          });
          this.applyStatus(status);
          if (status.executionBlocked) {
            this.notice =
              status.blockedStage === "final_report"
                ? finalReportBlockNotice(status)
                : "Final-test execution cannot resume because a durable shard is terminally blocked. Start a new campaign.";
            return false;
          }
          this.notice =
            "Final-test evidence complete. The winner was not reranked or replaced.";
          return true;
        } catch (error) {
          await this.reconcileMutationFailure(error, "final-test attempt");
          return false;
        } finally {
          this.busyAction = null;
        }
      },

      async revealPath(path: string): Promise<boolean> {
        if (this.busy || !safePath(path)) return false;
        try {
          this.busyAction = "path";
          this.error = "";
          await revealVcetTemporalCampaignPath(path);
          this.notice = "Revealed temporal campaign evidence.";
          return true;
        } catch (error) {
          this.error = errorText(error);
          return false;
        } finally {
          this.busyAction = null;
        }
      },

      async openWorkbook(path: string): Promise<boolean> {
        if (this.busy || !safePath(path)) return false;
        try {
          this.busyAction = "path";
          this.error = "";
          await openVcetTemporalCampaignWorkbook(path);
          this.notice = "Opened temporal campaign workbook.";
          return true;
        } catch (error) {
          this.error = errorText(error);
          return false;
        } finally {
          this.busyAction = null;
        }
      },
    },
  },
);
