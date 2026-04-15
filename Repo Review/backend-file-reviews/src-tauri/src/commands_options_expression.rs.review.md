# Review Packet: `src-tauri/src/commands_options_expression.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_options_expression.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Capable and already fairly well tested, but still too contract-heavy and side-effectful to accept as-is`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Artifact-contract-sensitive`, `Filesystem-contract-sensitive`, `Replay-cache-sensitive`

## What This File Owns

`commands_options_expression.rs` is the canonical backend boundary for the options-expression workflow.

This file owns:

- public Tauri commands for:
  - run execution: `src-tauri/src/commands_options_expression.rs:1933-1947`
  - feasibility preview: `src-tauri/src/commands_options_expression.rs:1949-1962`
  - artifact listing: `src-tauri/src/commands_options_expression.rs:1964-1974`
  - artifact load: `src-tauri/src/commands_options_expression.rs:1976-1988`
  - CSV export write: `src-tauri/src/commands_options_expression.rs:1990-1997`
  - tuple inspection: `src-tauri/src/commands_options_expression.rs:1999-2066`
  - event-path inspection: `src-tauri/src/commands_options_expression.rs:2067-2138`
- public headless entrypoints for the same workflow:
  - `src-tauri/src/commands_options_expression.rs:1782-1896`
- replay-input reconstruction from a saved parent artifact plus an imported options dataset:
  - `src-tauri/src/commands_options_expression.rs:1427-1717`
- in-process and persisted replay caches:
  - process caches: `src-tauri/src/commands_options_expression.rs:429-537`
  - persisted caches: `src-tauri/src/commands_options_expression.rs:791-949`
- local cache and artifact layout:
  - datasets/replay cache/runs directories: `src-tauri/src/commands_options_expression.rs:652-689`
- options-expression artifact building and summary generation:
  - artifact creation: `src-tauri/src/commands_options_expression.rs:1069-1097`
  - summary generation: `src-tauri/src/commands_options_expression.rs:1199-1384`
- saved-artifact replay provenance validation:
  - `src-tauri/src/commands_options_expression.rs:1394-1424`

This file does not own:

- options dataset import/download
- the core options-expression execution engine
- parent-strategy adaptation rules
- artifact-library aggregation across workflow families

Those concerns are delegated to:

- `src-tauri/src/commands_options_data.rs`
- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/options_parent_adapters.rs`
- `src-tauri/src/run_artifacts.rs`

## Classification

- Public surface: yes
  - commands are re-exported from `src-tauri/src/commands.rs:119-127`
  - commands are registered in `src-tauri/src/lib.rs:171-177`
- Hot path: no
  - this file orchestrates workflow execution, replay building, artifact access, and inspection
- Runtime-critical: high
  - it owns the live backend path for running options-expression analysis and replaying saved artifacts
- Benchmark-sensitive: moderate
  - replay-cache behavior and dataset loading materially affect perceived performance, even though the core compute lives in app-core
- CUDA/runtime-critical: indirectly
  - backend preference and artifact replay behavior affect CPU/GPU route evaluation, but this file is not itself CUDA code
- Strategy-local: yes
  - this is a dedicated options-expression workflow module
- Low-risk support code only: no
  - it owns artifact identity, path rules, cache rules, and public DTOs

## Direct Interactions

### Backend owners this file depends on

- options dataset contract shared with the import/download surface:
  - `src-tauri/src/commands_options_data.rs`
- parent-strategy adaptation:
  - `src-tauri/src/options_parent_adapters.rs:57-199`
- artifact-library bridge:
  - `src-tauri/src/run_artifacts.rs:8-14`
  - `src-tauri/src/run_artifacts.rs:326`
  - `src-tauri/src/run_artifacts.rs:2137`
  - `src-tauri/src/run_artifacts.rs:2338`
- core execution engine:
  - `crates/app_core/src/options_expression.rs:5222`

### Backend files that depend on this file’s contract

- live command hub and invoke surface:
  - `src-tauri/src/commands.rs:119-127`
  - `src-tauri/src/lib.rs:171-177`
- headless CLI consumers:
  - `src-tauri/src/bin/ta_options_expression_feasibility.rs`
  - `src-tauri/src/bin/ta_options_expression_compare.rs`
- artifact library snapshot uses this file’s summary DTO and summarizer:
  - `src-tauri/src/run_artifacts.rs`

### Frontend contract anchors

- typed command allowlist:
  - `web/src/tauri/commands.ts:91-97`
- primary service wrappers:
  - `web/src/services/optionsExpressionService.ts:1356-1409`
  - `web/src/services/optionsExpressionService.ts:1681-1695`
- artifact library is consumed indirectly through `list_artifact_library`, not the dedicated list command:
  - `web/src/services/optionsExpressionService.ts:1368-1376`
- main store consumer:
  - `web/src/stores/optionsExpressionStore.ts:645`
  - `web/src/stores/optionsExpressionStore.ts:739`
  - `web/src/stores/optionsExpressionStore.ts:786`
  - `web/src/stores/optionsExpressionStore.ts:849`
  - `web/src/stores/optionsExpressionStore.ts:1014`
  - `web/src/stores/optionsExpressionStore.ts:1035`

### REPO_MAP anchors

- module inventory:
  - `REPO_MAP.md:100`

## Standards and Research Log

This packet was written after a targeted official-source review focused on Tauri command behavior, filesystem containment, replay-cache serialization tradeoffs, blocking HTTP usage in async-adjacent contexts, and public API predictability.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this file exposes a large public Tauri command surface
   - Standard informed:
     - command DTOs, path rules, and error shapes are public contracts

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this module exposes many public DTOs and behavior-heavy command surfaces
   - Standard informed:
     - public behavior should be predictable, validated, and well documented

3. Rust `std::fs::canonicalize`
   - Source: <https://doc.rust-lang.org/std/fs/fn.canonicalize.html>
   - Why it was consulted:
     - this file contains several path-containment checks for artifacts and caches
   - Standard informed:
     - path containment should be explicit when loading user-controlled or artifact-controlled filesystem paths

4. Rust `std::fs::create_dir_all`
   - Source: <https://doc.rust-lang.org/std/fs/fn.create_dir_all.html>
   - Why it was consulted:
     - multiple helpers here create directories eagerly
   - Standard informed:
     - `create_dir_all` is a real write-side effect and should not be hidden inside read-only flows casually

5. reqwest blocking module docs
   - Source: <https://docs.rs/reqwest/latest/reqwest/blocking/index.html>
   - Why it was consulted:
     - replay reconstruction can reach blocking I/O and this repo uses blocking reqwest elsewhere in adjacent options flows
   - Standard informed:
     - blocking clients belong behind blocking boundaries, not directly inside async runtime execution

6. bincode crate documentation
   - Source: <https://docs.rs/crate/bincode/latest>
   - Why it was consulted:
     - persisted replay caches use `bincode = "1"` in `src-tauri/Cargo.toml`
   - Standard informed:
     - cache formats should remain rebuildable and corruption-tolerant
     - the current cache dependency is a long-term maintenance risk and should be treated as technical debt rather than a casual permanent choice

## Applicable Current Standards

- Tauri boundary standard:
  - async command wrappers should keep heavy work in `spawn_blocking`
  - path arguments and returned artifact/data DTOs are public IPC contracts
- Filesystem-boundary standard:
  - artifact and cache loading must remain directory-contained and explicit about side effects
  - read-only operations should avoid unnecessary directory creation where feasible
- Replay-cache standard:
  - caches are accelerators, not sources of truth
  - corrupt caches should degrade to cache misses rather than break the workflow when safe to do so
- Artifact-contract standard:
  - artifact kinds, file naming, and summary fields are downstream contract surfaces for both backend and frontend artifact library consumers
- Validation standard:
  - derived summary values should reflect actual tuple data, not synthetic defaults

## Findings

### Correctness / Safety

#### 1. Artifact summaries can report impossible “best” values when all tuples are negative

Objective issue:

- summary calculation uses `fold(0.0_f64, f64::max)` for:
  - `best_robust_score`: `src-tauri/src/commands_options_expression.rs:1206-1210`
  - `best_selection_expectancy_per_premium`: `src-tauri/src/commands_options_expression.rs:1211-1215`

Impact:

- if every tuple metric is negative, the summary reports `0.0`, even though no tuple achieved `0.0`
- artifact-library summaries become objectively incorrect for all-negative result sets

Smallest justified fix:

- switch both calculations to `max_by(total_cmp)` with an explicit empty-list fallback
- add a unit test with all-negative tuples

Sensitivity:

- safe-now

#### 2. Artifact listing can reopen arbitrary parent artifact paths embedded in saved options-expression artifacts

Objective issue:

- `summarize_options_expression_artifact` resolves parent strategy kind by reaching back out through the artifact’s stored parent path:
  - `src-tauri/src/commands_options_expression.rs:1204`
  - helper chain starts at `src-tauri/src/commands_options_expression.rs:453`
- artifact listing feeds saved artifacts directly into that summary path:
  - `src-tauri/src/commands_options_expression.rs:1858-1860`

Impact:

- a malformed or hand-edited options-expression artifact under `runs/` can trigger unrelated file reads during listing
- listing is less self-contained than the more explicit replay/inspection containment rules elsewhere in the file

Smallest justified fix:

- either constrain parent artifact reopening to the same allowed `runs/` directory
- or persist the needed parent strategy metadata into the options-expression artifact itself and stop reopening parent files during summary generation

Sensitivity:

- safe-now

### Error Handling

#### 3. A corrupt persisted signal-manifest replay cache can break the whole replay build instead of degrading to a cache miss

Objective issue:

- `load_signal_manifest_replay_cache` hard-fails on read or deserialize errors:
  - `src-tauri/src/commands_options_expression.rs:791-817`
- replay-input building propagates that result directly:
  - `src-tauri/src/commands_options_expression.rs:1515-1519`

Impact:

- a corrupt cache file can block replay construction even though the cache is only an accelerator
- that is stricter than necessary and weaker than the dataset replay cache fallback behavior in the same module

Smallest justified fix:

- treat signal-manifest cache read/deserialize failures as `None`
- optionally delete the bad cache file
- rebuild from the parent artifact instead of failing the whole workflow

Related hardening:

- persisted signal-manifest cache writes currently go directly to the final path:
  - `src-tauri/src/commands_options_expression.rs:819-843`
- if cache corruption matters operationally, consider temp-file + atomic rename on write

Sensitivity:

- safe-now

#### 4. Multiple read-oriented paths create directories as a side effect

Objective issue:

- these helpers all call `create_dir_all`:
  - `options_datasets_dir`: `src-tauri/src/commands_options_expression.rs:652-658`
  - `options_replay_cache_dir`: `src-tauri/src/commands_options_expression.rs:661-667`
  - `signal_manifest_replay_cache_dir`: `src-tauri/src/commands_options_expression.rs:670-676`
  - `options_dataset_replay_cache_dir`: `src-tauri/src/commands_options_expression.rs:678-683`
  - `runs_dir`: `src-tauri/src/commands_options_expression.rs:686-689`
- they are used by read-heavy flows such as:
  - replay-input building: `src-tauri/src/commands_options_expression.rs:1438` and `1593`
  - artifact listing: `src-tauri/src/commands_options_expression.rs:1829`
  - artifact load: `src-tauri/src/commands_options_expression.rs:1876`

Impact:

- read-only list/load/replay validation paths silently mutate disk
- these commands can fail in read-only environments for reasons unrelated to the requested read

Smallest justified fix:

- split each helper into a read-only resolver and a writer-only `ensure_*` function

Sensitivity:

- safe-now

#### 5. Artifact listing silently drops unreadable or malformed artifacts

Objective issue:

- artifact listing uses tolerant `continue` behavior on parse/deserialization/kind failures:
  - `src-tauri/src/commands_options_expression.rs:1849-1856`

Impact:

- corrupted artifacts disappear from the library view with no diagnostic signal
- operators cannot distinguish “no artifact” from “artifact exists but was skipped”

Smallest justified fix:

- keep tolerant listing behavior
- add a warning/log path or return skipped/error counts alongside summaries

Sensitivity:

- safe-now

### API and Naming

#### 6. The dedicated `list_options_expression_artifacts` command is not the frontend’s real listing contract

Objective observation:

- the command exists and is publicly registered:
  - `src-tauri/src/commands_options_expression.rs:1964-1974`
- but the frontend service currently loads options-expression summaries from `list_artifact_library` instead:
  - `web/src/services/optionsExpressionService.ts:1368-1376`

Impact:

- the true frontend contract for options-expression artifact summaries is shared with `run_artifacts.rs`, not solely this command
- maintaining both surfaces without explicit ownership notes increases confusion

Recommended action:

- document that the artifact-library summary path is the live frontend surface
- decide later whether the dedicated list command is primarily for headless/CLI use or should be consolidated

Sensitivity:

- document-only

### Module Boundaries / Tech Debt

#### 7. The module is carrying too many independently complex responsibilities

Objective issue:

- the same file owns:
  - Tauri command wrappers
  - replay-input reconstruction
  - path validation
  - process and persisted caches
  - artifact building
  - artifact summary generation
  - tuple/event inspection
  - CSV export

Impact:

- local changes require understanding a very wide surface area
- options-dataset contract changes, artifact changes, and replay-cache changes are coupled in one file

Recommended action:

- keep the public surface here for now
- defer any extraction until a later bounded refactor that separates:
  - replay/cache helpers
  - artifact summary/load helpers
  - command wrappers

Sensitivity:

- larger refactor but defer

#### 8. The persisted replay cache currently depends on `bincode = "1"`, which is now an upstream maintenance risk

Objective issue:

- local dependency:
  - `src-tauri/Cargo.toml:21`
- persisted dataset replay cache uses bincode serialization:
  - `src-tauri/src/commands_options_expression.rs:863`
  - `src-tauri/src/commands_options_expression.rs:942`

Impact:

- this is not an immediate correctness bug
- but it is real technical debt for a persisted cache format used in a critical workflow

Recommended action:

- do not churn the cache format immediately
- record this as deferred dependency-risk debt and revisit deliberately if cache-format work is already in scope

Sensitivity:

- defer

### Docs / Examples

#### 9. Public headless helpers and commands do not explain their filesystem effects or replay-cache behavior

Objective issue:

- the exported command/headless surface:
  - `src-tauri/src/commands_options_expression.rs:1782-2068`
- has little local documentation explaining:
  - which paths must stay inside `runs/`
  - which helpers may write cache files
  - which provenance checks are enforced for replay/inspection

Impact:

- maintainers have to infer operational behavior by tracing code paths
- that is expensive in a module this large

Smallest justified fix:

- add short rustdoc/comments for the public entrypoints and key path helpers

Sensitivity:

- document-only

### Tests / Missing Verification

#### 10. Coverage is substantial but still helper-heavy and command-boundary-light

Objective issue:

- local test run:
  - `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`
  - passed `15` tests
- in-file tests cover:
  - replay cache compatibility
  - artifact summary shaping
  - provenance checks
  - CSV save behavior
  - artifact verdict refresh
- there is no Rust integration coverage in `src-tauri/tests` for the public Tauri command surface
- frontend coverage is mostly mocked-IPC service/store/component coverage

Notable remaining gaps:

- no end-to-end Rust test for:
  - `run_options_expression`
  - `analyze_options_expression_feasibility`
  - `load_options_expression_artifact`
  - one inspection path
- no Rust integration test for path containment and artifact-kind enforcement at the command wrapper layer
- no dedicated service test for `inspectOptionsExpressionEventPath(...)`

Smallest justified fix:

- add a small `src-tauri/tests` runtime suite for the command surface
- add direct frontend service coverage for event-path inspection invoke shape

Sensitivity:

- safe-now

### Perf / Allocation / CUDA Concerns

#### 11. Replay-cache behavior matters more than micro-optimizing wrapper code

Observation:

- replay inputs already use:
  - process-memory caches
  - persisted replay caches
  - dataset content fingerprints
  - versioned cache formats
- most meaningful performance behavior in this file is therefore tied to cache hit/miss and artifact/dataset I/O, not local instruction-level tuning

Recommended action:

- prioritize corruption-tolerant caches, correct summary values, and verification first
- avoid speculative micro-optimization in the wrappers

## Recommended Action

- Overall packet disposition: `Small surgical refactor`

Safe-now changes:

- fix all-negative artifact summary maxima
- degrade corrupt signal-manifest replay cache to a cache miss
- split read-only vs writer-only directory helpers
- constrain or eliminate parent-artifact reopening during artifact summary generation
- add diagnostics for skipped malformed artifacts
- add command-boundary runtime tests

Document-only changes:

- clarify that artifact-library listing is the real frontend contract
- add local docs for filesystem effects, path constraints, and replay-cache behavior

Larger refactor but defer:

- module extraction of replay/cache helpers vs command wrappers vs artifact-summary helpers
- cache-format/dependency migration away from `bincode = "1"`

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_options_expression -- --nocapture`
2. Add and run targeted Rust integration tests for:
   - `run_options_expression`
   - `analyze_options_expression_feasibility`
   - `load_options_expression_artifact`
   - one inspection command
3. Add targeted unit tests for:
   - all-negative tuple summary maxima
   - corrupt signal-manifest cache fallback behavior
   - read-only listing/loading without directory-creation side effects
4. Rerun frontend consumers covering:
   - `web/src/services/optionsExpressionService.test.ts`
   - `web/src/stores/optionsExpressionStore.test.ts`
   - `web/src/components/OptionsExpressionPanel.test.ts`
5. If artifact summary fields or artifact-library behavior changes, verify `run_artifacts.rs`-backed library flows still return expected options-expression summaries

## Cross-File Coordination

- `src-tauri/src/commands_options_data.rs`
  - owns the imported options dataset store and manifest contract reused here
- `src-tauri/src/options_parent_adapters.rs`
  - owns supported-parent replay extraction/adaptation
- `src-tauri/src/run_artifacts.rs`
  - owns the artifact-library bridge that the frontend actually uses for options-expression summaries
- `crates/app_core/src/options_expression.rs`
  - owns the core options-expression execution engine
- `web/src/services/optionsExpressionService.ts`
  - owns the direct command wrappers and normalization surface
- `web/src/stores/optionsExpressionStore.ts`
  - is the main frontend orchestration consumer

## Code-Quality and Debt Register

- High-value debt:
  - summary maxima can report impossible values
  - signal-manifest replay cache is too strict on corruption
  - command-boundary integration coverage is missing
- Medium-value debt:
  - read-oriented helpers create directories as a side effect
  - artifact listing silently hides malformed artifacts
  - artifact listing reopens parent artifact paths too freely
- Low-value debt:
  - dedicated list command is not the real frontend listing surface
  - public entrypoints are under-documented
  - persisted cache dependency risk from `bincode = "1"`

## Performance Opportunity Register

- Safe-now:
  - none beyond making caches corruption-tolerant and keeping summary math correct
- Measurement-first:
  - any change to replay-cache format or cache invalidation rules
  - any concurrency or run-lock changes around feasibility preview vs run execution
- Not worth chasing here:
  - wrapper micro-optimizations detached from cache behavior

## Implementation Staging

1. Safe now
   - summary maxima fix
   - signal-manifest cache fallback hardening
   - read-only helper split
   - safer artifact listing behavior
   - runtime/integration coverage improvements
2. Coordination required
   - any change to artifact-library summary behavior consumed via `run_artifacts.rs`
3. Defer until later pass
   - module extraction
   - persisted-cache format migration / dependency replacement

## Change Blast Radius

- High blast radius:
  - artifact kind, summary fields, or artifact-library behavior changes
  - dataset replay/cache invalidation behavior changes
- Medium blast radius:
  - path-containment behavior for artifact summary or inspection
  - concurrency semantics around run vs feasibility preview
- Low blast radius:
  - all-negative max fix
  - cache corruption fallback
  - added tests and documentation

## Observability Gaps

- no diagnostic signal for skipped malformed artifacts during listing
- no test-backed visibility into the public Tauri command layer
- no explicit operator signal when signal-manifest replay cache entries are invalidated or rebuilt

## Open Questions and Assumptions

- Assumption:
  - feasibility preview is intentionally allowed outside the shared run lock, even though it can perform substantial replay-building work
- Open question:
  - should artifact summaries stop reopening parent artifacts entirely by persisting the required parent metadata?
- Open question:
  - should the dedicated `list_options_expression_artifacts` command remain public if the frontend continues to consume the artifact library instead?
