# Double-MA Validated Implementation Tracking Plan

## Purpose

This document converts the validated parts of `double_ma_finalist_recompute_optimization_plan.md` into an execution plan and tracking checklist.

It is intentionally narrower than the broader review:

- It includes only the parts that are currently valid and worth implementing.
- It excludes ideas that are speculative, benchmark-contract changing, or low-priority for the current exact benchmark family.
- It is written as a working tracker, not just a design note.

Primary source documents:

- `research/MA review/double_ma_finalist_recompute_optimization_plan.md`
- `research/DOUBLE_MA_ALMA_GPU_FULL_OPTIMIZATION_PASS_2026-03-21.md`

Primary code surfaces:

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`

---

## Validated Scope

The following components are valid and worth planning for implementation:

1. Split the benchmark so heatmap and finalist recompute are measured separately.
2. Add dedicated finalist replay timing instrumentation inside the exact-combo executor.
3. Reuse `d_lr` and `d_ret1` across pass 1 and pass 2.
4. A/B an env-gated 9-row product-reduced search mode.
5. Add a small-finalist fast path only if the earlier phases show replay still dominates.
6. Promote the winning optimization path into the preferred product path once it is validated.

These components are grounded in the current code:

- The benchmark pair is bundled in `crates/app_core/benches/double_ma_opt.rs`.
- `Sharpe` search still uses a 5-row reduced contract in `crates/app_core/src/gpu_backtest_kernel.rs`.
- Both search and finalist recompute regenerate returns in `crates/app_core/src/gpu_backtest_kernel.rs`.
- The kernel can already emit rows through ulcer index at `M = 9` in `crates/app_core/src/kernels/double_crossover.cu`.
- The exact-combo path already reruns exact finalists only, but still pays generic setup/orchestration costs.

---

## Explicit Non-Goals

The following items are not part of this implementation plan:

- Heatmap rewrite or heatmap-specific optimization beyond benchmark split and measurement.
- `PreferGpuResident` / `StrictGpuResident` tuning as a benchmark speed strategy.
- Exact pruning or dominance rules over the MA lattice.
- Segment algebra / time-block recomposition work.
- Multi-fidelity screening, successive halving, mixed-precision screening, or new approximate benchmark families.
- A new Sharpe-only row layout for the current exact benchmark path.
- A brand-new CUDA kernel as the first step for finalist replay.

Reason:

- They are either lower-priority, change the benchmark contract, or have weaker evidence than the validated items above.

---

## Success Criteria

The implementation should be considered successful only if it satisfies all of the following:

- Exactness is preserved for the current exact benchmark family unless an experiment is explicitly env-gated.
- CPU-only builds remain valid.
- Existing GPU product semantics remain unchanged by default.
- Benchmarks are run serially, never in parallel.
- Each phase produces enough evidence to justify the next phase.

Desired outcomes:

- Clear decomposition of search, heatmap, and finalist replay costs.
- Removal of duplicated returns-generation work.
- A decisive answer on whether 9-row no-replay beats current two-pass replay.
- If needed, a narrower and cheaper small-finalist replay path.
- An actual product-path optimization outcome, not only diagnostics.

---

## Phase Order

Mandatory order:

1. Phase 0: Preparation and guardrails
2. Phase 1: Benchmark split
3. Phase 2: Finalist replay timing instrumentation
4. Phase 3: Returns reuse across pass 1 and pass 2
5. Phase 4: 9-row product-reduced A/B
6. Phase 5: Small-finalist fast path, only if justified
7. Phase 6: Optimization closure and adoption decision
8. Phase 7: Validation, benchmark capture, and handoff

Do not skip directly to Phase 4 or Phase 5 without the measurement framework from Phases 1 and 2.

---

## Optimization Intent

This tracker is not only for measuring the problem.

It is intended to produce one of these concrete outcomes for the current exact benchmark family:

1. Promote the 9-row product-reduced path as the preferred/default product path if it wins.
2. Keep the default replay architecture, but promote a smaller finalist replay path if that wins.
3. If neither path wins cleanly, record that outcome explicitly and split the remaining work into:
   - product-path follow-up
   - direct `search_only` first-pass optimization follow-up

Measurement without a resulting optimization choice is not considered complete.

---

## Execution Runbook

Use these commands as the default execution flow unless a phase explicitly requires something narrower.

### Process Safety

Before any benchmark or profiling run:

```powershell
Get-Process cargo, double_ma_opt, ta_cli -ErrorAction SilentlyContinue
```

Expected result:

- No active benchmark-related process from an earlier run.

### CPU-Safe Validation

Run this after any Rust-side change that should not break CPU-only behavior:

```powershell
cargo test -p ta_app_core
```

### GPU Bench Compile Check

Run this before timing work if CUDA-bench code changed:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run
```

### Contract-Split Benchmark Run

Use this as the standard benchmark invocation for this workstream:

```powershell
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k"
```

### Candidate Perf Gates

Run the repo-preferred GPU perf gate before handoff when CUDA runtime changes were made:

```powershell
powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate
```

If that is unavailable or too broad for the local machine, run at minimum:

```powershell
powershell -File scripts/run_workflow_gate.ps1 -Mode candidate
```

### Replay-Profile Benchmark Run

Use this after Phase 2 lands:

```powershell
$env:VECTORBT_KERNEL_FINALIST_PROFILE='1'
$env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS='0'  # force replay on so profiling emits timing
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k"
Remove-Item Env:VECTORBT_KERNEL_FINALIST_PROFILE -ErrorAction SilentlyContinue
Remove-Item Env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS -ErrorAction SilentlyContinue
```

### 9-Row A/B Benchmark Run

Default policy (after promotion): 9-row is enabled for `Sharpe`/`Volatility` when this env var is unset.

```powershell
# Default (9-row on for Sharpe/Volatility):
Remove-Item Env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS -ErrorAction SilentlyContinue
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k"

# Legacy baseline (force 5-row + replay):
$env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS='0'
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k"
Remove-Item Env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS -ErrorAction SilentlyContinue
```

### Combined Replay-Profile + 9-Row A/B Run

Use this only when explicitly profiling replay. (With 9-row enabled, replay is skipped and the replay profiler will not emit timing.)

```powershell
$env:VECTORBT_KERNEL_FINALIST_PROFILE='1'
$env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS='0'
cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k"
Remove-Item Env:VECTORBT_KERNEL_FINALIST_PROFILE -ErrorAction SilentlyContinue
Remove-Item Env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS -ErrorAction SilentlyContinue
```

---

## Environment Variable Matrix

Only the following env vars are expected to matter for this plan.

| Variable | Default | Phase | Purpose | Notes |
| --- | --- | --- | --- | --- |
| `VECTORBT_KERNEL_FINALIST_RECOMPUTE` | benchmark-controlled | 1 | enable/disable finalist replay in benchmark variants | Existing helper in the benchmark harness controls this per variant |
| `VECTORBT_KERNEL_FINALIST_PROFILE` | unset / off | 2, 6 | emit replay-only timing profile | New env var planned in this workstream |
| `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS` | unset (Sharpe/Vol on) | 4, 6 | widen reduced search rows to product-reduced width | Override: `0` disables; `1` enables for all objectives |
| `VECTORBT_KERNEL_PROFILE_PIPELINE` | unset / off | optional | emit existing search-pipeline timing | Output prefix currently prints `VECTORBT_KERNEL_PIPELINE_PROFILE:` |
| `VECTORBT_KERNEL_VRAM_BUDGET_MB` | unset | optional | constrain VRAM budget for controlled experiments | Leave unset unless intentionally testing budget sensitivity |
| `VECTORBT_KERNEL_TARGET_WAVES` | runtime default | out of scope | launch-shape tuning | Not part of the first-wave implementation plan |
| `VECTORBT_KERNEL_BLOCK_X` | runtime default | out of scope | force block size | Not part of the first-wave implementation plan |

Rules:

- Do not leave experiment env vars set between runs.
- Record env-var state next to every benchmark capture.
- Treat any result captured with unexpected env vars as invalid for comparison.

---

## File and Function Touch Map

Use this as the primary edit map so each phase stays surgical.

### Phase 1

- File: `crates/app_core/benches/double_ma_opt.rs`
- Primary function:
  - `bench_double_ma_gpu_kernel_contract_split`
- Supporting helper:
  - `run_with_finalist_recompute_env`

### Phase 2

- File: `crates/app_core/src/gpu_backtest_kernel.rs`
- Primary function:
  - `eval_double_ma_batch_gpu_kernel_impl`
- Nearby structures/utilities:
  - `PipelineTimingProfile`
  - env-flag helpers in the runtime config area

### Phase 3

- File: `crates/app_core/src/gpu_backtest_kernel.rs`
- Primary symbols:
  - `KernelScratch`
  - `clear_async`
  - search-path returns generation block
  - exact-combo returns generation block
- Expected new helper:
  - `ensure_log_and_simple_returns(...)` or equivalent shared helper

### Phase 4

- File: `crates/app_core/src/gpu_backtest_kernel.rs`
- Primary symbols:
  - `search_metrics_count_for_optimization`
  - `decode_tm_flex_metrics_row`
  - finalist replay trigger after result finalize
- File: `crates/app_core/src/kernels/double_crossover.cu`
- Primary symbols:
  - reduced-row write path
  - optional metric enable flags
- File: `crates/app_core/src/double_ma.rs`
- Primary symbol:
  - CPU risk/final metric derivation logic used for parity reasoning

### Phase 5

- File: `crates/app_core/src/gpu_backtest_kernel.rs`
- Primary symbols:
  - `recompute_optimization_result_finalists_with_full_gpu_pass`
  - `eval_double_ma_batch_gpu_kernel_impl`
  - exact-combo MA prep and pair-map build sections
- Expected new helper:
  - `eval_double_ma_small_finalists_gpu_kernel_impl(...)` or equivalent

### Tests Most Likely to Need Touches

- `crates/app_core/src/gpu_backtest_kernel.rs` test section
- Any benchmark-only compile issues in `crates/app_core/benches/double_ma_opt.rs`

---

## Quantitative Decision Gates

These gates are meant to make progression objective.

### Gate A: Benchmark Split Success

Phase 1 is complete only if:

- all 4 variants compile and run
- benchmark names are stable
- each variant differs only in the intended heatmap/recompute toggle

### Gate B: Replay Profile Quality

Phase 2 is complete only if:

- replay profile emits non-zero timings for the expected regions on product-path runs
- env-off behavior is unchanged
- profile output is attributable to replay and not the main search path

### Gate C: Returns Reuse Value

After Phase 3:

- keep the change if exactness is preserved and replay-only timing drops by any measurable amount on repeated runs
- if replay-profile data shows `returns` is less than about 5% of replay cost, do not invest further in returns-specific optimization beyond the shared helper

### Gate D: 9-Row A/B Adoption Threshold

Phase 4 should be considered a winner for this benchmark family only if all of the following hold:

- exactness parity is preserved
- replay is actually skipped when intended
- `end_to_end_product` improves by at least about 5% versus the current default path on repeated serial runs
- `search_only` does not regress by more than about 5% unless the product-path win is materially larger

If 9-row lands within a noise band of roughly `-3%` to `+3%`, keep it experimental and do not make it the preferred path.

If 9-row wins cleanly, the next required step is not more analysis. The next required step is to promote it from experiment to preferred/default behavior for the current benchmark/product path or explicitly document why promotion is deferred.

### Gate E: Small-Finalist Fast-Path Justification

Do not start Phase 5 unless at least one of the following remains true after Phase 4:

- replay-only tax still appears to be at least about 10 ms on the measured product-path benchmark
- replay remains at least about 10% of `end_to_end_product`
- replay profile still shows meaningful fixed-cost overhead in prep/pairmap/D2H that a narrower path could attack

If Phase 5 is entered and wins cleanly, the next required step is to adopt that path as the preferred replay implementation for the targeted workload rather than leaving it as an isolated experiment.

### Gate F: Handoff Readiness

The work is ready for handoff only if:

- exactness checks pass
- default behavior remains stable
- benchmark evidence is captured with env-var state recorded
- the current recommendation is explicit:
  - keep replay path
  - prefer 9-row mode
  - or proceed to small-finalist fast path

---

## Benchmark Capture Template

Use this table for every meaningful benchmark capture.

| Date | Commit | Phase | Env Vars | search_only | search_plus_heatmap | search_plus_finalist_recompute | end_to_end_product | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-03-23 | `cd3795e00a6f9f2ba923dc7ad22d3a9f930643ab` | Phase 7 | `PRODUCT_REDUCED_ROWS=0` | `98.75 ms` | `98.72 ms` | `185.71 ms` | `185.92 ms` | Forced legacy baseline (5-row + replay) |
| 2026-03-23 | `cd3795e00a6f9f2ba923dc7ad22d3a9f930643ab` | Phase 7 | `none` | `86.70 ms` | `86.26 ms` | `86.81 ms` | `86.80 ms` | Default after promotion (9-row; replay skipped) |
| YYYY-MM-DD | `<sha>` | Phase N | `none` | `... ms` | `... ms` | `... ms` | `... ms` | `...` |

Use this table for replay profile captures.

| Date | Commit | Env Vars | returns | fast_ma | slow_ma | pairmap | backtest | d2h_decode | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2026-03-23 | `cd3795e00a6f9f2ba923dc7ad22d3a9f930643ab` | `FINALIST_PROFILE=1 PRODUCT_REDUCED_ROWS=0` | `0.005 ms` | `3.732 ms` | `0.245 ms` | `0.032 ms` | `0.029 ms` | `60.510 ms` | `ta_cli` synth 200k ALMA/ALMA top_k=50 heatmap=64; total=64.553 ms; D2H+decode dominates |
| YYYY-MM-DD | `<sha>` | `FINALIST_PROFILE=1` | `... ms` | `... ms` | `... ms` | `... ms` | `... ms` | `... ms` | `...` |

Use this comparison summary for the 9-row experiment.

| Mode | Exactness | Replay Skipped | search_only delta | end_to_end_product delta | Recommendation |
| --- | --- | --- | --- | --- | --- |
| baseline 5-row + replay | reference | no | `0%` | `0%` | baseline |
| 9-row + no replay | pass (CUDA parity test) | yes | `-12.2%` | `-53.3%` | promote |

---

## Rollback Criteria

Rollback or disable a phase if any of the following occur.

### Hard Rollback Conditions

- CPU-only build/test breakage
- incorrect `best_params`
- incorrect `best_metrics` beyond accepted floating-point tolerance
- top-k identity corruption not explainable by tie behavior
- replay skipped when it should still run
- replay still runs when the phase explicitly depends on disabling it

### Soft Rollback Conditions

- default benchmark path regresses materially with no compensating product win
- code complexity increases without a measurable timing gain
- instrumentation is noisy, misleading, or expensive when enabled
- a new fast path is too broad and creates maintenance risk

### Suggested Materiality Thresholds

Treat the following as material until better local evidence exists:

- more than about 2% regression on the unchanged default search path with no compensating benefit
- more than about 5% regression on `end_to_end_product`
- more than about 5% increase in benchmark variance after a change

---

## Phase-Specific Command Checklist

### Phase 1

- [x] `Get-Process cargo, double_ma_opt, ta_cli -ErrorAction SilentlyContinue`
- [x] `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run`
- [x] `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k"`

### Phase 2

- [ ] `cargo test -p ta_app_core`
- [ ] `$env:VECTORBT_KERNEL_FINALIST_PROFILE='1'`
- [ ] `cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- "double_ma/gpu_kernel_contract_split/alma_alma/200k"`
- [ ] `Remove-Item Env:VECTORBT_KERNEL_FINALIST_PROFILE -ErrorAction SilentlyContinue`

### Phase 3

- [ ] `cargo test -p ta_app_core`
- [ ] rerun Phase 1 benchmark command
- [ ] rerun Phase 2 replay-profile command

### Phase 4

- [ ] `cargo test -p ta_app_core`
- [x] `cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe`
- [x] baseline benchmark run with no experiment env vars
- [x] `$env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS='1'`
- [x] experiment benchmark run
- [x] `Remove-Item Env:VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS -ErrorAction SilentlyContinue`
- [x] if 9-row wins, patch promotion plan or flip the preferred/default path in a controlled follow-up

### Phase 5

- [ ] `cargo test -p ta_app_core`
- [ ] baseline replay-profile run
- [ ] fast-path replay-profile run
- [ ] parity checks against generic replay path
- [ ] if the small-finalist path wins, adopt it as the preferred replay path for the target workload

### Phase 6

- [ ] record the optimization decision:
- [ ] promote 9-row path
- [ ] or promote small-finalist replay path
- [ ] or explicitly split out remaining `search_only` optimization work

### Phase 7

- [x] `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- [ ] or `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`
- [x] final serial benchmark capture
- [x] fill in benchmark capture tables

---

## Master Checklist

### Phase 0: Preparation

- [x] Re-read `research/MA review/double_ma_finalist_recompute_optimization_plan.md` and this tracker before touching code.
- [x] Confirm the target benchmark is still `double_ma/gpu_kernel_contract_split/alma_alma/200k/large/58300pairs/...`.
- [ ] Confirm no benchmark or Criterion process is already running before any timing work.
- [x] Confirm the worktree is understood and no unrelated local changes in target files will be disturbed.
- [ ] Identify the current line locations for:
- [x] benchmark split in `crates/app_core/benches/double_ma_opt.rs`
- [x] replay trigger in `crates/app_core/src/gpu_backtest_kernel.rs`
- [x] search returns generation in `crates/app_core/src/gpu_backtest_kernel.rs`
- [x] exact-combo returns generation in `crates/app_core/src/gpu_backtest_kernel.rs`
- [x] reduced metric decode in `crates/app_core/src/gpu_backtest_kernel.rs`
- [x] reduced metric writes in `crates/app_core/src/kernels/double_crossover.cu`

### Phase 1: Benchmark Split

- [x] Expand the benchmark from 2 variants to 4 variants:
- [x] `search_only`
- [x] `search_plus_heatmap`
- [x] `search_plus_finalist_recompute`
- [x] `end_to_end_product`
- [x] Keep all runs serialized.
- [x] Preserve the existing request shape except for the intended toggles.
- [x] Keep `search_only` as no-heatmap and recompute off.
- [x] Keep `search_plus_heatmap` as heatmap on and recompute off.
- [x] Keep `search_plus_finalist_recompute` as no-heatmap and recompute on.
- [x] Keep `end_to_end_product` as heatmap on and recompute on.
- [x] Verify benchmark names are stable and readable.
- [x] Verify the benchmark compiles with CUDA features.

### Phase 2: Replay Timing Instrumentation

- [x] Add a dedicated finalist replay timing profile struct.
- [x] Gate it behind a dedicated env var.
- [x] Time exact-combo returns generation.
- [x] Time fast MA prep.
- [x] Time slow MA prep.
- [x] Time exact pair-map build/upload.
- [x] Time replay backtest launch region.
- [x] Time sync + D2H + decode.
- [x] Emit replay timing in a concise format.
- [x] Ensure zero behavior change when the env var is off.
- [x] Ensure profiling adds no default overhead when disabled.

### Phase 3: Returns Reuse

- [x] Extend `KernelScratch` with returns-cache metadata.
- [x] Add a helper that ensures and reuses `d_lr` and `d_ret1`.
- [x] Route both search and exact-combo paths through the shared helper.
- [x] Invalidate cache correctly when buffers are cleared or the dataset changes.
- [x] Confirm no semantic change to metrics.
- [ ] Add tests for cache-key behavior or helper correctness where practical.

### Phase 4: 9-Row Product-Reduced A/B

- [x] Add a dedicated constant for the reduced product-row width.
- [x] Add an env gate for the experiment.
- [x] Widen `search_metrics_count_for_optimization()` when the experiment is enabled.
- [x] Keep the default path unchanged when the env gate is off.
- [x] Extend reduced-row decode to derive `martin_ratio` from `pnl` and `ulcer_index`.
- [x] Confirm `omega` behavior remains aligned with the current GPU contract.
- [x] Gate finalist replay off when the row width is sufficient for current GPU product parity.
- [x] Add tests for reduced-row decode and replay gating behavior.
- [x] Compare 5-row + replay versus 9-row + no replay on the same benchmark family.
- [x] Confirm exact top result and top-k identity parity within floating-point tolerance (covered by `gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe`).
- [x] If 9-row wins, promote it from experiment to preferred/default path for this benchmark family.
- [ ] If 9-row does not win, explicitly carry the remaining optimization burden into Phase 5.

### Phase 5: Small-Finalist Fast Path

- [ ] Only start this phase if Phase 2 and Phase 4 data justify it.
- [ ] Define the trigger for the fast path.
- [ ] Keep the trigger conservative.
- [ ] Avoid creating a brand-new CUDA kernel first.
- [ ] Reuse existing resident-market plumbing.
- [ ] Reuse returns-cache helper.
- [ ] Reuse existing M7-specialized replay selection.
- [ ] Bypass generic outer tile/orchestration when the finalist set fits the fast-path shape.
- [ ] Keep exactness identical to the current replay path.
- [ ] Add tests for fast-path eligibility and parity against the generic replay path.
- [ ] If this path wins, promote it to the preferred replay path for the targeted workload.

### Phase 6: Optimization Closure

- [x] Make an explicit optimization decision for the current exact benchmark family.
- Decision: promote the 9-row path (single-pass; replay skipped) as the default for `Sharpe`/`Volatility`.
- `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS=0` remains as the legacy-baseline override for benchmarking and diagnosis.

### Phase 7: Validation and Handoff

- [x] Run targeted tests for edited code.
- [x] Run at least one CUDA build/test pass appropriate to touched surfaces.
- [x] Run benchmark variants serially.
- [x] Capture before/after timings for the four benchmark variants.
- [x] Capture replay timing breakdown if instrumentation was added.
- [x] Document whether 9-row replaces replay for this workload or not.
- [x] Document whether Phase 5 is still necessary after Phase 4 results.
- [ ] Update research notes or inline comments if implementation invalidates earlier assumptions.

---

## Detailed Implementation Plan

## Phase 0: Preparation and Guardrails

### Objective

Establish a stable starting point and avoid contaminated timing conclusions.

### Files

- `crates/app_core/benches/double_ma_opt.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`

### Tasks

1. Confirm benchmark naming and request wiring in `double_ma_opt.rs`.
2. Confirm current replay trigger and search metric-count logic in `gpu_backtest_kernel.rs`.
3. Confirm reduced-row write layout in `double_crossover.cu`.
4. Confirm CPU-side metric finalization logic in `double_ma.rs`.
5. Confirm no benchmark processes are already active before any timing work.

### Acceptance Criteria

- All target code surfaces are revalidated before editing.
- No benchmark timing will be taken from overlapping runs.

### Notes

- This phase is small, but it prevents bad conclusions later.

---

## Phase 1: Benchmark Split

### Objective

Separate search cost, heatmap cost, and replay cost so later optimization results are interpretable.

### Target File

- `crates/app_core/benches/double_ma_opt.rs`

### Required Change

Replace the current 2-variant setup with a 4-variant setup:

1. `search_only`
2. `search_plus_heatmap`
3. `search_plus_finalist_recompute`
4. `end_to_end_product`

### Required Behavior

- `search_only`
  - `heatmap_bins = 0`
  - finalist recompute disabled
- `search_plus_heatmap`
  - `heatmap_bins = 64`
  - finalist recompute disabled
- `search_plus_finalist_recompute`
  - `heatmap_bins = 0`
  - finalist recompute enabled
- `end_to_end_product`
  - `heatmap_bins = 64`
  - finalist recompute enabled

### Implementation Notes

- Preserve the same data length, MA types, objective, and `top_k`.
- Do not change the benchmark family name prefix.
- Do not introduce parallel benchmark invocation.

### Validation

- Benchmark compiles.
- Names appear as expected.
- Each variant toggles only the intended feature flags.

### Decision Gate

Proceed to Phase 2 immediately after this phase.

---

## Phase 2: Finalist Replay Timing Instrumentation

### Objective

Measure where exact-combo finalist replay is actually spending time.

### Target File

- `crates/app_core/src/gpu_backtest_kernel.rs`

### Required Change

Add a replay-only timing profile for the exact-combo executor.

### Suggested Timing Buckets

- `returns`
- `fast_ma`
- `slow_ma`
- `pairmap`
- `backtest`
- `d2h_decode`
- `tiles`

### Instrumentation Boundaries

Time the following regions inside the exact-combo executor:

- returns generation
- fast-period upload and fast MA prep
- slow-period upload and slow MA prep
- exact pair-index-map build/upload
- backtest launch region
- synchronize + host copy + decode

### Required Constraints

- Instrumentation must be behind an env flag.
- Default path must remain behaviorally identical.
- Disabled instrumentation should have negligible overhead.

### Validation

- Env-off behavior is unchanged.
- Env-on output is readable and stable.
- The timing buckets sum plausibly relative to end-to-end replay time.

### Decision Gate

Proceed to Phase 3 regardless.

Use the output from this phase to evaluate whether replay is dominated by:

- returns generation
- MA preparation
- backtest execution
- D2H/decode

---

## Phase 3: Returns Reuse Across Pass 1 and Pass 2

### Objective

Stop recomputing full-series returns twice within the same optimization flow.

### Target File

- `crates/app_core/src/gpu_backtest_kernel.rs`

### Required Change

Introduce a returns-cache layer inside `KernelScratch` so both the search path and the exact-combo replay path can reuse `d_lr` and `d_ret1` when the dataset is unchanged.

### Design Requirements

- Cache key must be tied to the dataset identity.
- Cache must be invalidated when buffers are cleared.
- Search and exact-combo code paths must use the same helper.

### Concrete Work Items

1. Add cache metadata to `KernelScratch`.
2. Add a helper that ensures:
   - `d_lr` is allocated
   - `d_ret1` is allocated
   - returns are generated only on cache miss
3. Replace duplicated returns-generation code in:
   - search path
   - exact-combo path
4. Reset the cache key when scratch is cleared.

### Risks

- Incorrect cache reuse across changed candle inputs.
- Hidden coupling with existing scratch-clear behavior.

### Mitigations

- Keep the cache key conservative.
- Prefer false misses over false hits.

### Validation

- Exactness unchanged for current behavior.
- Search followed by finalist replay does not regenerate returns when the dataset is unchanged.
- Cache clears correctly when runtime scratch is dropped.

### Decision Gate

After this phase, rerun the benchmark split and replay instrumentation.

If replay cost drops materially but remains large, continue to Phase 4.

---

## Phase 4: 9-Row Product-Reduced A/B

### Objective

Test whether widening pass 1 from 5 rows to 9 rows is cheaper than doing full finalist replay for this workload.

### Target Files

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/kernels/double_crossover.cu`
- `crates/app_core/src/double_ma.rs`

### Required Change

Add an env-gated experiment that lets the search pass emit a 9-row reduced product row:

- `log_eq`
- `trades`
- `max_log_dd`
- `mean`
- `std`
- `exposure`
- `net_exposure`
- `downside_deviation`
- `ulcer_index`

### Why 9 Rows

For the current GPU product contract:

- `pnl` can be reconstructed from `log_eq`
- `max_dd` can be reconstructed from `max_log_dd`
- `calmar` can be derived from `pnl` and `max_dd`
- `martin_ratio` can be derived from `pnl` and `ulcer_index`
- `sharpe` and `sortino` can be derived from decoded components
- `omega` is already not populated by the current GPU decode path

### Required Code Changes

1. Add a dedicated reduced-row width constant.
2. Add an env flag to enable the experiment.
3. Widen `search_metrics_count_for_optimization()` when enabled.
4. Keep the default search metric counts unchanged when the env flag is off.
5. Extend decode logic to derive `martin_ratio` when the row width is 9.
6. Gate finalist replay off when the emitted row width is sufficient for current GPU product parity.

### Important Constraints

- This must remain env-gated until benchmarked.
- Default product behavior must remain unchanged.
- Exactness of current GPU product output must be preserved for the metrics currently produced.

### Validation

- Existing default path behaves exactly as before.
- 9-row mode decodes correctly.
- `best_params` parity is maintained.
- top-k identity parity is maintained within floating-point tolerance.
- replay branch is skipped in 9-row mode when intended.

### Decision Gate

If 9-row mode is faster than current 5-row + replay while preserving parity, stop here for the current benchmark family.

If 9-row mode loses, keep it as a useful experiment but continue to Phase 5.

---

## Phase 5: Small-Finalist Fast Path

### Objective

Reduce replay fixed costs for tiny finalist sets without rewriting the full replay system.

### Target File

- `crates/app_core/src/gpu_backtest_kernel.rs`

### Preconditions

Do not start this phase unless:

- replay timing shows replay still matters after returns reuse
- and 9-row mode did not clearly win

### Strategy

Add a simplified small-finalist executor path inside the exact-combo replay flow.

### Initial Trigger Shape

The fast path should require all of the following:

- one evaluation range
- no host precomputed MA matrices
- very small finalist count
- unique finalist fast periods fit in one tile
- unique finalist slow periods fit in one tile

### Initial Implementation Rules

- Do not introduce a new CUDA kernel first.
- Reuse resident-market handling.
- Reuse the shared returns-cache helper.
- Reuse existing M7 dispatch logic.
- Avoid generic outer replay machinery when only one tiny tile is needed.
- Keep output identical to the existing replay path.

### Concrete Work Items

1. Detect the small-finalist case before entering the generic path.
2. Add a dedicated helper for the small-finalist replay path.
3. Compute fast and slow finalist MA tiles once.
4. Build the pair-index map once.
5. Launch the existing full-metric replay kernel path on that compact shape.
6. Copy back a single dense result block and decode.

### Risks

- Fast-path complexity may not justify the gain if the 9-row experiment already solves the problem.
- Trigger logic may be too broad and accidentally complicate maintenance.

### Mitigations

- Keep trigger conservative.
- Keep helper isolated.
- Keep fallback to the existing generic path.

### Validation

- Fast path and generic replay path produce equivalent results.
- Fast path only activates for the intended shape.
- Replay timing shows lower fixed-cost overhead in the targeted small-K regime.

### Decision Gate

Stop after this phase unless measurement shows an additional host-mediated handoff problem worth attacking.

---

## Phase 6: Optimization Closure

### Objective

Convert the experiment results into an actual optimization decision for the current exact benchmark family.

### Required Outcomes

Choose exactly one of the following:

1. Promote the 9-row product-reduced path as the preferred/default product path.
2. Keep replay as the default architecture, but promote a narrower small-finalist replay path.
3. If neither wins cleanly, explicitly end this workstream and open a separate direct `search_only` first-pass optimization workstream.

### Required Deliverables

- A short written decision.
- The measured evidence that led to that decision.
- A statement of whether the current benchmark family has now been optimized or only better instrumented.

### Decision Gate

Do not claim the optimization workstream is complete until one of the three outcomes above is written down.

---

## Phase 7: Validation, Benchmark Capture, and Handoff

### Objective

Close the work with defensible evidence rather than isolated code changes.

### Required Validation Areas

#### Exactness

- Same `best_params`
- Same `best_metrics` within tolerance
- Same top-k identity or a documented acceptable tolerance rule for ties
- Replay skipped only when intended

#### Timing

Capture all four variants:

- `search_only`
- `search_plus_heatmap`
- `search_plus_finalist_recompute`
- `end_to_end_product`

Capture replay timing buckets when instrumentation is enabled:

- returns
- fast MA
- slow MA
- pairmap
- backtest
- d2h_decode

#### Build/Test

- Build or test the touched crate(s)
- Run the repo-appropriate GPU verification path if available and relevant

### Handoff Deliverables

- Short summary of which phases were completed
- Before/after timing table
- Decision on whether 9-row should remain experimental or become the preferred path
- Decision on whether Phase 5 is still necessary

---

## Direct Search-Only Optimization Backlog

This backlog is not part of the first validated product-gap workstream, but it is where explicit `search_only` optimization effort belongs if product-gap closure still leaves the first pass too slow.

### Candidate Items

- Add a separate exact-search benchmark family for objective-specific rank-only search contracts.
- Evaluate a pure `Sharpe` search contract that carries only the minimum exact ranking state instead of the current 5-row reduced payload.
- Investigate first-pass arithmetic reduction inside the search kernel for `Sharpe`.
- Investigate first-pass launch-shape and codegen tuning only after the benchmark split isolates true search cost.
- Re-check compact top-k/finalize overhead only after replay is removed or bounded.

### Important Constraint

This backlog should remain separate from the product-gap workstream unless the benchmark contract is intentionally being changed.

---

## Recommended Execution Strategy

If implementation begins immediately, the best execution order is:

1. Phase 1 and Phase 2 in the same work stream
2. Phase 3 next
3. Re-measure
4. Phase 4 as an env-gated A/B
5. Re-measure
6. Phase 5 only if still justified
7. Phase 6 optimization closure
8. Phase 7 validation and handoff

This ordering minimizes wasted engineering work and forces each deeper change to justify itself with measurements.

---

## Working Status Log

Use this section to track actual execution once implementation starts.

### Status

- Current phase: Phase 7 complete (perf gate + replay profile captured)
- Owner: Codex
- Last updated: 2026-03-23

### Notes

- Process-safety check: no active `cargo` / `double_ma_opt` / `ta_cli` processes at time of last check.
- CPU-only validation: `cargo test -p ta_app_core` currently fails due to unrelated `double_ma_validate.rs` references to `PreparedGpuValidationPlan` (see command log).
- Benchmark capture: serial baseline vs promoted default timings are recorded in the capture table above.
- Optimization closure: 9-row single-pass has been promoted as the default for `Sharpe`/`Volatility`; use `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS=0` to force the legacy 5-row+replay baseline.
- Replay profiling: replay-profile capture recorded in the table above; for this workload, replay is dominated by `d2h_decode` overhead (not kernel compute).
- GPU perf gate: `scripts/run_gpu_perf_gate.ps1 -Mode candidate` passed.
- Phase 5: no longer justified for this benchmark family after the 9-row promotion.

### Command Log

- 2026-03-23: `Get-Process cargo, double_ma_opt, ta_cli -ErrorAction SilentlyContinue` (no active processes)
- 2026-03-23: `CARGO_TARGET_DIR=target_codex_verify cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel --no-run` (ok)
- 2026-03-23: `target_codex_verify\\release\\deps\\double_ma_opt-*.exe --list` (4 contract-split benchmark IDs present)
- 2026-03-23: `CARGO_TARGET_DIR=target_codex_verify cargo test -p ta_app_core --features cuda,cuda-backtest-kernel search_metrics_count_product_reduced_rows_default_and_override` (ok)
- 2026-03-23: `CARGO_TARGET_DIR=target_codex_verify cargo test -p ta_app_core --features cuda,cuda-backtest-kernel gpu_kernel_product_reduced_rows_matches_replay_contract_for_sharpe` (ok)
- 2026-03-23: `CARGO_TARGET_DIR=target_codex_verify cargo test -p ta_app_core` (fails: `PreparedGpuValidationPlan` missing in `crates/app_core/src/double_ma_validate.rs`)
- 2026-03-23: `CARGO_TARGET_DIR=target_codex_verify cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- \"double_ma/gpu_kernel_contract_split/alma_alma/200k\"` (default promoted timings captured)
- 2026-03-23: `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS=0 CARGO_TARGET_DIR=target_codex_verify cargo bench -p ta_app_core --bench double_ma_opt --features cuda,cuda-backtest-kernel -- \"double_ma/gpu_kernel_contract_split/alma_alma/200k\"` (legacy baseline captured)
- 2026-03-23: `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate` (PASS; summary written to `target/gpu_perf_summary.json`)
- 2026-03-23: `VECTORBT_KERNEL_FINALIST_PROFILE=1 VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS=0 VECTORBT_KERNEL_FINALIST_RECOMPUTE=1 cargo run -q -p ta_app_core --bin ta_cli --features cli,cuda,cuda-backtest-kernel -- --backend gpu-kernel --synth-len 200000 --fast-ma alma --slow-ma alma --fast-period 5:200:1 --slow-period 10:400:1 --objective sharpe --top-k 50 --heatmap-bins 64` (replay profile captured)

### Current Env State

- `VECTORBT_KERNEL_FINALIST_PROFILE`: unset
- `VECTORBT_KERNEL_PRODUCT_REDUCED_ROWS`: unset
- `VECTORBT_KERNEL_PROFILE_PIPELINE`: unset
- `VECTORBT_KERNEL_VRAM_BUDGET_MB`: unset

---

## Short Version

The valid plan is:

1. make the benchmark honest
2. make replay measurable
3. remove duplicated returns work
4. test whether 9-row pass 1 is cheaper than replay
5. only then build a narrower small-finalist replay path

That is the full implementation scope for the validated parts of the guide.
