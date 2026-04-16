# ML Signal Native Inference Backend Plan 2026-04-15

This document defines the implementation plan for moving `MlSignal` toward a native runtime path that maximizes performance while preserving the current additive workflow:

1. use CatBoost GPU where it already exists and is justified
2. move runtime model application into Rust
3. keep the current score-cache workflow working during rollout
4. avoid regressions to desktop validation, policy sweep, replay, and saved-run/history flows

The target is not "rewrite CatBoost in CUDA." The target is:

- external CatBoost training remains external
- runtime inference becomes native
- GPU is used only where it is already supported by CatBoost or where measured bottlenecks justify it
- CPU-native Rust fallback remains first-class

## Current Repo State

Repo-local constraints:

- `MlSignal` is currently a separate research workflow, not part of the normal optimizer route:
  - [REPO_MAP.md](</C:/Rust Projects/ta_desktop_demo/REPO_MAP.md:1215>)
  - [REPO_MAP.md](</C:/Rust Projects/ta_desktop_demo/REPO_MAP.md:1496>)
- Current high-level stages are:
  - dataset export
  - score-cache generation
  - validation
  - policy sweep
- `MlSignal` is currently a dedicated CPU research path:
  - [REPO_MAP.md](</C:/Rust Projects/ta_desktop_demo/REPO_MAP.md:1217>)
- Runtime entry points already exist in:
  - [crates/app_core/src/ml_signal.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/ml_signal.rs:1>)
  - [src-tauri/src/commands_ml_signal.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_ml_signal.rs:1>)
  - [web/src/services/mlSignalService.ts](</C:/Rust Projects/ta_desktop_demo/web/src/services/mlSignalService.ts:1>)
  - [web/src/components/MlSignalWorkspacePanel.vue](</C:/Rust Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue:1>)
- Existing workspace CUDA infrastructure is built around indicator/backtest kernels, not gradient-boosted tree inference:
  - [REPO_MAP.md](</C:/Rust Projects/ta_desktop_demo/REPO_MAP.md:193>)
  - [REPO_MAP.md](</C:/Rust Projects/ta_desktop_demo/REPO_MAP.md:200>)
  - [REPO_MAP.md](</C:/Rust Projects/ta_desktop_demo/REPO_MAP.md:985>)

Implication:

- `MlSignal` native inference should not start as a custom `vector-ta` CUDA kernel effort.
- The fastest viable path is CatBoost-native training plus Rust-managed runtime inference.

## External Support Assumptions

CatBoost support that matters for this plan:

- official GPU training support exists
- official Rust model-application support exists
- official native evaluation libraries exist
- native artifacts can be built with CUDA support

Primary-source references:

- CatBoost applying models overview: <https://catboost.ai/docs/en/concepts/applying__models>
- CatBoost Rust apply docs: <https://catboost.ai/docs/en/concepts/apply-rust>
- CatBoost C/C++ evaluation library docs: <https://catboost.ai/docs/en/concepts/c-plus-plus-api_dynamic-c-pluplus-wrapper>
- CatBoost native build docs: <https://catboost.ai/docs/en/installation/build-native-artifacts>
- CatBoost GPU training docs: <https://catboost.ai/docs/en/concepts/speed-up-training>
- CatBoost Python fit docs with GPU notes: <https://catboost.ai/docs/en/concepts/python-reference_catboost_fit>

Operational interpretation:

- training on GPU is straightforward and already supported in CatBoost
- runtime inference on CPU is straightforward through native model application
- runtime inference on GPU is possible only through CatBoost-native evaluation support and FFI, not through the repo's existing backtest CUDA kernels

## Architectural Decision

Recommended runtime backend order:

1. `NativeCpu`
2. `NativeCudaFfi`
3. legacy `ScoreCacheOnly`

Reason:

- `NativeCpu` is the lowest-risk path to native runtime and removes Python from application-time inference.
- `NativeCudaFfi` should be added only after CPU-native correctness and benchmark evidence exist.
- `ScoreCacheOnly` remains necessary for compatibility, replay, and fallback.

Training order:

1. keep Python CatBoost training
2. enable GPU training in the Python harness when available
3. do not attempt native Rust CatBoost training in this repo

## Backend Shape

Introduce an explicit runtime backend contract under `MlSignal`.

Proposed enum:

```rust
enum MlSignalInferenceBackend {
    ScoreCacheOnly,
    NativeCpu,
    NativeCudaFfi,
}
```

Proposed runtime contract:

- load `manifest.json`
- load `feature_schema.json`
- load `calibration.json` if present
- load CatBoost model artifacts
- score the currently loaded dataset candles directly
- emit the same effective probability/raw-margin columns the score-cache flow expects
- optionally persist those native scores into a cache artifact for replay and saved-run parity

Non-goal:

- do not replace the existing validation/policy sweep replay engine
- do not merge `MlSignal` into optimizer/drilldown/grid flows

## Proposed File Layout

Keep the implementation strategy-local.

Preferred additive shape:

- `crates/app_core/src/ml_signal.rs`
  - keep orchestration, validation, policy sweep, artifact contracts
- `crates/app_core/src/ml_signal_model.rs`
  - bundle manifest/model loading
  - feature-schema and calibration loading
  - backend selection
- `crates/app_core/src/ml_signal_infer_cpu.rs`
  - native CPU inference path
  - calibrated probability application
  - score row emission
- `crates/app_core/src/ml_signal_infer_ffi.rs`
  - CatBoost native library bindings
  - CPU/GPU-capable applier bridge
  - explicit unsafe isolation
- `crates/app_core/src/ml_signal_feature_frame.rs`
  - optional extraction split from `ml_signal.rs` if runtime scoring needs a reusable feature-frame builder
- `src-tauri/src/commands_ml_signal.rs`
  - new command surface for native score generation / native validation
- `web/src/services/mlSignalService.ts`
  - request shaping
  - backend preference
  - compatibility fallback
- `web/src/components/MlSignalWorkspacePanel.vue`
  - backend selection/status
  - GPU availability/status
  - explicit fallback notes

If the feature-frame extraction does not need reuse outside `ml_signal.rs`, keep it in place and only split the model/backend code.

## Phase Plan

### Phase 0: Benchmark And Contract Freeze

Goal:

- establish where the actual runtime cost is before adding a native backend

Work:

- benchmark current Stage 2 score-cache generation from external CSV
- benchmark current validation and policy sweep cost separately
- benchmark feature-frame construction cost inside `ml_signal.rs`
- benchmark a no-op "load bundle but do not score" path
- record whether scoring or replay dominates runtime

Outputs:

- baseline timing report under `research/`
- go/no-go on whether native inference alone materially improves operator latency

Acceptance:

- clear timing split across:
  - feature extraction
  - model application
  - validation replay
  - policy sweep expansion

### Phase 1: GPU-Enabled Training, No Runtime Change

Goal:

- maximize training performance immediately without destabilizing the app

Work:

- extend `research/ml_signal/train_catboost.py` to accept explicit GPU training configuration:
  - `task_type=GPU`
  - `devices`
  - optional GPU-specific border count / training knobs only when explicitly requested
- persist training device/mode in:
  - `manifest.json`
  - `train_metadata.json`
  - calibration metadata
- add harness-level environment diagnostics:
  - CatBoost build GPU support present/missing
  - visible GPUs
  - selected training device(s)

Files:

- [research/ml_signal/train_catboost.py](</C:/Rust Projects/ta_desktop_demo/research/ml_signal/train_catboost.py:1>)
- [research/ml_signal/README.md](</C:/Rust Projects/ta_desktop_demo/research/ml_signal/README.md:1>)
- research tests under [research/ml_signal/tests](</C:/Rust Projects/ta_desktop_demo/research/ml_signal/tests:1>)

Acceptance:

- CPU training path remains default and unchanged
- GPU training is opt-in
- bundle metadata clearly records whether the model was GPU-trained

### Phase 2: Rust-Native CPU Inference MVP

Goal:

- remove Python from application-time model application

Scope:

- CPU inference only
- no CUDA runtime yet
- preserve score-cache compatibility

Work:

- add bundle loader in Rust for:
  - `manifest.json`
  - `feature_schema.json`
  - `calibration.json`
  - CatBoost model artifacts
- implement feature-frame generation reuse from exported `MlSignal` dataset logic
- implement native model application on the current candles
- apply stored sigmoid/isotonic calibration in Rust
- produce in-memory score rows matching current score-cache expectations:
  - `timestamp`
  - `raw_long`
  - `raw_short`
  - `p_long`
  - `p_short`
- add a "native score build" command that writes the same cache JSON/manifest format the current Stage 2 uses

Files:

- [crates/app_core/src/ml_signal.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/ml_signal.rs:1>)
- [src-tauri/src/commands_ml_signal.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_ml_signal.rs:1>)
- [web/src/services/mlSignalService.ts](</C:/Rust Projects/ta_desktop_demo/web/src/services/mlSignalService.ts:1>)
- [web/src/components/MlSignalWorkspacePanel.vue](</C:/Rust Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue:1>)

Acceptance:

- for the same bundle and dataset, native CPU inference reproduces probability columns within tolerance against Python-produced scores
- validation and policy sweep can consume native-generated score caches without any schema change
- saved-run/history surfaces remain valid

### Phase 3: Rust FFI Bridge To CatBoost Native Applier

Goal:

- put model application on the fastest native path available

Scope:

- CPU first through the native applier
- same Rust orchestration layer regardless of backend

Work:

- add an FFI wrapper over CatBoost native evaluation library
- isolate all unsafe code into one small module
- add bundle loading and prediction APIs that return:
  - raw margins
  - probabilities if available, otherwise apply calibration in Rust
- add build scripts or vendored-binary handling for Windows desktop packaging
- define deployment rules for:
  - dev
  - CI
  - release builds

Key decision:

- if the official Rust CatBoost application package is sufficiently stable on Windows packaging, prefer it
- otherwise prefer direct FFI to `catboostmodel` / native evaluation library

Acceptance:

- CPU-native FFI path is at least as fast as the earlier CPU-native MVP
- packaging/build instructions are repeatable on Windows
- the app can select native runtime scoring without Python installed

### Phase 4: Optional CUDA Runtime Inference Backend

Goal:

- use GPU for model application only if profiling proves it matters

Important constraint:

- do not write a custom CatBoost CUDA scorer in this repo
- use CatBoost-native CUDA-capable evaluation support through the same Rust FFI layer

Work:

- extend backend selection to prefer `NativeCudaFfi` when:
  - app build includes the optional backend
  - CatBoost native library with CUDA support is available
  - supported NVIDIA driver/device is present
  - bundle/model type is supported by the native applier
- add startup/runtime capability probing
- add explicit UI/backend reporting:
  - CUDA runtime available/unavailable
  - reason for fallback
- benchmark:
  - small dataset
  - medium dataset
  - large dataset
  - repeated policy-sweep scoring reuse

Expected outcome:

- GPU inference may help only when model scoring itself is a meaningful fraction of end-to-end time
- if validation replay dominates total time, CUDA scoring may not justify operational complexity

Acceptance:

- GPU backend is optional and never required
- CPU backend remains default-safe
- GPU fallback to CPU is automatic and explicit in status text

### Phase 5: Native Validation Entry Path

Goal:

- reduce operator friction by removing mandatory external score-cache generation

Work:

- add a native validation request shape that accepts:
  - bundle id/path
  - backend preference
  - optional persistence mode
- runtime path:
  - build feature frame
  - score natively
  - run existing validation replay
  - optionally persist the generated score cache and manifest
- keep the existing external score-cache flow as a compatible fallback

Files:

- [src-tauri/src/commands_ml_signal.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_ml_signal.rs:1>)
- [web/src/services/mlSignalService.ts](</C:/Rust Projects/ta_desktop_demo/web/src/services/mlSignalService.ts:1>)
- [web/src/components/MlSignalWorkspacePanel.vue](</C:/Rust Projects/ta_desktop_demo/web/src/components/MlSignalWorkspacePanel.vue:1>)

Acceptance:

- user can validate directly from a bundle without manually producing a score CSV first
- persisted artifacts still expose the same provenance, scope, and compatibility metadata

### Phase 6: Native Policy Sweep Entry Path

Goal:

- allow policy sweep to consume freshly scored native outputs in the same run

Work:

- mirror Phase 5 for policy sweep
- allow reuse of a native in-memory score surface across multiple policy candidates in one run
- preserve current `MlSignal` policy axis limits and replay semantics

Acceptance:

- no change to sweep math or selection semantics
- lower operator latency when starting from a bundle rather than a prebuilt score cache

## Backend Selection Policy

Recommended default policy:

1. if native CUDA backend is enabled and healthy, use `NativeCudaFfi`
2. else if native CPU backend is enabled and healthy, use `NativeCpu`
3. else use `ScoreCacheOnly`

Do not hide fallback reasons.

Required status fields:

- requested backend
- actual backend used
- backend capability summary
- explicit fallback reason
- bundle/model compatibility status

## Build And Packaging Strategy

This is the main risk area.

Plan:

- keep native inference behind a feature flag initially:
  - example: `ml-signal-native-infer`
  - optional second feature: `ml-signal-native-cuda`
- do not make CatBoost native libraries mandatory for normal workspace builds
- ensure CPU-only workspace builds still succeed with no CatBoost native dependency installed
- add packaging notes for Tauri release builds on Windows

Recommended packaging order:

1. local dev with manually provided native library
2. CI artifact packaging for Windows
3. release packaging only after repeated success

## Verification Matrix

### Research / Python

- confirm CPU and GPU training produce bundle metadata with correct device info
- confirm calibration metadata is still additive
- confirm built-in baselines still work

### Rust Unit Tests

- bundle loading
- schema/calibration parsing
- sigmoid and isotonic calibration application
- backend selection and fallback
- native score rows match cache contract

### Golden Parity Tests

For a fixed dataset and bundle:

- compare Python `validation_scores.csv` against Rust-native CPU outputs
- compare Python rolling/proxy scoring against native score generation where applicable
- tolerance should be explicit and small

### Integration Tests

- Tauri command for native score build
- native validation
- native policy sweep
- saved-run persistence
- history/detail replay

### Performance Tests

- CPU-native inference vs score-cache ingest
- CPU-native inference vs external Python scoring workflow
- CUDA-native inference vs CPU-native inference
- end-to-end validation latency with and without native scoring

## Recommended Rollout Order

Recommended PR order:

1. Phase 0 benchmark and contract freeze
2. Phase 1 GPU training improvements only
3. Phase 2 CPU-native inference MVP with cache persistence
4. Phase 3 CatBoost native applier FFI
5. Phase 5 native validation path
6. Phase 6 native policy sweep path
7. Phase 4 CUDA runtime backend only if benchmarks justify it

This order is deliberate:

- it gets performance wins from training GPU immediately
- it removes Python from runtime before adding CUDA complexity
- it keeps the existing score-cache artifact contract alive throughout rollout
- it avoids betting on CUDA inference before measuring the real bottleneck

## Go / No-Go Criteria

Proceed to CUDA runtime only if all are true:

- CPU-native inference is correct and shipped behind a stable feature flag
- native scoring is a measurable fraction of end-to-end latency
- validation replay is not overwhelmingly dominant
- CatBoost native GPU applier packaging is stable on Windows
- fallback and status reporting are fully explicit

Do not proceed to a custom repo-owned CUDA tree scorer under this plan.

## Bottom-Line Recommendation

Best-performance practical roadmap for this repo:

1. keep CatBoost training external and enable GPU training there first
2. add Rust-native CPU inference next
3. unify runtime scoring behind a native backend selector
4. only add CUDA runtime inference through CatBoost-native applier support if benchmarks prove it matters

That maximizes performance without forcing the repo into a second custom GPU execution stack that does not match its current CUDA architecture.
