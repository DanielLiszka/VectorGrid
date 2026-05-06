# Validation Evidence Pack Implementation Plan

Date: 2026-05-05

Status: Implemented locally

Implemented slice: app-core pack DTOs live in `crates/app_core/src/validation_shared.rs`, the canonical builder lives in `crates/app_core/src/validation_pipeline.rs`, Double-MA/composite validation attach `validation_evidence_pack`, cross-ticker post-processing refreshes the pack, run artifacts and validation reports parse/summarize it, and the frontend evidence summary prefers pack-backed data with legacy fallback.

## Goal

Create a first-class, artifact-backed validation evidence pack for VectorGrid validation runs.

The feature should make each validation run reviewable from three places without recomputing the run:

- the active validation workspace;
- saved run artifact detail and history views;
- generated reports and exports.

The pack is a compact, versioned, machine-readable evidence object derived from existing validation outputs. It should explain what was validated, which route actually ran, which evidence modules were requested and available, what the final verdict means, and how replay/report review should be treated.

This is not a new optimizer, not a ranking change, and not a GPU promotion feature. It is a trust and review feature.

## Product Name

Working name: **Artifact-Backed Evidence Review Pack**.

Rust/JSON contract name: `ValidationEvidencePackV1`.

Recommended serialized field name on validation result objects:

```json
{
  "validation_evidence_pack": {
    "version": 1
  }
}
```

## Current Repo Context

This plan is based on the current repository shape after the deep-research report was reviewed against code. The research recommendation remains directionally correct, but the implementation should build on pieces that already exist.

Current pieces already present:

- General validation already emits `ValidationRunVerdict` in `crates/app_core/src/validation_pipeline.rs`.
- Validation result types already carry `backend_route`, `backend_fallback_reason`, `evidence_policy_summary`, and `run_verdict` in Double-MA and composite validation paths.
- `src-tauri/src/commands_validation.rs` already rebuilds run verdict and evidence policy metadata for some cross-ticker preserved-finalist flows.
- The frontend already has a derived evidence model in `web/src/utils/validationEvidenceSummary.ts`.
- `web/src/components/ValidationWorkspacePanel.vue` already renders a compact overfit-evidence panel.
- Artifact persistence is centralized in `src-tauri/src/run_artifacts.rs`.
- Validation reports are built through `src-tauri/src/commands_reports_validation_common.rs` and `src-tauri/src/commands_reports_validation_xlsx.rs`.
- The artifact/report evidence contract lives in `docs/artifact_replay_report_evidence_contract.md`.
- Output parity authority lives in `scripts/run_output_parity_check.mjs`.

The new work should turn the existing derived, partly frontend-local evidence into a stable artifact/report contract.

## Research Basis

This section records the external research and official documentation used to harden the plan. The implementation should treat these as design constraints, not as new algorithm requirements.

| Source | Relevant finding | Implementation implication |
|---|---|---|
| [Bailey, Borwein, Lopez de Prado, and Zhu, *The Probability of Backtest Overfitting*](https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf) | CSCV/PBO evaluates a strategy selection process by comparing in-sample selection with out-of-sample ranks. | `pbo` must be labeled as candidate-set or selection-process evidence, not candidate-specific proof. The pack must carry candidate-universe and trial-count context. |
| [Bailey and Lopez de Prado, *The Deflated Sharpe Ratio*](https://www.davidhbailey.com/dhbpapers/deflated-sharpe.pdf) | DSR adjusts Sharpe evidence for selection bias, multiple testing, sample length, skew, and kurtosis. | DSR/PSR rows need explicit sample-unit, trial-count, and comparability caveats. If prerequisites are missing, use `not_comparable` or `unavailable`, not a fabricated score. |
| [Hansen, *A Test for Superior Predictive Ability*](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=264569) | SPA is a multiple-comparison test against a benchmark/comparison set and is more powerful than the Reality Check in some settings. | SPA must expose comparison-set, requested/skipped, and p-value availability status. Do not collapse it to a simple pass badge. |
| [White, *A Reality Check for Data Snooping*](https://econpapers.repec.org/article/ecmemetrp/v_3a68_3ay_3a2000_3ai_3a5_3ap_3a1097-1126.htm), plus [Sullivan/Timmermann/White technical-trading application](https://eprints.lse.ac.uk/119144/1/dp303.pdf) | Data-snooping-adjusted evidence depends on the full model/rule universe, not only the selected winner. | The pack must preserve the candidate universe definition and search exposure. Report copy should explain when only a finalist subset is known. |
| [Serde attributes documentation](https://serde.rs/attributes.html) | Serde supports field defaults and field/container/variant attributes for compatibility-oriented serialization. | Persisted pack fields must be additive and use `#[serde(default)]` where old artifacts need to load. Avoid `deny_unknown_fields` for durable artifact payloads. |
| [SchemaVer documentation](https://docs.snowplow.io/docs/api-reference/iglu/common-architecture/schemaver/) | Schema changes differ between additions compatible with all historical data, revisions that may not fit all historical data, and model-breaking changes. | Treat `version: 1` as the pack model version and add a string `schema_version` such as `1-0-0` for future auditability. Additive fields should be documented as additions, not model breaks. |
| [Tauri command-scope documentation](https://tauri.app/security/scope/) | Command-specific scopes must be enforced by implementation code, and bypasses need auditing. | The pack must not introduce new path-opening, report-scanning, or filesystem behavior outside existing local-path/report policies. |
| [NYSE](https://www.nyse.com/markets/hours-calendars), [Nasdaq](https://www.nasdaq.com/holiday-trading-hours), and [Cboe](https://www.cboe.com/about/hours/us-options/) hours/holiday references | Exchange calendars and session details are venue-specific and time-sensitive. | Dataset time fields are provenance labels in this feature. Do not infer trading eligibility or execution policy from the pack's market-time labels. |

## Research-Backed Robustness Requirements

The pack must satisfy these additional requirements:

- `pbo` and `cscv_pbo` entries must carry `scope = "candidate_set"` or `scope = "selection_process"`.
- Candidate-set metrics must include a caveat that they are not candidate-specific proof.
- DSR must expose whether the pack knows the number of trials, effective observations, and return-distribution assumptions used by the underlying result.
- SPA must identify whether it was not requested, available, skipped, or unavailable; available SPA evidence should include p-value and comparison-set context where current result fields expose it.
- A metric threshold without a value must render as `unavailable` or `not_comparable`, not as a failed metric.
- History/detail summaries must not recompute PBO, DSR, SPA, or report existence.
- Schema changes after MVP must be classified as `addition`, `revision`, or `model` in implementation notes.
- Any report-sheet addition must be paired with output-parity planning.
- Market-time fields must be explicitly labeled as display/provenance metadata until the separate calendar-service work consumes them.

## Why This Feature

VectorGrid already has a broad feature surface: optimization, validation, GPU routes, reports, artifacts, options, ML/meta, and live/paper execution surfaces. The highest leverage next feature is not more surface area. It is durable evidence.

This feature improves:

- trust in validation results;
- review quality for saved artifacts;
- report consistency;
- future GPU validation productization;
- Simple Mode and public-release explanation;
- ML/meta evidence reuse later;
- debugging when route policy, fallback, or optional evidence modules drift.

The core value is that a saved validation result should answer these questions without opening every raw result section:

- How many candidates and original optimization trials were involved?
- Which backend was requested, which backend actually ran, and did it fall back?
- Which evidence modules were requested?
- Which evidence modules were available, skipped, unavailable, unsupported, or not comparable?
- What did the final run verdict decide?
- Did the result pass hard gates, receive approval, or only remain inspectable?
- What dataset and market-time assumptions are relevant?
- Can this saved artifact be reopened as exact evidence, or is it legacy/review-only?
- Does a report exist, and is it known to exist on disk?

## Non-Goals

Do not change validation scoring, candidate ordering, tuple admission, or optimizer ranking.

Do not add new statistical methods.

Do not compute expensive evidence during history list refresh.

Do not scan report directories, cache directories, or raw datasets to populate history fields.

Do not force migration of old artifacts.

Do not make one generic `artifact_evidence` object required across every artifact family. The existing artifact evidence contract explicitly allows artifact-family-specific payloads.

Do not promote any GPU route based only on this feature.

Do not imply trading approval, robustness guarantees, or production suitability.

Do not add new external dependencies.

## Scope

### MVP Scope

The MVP covers validation result payloads produced by:

- `run_double_ma_validation`;
- `run_composite_validation`;
- fixed-candidate validation paths used by product-facing strategy profiles that route through those commands.

It should persist in saved run artifacts when validation results are saved through the normal run artifact path.

It should render in:

- `ValidationWorkspacePanel.vue`;
- saved artifact detail/history summaries where validation stage completion is shown;
- validation XLSX reports.

### Follow-Up Scope

After the MVP is stable:

- ML Signal validation can emit the same pack shape or a compatible ML-specific pack profile.
- Simple Mode can show a compact subset.
- Options-expression validation can receive an options-local pack variant that shares the availability vocabulary but includes options-specific dataset/quote fields.
- Report CSV parity can be added if validation-shortlist CSV becomes an output-parity claim.

## Contract Principles

1. **Durable, not decorative**

   The pack is saved with artifacts and consumed by reports. The UI can derive labels from it, but the pack itself should not be only a UI model.

2. **Additive only**

   Add the new field with `#[serde(default, skip_serializing_if = "Option::is_none")]`. Old artifacts remain readable.

3. **Explicit availability**

   Evidence must distinguish `not_requested`, `available`, `skipped`, `unavailable`, `unsupported`, `not_comparable`, and `legacy_absent`.

4. **No false approval**

   The pack can say `approved`, `not_approved`, `failed`, `incomplete`, or `review_only`. It must not use language like `safe`, `certified`, or `robust` as a final claim.

5. **No hidden recomputation**

   History and artifact list views derive from saved payload fields. They do not rerun validation or scan raw data.

6. **Route truth over request intent**

   Always show the realized route and fallback reason when available. Requested backend is secondary.

7. **Report parity before report expansion**

   Any report shape change must include output-parity planning and baselines.

8. **Machine-readable first**

   Store enums, numbers, thresholds, and notes. Let the frontend produce display text.

9. **Deterministic output**

   Avoid volatile fields such as `generated_at_utc` inside the pack unless they are already part of the saved run artifact identity. The pack should be stable for the same input result.

10. **Privacy-aware**

   Dataset/report paths should use already-approved artifact/report metadata. Do not introduce new full private path exposure.

11. **Preserve metric scope**

   Candidate metrics, run-level verdicts, candidate-set metrics, and selection-process metrics must be distinguishable. PBO and CSCV are not row-local candidate facts even when candidate rows carry per-candidate CSCV helper stats.

12. **Gate comparability**

   PSR, DSR, PBO, SPA, Monte Carlo, and walk-forward outputs should be marked `not_comparable` when their sample unit, comparison universe, split policy, or return-series assumptions are missing or incompatible.

13. **Version for evolution**

   `version` identifies the model family. `schema_version` records additive/revision/model-level evolution. New optional fields are additions; renaming required fields or changing enum meaning is a model break.

## Proposed Data Model

### Top-Level Shape

Rust module:

```text
crates/app_core/src/validation_shared.rs
crates/app_core/src/validation_pipeline.rs
```

Top-level type:

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidationEvidencePackV1 {
    pub version: u16,
    pub schema_version: String,
    pub workflow: ValidationEvidenceWorkflow,
    pub status: ValidationEvidencePackStatus,
    pub route: ValidationRouteEvidenceV1,
    pub candidate_universe: ValidationCandidateUniverseEvidenceV1,
    pub verdict: ValidationVerdictEvidenceV1,
    #[serde(default)]
    pub metrics: Vec<ValidationMetricEvidenceV1>,
    #[serde(default)]
    pub modules: Vec<ValidationModuleEvidenceV1>,
    pub replay: ValidationReplayEvidenceV1,
    pub dataset: ValidationDatasetEvidenceV1,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
}
```

### Enums

Use snake_case serialization for stable JSON and TypeScript mapping.

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationEvidenceWorkflow {
    GeneralValidation,
    DoubleMaValidation,
    CompositeValidation,
    MlSignalValidation,
    Unknown,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationEvidencePackStatus {
    Available,
    Incomplete,
    ReviewOnly,
    LegacyAbsent,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationEvidenceAvailability {
    NotRequested,
    Available,
    Skipped,
    Unavailable,
    Unsupported,
    NotComparable,
    LegacyAbsent,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationEvidenceTone {
    Neutral,
    Pass,
    Warn,
    Fail,
}
```

### Route Evidence

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ValidationRouteEvidenceV1 {
    pub requested_backend: String,
    pub realized_backend: String,
    pub backend_route: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_fallback_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residency_tier: Option<String>,
    #[serde(default)]
    pub route_diagnostics: Vec<String>,
}
```

Field sources:

| Field | Source |
|---|---|
| `requested_backend` | validation request backend or result `requested_backend` when present |
| `realized_backend` | result `backend_used` |
| `backend_route` | result `backend_route` |
| `backend_fallback_reason` | result `backend_fallback_reason` |
| `residency_tier` | result `validation_residency_tier` where present |
| `route_diagnostics` | composite route diagnostics where present |

### Candidate Universe Evidence

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ValidationCandidateUniverseEvidenceV1 {
    pub candidate_source: String,
    pub objective: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optimization_trials: Option<usize>,
    pub candidates_returned: usize,
    pub candidates_tested: usize,
    pub candidates_survived_hard_gates: usize,
    pub approved_candidate_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_approved_rank: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_policy: Option<String>,
}
```

Field sources:

| Field | Source |
|---|---|
| `candidate_source` | request/result context: optimization finalists, fixed candidates, saved artifact replay, cross rerun |
| `objective` | optimization request/result objective if available |
| `optimization_trials` | optimization result `num_combos`, `evaluations`, or equivalent where available |
| `candidates_returned` | result candidates length |
| `candidates_tested` | `run_verdict.candidates_tested` |
| `candidates_survived_hard_gates` | `run_verdict.candidates_survived_hard_gates` |
| `approved_candidate_count` | `run_verdict.approved_candidate_count` |
| `first_approved_rank` | `run_verdict.first_approved_rank` |
| `admission_policy` | validation request admission policy where present |

### Verdict Evidence

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ValidationVerdictEvidenceV1 {
    pub passed: bool,
    pub approved: bool,
    pub status_label: String,
    pub final_recommendation: String,
    #[serde(default)]
    pub warning_flags: Vec<String>,
    #[serde(default)]
    pub fail_reasons: Vec<String>,
}
```

Recommended status derivation:

| Condition | `status_label` |
|---|---|
| `passed == true` and `approved == true` | `approved` |
| `passed == true` and `approved == false` | `not_approved` |
| `passed == false` | `failed` |
| no verdict in legacy fallback | `review_only` |

### Metric Evidence

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidationMetricEvidenceV1 {
    pub metric: String,
    pub label: String,
    pub scope: String,
    pub availability: ValidationEvidenceAvailability,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold_direction: Option<String>,
    pub gate_role: String,
    pub sample_unit: String,
    pub metric_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparability_basis: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    #[serde(default)]
    pub caveats: Vec<String>,
}
```

Initial metric rows:

| Metric | Value Source | Threshold Source | Notes |
|---|---|---|---|
| `psr` | `run_verdict.top_candidate_psr` or top candidate `psr` | request `min_psr` | scope `candidate`; raw bars caveat from `EvidencePolicySummary` |
| `dsr` | `run_verdict.top_candidate_dsr` or top candidate `dsr` | request `min_dsr` | scope `candidate`; include trial/sample-unit comparability caveat |
| `pbo` | `run_verdict.pbo` or `cscv_pbo.pbo` | request/result `max_pbo` | scope `candidate_set` or `selection_process`; not candidate-specific proof |
| `min_track_record_length` | top candidate observed/support fields | internal minimum | scope `candidate`; recommendation layer, not always hard gate |
| `spa_p_value` | `spa.p_value` | SPA config threshold if present | scope `comparison_set`; availability can be skipped |
| `robust_score` | top candidate ranking score if present | none | scope `candidate`; ranking-only, not a hard gate |

Do not invent values when not present. Use availability states instead.

### Module Evidence

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ValidationModuleEvidenceV1 {
    pub module: String,
    pub label: String,
    pub availability: ValidationEvidenceAvailability,
    pub requested: bool,
    pub supported_on_route: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skipped_reason: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}
```

Initial modules:

- `walk_forward`;
- `reopt_walk_forward`;
- `cscv_pbo`;
- `spa`;
- `monte_carlo`;
- `trade_bootstrap`;
- `perturb`;
- `cross_ticker`;
- `friction_stress`;
- `regime_slice_robustness`.

The builder should not require every validation family to support every module. Use `unsupported` or `not_requested`.

### Replay Evidence

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ValidationReplayEvidenceV1 {
    pub replay_class: String,
    pub artifact_schema_version: String,
    pub report_state: String,
    pub compatibility: String,
    pub exact_evidence_review: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_xlsx_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_xlsx_exists: Option<bool>,
}
```

Use existing replay classes from `docs/artifact_replay_report_evidence_contract.md`:

- `exact_evidence_review`;
- `rerunnable_with_current_inputs`;
- `review_only`;
- `blocked`.

For the MVP, artifact list/detail should use saved artifact fields to populate report state. Do not add report directory scans to normal history refresh.

### Dataset Evidence

```rust
#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct ValidationDatasetEvidenceV1 {
    pub dataset_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dataset_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeframe: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_time_zone: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_calendar_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_time_basis: Option<String>,
}
```

Dataset fields are intentionally lightweight in the MVP. Reuse existing dataset metadata and market-time helpers rather than introducing a calendar service dependency in this feature.

## Example JSON

Illustrative shape:

```json
{
  "version": 1,
  "schema_version": "1-0-0",
  "workflow": "double_ma_validation",
  "status": "available",
  "route": {
    "requested_backend": "gpu-kernel",
    "realized_backend": "Gpu",
    "backend_route": "GpuKernel",
    "backend_fallback_reason": null,
    "residency_tier": "gpu_kernel_resident",
    "route_diagnostics": []
  },
  "candidate_universe": {
    "candidate_source": "optimization_finalists",
    "objective": "sharpe",
    "optimization_trials": 58300,
    "candidates_returned": 25,
    "candidates_tested": 25,
    "candidates_survived_hard_gates": 4,
    "approved_candidate_count": 1,
    "first_approved_rank": 2,
    "admission_policy": "preserve_top_k"
  },
  "verdict": {
    "passed": true,
    "approved": true,
    "status_label": "approved",
    "final_recommendation": "approved",
    "warning_flags": ["high_pbo"],
    "fail_reasons": []
  },
  "metrics": [
    {
      "metric": "psr",
      "label": "PSR",
      "scope": "candidate",
      "availability": "available",
      "value": 0.91,
      "threshold": 0.8,
      "threshold_direction": "min",
      "gate_role": "hard_gate",
      "sample_unit": "raw_bars",
      "metric_class": "confidence_bounded",
      "source": "run_verdict.top_candidate_psr",
      "caveats": [
        "General validation PSR currently uses raw bar counts unless a later effective-count audit supplies a narrower unit."
      ]
    },
    {
      "metric": "pbo",
      "label": "CSCV/PBO",
      "scope": "candidate_set",
      "availability": "available",
      "value": 0.12,
      "threshold": 0.3,
      "threshold_direction": "max",
      "gate_role": "hard_gate",
      "sample_unit": "candidate_set_slices",
      "metric_class": "approximate",
      "source": "cscv_pbo.pbo",
      "comparability_basis": "same candidate set and CSCV slice policy",
      "caveats": [
        "CSCV/PBO is set-level overfit evidence, not a candidate-specific proof."
      ]
    }
  ],
  "modules": [
    {
      "module": "cscv_pbo",
      "label": "CSCV/PBO",
      "availability": "available",
      "requested": true,
      "supported_on_route": true,
      "summary": "evaluated"
    },
    {
      "module": "spa",
      "label": "SPA",
      "availability": "not_requested",
      "requested": false,
      "supported_on_route": true
    }
  ],
  "replay": {
    "replay_class": "exact_evidence_review",
    "artifact_schema_version": "current",
    "report_state": "not_exported",
    "compatibility": "supported",
    "exact_evidence_review": true
  },
  "dataset": {
    "dataset_id": "sample:vectorgrid_synthetic_ohlcv",
    "timeframe": "1m",
    "market_time_zone": "America/New_York",
    "raw_time_basis": "utc"
  },
  "warnings": [],
  "notes": []
}
```

## Implementation Architecture

### App-Core Builder

Add a dedicated module:

```text
crates/app_core/src/validation_shared.rs
crates/app_core/src/validation_pipeline.rs
```

Export it from:

```text
crates/app_core/src/lib.rs
```

Recommended builder API:

```rust
pub struct ValidationEvidencePackBuildInput<'a> {
    pub workflow: ValidationEvidenceWorkflow,
    pub requested_backend: Option<&'a str>,
    pub backend_used: Option<&'a str>,
    pub backend_route: Option<&'a str>,
    pub backend_fallback_reason: Option<&'a str>,
    pub residency_tier: Option<&'a str>,
    pub route_diagnostics: &'a [String],
    pub candidate_count: usize,
    pub optimization_trials: Option<usize>,
    pub objective: Option<&'a str>,
    pub admission_policy: Option<&'a str>,
    pub hard_gates: &'a ValidationHardGateConfig,
    pub run_verdict: Option<&'a ValidationRunVerdict>,
    pub evidence_policy_summary: Option<&'a EvidencePolicySummary>,
    pub cscv_pbo: Option<&'a CscvPboResult>,
    pub spa: Option<&'a SpaValidationStats>,
    pub dataset: ValidationDatasetEvidenceV1,
}

pub fn build_validation_evidence_pack(
    input: ValidationEvidencePackBuildInput<'_>,
) -> ValidationEvidencePackV1
```

Keep this builder side-effect-free and deterministic.

### Attach To Validation Results

Add optional fields to validation result structs:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub validation_evidence_pack: Option<ValidationEvidencePackV1>,
```

Initial target result structs:

- `DoubleMaValidationResult` in `crates/app_core/src/double_ma_validate.rs`;
- composite validation result type in `crates/app_core/src/composite_validate.rs`;
- later: `MlSignalValidationResult` in `crates/app_core/src/ml_signal.rs`.

Build the pack after:

- candidates are finalized;
- `run_verdict` is final;
- `evidence_policy_summary` is final;
- backend route/fallback fields are final;
- cross-ticker preserved-finalist filtering has recomputed the run verdict.

### Tauri Command Boundary

Primary file:

```text
src-tauri/src/commands_validation.rs
```

Responsibilities:

- preserve the pack when app-core result already includes it;
- rebuild or refresh it after command-layer transformations that change final verdict, candidate count, or cross-ticker filtering;
- ensure cross-ticker preserved-finalist paths update `validation_evidence_pack` after they rebuild `run_verdict`;
- ensure payload stays additive for frontend command compatibility.

Avoid adding frontend-only evidence fields at the Tauri boundary. The Tauri layer should carry the stable pack and the existing raw result fields.

### Artifact Persistence

Primary file:

```text
src-tauri/src/run_artifacts.rs
```

Artifacts should persist the pack inside the saved validation result object rather than duplicating it at an unrelated top-level location.

Expected saved paths depend on the artifact shape, but common accessors should check:

- `/run/res/validation_evidence_pack`;
- `/validation/res/validation_evidence_pack`;
- legacy fallback from `/run/res/run_verdict`, `/run/res/backend_route`, and related fields.

History/list/detail summary work:

- extend stage-completion extraction to prefer `validation_evidence_pack` when present;
- keep existing fallback extraction from `run_verdict` for older artifacts;
- do not recompute evidence or scan reports during list refresh;
- do not duplicate large metric arrays into list rows unless needed.

Recommended summary fields for list/detail:

```json
{
  "validation_evidence_status": "available",
  "validation_evidence_route": "GpuKernel",
  "validation_evidence_verdict": "approved",
  "validation_evidence_modules": "4/5 available",
  "validation_evidence_warnings": 1
}
```

Use these as derived summary fields, not as the source of truth.

### Reports

Primary files:

```text
src-tauri/src/commands_reports_validation_common.rs
src-tauri/src/commands_reports_validation_xlsx.rs
src-tauri/tests/validation_xlsx_report_tests.rs
```

Implementation approach:

1. Parse `validation_evidence_pack` in the validation report model.
2. Preserve existing report compatibility mode for old artifacts without a pack.
3. Add a compact evidence section to the Summary sheet.
4. Add a dedicated `Evidence` sheet only if the report output-parity plan is updated in the same implementation batch.
5. Ensure old artifacts still generate reports via legacy fallback.

Minimum Summary sheet rows:

- evidence pack version;
- evidence status;
- requested backend;
- realized route;
- fallback reason;
- verdict status;
- candidates tested;
- hard-gate survivors;
- approved candidates;
- PSR availability/value/gate;
- DSR availability/value/gate;
- PBO availability/value/gate;
- SPA availability/status;
- replay class;
- report state.

If a dedicated `Evidence` sheet is added, include:

- one row per metric;
- one row per module;
- warnings and caveats;
- route diagnostics.

### Frontend Normalization

Existing file:

```text
web/src/utils/validationEvidenceSummary.ts
```

Recommended options:

1. Extend this existing file to prefer `validation_evidence_pack` and retain legacy fallback.
2. Or split a new parser into:

```text
web/src/utils/validationEvidencePack.ts
```

and keep `validationEvidenceSummary.ts` as the presentation adapter.

Preferred approach:

- add `normalizeValidationEvidencePack(raw: unknown): ValidationEvidencePack | null`;
- update `buildValidationEvidenceSummary` to use normalized pack when present;
- preserve all current tests for legacy result shapes;
- add tests for complete, partial, and legacy-absent pack states.

The frontend should derive:

- summary line;
- overfit/evidence item grid;
- route badge;
- module availability counts;
- tooltip/title copy;
- history detail labels.

Do not persist frontend labels back into artifacts.

### Frontend UI

Primary files:

```text
web/src/components/ValidationWorkspacePanel.vue
web/src/components/ValidationWorkspacePanel.test.ts
web/src/services/validationService.ts
web/src/services/validationService.test.ts
```

Current UI already has an overfit evidence panel. The implementation should refine it rather than add a competing panel.

Recommended UI changes:

- The existing Evidence line should come from the pack when present.
- The overfit panel should show pack-backed metric/module items.
- Add route/fallback badge if not already clear in the validation summary.
- Add module availability count such as `Evidence modules 5/6 available`.
- For missing advanced metrics, display `not requested`, `skipped`, `not comparable`, or `legacy absent`, not `failed`.
- Keep compact layout. Avoid making the validation workspace a report viewer.

Recommended status labels:

| Pack State | UI Label |
|---|---|
| `available` | `Evidence available` |
| `incomplete` | `Evidence incomplete` |
| `review_only` | `Review only` |
| `legacy_absent` | `Legacy evidence` |

Metric item labels:

- `PSR`;
- `DSR`;
- `CSCV/PBO`;
- `SPA`;
- `Track record`;
- `Walk-forward`;
- `Distribution`;
- `Friction`.

### History And Artifact UI

Primary files:

```text
web/src/services/historyService.ts
web/src/stores/workflowArtifactsStore.ts
web/src/components/HistoryWorkspacePanel.vue
```

Recommended behavior:

- In artifact list rows, show a compact validation evidence badge only when validation exists.
- In artifact detail, show the same summary model used by the validation workspace.
- For old artifacts, show `Legacy evidence` and derive what can be derived from `run_verdict`.
- If a report path is recorded but missing, show report state from existing report metadata. Do not initiate scanning from the UI.

### TypeScript Types

Add or extend types close to normalization code:

```ts
export type ValidationEvidenceAvailability =
  | "not_requested"
  | "available"
  | "skipped"
  | "unavailable"
  | "unsupported"
  | "not_comparable"
  | "legacy_absent";

export interface ValidationEvidencePackV1 {
  version: 1;
  schemaVersion: string;
  workflow: string;
  status: string;
  route: ValidationRouteEvidenceV1;
  candidateUniverse: ValidationCandidateUniverseEvidenceV1;
  verdict: ValidationVerdictEvidenceV1;
  metrics: ValidationMetricEvidenceV1[];
  modules: ValidationModuleEvidenceV1[];
  replay: ValidationReplayEvidenceV1;
  dataset: ValidationDatasetEvidenceV1;
  warnings: string[];
  notes: string[];
}
```

Normalize snake_case Rust fields to camelCase in frontend services, following existing frontend conventions.

## Detailed Field Mapping

| Pack Area | Current Field/Input | Owner |
|---|---|---|
| Requested backend | validation request backend | `web/src/services/validationRequestBuilder.ts`, Tauri request structs |
| Realized backend | `backend_used` | app-core validation result |
| Route | `backend_route` | `double_ma_validate.rs`, `composite_validate.rs` |
| Fallback reason | `backend_fallback_reason` | app-core validation result |
| Route diagnostics | `route_diagnostics` | composite validation result |
| Residency tier | `validation_residency_tier` where present | app-core validation result |
| Candidates tested | `run_verdict.candidates_tested` | `validation_pipeline.rs` |
| Hard-gate survivors | `run_verdict.candidates_survived_hard_gates` | `validation_pipeline.rs` |
| Approved count | `run_verdict.approved_candidate_count` | `validation_pipeline.rs` |
| First approved rank | `run_verdict.first_approved_rank` | `validation_pipeline.rs` |
| PSR | `run_verdict.top_candidate_psr`, candidate `psr` | `validation_pipeline.rs` |
| DSR | `run_verdict.top_candidate_dsr`, candidate `dsr` | `validation_pipeline.rs` |
| PBO | `run_verdict.pbo`, `cscv_pbo.pbo` | `validation_pipeline.rs`, `cscv_pbo.rs` |
| SPA | validation `spa` result | `validation_pipeline.rs` |
| Metric scope | builder-defined from metric type | `validation_pipeline.rs` |
| Metric comparability basis | `evidence_policy_summary`, request module settings, and known result prerequisites | `validation_pipeline.rs` |
| Metric caveats | `evidence_policy_summary.metrics[]` | `validation_pipeline.rs` |
| Verdict flags | `run_verdict.warning_flags`, `fail_reasons` | `validation_pipeline.rs` |
| Report state | `report_xlsx_path`, `report_xlsx_exists` | `run_artifacts.rs`, report commands |
| Replay class | existing artifact/replay classification | `run_artifacts.rs`, artifact contract |
| Dataset id/hash | existing dataset metadata where available | `state.rs`, `run_artifacts.rs`, frontend stores |
| Market time zone | frontend/app metadata, currently `America/New_York` where display semantics apply | `web/src/utils/marketTime.ts` |

## Implementation Phases

### Phase 0: Baseline And Fixtures

Purpose: avoid changing behavior without knowing current output shape.

Tasks:

- Identify one Double-MA validation fixture with optional evidence modules.
- Identify one composite validation fixture with route diagnostics or GPU/CPU route labels.
- Identify one legacy/minimal validation artifact fixture without `run_verdict`.
- Capture current frontend evidence summary test expectations.
- Capture current validation XLSX report test expectations.
- Check whether current output parity pairs include validation reports impacted by this change.

Deliverables:

- a short implementation note in the PR description;
- no product code changes yet except optional fixture comments if needed.

### Phase 1: App-Core Pack Builder

Purpose: create the stable Rust contract and deterministic builder.

Tasks:

- Add shared pack DTOs in `crates/app_core/src/validation_shared.rs`.
- Define all DTOs and enums with serde snake_case.
- Add `build_validation_evidence_pack`.
- Add helpers for:
  - route evidence;
  - candidate universe evidence;
  - verdict evidence;
  - metric evidence;
  - module evidence;
  - replay defaults;
  - dataset evidence defaults.
- Export the module/types in `crates/app_core/src/lib.rs`.
- Add unit tests for:
  - no optional modules requested;
  - CSCV/PBO requested and available;
  - CSCV/PBO requested but skipped;
  - SPA requested but skipped;
  - fallback route;
  - missing run verdict legacy/review-only behavior;
  - serde roundtrip.

Acceptance criteria:

- Builder is deterministic.
- No validation score changes.
- CPU-only builds do not require CUDA.
- Old result structs can still deserialize without the field.

### Phase 2: Attach Pack To Validation Results

Purpose: make new validation runs emit the pack.

Tasks:

- Add `validation_evidence_pack` optional field to Double-MA validation result.
- Build the pack at the final Double-MA result construction site.
- Add `validation_evidence_pack` optional field to composite validation result.
- Build the pack at the final composite result construction site.
- Ensure route/fallback metadata is final before building.
- Ensure run verdict is final before building.
- Add targeted tests in:
  - `crates/app_core/src/double_ma_validate.rs`;
  - `crates/app_core/src/composite_validate.rs`.

Acceptance criteria:

- Double-MA validation result JSON includes the pack for new runs.
- Composite validation result JSON includes the pack for new runs.
- Existing tests that inspect route and run verdict still pass.
- Pack and raw fields agree.

### Phase 3: Command-Layer Rebuild After Transformations

Purpose: keep evidence truthful when Tauri command code modifies validation results.

Tasks:

- Update `src-tauri/src/commands_validation.rs` helper paths that recompute `run_verdict`.
- After cross-ticker preserved-finalist filtering, rebuild `validation_evidence_pack`.
- Add tests that verify:
  - filtered candidates update candidate counts;
  - PBO remains set-level when intended;
  - fallback route remains accurate;
  - pack warning/fail states match recomputed verdict.

Acceptance criteria:

- No stale pack after command-layer filtering.
- Existing cross-ticker tests remain green.

### Phase 4: Frontend Parser And Workspace Rendering

Purpose: use pack-backed evidence in the active validation UI.

Tasks:

- Add `normalizeValidationEvidencePack`.
- Update `buildValidationEvidenceSummary` to prefer the pack.
- Keep legacy fallback for result payloads without the pack.
- Update `validationService.ts` to pass the raw pack through normalization.
- Update `ValidationWorkspacePanel.vue` only where needed to show pack-backed status, route, and module counts.
- Add tests in:
  - `web/src/services/validationService.test.ts`;
  - `web/src/components/ValidationWorkspacePanel.test.ts`;
  - optional new `web/src/utils/validationEvidencePack.test.ts`.

Acceptance criteria:

- Existing overfit panel still renders for old result shapes.
- New pack-backed result renders available/skipped/not-comparable states correctly.
- Candidate-set metrics render as candidate-set evidence instead of candidate row approval.
- UI does not label unavailable evidence as failed.
- No broad selector churn in frontend tests.

### Phase 5: Artifact Persistence And History Detail

Purpose: make the pack durable and reviewable after save/reopen.

Tasks:

- Confirm saved run artifacts include the pack as part of the validation result payload.
- Extend `run_artifacts.rs` stage-completion summary extraction to prefer pack fields.
- Preserve fallback behavior for old artifacts.
- Update artifact detail/hydration responses only additively.
- Add tests in `src-tauri/src/run_artifacts.rs` for:
  - saved new artifact includes pack;
  - history summary uses pack;
  - legacy artifact summary falls back to `run_verdict`;
  - missing pack does not fail detail load;
  - report state is not recomputed via directory scan.

Acceptance criteria:

- New artifacts retain evidence pack.
- Old artifacts remain loadable.
- History list/detail shows a compact evidence status.
- No new artifact root scans are introduced.

### Phase 6: Report Integration

Purpose: make generated XLSX reports explain the same evidence contract.

Tasks:

- Parse `validation_evidence_pack` in `commands_reports_validation_common.rs`.
- Add summary model fields for pack status, route, module availability, key metrics, and replay class.
- Update `commands_reports_validation_xlsx.rs` Summary sheet.
- Decide whether to add a dedicated Evidence sheet in this phase.
- If report structure changes, update output parity pairs and baselines.
- Add or update tests in:
  - `src-tauri/tests/validation_xlsx_report_tests.rs`;
  - `src-tauri/src/commands_reports_validation_common.rs`;
  - `src-tauri/src/commands_reports_validation_xlsx.rs`.

Acceptance criteria:

- Reports generated from new artifacts show pack-backed evidence.
- Reports generated from old artifacts still work.
- Output parity is updated if the workbook shape changes.

### Phase 7: Verification Bundle And Documentation

Purpose: finish with evidence that the feature is stable.

Tasks:

- Add a short entry to `docs/REPO_MAP.md` only after implementation begins or lands.
- Update `docs/artifact_replay_report_evidence_contract.md` if the report/artifact contract changes.
- Add release note draft if this becomes public-facing.
- Run targeted tests.
- Run output parity if report shape changes.
- Run repo quality gate for broad cross-layer changes.

Recommended verification:

```powershell
cargo test -p ta_app_core validation_evidence_pack
cargo test -p ta_app_core double_ma_validation
cargo test -p ta_app_core composite_validation
npm --prefix web run test -- src/services/validationService.test.ts src/components/ValidationWorkspacePanel.test.ts
node scripts/run_output_parity_check.mjs
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1
```

For narrower implementation slices, run targeted commands first and save broad gates for handoff.

## Compatibility Strategy

### Serialization

All new fields are additive:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub validation_evidence_pack: Option<ValidationEvidencePackV1>
```

Old artifacts:

- load normally;
- produce `legacy_absent` or fallback summaries;
- do not require migration.

New artifacts:

- include raw result fields and the pack;
- keep reports compatible with older fallback parsing where possible.

### Artifact Repair

Do not use `repair_run_artifact` to eagerly add packs to old artifacts in the MVP.

If a later repair/backfill is desired, it should:

- be opt-in;
- be dry-run capable;
- derive only from already saved fields;
- not rerun validation;
- not alter candidate rows.

### Output Parity

If report XLSX shape changes:

- update output parity pairs intentionally;
- include a before/after note;
- do not bury report shape changes inside a validation logic PR.

If only JSON artifacts gain an additive field:

- decide whether existing structural parity should ignore or accept the new field;
- update semantic parity rules only if the field becomes a release claim.

## Risk Analysis

| Risk | Impact | Mitigation |
|---|---|---|
| Pack disagrees with raw fields | Trust regression | Build pack from final raw result fields; add consistency tests |
| Cross-ticker filtering leaves stale pack | Misleading candidate counts/verdict | Rebuild pack after command-layer result transformations |
| UI treats skipped evidence as failed | False negatives | Use explicit availability enum and tests for skipped/not requested |
| Compact UI looks like approval | False certainty | Neutral labels; distinguish `passed` from `approved` |
| Report shape changes without parity | Release/report regression | Require output parity update for report changes |
| Old artifacts fail to load | History regression | Additive serde only; legacy fallback parser |
| History refresh becomes expensive | Performance regression | No recomputation or directory scans |
| Backend adds UI copy | Localization/consistency issue | Store enums/values/caveats; frontend derives copy |
| Dataset path leakage | Privacy issue | Use existing redaction/report metadata, no new raw path display |
| Feature becomes hidden ranking input | Behavioral drift | Non-goal; tests should prove candidate order unchanged |

## Acceptance Criteria

The feature is complete when:

- New Double-MA validation results include `validation_evidence_pack.version == 1`.
- New Double-MA validation results include `validation_evidence_pack.schema_version == "1-0-0"` for the MVP model.
- New composite validation results include `validation_evidence_pack.version == 1`.
- PBO/CSCV metric entries are scoped as candidate-set or selection-process evidence.
- DSR/PSR entries include sample-unit and comparability caveats when available, and use `not_comparable` when prerequisites are missing.
- SPA entries do not render as a bare pass/fail value; they include availability and p-value/comparison-set context where available.
- Pack route fields agree with `backend_used`, `backend_route`, and `backend_fallback_reason`.
- Pack verdict fields agree with `run_verdict`.
- Requested evidence modules have explicit availability states.
- Old artifacts without the pack still load and show legacy fallback evidence.
- Saved run artifacts preserve the pack.
- History/detail surfaces expose compact evidence status without recomputation.
- Validation workspace renders pack-backed evidence and legacy fallback evidence.
- Validation XLSX reports include the pack summary or explicitly preserve old report shape with a documented follow-up.
- Targeted Rust and frontend tests pass.
- Output parity is updated if report shape changes.
- No scoring, ranking, or route policy changed.

## Suggested PR Breakdown

### PR 1: Backend Contract Only

Files:

- `crates/app_core/src/validation_shared.rs`;
- `crates/app_core/src/validation_pipeline.rs`;
- `crates/app_core/src/lib.rs`;
- targeted app-core tests.

Purpose:

- define DTOs;
- implement builder;
- test builder behavior.

No frontend or report changes.

### PR 2: Emit Pack From Validation Results

Files:

- `crates/app_core/src/double_ma_validate.rs`;
- `crates/app_core/src/composite_validate.rs`;
- `src-tauri/src/commands_validation.rs`;
- targeted Rust tests.

Purpose:

- attach pack;
- rebuild after command transformations.

### PR 3: Frontend Consumption

Files:

- `web/src/utils/validationEvidenceSummary.ts`;
- optional `web/src/utils/validationEvidencePack.ts`;
- `web/src/services/validationService.ts`;
- `web/src/components/ValidationWorkspacePanel.vue`;
- frontend tests.

Purpose:

- render pack-backed evidence;
- preserve legacy fallback.

### PR 4: Artifacts And Reports

Files:

- `src-tauri/src/run_artifacts.rs`;
- `src-tauri/src/commands_reports_validation_common.rs`;
- `src-tauri/src/commands_reports_validation_xlsx.rs`;
- `src-tauri/tests/validation_xlsx_report_tests.rs`;
- `scripts/output_parity_pairs.json` and parity fixtures if needed.

Purpose:

- saved review;
- history summary;
- report visibility;
- parity.

### PR 5: Documentation And Public Copy

Files:

- `docs/artifact_replay_report_evidence_contract.md`;
- `docs/REPO_MAP.md`;
- `docs/public/07_validation_and_evidence.md` if public docs are in scope.

Purpose:

- document the contract and user-facing interpretation.

## Testing Matrix

| Layer | Test Type | Cases |
|---|---|---|
| App-core | Unit | no modules, CSCV available, CSCV skipped, SPA skipped, fallback route |
| App-core | Serde | new pack roundtrip, missing pack defaults |
| Tauri validation | Unit/integration | cross-ticker rebuild, composite route diagnostics, Double-MA route truth |
| Artifact | Unit | saved new artifact, legacy fallback, stage completion summary |
| Report model | Unit | parse pack, parse legacy, report-state labels |
| XLSX report | Integration | summary rows, old artifact compatibility |
| Frontend utility | Unit | normalize complete/partial/legacy pack |
| Frontend service | Unit | summary line, overfit items, availability states |
| Frontend component | Unit | renders compact evidence panel and badges |
| Output parity | Script | JSON/XLSX report shape if changed |

## Verification Commands

Backend contract:

```powershell
cargo test -p ta_app_core validation_evidence_pack
```

Validation paths:

```powershell
cargo test -p ta_app_core double_ma_validation
cargo test -p ta_app_core composite_validation
```

Tauri artifact/report paths:

```powershell
cargo test -p ta_desktop_demo_app run_artifacts
cargo test -p ta_desktop_demo_app validation_xlsx_report_tests
```

Frontend:

```powershell
npm --prefix web run test -- src/services/validationService.test.ts src/components/ValidationWorkspacePanel.test.ts
npm --prefix web run typecheck
```

Output parity when report shape changes:

```powershell
node scripts/run_output_parity_check.mjs
```

Broad handoff gate:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1
```

Add `-IncludeWorkspaceTests` for broad backend/result-schema work.

## Rollout Plan

1. Land backend DTO and builder behind no feature flag.
2. Emit pack on new validation results.
3. Frontend consumes pack but keeps legacy fallback.
4. Artifacts persist pack naturally through existing save flow.
5. History/detail summarizes pack when present.
6. Reports include pack summary after parity plan is ready.
7. Public docs mention the pack only after report/history UX is stable.

No runtime feature flag is required because the field is additive and not behavior-changing. If report shape changes are risky, gate only the new report sheet behind an internal report schema/version decision, not the evidence pack itself.

## Open Decisions

1. Exact serialized field name:

   Recommended: `validation_evidence_pack`.

2. Whether to add a dedicated XLSX `Evidence` sheet in the first report PR:

   Recommended: start with Summary sheet additions unless the output parity update is already planned.

3. Whether `workflow` should be `general_validation` or strategy-specific for Double-MA and composite:

   Recommended: use strategy-specific values when known, with `general_validation` as fallback.

4. Whether ML Signal validation joins the MVP:

   Recommended: no. Keep it as a follow-up after general validation pack is stable.

5. Whether dataset evidence includes market calendar id in MVP:

   Recommended: include optional fields but do not block on a versioned calendar service.

6. Whether artifact repair should backfill old packs:

   Recommended: no for MVP.

7. Whether the MVP should generate a formal JSON Schema file:

   Recommended: no. Keep the Rust serde DTO as authority initially, but keep `schema_version` in the payload so a formal schema registry can be added later without guessing historical meaning.

## Implementation Checklist

- [ ] Add `ValidationEvidencePackV1` DTOs and enums.
- [ ] Include `schema_version` and document the MVP value as `1-0-0`.
- [ ] Add deterministic app-core builder.
- [ ] Add builder unit tests.
- [ ] Add metric-scope tests for candidate, candidate-set, selection-process, and comparison-set evidence.
- [ ] Add comparability-state tests for missing DSR/PBO/SPA prerequisites.
- [ ] Export types from `ta_app_core`.
- [ ] Add optional pack field to Double-MA validation result.
- [ ] Add optional pack field to composite validation result.
- [ ] Rebuild pack after Tauri command-layer result transformations.
- [ ] Add Rust tests for route/verdict/pack consistency.
- [ ] Add frontend pack normalizer.
- [ ] Update validation summary adapter to prefer pack.
- [ ] Update validation workspace tests.
- [ ] Update artifact summary extraction.
- [ ] Add artifact legacy fallback tests.
- [ ] Update report model.
- [ ] Update XLSX report rendering or defer with documented follow-up.
- [ ] Run output parity if report shape changes.
- [ ] Update artifact evidence contract docs if persisted/report semantics change.
- [ ] Update repo map after implementation lands.

## Final Recommended First Slice

The lowest-risk first implementation slice is:

1. Add the app-core DTO and builder.
2. Attach it to Double-MA validation only.
3. Add tests proving pack/raw-field consistency.
4. Do not touch reports yet.

This creates a durable backend contract without immediately changing the UI or report shape. The second slice can add composite validation and frontend consumption. Reports should come after the pack has at least one stable saved-artifact fixture.
