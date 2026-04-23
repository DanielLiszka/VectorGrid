# TM-Native MA Expansion Direction (Final)

## 1. Executive summary

This repo should **not** start a broad TM-native MA expansion campaign.

The adopted direction is a **single bounded VWMA pilot** targeted at the current `double_ma` runtime, with the goal of eliminating the row-major-plus-transpose path for **`vwma/vwma`** first and only on the existing TM-layout contract. The pilot should stay inside the current pair-level fast-path structure, keep `transpose_row_to_tm` as the fallback, keep hybrid/precomputed CPU-staged behavior intact, and leave `double_crossover.cu` unchanged in the first wave. This matches the prompt-pack framing: Prompt 2 is still a plausible product-upside lane, but it is **not** the best next lane for immediate `alma/alma` benchmark speed, so the work has to justify itself on non-ALMA product surfaces instead of the headline benchmark. fileciteturn9file6 fileciteturn9file9

The key repo reason for this narrow recommendation is that the current Phase 2 bundle still says the highest-value immediate performance work sits elsewhere: retained `large` and shortlist-heavy `xlarge` truth surfaces are still dominated by the backtest-family kernels and replay follow-on work, not by a proved TM-prep bottleneck. That means this lane should be treated as a **coverage-and-architecture pilot**, not as the next headline performance bet. fileciteturn9file2 fileciteturn9file3 fileciteturn9file4

## 2. Current repo MA-path summary

The current runtime already has a real VRAM-resident MA path. `gpu_backtest_kernel.rs` builds `VramMaInputs` from resident device market buffers and then prepares MA tiles on device before the backtest kernels run. The important split is not “GPU vs CPU”; it is **TM-native writer vs row-major writer plus transpose**. fileciteturn9file7

Today the direct TM-native fast path is intentionally narrow. `kernel_fast_path_enabled(...)` returns true only when **both** sides are `sma` or `alma`, and the execution paths then use `compute_period_ma_tm_into(...)`. When that gate is false, the runtime uses `compute_period_ma_into(...)` and then launches `transpose_row_to_tm` before the backtest kernels consume the result. In other words, the current repo bottleneck is not missing device residency; it is the narrow TM-native admission rule. fileciteturn9file7

The repo also already supports a hybrid/precomputed route. In `double_ma.rs`, the hybrid path decides whether an indicator must be CPU-staged, builds row-major host matrices with `ma_matrix_cpu_f32_staging(...)`, wraps them as `PrecomputedHostMaMatrix`, and hands them to the kernel executor, which then uploads packed TM-form rows internally. The tests in `double_ma.rs` also show that precomputed FRAMA matrices and CPU-staged kernel-capable MAs already stay on the GPU path when intended. This means there is already a working fallback architecture for unsupported or policy-staged MAs. fileciteturn9file5

## 3. Final recommendation

Adopt this concrete direction:

**Run one VWMA-only TM-native pilot for `vwma/vwma`, using the current runtime ownership model and keeping the existing pair-level fast-path structure.**

That means:

- first confirm whether the current `vector-ta` surface already allows `ma.compute_period_ma_tm_into("vwma", ...)` under the repo’s pinned `vector-ta = 0.2.4` dependency;
- if it does, make the smallest runtime change needed in `gpu_backtest_kernel.rs` so `vwma/vwma` can use the TM-native writer instead of row-major compute plus `transpose_row_to_tm`;
- if it does **not**, allow only a **small, version-aligned** `vector-ta` extension that adds TM-native VWMA output, while keeping workspace versions aligned across `crates/app_core` and `src-tauri`, as required by current repo policy and Cargo wiring;
- do **not** start with per-side capability gating, an EMA/WMA wave, or a broader VRAM MA redesign. fileciteturn9file0 fileciteturn9file1 fileciteturn9file7 fileciteturn9file12

VWMA is the right pilot because the repo already has explicit VWMA-specific device prep through `ensure_vwma_prefix_pv_vol_f64(...)`, and the frontend/product surface already makes VWMA a real user-facing family: `VFCT` only allows VWMA fast/slow trend lines, and `MARRT` currently allows `sma`, `ema`, `alma`, `wma`, `zlema`, and `vwma`. That gives the repo an immediately meaningful non-ALMA surface without inventing a new product route. fileciteturn9file7 fileciteturn9file8 fileciteturn9file13

This recommendation is intentionally narrower than a general “expand TM-native preparation” plan. The repo evidence supports a pilot; it does **not** yet support a multi-family rollout.

## 4. Explicitly retained current strengths

Keep these strengths unchanged in the first wave:

- The current `sma` / `alma` TM-native path stays exactly as-is. That path already underpins the repo’s public benchmark story, which remains anchored on `Double MA`, `ALMA/ALMA`, exact grid, and warm app-path GPU execution. fileciteturn9file6 fileciteturn9file10
- The backtest kernels continue to consume TM-layout inputs. The first wave should keep `transpose_row_to_tm` as the fallback rather than pushing layout complexity into `double_crossover.cu`. fileciteturn9file7
- The hybrid/precomputed CPU-staging route stays intact. That route already exists in `double_ma.rs`, and existing tests prove it can keep CPU-staged kernel-capable indicators on the GPU evaluation path. fileciteturn9file5
- CPU-only builds and current CUDA feature wiring remain unchanged. Repo policy explicitly says to keep CPU-only builds working, and the current Cargo wiring already forwards CUDA features through `ta_app_core` and `vector-ta`. fileciteturn9file0 fileciteturn9file1 fileciteturn9file12

## 5. Explicitly deferred ideas

These ideas stay deferred until the VWMA pilot wins:

1. **Mixed-pair TM-native expansion** such as `sma/vwma` or `alma/vwma`.
   The repo should not start with a broader admitted set until `vwma/vwma` proves that the direct TM-native route matters on a real product surface. fileciteturn9file7

2. **EMA / WMA / ZLEMA follow-on work for MARRT.**
   The frontend already admits those families for MARRT, but the attached runtime evidence is strongest for VWMA because VWMA already has explicit device-side prep. WMA also already carries GPU-kernel-specific period restrictions (`> 1`), which is a concrete sign that later-family rollout is not “free.” fileciteturn9file7 fileciteturn9file8

3. **Per-side capability gating.**
   A per-side gate may become worthwhile later, but it is not the right opening move. Starting there would optimize architectural cleanliness before proving product payoff. fileciteturn9file6

4. **Broader VRAM MA coverage or tile caching work.**
   Those are real longer-range ideas, but the current repo evidence does not justify merging them into this first TM-native expansion step. fileciteturn9file4

## 6. Explicitly rejected ideas

Reject these ideas for this lane right now:

1. **A broad `vector-ta` / VRAM MA redesign.**
   The repo already has a working VRAM-resident path and a working hybrid/precomputed fallback. A large indicator-surface rewrite would add cross-crate maintenance cost before one family has proved payoff. Repo policy also favors small, surgical changes over unrelated refactors. fileciteturn9file0 fileciteturn9file7

2. **Changing transpose ownership inside the hot backtest kernels.**
   The first-wave problem is admission to the TM-native writer, not the TM-layout consumer. Changing the consumer or supporting dual layouts in `double_crossover.cu` is too invasive for a first probe. fileciteturn9file7

3. **Frontend/request-builder redesign in the first wave.**
   `VFCT` and `MARRT` already expose the relevant MA families on the product side. This should remain a backend-only improvement first. fileciteturn9file8 fileciteturn9file13

## 7. Concrete code touch points by file and symbol

### Primary runtime ownership

`crates/app_core/src/gpu_backtest_kernel.rs` is the main implementation surface. The key symbols are:

- `kernel_fast_path_enabled(...)`
- the `use_fast_path` decisions in both optimization and evaluation paths
- `VramMaInputs`
- `ensure_sma_prefix_f64(...)`
- `ensure_vwma_prefix_pv_vol_f64(...)`
- `compute_period_ma_tm_into(...)`
- `compute_period_ma_into(...)`
- `transpose_row_to_tm`
- the `supports_vram_kernel_ma(...)` / WMA guard checks

That is where the repo decides whether to write TM-native MA tiles directly or to fall back to row-major-plus-transpose. fileciteturn9file7

### Fallback and parity ownership

`crates/app_core/src/double_ma.rs` is the main non-regression surface. The key symbols are:

- `ma_matrix_cpu_f32_staging(...)`
- `PrecomputedHostMaMatrix`
- the call sites that build precomputed host matrices and pass them into the kernel executor
- the tests:
  - `gpu_kernel_precomputed_frama_matrix_executes`
  - `gpu_hybrid_cpu_staged_kernel_capable_ma_stays_on_gpu_path`
  - `gpu_hybrid_vdps_cpu_staged_kernel_capable_ma_stays_on_gpu_path`

Those are the right places to protect hybrid/precomputed behavior while the VWMA pilot lands. fileciteturn9file5

### Dependency/version guardrail

If the pilot needs a new TM-native VWMA writer in `vector-ta`, the dependency guardrail lives in:

- `crates/app_core/Cargo.toml`
- `src-tauri/Cargo.toml`

Both currently pin `vector-ta = "0.2.4"`, and repo policy says to keep that version aligned across workspace crates. fileciteturn9file0 fileciteturn9file1 fileciteturn9file12

### Expected no-change files for stage 1

These files should stay unchanged in the first wave unless the pilot unexpectedly exposes a deeper problem:

- `crates/app_core/src/kernels/double_crossover.cu`
- `web/src/services/optimizeRequestBuilder.ts`
- `web/src/utils/strategyOptimizationCapabilities.ts`

The CUDA consumer layout and the frontend capability policy are not the problem being tested first. fileciteturn9file7 fileciteturn9file8 fileciteturn9file13

## 8. Staged implementation order

### Stage 0 — feasibility probe

Confirm whether current `vector-ta 0.2.4` already supports `compute_period_ma_tm_into("vwma", ...)` under the repo’s existing runtime surface. This is a compile/API check plus a tiny deterministic runtime check, not a performance project yet. fileciteturn9file1 fileciteturn9file7 fileciteturn9file12

### Stage 1 — narrow VWMA runtime pilot

If Stage 0 succeeds, make the smallest `gpu_backtest_kernel.rs` change that lets **`vwma/vwma`** use the TM-native route instead of row-major compute plus `transpose_row_to_tm`.

Implementation rule: do **not** introduce a generalized per-side capability framework here. Keep the current pair-level fast-path style and admit only the VWMA pilot surface first. Reuse the existing VWMA prefix prep. fileciteturn9file7

If Stage 0 fails because the current `vector-ta` surface lacks TM-native VWMA output, the only acceptable escalation is a **surgical, version-aligned** `vector-ta` addition for VWMA. If the required change grows beyond that, stop and defer the lane. fileciteturn9file0 fileciteturn9file1 fileciteturn9file12

### Stage 2 — parity and regression hardening

Add or update tests to prove:

- VWMA TM-native output matches the existing row-major-plus-transpose result on a deterministic candle set;
- missing-volume behavior still errors correctly for VWMA;
- precomputed/hybrid routes remain intact;
- current ALMA/SMA fast-path behavior is unchanged. fileciteturn9file5 fileciteturn9file7

### Stage 3 — benchmark the real pilot surfaces

Benchmark the new path on the surfaces that actually matter for this lane:

- a plain `DoubleMa` `vwma/vwma` exact-grid surface;
- a `VFCT` `vwma/vwma` exact-grid surface;
- optionally a `MARRT` `vwma/vwma` exact-grid surface.

Use the README `ALMA/ALMA` warm app-path and Criterion anchor only as regression sentinels, not as the payoff surface for this work. fileciteturn9file6 fileciteturn9file8 fileciteturn9file10 fileciteturn9file13

### Stage 4 — only if the pilot wins

Only after a clean VWMA win should the repo consider:

- mixed `sma/alma/vwma` fast-path admission,
- then one MARRT-family follow-on such as EMA or WMA,
- then, only later, any per-side gating cleanup.

Do not schedule those follow-ons before the VWMA pilot clears parity and performance gates. fileciteturn9file6 fileciteturn9file8

## 9. Benchmark and parity-validation plan

### Benchmark order

1. **Regression sentinels**
   - README-style `Double MA`, `ALMA/ALMA`, exact-grid warm app-path surface
   - Criterion `double_ma/gpu_kernel/alma_alma/200k/large/58300pairs`

2. **Pilot payoff surfaces**
   - `DoubleMa` `vwma/vwma`
   - `VFCT` `vwma/vwma`
   - optional `MARRT` `vwma/vwma`

3. **Escalation surface only if needed**
   - shortlist-heavy `xlarge` on a non-ALMA VWMA surface, but only after the large exact-grid result is clear

This keeps the benchmark plan aligned with the prompt-pack intent: do not infer payoff from the existing ALMA anchor only. fileciteturn9file6 fileciteturn9file10

### What to measure

Use the existing timing payloads already surfaced by the repo, especially:

- `gpu_kernel_timing_ms.route_wall_ms`
- `gpu_kernel_timing_ms.gpu_indicator_prep_ms`

The first proof target is a drop in **indicator-prep time** on the VWMA pilot surface. Route wall time should move with it if the lane is real. If the first pass is ambiguous, then refresh dispatch/pipeline logs or run a narrow Nsight Systems capture on the pilot surface only. fileciteturn9file5 fileciteturn9file7

### Verification steps

- `cargo test -p ta_app_core`
- `powershell -File scripts/run_gpu_perf_gate.ps1 -Mode candidate`
- `powershell -File scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate`

Repo policy also requires serialized benchmark/profiling runs and favors using the existing perf gates before drawing conclusions. fileciteturn9file0 fileciteturn9file10

## 10. Rejection criteria and stop conditions

Stop or reject this lane under any of these conditions:

1. **API/scope stop:** `compute_period_ma_tm_into("vwma", ...)` is not available, and enabling it requires a broad `vector-ta` redesign or workspace-wide dependency churn rather than a surgical addition. fileciteturn9file0 fileciteturn9file1 fileciteturn9file12

2. **Performance stop:** the VWMA pilot does not show a clear, repeatable reduction in `gpu_indicator_prep_ms` and does not move route wall time on the non-ALMA pilot surfaces. In that case, the repo should keep the current `sma` / `alma` TM-native scope and spend optimization budget elsewhere. fileciteturn9file2 fileciteturn9file3 fileciteturn9file4

3. **Regression stop:** the change regresses the current `ALMA/ALMA` benchmark sentinel or fails `scripts/run_gpu_perf_gate.ps1 -Mode candidate`. fileciteturn9file0 fileciteturn9file10

4. **Parity stop:** best params, top-k ordering, metric rows, or VWMA missing-volume behavior diverge from the existing fallback path. fileciteturn9file5 fileciteturn9file7

5. **Scope-creep stop:** the work starts to require `double_crossover.cu` consumer changes, frontend/request-builder changes, or a generalized multi-family architecture pass before one family has proved value. fileciteturn9file7 fileciteturn9file8 fileciteturn9file13

## Bottom line

The final repo direction is:

**Do one bounded `vwma/vwma` TM-native pilot in `gpu_backtest_kernel.rs`, keep the current TM consumer and transpose fallback intact, keep hybrid/precomputed behavior intact, and require clear non-ALMA route payoff before any broader TM-native MA expansion is allowed.** fileciteturn9file6 fileciteturn9file7
