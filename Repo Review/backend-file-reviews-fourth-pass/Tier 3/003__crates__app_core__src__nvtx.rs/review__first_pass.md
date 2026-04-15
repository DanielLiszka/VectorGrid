# Review Packet: `crates/app_core/src/nvtx.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/nvtx.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Operationally useful as a low-overhead profiling shim, but not fully public-review ready because the domain/color path is effectively disabled in the active code path, the public API silently degrades to a no-op for several error/setup conditions, and dormant FFI scaffolding plus an effectively compile-time-inert feature flag make the supported behavior harder to reason about than it should be`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface`, `FFI`, `Profiling-critical`, `Benchmark-sensitive`, `Runtime-adjacent`

## What This File Owns

`nvtx.rs` owns runtime NVTX integration for backend profiling: environment gating, dynamic library discovery, symbol loading, range start/pop wrappers, fallback naming, and RAII cleanup.

It owns:

- environment and ABI constants:
  - `NVTX_ENV`, version/message/color constants:
    - `crates/app_core/src/nvtx.rs:6-12`
- internal domain and color taxonomy:
  - `Domain`: `25-28`
  - `color` module: `30-43`
- FFI type aliases and loaded-API carrier:
  - FFI function-pointer aliases:
    - `14-22`
  - `NvtxApi`: `47-53`
  - event/message/payload FFI structs:
    - `56-103`
- public RAII guard:
  - `RangeGuard`: `106-140`
- public and crate-local range entry points:
  - `engine_range`: `142-144`
  - `kernel_range`: `146-148`
  - `scope`: `150-156`
  - `range`: `159-213`
- domain-handle creation and dynamic library loading:
  - `domain_handle`: `230-239`
  - `create_domain`: `241-249`
  - `api`: `252-254`
  - `try_load_api`: `256-301`
  - `load_symbol`: `303-305`
  - `library_candidates`: `318-349`
- capture-name and color policy:
  - `should_mirror_default_range`: `215-228`
  - `default_color`: `355-420`
  - `fallback_default_range_name`: `422-430`
- file-local tests:
  - `433-469`

This file does not own:

- the call sites that decide where profiling scopes are placed:
  - `crates/app_core/src/double_ma.rs`
  - `crates/app_core/src/gpu_backtest_kernel.rs`
  - `crates/app_core/src/options_expression.rs`
  - `src-tauri/src/commands_optimize.rs`
  - `src-tauri/src/commands_options_expression.rs`
- Nsight orchestration scripts and capture policy:
  - `scripts/profile_gpu_backtest_nsys.ps1`
  - `scripts/profile_options_expression_nsys.ps1`

## Classification

- Public surface: yes
  - the module is public at `crates/app_core/src/lib.rs:48`
  - `RangeGuard` and `scope` are public at `crates/app_core/src/nvtx.rs:106` and `150`
- Hot path: no for business logic, but yes for profiling sensitivity
  - this file wraps performance-sensitive engine/kernel regions, so overhead and capture semantics matter
- CUDA/runtime-critical: runtime-adjacent and profiling-critical, but not functionally required for CPU/CUDA correctness
- Strategy-local: no
- Low-risk support code: not entirely
  - it is support code, but it influences profiler capture behavior, naming, and script expectations

## Direct Interactions

### Backend interaction map

- crate export surface:
  - `crates/app_core/src/lib.rs:48`
- Double-MA engine scopes:
  - `crates/app_core/src/double_ma.rs:4985-4986`
  - `5617`
  - `7671`
  - `12390`
  - `12453`
  - `12500`
  - `12527`
  - `12826`
  - `12878`
- GPU runtime/kernel scopes:
  - `crates/app_core/src/gpu_backtest_kernel.rs:11921`
  - `11930`
  - `11938`
  - `12229`
  - `13082`
  - `13518`
  - `14815-14842`
  - `15431-15555`
  - `15791`
- options-expression engine scopes:
  - `crates/app_core/src/options_expression.rs:2507`
  - `3097`
  - `3464`
  - `5227-5230`
  - `5266-5313`
- Tauri command boundary:
  - `src-tauri/src/commands_optimize.rs:96`
  - `123`
  - `279`
  - `src-tauri/src/commands_options_expression.rs:1431-1956`
- profiling runner and script contract:
  - env toggles in `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs:351-353`
  - Nsight bounded-capture scripting:
    - `scripts/profile_gpu_backtest_nsys.ps1:245-255`
    - `279-283`
    - `355`
    - `501-502`
    - `scripts/profile_options_expression_nsys.ps1:164-165`
    - `175`
    - `228-250`
    - `318-319`
- feature wiring only:
  - `crates/app_core/Cargo.toml:6-10`
  - `src-tauri/Cargo.toml:11-16`

### Downstream verification impact if changed

- changing range names or fallback-name policy affects:
  - Nsight scripts that capture on explicit NVTX names
  - any profiler workflow that filters on specific capture labels
- changing env-variable behavior affects:
  - `profile_gpu_dynamic_runner.rs`
  - the Nsight PowerShell scripts
- changing FFI lifetime/loading behavior affects:
  - every profiling-enabled backend route across app-core and Tauri

## Standards And Research Log

This packet was written after targeted web research focused on NVTX/Nsight capture semantics, Rust FFI/loading invariants, one-time initialization behavior, and public API documentation expectations.

### Sources consulted

1. NVIDIA Nsight Systems User Guide, NVTX Trace section
   - Source: <https://archive.docs.nvidia.com/nsight-systems/2022.5/UserGuide/index.html>
   - Why it was consulted:
     - this file exists solely to feed NVTX ranges into Nsight-compatible profiling workflows
   - Standard informed:
     - NVTX supports domains, push/pop ranges, event attributes, and RAII-style wrappers; range matching and domain filtering are real profiler-facing contracts

2. NVTX C API Reference
   - Source: <https://nvidia.github.io/NVTX/doxygen/modules.html>
   - Why it was consulted:
     - the file defines domain handles, event attributes, and push/pop FFI bindings
   - Standard informed:
     - domains and event attributes are first-class NVTX concepts, not optional trivia

3. `libloading::Library` documentation
   - Source: <https://docs.rs/libloading/latest/libloading/struct.Library.html>
   - Why it was consulted:
     - this file dynamically loads NVTX symbols and copies raw function pointers out of the library
   - Standard informed:
     - symbol typing is unsafe-contract-sensitive, and library lifetime/unload behavior is part of the safety model

4. `OnceLock` documentation
   - Source: <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
   - Why it was consulted:
     - this file caches the loaded NVTX API and domain handles in process-wide `OnceLock`s
   - Standard informed:
     - `OnceLock` is a write-once global cache, so inert or unreachable cached state is real maintenance debt

5. Rust API Guidelines Checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - `scope(domain: &str, name: &str)` is a public API over a profiling/runtime surface
   - Standard informed:
     - public interfaces should be intentional, not broader or more weakly typed than necessary

6. Rust API Guidelines documentation guidance
   - Source: <https://rust-lang.github.io/api-guidelines/documentation.html>
   - Why it was consulted:
     - the public API currently has no rustdoc explaining fallback or no-op behavior
   - Standard informed:
     - public fallible or environment-sensitive APIs should document errors/fallbacks and provide example usage

7. `std::ffi::c_void`
   - Source: <https://doc.rust-lang.org/std/ffi/enum.c_void.html>
   - Why it was consulted:
     - the file models opaque NVTX handles as raw `*mut c_void`
   - Standard informed:
     - opaque FFI handles deserve explicit invariants and, where practical, stronger type boundaries than anonymous raw pointers

## Findings

## Correctness And Safety

### 1. The domain-scoped and event-attribute path is effectively disabled in the active range implementation

- Evidence:
  - `range` hardcodes:
    - `domain_handle = None`
    - `can_use_domain_api = false`
    - `pushed_default_range = true`
    - `crates/app_core/src/nvtx.rs:166-168`
  - the richer domain/event path is therefore unreachable:
    - `194-201`
  - the fallback path uses plain `nvtxRangePushA` and discards `color`:
    - `190-192`
  - compile verification still shows the richer-path scaffolding as unused:
    - `ENGINE_DOMAIN_NAME` / `KERNEL_DOMAIN_NAME`: `11-12`
    - `NvtxApi.domain_create_a` / `range_push_ex`: `47-53`
    - `domain_handle`: `230`
    - `create_domain`: `241`
    - `ENGINE_DOMAIN` / `KERNEL_DOMAIN`: `351-353`
- Why it matters:
  - the file is written as though domain separation and richer attributes are supported, but the current runtime behavior is names-only fallback ranges
  - that gap matters to profiler users because Nsight domain filtering and richer event metadata never become active
- Smallest justified fix:
  - either enable the intended path deliberately
  - or simplify the file so it honestly models the supported fallback-only behavior
  - do not keep dead domain/event machinery without documenting why it is intentionally inactive
- Review judgment:
  - medium-high correctness / profiler-contract issue

### 2. Safety currently depends on intentionally leaking the loaded library after copying raw function pointers out of `libloading`

- Evidence:
  - `try_load_api` opens a `Library`:
    - `crates/app_core/src/nvtx.rs:257-260`
  - `load_symbol` copies symbol values out as raw function pointers:
    - `303-305`
  - the code then calls `std::mem::forget(lib)` before returning the copied pointers:
    - `278`
- Why it matters:
  - this is a deliberate process-lifetime leak used to keep the library loaded after raw pointers are detached from the `Library` lifetime
  - it may be an acceptable design, but it is a real unsafe/FFI invariant and should be explicit
- Smallest justified fix:
  - document the invariant directly next to the unsafe load path
  - alternatively, retain the `Library` itself in a static cache instead of leaking it implicitly
- Review judgment:
  - medium safety / FFI-lifetime issue

## API And Naming

### 1. The public API is stringly typed even though the file already has an internal domain enum

- Evidence:
  - internal enum:
    - `crates/app_core/src/nvtx.rs:25-28`
  - public function:
    - `scope(domain: &str, name: &str)`: `150-156`
  - unknown strings silently return a disabled guard:
    - `155`
- Why it matters:
  - typos in `"engine"` / `"kernel"` silently suppress instrumentation instead of surfacing misuse at compile time
  - that is weak for a public API whose main job is reliable profiler annotation
- Smallest justified fix:
  - keep `scope(&str, &str)` only if external callers truly require free-form strings
  - otherwise add a typed public domain enum or public `engine_scope` / `kernel_scope` wrappers and make string-based dispatch an adapter, not the primary API
- Review judgment:
  - medium public-surface / API-shape issue

## Error Handling

### 1. Multiple setup or usage failures silently degrade to a no-op guard with no diagnostic surface

- Evidence:
  - env flag disabled:
    - `crates/app_core/src/nvtx.rs:160-162`
  - library load failure:
    - `163-165`
    - `256-300`
  - interior-NUL range names:
    - `174-178`
    - `181-185`
  - invalid public domain string:
    - `150-156`
- Why it matters:
  - silent fallback may be desirable in production, but it makes instrumentation loss hard to diagnose during profiling setup
  - profiler-facing support code benefits from at least debug-level visibility or documented failure modes
- Smallest justified fix:
  - keep the no-op fallback if low-friction profiling is the product goal
  - but add explicit rustdoc and, optionally, debug-only logging for disabled paths
- Review judgment:
  - medium error-handling / observability issue

### 2. `Drop` ignores NVTX pop return codes

- Evidence:
  - `domain_range_pop(domain)` return value is ignored:
    - `crates/app_core/src/nvtx.rs:127-132`
  - `range_pop()` return value is ignored:
    - `134-137`
- Why it matters:
  - this is probably fine for lightweight profiling helpers
  - but it removes any chance to surface imbalance or API-failure diagnostics when instrumentation is enabled
- Smallest justified fix:
  - if keeping the current no-throw RAII style, document that pop errors are intentionally ignored
  - do not turn this into hard failure on drop without a clear profiling-policy decision
- Review judgment:
  - low-medium error-handling issue

## Module Boundaries And Technical Debt

### 1. The file contains substantial dormant scaffolding that no longer matches the active behavior

- Evidence:
  - domain creation helpers and caches exist:
    - `crates/app_core/src/nvtx.rs:230-249`
    - `351-353`
  - event-attribute support exists:
    - `56-103`
  - richer symbols are loaded:
    - `271-297`
  - active range path still forces the fallback-only route:
    - `166-168`
  - compile verification with `--features nvtx-profiling` still reports the domain-oriented pieces as unused
- Why it matters:
  - the module is harder to maintain because a large slice of it looks supported but is operationally inert
  - future reviewers have to reverse-engineer which path is real
- Smallest justified fix:
  - document the intended target state if this is temporary
  - otherwise prune the dead path or re-enable it deliberately in a later profiling-focused pass
- Review judgment:
  - medium technical-debt issue

### 2. The `nvtx-profiling` feature is currently wiring-only and appears compile-time inert

- Evidence:
  - feature declarations exist:
    - `crates/app_core/Cargo.toml:6-10`
    - `src-tauri/Cargo.toml:11-16`
  - repo search only finds `nvtx-profiling` in those Cargo manifests:
    - no code-level `cfg(feature = "nvtx-profiling")` usage was found under `crates/` or `src-tauri/`
- Why it matters:
  - contributors may reasonably assume the feature materially changes compilation or instrumentation behavior
  - in current code, the real control surface is the runtime env var `VECTORBT_ENABLE_NVTX`
- Smallest justified fix:
  - either document that `nvtx-profiling` is only a workflow marker today
  - or make the feature actually gate the NVTX module/call sites if that was the intent
- Review judgment:
  - medium feature-contract / maintenance issue

## Docs And Examples

### 1. The public profiling contract is undocumented

- Evidence:
  - public items:
    - `RangeGuard`: `crates/app_core/src/nvtx.rs:106`
    - `scope`: `150`
  - there is no rustdoc explaining:
    - the `VECTORBT_ENABLE_NVTX` env contract
    - silent no-op behavior
    - library search order from `library_candidates`: `318-349`
    - name-preservation policy from `should_mirror_default_range`: `215-228`
    - why the file leaks the library handle to preserve raw symbol validity
    - why domains/colors are present in the code but inactive in the current runtime path
- Smallest justified fix:
  - add rustdoc for `scope` and `RangeGuard`
  - add a short module-level note describing the env var, supported behavior, and profiler-script coupling

## Tests And Missing Verification

### 1. Current tests only pin fallback naming, not behavior that actually determines whether profiling works

- Evidence:
  - file-local tests only cover fallback naming:
    - `crates/app_core/src/nvtx.rs:433-469`
  - local verification run:
    - `cargo test -p ta_app_core nvtx::tests:: -- --nocapture`
      - passed `2` tests
  - feature-specific compile verification runs:
    - `cargo check -p ta_app_core --features nvtx-profiling`
      - passed
    - `cargo check -p ta_desktop_demo_app --features nvtx-profiling`
      - passed
- Missing tests I would want before changing this file:
  - env-flag parsing:
    - `"0"`, `"false"`, unset, and enabled values
  - unknown-domain behavior:
    - confirm whether silent disable is intentional
  - fallback-name policy:
    - pin the exact top-level names expected by profiler scripts
  - feature wiring:
    - at least one compile-contract test or doc assertion clarifying what `nvtx-profiling` currently means

## Performance, Allocation, And CUDA Concerns

### 1. This file is benchmark-sensitive even though it is “only profiling code”

- Evidence:
  - the file wraps engine/kernel sections in:
    - `double_ma.rs`
    - `gpu_backtest_kernel.rs`
    - `options_expression.rs`
  - Nsight guidance explicitly notes that NVTX ranges should be used carefully around very small code regions
  - the profiling scripts depend on these ranges for bounded capture:
    - `scripts/profile_gpu_backtest_nsys.ps1:279-283`
    - `scripts/profile_options_expression_nsys.ps1:247-250`
- Review judgment:
  - do not change range creation strategy, naming, or enablement rules without profiler verification

### 2. Re-enabling `nvtxRangePushEx` or domain APIs would be a semantics-sensitive profiling change, not a pure cleanup

- Evidence:
  - the code loads `range_push_ex` and domain symbols:
    - `crates/app_core/src/nvtx.rs:264-297`
  - the active path currently uses `range_push_a` only:
    - `190-192`
  - the scripts explicitly compensate for current plain-string push/pop behavior by disabling “registered-only” capture:
    - `scripts/profile_gpu_backtest_nsys.ps1:250-253`
    - `scripts/profile_options_expression_nsys.ps1:228-232`
- Review judgment:
  - richer NVTX semantics are potentially desirable, but they are not safe-now cleanup
  - treat them as profiling-contract changes requiring script verification

## Code-Quality Improvement Register

- Safe now:
  - add rustdoc for `scope`, `RangeGuard`, and the env/search-order contract
  - document the intentional library-leak invariant if it remains
  - clarify whether `nvtx-profiling` is documentation-only or should become a real compile-time gate
- Coordination required:
  - changing range names or fallback-name policy
  - switching from plain push/pop to `nvtxRangePushEx` or domain APIs
  - tightening public API typing if external callers currently pass string domains
- Defer until later pass:
  - broader redesign of FFI handle typing
  - re-enabling or removing domain support unless profiler workflows are updated alongside it

## Technical-Debt Reduction Register

- Safe to reduce now:
  - missing docs
  - feature-contract ambiguity
  - explicit comments around the leaked-library safety invariant
- Coordination required:
  - dormant domain/event scaffolding
  - public API typing
  - profiler-script range-name coupling
- Useful debt that should not be “cleaned up” casually:
  - the RAII guard pattern itself
  - silent no-op fallback, if low-friction runtime behavior is an intentional product choice

## Performance Opportunity Register

- Safe structural improvements:
  - none urgent beyond documentation and explicit contract cleanup
- Benchmark-sensitive candidates:
  - enabling colored `nvtxRangePushEx`
  - enabling domain APIs
  - changing capture names or nesting policy
- Not the right place to chase performance without evidence:
  - any rewrite that alters profiler observability before verifying the scripts still work

## Observability And Diagnostics Gaps

- there is no direct signal when `scope` silently becomes a no-op
- the file does not state whether it is fallback-only by design or accidentally stuck in that mode
- feature wiring suggests compile-time semantics that the code does not currently implement

## Implementation Staging Guidance

- Safe now:
  - docs
  - additive tests
  - explicit comments around FFI invariants
  - feature-contract clarification
- Coordination required:
  - range-name changes
  - typed public API changes
  - any activation of domain/color support
- Defer until later pass:
  - larger profiling-surface redesign

## Change Blast Radius

- Local only:
  - docs
  - additive tests
  - comments clarifying safety invariants
- Same subsystem:
  - env-flag or fallback policy changes affect profiling runners and scripts
- Cross-backend / frontend-backend contract:
  - public API changes affect Tauri command code and app-core call sites
  - capture-name changes affect profiling automation and Nsight workflows

## Open Questions Or Assumptions

- assumption:
  - the current fallback-only runtime path is intentional or at least tolerated because scripts already compensate for plain-string NVTX ranges
- open question:
  - whether domain/color support was intentionally disabled for profiler compatibility or simply left incomplete
- open question:
  - whether external callers beyond this workspace depend on `scope(&str, &str)` as a public string-based API

## Recommended Action

`Small surgical refactor`

The right next pass is:

- document the true runtime behavior
- clarify the feature and env contracts
- add targeted tests around enablement and naming
- defer any actual activation of the dormant richer NVTX path until the profiling scripts are verified against it

## Exact Local Verification Required If Changed

- `cargo test -p ta_app_core nvtx::tests:: -- --nocapture`
- `cargo check -p ta_app_core --features nvtx-profiling`
- `cargo check -p ta_desktop_demo_app --features nvtx-profiling`
- if capture names or scope-policy change:
  - `powershell -File scripts/profile_gpu_backtest_nsys.ps1 -NvtxMode on -CaptureMode nvtx -NvtxCapture double_ma_opt_total`
  - `powershell -File scripts/profile_options_expression_nsys.ps1 -CaptureMode nvtx`

## Regression Risk And Feature-Preservation Constraints

- do not silently rename ranges that profiling scripts currently target
- do not activate domain/color support casually; it changes profiler-visible semantics even if business logic is unaffected
- do not remove the no-op fallback path unless the repo explicitly decides profiling setup failures should become hard failures
