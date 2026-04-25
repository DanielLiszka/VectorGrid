# Final CUDA Code Review Report

**Files reviewed**

- `options_expression.cu` — 362 lines
- `double_crossover(15).cu` — 11,922 lines

**Review type**: static source review. I was not able to compile or profile the kernels in this environment because `nvcc` is not installed. The findings below should therefore be treated as a source-level correctness, ABI, robustness, and performance review rather than a profiler-backed optimization report.

**Context assumed**: CUDA 13.x, use of CUB/CCCL through `#include <cub/block/block_radix_sort.cuh>`, GPU screening/backtesting kernels, and host-side exactification/ranking after GPU shortlist materialization.

---

## 1. Executive summary

The codebase is materially more sophisticated than a typical CUDA backtest implementation. Good design choices are already present:

- Time-major moving-average layout for the main backtest path.
- Compact pair modes and row-index indirection to avoid evaluating every dense pair when unnecessary.
- Grid-stride loops in many utility kernels.
- Extensive `__restrict__` usage in `double_crossover(15).cu`.
- CUB block-local radix sorting for first-stage top-k selection.
- Template specialization to reduce runtime branches in the flex backtest body.
- Clear contract comments in `options_expression.cu` about tuple-lane ownership and host-side exactification.

The main weakness is not algorithmic; it is that many fast kernels depend on implicit host-side invariants that are not consistently enforced. If the host ever sends a null pointer, negative count, inconsistent stride, bad top-k size, mixed metric-row format, or search/holdout boundary inconsistency, several kernels can either fault or silently rank invalid rows as valid winners.

The most important fixes are:

1. **Guard output pointers and output strides before any store.** Several kernels write zero rows before validating `out_metrics` or the input combo arrays.
2. **Harden the strategy-program ABI bounds checks.** Negative counts currently become huge unsigned values in some checks.
3. **Split canonical double-MA kernels from generic ABI-driven strategy-program kernels.** One TM M7 kernel currently overrides the supplied program for `condition_count >= 2 && group_count >= 4`, and its fallback generic path passes null indicator rows.
4. **Stop using all-zero rows as the invalid-row sentinel.** A zero metric row can rank highly for objectives like `-max_dd` or `-volatility`.
5. **Make metric-row format explicit.** Inferring log-space versus arithmetic-space from only `M < 17` is too weak because some M7 producers write arithmetic rows while flex reduced rows write log-space rows.
6. **Fix all top-k selectors so failed-constraint and non-finite-score rows keep `val = -1`.** Several selectors still propagate row IDs even when the score is `-inf`.
7. **Enforce top-k launch invariants.** The CUB sort type is fixed at 256 threads and 2 items/thread, so launches must match `blockDim.x == 256`, and `topk` must be `<= 512`.
8. **Use 64-bit index products everywhere.** `options_expression.cu`, transpose/pack helpers, top-k output offsets, and several pair-count expressions still have 32-bit overflow edges.
9. **Clamp generic flex commission and numeric transforms.** The generic flex body uses `log1pf(-commission)` without clamping, while other kernels clamp commission.
10. **Add a host-side contract checklist and debug validation path.** Many kernels can remain fast if the host validates shapes, pointers, flags, row formats, and launch dimensions before production launches.

---

## 2. Priority action list

### P0 — Fix before relying on production ranking

| Priority | Area | Why it matters | Recommended fix |
|---:|---|---|---|
| P0 | Output stores before validation | Null or bad `out_metrics` can fault; bad zero rows can be ranked. | Validate output pointer and minimum stride first; use `size_t base`; write invalid sentinel or validity mask. |
| P0 | Strategy ABI bounds | Negative counts can bypass unsigned checks; index addition can wrap. | Validate counts as signed before casting; validate offsets with subtractive bounds checks. |
| P0 | Mixed metric-row formats | Objective and constraints can decode M7 rows incorrectly. | Pass explicit `MetricRowFormat` into scorer/selector kernels. |
| P0 | Top-k invalid row propagation | Failed rows with `NEG_INF_F` can still be emitted when a block has few valid rows. | Keep `val = -1` until score is finite and constraints pass. |
| P0 | Canonical vs generic strategy kernels | Supplied strategy program can be ignored or evaluated with null indicator rows. | Split canonical double-crossover kernels from true ABI-driven kernels. |
| P0 | Top-k launch shape | CUB block collectives require the compile-time block shape to match the launch shape. | Enforce `blockDim.x == 256`, `topk <= 512`; add `__launch_bounds__(256)`. |

### P1 — High-value hardening and semantic cleanup

| Priority | Area | Why it matters | Recommended fix |
|---:|---|---|---|
| P1 | 32-bit index products | Large grids/strides can silently wrap. | Convert product arithmetic to `size_t` or `uint64_t`. |
| P1 | Options search/holdout boundary | `search_end > holdout_end` gives inconsistent score denominator. | Enforce `search_end <= holdout_end` or divide by actual search count. |
| P1 | Generic flex commission | `commission >= 1` produces `-inf`/`NaN`; negative commission acts as rebate. | Clamp to `[0, 0.95]`, matching other kernels. |
| P1 | `ret1` optionality | Pointer-walk path treats `ret1` as optional; non-pointer path dereferences when stats need it. | Either require `ret1` or consistently fallback to `expm1f(lr_t)`. |
| P1 | Numeric overflow in `expf(log_eq)-1` | Extremely profitable rows can become `inf` and be ranked as invalid. | Use `expm1f` with clamp or keep rank scoring in log space. |
| P1 | Macro redefinitions | Later unconditional `#define`s override build-time flag configuration. | Guard all strategy-flag macros with `#ifndef`. |

### P2 — Performance and maintainability

| Priority | Area | Why it matters | Recommended fix |
|---:|---|---|---|
| P2 | Options file missing `__restrict__` | Compiler cannot assume non-aliasing in pointer-heavy kernels. | Add `__restrict__` to non-aliasing input/output pointers and benchmark. |
| P2 | Tuple-lane memory coalescing | Random tuple offsets may scatter warp loads. | Sort/bucket tuple lanes by summary-index pattern or offset groups. |
| P2 | Full radix sort for tiny top-k | Sorting 512 items may be overkill for `topk` of 1–8. | Benchmark small-k selection alternatives. |
| P2 | ABI documentation | Host/device ABI structs have implicit padding. | Add `static_assert(sizeof/alignof)` in host and device build. |
| P2 | Unit tests | Many issues are contract drift bugs. | Add CPU golden tests, sanitizer tests, boundary tests, and launch-contract asserts. |

---

## 3. Detailed findings: `options_expression.cu`

### 3.1 32-bit summary-index arithmetic can wrap

**Locations**

- `options_expression_score_split_f64`, lines 43–46
- `options_expression_accumulate_lane`, lines 91–93

Current pattern:

```cpp
const unsigned int candidate_base = event_index * candidate_slot_stride;
const unsigned int summary_index =
    (candidate_base + candidate_offset) * summary_stride + summary_offset;
```

This silently wraps if the summary address space exceeds 32 bits. The larger file already uses 64-bit arithmetic for the analogous option-tuple scorer:

```cpp
const uint64_t candidate_slot_index =
    (uint64_t)event_idx * (uint64_t)candidate_slot_stride +
    (uint64_t)candidate_offset;
const uint64_t summary_index =
    candidate_slot_index * (uint64_t)summary_stride +
    (uint64_t)summary_offset;
```

**Risk**: silent corruption. A wrapped `summary_index` can read a valid-looking but wrong lane.

**Recommended change**

```cpp
static __device__ __forceinline__ size_t options_summary_index(
    uint64_t event_index,
    uint64_t candidate_slot_stride,
    uint64_t summary_stride,
    uint64_t candidate_offset,
    uint64_t summary_offset)
{
    const uint64_t candidate_slot =
        event_index * candidate_slot_stride + candidate_offset;
    return (size_t)(candidate_slot * summary_stride + summary_offset);
}
```

Use it in both the split kernel and the combined helper. If host ABI compatibility allows, make the stride and offset parameters `uint64_t` rather than `unsigned int`.

---

### 3.2 Combined search/holdout/stress scoring is inconsistent when `search_end > holdout_end`

**Location**: `options_expression_score_search_holdout_stress_f64`, lines 225–330.

The kernel loops only until `holdout_end`:

```cpp
for (unsigned int event_index = 0; event_index < holdout_end; ++event_index) {
    if (event_index < search_end) {
        ...
    } else {
        ...
    }
}
```

but the search utility later divides by `search_end`:

```cpp
const double search_score = options_expression_lane_utility(
    search_end,
    search_valid,
    search_total_pnl,
    search_total_premium
);
```

If `search_end > holdout_end`, the kernel accumulates only `holdout_end` search events but computes coverage against `search_end`.

**Risk**: robust score is biased downward or otherwise inconsistent for search rows. This is a silent ranking bug.

**Recommended fix**

Preferred: enforce this on the host:

```cpp
assert(search_end <= holdout_end);
```

Also document the precondition in the kernel comment.

Alternative if you want device-side tolerance:

```cpp
const unsigned int actual_search_end = min(search_end, holdout_end);

for (unsigned int event_index = 0; event_index < actual_search_end; ++event_index) {
    // search accumulation
}

for (unsigned int event_index = actual_search_end; event_index < holdout_end; ++event_index) {
    // holdout/conservative/harsher accumulation
}

const double search_score = options_expression_lane_utility(
    actual_search_end,
    search_valid,
    search_total_pnl,
    search_total_premium);
```

This also removes a branch from the hot loop.

---

### 3.3 Stress tuple offsets are loaded even when no holdout/stress range exists

**Location**: lines 188–195.

The combined kernel always reads baseline, conservative, and harsher tuple-offset arrays:

```cpp
const unsigned int conservative_candidate_offset =
    conservative_tuple_candidate_offsets[tuple_index];
...
const unsigned int harsher_candidate_offset = harsher_tuple_candidate_offsets[tuple_index];
```

If `holdout_end <= search_end`, conservative/harsher offsets are not logically needed. The current code still requires those arrays to be non-null and valid.

**Risk**: search-only runs can fault if the host omits stress arrays. Also causes unnecessary global loads.

**Recommended fix**

Either declare the arrays mandatory in the host contract, or conditionally load them:

```cpp
const bool has_holdout = holdout_end > search_end;

uint32_t conservative_candidate_offset = 0;
uint32_t conservative_summary_offset = 0;
uint32_t harsher_candidate_offset = 0;
uint32_t harsher_summary_offset = 0;

if (has_holdout) {
    conservative_candidate_offset = conservative_tuple_candidate_offsets[tuple_index];
    conservative_summary_offset = conservative_tuple_summary_offsets[tuple_index];
    harsher_candidate_offset = harsher_tuple_candidate_offsets[tuple_index];
    harsher_summary_offset = harsher_tuple_summary_offsets[tuple_index];
}
```

---

### 3.4 Missing pointer validation in `options_expression.cu`

The comments intentionally rely on host-side validation for premiums and valid flags. That is fine for performance, but the kernels also dereference many pointers before checking anything:

- Tuple-offset arrays: lines 33–34 and 188–195.
- Outcome arrays: lines 47–50 and through the helper at lines 94–97.
- Output arrays: lines 62–67 and 295–357.

**Risk**: null pointers or incorrectly sized buffers cause illegal memory access.

**Recommended policy**

For production performance, use host-side asserts. For debug builds or defensive kernels, add early guards:

```cpp
if (!tuple_candidate_offsets || !tuple_summary_offsets ||
    !outcome_valid || !outcome_pnl || !outcome_premium ||
    !out_valid || !out_wins || !out_total_pnl || !out_total_premium ||
    !out_sum_return_on_premium || !out_sumsq_return_on_premium) {
    return;
}
```

The exact set should match whether a pointer is truly optional.

---

### 3.5 Premium positivity is a host contract; add a debug validator

The top-of-file contract says valid outcomes guarantee positive premium, and the kernels divide by premium without rechecking. That is a reasonable speed tradeoff.

**Risk**: one bad premium can produce `inf` or `NaN`, poison sums, and alter ranking.

**Recommended fix**

Keep the fast path unchanged, but add a debug host validator or a tiny debug kernel that checks:

```text
outcome_valid[summary_index] == 0 ||
(isfinite(outcome_premium[summary_index]) && outcome_premium[summary_index] > 0)
```

Only run this when ingesting new datasets or when debugging.

---

### 3.6 Accumulator precision and naming

`options_expression_score_split_f64` and `options_expression_score_search_holdout_stress_f64` use double for the utility/robust score, but most per-lane aggregate sums are `float`.

**Risk**: not necessarily a correctness bug. It is a naming and ranking-stability concern. If event counts are large or P&L/premium distributions are heavy-tailed, float accumulation may change shortlist order.

**Recommended fix**

- Keep float accumulation if this is a coarse GPU screen and the host exactifies shortlisted rows.
- Rename kernels or document that `f64` refers to the utility/score output, not the full accumulation path.
- For sensitive ranking experiments, benchmark a double-accumulation or compensated-summation variant.

---

### 3.7 Add `__restrict__` in `options_expression.cu`

`double_crossover(15).cu` uses `__restrict__` extensively, but `options_expression.cu` does not. These kernels are pointer-heavy and mostly read-only with independent output arrays.

**Recommended pattern**

```cpp
const unsigned int* __restrict__ tuple_candidate_offsets,
const unsigned int* __restrict__ tuple_summary_offsets,
const unsigned char* __restrict__ outcome_valid,
const float* __restrict__ outcome_pnl,
const float* __restrict__ outcome_premium,
int* __restrict__ out_valid,
...
```

NVIDIA documents that restricted pointers let the compiler assume non-aliasing, enabling fewer loads and common-subexpression work, though this can increase register pressure and should be benchmarked.[^cuda-restrict]

---

## 4. Detailed findings: strategy-program ABI and generic strategy kernels

### 4.1 `strategy_program_load_indicator_value_f32` has an unsigned-cast hole

**Location**: `double_crossover(15).cu`, lines 198–217.

Current guard:

```cpp
if (!indicator_slots || indicator_slot_index >= (uint32_t)indicator_slot_count || t < 0) {
    return NAN;
}
```

If `indicator_slot_count` is negative, casting it to `uint32_t` makes it a very large positive value. The bounds check no longer protects the array load.

**Recommended fix**

```cpp
if (!indicator_slots || indicator_slot_count <= 0 || t < 0) {
    return NAN;
}
if (indicator_slot_index >= (uint32_t)indicator_slot_count) {
    return NAN;
}
```

---

### 4.2 `strategy_program_eval_group_f32` needs stronger bounds checks

**Location**: lines 314–370.

Current issues:

- `group_count <= 0` is not explicitly handled before `group_index >= group_count`.
- `group_condition_index_count` is cast to `uint32_t` at line 342; negative values become huge.
- `group.condition_offset + i` can wrap.
- `(int)condition_index >= condition_count` can fail if `condition_index > INT_MAX` because the cast may become negative.
- Any nonzero `group.mode` behaves as ANY/OR. That may be intended, but if only `0 = ALL` and `1 = ANY` are valid, invalid modes should be rejected.

**Recommended fix pattern**

```cpp
static __device__ __forceinline__ int strategy_program_eval_group_f32_safe(
    int group_index,
    const GpuIndicatorSlotAbi* __restrict__ indicator_slots,
    int indicator_slot_count,
    const GpuConditionAbi* __restrict__ conditions,
    int condition_count,
    const GpuConditionGroupAbi* __restrict__ groups,
    int group_count,
    const uint32_t* __restrict__ group_condition_indexes,
    int group_condition_index_count,
    const float* __restrict__ fast_row,
    const float* __restrict__ slow_row,
    int sig_t,
    int prev_t)
{
    if (!groups || group_count <= 0 || group_index < 0 || group_index >= group_count) {
        return 0;
    }
    if (!conditions || condition_count <= 0 ||
        !group_condition_indexes || group_condition_index_count <= 0) {
        return 0;
    }

    const GpuConditionGroupAbi group = groups[group_index];
    if (group.condition_count == 0u) return 0;
    if (group.mode != 0u && group.mode != 1u) return 0;

    const uint32_t index_count = (uint32_t)group_condition_index_count;
    if (group.condition_offset > index_count) return 0;
    if (group.condition_count > index_count - group.condition_offset) return 0;

    const uint32_t cond_count_u = (uint32_t)condition_count;
    const int is_all = (group.mode == 0u);
    int aggregate = is_all ? 1 : 0;

    for (uint32_t i = 0; i < group.condition_count; ++i) {
        const uint32_t idx_offset = group.condition_offset + i;
        const uint32_t condition_index = group_condition_indexes[idx_offset];
        if (condition_index >= cond_count_u) return 0;

        const int condition_ok = strategy_program_eval_condition_f32(
            conditions[condition_index],
            indicator_slots,
            indicator_slot_count,
            fast_row,
            slow_row,
            sig_t,
            prev_t);

        if (is_all) {
            aggregate = aggregate && condition_ok;
            if (!aggregate) break;
        } else {
            aggregate = aggregate || condition_ok;
            if (aggregate) break;
        }
    }
    return aggregate;
}
```

---

### 4.3 `GpuIndicatorSlotAbi::route` is included in the probe but ignored by evaluation

**Locations**

- Struct field: lines 63–66.
- Probe checksum includes route: lines 100–104.
- Evaluation switch uses only `indicator_index`: lines 209–217.

If `route` is reserved, this is harmless but should be documented. If it is supposed to distinguish input sources or routing modes, the strategy evaluator is ignoring part of the ABI.

**Recommended fix**

Either document:

```cpp
// route is currently reserved and must be zero.
```

and validate it on the host, or include route semantics in `strategy_program_load_indicator_value_f32`.

---

### 4.4 `strategy_program_double_ma_backtest_tm_f32_m7` silently overrides supplied programs

**Location**: lines 858–1020, especially lines 935–1005.

The kernel hard-codes canonical double-cross logic whenever `condition_count >= 2 && group_count >= 4`:

```cpp
if (condition_count >= 2 && group_count >= 4) {
    long_entry = cross_up;
    long_exit = cross_down;
    short_entry = cross_down;
    short_exit = cross_up;
} else {
    long_entry = strategy_program_eval_group_f32(..., nullptr, nullptr, ...);
    ...
}
```

There are two problems:

1. For many valid ABI programs, the supplied conditions, group composition, operators, constants, and signal bindings are ignored.
2. In the fallback generic path, both indicator rows are passed as `nullptr`, so indicator slots 0 and 1 evaluate to `NAN`, and normal fast/slow operands cannot work.

**Risk**: silent semantic mismatch. The host may believe it is evaluating a user-supplied strategy program, but the kernel runs canonical double-crossover logic.

**Recommended fix**

Split the kernel into two explicit entry points:

```cpp
extern "C" __global__
void strategy_program_double_ma_backtest_tm_canonical_m7(...);

extern "C" __global__
void strategy_program_double_ma_backtest_tm_program_m7(...);
```

The canonical kernel should not take unused ABI arrays. The program kernel should construct valid row pointers or provide an indicator loader that can read from time-major data.

---

### 4.5 Generic strategy kernels seed position using double-MA semantics

**Locations**

- Row-major strategy program kernel: lines 525–548.
- TM M7 kernel: lines 1008–1013.

The code seeds a long/short position at the first bar based on fast/slow relationship even when the ABI signals may not imply such a seed.

**Risk**: a generic strategy program can acquire double-MA-specific behavior that the host did not request.

**Recommended fix**

Make this controlled by a flag, for example:

```cpp
#ifndef STRAT_SEED_INITIAL_FAST_SLOW_STATE
#define STRAT_SEED_INITIAL_FAST_SLOW_STATE (1u << 7)
#endif
```

Then:

```cpp
if ((flags & STRAT_SEED_INITIAL_FAST_SLOW_STATE) &&
    t == start_t && pos == 0 && indicator_slot_count >= 2) {
    ...
}
```

Or keep seeding only in canonical double-MA kernels.

---

### 4.6 ABI struct layout should be asserted

These structs cross the host/device boundary:

```cpp
GpuValueOperandAbi
GpuConditionAbi
GpuConditionGroupAbi
GpuSignalBindingsAbi
GpuIndicatorSlotAbi
```

`GpuConditionAbi` in particular contains nested `uint64_t` data and may have padding.

**Recommended fix**

Add static assertions in a header shared by host and device compilation:

```cpp
static_assert(sizeof(GpuValueOperandAbi) == 16, "Unexpected GpuValueOperandAbi size");
static_assert(alignof(GpuValueOperandAbi) == 8, "Unexpected GpuValueOperandAbi alignment");
// Add equivalent assertions for every ABI struct.
```

Use the actual expected sizes from your host-side ABI packing.

---

## 5. Detailed findings: output validation and invalid-row semantics

### 5.1 `strategy_program_double_ma_backtest_f32` writes output before validating `out_metrics`

**Location**: lines 374–432.

Current order:

```cpp
const int fast_row_index = combo_fast_row_idx[idx];
...
const int base = idx * out_stride;

if (base < 0) return;
if (out_stride >= 10) {
    out_metrics[base + 0] = 0.0f;
    ...
}
if (!lr || !fast_row_major || !slow_row_major || !signals) return;
```

Issues:

- Combo arrays are read before validating those pointers.
- `base` is computed in `int` and can overflow.
- `out_metrics` is dereferenced before any `out_metrics` null check.
- `out_stride < 10` falls through without clear invalid-row behavior.

**Recommended fix pattern**

```cpp
const int idx = (int)(blockIdx.x * blockDim.x + threadIdx.x);
if (idx < 0 || idx >= combo_count) return;

if (!out_metrics || out_stride < 10) return;
if (!combo_fast_row_idx || !combo_slow_row_idx || !combo_start_t || !combo_end_t) return;
if (!lr || !fast_row_major || !slow_row_major || !signals) {
    const size_t base = (size_t)idx * (size_t)out_stride;
    write_invalid_metric_row(out_metrics, base, out_stride);
    return;
}

const size_t base = (size_t)idx * (size_t)out_stride;
write_invalid_metric_row(out_metrics, base, out_stride); // optional default

const int fast_row_index = combo_fast_row_idx[idx];
...
```

If all validation is guaranteed by the host, still move `out_metrics` and stride validation before the first output store.

---

### 5.2 Same early-store pattern appears in specialized kernels

Examples:

- `asr_backtest_metrics_f32`, lines 10665–10704.
- `vcc_backtest_metrics_f32`, lines 11066–11113.
- Several earlier specialized metric kernels use similar zero-row initialization before validating all pointers/ranges.

**Risk**: illegal memory access if outputs are missing; invalid rows get all-zero metrics.

**Recommended fix**

Create a reusable validation and invalid-write helper, then apply it consistently:

```cpp
static __device__ __forceinline__ void write_invalid_metric_row(
    float* __restrict__ out,
    size_t base,
    int M)
{
    if (!out || M <= 0) return;
    out[base + 0] = NEG_INF_F;
    if (M > 1) out[base + 1] = 0.0f;
    if (M > 2) out[base + 2] = 1.0e6f;
    if (M > 3) out[base + 3] = NEG_INF_F;
    if (M > 4) out[base + 4] = 1.0e6f;
    for (int j = 5; j < M; ++j) {
        out[base + (size_t)j] = 0.0f;
    }
}
```

A separate `uint8_t* row_valid` mask is even cleaner because it avoids encoding invalidity into metric values.

---

### 5.3 All-zero invalid rows can rank highly

`zero_metrics_row_fixed_f32` writes zeros to every metric slot:

```cpp
for (int i = 0; i < N; ++i) {
    metrics_out[base + (size_t)i] = 0.0f;
}
```

A zero row can score well under objectives such as:

- `objective == 2`: score is `-max_dd`, so `max_dd = 0` is excellent.
- `objective == 4`: score is `-std`, so `std = 0` is excellent.
- Calmar can also produce favorable behavior if `pnl` and drawdown are both zero depending on constraints.

**Risk**: invalid rows can enter top-k if `min_trades` is not enforced or if a block has few valid rows.

**Recommended fix**

Use one of these approaches:

1. **Best**: add `row_valid` mask and make selectors reject invalid rows.
2. **Good**: use a bad-rank sentinel row as shown above.
3. **Minimum**: require `min_trades > 0` on every ranking path and enforce it in host launch code.

---

## 6. Detailed findings: metric-row format and objective semantics

### 6.1 `metrics_row_uses_log_space_f32(M)` is too weak

**Location**: lines 4100–4103.

```cpp
static __device__ __forceinline__ bool metrics_row_uses_log_space_f32(int M)
{
    return M < 17;
}
```

Flex reduced rows deliberately write log-space values for `M < 17`:

```cpp
metrics_out[base + 0] = search_row ? log_eq : pnl;
metrics_out[base + 2] = search_row ? max_log_dd : max_dd;
```

But older/specialized M7 kernels write arithmetic M7 rows, for example:

```cpp
metrics_out[base + 0] = pnl;
metrics_out[base + 2] = max_dd;
```

**Risk**: `objective_score_f32` and constraints can decode a row incorrectly based only on `M`. Arithmetic M7 rows may be interpreted as log-space M7 rows, producing wrong Calmar, drawdown constraints, and displayed values.

**Recommended fix**

Make row format explicit:

```cpp
enum MetricRowFormat : int {
    METRIC_ROW_ARITH = 0,
    METRIC_ROW_LOG_EQ_LOG_DD = 1,
};

static __device__ __forceinline__ float read_metric_pnl(
    const float* __restrict__ metrics,
    size_t base,
    int format)
{
    const float x = metrics[base + 0];
    return format == METRIC_ROW_LOG_EQ_LOG_DD ? expm1f(x) : x;
}

static __device__ __forceinline__ float read_metric_dd(
    const float* __restrict__ metrics,
    size_t base,
    int format)
{
    const float x = metrics[base + 2];
    return format == METRIC_ROW_LOG_EQ_LOG_DD ? -expm1f(-x) : x;
}
```

Then pass `metric_row_format` into every selector/scorer that reads metric rows.

---

### 6.2 Constraints currently compare raw drawdown slot for log-space rows

**Location**: `passes_constraints_f32`, lines 8729–8732.

```cpp
const float dd = metrics[base + 2];
if (!(dd <= max_dd)) return false;
```

If the row is log-space, slot 2 is `max_log_dd`, not arithmetic `max_dd`.

**Risk**: a user-supplied arithmetic max drawdown threshold is compared against log drawdown units.

**Recommended fix**

```cpp
const float dd = read_metric_dd(metrics, base, metric_row_format);
```

Also apply explicit format decoding in Calmar and any host-side display/threshold path.

---

### 6.3 Objective 0 returns raw slot 0

**Location**: lines 8611–8613.

```cpp
if (objective == 0) {
    return (M > 0) ? metrics[base + 0] : NEG_INF_F;
}
```

For log-space rows, this ranks by `log_eq`; for arithmetic rows, it ranks by `pnl`. The ranking order is monotonic for finite values, so this is acceptable for pure ordering. It is not acceptable if score thresholds or reported objective values are interpreted as arithmetic P&L.

**Recommended fix**

If objective values are only used for sorting, document that log-space P&L objective returns log equity for log rows. If score thresholds are applied, decode explicitly.

---

### 6.4 Numeric overflow in `expf(log_eq) - 1.0f`

Examples:

- Flex body, line 4926.
- Closed-trade summary, line 4090.
- Many specialized kernels compute equivalent `pnl` or trade-return values.

If `log_eq` is large, `expf(log_eq)` overflows to `inf`. Later selectors often map non-finite scores to `NEG_INF_F`, so an extremely profitable row can be demoted to the worst possible score.

**Recommended fix**

Use `expm1f` and clamp for materialized arithmetic metrics:

```cpp
const float pnl = expm1f(fminf(log_eq, 80.0f));
```

For ranking, prefer log-space scoring when the objective is monotonic in P&L.

For closed trade summaries:

```cpp
const float trade_ret = expm1f(fminf(trade_log_ret, 80.0f));
```

---

### 6.5 Flex variance should clamp tiny negative values

**Location**: lines 4922–4927.

```cpp
const float variance = (need_std && n > 1) ? (m2 / (float)(n - 1)) : 0.0f;
const float std = sqrtf(variance);
```

Earlier kernels use `sqrtf(fmaxf(variance, 0.0f))`. The flex body should do the same to avoid a `NaN` from roundoff in Welford accumulation.

**Recommended fix**

```cpp
const float variance = (need_std && n > 1) ? (m2 / (float)(n - 1)) : 0.0f;
const float std = sqrtf(fmaxf(variance, 0.0f));
```

---

### 6.6 Trade count semantics should be documented

Several paths increment `trades` on entries, exits, and flips. Flips can count as two transactions. M22 also records `closed_trades`, `winning_trades`, and `losing_trades` as round-trip-like counters.

**Risk**: host constraints or UI can misinterpret `trades` as closed positions.

**Recommended fix**

Document metric slot 1 as transaction count if that is intended. If the host expects closed trades, use the M22 `closed_trades` slot or add a separate slot to reduced rows.

---

## 7. Detailed findings: flex backtest body

### 7.1 Generic flex commission is not clamped

**Location**: lines 4278–4280.

```cpp
const bool has_commission =
    (COMMISSION_MODE == 0) ? false : ((COMMISSION_MODE == 1) ? true : (commission > 0.0f));
const float log_comm = has_commission ? log1pf(-commission) : 0.0f;
```

If `commission >= 1`, `log1pf(-commission)` is `-inf` or `NaN`. If `commission < 0`, this behaves like a rebate.

Other kernels clamp commission to `[0, 0.95]`; the flex body should match.

**Recommended fix**

```cpp
const float c = fminf(fmaxf(commission, 0.0f), 0.95f);
const bool has_commission =
    (COMMISSION_MODE == 0) ? false :
    (COMMISSION_MODE == 1) ? (c > 0.0f) :
                             (c > 0.0f);
const float log_comm = has_commission ? log1pf(-c) : 0.0f;
```

---

### 7.2 `ret1` is optional in one path but mandatory in another

**Locations**

- Pointer-walk path: lines 4480–4488 and 4864–4868.
- Non-pointer path: lines 4912–4915.

Pointer-walk path:

```cpp
if (need_step_returns && ret1_ptr != nullptr) {
    ret1_cur = ld_ro_1d(ret1_ptr, 0);
}
```

Non-pointer path:

```cpp
if (need_step_returns) {
    ret1_t = ld_ro_1d(ret1, (size_t)t);
}
```

**Risk**: if `ret1 == nullptr` and a non-pointer path needs step returns, the kernel dereferences null.

**Recommended fix**

Choose one contract:

- Require `ret1` whenever stats requiring simple returns are enabled, and host-assert it; or
- Make all paths fallback consistently.

Fallback helper:

```cpp
static __device__ __forceinline__ float load_ret1_or_from_log(
    const float* __restrict__ ret1,
    int t,
    float lr_t)
{
    return ret1 ? ld_ro_1d(ret1, (size_t)t) : expm1f(lr_t);
}
```

---

### 7.3 Pair-count and stride arithmetic has 32-bit overflow edges

Examples:

- Pair count: `Pf_tile * Ps_tile`, lines 4300–4301.
- Grid-stride products in multiple kernels: `blockDim.x * gridDim.x` stored in `int`.
- Top-k block starts: `blockIdx.x * TOPK_BLOCK_ITEMS` stored in `int`.

**Risk**: silent wrap for very large grids or tiles.

**Recommended fix**

Use `size_t` for counts and offsets:

```cpp
const size_t dense_pair_count = (size_t)Pf_tile * (size_t)Ps_tile;
const size_t stride = (size_t)blockDim.x * (size_t)gridDim.x;
for (size_t idx = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
     idx < total;
     idx += stride) {
    ...
}
```

Where kernel ABI uses `int`, validate upper bounds on the host.

---

### 7.4 `metrics_stride` should be validated against `M`

**Location**: `metrics_row_stride_f32`, lines 5050–5052.

```cpp
return metrics_stride > 0 ? metrics_stride : M;
```

If `metrics_stride > 0` but `< M`, rows overlap.

**Recommended fix**

At host launch, assert:

```cpp
assert(M > 0);
assert(metrics_stride == 0 || metrics_stride >= M);
```

In kernels that consume rows:

```cpp
const int row_stride = metrics_stride > 0 ? metrics_stride : M;
if (M <= 0 || row_stride < M) return;
```

---

### 7.5 `compute_eps_f32<EPS_MODE == 1>` does not clamp negative epsilon

**Location**: lines 3548–3556.

```cpp
} else if constexpr (EPS_MODE == 1) {
    return eps_rel * fmaxf(1.0f, fabsf(s));
}
```

The runtime mode clamps by checking `eps_rel > 0.0f`; the specialized mode does not.

**Risk**: negative epsilon changes signal threshold semantics.

**Recommended fix**

Either host-assert `eps_rel >= 0.0f` or clamp in all modes:

```cpp
const float er = fmaxf(eps_rel, 0.0f);
return er * fmaxf(1.0f, fabsf(s));
```

---

### 7.6 Risk thresholds should be host-validated

Inputs such as `stop_log`, `take_log`, `regime_range_entry_rel`, and `regime_range_exit_rel` are used directly. If your intended semantics are conventional:

- `stop_log <= 0`
- `take_log >= 0`
- regime range parameters nonnegative

then validate them. Otherwise document negative/inverted semantics explicitly.

---

## 8. Detailed findings: fixed metric stores and alignment

### 8.1 N==22 vectorized store lacks the N==17 fallback structure

**Location**: `store_metrics_row_fixed_f32`, lines 4170–4198.

For `N == 17`, the code checks `float4` alignment, then `float2` alignment, then falls back to scalar stores. For `N == 22`, it checks `float4`, but if that fails it unconditionally casts to `float2*`:

```cpp
float2* const row2 = reinterpret_cast<float2*>(row);
```

**Risk**: if the row pointer is not naturally aligned for `float2`, this can produce inefficient or invalid vectorized access. `cudaMalloc` gives high base alignment, but row offsets and custom strides can break per-row alignment.

**Recommended fix**

Mirror the N==17 structure:

```cpp
if ((((uintptr_t)row) & (sizeof(float4) - 1u)) == 0u) {
    ...
    return;
}
if ((((uintptr_t)row) & (sizeof(float2) - 1u)) == 0u) {
    float2* const row2 = reinterpret_cast<float2*>(row);
    ...
    return;
}
// scalar fallback
row[0] = pnl;
row[1] = (float)trades;
...
```

NVIDIA documents that global memory instructions are most efficient when access size and natural alignment requirements are satisfied.[^cuda-memory]

---

## 9. Detailed findings: top-k selectors and scoring kernels

### 9.1 CUB top-k kernels require fixed launch shape

**Locations**

- Top-k constants: lines 8900–8902.
- CUB type instantiations throughout selector kernels.

```cpp
#define TOPK_BLOCK_THREADS 256
#define TOPK_ITEMS_PER_THREAD 2
#define TOPK_BLOCK_ITEMS (TOPK_BLOCK_THREADS * TOPK_ITEMS_PER_THREAD)

typedef cub::BlockRadixSort<float, TOPK_BLOCK_THREADS, TOPK_ITEMS_PER_THREAD, int> BlockSort;
```

CUB block collectives are parameterized by `TOPK_BLOCK_THREADS`. These kernels must be launched with `blockDim.x == 256`. Also, `topk` must be `<= 512`; otherwise, only the first 512 ranks can be produced by one block.

**Recommended fix**

Host-side:

```cpp
constexpr int kTopKBlockThreads = 256;
constexpr int kTopKBlockItems = 512;
assert(block_threads == kTopKBlockThreads);
assert(topk >= 0 && topk <= kTopKBlockItems);
```

Device-side annotation:

```cpp
extern "C" __global__
__launch_bounds__(TOPK_BLOCK_THREADS)
void select_topk_from_pairs_tm_f32(...)
```

NVIDIA documents that `__launch_bounds__` specifies the maximum block size and that launching with more threads than the launch bound fails.[^cuda-launch-bounds]

---

### 9.2 Top-k output is block-local, not global

Every top-k block sorts only its own `TOPK_BLOCK_ITEMS` candidates. The output arrays contain per-block top-k rows. They are not a global top-k unless the host or a second kernel merges them.

**Risk**: if any caller treats one selector launch as a global top-k, results are incorrect.

**Recommended fix**

Name outputs and host pipeline explicitly:

```text
stage1_block_topk -> stage2_merge_block_topk -> final_global_topk
```

Also consider outputting `(score, index)` pairs from stage 1 so the merge pass does not reread metric rows solely to reconstruct scores.

---

### 9.3 Several selectors propagate invalid row IDs

The common problematic pattern is:

```cpp
float score = NEG_INF_F;
int val = -1;
if (idx < rows) {
    score = objective_score_f32(...);
    if (!isfinite(score)) score = NEG_INF_F;
    val = idx;
}
```

or:

```cpp
row = in_row_idx[in_pos];
...
keys[item] = score;
vals[item] = row;
```

This emits a real row ID even if the row failed constraints or had a non-finite score.

**Examples**

- `select_topk_from_pairs_tm_f32`, lines 8936–8953.
- `select_topk_from_candidates_tm_f32`, lines 9008–9039.
- Scored-row selector, lines 9719–9722.
- Scored-row candidate selector, lines 9759–9767.
- Dense metric row v2 selector, lines 10033–10039.
- Candidate metric row v2 selector, lines 10173–10184.

**Risk**: if a block has fewer than `topk` valid rows, invalid rows can be emitted and later gathered/ranked.

**Recommended fix pattern**

```cpp
float score = NEG_INF_F;
int val = -1;

if (candidate_in_bounds) {
    const size_t base = ...;
    if (passes_constraints_f32(...)) {
        const float s = objective_score_f32(...);
        if (isfinite(s)) {
            score = s;
            val = row;
        }
    }
}

keys[item] = score;
vals[item] = val;
```

Downstream gather/merge kernels should keep filtering `val < 0`.

---

### 9.4 Top-k tie behavior is not deterministic by row ID

CUB sorting by float key only does not define a stable secondary order for equal scores. If reproducibility matters, add a deterministic tie-breaker.

Options:

- Sort a composite key with quantized score plus inverted row ID.
- Sort by score, then run deterministic post-processing for equal-score groups.
- Use key/value pairs where value is used in a stable host merge.

---

### 9.5 Top-k output offsets should use `size_t`

Current pattern:

```cpp
out_pair_idx[(int)blockIdx.x * topk + rank] = vals[item];
```

**Risk**: `blockIdx.x * topk` can overflow `int`.

**Recommended fix**

```cpp
out_pair_idx[(size_t)blockIdx.x * (size_t)topk + (size_t)rank] = vals[item];
```

Also validate that the output buffer has at least `gridDim.x * topk` elements.

---

## 10. Detailed findings: heatmap and atomic helpers

### 10.1 `atomicMaxFloat` assumes sane initialization

**Location**: lines 8783–8794.

```cpp
if (!(value > old_f)) break;
```

If the heatmap max buffer is initialized to `NaN`, then `value > old_f` is false and the bin can never update.

**Recommended fix**

Initialize `heatmap_out` to `NEG_INF_F`, not `NaN`. Document it in the host contract.

---

### 10.2 `bin_period_i32` can overflow intermediate multiplication

**Location**: lines 8796–8804.

```cpp
int b = (num * (bins - 1)) / denom;
```

Use 64-bit math:

```cpp
const int64_t denom = (int64_t)v_max - (int64_t)v_min;
const int64_t num = (int64_t)v - (int64_t)v_min;
int b = (int)((num * (int64_t)(bins - 1)) / denom);
```

---

### 10.3 `heatmap_update_scores_tm_f32` needs shape and pointer guards

**Location**: lines 8831–8897.

Validate:

```cpp
if (!metrics || !heatmap_out) return;
if (pairs <= 0 || M <= 0 || metrics_stride < M) return;
if (Pf_tile <= 0 || Ps_tile <= 0) return;
if (bins_fast <= 0 || bins_slow <= 0) return;
```

For dense mode, `fast_periods` and `slow_periods` must be non-null. For compact mode, `compact_fast_rows` and `compact_slow_rows` must be non-null if the compact branch is used.

Use `size_t` for the bin index:

```cpp
const size_t idx = (size_t)fb * (size_t)bins_slow + (size_t)sb;
```

---

## 11. Detailed findings: preprocessing and utility kernels

### 11.1 `transpose_row_to_tm` uses 32-bit total element count

**Location**: lines 3803–3816.

```cpp
const int total = rows * cols;
```

**Recommended fix**

```cpp
if (!in || !out || rows <= 0 || cols <= 0 || out_rows < rows) return;
const size_t total = (size_t)rows * (size_t)cols;
const size_t stride = (size_t)blockDim.x * (size_t)gridDim.x;
for (size_t k = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
     k < total;
     k += stride) {
    const size_t r = k / (size_t)cols;
    const size_t c = k - r * (size_t)cols;
    out[c * (size_t)out_rows + r] = in[r * (size_t)cols + c];
}
```

---

### 11.2 Return kernels can use a more stable log formula

**Locations**

- `compute_log_returns_f32`, lines 3823–3841.
- `compute_log_and_simple_returns_f32`, lines 3843–3866.

Current log return:

```cpp
lr_out[t] = logf(p / pm);
```

If `p / pm` overflows despite both prices being finite, `lr_out[t]` becomes `inf`.

**Recommended fix**

```cpp
lr_out[t] = logf(p) - logf(pm);
```

For `ret1_out`, you still need a ratio. Consider handling non-finite ratio:

```cpp
const float ratio = p / pm;
ret1_out[t] = isfinite(ratio) ? (ratio - 1.0f) : 0.0f;
```

---

### 11.3 `fill_f32` and `fill_i32` should guard null and use `size_t`

**Locations**: lines 8816–8827.

Current kernels assume `out` is non-null and `N` fits an `int` grid-stride loop.

**Recommended fix**

```cpp
if (!out || N <= 0) return;
const size_t n = (size_t)N;
const size_t stride = (size_t)blockDim.x * (size_t)gridDim.x;
for (size_t i = (size_t)blockIdx.x * blockDim.x + threadIdx.x;
     i < n;
     i += stride) {
    out[i] = value;
}
```

If you need more than `INT_MAX` elements, change `N` to `size_t` or `uint64_t` in the ABI.

---

### 11.4 `pack_rows_u64_ptr_f32` needs a stronger memory contract

**Location**: lines 10609–10624.

This kernel interprets integers as pointers:

```cpp
const uintptr_t raw = (uintptr_t)row_ptrs[row];
const float* __restrict__ src = reinterpret_cast<const float*>(raw);
out_row_major[idx] = src ? src[col] : NAN;
```

This only works when the pointer values are device-accessible addresses: device pointers, managed pointers, mapped pinned host pointers, or another address class legal for the target platform. Ordinary pageable host pointers are not generally legal device pointers. CUDA 13 introduces additional pageable-host-memory accessibility for Thor-based Jetson platforms, but that is not a general discrete-GPU assumption.[^cuda13-jetson]

Also:

```cpp
const int total = rows * cols;
```

can overflow.

**Recommended fix**

```cpp
if (!row_ptrs || !out_row_major || rows <= 0 || cols <= 0) return;
const size_t total = (size_t)rows * (size_t)cols;
...
```

Document the pointer-address contract prominently.

---

### 11.5 `nearest_period_i32` assumes sorted ascending period arrays

**Location**: lines 10484–10510.

This helper is a binary search. It requires `periods[0..len)` to be sorted ascending.

**Recommended fix**

Host-assert:

```cpp
assert(std::is_sorted(fast_periods.begin(), fast_periods.end()));
assert(std::is_sorted(slow_periods.begin(), slow_periods.end()));
```

Also use 64-bit distances to avoid integer overflow:

```cpp
const int64_t d_lo = llabs((int64_t)value - (int64_t)p_lo);
const int64_t d_hi = llabs((int64_t)value - (int64_t)p_hi);
```

---

### 11.6 `vdps_generate_seed_pairs_tm_i32` has pointer and integer-overflow assumptions

**Location**: lines 10514–10606.

Potential issues:

- `elite_fast`/`elite_slow` are required if `elite_len > 0` and elite/differential modes are used.
- `fast_periods`/`slow_periods` are required whenever `fast_len > 0` and `slow_len > 0`.
- `out_fast` and `out_slow` are always required.
- `rand_start = elite_count + diff_count` can overflow `int`.
- `f_delta = f_step * 2`, `2 * f_delta + 1` can overflow `int`.
- Invalid generated pairs are written as zero; downstream must filter zeros.

**Recommended fix**

Use host validation for counts and pointer mode. Internally use `int64_t` for count sums and deltas, then clamp before casting back to `int`.

---

## 12. Specialized ASR/VCC kernel notes

The ASR and VCC kernels have the same broad pattern as earlier strategy kernels:

- Combo row arrays are read before their pointers are validated.
- `base = idx * out_stride` is computed in `int`.
- `out_metrics` is zeroed before the pointer is checked.
- Zero invalid rows can rank well.

Examples:

- `asr_backtest_metrics_f32`, lines 10665–10704.
- `vcc_backtest_metrics_f32`, lines 11066–11113.

**Recommended refactor**

Create a shared prologue macro or helper pattern:

```cpp
const int idx = (int)(blockIdx.x * blockDim.x + threadIdx.x);
if (idx < 0 || idx >= combo_count) return;

if (!out_metrics || out_stride < REQUIRED_METRICS) return;
const size_t base = (size_t)idx * (size_t)out_stride;
write_invalid_metric_row(out_metrics, base, out_stride);

if (!combo_fast_row_idx || !combo_slow_row_idx || !combo_start_t || !combo_end_t) return;
...
```

Then validate all source arrays before constructing row pointers.

---

## 13. Macro and build-configuration issues

### 13.1 `_ALLOW_COMPILER_AND_STL_VERSION_MISMATCH` should not be defined in the source file

**Location**: lines 4–6.

```cpp
#ifndef _ALLOW_COMPILER_AND_STL_VERSION_MISMATCH
#define _ALLOW_COMPILER_AND_STL_VERSION_MISMATCH
#endif
```

This masks build environment problems. For CUDA 13.x, toolchain compatibility should be handled by choosing a supported compiler and build flags, not by hiding mismatch checks in a kernel source file.

**Recommended fix**

Remove this definition from the source. If absolutely necessary for a controlled build, keep it in the build system with an explicit comment and CI coverage.

---

### 13.2 Later strategy-flag macros override earlier build configuration

**Locations**

- Initial guarded definitions: lines 19–36.
- Later definitions: lines 4059–4073.

Later definitions include unconditional macros:

```cpp
#define STRAT_ENFORCE_FAST_LT_SLOW   (1u<<3)
#define STRAT_STOP_LOSS              (1u<<5)
#define STRAT_TAKE_PROFIT            (1u<<6)
```

`STRAT_STOP_LOSS` and `STRAT_TAKE_PROFIT` were guarded at the top but redefined unconditionally later.

**Recommended fix**

Guard all strategy macros consistently:

```cpp
#ifndef STRAT_ENFORCE_FAST_LT_SLOW
#define STRAT_ENFORCE_FAST_LT_SLOW   (1u<<3)
#endif
#ifndef STRAT_STOP_LOSS
#define STRAT_STOP_LOSS              (1u<<5)
#endif
#ifndef STRAT_TAKE_PROFIT
#define STRAT_TAKE_PROFIT            (1u<<6)
#endif
```

Or put all shared flags in one header and include it once.

---

## 14. CUDA 13.x-specific notes

### 14.1 CCCL/CUB include behavior

The code includes CUB through:

```cpp
#include <cub/block/block_radix_sort.cuh>
```

CUDA 13.0 includes CCCL 3.0, the unification of Thrust, CUB, and libcudacxx. NVIDIA says not to change includes to `<cccl/...>`. If CCCL headers are included only in `.cu` files compiled with `nvcc`, no include-path action is normally needed. If CCCL headers are used in host-compiler-only translation units, use CMake’s `CCCL::CCCL` or add `${CTK_ROOT}/include/cccl` to the include path.[^cuda13-cccl]

CCCL 3.0 requires C++17 or newer and supports GCC 7+, Clang 14+, and MSVC 2019+ according to NVIDIA’s CUDA 13.0 announcement.[^cuda13-cccl-req]

**Recommended build flags**

```bash
nvcc -std=c++17 --ptxas-options=-v ...
```

Use `--ptxas-options=-v` at least in CI/perf builds to track registers and spills per kernel.

---

### 14.2 Driver and architecture support

CUDA Toolkit 13.x requires driver range `>= 580` for minor-version compatibility according to the CUDA 13.2 release notes.[^cuda13-driver]

CUDA 13.0 also removed offline compilation support for GPU architectures prior to Turing / compute capability 7.5. If you need pre-7.5 architecture binaries, use CUDA 12.9 or earlier for those builds.[^cuda13-arch]

---

## 15. Performance opportunities

### 15.1 Coalescing and tuple ordering in options kernels

The options-expression kernels may be memory-transaction limited if adjacent threads load unrelated `summary_index` values. NVIDIA’s Best Practices Guide calls coalesced global-memory access a very important CUDA performance consideration and labels coalescing as high priority.[^cuda-coalescing]

**Recommended experiment**

Sort or bucket tuple lanes by one of:

```text
candidate_offset
summary_offset
(candidate_offset, summary_offset)
expected summary_index locality
```

Then compare:

- Global Memory Load Efficiency / Store Efficiency.
- Requested versus actual memory throughput.
- End-to-end shortlist generation time.

---

### 15.2 Time-major MA layout is good; benchmark warp de-duplication

The time-major layout is a strong choice for coalescing when neighboring threads evaluate neighboring pair rows at the same `t`. The `ld_tm_fast_warp_dedup` helper may reduce redundant loads when many lanes share `pf_local`, but `__match_any_sync`/shuffle overhead is not free.

**Recommended experiment**

For representative pair orderings, compare:

- Normal time-major loads.
- Warp-dedup fast-row loads.
- Compact pair orderings grouped by fast row.

Collect Nsight Compute metrics such as:

```text
l1tex__t_sectors_pipe_lsu_mem_global_op_ld.sum
dram__throughput.avg.pct_of_peak_sustained_elapsed
smsp__warps_active.avg.pct_of_peak_sustained_active
smsp__sass_inst_executed_op_branch_pred_on.sum
```

---

### 15.3 Top-k: full block radix sort may be overkill for tiny `topk`

A 512-item block sort is robust and simple. If `topk` is usually small, consider benchmarking:

- Repeated block-wide max for `k <= 8`.
- Small local heap per block.
- Selection network.
- Two-phase selection with score/index output to reduce rereads.

Keep the existing CUB version as the baseline. CUB may still win depending on architecture and distribution.

---

### 15.4 Register pressure and occupancy should be measured per specialization

The flex body is large and branchy. Template specialization likely helps, but representative instantiations may have very different register usage.

NVIDIA documents that execution configuration effects are kernel-dependent and recommends experimentation; register usage can be controlled by compiler options or launch bounds.[^cuda-occupancy]

**Recommended CI/perf artifact**

For each important kernel specialization, collect:

```text
kernel name
SM target
registers/thread
spill stores/loads
static shared memory
achieved occupancy
runtime on representative dataset
```

Compile with:

```bash
nvcc -std=c++17 --ptxas-options=-v ...
```

---

## 16. Host-side contract checklist

Add a single host-side validation function before launching each kernel family. For release builds, this can be compiled out or reduced to cheap asserts.

### 16.1 General launch contracts

- `blockDim.x > 0`, `gridDim.x > 0`.
- For CUB top-k kernels: `blockDim.x == 256`.
- For CUB top-k kernels: `0 <= topk <= 512`.
- Output buffers are non-null and sized for the full write range.
- Output offsets use `size_t` when computing allocation sizes.
- Strides are nonnegative and at least the number of columns they need to cover.
- All device pointers are from valid device, managed, or mapped memory.

### 16.2 Options-expression contracts

- `search_end <= holdout_end`, unless the kernel explicitly supports bounded search.
- `event_end >= event_start` for split kernels.
- `tuple_count >= 0` and output arrays have at least `tuple_count` slots.
- Tuple offset arrays have at least `tuple_count` slots.
- Summary arrays are large enough for the maximum summary index.
- If `outcome_valid[summary_index] != 0`, then `outcome_premium[summary_index]` is finite and positive.
- Stress arrays are either mandatory or only passed when `holdout_end > search_end`.

### 16.3 Strategy/backtest contracts

- `T > 0`.
- `eval_start`, `eval_end`, `start_t`, `end_t`, `first_valid`, and regime ranges are internally consistent.
- `fast_tm_stride >= Pf_total` or at least the largest accessed fast row count.
- `slow_tm_stride >= Ps_total` or at least the largest accessed slow row count.
- `fast_periods` and `slow_periods` are sorted ascending where binary search is used.
- `commission` is finite and in expected range, or will be clamped.
- `eps_rel >= 0`.
- If stop/take are enabled, thresholds have intended sign and finite values.
- `ret1` is non-null when stats requiring simple returns are enabled, unless fallback is implemented.
- `MetricRowFormat` matches the producer kernel.
- Invalid rows are represented by `row_valid == 0` or a bad-rank sentinel, not zeros.

### 16.4 Strategy-program ABI contracts

- ABI version matches expected host/device version.
- `indicator_slot_count`, `condition_count`, `group_count`, and `group_condition_index_count` are nonnegative.
- Each group’s `condition_offset + condition_count` is within `group_condition_index_count` using overflow-safe checks.
- Each condition index is `< condition_count`.
- Each indicator operand index is `< indicator_slot_count`.
- `route` is either zero/reserved or validated against supported routes.
- Struct sizes and alignments match host packing.

---

## 17. Suggested patch snippets

### 17.1 Explicit metric-row format

```cpp
enum MetricRowFormat : int {
    METRIC_ROW_ARITH = 0,
    METRIC_ROW_LOG_EQ_LOG_DD = 1,
};

static __device__ __forceinline__ float read_metric_pnl_f32(
    const float* __restrict__ m,
    size_t base,
    int format)
{
    if (format == METRIC_ROW_LOG_EQ_LOG_DD) {
        return expm1f(m[base + 0]);
    }
    return m[base + 0];
}

static __device__ __forceinline__ float read_metric_dd_f32(
    const float* __restrict__ m,
    size_t base,
    int format)
{
    if (format == METRIC_ROW_LOG_EQ_LOG_DD) {
        return -expm1f(-m[base + 2]);
    }
    return m[base + 2];
}
```

Then change:

```cpp
objective_score_f32(metrics, base, M, objective, hurdle_tau)
```

to:

```cpp
objective_score_f32(metrics, base, M, metric_row_format, objective, hurdle_tau)
```

---

### 17.2 Top-k valid-row pattern

```cpp
float score = NEG_INF_F;
int val = -1;

if (idx < rows) {
    const size_t base = (size_t)idx * (size_t)row_stride;
    const bool ok = passes_constraints_f32(...);
    if (ok) {
        const float s = objective_score_f32(...);
        if (isfinite(s)) {
            score = s;
            val = idx;
        }
    }
}

keys[item] = score;
vals[item] = val;
```

Do not set `val` before the row is proven valid.

---

### 17.3 Bad-rank invalid row helper

```cpp
static __device__ __forceinline__ void write_invalid_metric_row_f32(
    float* __restrict__ out,
    size_t base,
    int M)
{
    if (!out || M <= 0) return;
    out[base + 0] = NEG_INF_F;  // pnl/log_eq/objective base
    if (M > 1) out[base + 1] = 0.0f;       // trades
    if (M > 2) out[base + 2] = 1.0e6f;     // drawdown-like slot
    if (M > 3) out[base + 3] = NEG_INF_F;  // mean-like slot
    if (M > 4) out[base + 4] = 1.0e6f;     // volatility/std-like slot
    for (int j = 5; j < M; ++j) {
        out[base + (size_t)j] = 0.0f;
    }
}
```

Use a validity mask if you can; the sentinel is a fallback.

---

### 17.4 Safe options summary index

```cpp
static __device__ __forceinline__ size_t options_summary_index_f32(
    uint64_t event_index,
    uint64_t candidate_slot_stride,
    uint64_t summary_stride,
    uint64_t candidate_offset,
    uint64_t summary_offset)
{
    const uint64_t candidate_slot =
        event_index * candidate_slot_stride + candidate_offset;
    return (size_t)(candidate_slot * summary_stride + summary_offset);
}
```

Host-side should separately validate that the computed maximum fits in the allocated array.

---

### 17.5 Safe commission handling

```cpp
const float c = fminf(fmaxf(commission, 0.0f), 0.95f);
const bool has_commission =
    (COMMISSION_MODE == 0) ? false :
    (COMMISSION_MODE == 1) ? (c > 0.0f) :
                             (c > 0.0f);
const float log_comm = has_commission ? log1pf(-c) : 0.0f;
```

---

### 17.6 Safer group offset validation

```cpp
const uint32_t index_count = (uint32_t)group_condition_index_count;
if (group.condition_offset > index_count) return 0;
if (group.condition_count > index_count - group.condition_offset) return 0;
```

This avoids `group.condition_offset + group.condition_count` overflow.

---

## 18. Validation plan

### 18.1 Build and sanitizer checks

Run these on a CUDA-capable dev machine:

```bash
nvcc -std=c++17 --ptxas-options=-v -lineinfo ...
compute-sanitizer --tool memcheck ./your_test_binary
compute-sanitizer --tool racecheck ./your_test_binary
```

Use `racecheck` selectively; atomics in heatmap updates may produce expected reports depending on patterns.

### 18.2 CPU golden tests

Create small deterministic inputs and compare GPU rows against a CPU implementation for:

- One canonical double-crossover case.
- One generic ABI program with non-crossover comparisons.
- One long-only case.
- One short-enabled case.
- One no-flip case.
- Stop-loss and take-profit cases.
- Commission on/off/clamped cases.
- M7 arithmetic producer versus M7 log-space producer.

### 18.3 Boundary tests

Add tests for:

- Null optional pointers.
- Null required pointers in debug mode.
- Negative counts passed into ABI helpers.
- `search_end > holdout_end`.
- Very large summary index near `2^32`.
- `metrics_stride < M`.
- `topk == 0`, `topk == 1`, `topk == 512`, and `topk > 512`.
- Launching top-k with non-256 block size should fail host validation.
- Fewer valid rows than `topk` should emit `-1` for unused slots.
- `commission < 0`, `commission == 1`, and `commission > 1`.
- `ret1 == nullptr` with stats enabled.
- Very large `log_eq` that would overflow `expf`.
- Unsorted period arrays for VDPS generation should fail host validation.

### 18.4 Ranking tests

Specifically test that invalid rows never outrank valid rows under:

- P&L objective.
- Sharpe objective.
- `-max_dd` objective.
- Calmar objective.
- `-volatility` objective.
- Sortino/Omega objectives if present.

Use a block with fewer valid candidates than `topk` to confirm invalid slots remain `-1`.

---

## 19. Final recommended refactor sequence

1. **Host contract layer**: add launch validators for every kernel family. This is the fastest way to prevent most undefined behavior without slowing kernels.
2. **Output prologue cleanup**: move output-pointer and stride checks before all stores; switch `base` to `size_t`.
3. **Invalid-row representation**: add row validity mask or bad-rank sentinels; update selectors to reject invalid rows.
4. **Metric-row format enum**: remove the `M < 17` heuristic from readers; pass explicit row format to scorers/selectors.
5. **Top-k cleanup**: enforce block size and `topk <= 512`; fix `val = -1` propagation; use `size_t` output offsets.
6. **Strategy ABI hardening**: fix signed/unsigned count checks, route semantics, group offset validation, and struct static assertions.
7. **Canonical/generic split**: separate hard-coded double-MA kernels from true strategy-program kernels.
8. **Numeric hardening**: clamp commission, clamp variance, use `expm1f` with safe bounds, clarify `ret1` optionality.
9. **Options kernel cleanup**: use 64-bit summary indexing, split search/holdout loops, conditionally load stress offsets, add `__restrict__`.
10. **Performance benchmarking**: after correctness hardening, benchmark tuple ordering, top-k variants, warp-dedup loads, and register/occupancy tradeoffs.

---

## 20. References

[^cuda-restrict]: NVIDIA CUDA C++ Programming Guide, `__restrict__` section. It documents that restricted pointers can reduce memory accesses/computation but may increase register pressure. <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>

[^cuda-memory]: NVIDIA CUDA C++ Programming Guide, device-memory access and vector/alignment guidance. <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>

[^cuda-launch-bounds]: NVIDIA CUDA C++ Programming Guide, Launch Bounds section. It documents `__launch_bounds__`, register/resource implications, and that launching with more threads than the bound fails. <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>

[^cuda-coalescing]: NVIDIA CUDA C++ Best Practices Guide 13.2, Coalesced Access to Global Memory section. It labels coalescing as a high-priority consideration and discusses warp memory transactions. <https://docs.nvidia.com/cuda/cuda-c-best-practices-guide/index.html>

[^cuda-occupancy]: NVIDIA CUDA C++ Programming Guide, performance/occupancy guidance and launch-bounds/register discussion. <https://docs.nvidia.com/cuda/cuda-c-programming-guide/index.html>

[^cuda13-cccl]: NVIDIA Technical Blog, “What’s New and Important in CUDA Toolkit 13.0,” CCCL section. It describes CCCL 3.0, CUB/Thrust/libcu++ unification, and include-path guidance. <https://developer.nvidia.com/blog/whats-new-and-important-in-cuda-toolkit-13-0/>

[^cuda13-cccl-req]: NVIDIA Technical Blog, “What’s New and Important in CUDA Toolkit 13.0,” updated CCCL requirements. It states CCCL 3.0 requires C++17 or newer and lists supported host compiler baselines. <https://developer.nvidia.com/blog/whats-new-and-important-in-cuda-toolkit-13-0/>

[^cuda13-driver]: NVIDIA CUDA Toolkit 13.2 Update 1 Release Notes, minimum required driver table for CUDA minor-version compatibility. <https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html>

[^cuda13-arch]: NVIDIA Technical Blog, “What’s New and Important in CUDA Toolkit 13.0,” feature-complete architectures section. It states CUDA 13 removed offline compilation support for compute capabilities prior to 7.5. <https://developer.nvidia.com/blog/whats-new-and-important-in-cuda-toolkit-13-0/>

[^cuda13-jetson]: NVIDIA Technical Blog, “What’s New and Important in CUDA Toolkit 13.0,” Jetson/Thor UVM discussion. <https://developer.nvidia.com/blog/whats-new-and-important-in-cuda-toolkit-13-0/>
