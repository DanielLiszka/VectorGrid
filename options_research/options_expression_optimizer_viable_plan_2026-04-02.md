# Options Expression Optimizer — Repo-Fit Viable Implementation Plan

Date: 2026-04-02
Status: revised implementation plan aligned to the current VectorGrid product shape, artifact system, and benchmark priorities.

## Executive Verdict

The original plan is directionally strong.

It already gets the most important product framing right:

- this is **not** a general-purpose options simulator
- this is **not** a LEAN replacement
- this **is** an artifact-driven options expression optimizer over already-validated directional edges

That core framing should stay.

What needs to change is the **integration strategy**.

To fit the current VectorGrid repo and to keep the feature actually shippable, the first release should be narrowed into this shape:

1. **A separate artifact-driven follow-up workflow, not a new core strategy family.**
2. **A compiler-heavy CPU front half plus a narrow GPU scoring back half, with GPU productization gated by an early feasibility spike.**
3. **A derived signal-event manifest, not a broad mutation of every existing run artifact writer.**
4. **A single-underlying-root, long-single-leg, moneyness-first v1.**
5. **A separate options data path, not an extension of the current Polygon/live-execution stack.**
6. **A result artifact + diagnostics UI first, with heavyweight report/export surfaces deferred.**

That makes the feature coherent with the current repo instead of turning it into a second platform rewrite.

---

## 1. Product Definition

### What the feature is

The feature is:

**artifact-driven options expression optimization over validated underlying strategy artifacts**

The core question is still:

**“Given a directional edge that VectorGrid already validated, what is the best simple option expression for it?”**

### What the feature is not

The feature is not:

- a full broker-style options simulator
- a generic options scripting engine
- options-native signal discovery
- live options execution
- a replacement for the current Double-MA benchmark route

### Repo-fit product framing

The best product framing for VectorGrid is:

**“Express a validated directional artifact through options.”**

Not:

- “trade options inside the normal optimizer as just another strategy kind”
- “replace LEAN”
- “turn VectorGrid into a general options platform”

---

## 2. The Most Important Repo-Fit Decisions

These decisions should be frozen before implementation.

### Decision A — make this a separate workflow, not a new main optimize mode

The current repo already has special flows that are separate from the normal top-k optimization route.

That is the right pattern to copy.

So v1 should be introduced as:

**Options Expression** — a follow-up workflow launched from a validated artifact.

This should behave more like a specialized research/product flow than like a new member of the Double-MA strategy list.

### Decision B — do not mutate the current validation artifact contract first

Do **not** start by widening every existing run artifact schema.

Instead:

- keep the parent validated artifact authoritative
- project a deterministic **SignalEventManifestV1** from it
- persist that manifest as a **derived sibling artifact/cache**
- let the options workflow consume the manifest

This is far less invasive and much more likely to fit the current run-artifact system.

### Decision C — do not tie v1 to the current Double-MA hot kernel path

The repo’s public benchmark and current open performance work are still centered on the exact-grid `DoubleMa` route.

So the options feature should not begin by reshaping `double_ma` runtime internals.

Instead:

- keep options kernels separate
- keep options runtime orchestration separate
- reuse generic optimizer/search/report/artifact infrastructure
- avoid contaminating the benchmark-critical `double_ma` path unless and until the feature proves itself

### Decision D — v1 needs a CPU oracle and a GPU product path from day one

The repo explicitly keeps CPU-only builds working.

So options expression v1 must have:

- a deterministic CPU reference evaluator
- a CPU-capable workflow for correctness, development, and non-CUDA builds
- a GPU scoring path that is treated as a product requirement for a real external release, not as an optional luxury

The right mental model is:

- CPU is the truth path
- GPU is the differentiation path

If the workflow cannot justify a real GPU scoring advantage on compiled tensors, it should remain an internal or narrow research preview rather than a promoted product mode

### Decision E — v1 jobs should be single-underlying-root

A major complexity driver in options research is cross-symbol chain heterogeneity.

So v1 should require that an options-expression job resolves to exactly **one options root**.

If a parent artifact contains signals across multiple underlyings:

- split it into per-root manifests
- run one options-expression job per root
- do cross-root aggregation only in a later post-pass, if needed

This one decision removes a large amount of avoidable scope.

---

## 3. Scope To Freeze For V1

### In scope

- validated parent artifact input only
- long single-leg calls and puts only
- one options root per job
- minute quote-based entry/exit evaluation
- deterministic contract selection
- template-based exits
- CPU reference scoring
- GPU-targeted dense tuple scoring over precomputed summary tensors, with productization contingent on proving the economics
- conservative and mid fill models
- holdout-aware ranking using the parent artifact’s folds when available
- final options-expression artifact output linked to the parent artifact

### Out of scope for v1

- multi-leg spreads
- rolling
- short premium
- assignment or exercise
- margin modeling
- portfolio construction in the main hot loop
- options-native signal discovery
- arbitrary scripting
- live options execution
- broker integration for options
- dynamic chain search inside kernels
- IV-surface logic
- delta-driven management rules
- a new heavyweight XLSX export family

### Strongly recommended v1 defaults

- **selector family:** moneyness only
- **side:** implied from signal direction
- **data resolution:** 1-minute quotes
- **default DTE set:** `1DTE`, `3DTE`, `5DTE`
- **expiry policy:** forced liquidation before final tradable cutoff
- **optimization mode:** isolated-trade normalized metrics
- **filters:** static per-job filter bundle, not fully optimized axes
- **rescore:** conservative fill stress on top candidates only
- **0DTE:** optional narrow profile only after quote coverage and invalid-rate diagnostics prove it is stable enough

### Post-v1 expansion envelope

Do not close the door on broader option-expression families, but constrain expansion to shapes that still preserve the compiler-first, dense-scoring model.

Allowed future expansion should mean:

- more precompiled **expression geometries**
- not a generic multi-leg simulator
- not arbitrary scripting
- not raw chain search inside kernels

The key rule is:

**expansion is only allowed if preprocessing can still compile the strategy family into dense candidate slots and dense exit-summary tensors that keep the GPU scorer regular.**

Recommended future expansion order if v1 proves performant enough:

1. same-expiry fixed debit call spreads
2. same-expiry fixed debit put spreads
3. fixed-ratio two-leg call or put ladders
4. only later, other fixed-geometry bundles if they preserve tensor density and deterministic compilation

Hard constraints for any post-v1 multi-leg expansion:

- one structure family per job
- deterministic strike-offset geometry from a base selector
- same entry timestamp for all legs in the bundle
- same expiration for all legs in first-wave multi-leg support
- one fill-model family per job
- no dynamic rolling
- no assignment/exercise semantics
- no arbitrary per-leg branching in the hot scorer

Any multi-leg expansion must first pass a dedicated feasibility spike showing:

- candidate density remains high enough
- invalid-slot rates stay acceptable
- structural and summary caches still amortize
- compiled tensors remain regular enough that GPU scoring clearly beats CPU on the supported workloads

If those conditions fail, keep the expansion out of the product path and preserve the single-leg scope.

### GPU-compatible LEAN parity envelope

The right parity target is not “LEAN, but faster.”

The right parity target is:

**the subset of LEAN-style research capability that can still be compiled into deterministic dense tensors and scored efficiently on the GPU.**

That means future parity work should concentrate on five families.

#### 1. Universe and filter parity

Good additions:

- weekly-only vs standard-expiry families
- expiration-class and DTE policy bundles
- strike-distance and moneyness families
- session-window and weekday-entry policies
- daily precomputed `IV`, `delta`, `gamma`, `theta`, `vega`, and `OI` filter bundles

Constraint:

- these remain **job-static compile policies**, not a broad runtime Cartesian explosion

#### 2. Expression-family parity

Good additions:

- same-expiry debit call spreads
- same-expiry debit put spreads
- later, fixed-ratio same-expiry two-leg ladders

Constraint:

- each structure family must still compile into deterministic leg bundles from one base selector

#### 3. Execution-model parity

Good additions:

- conservative quote
- research mid
- bounded spread-fraction models such as `mid + x% of spread`
- explicit `no fill`, `stale quote`, and `crossed market` invalid outcomes

Constraint:

- fill-model family remains job-static and summary-compiler-friendly

#### 4. Overlay and risk parity

Good additions:

- capital-budget overlay
- max concurrent positions
- one-open-position-per-root or per-side
- cooldown after exit
- max daily entries
- side exposure caps

Constraint:

- these remain CPU post-pass policies over top finalists, not hot-loop search axes

#### 5. Validation parity

Good additions:

- walk-forward or fold-aware holdout
- side-level stability
- session-level stability
- harsher spread, fee, and liquidity stress rescoring

Constraint:

- robustness remains a rescore and ranking discipline, not a justification for widening the hot scorer

### Explicit non-goals even if LEAN supports them

Do not treat these as parity targets:

- assignment or exercise modeling
- rolling or repair logic
- buying-power or portfolio-margin simulation
- mixed-expiry structures such as calendars or diagonals
- arbitrary combo scripting
- arbitrary event-driven order semantics
- custom option-pricing models in the hot path
- live options execution
- dynamic chain search in the scorer

---

## 4. Workflow Placement Inside VectorGrid

### Recommended entry point

Do not add this as a brand-new top-level workspace first.

The smallest repo-fit entry is:

- choose a validated artifact in **History** or a validation result surface
- trigger **Express with Options**
- rehydrate the selected artifact into a dedicated options-expression store
- open a narrow workflow panel for diagnostics, run config, and results

For v1, freeze the placement more concretely:

- launch from **History** or validation results
- mount inside the existing shell as a **side panel / mounted results panel**
- do **not** add a new workflow tab
- do **not** add a new workspace tab

### Why this fits the repo better

This matches the current system’s strengths:

- artifacts are already a cross-workflow handoff mechanism
- history and artifact rehydration already exist
- validation artifacts already capture the “edge found here” concept the options layer needs

### Recommended frontend shape for v1

Use the existing shell and history/artifact flow rather than inventing a new high-churn navigation mode.

Suggested v1 frontend additions:

- new service: `web/src/services/optionsExpressionService.ts`
- new store: `web/src/stores/optionsExpressionStore.ts`
- new panel: `web/src/components/OptionsExpressionPanel.vue`
  - specifically as a History/Analyze-launched mounted panel or drawer, not a new workflow surface
- new quick action from history/validation surfaces:
  - **Express with Options**

### Files to reuse first

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/historyService.ts`
- `web/src/services/validationService.ts`
- `web/src/utils/core/workflowModel.ts`
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`

### What not to do in v1 UI

Do not start with:

- a whole new workspace family
- a new dense export/report matrix
- a fully separate design language from the current results shell

The goal is to feel like a natural artifact follow-up, not a second app inside the app.

---

## 5. Backend Surface And File Boundaries

### Recommended Tauri command boundary

Add a dedicated command module rather than overloading normal optimize commands.

Suggested module:

- `src-tauri/src/commands_options_expression.rs`

Suggested command surface:

- `list_options_expression_inputs`
- `preview_options_expression_diagnostics`
- `run_options_expression`
- `inspect_options_expression_trade`
- `list_options_datasets`
- `import_options_dataset`
- `delete_options_expression_artifact`

Also wire it through:

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`

### Recommended app-core module boundaries

Add a dedicated options-expression stack instead of extending `double_ma.rs` first.

Suggested new files:

- `crates/app_core/src/options_expression.rs`
  - top-level orchestration
- `crates/app_core/src/options_signal_manifest.rs`
  - parent-artifact -> event-manifest projection
- `crates/app_core/src/options_data_store.rs`
  - normalized options quote access and coverage diagnostics
- `crates/app_core/src/options_candidate_compiler.rs`
  - fixed-slot candidate compilation
- `crates/app_core/src/options_exit_summary.rs`
  - hold/target/stop summary compiler
- `crates/app_core/src/options_cpu_ref.rs`
  - deterministic reference scorer
- `crates/app_core/src/options_gpu_eval.rs`
  - GPU tuple scorer only
- `crates/app_core/src/options_result.rs`
  - decode/ranking/result-shaping helpers
- `crates/app_core/src/options_types.rs`
  - shared contracts and enums

Optional later:

- `crates/app_core/src/kernels/options_expression.cu`

### Existing infrastructure to reuse

- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/commands_reports_api.rs` for lightweight open/reveal/save wrappers
- existing headless benches/binaries patterns under `crates/app_core/benches` and `src-tauri/src/bin`

Important caveat:

- reuse the **ranking / top-k / aggregation patterns**
- do **not** assume the current `ta_optimizer` param/result types can be reused directly
- the current crate is still centered on `DoubleMaParams` and `Metrics`, so options tuples and options result rows should get their own local contracts first

### What to leave untouched initially

Avoid first-wave changes to:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `src-tauri/src/commands_execution.rs`
- the Polygon live execution path

---

## 6. Data And Artifact Contracts

The core repo-fit change is to separate **parent artifact**, **derived manifest**, **compiled caches**, and **final options result artifact**.

### 6.1 Parent artifact

This remains the authoritative validated artifact already produced by the current pipeline.

It is not rewritten.

### 6.2 Derived SignalEventManifestV1

This should be generated deterministically from:

- parent artifact
- loaded dataset identity
- current strategy/result metadata

It should contain both **timestamp-based** and **index-based** coordinates.

#### Minimum fields

- `manifest_id`
- `parent_artifact_id`
- `parent_run_kind`
- `dataset_id`
- `dataset_hash`
- `timeframe`
- `symbol_root`
- `side`
- `decision_timestamp`
- `earliest_tradable_timestamp`
- `decision_bar_index`
- `earliest_tradable_bar_index`
- `raw_underlier_price`
- `adjusted_underlier_price` if relevant
- `base_entry_timestamp` optional
- `base_exit_timestamp` optional
- `base_exit_bar_index` optional
- `fold_id` or split label
- `session_id`
- optional tags: regime, confidence, parent-score bucket
- `calendar_id`
- `manifest_version`

#### Why bar indices matter

The original plan correctly emphasizes timestamps.

For repo viability, add **bar indices** too.

That gives:

- deterministic joins back to the current dataset model
- fewer DST/session-boundary bugs
- cheaper indexing during candidate compilation

### 6.3 OptionsDatasetManifestV1

The normalized options data source needs its own manifest.

Minimum fields:

- `options_dataset_id`
- `root_symbol`
- `vendor/source`
- `quote_resolution`
- `calendar_id`
- `timezone`
- `coverage_start`
- `coverage_end`
- `normalization_version`
- `quote_semantics_version`
- `corporate_action_mapping_version`

### 6.4 StructuralCandidateCacheV1

Heavy compiled candidate data should **not** live inside normal run artifacts.

Store it as a data/cache object keyed by:

- `manifest_hash`
- `options_dataset_hash`
- `selector_family`
- `delay_buckets`
- `dte_buckets`
- `static_filter_bundle_hash`
- `calendar_version`
- `candidate_compiler_version`

### 6.5 ExitSummaryCacheV1

Store separately, keyed by:

- `structural_candidate_cache_hash`
- `fill_model_family`
- `hold_bucket_set`
- `target_bucket_set`
- `stop_bucket_set`
- `exit_template_family`
- `summary_compiler_version`

### 6.6 Final OptionsExpressionArtifactV1

Persist through the normal run-artifact channel as a new options-expression result kind.

Suggested fields:

- `artifact_kind = vectorgrid_options_expression_v1`
- `parent_artifact_id`
- `signal_manifest_hash`
- `options_dataset_hash`
- `backend_used`
- `search_config`
- `static_filter_bundle`
- `summary_cache_hash`
- `ranked_results`
- `holdout_metrics`
- `stress_metrics`
- `coverage_diagnostics`
- `environment_metadata`
- `version_hashes`

### 6.7 Artifact and workflow integration requirements

The new artifact kind requires explicit repo-surface integration work.

Do not assume it will automatically flow through existing history/workflow sync.

Required consequences:

- backend artifact classification must recognize `vectorgrid_options_expression_v1`
- artifact listing and load summaries must expose enough kind-specific metadata to route options-expression results correctly
- frontend workflow artifact sync must gain an options-aware payload path rather than forcing options results through `currentRun` / `currentValidation`
- history rehydration must reopen through the options-expression store/service path rather than the standard optimization rerun path

For the first spike, freeze one parent artifact family if needed.

Recommended default:

- start with the current validated `DoubleMa` family or the most deterministic validated artifact family already in broad use
- widen to other parent families only after manifest projection is proven deterministic and cheap

### Storage recommendation

Use this split:

- raw/normalized options data:
  - under market-data-root, e.g. `effective_market_data_root/options_cache/...`
- compiled candidate/summary caches:
  - under app-local data cache, e.g. `app_local_data_dir/data_cache/options_expression/...`
- final result artifacts:
  - under `app_local_data_dir/runs`

This keeps large reusable data out of run artifacts while preserving the current artifact model.

---

## 7. Options Data Layer

The original plan is correct that the data layer will make or break the feature.

To make it viable in the current repo, the data layer must be **separate** from the current Polygon/live path.

### Recommended v1 ingestion policy

Support import of already-downloaded historical options quotes first.

Suggested first-wave inputs:

- normalized Parquet
- normalized CSV with a strict schema

Do not start with:

- vendor download integration
- live options data streaming
- broker-linked options discovery

### Minimum normalized quote schema

For each quote row or quote-bar row:

- `root_symbol`
- `contract_id`
- `option_type`
- `expiration_date`
- `strike`
- `multiplier`
- `quote_timestamp`
- `bid`
- `ask`
- `mid` or derivable mid
- `last` optional
- `volume` optional
- `open_interest` optional
- `delta` optional
- `iv` optional
- `quote_age_ms` optional if derivable
- `session_id`

### Recommended normalization responsibilities

- contract-id normalization
- symbol-root mapping
- corporate action alignment
- timezone and session normalization
- stale quote filtering
- crossed/locked cleanup
- zero/negative quote cleanup
- absurd spread cleanup
- contract lifecycle tracking
- expiry cutoff alignment

### Recommended partitioning

Partition for the real access pattern:

**given one event time and one root symbol, find eligible contracts and future quote path quickly**

Good partitions:

- `root_symbol / trade_date / expiration_date`
- optional strike shard within expiration

### Universe restriction for v1

Ship with a limited liquid universe only.

Recommended initial rule:

- one root symbol per job
- liquid underlyings only
- roots already represented by validated artifacts
- start with a tiny allowlist if necessary

### Important repo-fit rule

Do not bolt this onto:

- current Polygon historical support
- current Polygon live stream
- current live execution path

Those systems are currently centered on underlying market data, not options chain research.

---

## 8. Search Space Design — Revised For Viability

The original plan is too generous about what should be optimized inside one dense job.

To make the first release viable, split job configuration into:

1. **static compile config**
2. **runtime tuple grid**

### 8.1 Static compile config (chosen once per job)

These settings affect candidate compilation and should **not** all be axes in the first dense grid:

- selector family
- max spread rule
- min volume rule
- min open-interest rule
- max premium rule
- latest entry cutoff
- minimum remaining time-to-expiry
- quote validity policy
- exit template family
- root-symbol universe

If the user wants to compare multiple filter bundles, that should be:

- multiple jobs
- or a tiny explicit compare set

not a giant dense tensor inside one job.

### 8.2 Runtime tuple grid (optimized inside the job)

These are the right first-wave dense axes:

- entry delay bucket
- DTE bucket
- moneyness bucket
- hold bucket
- target bucket
- stop bucket

### 8.3 Recommended v1 selector policy

Use **moneyness only** first.

Add delta later only if the data quality proves trustworthy.

Premium selection can wait until after moneyness is stable.

### 8.4 Recommended tractable first grid

#### Contract expression

- DTE buckets:
  - `1DTE`
  - `3DTE`
  - `5DTE`
- moneyness buckets:
  - `ATM`
  - `1 step OTM`
  - `2 steps OTM`
  - `1 step ITM`

#### Entry timing

- delay buckets:
  - `0 min`
  - `1 min`
  - `5 min`

#### Exit logic

- template families supported in first-wave v1, but **choose exactly one per job**:
  - `fixed_hold`
  - `fixed_hold_or_target_stop`
  - `end_of_day_or_target_stop`
  - `base_exit_or_target_stop`
- holds:
  - `5 min`
  - `15 min`
  - `30 min`
  - `60 min`
- targets:
  - `25%`
  - `50%`
  - `100%`
- stops:
  - `25%`
  - `50%`

This is already large enough to answer useful questions without turning the feature into a search-space explosion.

---

## 9. Candidate Compiler

The original candidate-compiler concept is the right backbone.

Keep it.

### Core rule

The hot scorer must never scan raw chains dynamically.

The compiler should transform irregular chains into fixed-width dense slots:

**event × delay × DTE × selector -> candidate slot**

### Required deterministic behavior

For each event:

1. resolve earliest tradable bar/time
2. apply entry delay
3. locate the first valid quote at or after entry time
4. resolve the target expiration bucket
5. resolve the nearest strike/moneyness bucket
6. apply the static filter bundle
7. emit either a valid fixed slot or an invalid slot with a reason code

### Invalid reason codes

Keep the original proposal here, but make them first-class product data:

- no quote after entry
- no expiration in bucket
- no strike in tolerance
- spread too wide
- premium too high
- insufficient volume
- insufficient open interest
- entry past cutoff
- insufficient future path
- stale/missing quote path

These should be visible in:

- diagnostics preview
- result summaries
- trade inspector
- stored artifact diagnostics

### Important v1 simplification

Keep option side implicit from the parent signal:

- bullish -> long call
- bearish -> long put

Do not add side as a separate optimized axis in v1.

---

## 10. Exit Summary Compiler

This is the second stage that makes the whole feature feasible.

The objective is the same as in the original plan:

**compile once, evaluate many**

### What to precompute

For each valid event/candidate slot:

- entry fill value per fill model family
- P/L at each hold bucket
- P/L at end of day
- P/L at forced expiry cutoff
- P/L at base-artifact exit if available
- earliest target hit index for each target bucket
- earliest stop hit index for each stop bucket
- MFE / MAE summaries
- path completeness flags

### Why this matters

The scorer can then operate on standardized summary tensors rather than quote paths.

This is the key step that makes a GPU scorer realistic without building a full options simulator.

### Important v1 rule

The fill-model family used by the summary compiler should be coarse and explicit.

Suggested first-wave families:

- `conservative_quote`
- `research_mid`

Do not create a large menu of nuanced fill models yet.

---

## 11. Execution Semantics

The original plan is broadly right here.

Freeze these rules before code is written.

### Fill policy

#### Conservative model

For long options:

- entry at `ask`
- exit at `bid`
- target/stop checks against the exitable side
- fees included

#### Research model

For long options:

- entry at `mid`
- exit at `mid`
- target/stop checks at `mid`

### Ambiguity policy

If target and stop are both reachable within one minute bar and ordering is unknowable:

- take the worse outcome in v1

### Expiry policy

- no exercise
- no assignment
- forced liquidation before final tradable cutoff

### Hold accounting

Use trading-minute-aware holding logic, not naive wall-clock time.

### Contract multiplier

Read from contract metadata.

Do not assume 100 everywhere.

---

## 12. CPU Oracle First, GPU Product Requirement

This is the biggest practical change from the original draft.

### 12.1 CPU reference evaluator is mandatory and first

Before any GPU scorer exists, build a deterministic CPU scorer that consumes:

- SignalEventManifestV1
- StructuralCandidateCacheV1
- ExitSummaryCacheV1
- tuple grid

This is the correctness oracle.

### 12.2 GPU scorer is mandatory for productization after feasibility proof

Do not invest in a production-quality GPU scorer until you can show:

- the candidate/summary representation is stable
- repeated jobs actually reuse caches
- the CPU scorer is correct and deterministic
- the compiled tensors are dense enough that GPU scoring should materially matter

But once those conditions are met, the external product path should assume a real GPU scorer.

If that threshold is never met, the work may still be useful as an internal or narrow CPU-only research preview, but it should not be promoted as a major new VectorGrid mode.

### 12.3 Backend policy for v1

Use a simpler backend story than the broader app if needed:

- `CpuOnly`
- `GpuKernel`
- `Auto`

Where:

- `CpuOnly` always works and remains the correctness/oracle path
- `GpuKernel` accelerates tuple scoring only and is the intended product path for meaningfully large jobs
- `Auto` prefers CPU below a tensor-size threshold and GPU above it

Do not interpret `Auto` as permission to make the whole feature effectively CPU-only under realistic research workloads.

### 12.4 GPU isolation rule

Put the first GPU scorer in a separate module and kernel file.

Do not begin by editing the current benchmark-critical kernel path.

### 12.5 What the GPU should do

The GPU scorer should only:

- iterate tuples
- read candidate/summary tensors
- apply exit precedence logic
- accumulate ranking metrics
- emit compact result rows

It should **not**:

- scan raw quotes
- scan raw chains
- resolve expirations dynamically
- run arbitrary user branching
- own complicated fill semantics beyond the precompiled summary family

### 12.6 Shipping rule

Treat CPU-only as:

- mandatory for correctness
- mandatory for development and non-CUDA builds
- acceptable for internal previewing

Treat GPU scoring as:

- mandatory for a serious external product launch of this mode
- the feature's main differentiation from slower event-driven engines

---

## 13. Ranking And Validation

The original plan is strong here and should mostly be kept.

### Primary ranking philosophy

1. hard sample/coverage constraints
2. then composite ranking

### Minimum guards

Require at least:

- minimum valid trades overall
- minimum trades per fold
- minimum trades per side
- maximum invalid-slot rate

### Holdout discipline

Do not call options results “validated” just because the parent artifact was validated.

Instead:

- reuse parent fold structure when available
- search on allowed folds only
- score finalists on untouched folds
- store in-sample and holdout separately

### Stress rescore

Top-K only:

- conservative fill
- widened spread
- higher fees
- stricter liquidity filter

### Recommended first-wave ranking outputs

- EV per trade
- EV per premium dollar
- win rate
- median premium return
- valid trade count
- invalid rate
- holdout EV per premium dollar
- side-specific summaries
- day/session concentration summaries

### Recommended simplification for v1

Because jobs are single-root in v1, de-emphasize multi-symbol weighting modes initially.

Focus instead on:

- trade-weighted
- day-equal-weighted
- fold-aware robustness

---

## 14. Portfolio Overlay

Keep portfolio replay as a **post-pass only**.

Do not mix it into the main dense search loop.

### Why

- preserves regularity
- avoids path-dependent ranking noise
- keeps the first scorer compact and deterministic

### V1 policy

Portfolio overlay is allowed only for:

- top-ranked finalists
- CPU replay
- explicit post-pass analysis

It is not part of the benchmark story and it is not part of the first hot loop.

---

## 15. UI / UX Scope For V1

### Must-have surfaces

- parent artifact picker / handoff
- diagnostics preview before run
- run config form
- ranked result table
- fold/coverage diagnostics
- baseline underlying vs options comparison
- trade inspector

### Nice-to-have but not mandatory in v1

- DTE × moneyness heatmap
- hold × target/stop heatmap

If added in v1, build them from result rows or lightweight post-processing.

Do **not** add a dedicated heavy GPU heatmap path first.

### Explicitly defer in v1

- bespoke XLSX workbook family
- export-all grid surfaces
- large-format reporting templates

Artifact + UI + optional CSV is enough for the first release.

---

## 16. Reporting And Persistence

### Recommended persistence pattern

- use the current run-artifact system for final result artifacts
- use data/cache storage for compiled candidate/summary caches
- keep the parent artifact reference visible everywhere

### Recommended report/export posture for v1

Ship with:

- saved run artifact
- lightweight CSV export if useful
- on-screen diagnostics and trade inspection

Defer:

- dedicated XLSX report builders
- parity-heavy report families

This reduces surface area and lets the feature prove itself before it inherits the full reporting burden.

---

## 17. Performance Model And Cache Design

The original two-level cache idea is one of the highest-value parts of the plan.

Keep it.

### Structural candidate cache

Reusable across jobs that share:

- same signal manifest
- same options dataset
- same selector family
- same static filter bundle
- same DTE/delay schema

### Exit summary cache

Reusable across jobs that share:

- same candidate cache
- same fill family
- same hold/target/stop bucket sets
- same template family

### Memory layout guidance

Use:

- struct-of-arrays
- contiguous numeric buffers
- compact integer indices
- validity bitmasks / reason-code arrays
- `f32` where precision is demonstrably enough

### Chunking guidance

Allow event-batch chunking.

Do not require all events in VRAM at once.

### Performance observability

Track at least:

- structural cache hit rate
- summary cache hit rate
- preprocessing time by stage
- scorer time by backend
- tuples/sec
- events/sec
- invalid reason distribution
- cache footprint

---

## 18. Correctness Plan

Keep the original correctness posture.

It is appropriate for the risk profile.

### Mandatory test layers

#### Unit tests

- manifest projection logic
- raw vs adjusted mapping
- earliest tradable timestamp/index logic
- DTE bucket resolution
- moneyness bucket resolution
- fill calculations
- ambiguity tie-breaks
- forced expiry liquidation

#### Golden tests

- small known artifact + small normalized options sample -> known ranked outputs

#### Differential tests

- CPU vs GPU on identical compiled tensors
- repeated-run determinism

#### Edge-case fixtures

- no quote after signal
- quote gap mid-trade
- late-day 0DTE entry
- early close session
- DST boundary
- split date / corporate-action mismatch
- stale quote chain
- no strike in selector tolerance
- missing expiration bucket

### Trade inspector is part of correctness

Treat the trade inspector as correctness infrastructure, not just UX.

If users cannot inspect why a slot was selected or rejected, trust will collapse quickly.

---

## 19. Revised Phased Roadmap

This roadmap is more viable than the original because it inserts explicit go/no-go checkpoints.

### Phase 0 — Repo-fit contract spike

Deliverables:

- freeze product boundary
- freeze first-spike parent artifact family
- audit current validated artifact kinds and run-artifact loading paths for deterministic manifest projection requirements
- define parent artifact -> SignalEventManifestV1 projection
- define storage split between runs and data cache
- define one-root-per-job rule
- define static compile config vs runtime tuple grid

Exit criterion:

- you can describe the entire workflow without mutating the current benchmark-critical runtime
- you know exactly which current artifact kinds and fields are sufficient for deterministic projection and which gaps would require a targeted export contract

### Phase 0.5 — Compiled-tensor feasibility spike

Deliverables:

- one-root, liquid-underlying sample dataset
- one parent artifact projected into `SignalEventManifestV1`
- moneyness-only candidate compilation for the default `1DTE`/`3DTE`/`5DTE` grid
- invalid-reason distribution and candidate-density measurements
- rough cache-size and tuple-count estimates
- first-pass CPU-versus-GPU scoring economics estimate based on compiled tensors

Exit criterion:

- compiled tensors are dense enough, reusable enough, and large enough that a GPU scoring path looks economically credible
- use explicit first-wave gates until repo-specific measurement justifies changing them:
  - valid-slot density for the default liquid-root profile is at least `30%`
  - invalid-slot rate for that profile is no worse than `70%`
  - immediate rerun on unchanged inputs projects at least `95%` structural-cache reuse and `95%` summary-cache reuse
  - projected compiled buffers fit the intended chunking/memory budget without pathological spill behavior
  - projected GPU scoring advantage on representative large jobs looks at least `3x` better than CPU scoring, or the scorer phase is large enough that a CUDA path would clearly matter

If not:

- narrow the scope further
- keep the work internal
- or stop before broad workflow implementation

### Phase 1 — Artifact handoff and diagnostics shell

Deliverables:

- history/validation action: **Express with Options**
- options-expression store/service shell
- parent artifact rehydration
- preflight diagnostics endpoint with no scorer yet

Exit criterion:

- a validated artifact can launch into an options-expression draft workflow and surface coverage diagnostics

### Phase 2 — Options data import and normalization

Deliverables:

- normalized import path
- dataset manifest
- coverage queries
- session/calendar alignment
- quote cleanup rules

Exit criterion:

- for one liquid root and date range, normalized quote access is reliable and auditable

### Phase 3 — Signal manifest + candidate compiler

Deliverables:

- deterministic SignalEventManifestV1 projection
- fixed candidate slots
- invalid reason accounting
- structural candidate cache

Exit criterion:

- one job can compile candidate slots with stable diagnostics and cache reuse on rerun

### Phase 4 — Exit summary compiler + CPU scorer

Deliverables:

- summary cache
- deterministic CPU scorer
- baseline ranking outputs
- trade inspector data model

Exit criterion:

- repeated CPU runs over the same compiled tensors are correct, explainable, and materially faster than recompiling everything each time

### Phase 4.5 — GPU productization checkpoint

Questions:

- do real workloads still match the density/caching assumptions from Phase 0.5?
- does rerun amortization remain strong enough under actual user-sized jobs?
- does the measured scorer workload still justify a product-quality CUDA path?

Required measured gates before promoting the CUDA path:

- valid-slot density on representative product-sized jobs remains at least `25%`
- invalid-slot rate on those jobs remains no worse than `75%`
- immediate rerun cache reuse remains at least `90%` for both structural and summary caches
- the measured GPU scorer is at least `3x` faster than the CPU scorer on representative large compiled-tensor jobs
- the scoring phase is still a meaningful share of rerun wall time rather than being drowned by unreusable preprocessing

If not:

- do not promote the feature as a major new external mode
- keep it narrow or internal until the economics justify CUDA work

### Phase 5 — GPU scorer

Deliverables:

- isolated options GPU scorer
- backend `Auto` policy
- CPU/GPU parity fixtures
- scorer telemetry

Exit criterion:

- GPU scorer wins clearly on the compiled-tensor workloads that matter and does not destabilize correctness

### Phase 6 — Result artifact + robustness rescore

Deliverables:

- final options-expression artifact kind
- holdout metrics
- stress rescore
- conservative ranking view

Exit criterion:

- the feature can save, reopen, inspect, and compare robust finalists

### Phase 7 — UI polish + CSV export

Deliverables:

- refined results panel
- comparison views
- optional lightweight CSV export
- optional simple heatmaps built from result rows

Exit criterion:

- a user can run, inspect, and compare options-expression jobs end-to-end without opening internal tools

### Phase 8 — Narrow beta

Deliverables:

- tiny allowlist of liquid roots
- documented assumptions
- telemetry review
- correctness issue log
- UX cleanup

Exit criterion:

- stable outputs, understandable diagnostics, and acceptable rerun economics

---

## 20. Kill Criteria

This section is essential.

The feature should stop or narrow further if any of the following happen.

### Kill criterion 1 — parent artifact projection is not deterministic enough

If you cannot reliably project event manifests from current validated artifacts, stop widening the options layer and first add a clean validation-export contract.

### Kill criterion 2 — compiled caches do not amortize

If structural and summary caches are large but rarely reusable, the core performance model is wrong.

At that point, reduce scope sharply or stop before wider rollout.

### Kill criterion 3 — GPU scorer does not materially beat CPU on compiled tensors

If GPU scoring is only marginally faster after the compiler stages are done, do not keep investing in a CUDA route just because VectorGrid is GPU-branded.

Keep the feature as an internal or narrow research preview if needed, but do not present it as a major product mode until the CUDA route earns its place.

### Kill criterion 4 — 0DTE coverage is too thin or too late-day noisy

If late-day coverage and data quality are too poor, keep `0DTE` out of the default profile and leave it as an optional specialized lane.

### Kill criterion 5 — results are too fill-model fragile

If top candidates reorder dramatically between conservative and slightly harsher assumptions, the feature is not yet trustworthy enough to present as optimization.

### Kill criterion 6 — scope pressure pushes toward a general simulator

If the work starts to require:

- multi-leg modeling
- exercise/assignment
- broker semantics
- full portfolio margin
- arbitrary scripting

then v1 discipline has already been lost.

Stop and re-scope.

---

## 21. Benchmark And Verification Plan

### Benchmarks to add

Suggested additions:

- `crates/app_core/benches/options_expression_candidate_compile.rs`
- `crates/app_core/benches/options_expression_summary_compile.rs`
- `crates/app_core/benches/options_expression_cpu_score.rs`
- `crates/app_core/benches/options_expression_gpu_score.rs`

Suggested headless tools:

- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`

### Verification posture

Reuse repo habits:

- workspace tests
- frontend parity/typecheck/lint
- workflow regression style checks
- output-parity style checks for artifacts
- CPU/GPU bench baselines once the scorer exists

### Important measurement split

Always split timing into:

1. signal-manifest projection
2. options-data lookup / candidate compilation
3. summary compilation
4. scoring backend time
5. post-pass robustness rescore

If you collapse these into one number too early, you will not know where the feature is actually paying off.

---

## 22. Final Recommendation

The shortest viable external-product version of this feature is:

**a separate artifact-driven options-expression workflow that compiles irregular options data into dense candidate and exit-summary tensors, proves tensor density and cache reuse early, scores them on a CPU oracle and then on a dedicated GPU scorer, and saves a parent-linked options result artifact without touching the current Double-MA benchmark path.**

That is the version most likely to fit the current VectorGrid repo.

If I had to reduce the entire plan to five rules, they would be:

1. **Separate workflow, not new main strategy kind.**
2. **Derived signal manifest, not broad artifact-schema churn.**
3. **Single-root, long-single-leg, moneyness-first v1.**
4. **CPU oracle first; GPU scorer required for real productization.**
5. **Protect the current Double-MA benchmark route from options feature creep.**

That is the path that keeps VectorGrid focused on doing one thing well while still opening a credible, differentiated options research lane.

---

## Appendix A — Suggested File Touchpoints

### Frontend

- `web/src/stores/workflowArtifactsStore.ts`
- `web/src/services/workflowArtifactsSync.ts`
- `web/src/services/historyService.ts`
- `web/src/services/validationService.ts`
- `web/src/services/optionsExpressionService.ts` *(new)*
- `web/src/stores/optionsExpressionStore.ts` *(new)*
- `web/src/components/OptionsExpressionPanel.vue` *(new)*
- `web/src/composables/useHeadlessMatrixTable.ts`
- `web/src/composables/usePersistedMatrixColumnState.ts`

### Tauri

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/commands_artifacts.rs`
- `src-tauri/src/run_artifacts.rs`
- `src-tauri/src/commands_options_expression.rs` *(new)*
- `src-tauri/src/commands_options_data.rs` *(new)*

### App core

- `crates/app_core/src/options_expression.rs` *(new)*
- `crates/app_core/src/options_signal_manifest.rs` *(new)*
- `crates/app_core/src/options_data_store.rs` *(new)*
- `crates/app_core/src/options_candidate_compiler.rs` *(new)*
- `crates/app_core/src/options_exit_summary.rs` *(new)*
- `crates/app_core/src/options_cpu_ref.rs` *(new)*
- `crates/app_core/src/options_gpu_eval.rs` *(new)*
- `crates/app_core/src/options_result.rs` *(new)*
- `crates/app_core/src/options_types.rs` *(new)*
- `crates/app_core/src/kernels/options_expression.cu` *(new, later phase)*
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`

### Verification / benchmarking

- `crates/app_core/benches/options_expression_candidate_compile.rs` *(new)*
- `crates/app_core/benches/options_expression_cpu_score.rs` *(new)*
- `crates/app_core/benches/options_expression_gpu_score.rs` *(new)*
- `src-tauri/src/bin/ta_options_expression_smoke.rs` *(new)*

---

## Appendix B — 0DTE-First Profile (Optional Narrow Launch)

This appendix should remain optional until the default `1DTE` / `3DTE` / `5DTE` profile is stable and the quote-coverage diagnostics show that same-day entries are trustworthy enough.

If you want the shortest path to answering your original product question around 0DTE expression, freeze an even narrower launch profile:

- one root only
- same-day expiry only
- moneyness buckets: `ATM`, `1 OTM`, `2 OTM`
- delay buckets: `0`, `1`, `5`
- holds: `5`, `15`, `30`, `60`, `EOD`
- targets: `25%`, `50%`, `100%`
- stops: `25%`, `50%`
- conservative fill default
- no delta selectors
- no portfolio overlay in ranking
- no multi-day holds

That profile is narrow, but it is highly aligned with the actual problem statement that motivated the feature.
