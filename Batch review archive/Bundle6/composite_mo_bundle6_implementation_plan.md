# CompositeMo Search Efficiency and GPU Prescan — Comprehensive Optimization & Implementation Plan

## Review framing (Bundle 6 from the repo review map)

**Surface under review**

`CompositeMo` is a CPU-side multi-objective optimizer that optionally runs a stage-0 Double-MA prescan to shrink the MA-length search space before executing `Random`, `Qmc`, `Tpe`, or `Nsga2`. In the current implementation, the prescan is the only GPU-adjacent part of the workflow; the main multi-objective evaluation path remains CPU-bound.

**Files reviewed**

- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/lib.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`
- `crates/app_core/benches/dynamic_strategy_cpu.rs`

**Extra context applied from the repo review map**

This plan follows the Bundle 6 scope exactly:

- search-space efficiency review
- sampler-overhead review
- GPU-prescan payoff review
- request-shape and default-value review

It also follows the prompt rules from the review map:

- optimization-first, benchmark-first, parity-preserving
- no broad product expansion
- prefer small safe wins before large redesigns
- explicitly separate confirmed issues, likely issues, open questions, and higher-risk opportunities
- attach a timing boundary to each material finding

**Timing boundary legend used below**

- **app-path**: end-to-end runtime of `run_composite_mo_optimization_blocking_with_candles(...)`
- **Criterion**: current benchmark coverage in `dynamic_strategy_cpu.rs`
- **profiling-only**: would need Nsight or deeper tracing to prove payoff
- **unknown**: likely real, but current files do not contain enough measurement evidence

---

## 1. Surface understanding

### 1.1 Current execution pipeline

Today the `CompositeMo` path works like this:

1. **Frontend request shaping**
   - `optimizeRequestBuilder.ts` builds a `CompositeMo` request, normalizes the search space, injects `fast_len` and `slow_len` domains, picks one sampler, and optionally emits a `gpu_prescan` block.
   - `strategyOptimizationCapabilities.ts` describes `CompositeMo` as managing its own sampler and optional GPU prescan.

2. **Backend prep**
   - `composite_mo.rs` validates the request, normalizes the base strategy, resolves MA length domains, and optionally runs a stage-0 prescan.
   - If prescan is enabled and `fast_len`/`slow_len` are `IntRange`, it runs one or more Double-MA optimization requests and uses the results to rewrite the MA-length domains to categorical candidate lists.

3. **MA bank precompute**
   - `composite_mo.rs` precomputes MA matrices for all allowed fast and slow periods, grouped by MA type.

4. **Main multi-objective search**
   - The inner evaluator decodes params, checks constraints, computes or reuses indicator bundles, then computes composite metrics on the CPU.
   - The evaluator is fed into one of four samplers implemented in `crates/optimizer/src/mo.rs`:
     - `random_mo_search`
     - `qmc_mo_search`
     - `tpe_mo_search`
     - `nsga2_optimize`

5. **Result shaping**
   - `CompositeMoResult` currently returns only:
     - `data_id`
     - `pareto`
     - `total_evaluations`
     - `runtime_ms`

### 1.2 Important current architecture reality

The current codebase does **not** execute the main `CompositeMo` inner loop on the GPU. The GPU contribution is limited to the optional prescan, and even that prescan is routed through repeated Double-MA optimization calls instead of a dedicated CompositeMo GPU path.

That means:

- any CUDA speedup ceiling is limited by how much the prescan can prune downstream CPU work
- if prescan setup/upload/routing waste is high, it can erase its own benefit
- the biggest practical wins right now come from:
  1. making prescan actually predictable and measurable,
  2. reducing post-prescan wasted CPU work,
  3. making the samplers stop re-evaluating structurally invalid or duplicate points

### 1.3 Current benchmark coverage

The provided benchmark file (`dynamic_strategy_cpu.rs`) only benchmarks a small 2k-candle `CompositeMo` full run with:

- `Random` sampler
- `trials = 40`
- `gpu_prescan = None`

That benchmark is useful for dynamic-strategy overhead parity, but it does **not** measure:

- prescan on/off payoff
- large-range MA search spaces
- sampler duplication rates
- sparse-bank build cost after prescan
- NSGA-II end-of-run frontier extraction cost
- any CUDA/prescan warm-path behavior

---

## 2. Findings

## 2.1 Confirmed issues

### F1. Prescan device selection is effectively ignored by the backend and hardcoded by the frontend

**Status:** confirmed issue  
**Severity:** high  
**Type:** correctness / performance  
**Location:**  
- `crates/app_core/src/composite_mo.rs` — `run_composite_mo_optimization_blocking_with_candles(...)`
- `web/src/services/optimizeRequestBuilder.ts` — `buildCompositeMoOptimizationRequest(...)`  
**Timing boundary affected:** app-path  

**Why it matters**

`CompositeGpuPrescanConfig` exposes `device_id: Option<u32>`, but the backend ignores `cfg.device_id` and always resolves a preferred CUDA device before building `Backend::Auto { device_id }`. At the same time, the frontend always emits `device_id: 0` when prescan is enabled.

That creates two separate problems:

1. the backend cannot actually honor a caller-selected GPU
2. the frontend forces a device id even when the user has not explicitly requested one

On multi-GPU machines, this can make prescan benchmarking noisy, can break repeatability, and can silently route work to the wrong GPU.

**Recommended change**

- In the backend, honor `cfg.device_id` first, then fall back to preferred-device resolution, then fall back to `0`.
- In the frontend, stop emitting `device_id: 0` unconditionally. Only send a `device_id` when the user explicitly sets one.
- Add telemetry showing the requested and resolved device ids.

---

### F2. The “GPU prescan” is launched through `Auto + BestEffort`, which weakens GPU residency and makes payoff harder to measure

**Status:** confirmed issue  
**Severity:** high  
**Type:** performance / benchmark-fidelity  
**Location:** `crates/app_core/src/composite_mo.rs` — construction of the prescan `DoubleMaRequest`  
**Timing boundary affected:** app-path  

**Why it matters**

The prescan loop builds a `DoubleMaRequest` with:

- `backend: Backend::Auto { device_id }`
- `gpu_execution: BestEffort`
- `persist_followup_cache: false`

That combination is conservative, but it also means:

- prescan can silently land on CPU even when a GPU was intended
- it does not express a “keep resident if possible” preference
- each prescan call is an isolated request with no follow-up reuse
- benchmark results can look like “GPU prescan is not worth it” when the real issue is weak routing or no reuse

**Recommended change**

- When prescan is explicitly requested, route it with a stronger GPU intent:
  - keep `Backend::Auto` if you must preserve fallback,
  - but use a stricter GPU-execution policy than `BestEffort` when the request is clearly prescan-oriented
- Record `backend_used` for every prescan sub-request and summarize it in the final result
- Add a small prescan summary to the result so the caller can see whether prescan actually used GPU

---

### F3. Prescan narrowing is only length-aware, not MA-type-aware

**Status:** confirmed issue  
**Severity:** high  
**Type:** correctness / performance  
**Location:** `crates/app_core/src/composite_mo.rs` — prescan candidate-set construction and `allowed_pairs`  
**Timing boundary affected:** app-path  

**Why it matters**

Prescan currently collects:

- `fast_set`
- `slow_set`
- `pair_set: HashSet<(u16, u16)>`

It does **not** retain the MA type pair that produced a candidate pair.

That matters whenever `fast_ma_type` and/or `slow_ma_type` are themselves part of the search space. In that case:

- the search can evaluate `(ema, ema, 8, 21)` using a candidate pair that was only discovered while scanning `(sma, sma)` or some other pair
- a prescan that looked narrow and efficient is actually leaking candidates across incompatible MA type pairs
- the algorithm can under-cover good regions for non-base MA type pairs, or over-admit invalid pair/type combinations

**Recommended change**

Replace `allowed_pairs: HashSet<(u16, u16)>` with a type-aware structure, for example:

- `HashMap<(String, String), Vec<(u16, u16)>>`
- or `HashSet<PrescannedConfigKey { fast_ma_type, slow_ma_type, fast_len, slow_len }>`.

Then make both:
- `eval_one(...)`
- `repair(...)`

validate and repair against the pair list for the **currently decoded MA type pair**, not a global union of length pairs.

---

### F4. Repair only enforces `fast_len < slow_len`; it does not snap to a prescanned valid pair

**Status:** confirmed issue  
**Severity:** high  
**Type:** performance  
**Location:** `crates/app_core/src/composite_mo.rs` — `repair` closure  
**Timing boundary affected:** app-path  

**Why it matters**

After prescan, the backend builds `allowed_pairs`, but `repair(...)` only ensures the ordering constraint `fast_len < slow_len`.

That means the samplers can still generate:

- legal orderings that are **not** in the prescanned candidate graph

Those points survive repair, enter the evaluator, and then get rejected by the `allowed_pairs` check inside `eval_one(...)`.

That is pure wasted work.

**Recommended change**

Make repair prescan-aware:

- decode the active MA type pair
- look up the allowed pair list for that pair
- snap `(fast_len, slow_len)` to the nearest valid prescanned pair
- only fall back to generic ordering repair when prescan is disabled

This one change removes a large class of avoidable invalid evaluations.

---

### F5. Sparse/non-uniform post-prescan candidate sets force row-by-row MA bank construction

**Status:** confirmed issue  
**Severity:** high  
**Type:** performance  
**Location:** `crates/app_core/src/composite_mo.rs` — `build_ma_bank`  
**Timing boundary affected:** app-path  

**Why it matters**

The code already optimizes the “uniform period array” case by calling `ma_matrix_cpu_f64(...)` once for the whole set.

But when the candidate periods are not a single arithmetic run, it falls back to:

- one `ma_matrix_cpu_f64(...)` call **per period**
- then one row copy into the final bank

Prescan often creates exactly these non-uniform categorical period sets.

So the current flow can become:

- **GPU/Double-MA prescan saves search work**
- but **CPU sparse-bank rebuilding gives a large part of that time back**

**Recommended change**

Batch non-uniform periods by arithmetic runs instead of by single rows:

- detect contiguous same-step runs
- call `ma_matrix_cpu_f64(...)` once per run
- scatter each run’s rows into the final matrix

This should preserve the current MA bank layout while dramatically reducing call count for common prescan-generated candidate shapes.

---

### F6. `Random`, `Qmc`, and `Nsga2` do not suppress duplicates, while `Tpe` does

**Status:** confirmed issue  
**Severity:** high  
**Type:** performance  
**Location:** `crates/optimizer/src/mo.rs` —  
- `random_mo_search(...)`
- `qmc_mo_search(...)`
- `nsga2_optimize(...)`
- contrast: `tpe_mo_search(...)` uses `seen`  
**Timing boundary affected:** app-path  

**Why it matters**

The optimizer crate already has a value-hash helper:

- `values_hash(...)`

and `tpe_mo_search(...)` already uses a `HashSet<u64>` to reduce duplicate suggestions.

But:

- `random_mo_search(...)` does not deduplicate
- `qmc_mo_search(...)` does not deduplicate
- `nsga2_optimize(...)` does not deduplicate initial population or offspring

That is especially costly after prescan because the effective search space becomes:

- more discrete
- more categorical
- more repair-projected
- more likely to collapse distinct raw samples into the same repaired point

**Recommended change**

Unify duplicate suppression across all samplers:

- expose `values_hash(...)` as a reusable helper
- dedup initial and offspring populations in `nsga2_optimize(...)`
- dedup generated samples in `random_mo_search(...)` and `qmc_mo_search(...)`
- keep a `unique_evaluations` count in the final telemetry

---

### F7. NSGA-II uses a quadratic final Pareto extraction even though a faster constrained 2D frontier helper already exists

**Status:** confirmed issue  
**Severity:** medium-high  
**Type:** performance  
**Location:** `crates/optimizer/src/mo.rs` — `nsga2_optimize(...)` final Pareto extraction  
**Timing boundary affected:** app-path  

**Why it matters**

At the end of `nsga2_optimize(...)`, the code does:

- `non_dominated_sort(&all_seen)`

over **all evaluated points**, then returns the first front.

But the same file already has a faster helper:

- `pareto_front_constrained(...)`
- backed by `pareto_front_constrained_indices(...)`

for the exact 2-objective constrained case used here.

So NSGA-II is paying an avoidable O(n²) tail cost.

**Recommended change**

At minimum:

- dedup `all_seen` by `values_hash(...)`
- replace the final `non_dominated_sort(...)` call with `pareto_front_constrained(&all_seen)`

Longer term:

- maintain an incremental frontier instead of keeping every point for the final extraction

---

### F8. `CompositeMo` does not auto-switch to exact grid even though the optimizer crate already supports grid enumeration

**Status:** confirmed issue  
**Severity:** medium-high  
**Type:** performance / maintainability  
**Location:**  
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs` (`SearchSpace::grid_iter(...)`, `SearchSpace::grid(...)`)  
**Timing boundary affected:** app-path  

**Why it matters**

The optimizer crate already supports enumerating a fully discrete Cartesian product via:

- `SearchSpace::grid_iter(...)`
- `SearchSpace::grid(...)`

But `CompositeMo` never uses that capability, even when prescan has transformed the key dimensions into small categorical sets and the remaining search space is fully discrete.

In those cases, continuing to use stochastic sampling can be strictly worse than exact enumeration because it introduces:

- duplicate waste
- incomplete coverage
- extra sampler overhead
- less stable regression evidence

**Recommended change**

Add a small exact-grid fast path for the fully discrete/small-space case:

- if `search_space.grid(...)` succeeds under conservative thresholds, run an exact frontier enumeration instead of a stochastic sampler
- do this automatically inside `CompositeMo`; do not add a new user-facing knob yet

---

### F9. Result telemetry is too thin to judge whether prescan or dedup changes helped

**Status:** confirmed issue  
**Severity:** medium-high  
**Type:** benchmark-fidelity / maintainability  
**Location:** `crates/app_core/src/composite_mo.rs` — `CompositeMoResult` and artifact emission  
**Timing boundary affected:** app-path / Criterion  

**Why it matters**

The final result only exposes:

- `total_evaluations`
- `runtime_ms`

It does **not** expose:

- whether prescan ran
- which backend prescan actually used
- how many candidate periods/pairs prescan produced
- how many evaluations were unique
- how many were rejected by structural constraints
- whether the final path should be described as CPU-only or CPU + GPU-prescan

That makes optimization work much harder to validate.

There is also an artifact emission path that hardcodes the backend string to `"Cpu"`, even if the request performed a GPU-assisted prescan.

**Recommended change**

Extend `CompositeMoResult` with a telemetry block, for example:

- `prescan: Option<CompositeMoPrescanStats>`
- `search: CompositeMoSearchStats`
- `runtime_backend: String`

Also fix artifact emission so the recorded backend reflects actual execution shape:
- `"Cpu"`
- `"Cpu+GpuPrescan"`
- `"Cpu+MixedPrescan"`

---

### F10. Current benchmark coverage does not measure the hotspots you actually need to optimize

**Status:** confirmed issue  
**Severity:** medium-high  
**Type:** benchmark-fidelity  
**Location:** `crates/app_core/benches/dynamic_strategy_cpu.rs`  
**Timing boundary affected:** Criterion  

**Why it matters**

The current `CompositeMo` benchmark in the provided files only covers:

- 2k candles
- `Random`
- 40 trials
- no prescan
- dynamic strategy env off/on

That benchmark does not measure:

- prescan benefit
- post-prescan bank build cost
- sampler-specific overhead
- duplicate collapse
- NSGA-II tail cost

**Recommended change**

Add dedicated benchmark groups for:

- prescan off vs on
- sparse-bank candidate sets
- sampler overhead in discrete post-prescan-like spaces
- NSGA-II frontier tail

The file-by-file benchmark plan later in this document includes concrete benchmark skeletons.

---

## 2.2 Likely issues

### L1. `Mutex`-guarded caches may serialize a large portion of the Rayon evaluation path

**Status:** likely issue  
**Severity:** medium  
**Type:** performance  
**Location:** `crates/app_core/src/composite_mo.rs` — `indicator_cache` and `dynamic_plan_cache`  
**Timing boundary affected:** app-path / profiling-only  

**Why it matters**

Both caches are wrapped in `Arc<Mutex<_>>`, and they sit directly on the hot path of the per-point evaluator.

Under a parallel sampler evaluation, that means:

- every cache hit and miss takes a global lock
- repeated indicator reuse can become lock-contention-limited
- dynamic-plan reuse can become lock-contention-limited

This is plausible, but the current provided benchmark does not isolate it.

**Recommended change**

Do **not** change this first.

After the safe fixes land, profile:

- if contention is real, switch to one of:
  - sharded caches
  - thread-local caches
  - `RwLock` plus double-checked insert
  - lazy per-type or per-thread bank/plan caches

---

### L2. Eager MA bank build for every MA type can overpay startup cost

**Status:** likely issue  
**Severity:** medium  
**Type:** performance  
**Location:** `crates/app_core/src/composite_mo.rs` — eager `fast_banks` / `slow_banks` construction  
**Timing boundary affected:** app-path  

**Why it matters**

`CompositeMo` eagerly builds MA banks for all allowed fast and slow MA types before any sampling begins.

That is the safest design, but in a search where:

- MA types are optimized
- only a subset of types are actually sampled meaningfully
- prescan narrowed lengths aggressively

it may overpay a large upfront cost.

**Recommended change**

Keep eager build for the first round of fixes. If startup remains dominant after the sparse-run bank change, consider lazy bank build keyed by MA type.

---

### L3. Frontend defaults are not shape-aware

**Status:** likely issue  
**Severity:** medium  
**Type:** performance / request-shape  
**Location:** `web/src/services/optimizeRequestBuilder.ts`  
**Timing boundary affected:** app-path / Criterion  

**Why it matters**

The current frontend defaults are static:

- `Random`: 2000
- `Tpe`: 5000
- `Qmc`: 5000
- `Nsga2`: 80 × 40 (~3280 evals)
- prescan defaults: `top_k = 128`, `window = 8`, `predictive_top_k = 12`, `predictive_stride = 8`

Those defaults may be reasonable as broad defaults, but they are not adjusted based on:

- small fully discrete spaces
- post-prescan-like candidate counts
- optimized MA type count
- exact-grid eligibility

**Recommended change**

Make defaults shape-aware **after** backend telemetry and benchmark coverage exist. Do not blindly retune constants first.

---

## 2.3 Open questions

### O1. How much of the total app-path time is currently spent in stage-0 prescan vs sparse bank building vs inner-loop evaluation?

**Status:** open question  
**Severity:** medium  
**Type:** benchmark-fidelity  
**Location:** multiple `CompositeMo` stages  
**Timing boundary affected:** app-path / profiling-only  

**Why it matters**

You need to know the time split before deciding whether to prioritize:

- better prescan routing
- sparse-bank build optimization
- inner-loop CPU evaluator work
- sampler dedup
- a future GPU batch evaluator

**Recommended change**

Add timing spans around:

- prescan
- MA bank build
- sampler/evaluator total
- final Pareto shaping

and include them in telemetry.

---

### O2. After the safe changes, does the remaining ceiling justify a real GPU inner-loop evaluator?

**Status:** open question  
**Severity:** medium  
**Type:** performance / architecture  
**Location:** overall `CompositeMo` design  
**Timing boundary affected:** app-path / profiling-only  

**Why it matters**

A true CUDA win for `CompositeMo` probably requires GPU execution of the main evaluator, not just prescan. But that is a much larger change and should only happen after the cheap waste is removed.

**Recommended change**

Do not start there. Re-measure after:
- device fix
- pair-aware repair
- sparse bank build
- dedup
- exact-grid fast path
- new benchmarks

---

## 3. Safe optimization opportunities

The following changes are the best first wave because they are local, parity-safe, and directly supported by the current code shape.

### 3.1 Fix routing and add telemetry first

Implement these before anything else:

1. honor `gpu_prescan.device_id`
2. stop hardcoding `device_id: 0` in the frontend
3. record prescan backend usage and runtime
4. record candidate counts and unique evaluation counts

This unlocks trustworthy measurement.

---

### 3.2 Make prescan constraints shape the search, not just the evaluator

The current search still samples many points that the evaluator later rejects.

Fix that by:

1. making prescan type-aware
2. making repair snap to valid prescanned pairs
3. optionally auto-generating `ma_pairs` from optimized MA-type domains in the frontend when feasible

This reduces wasted evaluations without changing scoring logic.

---

### 3.3 Reduce post-prescan bank rebuild tax

The sparse-bank issue is likely the largest backend-local CPU waste after prescan.

Fix it by:

1. batching arithmetic runs
2. keeping the same output matrix layout
3. adding a benchmark that isolates sparse vs uniform candidate shapes

---

### 3.4 Dedup sampler outputs consistently

Unify sampler behavior so all samplers avoid needless repeats in discrete spaces.

Prioritize:

1. `Random`
2. `Qmc`
3. `Nsga2`

`Tpe` already has a `seen` set and should remain the reference behavior.

---

### 3.5 Add an exact-grid fast path for small fully discrete spaces

This is low risk because the optimizer crate already has the grid machinery.

Use it only when:

- the space is fully discrete
- grid cardinality is below a conservative cap
- exact enumeration is clearly cheaper than stochastic retry + dedup

---

## 4. Higher-risk optimization opportunities

These are worthwhile, but they should come **after** the safe wins.

### 4.1 GPU-resident prescan reuse

Today prescan is a loop of isolated Double-MA requests.

A better second step is to reuse:

- uploaded candles / MA source state
- resolved GPU route metadata
- possibly kernel-resident intermediate state

This is higher risk because it depends on adjacent Double-MA runtime behavior not fully shown in the provided files.

---

### 4.2 Lazy MA bank construction

Only build the bank for MA types that are actually touched.

This can reduce startup, but it adds complexity to the evaluator and can introduce synchronization overhead if built on-demand under parallel evaluation.

---

### 4.3 GPU batch evaluator for the CompositeMo inner loop

This is the long-term CUDA path if you want large gains.

A real GPU path would need to batch:
- fast/slow MA lookup
- indicator bundle evaluation
- strategy metric calculation
- multi-objective scoring

This is a valid long-term direction, but it is not the right first patch set.

---

## 5. Benchmark and perf-gate review

### 5.1 What the current benchmark tells you

The benchmark in `dynamic_strategy_cpu.rs` is useful for only one question:

- “Does enabling the dynamic-strategy experimental path materially change small-run CompositeMo CPU runtime?”

That is worth keeping.

### 5.2 What it does not tell you

It does **not** answer any of these higher-value optimization questions:

- Is prescan a net win?
- Is sparse bank building a bottleneck?
- How many points are duplicates or invalid after repair?
- Which sampler gives the best unique exploration per second?
- How expensive is NSGA-II’s final frontier extraction?

### 5.3 Benchmark additions you should make

Add four benchmark groups:

#### B1. Prescan payoff benchmark

Measure:
- prescan off
- prescan objective-only
- prescan objective + predictive rescue

with a length range that is large **but still valid without prescan**, so you can do an apples-to-apples comparison.

Suggested shape:
- fast range: `5..200`
- slow range: `30..250`
- sampler: `Random { trials: 256 }`
- same candles and strategy in all cases

Record:
- total runtime
- telemetry-prescan runtime
- candidate counts
- unique evaluations
- final Pareto size

#### B2. Sparse-bank benchmark

Construct a prescan-like categorical candidate set with:
- one fully uniform period run
- one sparse multi-run period set

Use the same total candidate count and compare MA bank build time.

#### B3. Sampler duplicate benchmark

Use a fully discrete, small post-prescan-like space and compare:
- `Random`
- `Qmc`
- `Tpe`
- `Nsga2`

Record:
- total evaluations
- unique evaluations
- feasible evaluations
- runtime
- Pareto size

#### B4. NSGA-II tail benchmark

Isolate the final frontier extraction by running larger NSGA-II budgets and measuring:
- total runtime
- final frontier extraction time
- point count in `all_seen`
- point count after dedup

---

## 6. Test review

### 6.1 Existing test strengths

The current `composite_mo.rs` tests already cover:

- smoke runs for all samplers
- dynamic-strategy env parity
- shadow-mode parity
- unsorted categorical length domains
- MA type optimization
- prescan smoke tests
- prescan with multiple MA pairs

That is a strong correctness floor.

### 6.2 Missing tests that should be added

#### T1. Prescan device-id respect test
Verify that:
- if `gpu_prescan.device_id = Some(x)`, the resolved device id in telemetry is `x`

#### T2. Prescan telemetry test
Verify that:
- prescan runtime is populated
- candidate fast/slow/pair counts are populated
- `backend_used` is populated

#### T3. Type-aware prescan validity test
If MA types are optimized and prescan uses multiple MA pairs, verify that:
- evaluation only accepts the valid pair list for the active MA type pair
- repair snaps to a type-valid pair

#### T4. Pair-aware repair test
Given a repaired point outside `allowed_pairs`, verify that:
- `repair(...)` returns a valid pair instead of just any ordered pair

#### T5. Cross-sampler dedup test
For a tiny discrete space, verify that:
- `Random`
- `Qmc`
- `Nsga2`

do not emit duplicate candidate hashes beyond a tolerated fallback threshold.

#### T6. Exact-grid fast path test
For a fully discrete small space, verify that:
- the exact-grid path runs
- returned Pareto front matches the stochastic path’s optimal set on the same tiny space

#### T7. Sparse-run MA bank equivalence test
Verify that run-batched sparse bank construction matches the current row-by-row output exactly.

---

## 7. Suggested improvement plan

## Phase 1 — observability and correctness

**Goal:** make prescan measurable and deterministic enough to trust benchmarks.

**Changes**
- honor `gpu_prescan.device_id`
- stop hardcoding frontend `device_id: 0`
- add prescan/search telemetry to `CompositeMoResult`
- fix backend artifact label from always `"Cpu"` to actual execution shape

**Why first**
Without this phase, you cannot trust later runtime comparisons.

---

## Phase 2 — eliminate structurally wasted evaluations

**Goal:** stop evaluating points that the search already knows are bad.

**Changes**
- replace `allowed_pairs` with type-aware allowed configs
- make repair snap to valid prescanned pairs
- add cross-sampler dedup
- add unique/feasible/invalid counters

**Why second**
This usually gives immediate speedup without changing actual metric computation.

---

## Phase 3 — reduce post-prescan CPU rebuild tax

**Goal:** ensure prescan savings are not erased by bank rebuild overhead.

**Changes**
- arithmetic-run batching in `build_ma_bank`
- sparse-bank benchmark
- optional lazy bank follow-up only if still needed

**Why third**
This directly improves the most obvious prescan-created CPU hotspot.

---

## Phase 4 — exact-grid fast path and adaptive defaults

**Goal:** exploit already-available exact enumeration in cases where it beats stochastic sampling.

**Changes**
- add exact-grid helper in `mo.rs`
- auto-switch `CompositeMo` when the discrete space is small enough
- make frontend sampler defaults aware of estimated search-space shape

**Why fourth**
It is a clean improvement, but it depends on earlier telemetry and search-shape fixes.

---

## Phase 5 — larger CUDA work only after re-measurement

**Goal:** decide whether remaining runtime justifies deeper GPU work.

**Changes**
- GPU-resident prescan reuse
- maybe lazy GPU-prepared state
- only then consider a full GPU inner-loop evaluator

**Why fifth**
Do not start expensive GPU redesign before removing the easy CPU waste.

---

## 8. Detailed file-by-file code edit plan and snippets

## 8.1 `crates/app_core/src/composite_mo.rs`

### 8.1.1 Add telemetry/result structs

Add small telemetry types to carry the information you need for benchmarks and regression analysis.

```rust
#[derive(Clone, Debug, Serialize, Default)]
pub struct CompositeMoSearchStats {
    pub total_evaluations: usize,
    pub unique_evaluations: usize,
    pub feasible_evaluations: usize,
    pub rejected_decode_or_order: usize,
    pub rejected_prescan_pair: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoPrescanStats {
    pub requested_device_id: Option<u32>,
    pub resolved_device_id: Option<u32>,
    pub backend_used: String,
    pub runtime_ms: u64,
    pub scanned_objectives: usize,
    pub scanned_ma_pairs: usize,
    pub candidate_fast_periods: usize,
    pub candidate_slow_periods: usize,
    pub candidate_pairs: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoTelemetry {
    pub runtime_backend: String,
    pub prescan: Option<CompositeMoPrescanStats>,
    pub search: CompositeMoSearchStats,
}

#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoResult {
    pub data_id: String,
    pub pareto: Vec<CompositeMoPoint>,
    pub total_evaluations: usize,
    pub runtime_ms: u64,
    pub telemetry: CompositeMoTelemetry,
}
```

### 8.1.2 Fix prescan device resolution

Replace the current hardwired preferred-device path with a config-first resolver.

```rust
fn resolve_composite_prescan_device_id(
    cfg: &CompositeGpuPrescanConfig,
) -> Result<Option<u32>, String> {
    if let Some(device_id) = cfg.device_id {
        return Ok(Some(device_id));
    }

    Ok(crate::preferred_cuda_device()
        .map_err(|err| format!("failed to resolve preferred CUDA device: {err}"))?
        .map(|device| device.device_id))
}
```

Then in the prescan block:

```rust
let prescan_requested_device_id = cfg.device_id;
let resolved_device_id = resolve_composite_prescan_device_id(&cfg)?;
let device_id = resolved_device_id.unwrap_or(0);
```

### 8.1.3 Make prescan type-aware

Introduce a type-aware allowed-config key.

```rust
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PrescannedConfigKey {
    fast_ma_type: String,
    slow_ma_type: String,
    fast_len: u16,
    slow_len: u16,
}
```

Replace:

```rust
let mut pair_set: HashSet<(u16, u16)> = HashSet::new();
```

with:

```rust
let mut allowed_configs: HashSet<PrescannedConfigKey> = HashSet::new();
let mut allowed_pairs_by_type: HashMap<(String, String), Vec<(u16, u16)>> = HashMap::new();
```

When expanding prescan windows:

```rust
for &fast in fast_window {
    for &slow in slow_window {
        if fast >= slow {
            continue;
        }

        fast_set.insert(fast);
        slow_set.insert(slow);

        allowed_configs.insert(PrescannedConfigKey {
            fast_ma_type: fast_ma_type.clone(),
            slow_ma_type: slow_ma_type.clone(),
            fast_len: fast,
            slow_len: slow,
        });
    }
}
```

After candidate rewrite, rebuild the per-type pair map:

```rust
let mut allowed_pairs_by_type: HashMap<(String, String), Vec<(u16, u16)>> = HashMap::new();
for cfg_key in allowed_configs.into_iter() {
    if !fast_allowed.contains(&cfg_key.fast_len) || !slow_allowed.contains(&cfg_key.slow_len) {
        continue;
    }
    if cfg_key.fast_len >= cfg_key.slow_len {
        continue;
    }

    allowed_pairs_by_type
        .entry((cfg_key.fast_ma_type, cfg_key.slow_ma_type))
        .or_default()
        .push((cfg_key.fast_len, cfg_key.slow_len));
}

for pairs in allowed_pairs_by_type.values_mut() {
    pairs.sort_unstable();
    pairs.dedup();
}
```

### 8.1.4 Decode MA types before prescan-pair validation

Today `eval_one(...)` checks `allowed_pairs` before it knows the active MA types. Move MA-type decoding up and validate against the type-specific pair list.

```rust
let fast_ma_type =
    match decode_ma_type(&search_space, values, idx.fast_ma_type, &base_fast_ma) {
        Ok(v) => v,
        Err(_) => return invalid_default_point(/* ... */),
    };

let slow_ma_type =
    match decode_ma_type(&search_space, values, idx.slow_ma_type, &base_slow_ma) {
        Ok(v) => v,
        Err(_) => return invalid_default_point(/* ... */),
    };

if let Some(map) = allowed_pairs_by_type.as_ref() {
    let key = (fast_ma_type.clone(), slow_ma_type.clone());
    let valid = map
        .get(&key)
        .is_some_and(|pairs| pairs.binary_search(&(fast_len, slow_len)).is_ok());

    if !valid {
        rejected_prescan_pair.fetch_add(1, Ordering::Relaxed);
        return invalid_default_point(/* ... */);
    }
}
```

### 8.1.5 Make repair prescan-aware

Add a helper that snaps to the nearest valid pair for the current MA type pair.

```rust
fn nearest_allowed_pair(
    pairs: &[(u16, u16)],
    fast_len: u16,
    slow_len: u16,
) -> Option<(u16, u16)> {
    pairs.iter().copied().min_by_key(|&(f, s)| {
        let df = (f as i32 - fast_len as i32).abs() as u32;
        let ds = (s as i32 - slow_len as i32).abs() as u32;
        (df + ds, df.max(ds), f as u32, s as u32)
    })
}
```

Then modify `repair(...)`:

```rust
let repair = |values: &mut [ParamValue]| {
    let fast_len = decode_len(fast_domain, values, idx.fast_len, fast_choice_map.as_deref()).unwrap_or(0);
    let slow_len = decode_len(slow_domain, values, idx.slow_len, slow_choice_map.as_deref()).unwrap_or(0);

    let fast_ma_type =
        decode_ma_type(&search_space, values, idx.fast_ma_type, &base_fast_ma).unwrap_or_else(|_| base_fast_ma.clone());
    let slow_ma_type =
        decode_ma_type(&search_space, values, idx.slow_ma_type, &base_slow_ma).unwrap_or_else(|_| base_slow_ma.clone());

    if let Some(map) = allowed_pairs_by_type.as_ref() {
        if let Some(pairs) = map.get(&(fast_ma_type.clone(), slow_ma_type.clone())) {
            if let Some((new_fast, new_slow)) = nearest_allowed_pair(pairs, fast_len.max(1), slow_len.max(1)) {
                set_len_value(values, idx.fast_len, fast_domain, fast_choice_idx_by_period.as_ref(), new_fast);
                set_len_value(values, idx.slow_len, slow_domain, slow_choice_idx_by_period.as_ref(), new_slow);
                return;
            }
        }
    }

    // fallback to existing ordering-only repair
    if fast_len == 0 || slow_len == 0 || fast_len < slow_len {
        return;
    }

    // existing max_less_than / min_greater_than fallback stays here
};
```

### 8.1.6 Replace row-by-row sparse bank building with arithmetic-run batching

Add a run splitter:

```rust
fn arithmetic_period_runs(periods: &[u16]) -> Vec<&[u16]> {
    if periods.is_empty() {
        return Vec::new();
    }
    if periods.len() <= 2 {
        return vec![periods];
    }

    let mut runs: Vec<&[u16]> = Vec::new();
    let mut start = 0usize;
    let mut step = periods[1] as i32 - periods[0] as i32;

    for i in 2..periods.len() {
        let cur_step = periods[i] as i32 - periods[i - 1] as i32;
        if cur_step != step {
            runs.push(&periods[start..i]);
            start = i;
            if i + 1 < periods.len() {
                step = periods[i + 1] as i32 - periods[i] as i32;
            }
        }
    }

    runs.push(&periods[start..]);
    runs
}
```

Then change the sparse path in `build_ma_bank(...)`:

```rust
let total = periods.len().saturating_mul(cols);
let mut values = crate::alloc_policy::alloc_f64_with_prefix_uninit_tail(total, 0, 0.0);

for run in arithmetic_period_runs(periods) {
    let (mat, cols2) = ma_matrix_cpu_f64(
        candles,
        ma_type,
        &req.ma_source,
        run,
        ma_params,
        alma_offset,
        alma_sigma,
    )?;
    if cols2 != cols {
        return Err("MA matrix column mismatch".to_string());
    }

    for (local_row, &period) in run.iter().enumerate() {
        let global_row = *map
            .get(&period)
            .ok_or_else(|| "period missing from MA bank index map".to_string())?;
        let src = local_row.saturating_mul(cols);
        let dst = global_row.saturating_mul(cols);
        values[dst..dst + cols].copy_from_slice(&mat[src..src + cols]);
    }
}
```

This preserves the current bank layout but should cut the number of `ma_matrix_cpu_f64(...)` calls sharply in common prescan-generated candidate sets.

### 8.1.7 Add exact-grid auto-switch for small fully discrete spaces

Use the optimizer crate’s existing grid support when the discrete space is small enough.

```rust
const EXACT_GRID_MAX_AXIS_VALUES: usize = 512;
const EXACT_GRID_MAX_COMBOS: usize = 100_000;

fn should_use_exact_grid(space: &SearchSpace, configured_budget: usize) -> bool {
    match space.grid(EXACT_GRID_MAX_AXIS_VALUES, EXACT_GRID_MAX_COMBOS) {
        Ok(grid) => {
            let combos = grid.len();
            combos > 0 && combos <= configured_budget.saturating_mul(2).max(1024)
        }
        Err(_) => false,
    }
}
```

Then before sampler dispatch:

```rust
let configured_budget = match req.sampler.clone() {
    CompositeMoSampler::Nsga2 { config } => config.population_size.saturating_mul(config.generations.saturating_add(1)),
    CompositeMoSampler::Random { config } => config.trials,
    CompositeMoSampler::Tpe { config } => config.trials,
    CompositeMoSampler::Qmc { config } => config.trials,
};

if should_use_exact_grid(&req.search_space, configured_budget) {
    let pareto = ta_optimizer::mo::exact_grid_mo_search(
        &req.search_space,
        EXACT_GRID_MAX_AXIS_VALUES,
        EXACT_GRID_MAX_COMBOS,
        eval_with_progress,
    )?;

    let runtime_ms = start.elapsed().as_millis() as u64;
    return Ok(CompositeMoResult {
        data_id: req.data_id,
        pareto: pareto.into_iter().map(/* existing MoPoint -> CompositeMoPoint mapping */).collect(),
        total_evaluations: done_counter.load(Ordering::Relaxed),
        runtime_ms,
        telemetry: /* fill telemetry */,
    });
}
```

### 8.1.8 Add evaluation counters and backend artifact fix

Track unique/feasible/invalid counts with atomics.

```rust
let unique_counter = AtomicUsize::new(0);
let feasible_counter = AtomicUsize::new(0);
let rejected_decode_or_order = AtomicUsize::new(0);
let rejected_prescan_pair = AtomicUsize::new(0);

let seen_hashes = Arc::new(Mutex::new(HashSet::<u64>::with_capacity(progress_total.saturating_mul(2).max(64))));
```

Inside `eval_with_progress(...)`:

```rust
if let Ok(mut seen) = seen_hashes.lock() {
    let key = ta_optimizer::mo::param_values_hash(values);
    if seen.insert(key) {
        unique_counter.fetch_add(1, Ordering::Relaxed);
    }
}

let out = eval_one(values);
if out.constraints.iter().all(|&v| v <= 0.0) {
    feasible_counter.fetch_add(1, Ordering::Relaxed);
}
```

At result assembly:

```rust
let search_stats = CompositeMoSearchStats {
    total_evaluations: total_evals,
    unique_evaluations: unique_counter.load(Ordering::Relaxed),
    feasible_evaluations: feasible_counter.load(Ordering::Relaxed),
    rejected_decode_or_order: rejected_decode_or_order.load(Ordering::Relaxed),
    rejected_prescan_pair: rejected_prescan_pair.load(Ordering::Relaxed),
};

let runtime_backend = match prescan_stats.as_ref().map(|s| s.backend_used.as_str()) {
    Some("Cpu") => "Cpu".to_string(),
    Some("Mixed") => "Cpu+MixedPrescan".to_string(),
    Some(_) => "Cpu+GpuPrescan".to_string(),
    None => "Cpu".to_string(),
};
```

And fix the artifact emission:

```rust
crate::strategy_dynamic::maybe_append_dynamic_strategy_artifact(
    crate::strategy_dynamic::DynamicStrategyArtifactRecord {
        scope: "composite_mo",
        profile: "CompositeMo",
        backend: &runtime_backend,
        dynamic_enabled: dynamic_strategy_enabled,
        shadow_enabled: dynamic_shadow_enabled,
        shadow_every: dynamic_shadow_every,
        shadow_sampled,
        shadow_mismatches,
        runtime_ms,
        work_items: total_evals,
    },
);
```

> If `DynamicStrategyArtifactRecord.backend` cannot borrow a dynamic string today, use a short enum-to-static-string mapping instead:
> - `"Cpu"`
> - `"Cpu+GpuPrescan"`
> - `"Cpu+MixedPrescan"`

### 8.1.9 Tests to add in `composite_mo.rs`

Add at minimum:

```rust
#[test]
fn composite_mo_prescan_uses_explicit_device_id_when_provided() {
    // Build a prescan request with device_id = Some(7)
    // Run with a mocked or test-only resolver if available
    // Assert telemetry.prescan.resolved_device_id == Some(7)
}

#[test]
fn composite_mo_prescan_repair_snaps_to_valid_pair() {
    // Build a request with categorical post-prescan fast/slow candidates
    // Construct a values vector that is ordered but not in the valid pair set
    // Run repair(...)
    // Assert the repaired pair is in the valid pair set
}

#[test]
fn composite_mo_type_aware_prescan_restricts_pairs_per_ma_type() {
    // Enable fast_ma_type / slow_ma_type optimization plus multi-pair prescan
    // Assert invalid type/pair combinations become infeasible
}

#[test]
fn composite_mo_reports_prescan_telemetry() {
    // Run with prescan on
    // Assert telemetry.prescan.is_some()
    // Assert runtime_ms and candidate counts are populated
}
```

---

## 8.2 `crates/optimizer/src/mo.rs`

### 8.2.1 Expose the parameter-value hash helper

Make the hash reusable from `CompositeMo`:

```rust
pub fn param_values_hash(values: &[ParamValue]) -> u64 {
    let mut h: u64 = 0x9E37_79B9_7F4A_7C15;
    for v in values {
        let (tag, x): (u64, u64) = match v {
            ParamValue::Int(i) => (1, *i as u64),
            ParamValue::Float(f) => (2, f.to_bits()),
            ParamValue::Bool(b) => (3, if *b { 1 } else { 0 }),
            ParamValue::Cat(i) => (4, *i as u64),
        };
        let k = x
            .wrapping_add(tag.wrapping_mul(0xD6E8_FEB8_6659_FD93))
            .wrapping_add(0xA076_1D64_78BD_642F);
        h ^= k;
        h = h.rotate_left(27).wrapping_mul(0x3C79_AC49_2BA7_B653);
        h = h.wrapping_add(0x1C69_B3F7_4AC4_A3C1);
    }
    h
}
```

Then replace internal `values_hash(...)` calls with `param_values_hash(...)`.

### 8.2.2 Add an exact-grid helper

This lets `CompositeMo` use exact enumeration without duplicating logic.

```rust
pub fn exact_grid_mo_search<T, F>(
    space: &SearchSpace,
    max_axis_values: usize,
    max_combos: usize,
    eval: F,
) -> Result<Vec<MoPoint<T>>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
{
    let grid = space.grid(max_axis_values, max_combos)?;
    let mut evaluated: Vec<MoPoint<T>> = Vec::with_capacity(grid.len());

    for chunk in grid.chunks(2048) {
        let batch: Vec<MoPoint<T>> = chunk
            .par_iter()
            .map(|values| {
                let out = eval(values);
                let violation = overall_violation(&out.constraints);
                MoPoint {
                    values: values.clone(),
                    objectives: out.objectives,
                    violation,
                    payload: out.payload,
                }
            })
            .collect();
        evaluated.extend(batch);
    }

    Ok(pareto_front_constrained(&evaluated))
}
```

### 8.2.3 Dedup `Random` samples

Use the same hash helper to suppress duplicate repaired values.

```rust
pub fn random_mo_search<T, F, Repair>(
    space: &SearchSpace,
    cfg: RandomMoConfig,
    repair: Repair,
    eval: F,
) -> Result<Vec<MoPoint<T>>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
{
    space.validate()?;
    if cfg.trials == 0 {
        return Ok(Vec::new());
    }

    let mut rng = StdRng::seed_from_u64(cfg.seed);
    let mut seen: HashSet<u64> = HashSet::with_capacity(cfg.trials.saturating_mul(2));
    let mut values: Vec<Vec<ParamValue>> = Vec::with_capacity(cfg.trials);

    let max_attempts = cfg.trials.saturating_mul(8).max(cfg.trials);
    let mut attempts = 0usize;

    while values.len() < cfg.trials && attempts < max_attempts {
        let mut v = space.sample(&mut rng);
        repair(&mut v);
        let key = param_values_hash(&v);
        if seen.insert(key) {
            values.push(v);
        }
        attempts = attempts.saturating_add(1);
    }

    let evaluated: Vec<MoPoint<T>> = values
        .par_iter()
        .map(|v| {
            let out = eval(v);
            let violation = overall_violation(&out.constraints);
            MoPoint {
                values: v.clone(),
                objectives: out.objectives,
                violation,
                payload: out.payload,
            }
        })
        .collect();

    Ok(pareto_front_constrained(&evaluated))
}
```

### 8.2.4 Dedup `Qmc` samples

Apply the same approach after repair:

```rust
let mut seen: HashSet<u64> = HashSet::with_capacity(cfg.trials.saturating_mul(2));
...
repair(&mut v);
let key = param_values_hash(&v);
if seen.insert(key) {
    values.push(v);
}
```

If you want stronger coverage in tiny spaces, keep drawing additional Sobol/Halton points until you hit:
- `cfg.trials`
- or a max-attempts threshold

### 8.2.5 Dedup initial and offspring NSGA-II populations

Add generation-local dedup during both:
- initial population generation
- offspring generation

```rust
let mut init_seen: HashSet<u64> = HashSet::with_capacity(cfg.population_size.saturating_mul(2));
let mut population_values: Vec<Vec<ParamValue>> = Vec::with_capacity(cfg.population_size);
let mut init_attempts = 0usize;

while population_values.len() < cfg.population_size && init_attempts < cfg.population_size.saturating_mul(8) {
    let mut v = space.sample(&mut rng);
    repair(&mut v);
    let key = param_values_hash(&v);
    if init_seen.insert(key) {
        population_values.push(v);
    }
    init_attempts = init_attempts.saturating_add(1);
}
```

For offspring:

```rust
let mut offspring_values: Vec<Vec<ParamValue>> = Vec::with_capacity(cfg.population_size);
let mut offspring_seen: HashSet<u64> = HashSet::with_capacity(cfg.population_size.saturating_mul(2));
let mut offspring_attempts = 0usize;

while offspring_values.len() < cfg.population_size
    && offspring_attempts < cfg.population_size.saturating_mul(16)
{
    // existing crossover/mutation logic
    repair(&mut child);

    let key = param_values_hash(&child);
    if offspring_seen.insert(key) {
        offspring_values.push(child);
    }

    offspring_attempts = offspring_attempts.saturating_add(1);
}
```

### 8.2.6 Replace NSGA-II’s final full non-dominated sort

This is the easiest NSGA-II tail reduction.

```rust
let mut final_seen: HashSet<u64> = HashSet::with_capacity(all_seen.len().saturating_mul(2));
all_seen.retain(|p| final_seen.insert(param_values_hash(&p.values)));

let pareto = pareto_front_constrained(&all_seen);
```

That replaces:

```rust
let fronts = non_dominated_sort(&all_seen);
let pareto = fronts
    .first()
    .cloned()
    .unwrap_or_default()
    .into_iter()
    .map(|idx| all_seen[idx].clone())
    .collect::<Vec<_>>();
```

### 8.2.7 Tests to add in `mo.rs`

```rust
#[test]
fn random_mo_search_dedups_repaired_duplicates() {
    // Tiny categorical space with a repair that collapses many raw samples
    // Assert evaluated frontier contains only unique hashes
}

#[test]
fn qmc_mo_search_dedups_discrete_projection_duplicates() {
    // Tiny Bool + small categorical space
    // Assert no repeated repaired values in the evaluated set path
}

#[test]
fn nsga2_final_pareto_matches_fast_frontier_helper() {
    // Run a small deterministic NSGA-II search
    // Compare old front and new front logically
}

#[test]
fn exact_grid_mo_search_matches_known_pareto_set() {
    // Tiny fully discrete search space with a hand-checkable frontier
}
```

---

## 8.3 `crates/optimizer/src/lib.rs`

### Recommended change

No functional API expansion is required here for the safe optimization set. The existing enum surface is adequate.

The useful change is a documentation clarification so future readers do not assume `GpuVdps` accelerates the full `CompositeMo` inner loop.

```rust
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum OptimizationMode {
    Grid,
    CoarseToFine,
    /// VRAM-aware deterministic progressive search (GPU-oriented).
    ///
    /// Today this mode is primarily relevant to Double-MA-family search paths.
    /// In CompositeMo, it currently affects only stage-0 prescan forwarding,
    /// not the main CPU-side multi-objective evaluator.
    GpuVdps,
    Auto,
}
```

### Why this file does not need more changes right now

- `SearchSpace::grid_iter(...)` and `SearchSpace::grid(...)` already exist in `mo.rs`
- the biggest `CompositeMo` improvements are in the app-core integration, not this root module
- adding new global optimizer enums or modes now would be premature

---

## 8.4 `web/src/services/optimizeRequestBuilder.ts`

### 8.4.1 Add an optional prescan device-id input and stop forcing `device_id: 0`

Extend the input interface:

```ts
export interface CompositeMoOptimizationBuildInputs extends StrategyConfigBuildInputs {
  // existing fields ...
  moPrescanDeviceId?: unknown;
}
```

Add a helper:

```ts
function parseOptionalNonNegativeU32(
  value: unknown,
  label: string,
): number | null {
  const parsed = parseOptionalFinite(value);
  if (parsed == null) return null;
  const integer = Math.floor(parsed);
  if (!Number.isFinite(integer) || integer < 0 || integer > 0xffff_ffff) {
    throw new Error(`${label} must be in [0, 4294967295].`);
  }
  return integer;
}
```

Then in `buildCompositeMoOptimizationRequest(...)`:

```ts
const prescanDeviceId = parseOptionalNonNegativeU32(
  inputs.moPrescanDeviceId,
  "Composite MO prescan device id",
);

gpuPrescan = {
  top_k: topK,
  window,
  predictive_top_k: predictiveTopK,
  predictive_stride: predictiveStride,
  ...(prescanDeviceId == null ? {} : { device_id: prescanDeviceId }),
  require_gpu: normalizeBoolean(inputs.moPrescanRequireGpu),
  objectives,
  ma_pairs: maPairs,
  mode,
};
```

### 8.4.2 Estimate whether the normalized search space is fully discrete and small

This is useful for:
- capping default trial counts
- future exact-grid auto-eligibility logs
- warning when a tiny space gets a huge stochastic budget

```ts
function estimateCompositeMoDiscreteCombos(
  searchSpace: Record<string, unknown>,
  comboLimit = 200_000,
): { fullyDiscrete: boolean; combos: number | null } {
  const params = Array.isArray(searchSpace.params)
    ? (searchSpace.params as Array<Record<string, unknown>>)
    : [];

  let combos = 1;

  for (const param of params) {
    const domain = param.domain;
    let axisCount: number | null = null;

    if (domain === "Bool") {
      axisCount = 2;
    } else if (domain && typeof domain === "object") {
      const row = domain as Record<string, unknown>;

      if (row.Categorical && typeof row.Categorical === "object") {
        const choices = (row.Categorical as Record<string, unknown>).choices;
        if (Array.isArray(choices)) {
          axisCount = choices.length;
        }
      } else if (row.IntRange && typeof row.IntRange === "object") {
        const range = row.IntRange as Record<string, unknown>;
        axisCount = countF64RangeValues(
          num(range.low, Number.NaN),
          num(range.high, Number.NaN),
          num(range.step, Number.NaN),
        );
      }
    }

    if (!Number.isFinite(axisCount) || axisCount == null || axisCount < 1) {
      return { fullyDiscrete: false, combos: null };
    }

    combos *= Math.max(1, Math.floor(axisCount));
    if (!Number.isFinite(combos) || combos > comboLimit) {
      return { fullyDiscrete: true, combos: comboLimit + 1 };
    }
  }

  return { fullyDiscrete: true, combos };
}
```

### 8.4.3 Auto-expand prescan MA pairs when MA types are optimized

This prevents type-undercoverage when the raw search space contains categorical `fast_ma_type` / `slow_ma_type` params.

```ts
function extractCategoricalStringChoices(
  searchSpace: Record<string, unknown>,
  paramName: string,
): string[] | null {
  const params = Array.isArray(searchSpace.params)
    ? (searchSpace.params as Array<Record<string, unknown>>)
    : [];

  for (const param of params) {
    const name = normalizeText(param.name).toLowerCase();
    if (name !== paramName.toLowerCase()) continue;

    const domain = param.domain;
    if (!domain || typeof domain !== "object") return null;

    const row = domain as Record<string, unknown>;
    const cat = row.Categorical;
    if (!cat || typeof cat !== "object") return null;

    const choices = (cat as Record<string, unknown>).choices;
    if (!Array.isArray(choices)) return null;

    const out = choices
      .map((choice) => normalizeText(choice).toLowerCase())
      .filter(Boolean);

    return out.length ? Array.from(new Set(out)) : null;
  }

  return null;
}
```

Then:

```ts
function buildCompositeMoDefaultPrescanPairs(
  searchSpace: Record<string, unknown>,
  fastFallback: string,
  slowFallback: string,
  maxPairs = 8,
): Array<{ fast_ma_type: string; slow_ma_type: string }> | null {
  const fastChoices =
    extractCategoricalStringChoices(searchSpace, "fast_ma_type") ?? [fastFallback.toLowerCase()];
  const slowChoices =
    extractCategoricalStringChoices(searchSpace, "slow_ma_type") ?? [slowFallback.toLowerCase()];

  const out: Array<{ fast_ma_type: string; slow_ma_type: string }> = [];
  for (const fast of fastChoices) {
    for (const slow of slowChoices) {
      out.push({ fast_ma_type: fast, slow_ma_type: slow });
      if (out.length > maxPairs) {
        throw new Error(
          `Composite MO prescan would need more than ${maxPairs} MA pairs. Narrow fast/slow MA type choices or specify prescan pairs explicitly.`,
        );
      }
    }
  }

  return out;
}
```

Use it when prescan is enabled and the user did not explicitly provide `moPrescanMaPairs`.

### 8.4.4 Make sampler defaults shape-aware without adding new knobs

After building `searchSpace`, estimate discrete combos and cap default trial counts when the user did **not** explicitly set them.

Example for `Random`:

```ts
const searchStats = estimateCompositeMoDiscreteCombos(searchSpace);
const randomTrialsDefault =
  searchStats.fullyDiscrete && searchStats.combos != null
    ? Math.min(2000, Math.max(1, searchStats.combos))
    : 2000;
```

Repeat the same idea for:
- `Qmc`
- `Tpe`
- `Nsga2` total evaluation budget

Do **not** add a new UI knob yet. Keep it as an internal request-builder improvement.

### 8.4.5 Remove the redundant `strategy` field from the CompositeMo payload

The backend request consumes `base_strategy`. The extra `strategy` field is redundant and can cause confusion.

Change:

```ts
return {
  strategy_kind: "CompositeMo",
  // ...
  base_strategy: baseStrategy,
  signal_mode: signalMode,
  strategy: baseStrategy,
  search_space: searchSpace,
  // ...
};
```

to:

```ts
return {
  strategy_kind: "CompositeMo",
  // ...
  base_strategy: baseStrategy,
  signal_mode: signalMode,
  search_space: searchSpace,
  // ...
};
```

### 8.4.6 Builder validation additions

Add two safe validations:

1. if prescan is enabled and optimized MA types imply > 8 prescan pairs, fail early with a clear message
2. if the space is tiny and fully discrete, cap default stochastic budgets rather than letting them overshoot the space

---

## 8.5 `web/src/utils/strategyOptimizationCapabilities.ts`

The current description is directionally right but too vague for route transparency.

### Recommended change

Make it explicit that:

- the main `CompositeMo` evaluator is CPU-side
- optional GPU prescan only narrows MA domains
- it does not participate in the global backend selector

```ts
if (kind === 'CompositeMo') {
  return {
    kind,
    usesGlobalBackend: false,
    usesGlobalMode: false,
    supportsHeatmap: false,
    supportsExportAll: false,
    supportsTopK: false,
    supportsIncludeAll: false,
    supportsExtraShortlistObjectives: false,
    supportsTradeMetrics: false,
    supportsShortlistConfidenceMode: false,
    supportsShortlistUnionMode: false,
    supportsAdvancedExactKernelControls: false,
    executionSummary:
      'Composite MO runs a CPU-side multi-objective evaluator with its own sampler settings. An optional GPU prescan can narrow MA-length domains before the CPU search begins.',
    modeSummary:
      'Composite MO does not use the global optimization-mode selector. Prescan mode only affects the stage-0 Double-MA narrowing pass.',
    outputSummary:
      'Composite MO returns Pareto points from its own search-space and prescan settings rather than Double-MA Top-K, heatmap, include-all, or export-all outputs.',
  }
}
```

This improves route/fallback honesty without expanding UI surface area.

---

## 8.6 `crates/app_core/benches/dynamic_strategy_cpu.rs`

Keep the current dynamic-env benchmark group, but add a new `CompositeMo`-focused set.

### 8.6.1 Prescan payoff benchmark group

```rust
let candles_mo_large = make_synth_candles(10_000);

let req_mo_large_no_prescan = CompositeMoRequest {
    data_id: "bench".to_string(),
    fast_ma_type: "sma".to_string(),
    slow_ma_type: "sma".to_string(),
    ma_source: ma_source.to_string(),
    fast_ma_params: None,
    slow_ma_params: None,
    base_strategy: StrategyConfig::default(),
    signal_mode: CompositeSignalMode::HardGate,
    search_space: SearchSpace {
        params: vec![
            ParamDef {
                name: "fast_len".to_string(),
                domain: ParamDomain::IntRange {
                    low: 5,
                    high: 200,
                    step: 1,
                    log: false,
                },
            },
            ParamDef {
                name: "slow_len".to_string(),
                domain: ParamDomain::IntRange {
                    low: 30,
                    high: 250,
                    step: 1,
                    log: false,
                },
            },
            ParamDef {
                name: "ott_ma_type".to_string(),
                domain: ParamDomain::Categorical {
                    choices: vec!["sma".to_string()],
                },
            },
        ],
    },
    objectives: CompositeMoObjectives {
        maximize: ObjectiveKind::Sharpe,
        minimize: ObjectiveKind::MaxDrawdown,
    },
    sampler: CompositeMoSampler::Random {
        config: RandomMoConfig { seed: 42, trials: 256 },
    },
    constraints: None,
    gpu_prescan: None,
    mode: None,
};

let req_mo_large_prescan = CompositeMoRequest {
    gpu_prescan: Some(CompositeGpuPrescanConfig {
        top_k: 16,
        window: 2,
        predictive_top_k: 8,
        predictive_stride: 8,
        device_id: None,
        require_gpu: false,
        objectives: CompositeGpuPrescanObjectives::Both,
        ma_pairs: None,
        mode: OptimizationMode::Auto,
    }),
    ..req_mo_large_no_prescan.clone()
};

let mut prescan_group = c.benchmark_group("composite_mo/prescan_payoff/10k");
prescan_group.measurement_time(Duration::from_secs(8));
prescan_group.sample_size(10);

prescan_group.bench_function("random/no_prescan", |b| {
    b.iter(|| {
        let out = run_composite_mo_optimization_blocking_with_candles(
            req_mo_large_no_prescan.clone(),
            &candles_mo_large,
            &cancel,
            None,
        ).expect("CompositeMo no-prescan run");
        black_box(out.runtime_ms);
    });
});

prescan_group.bench_function("random/prescan", |b| {
    b.iter(|| {
        let out = run_composite_mo_optimization_blocking_with_candles(
            req_mo_large_prescan.clone(),
            &candles_mo_large,
            &cancel,
            None,
        ).expect("CompositeMo prescan run");
        black_box(out.telemetry.prescan.as_ref().map(|s| s.candidate_pairs));
    });
});

prescan_group.finish();
```

### 8.6.2 Sampler-overhead benchmark group

Build a small fully discrete `CompositeMo` space and compare:

- `Random`
- `Qmc`
- `Tpe`
- `Nsga2`

The key is not raw runtime alone. Once telemetry exists, consume:

- `unique_evaluations`
- `feasible_evaluations`
- `pareto.len()`

### 8.6.3 Sparse-bank benchmark group

Create two requests that differ only in the post-prescan-like period shapes:

- one uniform run
- one sparse categorical candidate list

Benchmark only the prep+bank-build portion if possible.

### 8.6.4 NSGA-II tail benchmark group

Run larger NSGA-II budgets and use telemetry timestamps to track:
- total runtime
- final frontier extraction cost
- unique points seen

---

## 9. Rollout order and implementation notes

### 9.1 Recommended order

1. `composite_mo.rs`
   - telemetry
   - device-id fix
   - artifact backend label fix

2. `mo.rs`
   - public hash helper
   - NSGA-II tail fix
   - random/qmc dedup

3. `composite_mo.rs`
   - type-aware allowed pairs
   - pair-aware repair

4. `composite_mo.rs`
   - sparse bank run batching

5. `mo.rs` + `composite_mo.rs`
   - exact-grid fast path

6. `optimizeRequestBuilder.ts`
   - optional prescan device id
   - type-pair auto-expansion
   - shape-aware defaults
   - redundant payload cleanup

7. `strategyOptimizationCapabilities.ts`
   - transparency wording update

8. `dynamic_strategy_cpu.rs`
   - benchmark additions

### 9.2 Things to avoid during this implementation

Do **not** do these in the first pass:

- do not add a new user-facing “exact grid” toggle
- do not redesign the optimizer API around a new global mode enum
- do not rewrite the main `CompositeMo` evaluator for GPU first
- do not add new UI surfaces
- do not retune all sampler constants before telemetry and benchmarks exist

### 9.3 Acceptance criteria for the first pass

A good first-pass implementation should satisfy all of these:

- prescan device id is actually honored
- `CompositeMoResult` exposes prescan + search telemetry
- `repair(...)` never leaves a point outside the valid prescanned pair set for the active MA type pair
- `Random`, `Qmc`, and `Nsga2` duplicate rates drop materially in small discrete spaces
- NSGA-II final frontier extraction no longer uses full `non_dominated_sort(...)` over `all_seen`
- sparse bank building no longer performs one full MA call per non-uniform period
- benchmark coverage exists for prescan on/off and sampler overhead

---

## 10. Comprehensive implementation checklist

### 10.1 `crates/app_core/src/composite_mo.rs`

- [ ] Add `CompositeMoSearchStats`
- [ ] Add `CompositeMoPrescanStats`
- [ ] Add `CompositeMoTelemetry`
- [ ] Extend `CompositeMoResult` with `telemetry`
- [ ] Add a prescan device resolver that honors `cfg.device_id`
- [ ] Capture `requested_device_id` and `resolved_device_id`
- [ ] Capture per-prescan-call `backend_used`
- [ ] Summarize prescan backend usage into one string (`Cpu`, `Mixed`, `Gpu*`)
- [ ] Measure prescan runtime separately from total runtime
- [ ] Record prescan candidate fast period count
- [ ] Record prescan candidate slow period count
- [ ] Record prescan candidate pair count
- [ ] Replace global `allowed_pairs` with a type-aware allowed-config structure
- [ ] Decode MA types before prescan-pair validation in `eval_one(...)`
- [ ] Make prescan validity checks type-aware
- [ ] Add pair-aware snapping in `repair(...)`
- [ ] Keep ordering-only repair as fallback when prescan is absent
- [ ] Add counters for rejected decode/order points
- [ ] Add counters for rejected prescan-pair points
- [ ] Add counter for unique evaluations
- [ ] Add counter for feasible evaluations
- [ ] Expose runtime backend in the final result
- [ ] Stop hardcoding the artifact backend to `"Cpu"`
- [ ] Add arithmetic-run batching for sparse period lists
- [ ] Keep MA bank output layout unchanged
- [ ] Add exact-grid auto-switch detection
- [ ] Integrate exact-grid path before sampler dispatch
- [ ] Preserve existing sampler behavior when exact-grid is not eligible
- [ ] Add tests for device-id respect
- [ ] Add tests for prescan telemetry
- [ ] Add tests for type-aware prescan validity
- [ ] Add tests for pair-aware repair
- [ ] Add tests for sparse-bank equivalence

### 10.2 `crates/optimizer/src/mo.rs`

- [ ] Expose `param_values_hash(...)`
- [ ] Replace internal `values_hash(...)` call sites with the shared helper
- [ ] Add `exact_grid_mo_search(...)`
- [ ] Dedup repaired values in `random_mo_search(...)`
- [ ] Dedup repaired values in `qmc_mo_search(...)`
- [ ] Dedup initial NSGA-II population
- [ ] Dedup NSGA-II offspring
- [ ] Add a bounded retry strategy so dedup does not infinite-loop
- [ ] Dedup `all_seen` before final frontier extraction
- [ ] Replace NSGA-II final `non_dominated_sort(...)` with `pareto_front_constrained(...)`
- [ ] Add tests for random dedup
- [ ] Add tests for QMC dedup
- [ ] Add tests for NSGA-II final-frontier parity
- [ ] Add tests for exact-grid helper correctness

### 10.3 `crates/optimizer/src/lib.rs`

- [ ] Add doc comment clarifying that `GpuVdps` is not currently a full `CompositeMo` inner-loop accelerator
- [ ] Avoid adding new global optimizer modes in this pass
- [ ] Keep enum/API surface stable unless a later benchmark proves otherwise

### 10.4 `web/src/services/optimizeRequestBuilder.ts`

- [ ] Add `moPrescanDeviceId` input field
- [ ] Add helper to parse optional non-negative device id
- [ ] Stop unconditionally sending `device_id: 0`
- [ ] Only include `device_id` in the request when explicitly provided
- [ ] Add discrete search-space combo estimator
- [ ] Use the estimator to cap default Random trial counts when the space is tiny
- [ ] Use the estimator to cap default QMC trial counts when the space is tiny
- [ ] Use the estimator to cap default TPE trial counts when the space is tiny
- [ ] Use the estimator to cap default NSGA-II total eval budget when the space is tiny
- [ ] Add helper to extract categorical `fast_ma_type` choices from raw search space
- [ ] Add helper to extract categorical `slow_ma_type` choices from raw search space
- [ ] Auto-build prescan MA pairs from MA type domains when the user did not supply them
- [ ] Enforce the backend’s current max-pairs limit in the builder
- [ ] Remove redundant `strategy` from the CompositeMo payload
- [ ] Add tests for device-id shaping
- [ ] Add tests for MA-pair auto-expansion
- [ ] Add tests for tiny-space default trial capping

### 10.5 `web/src/utils/strategyOptimizationCapabilities.ts`

- [ ] Update `CompositeMo` execution summary to say the main evaluator is CPU-side
- [ ] Update `CompositeMo` mode summary to say prescan mode only affects stage-0 narrowing
- [ ] Update `CompositeMo` output summary to reflect Pareto + telemetry semantics
- [ ] Keep capability flags unchanged unless the backend surface actually changes

### 10.6 `crates/app_core/benches/dynamic_strategy_cpu.rs`

- [ ] Keep the existing dynamic-strategy env on/off benchmark
- [ ] Add a `CompositeMo` prescan-off vs prescan-on benchmark group
- [ ] Use a length range that is large but valid without prescan
- [ ] Add a sampler comparison benchmark group
- [ ] Add a sparse-bank benchmark group
- [ ] Add an NSGA-II tail benchmark group
- [ ] Consume new telemetry in the benchmark output/black-box values
- [ ] Keep benchmark names stable and descriptive

### 10.7 Verification after implementation

- [ ] Run the `CompositeMo` unit tests
- [ ] Run the optimizer crate tests for `mo.rs`
- [ ] Run the updated Criterion benchmark file
- [ ] Compare prescan on/off runtime on at least one large-range request
- [ ] Compare unique evaluation counts before/after dedup
- [ ] Compare Pareto contents before/after exact-grid auto-switch on a tiny deterministic space
- [ ] Validate that dynamic-strategy env on/off parity still holds
- [ ] Validate that result JSON shape changes are handled by any downstream caller

### 10.8 Optional second-wave follow-up (only after re-measurement)

- [ ] Profile `indicator_cache` and `dynamic_plan_cache` lock contention
- [ ] Decide whether lazy MA bank construction is worth the complexity
- [ ] Decide whether prescan should request stronger GPU residency by default
- [ ] Decide whether deeper GPU-resident prescan reuse is justified
- [ ] Decide whether a true GPU inner-loop evaluator is worth building
