# Fourth-Pass Review: `crates/optimizer/src/grid.rs`

- Review date: `2026-04-16`
- Source file: `crates/optimizer/src/grid.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/optimizer/src/grid.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/optimizer/src/grid.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/optimizer/src/grid.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The live file is still small, but the prior packets converged too quickly on “comments only.” That is too narrow for the actual contract surface.

The third-pass packet was right about one important thing: this file is not just a convenience helper. It emits a shared `OptimizationResult` DTO and its ranking/output behavior is observable outside the module. That part of the framing still stands.

What warranted a fourth pass is that the earlier packets underweighted a more concrete correctness issue than documentation drift: `grid_search()` assumes `BacktestEngine::eval_batch()` returns exactly one `Metrics` row per input combo, but neither the trait nor the function enforces that contract (`crates/optimizer/src/lib.rs:30-31`, `crates/optimizer/src/grid.rs:33-88`). With a short metrics vector, `all` truncates silently while `num_combos` still reports the original input length (`crates/optimizer/src/grid.rs:66-87`). With a long metrics vector, `best_params` and `top` can index past `combos` and panic (`crates/optimizer/src/grid.rs:49-64`). That is a real safe-now hardening target, not just a comment opportunity.

So the current third-pass bucket should stand, but the content of that bucket should change: this is still a small surgical refactor, just not a comment-only one.

## Fourth-Pass Research Log

1. Re-read the live `grid.rs` and the third-pass packet first, per bundle instructions.
   - Focus: whether the third-pass framing still matches the live code.

2. Re-read `crates/optimizer/src/lib.rs` to verify the public DTO and trait contract.
   - `BacktestEngine` is a public trait with no cardinality contract in its signature (`crates/optimizer/src/lib.rs:30-31`).
   - `OptimizationResult` is serialized/deserialized, and several optional fields are omitted from serialized output when `None` (`crates/optimizer/src/lib.rs:53-69`).

3. Checked current Rust `slice::sort_by` behavior in the standard-library docs.
   - Relevant point: `sort_by` is stable and preserves the initial order of equal elements.
   - Why it matters here: `grid_search()` sorts an index vector built in input order, so equal scores currently keep input order in `top` (`crates/optimizer/src/grid.rs:55-64`). That is more specific than the prior packet’s generic “ordering matters” warning.

4. Checked current Rust guidance on panic-vs-contract violations.
   - Relevant point: panicking can be appropriate when a documented contract violation indicates a caller bug.
   - Why it matters here: if `eval_batch()` cardinality is a hard contract, silently truncating on under-delivery and panicking later on over-delivery is the worst of both worlds. The code should pick a deliberate contract and enforce it consistently.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was correct to keep the file in `Small surgical refactor`. Nothing in the live source justifies escalating this to a broad redesign. It was also correct that emitted ordering and result-shape semantics are compatibility-sensitive, because `OptimizationResult` is a serialized DTO, not a purely local struct (`crates/optimizer/src/lib.rs:53-69`).

It was also directionally right to be conservative about “performance cleanups.” This file is small, and any change that touches ranking or payload shape can ripple into downstream optimize, report, or artifact flows.

### What it missed or underweighted

#### 1. The `BacktestEngine` cardinality contract is the main live correctness gap

This is the biggest miss.

`grid_search()` accepts `combos`, then trusts `engine.eval_batch(combos)` without verifying that the returned vector length matches `combos.len()` (`crates/optimizer/src/grid.rs:33-35`). The function only checks `metrics.is_empty()`, which means:

- if `metrics.len() < combos.len()`, `best` and `top` only reflect a prefix of the requested search space, `all` truncates to the shorter length because `zip` truncates, but `num_combos` still reports the full input length (`crates/optimizer/src/grid.rs:66-87`);
- if `metrics.len() > combos.len()`, `best_params = combos[best_idx]` and the `top` mapping can panic (`crates/optimizer/src/grid.rs:49-64`).

That is more serious than the third-pass packet’s framing that result-shape compatibility is “the real constraint.” Result-shape compatibility matters, but this cardinality assumption is an actual correctness hole.

#### 2. The score fields are normalized ranking scores, not raw metric echoes

The earlier packets focused on non-finite clamping, but they still under-described what is actually serialized.

For `Volatility` and `MaxDrawdown`, the score helper negates the raw metric before ranking (`crates/optimizer/src/grid.rs:11-13`). As a result, `best_score` and `top_scores` are optimization-direction scores, while `best_metrics` and `top` keep the raw metrics (`crates/optimizer/src/grid.rs:77-84`). That is an observable DTO contract, not an implementation detail, and it matters for IPC/persistence because `OptimizationResult` is serialized (`crates/optimizer/src/lib.rs:53-69`).

That contract is also under-tested: the local tests only exercise `Sharpe` (`crates/optimizer/src/grid.rs:123-185`).

#### 3. The current ordering contract is more specific than “preserve ordering”

The third-pass packet correctly warned about ordering, but it stayed too generic.

Today the code does not just produce “some deterministic order.” It sorts an index vector built as `0..metrics.len()`, uses `sort_by`, and compares descending normalized scores (`crates/optimizer/src/grid.rs:55-60`). Because the sort is stable, equal scores currently preserve input order. That means the live top-k contract is effectively:

- descending normalized score;
- ties keep original combo order.

That tie behavior is a real compatibility surface. A seemingly harmless switch to `sort_unstable`, a heap, or a partial-selection path could change it.

#### 4. The local tests are not as complete as the prior packets implied

Saying the tests are “otherwise solid” was too strong.

The existing tests pin:

- non-finite clamping for a `Sharpe = NaN` case;
- parity with `StreamAggregator` for `Sharpe` best/top-k/top-scores and `all` length (`crates/optimizer/src/grid.rs:123-185`).

They do **not** pin:

- `+inf` behavior, even though `score()` demotes every non-finite value to `NEG_INFINITY` (`crates/optimizer/src/grid.rs:15-18`);
- risk-objective sign semantics for `Volatility` and `MaxDrawdown` (`crates/optimizer/src/grid.rs:11-13`);
- tie ordering;
- `top_k == 0` shape, especially `top_scores: Some(vec![])`;
- the current `all_scores: None` omission;
- the cardinality mismatch contract.

So the third-pass packet’s emphasis on fixtures was right, but it did not identify the fixtures that are still missing.

### Claims from the current packet that are too weak or too strong

The strongest overstatement is the idea that comment-level hardening is the useful improvement. That is too weak for the live file.

The strongest overstatement on the other side is that output-shape compatibility is the main remaining issue. It is one of the issues, but it is not the only one, and the unguarded `eval_batch()` cardinality assumption is at least as important.

There is also a milder overstatement in calling this a fully established “compatibility path.” The live tests prove partial parity with stream aggregation, not total parity across all objectives and all emitted fields (`crates/optimizer/src/grid.rs:145-185`).

## Implementation Priority

### Safe-now hardening

1. **Make the `BacktestEngine` cardinality contract explicit and enforce it deliberately.**
   - At minimum, document that `eval_batch()` must return exactly one `Metrics` row per input combo (`crates/optimizer/src/lib.rs:30-31`).
   - Then enforce that assumption in `grid_search()` instead of leaving the current silent-truncation/possible-panic split in place (`crates/optimizer/src/grid.rs:33-88`).
   - I would treat this as a contract violation, not a recoverable “no result” case. If the project chooses fail-fast, the panic message should be explicit. If it chooses graceful rejection, it should not reuse the current empty-result ambiguity casually.

2. **Document and test that score fields are normalized ranking scores.**
   - Add targeted coverage for `Volatility` and `MaxDrawdown` so the sign convention is pinned.
   - Add a direct test that `+inf` is also demoted, not just `NaN`, because the live helper treats every non-finite as worst rank (`crates/optimizer/src/grid.rs:15-18`).

3. **Add tests for the currently live result-shape edges.**
   - `top_k == 0` should pin `top = []` and the exact `top_scores` shape.
   - `include_all = true` should pin whether `all_scores` remains `None`.
   - equal-score inputs should pin tie ordering.

4. **Only if the file is already being touched, consider a tiny score-caching cleanup.**
   - Today `score()` is recomputed during the best scan, again throughout sorting, and again when building `top_scores` (`crates/optimizer/src/grid.rs:38-45`, `55-60`, `72-75`).
   - Precomputing scores once is a reasonable micro-cleanup, but only after the ordering/tie semantics are pinned so a “performance tweak” does not quietly change behavior.

### Larger redesign still deferred

1. **Do not casually unify grid and stream scoring until the shared contract is frozen.**
   - A shared helper is plausible and probably the right eventual destination.
   - It should still be deferred until the repo has explicit tests for normalized-score semantics, tie behavior, and mismatch handling.

2. **Do not switch top-k extraction away from full stable sorting yet.**
   - Heap or partial-selection paths may be faster when `top_k << n`, but they are exactly the kind of refactor that tends to perturb tie order or result ordering.
   - That is a redesign task, not today’s safe-now hardening.

3. **Do not start populating `all_scores` without treating it as a DTO migration.**
   - `OptimizationResult` skips `None` fields during serialization (`crates/optimizer/src/lib.rs:57-67`).
   - Filling `all_scores` would change payload shape and size for IPC, persisted artifacts, and any downstream serde consumers. Even if the new field is “obviously useful,” it is not a no-risk cleanup.

## Verification If Changed

Minimum verification for the safe-now items:

- `cargo test -p ta_optimizer -- --nocapture`

If any change touches `OptimizationResult` field population or serialized shape rather than just docs/tests/guarding:

- `cargo test --workspace`
- `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeWorkspaceTests`

The important test additions are local and specific:

- one test for risk-objective score polarity;
- one test for `+inf` clamping;
- one test for equal-score tie order;
- one test for `top_k == 0` and `all_scores` omission;
- one test for the chosen cardinality-mismatch contract.

## Regression And Feature-Preservation Constraints

- Preserve the current normalized-score behavior unless grid and stream move together. The non-finite clamp is not just a ranking detail; it is also part of making the sort comparator safe to use.
- Preserve the distinction between raw metrics and normalized scores unless the DTO is renamed or migrated deliberately.
- Treat current tie ordering as observable. Any move away from stable sorting needs explicit sign-off and tests.
- Do not populate `all_scores` casually. That is an IPC/persistence payload change because `OptimizationResult` is serialized and `None` fields are currently omitted (`crates/optimizer/src/lib.rs:57-67`).
- There is no direct GPU ABI risk in this file, but there **is** shared optimizer-contract risk. Changes here can propagate into optimize result handling, report generation, and persisted artifacts even though the file itself is CPU-only support code.
- If cardinality is enforced, enforce it explicitly and consistently. The current behavior is neither a clean no-result path nor a clean fail-fast contract.

## Fourth-Pass Conclusion

`Bucket unchanged.` The current third-pass bucket should stand as `Small surgical refactor`.

What should change is the content of the recommendation. The earlier packets were right to be conservative, but they were too comment-centric. The live file still deserves a small change set, yet that change set should center on **contract hardening and tests**, not just prose:

- make the `eval_batch()` cardinality assumption explicit and deliberate;
- pin normalized-score semantics, especially for risk objectives and non-finite values;
- pin tie ordering and the current result-shape omissions before any cleanup.

Anything larger than that—shared scorer extraction, new top-k selection machinery, or filling `all_scores`—should stay deferred because those changes carry avoidable compatibility, IPC, and persistence risk for a file this small.
