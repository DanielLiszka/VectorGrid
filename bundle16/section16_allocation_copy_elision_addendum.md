# Section 16 addendum: extra fixes, missed opportunities, and beyond-scope follow-ups

This addendum is meant to sit next to the original section-16 plan.

It has three goals:

1. capture the most important item that was not called out strongly enough in the original plan,
2. tighten the rollout / benchmark / verification guidance,
3. separate small in-scope allocation/copy-elision work from larger optional follow-ups that are performance-relevant but belong to a neighboring bundle.

---

## 1. The main in-scope item I would add to the original plan

## `ml_signal.rs` — `extract_pattern_matrix(...)`

### Why this belongs in the addendum

The original plan focused on `log_returns_from_close`, `prefix_sum`, `prefix_sum_squares`, and the reusable `requested` / `desired` working vectors.

That was correct, but one additional allocation/fill pattern in the same file is worth capturing:

```rust
let mut matrix = vec![vec![0u8; cols]; rows];
```

`extract_pattern_matrix(...)` then writes every element of every row in both supported branches:

- `DynamicIndicatorSeries::I32(values)`
- `DynamicIndicatorSeries::Bool(values)`

So the nested zero fill is redundant work.

### Why I would treat it as optional-first-pass, not mandatory-first-pass

This is a real optimization opportunity, but it is slightly different from the other section-16 wins:

- there is no existing shared `alloc_policy` helper for nested `Vec<Vec<u8>>`
- the current return type is nested, not flat row-major
- a totally safe rewrite is easy, but it is less “drop-in helper reuse” than the `f32` / `f64` / `i32` fixes

So I would add it to the original markdown as:

- **medium severity / performance / app-path**
- **safe optimization opportunity**
- **lower priority than the confirmed correctness bugs and D2H i32 prefill removal**

### Minimal parity-safe rewrite (no unsafe)

This keeps the return type unchanged and avoids the full nested zero fill.

```rust
fn extract_pattern_matrix(
    output: &crate::dynamic_indicator_adapter::DynamicIndicatorComputeOutput,
    pattern_ids: &[String],
) -> Result<Vec<Vec<u8>>, String> {
    let rows = output.rows;
    let cols = output.cols;
    if rows == 0 || cols == 0 {
        return Ok(Vec::new());
    }
    if !pattern_ids.is_empty() && rows != pattern_ids.len() {
        return Err(format!(
            "pattern_recognition rows {} do not match pattern_ids {}",
            rows,
            pattern_ids.len()
        ));
    }

    let mut matrix: Vec<Vec<u8>> = Vec::with_capacity(rows);
    match &output.series {
        DynamicIndicatorSeries::I32(values) => {
            if values.len() != rows.saturating_mul(cols) {
                return Err("pattern_recognition matrix length mismatch".to_string());
            }
            for row in 0..rows {
                let offset = row * cols;
                let src = &values[offset..offset + cols];
                let mut out_row = Vec::with_capacity(cols);
                for &value in src {
                    out_row.push(u8::from(value != 0));
                }
                matrix.push(out_row);
            }
        }
        DynamicIndicatorSeries::Bool(values) => {
            if values.len() != rows.saturating_mul(cols) {
                return Err("pattern_recognition matrix length mismatch".to_string());
            }
            for row in 0..rows {
                let offset = row * cols;
                let src = &values[offset..offset + cols];
                let mut out_row = Vec::with_capacity(cols);
                for &value in src {
                    out_row.push(u8::from(value));
                }
                matrix.push(out_row);
            }
        }
        _ => {
            return Err("pattern_recognition produced unsupported output type".to_string());
        }
    }
    Ok(matrix)
}
```

### Deeper optional rewrite

If ML export becomes a large measured hotspot, the better long-term shape is likely:

- flatten `pattern_hits` to a single `Vec<u8>` (or `Box<[u8]>`)
- keep `(rows, cols)` metadata
- add row access helpers instead of materializing `Vec<Vec<u8>>`

That would reduce:

- nested `Vec` overhead,
- allocator fragmentation,
- cache-unfriendly row indirection,
- redundant scans when building `pattern_prefix`.

But that is a larger internal representation change, so I would keep it explicitly out of the first pass.

---

## 2. Rollout guidance I would strengthen in the original plan

## Add an explicit debug/release verification matrix

Because `alloc_policy.rs` intentionally behaves differently under `debug_assertions` versus release, the original plan should be more explicit that the tests must be run in both modes.

### Recommended matrix

1. **Debug test pass**
   - validates poison-path semantics and ordinary parity expectations

2. **Release test pass**
   - validates the actual `set_len(...)` / uninitialized-resize paths
   - catches the real reserve-arithmetic bug class that debug builds can mask

### Suggested commands

```bash
cargo test alloc_policy -- --nocapture
cargo test vemr_gpu_kernel -- --nocapture
cargo test vcc_gpu_kernel -- --nocapture
cargo test double_ma -- --nocapture
cargo test ml_signal -- --nocapture

cargo test --release alloc_policy -- --nocapture
cargo test --release vemr_gpu_kernel -- --nocapture
cargo test --release vcc_gpu_kernel -- --nocapture
cargo test --release double_ma -- --nocapture
cargo test --release ml_signal -- --nocapture
```

## Optional unsafe-correctness lane

For the pure-Rust helpers (`alloc_policy.rs` especially), I would consider an optional `miri` pass.

### Why

This is not a throughput benchmark, but it is useful because the most dangerous section-16 bugs are not “slow”; they are “undefined behavior that happens to look fast until it corrupts state.”

### Scope guard

Do not try to run Miri over CUDA-facing code. Keep it limited to the pure helper / unit-test surface.

---

## 3. Benchmark mapping I would add to the original plan

The original plan was correct about using existing harnesses, but I would make the evidence path concrete.

## After Phase 1 (correctness fixes)

Run only targeted tests first.

Do **not** interpret performance numbers until:

- `resize_uninit_f32` reserve math is fixed,
- `resize_uninit_i32` exists and replaces the VEMR custom helper,
- `double_ma` fallback-row NaN repair is in place.

## After Phase 2 (safe performance cleanup)

Use existing warm app-path / workflow harnesses rather than inventing a new microbench first.

### Recommended mapping

- **GPU readback / shortlist / heatmap effects**
  - use the existing warm app-path Double-MA route / executor evidence path
  - prefer the app-path faceoff / workflow regression surfaces already used in the repo

- **ML signal effects**
  - use the existing end-to-end ML workflow bench / workflow evidence path
  - measure export + score-cache validation / candidate validation impact at the workflow level

### What the addendum would explicitly state

- `gpu_backtest_kernel.rs` i32 readback changes belong to **warm app-path** evidence, not a synthetic helper microbench
- `ml_signal.rs` changes belong to **workflow / e2e ML** evidence, not only a tiny unit timing

---

## 4. Optional changes that go beyond section 16, but are worth tracking

These are not omissions from the original review logic. They are adjacent, higher-leverage follow-ups that matter if your main objective is broader performance + correctness.

## A. Expand the next review bundle to `standalone_gpu_runtime.rs`

### Why

The current section-16 bundle already shows that many adjacent GPU strategy files depend on a shared host-buffer helper (`ensure_host_f32_len(...)`), but that file was not included.

That means the highest-leverage next step is not to hand-patch every strategy-local kernel file blindly. It is to review and, if needed, fix the shared helper once.

### What to ask for next

- whether `ensure_host_f32_len(...)` zero-fills or uses uninitialized resize
- whether there should also be `ensure_host_i32_len(...)` / `ensure_host_f64_len(...)`
- whether strategy-local host readback scratch can be normalized around one typed helper family

### Why this is beyond current scope

It requires a new file not included in the section-16 attachment set.

---

## B. `ml_signal.rs` feature extraction is a separate, likely larger performance surface

### Why

`feature_frame(...)` performs repeated dynamic-indicator computation calls for:

- `pattern_recognition`
- `alma(H)`
- `alma(3H)`
- `alma(10H)`
- `adx(2H)`
- `atr(H)`

The section-16 plan intentionally stayed on allocation/copy-elision, but if dataset export is a material bottleneck, repeated indicator computation and repeated traversal are likely larger than the vector-fill wins.

### Optional follow-up ideas

- cache / batch indicator work when the same candle set and source are reused
- measure whether `pattern_recognition` dominates export time
- avoid re-traversing large intermediate structures when building the dataset rows

### Why this is beyond current scope

This is a **feature-extraction cost** review, which belongs more naturally to the `MlSignal` workflow bundle than to section 16.

---

## C. `ml_signal.rs` candidate validation rescans full series per candidate

### Why

`validate_ml_signal_candidates(...)` currently reuses the `requested` and `desired` vectors, which is good, but it still does a full-bar pass through:

- `fill_requested_targets(...)`
- `fill_desired_positions(...)`
- then `simulate_iterative_from_signals(...)`

for every candidate.

If candidate counts are large, this algorithmic pass structure can dominate the benefit of the allocation wins.

### Optional follow-up directions

- precompute threshold masks or score ordering structure when many candidates differ only by thresholds
- specialize the policy sweep path so it shares more work between candidates
- consider whether `requested` can be derived in a more incremental way for sweep families

### Why this is beyond current scope

This is not primarily an allocation/copy-elision issue. It is a candidate-loop / algorithmic throughput issue.

---

## D. Flatten pattern-hit + prefix storage if ML export becomes a proven hotspot

### Current shape

The export path keeps:

- `pattern_hits: Vec<Vec<u8>>`
- then builds `pattern_prefix: Vec<Vec<usize>>`
- then scans `pattern_hits` again to maintain `last_seen`

That is straightforward and readable, but it is not the tightest memory/access pattern.

### Optional deeper refactor

Move to:

- one flat hit matrix,
- one flat prefix-count matrix or rolling-count helper,
- row-offset indexing helpers,
- possibly fused initialization of `last_seen` / prefix counts.

### Why this should stay optional

It changes the internal data model and is best justified only after workflow-level measurement.

---

## 5. Concrete checklist items to add to the original markdown

Add these items to the checklist section of the original plan.

### Add under “implementation”

- [ ] `ml_signal.rs`: evaluate `extract_pattern_matrix(...)` for safe nested-zero-fill removal
- [ ] If kept nested, switch to `Vec::with_capacity(...) + push(...)` per row
- [ ] If ML export remains hot after first-pass fixes, consider flat `pattern_hits` representation as a separate refactor

### Add under “verification”

- [ ] Run targeted section-16 tests in **debug** mode
- [ ] Run targeted section-16 tests in **release** mode
- [ ] Optionally run Miri on pure helper tests in `alloc_policy.rs`

### Add under “benchmarking”

- [ ] Measure GPU readback changes on existing warm app-path / workflow evidence paths
- [ ] Measure ML signal changes on the existing end-to-end ML workflow evidence path
- [ ] Record whether the observed gain is app-path, workflow, or only micro-level

### Add under “next bundle / beyond current scope”

- [ ] Review `standalone_gpu_runtime.rs` host-buffer helpers next
- [ ] If ML export is still a hotspot, open a separate `MlSignal` feature-extraction review bundle
- [ ] If policy-sweep validation is still a hotspot, open a separate `MlSignal` candidate-loop throughput review bundle

---

## 6. Recommended priority order after the original plan

1. Land the original correctness fixes.
2. Land the original low-risk copy-elision fixes.
3. Add the **minimal safe** `extract_pattern_matrix(...)` rewrite.
4. Run debug + release verification.
5. Measure on the existing app-path / ML workflow evidence paths.
6. Only then decide whether the bigger ML-shape changes are justified.

That order keeps the work aligned with your main goal:

- improve performance,
- preserve correctness,
- avoid feature loss,
- avoid behavior drift,
- and defer the larger design shifts until the small safe wins have been measured.
