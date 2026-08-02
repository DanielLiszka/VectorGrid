<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { storeToRefs } from "pinia";
import {
  buildVcetTemporalValidationPlan,
  type VcetTemporalFoldMode,
  type VcetTemporalPlanDraft,
  type VcetTemporalPositionBoundaryPolicy,
  type VcetTemporalSourceEligibilityIdentity,
} from "../services/vcetTemporalCampaignService";
import { useVcetTemporalCampaignStore } from "../stores/vcetTemporalCampaignStore";
import ContextHelpTip from "./ContextHelpTip.vue";
import AppButton from "./ui/AppButton.vue";

const props = withDefaults(
  defineProps<{
    sourceCampaignSummaryPath?: string;
    sourceEligible?: boolean;
    sourceEligibilityIdentity?: VcetTemporalSourceEligibilityIdentity | null;
    sourceEligibilityMessage?: string;
  }>(),
  {
    sourceCampaignSummaryPath: "",
    sourceEligible: false,
    sourceEligibilityIdentity: null,
    sourceEligibilityMessage: "",
  },
);

const store = useVcetTemporalCampaignStore();
const {
  busy,
  busyAction,
  error,
  notice,
  status,
  eligibleSourcePath,
  eligibleSourceIdentity,
  eligibilityMessage,
  currentSourceCampaignSummaryPath,
  currentPreparationArtifactPath,
  currentPreparationArtifactSha256,
  currentPreparationPlanDraft,
  planDraft,
  hasCurrentPreparation,
  draftDiffersFromActive,
  activePlanComparisonKnown,
  activeSourceComparisonKnown,
  sourceDiffersFromActive,
  canForgetCurrentPreparation,
  canRunSelection,
  canOpenFinalTest,
  winnerLockConfirmationSuffix,
} = storeToRefs(store);

const finalDisclosureAccepted = ref(false);
const winnerLockConfirmation = ref("");
const replacementAccepted = ref(false);
const forgetIdentityAccepted = ref(false);
const windowBarFields: ReadonlyArray<{
  key:
    | "trainBars"
    | "validationBars"
    | "finalTestBars"
    | "stepBars"
    | "purgeBars"
    | "embargoBars"
    | "warmupBars";
  label: string;
  minimum: number;
}> = [
  { key: "trainBars", label: "Train", minimum: 1 },
  { key: "validationBars", label: "Validation", minimum: 1 },
  { key: "finalTestBars", label: "Final test", minimum: 1 },
  { key: "stepBars", label: "Step", minimum: 1 },
  { key: "purgeBars", label: "Purge", minimum: 0 },
  { key: "embargoBars", label: "Embargo", minimum: 0 },
  { key: "warmupBars", label: "Warmup", minimum: 0 },
];

const planValidation = computed(() => {
  try {
    const built = buildVcetTemporalValidationPlan(planDraft.value);
    return {
      valid: true,
      message: `Minimum layout: ${built.minimumRequiredBars.toLocaleString()} bars per dataset before data-specific coverage checks.`,
    };
  } catch (validationError) {
    return {
      valid: false,
      message:
        validationError instanceof Error
          ? validationError.message
          : "Temporal plan is invalid.",
    };
  }
});

const hasVerifiedActivePreparation = computed(
  () => hasCurrentPreparation.value && !!status.value,
);
const replacementNeeded = computed(
  () =>
    hasVerifiedActivePreparation.value &&
    (draftDiffersFromActive.value || sourceDiffersFromActive.value),
);
const canPrepare = computed(() => {
  if (!eligibleSourcePath.value || !planValidation.value.valid || busy.value) {
    return false;
  }
  if (!hasCurrentPreparation.value) return true;
  return replacementNeeded.value && replacementAccepted.value;
});
const finalConfirmationMatches = computed(
  () =>
    !!winnerLockConfirmationSuffix.value &&
    winnerLockConfirmation.value.trim().toLowerCase() ===
      winnerLockConfirmationSuffix.value.toLowerCase(),
);
const canConfirmFinal = computed(
  () =>
    canOpenFinalTest.value &&
    finalDisclosureAccepted.value &&
    finalConfirmationMatches.value,
);
const isReportBlock = computed(
  () =>
    status.value?.executionBlocked === true &&
    (status.value.blockedStage === "selection_report" ||
      status.value.blockedStage === "final_report"),
);
const hasRetainedSelectionScientificEvidence = computed(() => {
  const currentStatus = status.value;
  return !!(
    currentStatus?.winnerCandidateId &&
    currentStatus.winnerLockSha256 &&
    currentStatus.preFinalReductionArtifactPath &&
    currentStatus.preFinalReductionArtifactSha256 &&
    currentStatus.completedSelectionShardCount ===
      currentStatus.selectionShardCount
  );
});
const hasRetainedFinalScientificEvidence = computed(() => {
  const currentStatus = status.value;
  return !!(
    currentStatus?.finalReductionArtifactPath &&
    currentStatus.finalReductionArtifactSha256 &&
    currentStatus.completedFinalTestShardCount ===
      currentStatus.finalTestShardCount
  );
});

function pathBasename(value: string | null | undefined): string {
  const normalized = String(value ?? "")
    .trim()
    .replace(/\\/gu, "/");
  if (!normalized) return "";
  return normalized.slice(normalized.lastIndexOf("/") + 1) || normalized;
}

function compactHash(value: string | null | undefined): string {
  const hash = String(value ?? "").replace(/^sha256:/iu, "");
  if (!hash) return "—";
  return hash.length > 20
    ? `sha256:${hash.slice(0, 8)}…${hash.slice(-8)}`
    : `sha256:${hash}`;
}

function formatCount(value: number | null | undefined): string {
  return Math.max(0, Number(value ?? 0)).toLocaleString();
}

function stageState(stage: "prepare" | "selection" | "final"): string {
  const currentStatus = status.value;
  const state = currentStatus?.state;
  if (!state) return stage === "prepare" ? "ready" : "pending";
  if (currentStatus.executionBlocked) {
    if (stage === "prepare") return "complete";
    if (currentStatus.blockedStage === "pre_final_reduction") {
      if (stage === "selection") return "blocked";
      return state === "final_test_complete" ? "complete" : "sealed";
    }
    if (currentStatus.blockedStage === "selection_report") {
      if (stage === "selection") return "blocked";
      return currentStatus.finalReductionArtifactPath ? "complete" : "sealed";
    }
    if (currentStatus.blockedStage === "final_report") {
      if (stage === "selection") {
        return hasRetainedSelectionScientificEvidence.value
          ? "complete"
          : "sealed";
      }
      return "blocked";
    }
    return stage === "selection" ? "complete" : "blocked";
  }
  if (stage === "prepare") return "complete";
  if (stage === "selection") {
    return state === "prepared" ? "ready" : "complete";
  }
  if (state === "final_test_complete") return "complete";
  return state === "selection_complete" ? "ready" : "sealed";
}

function updateTextField(key: keyof VcetTemporalPlanDraft, event: Event): void {
  const value = String((event.target as HTMLInputElement).value ?? "");
  store.setPlanField(key, value as never);
}

function updateFoldMode(event: Event): void {
  store.setPlanField(
    "foldMode",
    String((event.target as HTMLSelectElement).value) as VcetTemporalFoldMode,
  );
}

function updatePositionPolicy(event: Event): void {
  store.setPlanField(
    "positionBoundaryPolicy",
    String(
      (event.target as HTMLSelectElement).value,
    ) as VcetTemporalPositionBoundaryPolicy,
  );
}

async function confirmFinalTest(): Promise<void> {
  const completed = await store.openFinalTest({
    disclosureAccepted: finalDisclosureAccepted.value,
    winnerLockSuffix: winnerLockConfirmation.value,
  });
  if (completed) {
    finalDisclosureAccepted.value = false;
    winnerLockConfirmation.value = "";
  }
}

async function prepareDraft(): Promise<void> {
  const completed = await store.prepare({
    replaceActivePreparation:
      hasVerifiedActivePreparation.value && replacementAccepted.value,
  });
  if (completed) replacementAccepted.value = false;
}

function forgetCurrentPreparation(): void {
  if (
    store.forgetCurrentPreparation({
      confirmed: forgetIdentityAccepted.value,
    })
  ) {
    forgetIdentityAccepted.value = false;
  }
}

function blockedStageDescription(): string {
  const currentStatus = status.value;
  const blockedStage = currentStatus?.blockedStage;
  if (blockedStage === "final_test_shard") {
    const ordinal = currentStatus?.blockedShardOrdinal ?? null;
    return ordinal === null
      ? "a final-test shard"
      : `final-test shard ${ordinal + 1}`;
  }
  if (blockedStage === "selection_report") return "the selection report";
  if (blockedStage === "final_report") return "the final report";
  return "the pre-final reduction";
}

function reportBlockReasonDescription(): string {
  if (status.value?.blockedReasonCode === "report_xlsx_not_recoverable") {
    return "Its retained receipt is valid, but the missing or corrupt workbook could not be regenerated to the receipt's exact bytes and SHA-256.";
  }
  return hasRetainedSelectionScientificEvidence.value ||
    hasRetainedFinalScientificEvidence.value
    ? "Its receipt is invalid, foreign to the retained scientific evidence, or otherwise cannot be trusted."
    : "Its receipt is invalid, foreign to this preparation, or otherwise cannot be trusted.";
}

function retainedReportEvidenceDescription(): string {
  if (status.value?.blockedStage === "final_report") {
    if (hasRetainedFinalScientificEvidence.value) {
      return "The sealed final scientific reduction remains complete and reviewable; this report failure does not rerank or erase it.";
    }
    if (hasRetainedSelectionScientificEvidence.value) {
      return "The locked selection winner and pre-final scientific reduction remain complete and reviewable. No final scientific reduction was retained before this report failure, so no completed final evidence is claimed.";
    }
    return "The report receipt was rejected before a scientific reduction was retained. No completed selection or final evidence is claimed.";
  }
  if (hasRetainedFinalScientificEvidence.value) {
    return "The locked selection winner and retained final scientific reduction remain complete and reviewable; this report failure does not rerank or erase them.";
  }
  if (hasRetainedSelectionScientificEvidence.value) {
    return "The locked selection winner and pre-final scientific reduction remain complete and reviewable; this report failure does not rerank or erase them.";
  }
  return "The report receipt was rejected before a locked winner or pre-final scientific reduction was retained. No completed scientific selection evidence is claimed.";
}

function readableLimitation(value: string): string {
  const text = String(value || "").trim();
  return text ? text.replace(/_/gu, " ") : "Unspecified limitation";
}

watch(
  () =>
    [
      props.sourceEligible,
      props.sourceCampaignSummaryPath,
      props.sourceEligibilityIdentity?.nativePathKey ?? "",
      props.sourceEligibilityIdentity?.summaryContentFingerprint ?? "",
      props.sourceEligibilityMessage,
    ] as const,
  ([sourceEligible, sourcePath, , , sourceMessage]) => {
    store.setEligibleSource(
      sourceEligible ? sourcePath : "",
      sourceMessage,
      sourceEligible ? props.sourceEligibilityIdentity : null,
    );
  },
  { immediate: true },
);

watch(
  () => status.value?.winnerLockSha256,
  () => {
    finalDisclosureAccepted.value = false;
    winnerLockConfirmation.value = "";
  },
);

watch(
  [
    () => JSON.stringify(planDraft.value),
    () => eligibleSourcePath.value,
    () => eligibleSourceIdentity.value?.summaryContentFingerprint ?? "",
    () => currentPreparationArtifactSha256.value,
  ],
  () => {
    replacementAccepted.value = false;
  },
);

watch(
  () => canForgetCurrentPreparation.value,
  (canForget) => {
    if (!canForget) forgetIdentityAccepted.value = false;
  },
);

onMounted(async () => {
  await store.resumeAfterHydration();
});
</script>

<template>
  <section
    class="vcetTemporalPanel"
    aria-labelledby="vcetTemporalCampaignHeading"
    data-testid="vcet-temporal-campaign-host"
  >
    <header class="vcetTemporalHeader">
      <div>
        <div class="eyebrow">Bounded temporal evidence</div>
        <h3 id="vcetTemporalCampaignHeading">VCET temporal campaign</h3>
        <p>
          Seal one time-based research plan, select on train/validation
          evidence, then explicitly open the final test for the locked winner
          only.
        </p>
      </div>
      <ContextHelpTip
        title="Temporal campaign boundary"
        summary="Preparation and selection do not open final-test evidence."
        detail="The final command requires both a disclosure acknowledgement and the exact suffix of the persisted winner lock. Final evidence never reranks the candidate set."
        aria-label="VCET temporal campaign boundary help"
      />
    </header>

    <div class="boundedContract" role="note">
      <strong>Bounded desktop contract:</strong> at most 64 source datasets,
      with one resident dataset per execution shard, at most 2,000,000 timeline
      values per dataset, and 10,000,000 timeline values in aggregate. This is
      not the 10,000-stock provider-scale path; larger universes require
      separately sealed campaigns.
    </div>

    <div
      class="sourceCard"
      :class="{ eligible: !!eligibleSourcePath }"
      :data-source-eligible="!!eligibleSourcePath"
    >
      <div>
        <div class="fieldKey">
          Structurally eligible source · sealing verification pending
        </div>
        <div
          v-if="eligibleSourcePath"
          class="sourceValue mono"
          :title="eligibleSourcePath"
        >
          {{ pathBasename(eligibleSourcePath) }}
        </div>
        <div v-else class="sourceValue">Not available</div>
      </div>
      <p>{{ eligibilityMessage }}</p>
    </div>

    <section
      v-if="hasCurrentPreparation"
      class="activePreparation"
      aria-labelledby="vcetTemporalActivePreparationHeading"
    >
      <div class="activePreparationHeader">
        <div>
          <div class="eyebrow">Frozen execution context</div>
          <h4 id="vcetTemporalActivePreparationHeading">
            Active sealed preparation
          </h4>
        </div>
        <span
          class="activeStateChip"
          :class="{ blocked: status?.executionBlocked }"
        >
          {{
            status?.executionBlocked
              ? "execution blocked"
              : status?.state.replace(/_/gu, " ") || "unverified"
          }}
        </span>
      </div>
      <dl class="activePreparationGrid">
        <div>
          <dt>Source</dt>
          <dd
            class="mono"
            :title="currentSourceCampaignSummaryPath"
            data-testid="vcet-temporal-active-source"
          >
            {{ pathBasename(currentSourceCampaignSummaryPath) || "Unknown" }}
          </dd>
        </div>
        <div>
          <dt>Preparation identity</dt>
          <dd class="mono" :title="currentPreparationArtifactSha256">
            {{ compactHash(currentPreparationArtifactSha256) }}
          </dd>
        </div>
        <div class="activePlanCell">
          <dt>Locally recorded submitted-plan snapshot</dt>
          <dd
            v-if="currentPreparationPlanDraft"
            data-testid="vcet-temporal-active-plan"
          >
            {{ currentPreparationPlanDraft.foldMode }} ·
            {{ currentPreparationPlanDraft.trainBars }} train /
            {{ currentPreparationPlanDraft.validationBars }} validation /
            {{ currentPreparationPlanDraft.finalTestBars }} final bars ·
            {{ currentPreparationPlanDraft.startUtc }} →
            {{ currentPreparationPlanDraft.endUtc }} UTC. The preparation
            artifact remains authoritative.
          </dd>
          <dd v-else>
            Not retained by this legacy local pointer. The sealed preparation
            artifact remains authoritative.
          </dd>
        </div>
      </dl>
      <div
        v-if="
          draftDiffersFromActive ||
          sourceDiffersFromActive ||
          !activePlanComparisonKnown ||
          !activeSourceComparisonKnown
        "
        class="driftWarning"
        role="status"
        aria-live="polite"
        data-testid="vcet-temporal-drift-warning"
      >
        <strong>
          {{
            draftDiffersFromActive || sourceDiffersFromActive
              ? "Next-campaign changes detected."
              : "Identity comparison unavailable."
          }}
        </strong>
        <ul>
          <li v-if="sourceDiffersFromActive">
            The structurally eligible source has a different native path or
            retained summary-content identity from the active sealed source.
          </li>
          <li v-if="draftDiffersFromActive">
            The editable research draft differs from the active sealed plan.
          </li>
          <li v-if="!activePlanComparisonKnown">
            This legacy local pointer has no submitted-plan snapshot, so the
            next draft cannot be compared automatically.
          </li>
          <li v-if="!activeSourceComparisonKnown">
            This local pointer has no retained summary-content fingerprint, so
            matching path text alone cannot prove that the source is unchanged.
          </li>
        </ul>
        Active selection and final-test actions continue to use the frozen
        preparation above.
      </div>
    </section>

    <details class="planDisclosure" open>
      <summary>
        Next campaign draft <span>editable · defaults require review</span>
      </summary>
      <div class="planBody">
        <p id="vcetTemporalResearchDefaults" class="researchWarning">
          These conservative research defaults are a starting point, not a
          recommendation. Review them against timeframe, dataset coverage,
          market calendar, and expected trade frequency before preparation.
        </p>

        <fieldset
          :disabled="busy"
          aria-describedby="vcetTemporalResearchDefaults"
        >
          <legend>UTC bounds and fold semantics</legend>
          <div class="formGrid formGridWide">
            <label>
              <span>UTC start</span>
              <input
                id="vcetTemporalStartUtc"
                type="datetime-local"
                step="60"
                :value="planDraft.startUtc"
                @input="updateTextField('startUtc', $event)"
              />
            </label>
            <label>
              <span>UTC end</span>
              <input
                id="vcetTemporalEndUtc"
                type="datetime-local"
                step="60"
                :value="planDraft.endUtc"
                @input="updateTextField('endUtc', $event)"
              />
            </label>
            <label>
              <span>Fold mode</span>
              <select
                id="vcetTemporalFoldMode"
                :value="planDraft.foldMode"
                @change="updateFoldMode"
              >
                <option value="anchored">Anchored (training expands)</option>
                <option value="rolling">Rolling (fixed training length)</option>
              </select>
            </label>
            <label>
              <span>Position boundary</span>
              <select
                id="vcetTemporalPositionBoundary"
                :value="planDraft.positionBoundaryPolicy"
                @change="updatePositionPolicy"
              >
                <option value="reset_flat_discard_open">
                  Reset flat; discard open position
                </option>
              </select>
            </label>
          </div>
        </fieldset>

        <fieldset :disabled="busy">
          <legend>Window bars</legend>
          <div class="formGrid">
            <label v-for="field in windowBarFields" :key="field.key">
              <span>{{ field.label }} bars</span>
              <input
                :id="`vcetTemporal_${field.key}`"
                type="number"
                :min="field.minimum"
                max="2000000"
                step="1"
                inputmode="numeric"
                :value="planDraft[field.key]"
                @input="updateTextField(field.key, $event)"
              />
            </label>
          </div>
        </fieldset>

        <fieldset :disabled="busy">
          <legend>Minimum support</legend>
          <div class="formGrid">
            <label>
              <span>Folds</span>
              <input
                id="vcetTemporalMinFolds"
                type="number"
                min="2"
                max="10000"
                step="1"
                :value="planDraft.minFolds"
                @input="updateTextField('minFolds', $event)"
              />
            </label>
            <label>
              <span>Train bars</span>
              <input
                id="vcetTemporalMinTrainBars"
                type="number"
                min="1"
                max="2000000"
                step="1"
                :value="planDraft.minTrainBars"
                @input="updateTextField('minTrainBars', $event)"
              />
            </label>
            <label>
              <span>Validation bars</span>
              <input
                id="vcetTemporalMinValidationBars"
                type="number"
                min="1"
                max="2000000"
                step="1"
                :value="planDraft.minValidationBars"
                @input="updateTextField('minValidationBars', $event)"
              />
            </label>
            <label>
              <span>Final-test bars</span>
              <input
                id="vcetTemporalMinFinalBars"
                type="number"
                min="1"
                max="2000000"
                step="1"
                :value="planDraft.minFinalTestBars"
                @input="updateTextField('minFinalTestBars', $event)"
              />
            </label>
            <label>
              <span>Train trades / fold</span>
              <input
                id="vcetTemporalMinTrainTrades"
                type="number"
                min="1"
                max="1000000"
                step="1"
                :value="planDraft.minTrainTradesPerFold"
                @input="updateTextField('minTrainTradesPerFold', $event)"
              />
            </label>
            <label>
              <span>Validation trades / fold</span>
              <input
                id="vcetTemporalMinValidationTrades"
                type="number"
                min="1"
                max="1000000"
                step="1"
                :value="planDraft.minValidationTradesPerFold"
                @input="updateTextField('minValidationTradesPerFold', $event)"
              />
            </label>
            <label>
              <span>Final-test trades</span>
              <input
                id="vcetTemporalMinFinalTrades"
                type="number"
                min="1"
                max="1000000"
                step="1"
                :value="planDraft.minFinalTestTrades"
                @input="updateTextField('minFinalTestTrades', $event)"
              />
            </label>
            <label>
              <span>Report label</span>
              <input
                id="vcetTemporalReportLabel"
                type="text"
                maxlength="128"
                :value="planDraft.reportLabel"
                data-testid="vcet-temporal-report-label"
                @input="updateTextField('reportLabel', $event)"
              />
            </label>
          </div>
        </fieldset>

        <div
          class="validationLine"
          :class="{ invalid: !planValidation.valid }"
          role="status"
          aria-live="polite"
        >
          {{ planValidation.message }}
        </div>
        <div class="actionRow">
          <label
            v-if="replacementNeeded"
            class="confirmCheck replacementConfirmation"
          >
            <input
              v-model="replacementAccepted"
              type="checkbox"
              data-testid="vcet-temporal-replacement-confirmation"
              data-qa-action="aggregate.batch.vcetTemporal.confirmReplacement"
            />
            <span>
              Start a new sealed campaign and replace only the app's active
              local pointer. Existing evidence files are preserved; blocked or
              completed evidence is never rewritten.
            </span>
          </label>
          <p
            v-else-if="hasCurrentPreparation"
            class="replacementBlocked"
            role="status"
          >
            <template v-if="hasVerifiedActivePreparation">
              <template v-if="status?.executionBlocked">
                This blocked campaign has no confirmed plan or source-identity
                drift. Retrying its deterministic identity is disabled. Change
                the report label or research plan, or load a structurally
                eligible source with a different native/content identity, to
                unlock replacement.
              </template>
              <template v-else>
                This source and draft have no confirmed identity drift from the
                active campaign. Continue its next stage or change the plan or
                source identity before replacing it.
              </template>
            </template>
            <template v-else>
              Verify this saved preparation, or clear its stale local identity,
              before starting a new campaign.
            </template>
          </p>
          <AppButton
            variant="primary"
            data-testid="vcet-temporal-prepare"
            :disabled="!canPrepare"
            :disabled-reason="
              !eligibleSourcePath
                ? 'A compatible latest VCET campaign is required.'
                : !planValidation.valid
                  ? planValidation.message
                  : hasCurrentPreparation && !status
                    ? 'Verify or clear the stale saved preparation identity first.'
                    : hasVerifiedActivePreparation && !replacementNeeded
                      ? status?.executionBlocked
                        ? 'Change the report label, plan, or source native/content identity before replacing this deterministic blocked campaign.'
                        : 'Change the plan or source identity before replacing the active campaign.'
                      : replacementNeeded && !replacementAccepted
                        ? 'Confirm the explicit active-campaign replacement transition.'
                        : ''
            "
            :loading="busyAction === 'prepare'"
            loading-label="Preparing"
            data-qa-action="aggregate.batch.vcetTemporal.prepare"
            @click="prepareDraft"
          >
            {{
              replacementNeeded
                ? "Start and replace active campaign"
                : hasVerifiedActivePreparation
                  ? "Active campaign already prepared"
                  : "Prepare temporal campaign"
            }}
          </AppButton>
          <AppButton
            :disabled="!hasCurrentPreparation || busy"
            :loading="busyAction === 'refresh'"
            loading-label="Refreshing"
            data-qa-action="aggregate.batch.vcetTemporal.refreshStatus"
            @click="store.refreshStatus()"
          >
            Refresh status
          </AppButton>
        </div>
      </div>
    </details>

    <div v-if="error" class="feedback feedbackError" role="alert">
      {{ error }}
    </div>
    <div v-else-if="notice" class="feedback" role="status" aria-live="polite">
      {{ notice }}
    </div>

    <ol class="stageRail" aria-label="VCET temporal campaign stages">
      <li
        :data-stage-state="stageState('prepare')"
        data-testid="vcet-temporal-stage-prepare"
      >
        <div class="stageMarker" aria-hidden="true">1</div>
        <div class="stageBody">
          <div class="stageHeading">
            <div class="stageTitle">Prepare</div>
            <span class="stageStateLabel">{{ stageState("prepare") }}</span>
          </div>
          <p>
            Attest the exact source cohort, plan, candidate set, and universe.
          </p>
          <div v-if="status" class="countLine">
            {{ formatCount(status.candidateCount) }} candidates ·
            {{ formatCount(status.datasetCount) }} datasets
          </div>
        </div>
      </li>
      <li
        :data-stage-state="stageState('selection')"
        data-testid="vcet-temporal-stage-selection"
      >
        <div class="stageMarker" aria-hidden="true">2</div>
        <div class="stageBody">
          <div class="stageHeading">
            <div class="stageTitle">Selection</div>
            <span class="stageStateLabel">{{ stageState("selection") }}</span>
          </div>
          <p>Run sealed train/validation shards and lock one global winner.</p>
          <template v-if="status">
            <div class="countLine">
              {{ formatCount(status.completedSelectionShardCount) }} /
              {{ formatCount(status.selectionShardCount) }} selection shards ·
              {{ formatCount(status.selectionDatasetCount) }} datasets
            </div>
            <progress
              :value="status.completedSelectionShardCount"
              :max="Math.max(1, status.selectionShardCount)"
              aria-label="Selection shard completion"
            ></progress>
          </template>
        </div>
      </li>
      <li
        :data-stage-state="stageState('final')"
        data-testid="vcet-temporal-stage-final"
      >
        <div class="stageMarker" aria-hidden="true">3</div>
        <div class="stageBody">
          <div class="stageHeading">
            <div class="stageTitle">Sealed Final</div>
            <span class="stageStateLabel">{{ stageState("final") }}</span>
          </div>
          <p>Score only the locked winner. Final evidence never reranks.</p>
          <template v-if="status">
            <div class="countLine">
              {{ formatCount(status.completedFinalTestShardCount) }} /
              {{ formatCount(status.finalTestShardCount) }} final shards ·
              {{ formatCount(status.finalTestDatasetCount) }} datasets
            </div>
            <progress
              :value="status.completedFinalTestShardCount"
              :max="Math.max(1, status.finalTestShardCount)"
              aria-label="Final-test shard completion"
            ></progress>
          </template>
        </div>
      </li>
    </ol>

    <template v-if="status">
      <div class="statusGrid" aria-label="Temporal campaign status">
        <article>
          <div class="fieldKey">State</div>
          <div class="statusValue">{{ status.state.replace(/_/gu, " ") }}</div>
        </article>
        <article>
          <div class="fieldKey">Next required action</div>
          <div class="statusValue">
            {{ status.nextRequiredAction.replace(/_/gu, " ") }}
          </div>
        </article>
        <article>
          <div class="fieldKey">Winner</div>
          <div class="statusValue mono">
            {{ status.winnerCandidateId || "Not locked" }}
          </div>
        </article>
        <article v-if="status.winnerRunnerUpMargin !== null">
          <div class="fieldKey">Winner / runner-up margin</div>
          <div class="statusValue mono">
            {{ status.winnerRunnerUpMargin.toFixed(6) }}
          </div>
        </article>
      </div>

      <section
        v-if="status.executionBlocked"
        class="blockedBoundary"
        role="alert"
        aria-labelledby="vcetTemporalBlockedHeading"
        data-testid="vcet-temporal-execution-blocked"
      >
        <div>
          <div class="eyebrow">Durable terminal state</div>
          <h4 id="vcetTemporalBlockedHeading">This campaign cannot resume</h4>
        </div>
        <template v-if="isReportBlock">
          <p>
            {{ blockedStageDescription() }} is permanently blocked.
            {{ reportBlockReasonDescription() }} Selection and final-test
            execution are disabled for this preparation.
          </p>
          <p>{{ retainedReportEvidenceDescription() }}</p>
          <p data-testid="vcet-temporal-report-block-guidance">
            Review the retained evidence. To create a distinct replacement,
            change the report label, structurally eligible source identity, or
            validation plan. An identical retry is disabled, and this verified
            blocked pointer cannot be forgotten; evidence files are never
            deleted by this screen.
          </p>
        </template>
        <template v-else>
          <p>
            {{ blockedStageDescription() }} is permanently blocked because its
            durable stage record is failed, interrupted, or invalid. Selection
            and final-test execution are disabled for this preparation.
          </p>
          <p>
            Review the retained evidence. A replacement is available only after
            the next draft or structurally eligible source has a confirmed
            identity change. This verified blocked pointer cannot be forgotten
            merely to retry the same deterministic campaign; evidence files are
            never deleted by this screen.
          </p>
        </template>
      </section>

      <div class="actionRow stageActions">
        <AppButton
          v-if="!status.executionBlocked"
          variant="primary"
          data-testid="vcet-temporal-selection"
          data-qa-action="aggregate.batch.vcetTemporal.runSelection"
          :disabled="!canRunSelection"
          :loading="busyAction === 'selection'"
          loading-label="Running selection"
          @click="store.runSelection()"
        >
          {{
            status.nextRequiredAction === "resume_selection_report"
              ? "Resume selection report"
              : status.nextRequiredAction === "resume_selection"
                ? "Resume sealed selection"
                : "Run sealed selection"
          }}
        </AppButton>
        <AppButton
          :disabled="busy"
          data-qa-action="aggregate.batch.vcetTemporal.revealPreparation"
          @click="store.revealPath(status.preparationArtifactPath)"
        >
          Reveal preparation
        </AppButton>
        <AppButton
          :disabled="busy"
          data-qa-action="aggregate.batch.vcetTemporal.revealSource"
          @click="store.revealPath(status.sourceCampaignSummaryPath)"
        >
          Reveal source summary
        </AppButton>
        <AppButton
          v-if="status.preFinalReductionArtifactPath"
          :disabled="busy"
          data-qa-action="aggregate.batch.vcetTemporal.revealSelectionReduction"
          @click="store.revealPath(status.preFinalReductionArtifactPath)"
        >
          Reveal selection reduction
        </AppButton>
        <AppButton
          v-if="status.selectionReportXlsxPath"
          :disabled="busy"
          data-qa-action="aggregate.batch.vcetTemporal.openSelectionXlsx"
          @click="store.openWorkbook(status.selectionReportXlsxPath)"
        >
          Open selection XLSX
        </AppButton>
        <AppButton
          v-if="status.finalReductionArtifactPath"
          :disabled="busy"
          data-qa-action="aggregate.batch.vcetTemporal.revealFinalReduction"
          @click="store.revealPath(status.finalReductionArtifactPath)"
        >
          Reveal final reduction
        </AppButton>
        <AppButton
          v-if="status.finalReportXlsxPath"
          :disabled="busy"
          data-qa-action="aggregate.batch.vcetTemporal.openFinalXlsx"
          @click="store.openWorkbook(status.finalReportXlsxPath)"
        >
          Open final XLSX
        </AppButton>
      </div>

      <section
        v-if="status.state === 'selection_complete' && !status.executionBlocked"
        class="finalBoundary"
        aria-labelledby="vcetTemporalFinalBoundaryHeading"
      >
        <h4 id="vcetTemporalFinalBoundaryHeading">Final-test disclosure</h4>
        <p>
          This action irreversibly opens the sealed final-test cohort for the
          exact locked winner. It cannot be used to compare, replace, or rerank
          candidates. A weak final result remains final evidence.
        </p>
        <label class="confirmCheck">
          <input
            v-model="finalDisclosureAccepted"
            type="checkbox"
            data-testid="vcet-temporal-final-disclosure"
            data-qa-action="aggregate.batch.vcetTemporal.acceptFinalDisclosure"
          />
          <span
            >I understand that this opens final-test evidence once for the
            locked winner only.</span
          >
        </label>
        <label for="vcetTemporalWinnerLockConfirmation">
          <span>
            Type winner-lock suffix
            <code>{{ winnerLockConfirmationSuffix }}</code>
          </span>
          <input
            id="vcetTemporalWinnerLockConfirmation"
            v-model="winnerLockConfirmation"
            type="text"
            inputmode="text"
            autocomplete="off"
            autocapitalize="none"
            spellcheck="false"
            maxlength="8"
            data-testid="vcet-temporal-final-confirmation"
            data-qa-action="aggregate.batch.vcetTemporal.confirmWinnerSuffix"
          />
        </label>
        <AppButton
          variant="danger"
          data-testid="vcet-temporal-final-open"
          data-qa-action="aggregate.batch.vcetTemporal.openFinalTest"
          :disabled="!canConfirmFinal"
          disabled-reason="Accept the disclosure and type the exact winner-lock suffix."
          :loading="busyAction === 'final'"
          loading-label="Opening sealed final"
          @click="confirmFinalTest"
        >
          {{
            status.nextRequiredAction === "confirm_and_resume_final_report"
              ? "Confirm and resume final report"
              : status.nextRequiredAction === "confirm_and_resume_final_test"
                ? "Confirm and resume final test"
                : "Open sealed final test"
          }}
        </AppButton>
      </section>

      <section
        v-else-if="
          status.state === 'final_test_complete' ||
          (isReportBlock && hasRetainedFinalScientificEvidence)
        "
        class="finalComplete"
      >
        <strong>
          {{
            status.state !== "final_test_complete"
              ? "Retained final scientific reduction is complete."
              : status.executionBlocked
                ? "Retained final evidence is complete."
                : "Final evidence is complete."
          }}
        </strong>
        <template v-if="isReportBlock">
          The sealed final reduction remains reviewable; the report block does
          not rerank the winner or erase scientific evidence.
        </template>
        <template v-else-if="status.executionBlocked">
          Those late-stage artifacts remain available for review, while the
          durable execution block remains authoritative and non-resumable.
        </template>
        <template v-else>
          The pre-final winner remained locked; no final rerank occurred.
        </template>
      </section>

      <details class="provenanceDisclosure">
        <summary>Provenance and product limits</summary>
        <dl class="provenanceGrid">
          <div>
            <dt>Campaign</dt>
            <dd class="mono">{{ status.campaignId }}</dd>
          </div>
          <div>
            <dt>Preparation</dt>
            <dd class="mono" :title="status.preparationArtifactSha256">
              {{ compactHash(status.preparationArtifactSha256) }}
            </dd>
          </div>
          <div>
            <dt>Source summary</dt>
            <dd class="mono" :title="status.sourceCampaignSummarySha256">
              {{ compactHash(status.sourceCampaignSummarySha256) }}
            </dd>
          </div>
          <div>
            <dt>Universe</dt>
            <dd class="mono" :title="status.universeSha256">
              {{ compactHash(status.universeSha256) }}
            </dd>
          </div>
          <div v-if="status.winnerLockSha256">
            <dt>Winner lock</dt>
            <dd class="mono" :title="status.winnerLockSha256">
              {{ compactHash(status.winnerLockSha256) }}
            </dd>
          </div>
          <div v-if="status.preFinalReductionArtifactSha256">
            <dt>Selection reduction</dt>
            <dd class="mono" :title="status.preFinalReductionArtifactSha256">
              {{ compactHash(status.preFinalReductionArtifactSha256) }}
            </dd>
          </div>
          <div v-if="status.finalReductionArtifactSha256">
            <dt>Final reduction</dt>
            <dd class="mono" :title="status.finalReductionArtifactSha256">
              {{ compactHash(status.finalReductionArtifactSha256) }}
            </dd>
          </div>
        </dl>
        <ul class="limitationList">
          <li v-for="limitation in status.limitations" :key="limitation">
            {{ readableLimitation(limitation) }}
          </li>
        </ul>
      </details>
    </template>

    <section
      v-else-if="currentPreparationArtifactPath"
      class="staleStatus"
      aria-labelledby="vcetTemporalStaleHeading"
    >
      <strong id="vcetTemporalStaleHeading"
        >Saved preparation needs attention.</strong
      >
      <p>
        Its compact status could not be verified. Refresh if the artifact is
        still available. If it was moved, deleted, or corrupted, you can forget
        only this local pointer and start again; evidence files are never
        deleted by this action.
      </p>
      <label class="confirmCheck staleConfirmation">
        <input
          v-model="forgetIdentityAccepted"
          type="checkbox"
          data-testid="vcet-temporal-forget-confirmation"
          data-qa-action="aggregate.batch.vcetTemporal.confirmForgetStale"
        />
        <span>I understand this clears only the saved local identity.</span>
      </label>
      <AppButton
        variant="danger"
        data-testid="vcet-temporal-forget-stale"
        data-qa-action="aggregate.batch.vcetTemporal.forgetStale"
        :disabled="!canForgetCurrentPreparation || !forgetIdentityAccepted"
        disabled-reason="Confirm that only the stale local identity should be forgotten."
        @click="forgetCurrentPreparation"
      >
        Forget stale identity
      </AppButton>
    </section>
  </section>
</template>

<style scoped>
.vcetTemporalPanel {
  display: grid;
  gap: 1rem;
  min-width: 0;
  padding: clamp(0.9rem, 2vw, 1.25rem);
  border: 1px solid
    color-mix(in oklab, var(--border) 82%, var(--panel-focus-edge));
  border-radius: 14px;
  background:
    radial-gradient(
      circle at 92% 2%,
      color-mix(in oklab, var(--selection-rail) 13%, transparent),
      transparent 31%
    ),
    color-mix(in oklab, var(--surface2) 86%, var(--panel));
}

.vcetTemporalHeader {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.vcetTemporalHeader h3,
.finalBoundary h4 {
  margin: 0;
}

.vcetTemporalHeader p,
.finalBoundary p,
.sourceCard p,
.stageBody p {
  margin: 0.3rem 0 0;
  color: var(--muted);
  line-height: 1.5;
}

.eyebrow,
.fieldKey {
  color: var(--muted);
  font-size: 0.72rem;
  font-weight: 750;
  letter-spacing: 0.075em;
  text-transform: uppercase;
}

.boundedContract,
.researchWarning,
.feedback,
.finalComplete,
.staleStatus,
.driftWarning {
  padding: 0.78rem 0.9rem;
  border: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  border-radius: 10px;
  background: color-mix(in oklab, var(--surface-inline) 92%, transparent);
  line-height: 1.45;
}

.activePreparation {
  display: grid;
  gap: 0.75rem;
  padding: 0.9rem;
  border: 1px solid
    color-mix(in oklab, var(--selection-rail) 52%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--selection-rail) 7%, var(--panel));
}

.activePreparationHeader {
  display: flex;
  flex-wrap: wrap;
  gap: 0.7rem;
  align-items: flex-start;
  justify-content: space-between;
}

.activePreparationHeader h4 {
  margin: 0.15rem 0 0;
}

.activeStateChip {
  padding: 0.2rem 0.48rem;
  border: 1px solid currentColor;
  border-radius: 999px;
  color: var(--muted-strong);
  font-size: 0.67rem;
  font-weight: 780;
  letter-spacing: 0.045em;
  text-transform: uppercase;
}

.activeStateChip.blocked {
  color: var(--danger);
}

.activePreparationGrid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.6rem;
  margin: 0;
}

.activePreparationGrid > div {
  min-width: 0;
  padding: 0.65rem;
  border: 1px solid color-mix(in oklab, var(--border) 82%, transparent);
  border-radius: 8px;
  background: color-mix(in oklab, var(--surface-inline) 88%, transparent);
}

.activePreparationGrid .activePlanCell {
  grid-column: 1 / -1;
}

.activePreparationGrid dt {
  color: var(--muted);
  font-size: 0.72rem;
}

.activePreparationGrid dd {
  margin: 0.25rem 0 0;
  overflow-wrap: anywhere;
  line-height: 1.45;
}

.driftWarning {
  border-color: color-mix(in oklab, var(--warning) 58%, var(--border));
}

.driftWarning ul {
  margin: 0.35rem 0;
  padding-left: 1.2rem;
}

.blockedBoundary {
  display: grid;
  gap: 0.75rem;
  padding: 1rem;
  border: 2px solid color-mix(in oklab, var(--danger) 64%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--danger) 8%, var(--panel));
}

.blockedBoundary h4,
.blockedBoundary p {
  margin: 0;
}

.blockedBoundary p {
  color: var(--muted-strong);
  line-height: 1.5;
}

.blockedForgetConfirmation {
  max-width: 40rem;
}

.boundedContract {
  border-color: color-mix(in oklab, var(--warning) 52%, var(--border));
}

.sourceCard {
  display: grid;
  grid-template-columns: minmax(10rem, 0.8fr) minmax(14rem, 1.5fr);
  gap: 0.8rem 1rem;
  align-items: center;
  padding: 0.8rem 0.9rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel) 86%, transparent);
}

.sourceCard.eligible {
  border-color: color-mix(in oklab, var(--ok) 55%, var(--border));
}

.sourceValue,
.statusValue {
  margin-top: 0.2rem;
  overflow-wrap: anywhere;
  font-weight: 650;
}

.planDisclosure,
.provenanceDisclosure {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel) 90%, transparent);
}

.planDisclosure > summary,
.provenanceDisclosure > summary {
  cursor: pointer;
  padding: 0.78rem 0.9rem;
  font-weight: 720;
}

.planDisclosure > summary span {
  color: var(--warning);
  font-size: 0.78rem;
  font-weight: 650;
}

.planBody {
  display: grid;
  gap: 0.9rem;
  padding: 0 0.9rem 0.9rem;
}

fieldset {
  min-width: 0;
  margin: 0;
  padding: 0.8rem;
  border: 1px solid var(--border);
  border-radius: 9px;
}

legend {
  padding-inline: 0.35rem;
  color: var(--muted-strong);
  font-size: 0.8rem;
  font-weight: 700;
}

.formGrid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9.5rem, 1fr));
  gap: 0.72rem;
}

.formGridWide {
  grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
}

label {
  display: grid;
  gap: 0.34rem;
  min-width: 0;
  color: var(--muted-strong);
  font-size: 0.78rem;
  font-weight: 650;
}

input,
select {
  width: 100%;
  min-width: 0;
}

.validationLine {
  color: var(--muted-strong);
  font-size: 0.8rem;
}

.validationLine.invalid,
.feedbackError {
  color: var(--danger);
}

.actionRow {
  display: flex;
  flex-wrap: wrap;
  gap: 0.55rem;
  align-items: center;
}

.replacementConfirmation,
.replacementBlocked {
  flex-basis: 100%;
}

.replacementBlocked,
.staleStatus p {
  margin: 0;
  color: var(--muted-strong);
  line-height: 1.45;
}

.staleStatus {
  display: grid;
  gap: 0.7rem;
  border-color: color-mix(in oklab, var(--warning) 58%, var(--border));
}

.staleConfirmation {
  max-width: 38rem;
}

.stageRail {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 0.7rem;
  margin: 0;
  padding: 0;
  list-style: none;
}

.stageRail li {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.65rem;
  min-width: 0;
  padding: 0.8rem;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: color-mix(in oklab, var(--panel) 86%, transparent);
}

.stageRail li[data-stage-state="complete"] {
  border-color: color-mix(in oklab, var(--ok) 56%, var(--border));
}

.stageRail li[data-stage-state="ready"] {
  border-color: color-mix(in oklab, var(--selection-rail) 58%, var(--border));
}

.stageRail li[data-stage-state="sealed"] {
  border-style: dashed;
}

.stageRail li[data-stage-state="blocked"] {
  border-color: color-mix(in oklab, var(--danger) 64%, var(--border));
  background: color-mix(in oklab, var(--danger) 7%, var(--panel));
}

.stageMarker {
  display: grid;
  place-items: center;
  width: 1.8rem;
  height: 1.8rem;
  border-radius: 999px;
  background: var(--surface2);
  color: var(--muted-strong);
  font-weight: 800;
}

.stageTitle {
  font-weight: 760;
}

.stageHeading {
  display: flex;
  flex-wrap: wrap;
  gap: 0.4rem;
  align-items: center;
  justify-content: space-between;
}

.stageStateLabel {
  padding: 0.13rem 0.38rem;
  border: 1px solid currentColor;
  border-radius: 999px;
  color: var(--muted-strong);
  font-size: 0.64rem;
  font-weight: 780;
  letter-spacing: 0.045em;
  text-transform: uppercase;
}

.countLine {
  margin-top: 0.5rem;
  color: var(--muted-strong);
  font-size: 0.78rem;
  font-variant-numeric: tabular-nums;
}

progress {
  width: 100%;
  height: 0.5rem;
  margin-top: 0.45rem;
  accent-color: var(--selection-rail);
}

.statusGrid,
.provenanceGrid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(11rem, 1fr));
  gap: 0.65rem;
}

.statusGrid article,
.provenanceGrid > div {
  min-width: 0;
  padding: 0.7rem;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: color-mix(in oklab, var(--surface-inline) 90%, transparent);
}

.finalBoundary {
  display: grid;
  gap: 0.8rem;
  padding: 1rem;
  border: 2px solid color-mix(in oklab, var(--danger) 62%, var(--border));
  border-radius: 12px;
  background: color-mix(in oklab, var(--danger) 8%, var(--panel));
}

.confirmCheck {
  grid-template-columns: auto minmax(0, 1fr);
  align-items: start;
}

.confirmCheck input {
  width: auto;
  margin-top: 0.15rem;
}

.provenanceGrid {
  padding: 0 0.9rem 0.9rem;
}

.provenanceGrid dt {
  color: var(--muted);
  font-size: 0.72rem;
}

.provenanceGrid dd {
  margin: 0.25rem 0 0;
  overflow-wrap: anywhere;
}

.limitationList {
  margin: 0;
  padding: 0 1.8rem 0.9rem 2rem;
  color: var(--muted);
  font-size: 0.78rem;
  line-height: 1.45;
}

.mono {
  font-family: var(--font-mono);
  font-variant-numeric: tabular-nums;
}

@media (max-width: 880px) {
  .stageRail {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 620px) {
  .vcetTemporalHeader {
    flex-direction: column;
  }

  .sourceCard {
    grid-template-columns: 1fr;
  }

  .activePreparationGrid {
    grid-template-columns: 1fr;
  }

  .actionRow :deep(.appButton) {
    width: 100%;
  }
}
</style>
