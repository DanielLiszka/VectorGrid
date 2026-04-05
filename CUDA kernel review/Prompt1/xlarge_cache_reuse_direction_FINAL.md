# xlarge_cache_reuse_direction_FINAL

Prepared for Prompt 1 in the current `double_ma` backtest performance prompt pack, using the repo’s prompt bundle and routing constraints from `AGENTS.md` and `REPO_MAP.md`. fileciteturn12file10 fileciteturn12file12 fileciteturn12file0 fileciteturn12file14

## 1. Executive summary

The adopted direction is **not** an immediate cache-policy or kernel rewrite. The repo should run one **measurement-first, `xlarge`-only cache-fit tiling probe** on the exact `double_ma` `alma/alma` route, with `xlarge/full17` as the primary truth surface. The first implementation step is to use the existing `VECTORBT_KERNEL_VRAM_BUDGET_MB` and `VECTORBT_KERNEL_TILE_DEBUG` controls to force smaller tiles and test whether shrinking the hot TM working set improves both the main lockstep kernel’s cache behavior and route wall time. Only if that probe is positive should the repo add an `xlarge`-specific cache-fit cap inside the existing tile chooser. fileciteturn12file10 fileciteturn12file2 fileciteturn12file4 fileciteturn14file6 fileciteturn13file17

The reason this wins now is narrow and repo-specific: `large` is already cache-friendly; the retained replay-batching branch already cut the main replay-side `xlarge` penalty; the remaining `xlarge/full17` problem still looks like a combination of **larger hot TM-state footprint** plus a **`full17`-specific store/access weakness**, not a clean “price data is missing from cache” story. fileciteturn12file2 fileciteturn12file5 fileciteturn12file6 fileciteturn12file4

## 2. Current repo evidence summary

The current cache measurements already close the `large` question. On `large`, the exact `alma/alma` lockstep kernels hold roughly `63.4%` L1/TEX hit rate and roughly `96.4%` to `97.0%` L2 hit rate across `auto`, `full17`, and `trade22`. On `xlarge`, cache stress is materially worse, with `xlarge/full17` the clear worst case at `36.44%` L1/TEX and `70.21%` L2, while `xlarge/trade22` is materially healthier at `38.84%` L1/TEX and `84.54%` L2. fileciteturn12file1 fileciteturn12file2

The runtime-attribution bundle still says the main absolute hotspot is the lockstep backtest family, not reducer/search-stage kernels. On shortlist-heavy `xlarge`, the surfaced secondary GPU cost was the `windowed_m17` / `windowed_m22` replay family, not the reducer path, and the retained replay-batching pass already collapsed that replay bucket to about `28 ms` on both retained contracts. fileciteturn12file6 fileciteturn12file7 fileciteturn12file5

Nsight Compute also still points away from a spill story. The retained hot lockstep kernels show no local or shared spilling on `large` or `xlarge`. `trade22` continues to preserve materially better `global_store_bytes_per_sector` than `full17`, including on `xlarge`, even though `trade22` is the wider contract. That makes a pure “row width caused the cache miss” explanation weak. fileciteturn12file4 fileciteturn12file6

The runtime already has meaningful reuse surfaces before any new cache work is added. The warm app-path benchmark explicitly reuses resident market-series buffers. In the CUDA runtime, `CudaKernelRuntime` already keeps resident market state, valid-pair maps, and persistent scratch, and `ensure_log_and_simple_returns(...)` reuses cached `d_lr` and `d_ret1` keyed by dataset signature. fileciteturn15file1 fileciteturn15file7 fileciteturn15file0

The tile logic is also already explicit. `estimate_tile_bytes(...)` budgets not only inputs, but also the fast/slow TM MA tiles and the per-pair metrics tile. `choose_tiles(...)` is driven by `vram_budget_bytes()`, and `VECTORBT_KERNEL_TILE_DEBUG=1` already reports the chosen tile and the full-fit estimate. That means the repo already has the machinery needed for a no-new-kernel feasibility probe. fileciteturn13file0 fileciteturn13file8 fileciteturn13file17

## 3. Final recommendation

### Adopted direction

Adopt a **measurement-gated, `xlarge`-only cache-fit tiling probe** for the exact `double_ma` `alma/alma` route, with `xlarge/full17` as the primary target and `xlarge/trade22` as the contrast case.

The practical read is:

- treat `large` as solved for this prompt
- treat replay-side cleanup as already harvested for the first meaningful win
- do **not** ship any new cache-policy code yet
- first ask whether a smaller exact-search tile meaningfully improves the hot lockstep kernel on `xlarge/full17`

If the forced-smaller-tile probe clearly improves both cache behavior and route wall time, then the follow-on code change is narrow: add an `xlarge`-only cache-fit cap to `choose_tiles()` so the existing tile chooser stops optimizing only for VRAM fit and also avoids obviously cache-hostile working-set sizes on this surface. fileciteturn12file2 fileciteturn12file6 fileciteturn12file5 fileciteturn13file0 fileciteturn13file8

### Why this is the right single direction

The remaining `xlarge` cache stress looks more like **working-set growth in the TM MA tiles and per-pair metrics tile**, with a secondary `full17`-specific store/access penalty, than like a missing explicit cache policy for price arrays. That is the more plausible read because:

- the runtime already reuses resident market series and cached returns rather than rebuilding them every run; fileciteturn15file1 fileciteturn15file0
- the `alma/alma` route is already on the TM-native fast path, so a generic transpose/relayout story is weaker here than on non-fast-path MA families; fileciteturn14file1 fileciteturn14file6
- `trade22` is wider than `full17` but still caches better and stores more efficiently on `xlarge`, which points to `full17` access/store shape plus overall `xlarge` pressure, not just width. fileciteturn12file1 fileciteturn12file4

This is also more plausible than an immediate L2-persistence branch. NVIDIA’s current CUDA Programming Guide says persisting L2 works through an **access policy window over one contiguous global-memory region**, bounded by `accessPolicyMaxWindowSize`, backed by a limited set-aside portion of L2, and tuned with `hitRatio` to avoid thrashing. The guide also warns that benefits diminish once the volume of persisting accesses exceeds the set-aside capacity, and it recommends resetting persisting state after use. On Ada, the GeForce RTX 4090 table in NVIDIA’s Ada architecture whitepaper lists `73728 KB` of L2 for the card SKU, but the persisting-L2 mechanism still only protects selected windows, not an arbitrarily large heterogeneous working set. citeturn749344view0turn733287view0turn733287view3turn165026view1

It is also more plausible than a shared-memory rewrite as the next move. NVIDIA’s current programming guide describes shared memory as a user-managed scratchpad, but it also states that shared memory and L1 share the same physical resource on these devices and that cache-config choices are only preferences, not guarantees. That makes a software-managed cache rewrite a much higher-risk branch than a tile-sensitivity probe. citeturn749344view3turn749344view4

## 4. Explicitly retained current strengths

The repo should explicitly keep these strengths unchanged while this prompt is resolved:

1. **Keep the current `large` route behavior unchanged.** The cache evidence says `large` is already in a strong regime, so Prompt 1 should not reopen `large`-side cache work. fileciteturn12file1 fileciteturn12file2
2. **Keep CPU fallback behavior unchanged.** That is a repo invariant for any backend/runtime work. fileciteturn12file0
3. **Keep exact finalist semantics unchanged.** Prompt 1 is not a candidate to reopen compact-finalist or replay-authority semantics. fileciteturn12file10 fileciteturn12file3
4. **Keep the retained bounded lockstep row-base change.** That pass already landed and passed the repo gates. fileciteturn12file3 fileciteturn12file8
5. **Keep the retained replay-batching implementation.** That already harvested the first meaningful shortlist replay win and should not be reopened in this prompt. fileciteturn12file5 fileciteturn12file7
6. **Keep the current `alma/alma` TM-native fast path unchanged during the probe.** The first probe is about tile size and reuse distance, not about widening MA-path scope. fileciteturn14file1 fileciteturn14file9

## 5. Explicitly deferred ideas

These ideas stay deferred unless the adopted tile-sensitivity probe fails in a way that clearly promotes them:

1. **Actual `xlarge` cache-fit code in `choose_tiles()`** — deferred until the measurement probe proves that smaller tiles help route time, not just hit rate. fileciteturn13file0 fileciteturn13file17
2. **Explicit persisting-L2 micro-benchmarks** — deferred to a later diagnostic branch, and only for one clearly contiguous hot buffer after the probe says tiling is insufficient. The current NVIDIA docs make this a narrow, carefully tuned mechanism rather than a broad “make the working set resident” switch. citeturn749344view0turn733287view0turn733287view3
3. **Shared-memory / software-managed cache rewrites** — deferred unless future counters isolate a small repeated access region whose occupancy trade is acceptable. citeturn749344view3turn749344view4
4. **Broader replay/dataflow architecture changes** — deferred out of Prompt 1 because that lane already exists as the larger remaining architecture question and outranks any cache micro-idea if the team wants the next step-function win. fileciteturn15file5 fileciteturn15file17 fileciteturn15file18

## 6. Explicitly rejected ideas

The repo should reject these as the next Prompt 1 action:

1. **Broad price-data persistence as a product change.** The runtime already reuses resident market series and cached returns, so this does not target the most plausible remaining footprint. fileciteturn15file1 fileciteturn15file0
2. **Generic “add a cache” work.** The repo already has resident market buffers, valid-pair-map caching, scratch persistence, and replay-side improvements; the open question is cache fit, not memoization in the abstract. fileciteturn15file7 fileciteturn15file16
3. **Another immediate kernel-body tweak in `double_crossover.cu`.** The retained lockstep row-base pass already captured the bounded body-local win that the current evidence justified. fileciteturn12file3 fileciteturn12file8
4. **Reducer-first or selector-first interpretation of the cache evidence.** Whole-process time still says reducer/search kernels are too small to justify becoming the default next move. fileciteturn12file6 fileciteturn12file3
5. **Broad PTX/SASS reopening.** The prompt pack and current repo plan both say not to reopen that lane as the default path. fileciteturn12file10 fileciteturn12file8

## 7. Concrete code touch points by file and symbol

### Primary measurement and conditional implementation surface

**`crates/app_core/src/gpu_backtest_kernel.rs`**

Use these symbols first:

- `vram_budget_bytes()`
  - existing route-level budget control, already wired to `VECTORBT_KERNEL_VRAM_BUDGET_MB`; this is the no-code control for the first probe. fileciteturn13file8
- `estimate_tile_bytes(...)`
  - current footprint model; it already includes fast/slow TM MA tiles and the per-pair metrics tile, which is exactly why it is the right place for any later cache-fit cap. fileciteturn13file0
- `choose_tiles(...)`
  - current tile chooser; this is the only place Prompt 1 should change if the probe is clearly positive. fileciteturn13file0 fileciteturn13file17
- `VECTORBT_KERNEL_TILE_DEBUG`
  - existing debug print inside the tile chooser; use it to record the full-fit estimate and the chosen tile without editing code. fileciteturn13file17
- `ensure_log_and_simple_returns(...)`
  - evidence that cached `d_lr` / `d_ret1` already exist, which weakens the case for a price-array persistence change. fileciteturn15file0
- `CudaKernelRuntime { resident_market, staged_market, valid_pair_maps, scratch, ... }`
  - evidence that reuse infrastructure already exists and should not be duplicated by Prompt 1. fileciteturn15file7 fileciteturn15file13

### Secondary surface for diagnostics only

**`crates/app_core/src/double_ma.rs`**

- No primary algorithm change is recommended here for Prompt 1.
- Only add route-diagnostic surfacing if the probe needs more explicit reporting of chosen tile sizes or the exact route surface used. The fast-path/product semantics should remain unchanged. fileciteturn12file10 fileciteturn14file17

### Explicit non-touch surface for phase 1

**`crates/app_core/src/kernels/double_crossover.cu`**

- No code change in the first branch.
- Prompt 1 should not start by editing the hot kernel body again. fileciteturn12file3 fileciteturn12file8

## 8. Benchmark and profiler plan

### Phase 1: no-code tile-sensitivity probe

Use the retained exact route and remove replay from the measurement surface:

- binary: `target/release/profile_gpu_dynamic_runner.exe`
- strategy: `double_ma`
- `--fast-ma alma`
- `--slow-ma alma`
- `--adx-mode off`
- `--workload xlarge`
- `--candle-shape synth`
- `--metric-contract full17`
- `--shortlist-slice-mode none`
- `--heatmap-bins 0`
- `--dynamic-mode off`
- `--cuda-graphs off`

This is the same exact-lockstep surface used by the cache-hit pass, which keeps replay/dataflow out of the main kernel cache readout. fileciteturn12file2

Run order:

1. **Baseline run with tile debug enabled**
   - set `VECTORBT_KERNEL_TILE_DEBUG=1`
   - capture the reported `full` tile size and chosen tile under the default budget. fileciteturn13file17
2. **Budget sweep using the existing budget knob**
   - use `VECTORBT_KERNEL_VRAM_BUDGET_MB` to force smaller tiles
   - choose three to five budgets that step below the full-fit threshold reported in step 1, so the run actually leaves the full-surface regime and begins shrinking `pf_tile` and/or `ps_tile` rather than merely restating the same tile. fileciteturn13file8 fileciteturn13file17
3. **Per-budget direct-runner capture**
   - keep a short repeated-run mean for route wall time
   - record dispatch/tile diagnostics for each point. fileciteturn12file6
4. **Per-budget Nsight Compute on the hot lockstep kernel**
   - keep `MemoryWorkloadAnalysis`
   - target `^double_cross_backtest_tm_flex_f32_lockstep_time_m17`
   - compare cache hit behavior and memory pressure at baseline vs the best-smaller-tile point. fileciteturn12file2
5. **Contrast and regression controls**
   - rerun the same sweep shape on `xlarge/trade22`
   - use `large/full17` as the regression control and ensure it stays in the already-good regime. fileciteturn12file1 fileciteturn12file2

### Phase 2: only if Phase 1 is clearly positive

If the smaller-tile probe is clearly beneficial, implement the narrow follow-on:

- add an `xlarge`-specific cache-fit cap in `choose_tiles()`
- keep the footprint math rooted in `estimate_tile_bytes(...)`
- leave other routes and contracts unchanged unless the same evidence repeats on them. fileciteturn13file0

## 9. Verification plan

### Verification for the no-code probe

The first verification layer is the artifact bundle itself:

- direct-runner JSON or logs for each budget point
- tile-debug output showing full-fit vs chosen tiles
- Nsight Compute `MemoryWorkloadAnalysis` for baseline and best-smaller-tile points
- a short surfaced comparison note against `xlarge/trade22` and `large/full17` fileciteturn12file2 fileciteturn12file3

### Verification for a promoted code change

If the probe is positive and a narrow tile-cap change is promoted, the minimum repo verification order is:

1. rerun exact `large/full17`
2. rerun exact `large/trade22`
3. rerun exact `xlarge/full17`
4. rerun exact `xlarge/trade22`
5. if the changed tile behavior touches shortlist-heavy paths in practice, rerun the retained shortlist-heavy `xlarge` truth surfaces as a guardrail
6. run `scripts/run_gpu_perf_gate.ps1 -Mode candidate`
7. run `scripts/run_gpu_alma_contract_rows_perf_gate.ps1 -Mode candidate` fileciteturn12file0 fileciteturn12file8 fileciteturn12file3

The parity rule remains unchanged:

- CPU fallback unchanged
- exact finalist semantics unchanged
- no silent change to shortlist diagnostics semantics. fileciteturn12file0 fileciteturn12file10

## 10. Rejection criteria and stop conditions

Reject the tile-cap idea **before implementation** if any of these happen in the no-code probe:

1. **Tile shrinkage does not actually occur.** If the forced budgets do not produce smaller `pf_tile` / `ps_tile`, the probe has not tested the hypothesis and should stop until the sweep is corrected. fileciteturn13file17
2. **Cache metrics do not move meaningfully on `xlarge/full17`.** If L1/TEX and L2 stay essentially flat as tiles shrink, the remaining problem is probably not reuse distance inside the current tile chooser. fileciteturn12file1
3. **Cache metrics improve but route wall time does not.** In that case, the extra tiling/prep overhead is canceling the gain and the repo should not promote a code change. fileciteturn12file6
4. **The effect is not portable across the relevant contrast case.** If `xlarge/full17` appears to improve but `xlarge/trade22` does not move in the same direction at all, the repo should treat the issue as more contract-specific than cache-fit-generic. fileciteturn12file1 fileciteturn12file4
5. **`large` regresses or becomes unstable.** Prompt 1 does not justify paying back benchmark-anchor regressions on the already-healthy `large` route. fileciteturn12file1 fileciteturn15file1
6. **The evidence continues to point more strongly at `full17` write/access shape than at cache fit.** If the store-side metrics remain the stable differentiator while cache-hit movement is weak, stop the cache lane and reclassify the remaining issue as a `full17` layout/store problem. fileciteturn12file4
7. **The win is too fragile.** A single hand-tuned budget point with no repeatability is not enough to justify product code. A practical promotion threshold is a repeatable low-single-digit-to-mid-single-digit route-wall win on `xlarge/full17`, with no meaningful regression on the `large` controls. This threshold is a decision rule for this branch, not a repo-proved fact. fileciteturn12file6 fileciteturn12file1

## Bottom line

For Prompt 1, the repo should make exactly one decision:

- **Adopt now:** a measurement-first `xlarge` tile-sensitivity probe using the existing VRAM-budget and tile-debug controls.
- **Defer:** actual `xlarge` cache-fit tile-capping code until the probe is clearly positive.
- **Reject:** broad price-data persistence, generic cache work, another immediate kernel-body pass, and reducer-first interpretations of the current evidence. fileciteturn12file10 fileciteturn12file2 fileciteturn12file6 fileciteturn12file3
