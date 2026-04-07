# Options Expression Optimizer — Repo-Fit Viable Implementation Plan

Date: 2026-04-02
Status: revised implementation plan aligned to the current VectorGrid product shape, with the backend staged-compiler slice, CUDA scorer path, current History follow-up reporting/drilldown slice, broadened double-like parent-artifact support, quote-aware CSV import support, and the first bounded fixed-geometry two-leg expansion now landed in the repo while broader non-double-like parent coverage remains future work.

Future expansion work is tracked separately in:

- [options_expression_future_scope_roadmap_2026-04-07.md](/C:/Rust%20Projects/ta_desktop_demo/options_research/options_expression_future_scope_roadmap_2026-04-07.md)

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
4. **A single-underlying-root, moneyness-first v1 with long-single as the default and tightly bounded two-leg extensions allowed only when they preserve tensor regularity.**
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
- long single-leg calls and puts
- bounded same-expiry fixed-geometry two-leg families that preserve deterministic compilation
  - current shipped families: `debit_vertical`, `long_pair`
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

- arbitrary or broader multi-leg spread families
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
- **data resolution:** 1-minute options data
  - prefer Massive minute aggregates in v1
  - treat full historical quotes as an optional higher-realism lane
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

Recommended future expansion order if the currently shipped two-leg families continue to prove performant enough:

1. fixed-ratio two-leg call or put ladders beyond the currently shipped `long_pair`
2. additional same-expiry fixed debit families beyond the currently shipped `debit_vertical`
3. only later, other fixed-geometry bundles if they preserve tensor density and deterministic compilation

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
- make the mode boundary visually explicit so users cannot confuse options follow-up work with the conventional stock optimization/validation workflow
- keep the options surface labeled as an artifact-driven follow-up, not as the default path for ordinary stock backtests

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
It should also be immediately obvious which surface is options-specific and which surface remains the ordinary stock workflow.

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
- persistent source dataset identity
- current strategy/result metadata

It should contain both **timestamp-based** and **index-based** coordinates.

#### Minimum fields

- `manifest_id`
- `parent_artifact_id`
- `parent_run_kind`
- `source_dataset_identity_hash`
- `source_content_fingerprint`
- `dataset_hash`
- `timeframe`
- `resample_spec_hash` if relevant
- `adjustment_mode`
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
- `runtime_dataset_id` optional
  - launch-only handle for the currently loaded app session
- `manifest_version`

#### Why bar indices matter

The original plan correctly emphasizes timestamps.

For repo viability, add **bar indices** too.

That gives:

- deterministic joins back to the current dataset model
- fewer DST/session-boundary bugs
- cheaper indexing during candidate compilation

#### Canonical projector rule

Do not treat manifest projection as a generic reshape of arbitrary run artifacts.

For each supported parent artifact family, define:

- one canonical projector
- one projector version
- one explicit source of fold/split labels
- one explicit rule for recovering `raw_underlier_price` at decision and tradable time

For the first implementation slice, freeze one parent family only.

Recommended default:

- the current validated `DoubleMa` family

Other parent families should remain out of scope until they each have a deterministic projector contract.

#### Persistent identity rule

`runtime_dataset_id` is not persistent identity.

It is only a temporary launch handle for the currently loaded app session.

All persistent manifest and cache identity must be derived from:

- source content fingerprint
- timeframe / resample specification
- adjustment mode
- calendar/session version

not from runtime `data-N` ids.

### 6.3 OptionsDatasetManifestV1

The normalized options data source needs its own manifest.

Minimum fields:

- `options_dataset_id`
- `root_symbol`
- `provider_name`
  - current implementation assumption: `massive` / `polygon`
- `provider_source_surface`
  - current v1 expectation: `minute_aggregate`
- `provider_entitlement_profile`
  - record the entitlement or access assumption used when the dataset was produced
- `quote_resolution`
- `price_semantics`
  - for example: `top_of_book_quote`, `minute_aggregate`, or `derived_mid_snapshot`
- `calendar_id`
- `timezone`
- `coverage_start`
- `coverage_end`
- `normalization_version`
- `quote_semantics_version`
- `corporate_action_mapping_version`
- `contract_reference_source`
- `contract_reference_as_of_utc`
- `underlier_reference_source`

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
- `selection_metrics`
- `holdout_metrics`
- `sealed_evaluation_metrics` optional
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

Minimum summary metadata for list/detail surfaces should include:

- `parent_artifact_id`
- `root_symbol`
- `options_dataset_hash`
- `backend_used`
- `price_semantics`
- whether sealed evaluation metrics are present

### 6.8 Rehydration contract

Do not assume the current generic rerun flow is sufficient.

Options-expression reopen and rerun must define:

- whether the parent underlying dataset is reloaded from source on every reopen
- how timeframe/resample state is reconstructed deterministically
- how source fingerprint mismatches are detected
- how missing or moved underlying CSVs fail
- whether compiled candidate/summary caches can be reused only after identity rechecks succeed
- how options dataset manifests are revalidated before any cached artifact is trusted

The safe default is:

- re-verify parent source identity and options dataset identity on every reopen
- refuse silent reuse when fingerprints, calendar versions, or provider-semantics metadata do not match

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

### Recommended provider posture

Treat the first-wave implementation as **Massive/Polygon-only at the data-contract level**, while keeping it separate from the current Polygon spot/live runtime path.

That means:

- design the import and normalization layers so they record Massive/Polygon provider/source semantics explicitly
- do **not** hardwire the runtime to current Polygon candle-download flows
- do **not** assume that every options data surface has the same entitlement, history depth, recency, or file-size economics

The v1 plan should assume:

- contract/reference data can come from Massive contract-reference exports
- first-wave backtestable price data is more likely to come from Massive minute aggregates than from full historical quotes
- full historical quotes are an advanced realism lane, not the default baseline

### Recommended Massive-first v1 path

The preferred first-wave data shape is:

1. Massive contract-reference data for contract universe and metadata
2. Massive options **1-minute** aggregates for contract price-path research
3. optional later quote-aware imports for higher-realism entry/exit semantics

This is the safer first release because:

- contract metadata is needed for deterministic selector compilation either way
- minute aggregates are dramatically more tractable than full options quote archives
- quote archives are much heavier and have stricter entitlement requirements, so they should not be assumed as the default path

For v1, phrase the mode as:

- options expression research over normalized Massive-derived options data

not:

- a direct live-connected Massive options workstation

### Important provider assumption

Do **not** assume that historical options quotes are broadly available, lightweight, or included in low-tier plans.

The plan should remain valid even if the user only has:

- Massive contract/reference data
- Massive options minute aggregates

and does **not** have:

- full historical top-of-book quote entitlement

If quote-aware datasets are available later, they should enter as a stricter realism profile, not as a prerequisite for the entire feature.

### Recommended v1 ingestion policy

Support import of already-downloaded normalized options datasets first.

Current implementation status:

- the frontend and backend are wired for Polygon/Massive-origin **normalized dataset import**
- the current UI launches import through the native file picker and stores the dataset locally
- the current backend import command normalizes provider metadata to Polygon/Massive assumptions
- the backend now also has a Polygon/Massive provider-download path for contract-reference plus `1m` minute aggregates
- the current options follow-up UI now also exposes a bounded Polygon/Massive `1m` download flow that stays separate from the main stock-data workspace
- live-connected provider ingestion remains a separate future decision, not an implicit part of v1
- the normalized import path still remains supported for manually prepared datasets and provider-derived exports

Suggested first-wave inputs:

- Massive-derived contract-reference exports
- Massive-derived options **1-minute** aggregates
- normalized Parquet
- normalized CSV with a strict schema

For the avoidance of doubt, v1 should treat `1m` as the canonical options-resolution target for the optimizer hot path.
If later work introduces coarser or quote-aware surfaces, those should remain secondary realism or fallback lanes rather than replacing the first-wave `1m` aggregate path.

Optional later:

- Massive-derived historical quote datasets
- higher-fidelity quote snapshots or derived mids built from quote archives

Do not start with:

- vendor download integration
- live options data streaming
- broker-linked options discovery

### Minimum normalized price-row schema

For each imported price row:

- `root_symbol`
- `contract_id`
- `option_type`
- `expiration_date`
- `strike`
- `multiplier`
- `quote_timestamp`
- `bid` optional
- `ask` optional
- `mid` optional or derivable when the provider surface supports it
- `last` optional
- `volume` optional
- `open_interest` optional
- `delta` optional
- `iv` optional
- `quote_age_ms` optional if derivable
- `session_id`

If the imported surface is minute aggregates rather than true quotes, the normalized dataset must still declare that explicitly through `price_semantics` and `provider_source_surface`.

Do not silently label minute aggregates as quotes.

#### Contract-reference time-validity rule

The candidate compiler must not use a post-hoc contract master as if it were timeless truth.

For every eligible contract decision, the backend must be able to prove that the contract reference snapshot used for selection was valid **as of the event timestamp**.

That means:

- contract-reference sources need an explicit `as_of` version or timestamp
- candidate compilation must reject contracts that were not eligible/listed as of the event time
- this rule must be enforced before any GPU-facing dense slot is emitted

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
- provider-surface normalization
  - preserve whether the imported series came from contract reference, minute aggregates, or quotes
- provider-entitlement traceability
  - store enough metadata to explain what realism assumptions the dataset supports

### Semantics gating rule

`price_semantics` must gate what execution models, invalid-reason codes, and target/stop rules are allowed.

At minimum:

- `minute_aggregate` datasets may support aggregate-based research fills and bar-level ambiguity rules
- `top_of_book_quote` datasets may support quote-side fills, quote staleness rules, and spread-based invalidation
- `derived_mid_snapshot` datasets may support mid-style fills only if the derivation rule is explicit and versioned

Do not let aggregate-based datasets silently opt into quote-aware execution semantics.

### Recommended first-wave realism policy

Support two explicit first-wave realism families:

1. **aggregate-based research realism**
   - built from contract metadata + minute aggregates
   - suitable for fast expression research and first-wave optimization
2. **quote-aware realism**
   - built from top-of-book options quotes when available
   - suitable for stricter finalist validation and later advanced lanes

The first family should be enough to start the backend implementation.

The second family should improve realism later without being a prerequisite for the whole feature.

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
3. locate the first valid price row at or after entry time
4. resolve the target expiration bucket
5. resolve the nearest strike/moneyness bucket
6. apply the static filter bundle
7. emit either a valid fixed slot or an invalid slot with a reason code

### Invalid reason codes

Keep the original proposal here, but make them first-class product data:

- no price row after entry
- no expiration in bucket
- no strike in tolerance
- spread too wide
- premium too high
- insufficient volume
- insufficient open interest
- entry past cutoff
- insufficient future path
- stale/missing path

Not every code applies to every `price_semantics` family.

For example:

- `spread too wide` and `stale quote` are quote-aware codes
- aggregate-based datasets need their own bar/path completeness codes and must not fabricate quote-specific failures

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

- `aggregate_research`
- `quote_conservative`
- `quote_mid`

Do not create a large menu of nuanced fill models yet.

Also:

- `aggregate_research` is valid for `minute_aggregate`
- `quote_conservative` and `quote_mid` require `top_of_book_quote`
- fill-model families must be rejected if they do not match the dataset `price_semantics`

---

## 11. Execution Semantics

The original plan is broadly right here.

Freeze these rules before code is written.

### Fill policy

#### Aggregate-based research model (`minute_aggregate`)

For long options:

- use an explicit aggregate-derived entry/exit field, versioned in the dataset semantics
- do not claim true bid/ask realism
- use conservative same-bar ambiguity handling when target and stop are both reachable within one bar
- treat this as research realism, not executable quote realism

#### Quote-aware conservative model (`top_of_book_quote`)

For long options:

- entry at `ask`
- exit at `bid`
- target/stop checks against the exitable side
- fees included

#### Quote-aware research model (`top_of_book_quote` or explicit `derived_mid_snapshot`)

For long options:

- entry at `mid`
- exit at `mid`
- target/stop checks at `mid`

#### Fill-family compatibility rule

The backend must reject incompatible combinations:

- no `ask/bid` model on `minute_aggregate`
- no spread-based invalidation on aggregate-only datasets
- no quote-age or crossed-market logic unless the imported surface actually supports it

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

### Sealed finalist evaluation rule

Do not let the first holdout become a second tuning surface.

At minimum, the implementation must choose one of these:

1. `train -> finalist selection -> final sealed test`
2. walk-forward evaluation where the reported score comes from slices never reused for finalist selection

Also require:

- a frozen finalist-count budget before holdout evaluation
- no repeated manual retuning against the same sealed evaluation slice
- final artifacts to label `search`, `selection`, and `sealed evaluation` metrics separately when applicable

### Stress rescore

Top-K only:

- conservative fill
- widened spread
- higher fees
- stricter liquidity filter

### Required options-layer validation stage

Treat this as mandatory for the feature, even if v1 does not expose it as a full standalone workflow tab.

The key rule is:

**the parent strategy being validated does not validate the options expression layer.**

So every meaningful options-expression run should end with a finalist validation stage that:

- evaluates top tuples on untouched holdout folds
- reruns finalists under harsher fill and liquidity assumptions
- stores in-sample, holdout, and stress results separately
- promotes finalists based on robustness, not only best-case search metrics

For v1, this can ship as:

- a backend-first finalist-validation / robustness pass
- a saved artifact section and results view
- no dedicated top-level `Validate` tab required yet

If the options mode later becomes a discrete shell mode, this stage can grow into a more explicit validation surface.

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
- chunk-size invariance on identical compiled tensors
- cache invalidation when `price_semantics`, fill family, calendar version, or contract-reference `as_of` metadata changes

#### Edge-case fixtures

- no price row after signal
- quote gap mid-trade
- late-day 0DTE entry
- early close session
- DST boundary
- split date / corporate-action mismatch
- stale quote chain
- no strike in selector tolerance
- missing expiration bucket
- contract not listed as of event time
- aggregate dataset incorrectly paired with quote-only fill family

### Trade inspector is part of correctness

Treat the trade inspector as correctness infrastructure, not just UX.

If users cannot inspect why a slot was selected or rejected, trust will collapse quickly.

---

## 19. Revised Phased Roadmap

This roadmap is more viable than the original because it inserts explicit go/no-go checkpoints.

### Working implementation checklist

This section is meant to stay short and operational.

Use it as the current build-status view, while the phase sections below remain the fuller design rationale.

Status key:

- `[x]` implemented in the repo
- `[ ]` not implemented yet
- `[~]` partially implemented / intentionally narrow first slice

#### Completed in the current vertical slice

- `[x]` Add dedicated app-core options types and artifact contracts
- `[x]` Add deterministic `DoubleMa -> SignalEventManifestV1` projector
- `[x]` Add Polygon/Massive normalized options dataset import/load path for single-root datasets
- `[x]` Wire the frontend and backend to import Polygon/Massive-origin options datasets through the normalized local-dataset path
- `[x]` Lock the first-wave optimizer price surface to Polygon/Massive `1m` minute aggregates plus contract-reference metadata
- `[x]` Add backend Polygon/Massive provider-download support for contract-reference plus `1m` minute-aggregate options data
- `[x]` Add options dataset manifest persistence and listing
- `[x]` Add the explicit compiled structural candidate cache / candidate-compiler stage for the current constrained search space
- `[x]` Add the explicit exit-summary cache / exit-summary compiler stage for the current constrained search space
- `[x]` Add staging diagnostics to run results, saved artifacts, and the current frontend follow-up surface
- `[x]` Add CPU reference options-expression scorer for the constrained single-leg search space
- `[x]` Add options-expression artifact persistence, listing, and reopen support
- `[x]` Add Tauri commands for options dataset import/list/load and options-expression run/list/load
- `[x]` Add frontend service bindings for the new commands
- `[x]` Add isolated frontend store for options-expression follow-up state
- `[x]` Mount options expression as a History follow-up panel rather than a new primary workspace mode
- `[x]` Keep the new feature isolated from `workflowArtifactsStore` hydration
- `[x]` Add targeted backend tests for signal-manifest projection, dataset loading, and CPU scoring
- `[x]` Add targeted frontend tests for service/store/component behavior
- `[x]` Verify the full frontend test suite still passes after integration
- `[x]` Verify web typecheck and production build still pass
- `[x]` Harden parent-artifact and saved-artifact path checks to stay inside app-owned run directories
- `[x]` Sanitize options dataset ids before manifest lookup and persisted request usage
- `[x]` Reject unsupported aggregate semantics for the current minute-aggregate v1 path
- `[x]` Stop artifact listing from failing hard on malformed options-expression files
- `[x]` Add CSV export for ranked options-expression tuples in the current History follow-up workflow
- `[x]` Expose invalid-rate plus holdout/sealed/stress reporting columns in the current result table when those metrics are present
- `[x]` Add a tuple detail inspector and dataset-provenance reporting inside the current History follow-up panel
- `[x]` Surface richer saved-artifact summaries plus run/artifact provenance using the existing response payload
- `[x]` Add lazy per-tuple event drilldown in the History follow-up panel without widening the main run payload or touching workflow hydration
- `[x]` Add a feasibility preview path with density, invalid-reason, event-coverage, compile-timing, and replay-cost reporting
- `[x]` Add a headless `ta_options_expression_feasibility` harness so Phase `0.5` can be run from the terminal against saved artifacts
- `[x]` Add a synthetic `ta_options_expression_smoke` runner so the backend can be regression-smoked without local options datasets
- `[x]` Add first-wave Criterion benches for options-expression feasibility preview and CPU scoring
- `[x]` Add a summary-heavy Criterion bench for exit-summary compilation pressure using the current constrained aggregate path
- `[x]` Extend the headless feasibility harness so it can optionally run the full backend and inspect the top tuple from the same CLI entrypoint
- `[x]` Add a headless `ta_options_dataset_import` CLI so normalized options CSVs can be imported from the terminal without using the current panel workflow
- `[x]` Add a headless `ta_options_dataset_download` CLI so the backend Polygon/Massive download path can be exercised without building the dedicated options-download UI first
- `[x]` Add a headless `ta_options_parent_artifact` CLI so a bounded `DoubleMa` parent artifact can be generated from local `1m` stock bars without going through the UI
- `[x]` Include candidate-miss reason codes in tuple drilldown rows and surface them in the current options inspection table
- `[x]` Add a headless `ta_options_expression_compare` tool plus reusable headless artifact list/load helpers for comparing saved options-expression artifacts without widening the frontend workflow
- `[x]` Add in-process structural candidate cache reuse and exit-summary cache reuse for repeated identical previews/runs inside one process
- `[x]` Expose cache hit/miss telemetry in the feasibility report and add warm-preview iteration reporting in the headless feasibility harness
- `[x]` Enrich tuple-inspection payloads with path-level trade context (entry lag, realized hold, expiry distance, and option/underlier path extrema) without widening the main run payload
- `[x]` Surface feasibility cache reuse telemetry in the current options preview panel
- `[x]` Add inspection-row CSV export for the current tuple drilldown table without widening the main run payload
- `[x]` Convert sealed evaluation into a real finalist-budget stage and surface finalist rank/status in saved artifacts and the current History follow-up panel
- `[x]` Add a second conservative robustness lane for sealed finalists and surface it in the current options result reporting
- `[x]` Add a selectable event-detail card inside the lazy tuple drilldown so the current History follow-up panel has a real row-level trade inspector without widening the backend payload again
- `[x]` Add row-level event-path replay from saved artifacts plus richer artifact-summary reporting inside the current History follow-up surface
- `[x]` Implement the GPU scorer / compiled-tensor evaluation path for CUDA builds and keep CPU exactification for retained finalists
- `[x]` Expose backend preference plus CPU/GPU feasibility timing/reporting in the current History follow-up surface
- `[x]` Make sealed finalist rank reflect sealed-evaluation ordering instead of only pre-sealed search order
- `[x]` Surface the broader harsher robustness lane end to end in artifact summaries, feasibility preview reporting, result tables, tuple detail, and CSV export
- `[x]` Add a CUDA-gated `options_expression_gpu_score` Criterion bench plus synthetic smoke compare coverage so CPU-vs-GPU proof work can start before real imported options datasets are wired in
- `[x]` Expose sealed finalist count as a first-class frontend control for both preview and run requests instead of keeping it implicit
- `[x]` Align the headless Phase `0.5` feasibility harness with explicit sealed-finalist-count input instead of keeping it implicit
- `[x]` Add a repo-local `run_options_expression_phase05.ps1` wrapper so import + feasibility + compare output can be repeated once a real normalized options CSV is available
- `[x]` Add explicit Phase `0.5` GPU-viability gate badges plus structured per-lane diagnostics in the current History follow-up panel
- `[x]` Add client-side tuple-table sort/filter controls plus richer run-provenance dataset/filter reporting in the current History follow-up panel
- `[x]` Add direct Polygon/Massive options download integration in the current options UI while keeping it visually separate from the normal stock-data workflow
- `[x]` Add a non-breaking session-aware split-assignment mode for backend/headless experiments while keeping event-level chronology as the current default
- `[x]` Expose the split-assignment mode through the headless feasibility CLI and the repo-local Phase `0.5` PowerShell wrapper for repeatable real-data experiments
- `[x]` Surface split-assignment mode in the advanced History follow-up panel and run-provenance summary while keeping `event_chronological` as the default UI path
- `[x]` Carry microsecond hybrid-ranking timings through feasibility reports, headless compare output, and the current History follow-up summary so sub-millisecond preview work does not collapse to `0 ms`
- `[x]` Switch the first-wave CUDA scorer from split-array full-event scans to contiguous event-range launches for search/holdout workloads and capture an updated large-grid Phase `0.5` compare run
- `[x]` Prebuild flat GPU score tensors during prepare and reuse uploaded shared/outcome buffers plus tuple-sized output buffers inside the thread-local CUDA runtime
- `[x]` Split GPU feasibility timing into tensor-score vs CPU exactify vs sealed-finalist finalize spans, and surface the breakdown plus GPU upload-reuse telemetry in the current History follow-up panel
- `[x]` Keep the current History follow-up default backend preference at `cpu_reference` until the Phase `0.5` GPU promotion gate is actually closed, while leaving explicit `gpu_compiled_tensor` available for experiments
- `[x]` Add quality recommendations to the Phase `0.5` harness output and the current History follow-up panel so density failures point users toward strike/DTE/cutoff adjustments instead of treating the GPU scorer as the blocker
- `[x]` Add a dedicated `run_options_expression_gpu_promotion_gate.ps1` wrapper so the current GPU-promotion decision can be repeated from the terminal against the latest provider-backed artifact/dataset pair
- `[x]` Add repo-root `.env` fallback parsing for `POLYGON_KEY` / `POLYGON_API_KEY` so local provider-backed headless/download workflows can run without requiring a separately exported shell variable when no saved app key is present
- `[x]` Add process-local parent-signal and imported-dataset replay caches so repeated feasibility/rerun work stops rebuilding the same artifact and options CSV inputs on every call
- `[x]` Persist derived `SignalEventManifestV1` replay artifacts under an app-owned options replay cache so restart no longer forces a full parent-signal reprojection on every follow-up run
- `[x]` Stamp explicit session ids into the current `DoubleMa`-derived options signal manifests so `session_chronological` follow-up work no longer relies on implicit UTC-day grouping
- `[x]` Restore the current options follow-up default backend preference to gate-driven `auto` once the scorer stops treating “GPU available” as equivalent to “GPU recommended”
- `[x]` Add a gate-driven `auto` backend policy so the current options scorer no longer treats “GPU available” as equivalent to “GPU recommended,” and surface the recommendation plus reasons in feasibility preview reporting
- `[x]` Surface full options-dataset manifest inspection plus dataset-path reveal in the current History follow-up panel
- `[x]` Surface latest Polygon/Massive download audit details, including the underlying CSV/cache provenance, in the current History follow-up panel
- `[x]` Add a saved-artifact comparison surface for options-expression follow-ups from the same parent run without widening the main workflow shell
- `[x]` Switch options-expression artifact listing to the canonical artifact-library path while keeping the dedicated backend/headless commands available for internal tooling
- `[x]` Base CUDA tuple-chunk planning on current free device memory plus explicit usable-memory headroom rather than cached total VRAM alone
- `[x]` Surface explicit GPU planner fit/fallback telemetry and backend-fallback reasons through feasibility previews, saved artifacts, and the current History follow-up panel
- `[x]` Carry backend fallback reasons through saved-artifact summaries plus the headless compare / feasibility full-run surfaces so CPU results can be distinguished from GPU fallback runs
- `[x]` Carry auto-backend recommendation and GPU planner fit/note details through the repeated-preview feasibility summaries and markdown output instead of dropping them outside the base feasibility JSON
- `[x]` Add backend unit coverage for feasibility-report GPU-planner failure handling so planner notes and forced-CPU auto-backend overrides are pinned by Rust tests rather than frontend mocks alone

#### Still pending for the full roadmap

- `[x]` Run the documented Phase `0.5` compiled-tensor feasibility spike and capture real density/cache metrics
  Current measured status:
  real SPY `1m` slices executed on April 6-7, 2026 using provider-downloaded contract-reference + minute aggregates, bounded saved `DoubleMa` parent artifacts, and the headless feasibility harness. The first under-centered strike window (`500-560`) failed density/invalid gates and exposed that the strike band was misaligned with live SPY levels. The corrected near-spot strike window (`645-665`) produced real slices that pass the density/cache gates. The current authoritative product-promotion evidence is the broader expiry-covered DTE7 slice (`2026-03-31` through `2026-04-02`, strikes `645-665`, `max_dte_days=7`, `105` contracts, `54,985` option rows) captured after the newer GPU-planner/fit telemetry work and rerun quietly with the updated feasibility summary surfaces. On that slice the current measured result is: valid-slot density `0.5247`, invalid-slot rate `0.4753`, structural-cache reuse `1.0000`, summary-cache reuse `1.0000`, repeated hybrid-ranking GPU speedup vs CPU `5.3197x`, and estimated backend-only preview wall speedup vs CPU `1.3333x`. That means the density/cache gates and the scorer-speed gate are now met on the authoritative DTE7 slice, but the bounded preview still auto-recommends CPU because the search-event count and projected event-tuple work remain below the current auto thresholds. Earlier compare runs remain useful as exploratory profiler evidence, but the `_cuda_features` DTE7 pair below is the authoritative proof set for current promotion decisions. Concrete evidence:
  - sparse first pass: `output/options_expression_phase05/spy_phase05_overlap_trimmed_relaxed.{json,md}`
  - corrected dense pass: `output/options_expression_phase05/spy_phase05_overlap_trimmed_relaxed_645_665.{json,md}`
  - CUDA compare pass: `output/options_expression_phase05/spy_phase05_overlap_trimmed_relaxed_645_665_compare.{json,md}`
  - multi-day dense pass: `output/options_expression_phase05/spy_phase05_multi_day_relaxed.{json,md}`
  - multi-day CUDA compare pass: `output/options_expression_phase05/spy_phase05_multi_day_relaxed_compare.{json,md}`
  - multi-day large-grid precise CUDA compare pass: `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise.{json,md}`
  - multi-day large-grid range-based CUDA compare pass: `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_range.{json,md}`
  - multi-day large-grid cached-upload CUDA compare pass: `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached.{json,md}`
  - multi-day large-grid replay-cache CUDA compare pass: `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_replaycache.{json,md}`
- authoritative DTE7 promotion evidence with current telemetry: `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.{json,md}`
- `[x]` Prove the GPU scorer materially beats CPU on representative large jobs before product promotion
- `[x]` Promote `Auto` toward GPU-first productization while retaining CPU as the correctness oracle and explicit fallback/debug lane
  Current shipped posture:
  the options follow-up UI defaults to `auto`, the backend `Auto` policy now uses materially lower workload floors and a lower measured-speedup gate than the original conservative experiment-only posture, and representative provider-backed DTE7 evidence now clears the scorer-speed gate. CPU remains retained for correctness, forced debugging, and fallback when planner/runtime conditions fail.
- `[~]` Keep using measured provider-slice diagnostics to choose representative strike/DTE windows before concluding on GPU productization
  Current measured lesson from the first SPY slice:
  an under-centered strike band (`500-560`) produced artificial `no_strike_in_tolerance` failure even though the backend/compiler path was correct. The corrected near-spot band (`645-665`) cleared the density and invalid-rate gates on the same underlying day.
- `[x]` Add robustness rescore variants beyond the current narrow baseline
- `[~]` Add any further scenario-rescore families only if later real-data usage proves they are worth the extra tensor and reporting complexity
  Current shipped scope:
  baseline, conservative, harsher, plus an additional finalist-only `extreme` stress lane are implemented. Any broader scenario family should now be justified by concrete usage rather than roadmap symmetry.
- `[x]` Add sealed finalist evaluation artifact/reporting beyond the current current finalist/stress summaries once broader comparison surfaces proved necessary
- `[x]` Add richer trade inspector / full path-level event drilldown beyond the original lazy tuple table baseline
  Current shipped scope:
  tuple drilldown now includes split/status rollups, invalid-reason rollups, row-level selection, companion-leg detail where applicable, event-path replay, event-path analytics, and CSV export for both tuple rows and replay paths.
- `[x]` Add broader reporting surfaces beyond the current CSV export path
- `[x]` Broaden parent artifact support from `DoubleMa` to the current double-like saved-run families (`DoubleMa`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`) while keeping non-double-like strategies read-only
- `[x]` Expose quote-aware Polygon/Massive CSV import semantics in the current options UI while keeping direct provider download on the bounded `1m` aggregate lane
- `[~]` Add optional post-v1 multi-leg fixed-geometry expression families only if tensor density remains acceptable
  Current shipped scope:
  `long_single` remains the default structure family, and two bounded same-expiry fixed-geometry two-leg families are now implemented: `debit_vertical` and `long_pair`. Any broader ladders, additional bundles, or more general multi-leg geometry remain post-v1.
- `[x]` Keep the normalized import path supported even after direct provider download is added

#### Intentionally narrow first-slice constraints that still hold

- `[~]` Parent artifact support is currently limited to the current double-like saved-run families (`DoubleMa`, `CompositeTrend`, `RegimeHybrid`, `BreakoutMomentum`)
- `[~]` Direct provider download remains limited to the constrained `1m` minute-aggregate path, while quote-aware realism currently enters through imported Polygon/Massive CSV datasets
- `[~]` The canonical first-wave options resolution is Polygon/Massive `1m`; coarser or quote-aware surfaces remain secondary lanes
- `[~]` Current Polygon/Massive support is bounded `1m` import/download inside the options follow-up path, not a generic live-connected provider workflow or a reuse of the main stock-data panel
- `[~]` CPU remains the correctness oracle and the retained-finalist exactifier; the CUDA scorer is implemented and now has representative provider-backed proof, but CPU remains the explicit fallback/debug lane
- `[~]` The current options UI default backend is `auto`; explicit `cpu_reference` and `gpu_compiled_tensor` remain available for oracle/debug and forced-CUDA experiments
- `[~]` The currently supported structure families are `long_single` plus two bounded same-expiry fixed-geometry two-leg families (`debit_vertical`, `long_pair`); broader multi-leg, rolling, and assignment-style modeling remain out of scope
- `[~]` The current frontend host is the History follow-up panel, not a dedicated options mode

### Phase 0 — Repo-fit contract spike

Deliverables:

- freeze product boundary
- freeze first-spike parent artifact family
- audit current validated artifact kinds and run-artifact loading paths for deterministic manifest projection requirements
- define parent artifact -> SignalEventManifestV1 projection
- define persistent dataset identity rules separate from runtime `data_id`
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

Current measured baseline, corrected real-data runs:

- provider/data path: Polygon/Massive-derived SPY contract-reference + `1m` minute aggregates
- underlying parent: saved `DoubleMa` artifact generated from bounded SPY `1m` stock bars aligned to the options coverage window
- artifact/result evidence:
  - `output/options_expression_phase05/spy_phase05_overlap_trimmed_relaxed_645_665.json`
  - `output/options_expression_phase05/spy_phase05_overlap_trimmed_relaxed_645_665.md`
  - `output/options_expression_phase05/spy_phase05_overlap_trimmed_relaxed_645_665_compare.json`
  - `output/options_expression_phase05/spy_phase05_overlap_trimmed_relaxed_645_665_compare.md`
  - `output/options_expression_phase05/spy_phase05_multi_day_relaxed.json`
  - `output/options_expression_phase05/spy_phase05_multi_day_relaxed.md`
  - `output/options_expression_phase05/spy_phase05_multi_day_relaxed_compare.json`
  - `output/options_expression_phase05/spy_phase05_multi_day_relaxed_compare.md`
  - `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`
  - `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.md`
- measured outcome on April 6, 2026:
  - valid-slot density: `1.0000`
  - invalid-slot rate: `0.0000`
  - structural-cache reuse: `1.0000`
  - summary-cache reuse: `1.0000`
  - GPU compare result on the corrected small slice: accelerated preview wall speedup vs CPU `0.6875x` (`gpu_compiled_tensor_hybrid` slower than CPU on this small workload)
- measured outcome on April 7, 2026 for the larger three-session SPY slice (`2026-03-31` through `2026-04-02`, strikes `645-665`):
  - valid-slot density: `0.6111`
  - invalid-slot rate: `0.3889`
  - structural-cache reuse: `1.0000`
  - summary-cache reuse: `1.0000`
  - GPU compare result on the larger slice: accelerated preview wall speedup vs CPU `0.9651x` (`gpu_compiled_tensor_hybrid` still slightly slower than CPU, but materially closer than the first small-slice compare)
- measured outcome on April 7, 2026 for the authoritative broader DTE-covered representative grid on the same three-session SPY slice (`2026-03-31` through `2026-04-02`, strikes `645-665`, `max_dte_days=7`, `105` contracts) after the current free-memory-aware GPU planner and corrected fit/fallback telemetry:
  - valid-slot density: `0.5247`
  - invalid-slot rate: `0.4753`
  - structural-cache reuse: `1.0000`
  - summary-cache reuse: `1.0000`
  - CPU hybrid-ranking median: `1.697 ms`
  - accelerated hybrid-ranking median: `0.319 ms`
  - repeated hybrid-ranking GPU speedup vs CPU: `5.3197x`
  - estimated backend-only preview wall speedup vs CPU: `1.3333x`
  - current auto-backend recommendation on that bounded job: `cpu_reference`
  - concrete evidence:
    - `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.json`
    - `output/options_expression_phase05/spy_phase05_multi_day_large_grid_compare_cuda_precise_cached_dte7_cuda_features.md`
- measured outcome on April 7, 2026 for the same larger three-session slice under the new `session_chronological` split-assignment mode (preview-only, CPU-safe):
  - valid-slot density: `0.6111`
  - invalid-slot rate: `0.3889`
  - holdout events with candidates: `3` (up from `0` under event chronology)
  - sealed-test events with candidates: `5`
  - concrete evidence:
    - `output/options_expression_phase05/spy_phase05_multi_day_relaxed_session_split_preview.json`
    - `output/options_expression_phase05/spy_phase05_multi_day_relaxed_session_split_preview.md`
- measured notes from the corrected provider-backed slices:
  - candidate compilation and exit-summary density are now high enough to justify further GPU proof work
  - the broader DTE-covered representative slice now meets the density/cache gates and clears the documented `3x` scorer-speed threshold on the authoritative quiet rerun
  - replay/process-local cache reuse is strong enough to justify retaining a gate-driven `auto` mode, but the current bounded preview still recommends CPU on the measured DTE7 job because workload size remains below the auto-promotion floors even though scorer speed is now favorable
  - late-session `entry_past_cutoff` invalidations are still a real factor on multi-session runs, but the new session-aware split mode materially improves holdout coverage without changing the default path yet

Interpretation:

- the backend/provider path is now real and functional
- the candidate/compiler path is viable on a real liquid-root slice when the downloaded strike window matches the actual underlier regime
- the next engineering step is further GPU productization work and larger quiet-environment representative compares before treating the GPU scorer as promotion-ready

### Phase 1 — Artifact handoff and diagnostics shell

Deliverables:

- history/validation action: **Express with Options**
- options-expression store/service shell
- parent artifact rehydration
- parent-artifact diagnostics and identity checks with no scorer yet

Exit criterion:

- a validated artifact can launch into an options-expression draft workflow and surface deterministic parent-artifact / rehydration diagnostics

### Phase 2 — Options data import and normalization

Deliverables:

- Polygon/Massive normalized import path
- Polygon/Massive-first provider adapters for:
  - contract-reference imports
  - `1m` minute-aggregate imports
- dataset manifest
- coverage queries
- session/calendar alignment
- price/quote cleanup rules
- provider-surface semantics captured in the manifest
- contract-reference `as_of` semantics enforced
- direct-provider download contract defined for later options-UI integration, while keeping backend-first implementation order

Exit criterion:

- for one liquid root and date range, normalized Massive-derived contract metadata and minute-price access are reliable, auditable, and time-valid as of the event horizon

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
- required finalist validation stage for the options layer
- search / selection / sealed-evaluation metrics separation where applicable
- holdout metrics
- stress rescore
- conservative ranking view

Exit criterion:

- the feature can save, reopen, inspect, validate, and compare robust finalists

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
