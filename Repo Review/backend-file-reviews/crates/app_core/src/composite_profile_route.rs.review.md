# Review Packet: `crates/app_core/src/composite_profile_route.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_profile_route.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Internally solid and well-tested, but not fully accept-as-is because route-policy ownership and naming semantics are under-documented and the same classification logic is duplicated across adjacent surfaces`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Internal-policy-surface`, `Route-contract-sensitive`, `Cross-file-drift-sensitive`, `Low-throughput-risk`

## What This File Owns

`composite_profile_route.rs` owns a narrow but important policy layer for the composite profile family. It does not execute optimization or validation work itself. Instead, it classifies a `StrategyConfig` into the current composite profile kind, exposes a runtime-family label used by callers when describing the route, and exposes the current intended GPU target for that profile.

This file owns:

- profile classification from strategy flags:
  - `composite_profile_from_strategy`: `crates/app_core/src/composite_profile_route.rs:11-20`
- route-label helpers:
  - `composite_profile_runtime_label`: `24-31`
  - `composite_profile_gpu_target_label`: `34-39`
- current intended GPU target policy:
  - `composite_profile_gpu_target`: `42-63`
- file-local unit tests that lock current precedence and target semantics:
  - `65-239`

This file does not own:

- realized backend execution routing
  - that happens later in optimization/validation owners such as `composite_cpu.rs` and `double_ma.rs`
- strategy validation
  - mixed-flag validity and broader request semantics are checked elsewhere
- frontend policy preview
  - the frontend has its own mirror for route previews

## Classification

- Public surface: no
  - all items are `pub(crate)`
- Hot path: no
  - this file is classification-only and allocation-free
- CUDA/runtime-critical: indirect only
  - it influences target-route selection, but it does not own CUDA launch or fallback mechanics
- Strategy-local: yes
  - it is specific to the composite-profile family
- Low-risk support code: mostly yes
  - the main risk is policy drift, not throughput or memory safety

## Direct Interactions

### Backend interaction map

- shared profile enum consumed and produced here:
  - `crates/app_core/src/strategy_ir.rs:79-85`
- input strategy flags and ADX helpers consumed here:
  - `crates/strategies/src/double_ma/mod.rs:624-670`
  - `crates/strategies/src/double_ma/mod.rs:1027-1029`
  - `crates/strategies/src/double_ma/mod.rs:1162-1164`
- optimization route summaries and fallback messages:
  - `crates/app_core/src/composite_cpu.rs:10992-11018`
- plain-`CompositeCpu` validation gating:
  - `crates/app_core/src/composite_validate.rs:667`
  - `crates/app_core/src/composite_validate.rs:5843-5894`
- duplicated backend profile classification logic:
  - `crates/app_core/src/strategy_dynamic.rs:635-646`
- realized backend downgrade/upgrade logic that can differ from this helper's target classification:
  - `crates/app_core/src/double_ma.rs:5260-5271`
  - tests at `crates/app_core/src/double_ma.rs:17843-17988`

### Out-of-scope but parity-sensitive mirror

- frontend route-preview mirror:
  - `web/src/strategies/compositeProfileGpuPolicy.ts:52-63`
- frontend parity tests:
  - `web/src/strategies/compositeProfileGpuPolicy.test.ts:9-80`

### Downstream verification impact if changed

- any change to classification precedence affects:
  - composite optimization routing
  - validation admission rules for plain `CompositeCpu`
  - dynamic strategy-plan compilation
  - frontend route previews if the mirror is not updated
- any change to label semantics affects:
  - backend route diagnostics
  - tests that lock current runtime and GPU-target labels

## Standards And Research Log

This packet was written after targeted web research focused on API predictability, naming clarity, and test-contract quality for internal policy helpers whose main risk is semantic drift rather than algorithmic failure.

### Sources consulted

1. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this file defines internal API helpers that other backend modules rely on for route-policy decisions
   - Standard informed:
     - helper surfaces should remain predictable, easy to audit, and aligned with documented behavior

2. Rust API Guidelines, Predictability
   - Source: <https://rust-lang.github.io/api-guidelines/predictability.html>
   - Why it was consulted:
     - this file exposes naming and classification helpers whose behavior can be misread if names imply realized routing rather than intended policy
   - Standard informed:
     - helper names and outputs should make semantics unsurprising to callers

3. Rust API Guidelines, Naming
   - Source: <https://rust-lang.github.io/api-guidelines/naming.html>
   - Why it was consulted:
     - the file’s `runtime_label` helper returns CPU-family strings even when the GPU target is kernel or hybrid
   - Standard informed:
     - names should communicate intent precisely enough to avoid contract drift

4. Rust Book, How to Write Tests
   - Source: <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
   - Why it was consulted:
     - this file’s real quality bar is mostly in the test contract that locks current precedence and label behavior
   - Standard informed:
     - tests should explicitly encode expected behavior and fail clearly when route policy drifts

## Findings

## Correctness And Safety

### 1. No current in-file correctness defect was identified

- Evidence:
  - precedence is internally consistent:
    - breakout wins first: `crates/app_core/src/composite_profile_route.rs:12-13`
    - regime wins next: `14-15`
    - trend wins next: `16-17`
    - default falls back to `CompositeCpu`: `18-19`
  - ADX-sensitive GPU target mapping is internally consistent:
    - trend path: `44-50`
    - breakout path: `51-57`
    - regime hybrid path: `58`
  - file-local tests lock the intended matrix:
    - `69-97`
    - `99-145`
    - `148-239`
- Why it matters:
  - this packet should not invent a defect where the current logic and tests agree
- Review judgment:
  - accept current logic as coherent
  - focus improvement effort on ownership clarity and drift control

## API And Naming

### 1. `composite_profile_runtime_label` is easy to misread as a realized backend label

- Evidence:
  - the helper name suggests runtime/backend identity:
    - `crates/app_core/src/composite_profile_route.rs:24`
  - but it returns CPU-family strings for profiles that can target GPU or GPU-hybrid:
    - `"CompositeTrendCpu"`: `27`
    - `"RegimeHybridCpu"`: `28`
    - `"BreakoutMomentumCpu"`: `29`
  - the same strategies can still target GPU routes through:
    - `42-63`
  - callers use the label in route summaries:
    - `crates/app_core/src/composite_cpu.rs:10992-11018`
- Why it matters:
  - future contributors can reasonably infer that the label reflects the realized backend route when it really reflects the CPU-family runtime grouping currently used by surrounding code
- Smallest justified fix:
  - add rustdoc stating that this helper returns the composite runtime-family label, not the realized backend route
  - if later compatibility review permits, rename the helper to something more explicit such as `composite_profile_cpu_family_label`
- Review judgment:
  - documentation-first fix is safe now
  - renaming is semantics-sensitive because tests and callers already lock the current vocabulary

### 2. The file behaves like a target-policy helper, but that ownership is not documented

- Evidence:
  - the file computes intended target categories only:
    - `crates/app_core/src/composite_profile_route.rs:42-63`
  - realized execution can still be rewritten later by environment or runtime constraints:
    - `crates/app_core/src/double_ma.rs:5260-5271`
  - tests in `double_ma.rs` lock those later route adjustments:
    - `17843-17988`
- Why it matters:
  - contributors can over-ascribe authority to this file and miss the later owners that decide whether the target route is actually realizable
- Smallest justified fix:
  - add top-of-file rustdoc describing this module as classification/target-policy only and pointing to execution owners for final route realization
- Review judgment:
  - safe-now clarity improvement

## Error Handling

### 1. No dedicated error-handling issue exists in this file

- Evidence:
  - all helpers are total functions over `StrategyConfig` and do not allocate or return `Result`
- Why it matters:
  - the main risk surface here is semantic drift, not recoverable error transport
- Review judgment:
  - no change needed in-file

## Module Boundaries And Technical Debt

### 1. Composite profile classification is duplicated in another backend module

- Evidence:
  - this file encodes profile precedence at:
    - `crates/app_core/src/composite_profile_route.rs:11-20`
  - `strategy_dynamic.rs` duplicates the same precedence logic:
    - `crates/app_core/src/strategy_dynamic.rs:635-646`
- Why it matters:
  - if one copy changes without the other, the repo can silently disagree about which composite family a strategy belongs to
  - this is a genuine maintenance hazard because the duplicated logic is short enough to drift casually
- Smallest justified fix:
  - route `strategy_dynamic.rs` through the shared helper instead of maintaining a second local classifier
  - if ownership boundaries make reuse awkward, add an explicit parity test that compares the two classifiers across representative flag matrices
- Review judgment:
  - safe-now technical-debt reduction

### 2. Unreachable `DoubleMa` match arms obscure the true domain of these helpers

- Evidence:
  - `composite_profile_from_strategy` never returns `StrategyProfileKind::DoubleMa`:
    - `crates/app_core/src/composite_profile_route.rs:11-20`
  - but the label and target helpers still carry `DoubleMa` arms:
    - `30`
    - `59-60`
- Why it matters:
  - the extra arms are harmless, but they make the helper domain look broader than it currently is
- Smallest justified fix:
  - either document that the arms are defensive compatibility ballast
  - or remove them in a small cleanup after confirming no future caller depends on that wider domain
- Review judgment:
  - low-severity debt
  - safe to defer if the team prefers explicit exhaustiveness over domain minimalism

### 3. Backend policy and frontend route-preview policy can drift independently

- Evidence:
  - backend target policy:
    - `crates/app_core/src/composite_profile_route.rs:42-63`
  - frontend mirror:
    - `web/src/strategies/compositeProfileGpuPolicy.ts:52-63`
  - frontend tests lock a similar route matrix:
    - `web/src/strategies/compositeProfileGpuPolicy.test.ts:9-80`
- Why it matters:
  - even though frontend files are out of scope for this campaign, this backend helper participates in a cross-surface contract
  - policy drift would confuse route previews, diagnostics, and user expectations
- Smallest justified fix:
  - add a backend review note requiring frontend parity verification whenever this file changes
  - longer term, derive parity tests from a shared route matrix or serialized fixture
- Review judgment:
  - cross-surface debt worth tracking now

## Docs And Examples

### 1. Precedence and ownership rules are encoded only in code/tests

- Evidence:
  - precedence is defined in code:
    - `crates/app_core/src/composite_profile_route.rs:11-20`
  - precedence is indirectly documented only by tests:
    - `148-168`
- Why it matters:
  - readers need to infer why mixed-flag states collapse to `breakout > regime > trend > base composite`
  - the file also does not say whether mixed-flag states are expected, tolerated, or later rejected elsewhere
- Smallest justified fix:
  - add rustdoc summarizing the precedence rule and stating that this file classifies current policy rather than validating all mixed-flag combinations
- Review judgment:
  - safe-now documentation hardening

## Tests And Missing Verification

### 1. File-local coverage is good, but cross-file parity coverage is missing

- Evidence:
  - local tests cover:
    - ADX/no-ADX split for trend and breakout: `69-126`
    - regime and precedence rules: `129-168`
    - default route and label contract: `171-183`
    - GPU target label matrix: `186-239`
  - there is no direct parity test against:
    - duplicated backend classifier in `strategy_dynamic.rs:635-646`
    - frontend mirror in `web/src/strategies/compositeProfileGpuPolicy.ts:52-63`
- Why it matters:
  - current unit tests prove local behavior only
  - they do not fail when adjacent mirrors drift
- Smallest justified fix:
  - add a compact parity matrix test in Rust for backend duplication
  - require coordinated frontend verification when this file changes
- Review judgment:
  - highest-value missing verification

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance opportunity exists in-file

- Evidence:
  - the file is branch-only classification logic with no heap allocation and no iteration-heavy work:
    - `crates/app_core/src/composite_profile_route.rs:11-63`
- Why it matters:
  - performance edits here would amount to churn, not value
- Review judgment:
  - accept as-is for performance
  - spend optimization effort in execution owners instead

### 2. Benchmark-sensitive changes are effectively absent for this file

- Evidence:
  - this module does not launch kernels, manage buffers, or participate in throughput loops
- Why it matters:
  - recommendations should not pretend every backend file has a CUDA/perf angle
- Review judgment:
  - no benchmark gate needed for documentation-only or deduplication-only changes here

## Code-Quality Improvement Register

- safe-now:
  - document that this module owns target classification only, not realized routing
  - document precedence explicitly
  - add a backend parity test against duplicated classification logic
- defer-later:
  - rename `composite_profile_runtime_label` only if the team is willing to update diagnostics/tests as a compatibility-reviewed cleanup

## Technical-Debt Reduction Register

- safe-now:
  - remove or explain duplicated profile-classification logic in `strategy_dynamic.rs`
  - add an explicit cross-surface verification note for the frontend route-preview mirror
- defer:
  - remove `DoubleMa` arms only after confirming no future broadening of helper inputs is intended
- do not casually “clean up”:
  - the current label strings are already part of route-diagnostic expectations, so vocabulary changes should not be treated as cosmetic

## Performance Opportunity Register

- safe structural improvements:
  - none needed in-file
- benchmark-sensitive candidates:
  - none
- measurement-first note:
  - if this helper is ever inlined into a larger routing refactor, treat the work as semantics-sensitive rather than benchmark-sensitive

## Recommended Action

- Overall bucket: `Small surgical refactor`
- Accept as-is:
  - current precedence and GPU-target logic
  - current file-local tests
- Document only:
  - target-policy ownership
  - precedence rule
  - meaning of `runtime_label`
- Small surgical refactor:
  - deduplicate backend classification logic or add backend parity tests
- Larger refactor but defer:
  - rename helpers/labels only if a broader route-diagnostic vocabulary cleanup is planned
- Benchmark-sensitive; do not touch yet:
  - none

## Exact Local Verification Required If Changed

If this file changes, run at minimum:

1. `cargo test -p ta_app_core composite_profile_route -- --nocapture`
2. `cargo test -p ta_app_core composite_cpu -- --nocapture`
3. `cargo test -p ta_app_core composite_validate -- --nocapture`
4. any targeted tests covering `strategy_dynamic` route compilation if the classifier is deduplicated there
5. frontend parity verification for the out-of-scope mirror:
   - `web/src/strategies/compositeProfileGpuPolicy.test.ts`

## Regression Risk And Feature-Preservation Constraints

This file should not lose any existing route vocabulary or precedence behavior unless the product route matrix is intentionally being changed across backend and frontend together.

The main regression risk is semantic drift:

- breaking route-diagnostic strings that callers or tests already expect
- changing precedence without updating validation, optimization, and frontend preview logic together
- changing target-policy semantics while forgetting that realized execution still depends on later runtime owners

The correct improvement posture for this file is additive clarification plus parity hardening, not behavioral simplification.
