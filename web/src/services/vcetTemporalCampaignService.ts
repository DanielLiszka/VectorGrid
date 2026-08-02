import { invokeCommand } from "../tauri/invoke";
import { openPathWithRevealFallback } from "../utils/openPath";

export type VcetTemporalFoldMode = "anchored" | "rolling";

export type VcetTemporalPositionBoundaryPolicy = "reset_flat_discard_open";

export type VcetTemporalCampaignState =
  | "prepared"
  | "selection_complete"
  | "final_test_complete";

export type VcetTemporalNextRequiredAction =
  | "run_selection"
  | "resume_selection"
  | "resume_selection_report"
  | "confirm_and_open_final_test"
  | "confirm_and_resume_final_test"
  | "confirm_and_resume_final_report"
  | "start_new_campaign"
  | "none";

export type VcetTemporalBlockedStage =
  | "pre_final_reduction"
  | "final_test_shard"
  | "selection_report"
  | "final_report";

export type VcetTemporalBlockedReasonCode =
  | "durable_stage_failed_interrupted_or_invalid"
  | "report_receipt_invalid_or_foreign"
  | "report_xlsx_not_recoverable";

export interface VcetTemporalPlanDraft {
  startUtc: string;
  endUtc: string;
  foldMode: VcetTemporalFoldMode;
  trainBars: string;
  validationBars: string;
  finalTestBars: string;
  stepBars: string;
  purgeBars: string;
  embargoBars: string;
  warmupBars: string;
  positionBoundaryPolicy: VcetTemporalPositionBoundaryPolicy;
  minFolds: string;
  minTrainBars: string;
  minValidationBars: string;
  minFinalTestBars: string;
  minTrainTradesPerFold: string;
  minValidationTradesPerFold: string;
  minFinalTestTrades: string;
  reportLabel: string;
}

export interface VcetTemporalValidationPlanV1 {
  schema_version: 1;
  requested_bounds: {
    utc: {
      start_utc_ms_inclusive: number;
      end_utc_ms_inclusive: number;
    };
  };
  fold_mode: VcetTemporalFoldMode;
  train_bars: number;
  validation_bars: number;
  final_test_bars: number;
  step_bars: number;
  purge_bars: number;
  embargo_bars: number;
  warmup_bars: number;
  position_boundary_policy: VcetTemporalPositionBoundaryPolicy;
  minimum_support: {
    min_folds: number;
    min_train_bars: number;
    min_validation_bars: number;
    min_final_test_bars: number;
    min_train_trades_per_fold: number;
    min_validation_trades_per_fold: number;
    min_final_test_trades: number;
  };
}

export interface BuiltVcetTemporalPlan {
  plan: VcetTemporalValidationPlanV1;
  reportLabel: string | undefined;
  minimumRequiredBars: number;
}

export interface VcetTemporalCampaignStatus {
  schemaVersion: 1;
  campaignId: string;
  state: VcetTemporalCampaignState;
  preparationArtifactPath: string;
  preparationArtifactSha256: string;
  sourceCampaignSummaryPath: string;
  sourceCampaignSummarySha256: string;
  universeSha256: string;
  candidateCount: number;
  datasetCount: number;
  selectionDatasetCount: number;
  finalTestDatasetCount: number;
  selectionShardCount: number;
  completedSelectionShardCount: number;
  finalTestShardCount: number;
  completedFinalTestShardCount: number;
  winnerCandidateId: string | null;
  winnerLockSha256: string | null;
  winnerRunnerUpMargin: number | null;
  preFinalReductionArtifactPath: string | null;
  preFinalReductionArtifactSha256: string | null;
  finalReductionArtifactPath: string | null;
  finalReductionArtifactSha256: string | null;
  selectionReportXlsxPath: string | null;
  finalReportXlsxPath: string | null;
  executionBlocked: boolean;
  blockedStage: VcetTemporalBlockedStage | null;
  blockedShardOrdinal: number | null;
  blockedReasonCode: VcetTemporalBlockedReasonCode | null;
  nextRequiredAction: VcetTemporalNextRequiredAction;
  limitations: string[];
}

export interface VcetTemporalSourceEligibilityIdentity {
  nativePathKey: string;
  summaryContentFingerprint: string;
}

interface ExpectedStatusIdentity {
  preparationArtifactPath?: string;
  preparationArtifactSha256?: string;
  winnerLockSha256?: string;
}

const MAX_PLAN_INTEGER = 2_000_000;
const MAX_TRADE_INTEGER = 1_000_000;
const MAX_STATUS_JSON_CHARS = 128 * 1024;
const MAX_PATH_CHARS = 4096;
const SHA256_PATTERN = /^(?:sha256:)?([0-9a-f]{64})$/iu;
const ALLOWED_STATES = new Set<VcetTemporalCampaignState>([
  "prepared",
  "selection_complete",
  "final_test_complete",
]);
const ALLOWED_FOLD_MODES = new Set<VcetTemporalFoldMode>([
  "anchored",
  "rolling",
]);
const ALLOWED_POSITION_POLICIES = new Set<VcetTemporalPositionBoundaryPolicy>([
  "reset_flat_discard_open",
]);
const DURABLE_STAGE_BLOCKED_REASON_CODE: VcetTemporalBlockedReasonCode =
  "durable_stage_failed_interrupted_or_invalid";
const REPORT_BLOCKED_REASON_CODES = new Set<VcetTemporalBlockedReasonCode>([
  "report_receipt_invalid_or_foreign",
  "report_xlsx_not_recoverable",
]);

export function vcetTemporalNativePathKey(value: unknown): string {
  if (typeof value !== "string") return "";
  let path = value.trim().replace(/\\/gu, "/");
  if (!path) return "";

  if (/^\/\/\?\/unc\//iu.test(path)) {
    path = `//${path.slice("//?/unc/".length)}`;
  } else if (/^\/\/\?\//u.test(path)) {
    path = path.slice("//?/".length);
  }

  const windowsPath = /^[a-z]:\//iu.test(path) || path.startsWith("//");
  const prefix = path.startsWith("//")
    ? "//"
    : /^[a-z]:\//iu.test(path)
      ? path.slice(0, 3)
      : path.startsWith("/")
        ? "/"
        : "";
  const remainder = prefix ? path.slice(prefix.length) : path;
  const segments: string[] = [];
  for (const segment of remainder.split(/\/+/u)) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (segments.length > 0 && segments[segments.length - 1] !== "..") {
        segments.pop();
      } else if (!prefix) {
        segments.push(segment);
      }
      continue;
    }
    segments.push(segment);
  }
  const joined = `${prefix}${segments.join("/")}`;
  return windowsPath ? joined.toLowerCase() : joined;
}

function hasOwn(row: Record<string, unknown>, key: string): boolean {
  return Object.prototype.hasOwnProperty.call(row, key);
}

export function createDefaultVcetTemporalPlanDraft(): VcetTemporalPlanDraft {
  return {
    startUtc: "2018-01-01T00:00",
    endUtc: "2025-12-31T23:59",
    foldMode: "anchored",
    trainBars: "2048",
    validationBars: "512",
    finalTestBars: "512",
    stepBars: "544",
    purgeBars: "32",
    embargoBars: "32",
    warmupBars: "256",
    positionBoundaryPolicy: "reset_flat_discard_open",
    minFolds: "3",
    minTrainBars: "2048",
    minValidationBars: "512",
    minFinalTestBars: "512",
    minTrainTradesPerFold: "10",
    minValidationTradesPerFold: "5",
    minFinalTestTrades: "5",
    reportLabel: "VCET temporal research",
  };
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value !== null && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function parseBoundedInteger(
  value: unknown,
  field: string,
  minimum: number,
  maximum: number,
): number {
  if (typeof value !== "string") {
    throw new Error(`${field} must be a whole number.`);
  }
  const text = value.trim();
  if (!/^(?:0|[1-9]\d*)$/u.test(text)) {
    throw new Error(`${field} must be a whole number.`);
  }
  const parsed = Number(text);
  if (!Number.isSafeInteger(parsed) || parsed < minimum || parsed > maximum) {
    throw new Error(`${field} must be between ${minimum} and ${maximum}.`);
  }
  return parsed;
}

function parseUtcInput(value: unknown, field: string): number {
  if (typeof value !== "string") {
    throw new Error(`${field} must be a complete UTC date and time.`);
  }
  const text = value.trim();
  const match = text.match(
    /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2}))?$/u,
  );
  if (!match) {
    throw new Error(`${field} must be a complete UTC date and time.`);
  }
  const year = Number(match[1]);
  const month = Number(match[2]);
  const day = Number(match[3]);
  const hour = Number(match[4]);
  const minute = Number(match[5]);
  const second = Number(match[6] ?? 0);
  if (year < 1970 || year > 2100) {
    throw new Error(`${field} year must be between 1970 and 2100.`);
  }
  const parsed = Date.UTC(year, month - 1, day, hour, minute, second, 0);
  const exact = new Date(parsed);
  if (
    !Number.isSafeInteger(parsed) ||
    exact.getUTCFullYear() !== year ||
    exact.getUTCMonth() !== month - 1 ||
    exact.getUTCDate() !== day ||
    exact.getUTCHours() !== hour ||
    exact.getUTCMinutes() !== minute ||
    exact.getUTCSeconds() !== second
  ) {
    throw new Error(`${field} is not a valid UTC date and time.`);
  }
  return parsed;
}

function normalizeReportLabel(value: unknown): string | undefined {
  if (typeof value !== "string") {
    throw new Error("Report label must be text.");
  }
  const label = value.trim();
  if (!label) return undefined;
  if (
    new TextEncoder().encode(label).byteLength > 128 ||
    /\p{Cc}/u.test(label)
  ) {
    throw new Error(
      "Report label must be at most 128 UTF-8 bytes without control characters.",
    );
  }
  return label;
}

export function buildVcetTemporalValidationPlan(
  draft: VcetTemporalPlanDraft,
): BuiltVcetTemporalPlan {
  const startUtcMs = parseUtcInput(draft.startUtc, "UTC start");
  const endUtcMs = parseUtcInput(draft.endUtc, "UTC end");
  if (endUtcMs <= startUtcMs) {
    throw new Error("UTC end must be later than UTC start.");
  }

  if (!ALLOWED_FOLD_MODES.has(draft.foldMode)) {
    throw new Error("Fold mode must be anchored or rolling.");
  }
  if (!ALLOWED_POSITION_POLICIES.has(draft.positionBoundaryPolicy)) {
    throw new Error(
      "Product V1 requires the reset-flat/discard-open position boundary policy.",
    );
  }

  const trainBars = parseBoundedInteger(
    draft.trainBars,
    "Train bars",
    1,
    MAX_PLAN_INTEGER,
  );
  const validationBars = parseBoundedInteger(
    draft.validationBars,
    "Validation bars",
    1,
    MAX_PLAN_INTEGER,
  );
  const finalTestBars = parseBoundedInteger(
    draft.finalTestBars,
    "Final-test bars",
    1,
    MAX_PLAN_INTEGER,
  );
  const stepBars = parseBoundedInteger(
    draft.stepBars,
    "Step bars",
    1,
    MAX_PLAN_INTEGER,
  );
  const purgeBars = parseBoundedInteger(
    draft.purgeBars,
    "Purge bars",
    0,
    MAX_PLAN_INTEGER,
  );
  const embargoBars = parseBoundedInteger(
    draft.embargoBars,
    "Embargo bars",
    0,
    MAX_PLAN_INTEGER,
  );
  const warmupBars = parseBoundedInteger(
    draft.warmupBars,
    "Warmup bars",
    0,
    MAX_PLAN_INTEGER,
  );
  const minFolds = parseBoundedInteger(
    draft.minFolds,
    "Minimum folds",
    2,
    10_000,
  );
  const minTrainBars = parseBoundedInteger(
    draft.minTrainBars,
    "Minimum train bars",
    1,
    MAX_PLAN_INTEGER,
  );
  const minValidationBars = parseBoundedInteger(
    draft.minValidationBars,
    "Minimum validation bars",
    1,
    MAX_PLAN_INTEGER,
  );
  const minFinalTestBars = parseBoundedInteger(
    draft.minFinalTestBars,
    "Minimum final-test bars",
    1,
    MAX_PLAN_INTEGER,
  );
  const minTrainTradesPerFold = parseBoundedInteger(
    draft.minTrainTradesPerFold,
    "Minimum train trades per fold",
    1,
    MAX_TRADE_INTEGER,
  );
  const minValidationTradesPerFold = parseBoundedInteger(
    draft.minValidationTradesPerFold,
    "Minimum validation trades per fold",
    1,
    MAX_TRADE_INTEGER,
  );
  const minFinalTestTrades = parseBoundedInteger(
    draft.minFinalTestTrades,
    "Minimum final-test trades",
    1,
    MAX_TRADE_INTEGER,
  );

  if (stepBars < validationBars + embargoBars) {
    throw new Error(
      "Step bars must be at least validation bars plus embargo bars.",
    );
  }
  if (minTrainBars > trainBars) {
    throw new Error("Minimum train bars cannot exceed train bars.");
  }
  if (minValidationBars > validationBars) {
    throw new Error("Minimum validation bars cannot exceed validation bars.");
  }
  if (minFinalTestBars > finalTestBars) {
    throw new Error("Minimum final-test bars cannot exceed final-test bars.");
  }

  const minimumRequiredBars =
    warmupBars +
    trainBars +
    purgeBars +
    validationBars +
    (minFolds - 1) * stepBars +
    embargoBars +
    purgeBars +
    finalTestBars;
  if (
    !Number.isSafeInteger(minimumRequiredBars) ||
    minimumRequiredBars > MAX_PLAN_INTEGER
  ) {
    throw new Error(
      `The minimum temporal layout must not exceed the ${MAX_PLAN_INTEGER.toLocaleString()}-bar per-dataset product limit.`,
    );
  }

  return {
    plan: {
      schema_version: 1,
      requested_bounds: {
        utc: {
          start_utc_ms_inclusive: startUtcMs,
          end_utc_ms_inclusive: endUtcMs,
        },
      },
      fold_mode: draft.foldMode,
      train_bars: trainBars,
      validation_bars: validationBars,
      final_test_bars: finalTestBars,
      step_bars: stepBars,
      purge_bars: purgeBars,
      embargo_bars: embargoBars,
      warmup_bars: warmupBars,
      position_boundary_policy: draft.positionBoundaryPolicy,
      minimum_support: {
        min_folds: minFolds,
        min_train_bars: minTrainBars,
        min_validation_bars: minValidationBars,
        min_final_test_bars: minFinalTestBars,
        min_train_trades_per_fold: minTrainTradesPerFold,
        min_validation_trades_per_fold: minValidationTradesPerFold,
        min_final_test_trades: minFinalTestTrades,
      },
    },
    reportLabel: normalizeReportLabel(draft.reportLabel),
    minimumRequiredBars,
  };
}

function requiredText(value: unknown, field: string, maximum = 512): string {
  if (typeof value !== "string" || value !== value.trim()) {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  const text = value;
  if (!text || text.length > maximum || /\p{Cc}/u.test(text)) {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  return text;
}

function requiredPath(value: unknown, field: string): string {
  if (typeof value !== "string") {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  if (
    !value.trim() ||
    value !== value.trim() ||
    value.length > MAX_PATH_CHARS ||
    /\p{Cc}/u.test(value)
  ) {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  // Paths are provenance. Preserve the exact validated picker/backend spelling
  // instead of normalizing separators or case into a different identity.
  return value;
}

function optionalPath(value: unknown, field: string): string | null {
  if (value === null || value === undefined) {
    return null;
  }
  return requiredPath(value, field);
}

function requiredHash(value: unknown, field: string): string {
  if (typeof value !== "string" || value !== value.trim()) {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  const hash = value.toLowerCase();
  const match = hash.match(SHA256_PATTERN);
  if (!match?.[1]) {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  return `sha256:${match[1].toLowerCase()}`;
}

function optionalHash(value: unknown, field: string): string | null {
  if (value === null || value === undefined) {
    return null;
  }
  return requiredHash(value, field);
}

function responseCount(value: unknown, field: string): number {
  if (typeof value !== "number") {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  const count = value;
  if (!Number.isSafeInteger(count) || count < 0 || count > 20_000_000) {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  return count;
}

function optionalFiniteNumber(value: unknown, field: string): number | null {
  if (value === null || value === undefined) return null;
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`Temporal campaign response has invalid ${field}.`);
  }
  return value;
}

function assertOptionalArtifactPair(
  path: string | null,
  hash: string | null,
  field: string,
): void {
  if (!!path !== !!hash) {
    throw new Error(
      `Temporal campaign response has incomplete ${field} provenance.`,
    );
  }
}

export function normalizeVcetTemporalCampaignStatus(
  value: unknown,
  expected: ExpectedStatusIdentity = {},
): VcetTemporalCampaignStatus {
  let serializedLength = 0;
  try {
    const serialized = JSON.stringify(value);
    if (typeof serialized !== "string") {
      throw new Error("not serializable");
    }
    serializedLength = serialized.length;
  } catch {
    throw new Error("Temporal campaign response is not serializable.");
  }
  if (serializedLength > MAX_STATUS_JSON_CHARS) {
    throw new Error(
      "Temporal campaign response exceeded the compact status limit.",
    );
  }

  const row = asRecord(value);
  if (!row || row.schema_version !== 1) {
    throw new Error("Temporal campaign response has an unsupported schema.");
  }
  const state = row.state as VcetTemporalCampaignState;
  if (!ALLOWED_STATES.has(state)) {
    throw new Error("Temporal campaign response has an invalid state.");
  }

  const preparationArtifactPath = requiredPath(
    row.preparation_artifact_path,
    "preparation artifact path",
  );
  const preparationArtifactSha256 = requiredHash(
    row.preparation_artifact_sha256,
    "preparation artifact SHA-256",
  );
  const sourceCampaignSummaryPath = requiredPath(
    row.source_campaign_summary_path,
    "source campaign summary path",
  );
  const sourceCampaignSummarySha256 = requiredHash(
    row.source_campaign_summary_sha256,
    "source campaign summary SHA-256",
  );
  const universeSha256 = requiredHash(row.universe_sha256, "universe SHA-256");

  const expectedPreparationArtifactSha256 = expected.preparationArtifactSha256
    ? requiredHash(
        expected.preparationArtifactSha256,
        "expected preparation artifact SHA-256",
      )
    : null;
  if (expectedPreparationArtifactSha256) {
    if (preparationArtifactSha256 !== expectedPreparationArtifactSha256) {
      throw new Error(
        "Temporal campaign response changed the preparation hash.",
      );
    }
  } else if (
    expected.preparationArtifactPath &&
    vcetTemporalNativePathKey(preparationArtifactPath) !==
      vcetTemporalNativePathKey(expected.preparationArtifactPath)
  ) {
    throw new Error("Temporal campaign response changed the preparation path.");
  }

  const candidateCount = responseCount(row.candidate_count, "candidate count");
  const datasetCount = responseCount(row.dataset_count, "dataset count");
  const selectionDatasetCount = responseCount(
    row.selection_dataset_count,
    "selection dataset count",
  );
  const finalTestDatasetCount = responseCount(
    row.final_test_dataset_count,
    "final-test dataset count",
  );
  const selectionShardCount = responseCount(
    row.selection_shard_count,
    "selection shard count",
  );
  const completedSelectionShardCount = responseCount(
    row.completed_selection_shard_count,
    "completed selection shard count",
  );
  const finalTestShardCount = responseCount(
    row.final_test_shard_count,
    "final-test shard count",
  );
  const completedFinalTestShardCount = responseCount(
    row.completed_final_test_shard_count,
    "completed final-test shard count",
  );
  if (
    candidateCount < 1 ||
    datasetCount < 1 ||
    datasetCount > 64 ||
    selectionDatasetCount < 1 ||
    finalTestDatasetCount < 1 ||
    selectionDatasetCount + finalTestDatasetCount !== datasetCount ||
    selectionShardCount !== selectionDatasetCount ||
    finalTestShardCount !== finalTestDatasetCount ||
    completedSelectionShardCount > selectionShardCount ||
    completedFinalTestShardCount > finalTestShardCount
  ) {
    throw new Error("Temporal campaign response has inconsistent counts.");
  }

  const winnerCandidateId =
    row.winner_candidate_id === null || row.winner_candidate_id === undefined
      ? null
      : requiredText(row.winner_candidate_id, "winner candidate ID", 512);
  const winnerLockSha256 = optionalHash(
    row.winner_lock_sha256,
    "winner-lock SHA-256",
  );
  if (!!winnerCandidateId !== !!winnerLockSha256) {
    throw new Error(
      "Temporal campaign response has incomplete winner provenance.",
    );
  }

  const preFinalReductionArtifactPath = optionalPath(
    row.pre_final_reduction_artifact_path,
    "pre-final reduction artifact path",
  );
  const preFinalReductionArtifactSha256 = optionalHash(
    row.pre_final_reduction_artifact_sha256,
    "pre-final reduction artifact SHA-256",
  );
  const finalReductionArtifactPath = optionalPath(
    row.final_reduction_artifact_path,
    "final reduction artifact path",
  );
  const finalReductionArtifactSha256 = optionalHash(
    row.final_reduction_artifact_sha256,
    "final reduction artifact SHA-256",
  );
  assertOptionalArtifactPair(
    preFinalReductionArtifactPath,
    preFinalReductionArtifactSha256,
    "pre-final reduction",
  );
  assertOptionalArtifactPair(
    finalReductionArtifactPath,
    finalReductionArtifactSha256,
    "final reduction",
  );

  const winnerRunnerUpMargin = optionalFiniteNumber(
    row.winner_runner_up_margin ?? row.runner_up_margin,
    "winner / runner-up margin",
  );
  const selectionReportXlsxPath = optionalPath(
    row.selection_report_xlsx_path,
    "selection report XLSX path",
  );
  const finalReportXlsxPath = optionalPath(
    row.final_report_xlsx_path,
    "final report XLSX path",
  );
  const selectionReportBlocked =
    row.execution_blocked === true && row.blocked_stage === "selection_report";
  if (
    expected.winnerLockSha256 &&
    winnerLockSha256 !==
      requiredHash(expected.winnerLockSha256, "expected winner-lock SHA-256")
  ) {
    throw new Error("Temporal campaign response changed the winner lock.");
  }
  const repairableSelectionProjectionGap =
    state === "prepared" &&
    preFinalReductionArtifactPath !== null &&
    selectionReportXlsxPath === null &&
    completedSelectionShardCount < selectionShardCount;
  if (
    !!winnerCandidateId !== !!preFinalReductionArtifactPath ||
    (preFinalReductionArtifactPath !== null &&
      completedSelectionShardCount !== selectionShardCount &&
      !repairableSelectionProjectionGap &&
      row.execution_blocked !== true) ||
    (winnerRunnerUpMargin !== null &&
      (!winnerCandidateId || winnerRunnerUpMargin < 0)) ||
    (selectionReportXlsxPath !== null &&
      (!preFinalReductionArtifactPath || state === "prepared")) ||
    (finalReductionArtifactPath !== null &&
      !selectionReportXlsxPath &&
      !selectionReportBlocked) ||
    (finalReductionArtifactPath !== null &&
      completedFinalTestShardCount !== finalTestShardCount) ||
    (finalReportXlsxPath !== null &&
      (!finalReductionArtifactPath || state !== "final_test_complete")) ||
    (completedFinalTestShardCount > 0 &&
      !selectionReportXlsxPath &&
      !selectionReportBlocked)
  ) {
    throw new Error(
      "Temporal campaign response has inconsistent staged provenance.",
    );
  }

  if (
    state !== "prepared" &&
    (!winnerCandidateId ||
      !preFinalReductionArtifactPath ||
      (!selectionReportXlsxPath && !selectionReportBlocked) ||
      (completedSelectionShardCount !== selectionShardCount &&
        row.execution_blocked !== true))
  ) {
    throw new Error(
      "Temporal campaign response is missing completed selection evidence.",
    );
  }
  if (
    state === "final_test_complete" &&
    (!finalReductionArtifactPath ||
      !finalReportXlsxPath ||
      completedFinalTestShardCount !== finalTestShardCount)
  ) {
    throw new Error(
      "Temporal campaign response is missing completed final evidence.",
    );
  }

  if (typeof row.execution_blocked !== "boolean") {
    throw new Error(
      "Temporal campaign response has invalid execution-blocked state.",
    );
  }
  const executionBlocked = row.execution_blocked;
  let blockedStage: VcetTemporalBlockedStage | null = null;
  let blockedShardOrdinal: number | null = null;
  let blockedReasonCode: VcetTemporalBlockedReasonCode | null = null;
  if (executionBlocked) {
    if (
      row.blocked_stage !== "pre_final_reduction" &&
      row.blocked_stage !== "final_test_shard" &&
      row.blocked_stage !== "selection_report" &&
      row.blocked_stage !== "final_report"
    ) {
      throw new Error(
        "Temporal campaign response has invalid blocked-stage evidence.",
      );
    }
    blockedStage = row.blocked_stage;
    const reportBlocked =
      blockedStage === "selection_report" || blockedStage === "final_report";
    if (reportBlocked) {
      if (
        !REPORT_BLOCKED_REASON_CODES.has(
          row.blocked_reason_code as VcetTemporalBlockedReasonCode,
        )
      ) {
        throw new Error(
          "Temporal campaign response has invalid blocked-reason evidence.",
        );
      }
      blockedReasonCode =
        row.blocked_reason_code as VcetTemporalBlockedReasonCode;
    } else {
      if (row.blocked_reason_code !== DURABLE_STAGE_BLOCKED_REASON_CODE) {
        throw new Error(
          "Temporal campaign response has invalid blocked-reason evidence.",
        );
      }
      blockedReasonCode = DURABLE_STAGE_BLOCKED_REASON_CODE;
    }
    if (blockedStage === "final_test_shard") {
      if (!hasOwn(row, "blocked_shard_ordinal")) {
        throw new Error(
          "Temporal campaign response is missing the blocked final shard ordinal.",
        );
      }
      blockedShardOrdinal = responseCount(
        row.blocked_shard_ordinal,
        "blocked shard ordinal",
      );
      if (
        blockedShardOrdinal >= finalTestShardCount ||
        !winnerCandidateId ||
        !preFinalReductionArtifactPath
      ) {
        throw new Error(
          "Temporal campaign response has inconsistent blocked final-shard evidence.",
        );
      }
    } else if (hasOwn(row, "blocked_shard_ordinal")) {
      const blockedStageLabel =
        blockedStage === "pre_final_reduction"
          ? "pre-final"
          : blockedStage === "selection_report"
            ? "selection-report"
            : "final-report";
      throw new Error(
        `Temporal campaign response has inconsistent blocked ${blockedStageLabel} evidence.`,
      );
    }
    if (blockedStage === "selection_report") {
      const reportReceiptPoisonedBeforeSelectionReduction =
        blockedReasonCode === "report_receipt_invalid_or_foreign" &&
        state === "prepared" &&
        winnerCandidateId === null &&
        preFinalReductionArtifactPath === null &&
        selectionReportXlsxPath === null &&
        finalReductionArtifactPath === null &&
        finalReportXlsxPath === null;
      const stateMatchesRetainedArtifacts =
        state === "prepared"
          ? finalReductionArtifactPath === null && finalReportXlsxPath === null
          : state === "selection_complete"
            ? finalReductionArtifactPath !== null &&
              finalReportXlsxPath === null
            : finalReductionArtifactPath !== null &&
              finalReportXlsxPath !== null;
      const retainedSelectionReductionIsConsistent =
        !!winnerCandidateId &&
        !!preFinalReductionArtifactPath &&
        completedSelectionShardCount === selectionShardCount &&
        selectionReportXlsxPath === null &&
        stateMatchesRetainedArtifacts;
      if (
        !reportReceiptPoisonedBeforeSelectionReduction &&
        !retainedSelectionReductionIsConsistent
      ) {
        throw new Error(
          "Temporal campaign response has inconsistent blocked selection-report evidence.",
        );
      }
    } else if (blockedStage === "final_report") {
      const reportReceiptPoisonedBeforeFinalReduction =
        blockedReasonCode === "report_receipt_invalid_or_foreign" &&
        state === "selection_complete" &&
        !!winnerCandidateId &&
        !!preFinalReductionArtifactPath &&
        !!selectionReportXlsxPath &&
        finalReductionArtifactPath === null &&
        finalReportXlsxPath === null &&
        completedSelectionShardCount === selectionShardCount;
      const retainedFinalReductionIsConsistent =
        state === "selection_complete" &&
        !!winnerCandidateId &&
        !!preFinalReductionArtifactPath &&
        !!selectionReportXlsxPath &&
        !!finalReductionArtifactPath &&
        finalReportXlsxPath === null &&
        completedSelectionShardCount === selectionShardCount &&
        completedFinalTestShardCount === finalTestShardCount;
      if (
        !reportReceiptPoisonedBeforeFinalReduction &&
        !retainedFinalReductionIsConsistent
      ) {
        throw new Error(
          "Temporal campaign response has inconsistent blocked final-report evidence.",
        );
      }
    }
  } else if (
    hasOwn(row, "blocked_stage") ||
    hasOwn(row, "blocked_shard_ordinal") ||
    hasOwn(row, "blocked_reason_code")
  ) {
    throw new Error(
      "Temporal campaign response exposed blocked evidence while execution is resumable.",
    );
  }

  const nextRequiredAction = requiredText(
    row.next_required_action,
    "next required action",
    500,
  ) as VcetTemporalNextRequiredAction;
  const expectedNextRequiredAction: VcetTemporalNextRequiredAction =
    executionBlocked
      ? "start_new_campaign"
      : state === "prepared" &&
          completedSelectionShardCount < selectionShardCount
        ? completedSelectionShardCount === 0 && !preFinalReductionArtifactPath
          ? "run_selection"
          : "resume_selection"
        : preFinalReductionArtifactPath && !selectionReportXlsxPath
          ? "resume_selection_report"
          : finalReductionArtifactPath && !finalReportXlsxPath
            ? "confirm_and_resume_final_report"
            : state === "prepared"
              ? completedSelectionShardCount === 0
                ? "run_selection"
                : "resume_selection"
              : state === "selection_complete"
                ? completedFinalTestShardCount === 0
                  ? "confirm_and_open_final_test"
                  : "confirm_and_resume_final_test"
                : "none";
  if (nextRequiredAction !== expectedNextRequiredAction) {
    throw new Error(
      "Temporal campaign response has an inconsistent next required action.",
    );
  }

  const rawLimitations = row.limitations;
  if (!Array.isArray(rawLimitations) || rawLimitations.length > 64) {
    throw new Error("Temporal campaign response has invalid limitations.");
  }
  const limitations = rawLimitations.map((entry) =>
    requiredText(entry, "limitation", 500),
  );

  return {
    schemaVersion: 1,
    campaignId: requiredText(row.campaign_id, "campaign ID", 512),
    state,
    preparationArtifactPath,
    preparationArtifactSha256,
    sourceCampaignSummaryPath,
    sourceCampaignSummarySha256,
    universeSha256,
    candidateCount,
    datasetCount,
    selectionDatasetCount,
    finalTestDatasetCount,
    selectionShardCount,
    completedSelectionShardCount,
    finalTestShardCount,
    completedFinalTestShardCount,
    winnerCandidateId,
    winnerLockSha256,
    winnerRunnerUpMargin,
    preFinalReductionArtifactPath,
    preFinalReductionArtifactSha256,
    finalReductionArtifactPath,
    finalReductionArtifactSha256,
    selectionReportXlsxPath,
    finalReportXlsxPath,
    executionBlocked,
    blockedStage,
    blockedShardOrdinal,
    blockedReasonCode,
    nextRequiredAction,
    limitations,
  };
}

function requirePath(value: unknown, field: string): string {
  return requiredPath(value, field);
}

function requireExpectedHash(value: unknown, field: string): string {
  return requiredHash(value, field);
}

function requireActionState(
  status: VcetTemporalCampaignStatus,
  expectedState: VcetTemporalCampaignState,
  action: string,
  allowedBlockedStages: readonly VcetTemporalBlockedStage[] = [],
): VcetTemporalCampaignStatus {
  if (
    status.executionBlocked &&
    status.blockedStage !== null &&
    allowedBlockedStages.includes(status.blockedStage)
  ) {
    return status;
  }
  if (status.executionBlocked) {
    throw new Error(
      `Temporal campaign ${action} returned a terminal execution block instead of a new ${expectedState} campaign.`,
    );
  }
  if (status.state !== expectedState) {
    throw new Error(
      `Temporal campaign ${action} returned ${status.state}; expected ${expectedState}.`,
    );
  }
  return status;
}

export async function prepareVcetTemporalCampaign(input: {
  sourceCampaignSummaryPath: string;
  planDraft: VcetTemporalPlanDraft;
}): Promise<VcetTemporalCampaignStatus> {
  const sourceCampaignSummaryPath = requirePath(
    input.sourceCampaignSummaryPath,
    "source campaign summary path",
  );
  const built = buildVcetTemporalValidationPlan(input.planDraft);
  const req: Record<string, unknown> = {
    source_campaign_summary_path: sourceCampaignSummaryPath,
    plan: built.plan,
  };
  if (built.reportLabel) req.report_label = built.reportLabel;
  const response = await invokeCommand("prepare_vcet_temporal_campaign", {
    req,
  });
  return requireActionState(
    normalizeVcetTemporalCampaignStatus(response),
    "prepared",
    "preparation",
  );
}

export async function runVcetTemporalCampaignSelection(input: {
  preparationArtifactPath: string;
  expectedPreparationArtifactSha256: string;
}): Promise<VcetTemporalCampaignStatus> {
  const preparationArtifactPath = requirePath(
    input.preparationArtifactPath,
    "preparation artifact path",
  );
  const expectedPreparationArtifactSha256 = requireExpectedHash(
    input.expectedPreparationArtifactSha256,
    "expected preparation artifact SHA-256",
  );
  const response = await invokeCommand("run_vcet_temporal_campaign_selection", {
    req: {
      preparation_artifact_path: preparationArtifactPath,
      expected_preparation_artifact_sha256: expectedPreparationArtifactSha256,
    },
  });
  return requireActionState(
    normalizeVcetTemporalCampaignStatus(response, {
      preparationArtifactPath,
      preparationArtifactSha256: expectedPreparationArtifactSha256,
    }),
    "selection_complete",
    "selection",
    ["pre_final_reduction", "selection_report"],
  );
}

export async function openVcetTemporalCampaignFinalTest(input: {
  preparationArtifactPath: string;
  expectedPreparationArtifactSha256: string;
  expectedWinnerLockSha256: string;
}): Promise<VcetTemporalCampaignStatus> {
  const preparationArtifactPath = requirePath(
    input.preparationArtifactPath,
    "preparation artifact path",
  );
  const expectedPreparationArtifactSha256 = requireExpectedHash(
    input.expectedPreparationArtifactSha256,
    "expected preparation artifact SHA-256",
  );
  const expectedWinnerLockSha256 = requireExpectedHash(
    input.expectedWinnerLockSha256,
    "expected winner-lock SHA-256",
  );
  const response = await invokeCommand(
    "open_vcet_temporal_campaign_final_test",
    {
      req: {
        preparation_artifact_path: preparationArtifactPath,
        expected_preparation_artifact_sha256: expectedPreparationArtifactSha256,
        expected_winner_lock_sha256: expectedWinnerLockSha256,
        confirm_final_test_access: true,
      },
    },
  );
  return requireActionState(
    normalizeVcetTemporalCampaignStatus(response, {
      preparationArtifactPath,
      preparationArtifactSha256: expectedPreparationArtifactSha256,
      winnerLockSha256: expectedWinnerLockSha256,
    }),
    "final_test_complete",
    "final-test opening",
    ["final_test_shard", "final_report"],
  );
}

export async function getVcetTemporalCampaignStatus(input: {
  preparationArtifactPath: string;
  expectedPreparationArtifactSha256: string;
}): Promise<VcetTemporalCampaignStatus> {
  const preparationArtifactPath = requirePath(
    input.preparationArtifactPath,
    "preparation artifact path",
  );
  const expectedPreparationArtifactSha256 = requireExpectedHash(
    input.expectedPreparationArtifactSha256,
    "expected preparation artifact SHA-256",
  );
  const response = await invokeCommand("get_vcet_temporal_campaign_status", {
    req: {
      preparation_artifact_path: preparationArtifactPath,
      expected_preparation_artifact_sha256: expectedPreparationArtifactSha256,
    },
  });
  return normalizeVcetTemporalCampaignStatus(response, {
    preparationArtifactPath,
    preparationArtifactSha256: expectedPreparationArtifactSha256,
  });
}

export async function revealVcetTemporalCampaignPath(
  path: string,
): Promise<void> {
  await invokeCommand("reveal_path", {
    path: requirePath(path, "artifact path"),
  });
}

export async function openVcetTemporalCampaignWorkbook(
  path: string,
): Promise<void> {
  await openPathWithRevealFallback(requirePath(path, "workbook path"));
}
