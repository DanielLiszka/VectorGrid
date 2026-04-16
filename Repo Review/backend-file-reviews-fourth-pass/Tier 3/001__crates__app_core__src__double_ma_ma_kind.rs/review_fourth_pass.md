# Fourth-Pass Review: `crates/app_core/src/double_ma_ma_kind.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/double_ma_ma_kind.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/double_ma_ma_kind.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/double_ma_ma_kind.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/double_ma_ma_kind.rs.review.md`
- Third-pass action bucket: `Accept as-is`
- Fourth-pass action bucket: `Accept as-is`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file still looks like a small, low-risk support helper, so the fourth pass was not about manufacturing novelty. It was justified because the live third-pass packet is materially misaligned with the actual file.

`double_ma_ma_kind.rs` is not a heuristic or approximation helper. It is a tiny registry-backed contract gate: `lookup_ma_info` performs the MA descriptor lookup, and `validate_ma_kind` trims input, rejects unknown MA ids, and rejects shapes that are not both `period_based` and `single_output` (`crates/app_core/src/double_ma_ma_kind.rs:1-23`). The repo map also explicitly describes this file as the Rust-side owner of MA acceptance, period-based constraints, and single-output checks (`context__REPO_MAP.md:934-935`).

That matters because the current third-pass prose talks about tolerant parsing, approximation boundaries, and heuristic policy. Those claims do not match the code under review. The bucket can still be right while the packet rationale is wrong.

## Fourth-Pass Research Log

1. I reread the live source first. The file has only two helpers and two tests. The implementation is straightforward, but the tests currently cover only one positive path and unknown-id rejection (`crates/app_core/src/double_ma_ma_kind.rs:29-43`).

2. I checked surrounding ownership in the local context files. `validate_ma_kind` is called in Double-MA drilldown, local-stability, optimization, and validation entry paths (`context__02__crates__app_core__src__double_ma.rs:4491-4494`, `4658-4666`, `5045-5052`; `context__03__crates__app_core__src__double_ma_validate.rs:1856-1862`). `lookup_ma_info` also feeds backend support decisions, candle-requirement routing, and MA-series building (`context__02__crates__app_core__src__double_ma.rs:5222-5230`, `5306-5314`, `8045-8047`, `10757-10764`).

3. I checked the app-core export surface. `lib.rs` reexports the public optimization and validation entrypoints that depend on these checks, so the `String` errors emitted here are effectively cross-layer contract behavior, even though this helper itself is only `pub(crate)` (`context__01__crates__app_core__src__lib.rs:103-111`, `144-148`).

4. I reviewed the current public Rust and VectorTA docs to challenge weak assumptions rather than rely on memory. The Rust API Guidelines still call out argument validation and meaningful errors as core dependability/interoperability concerns, and the standard library still documents `eq_ignore_ascii_case` as an ASCII-only comparison that avoids allocating lowercase temporaries. The current public VectorTA moving-average registry source shows that `MovingAverageInfo` carries `period_based` and `single_output` flags and that `list_moving_averages()` returns a static slice. The public registry also shows both non-period-based entries and period-based multi-output entries, so both rejection branches in `validate_ma_kind` are realistic, not hypothetical.

5. I treated the external VectorTA registry evidence conservatively. The exact pinned `vector-ta` version is not present in this bundle, so concrete ids from the public docs should be treated as illustrative only. Any additional tests in this repo should select registry examples dynamically rather than hard-code ids copied from external docs.

## What Fourth Pass Sharpened

### 1. The third-pass action bucket should stand, but the third-pass packet should not

The file is still appropriately bucketed as `Accept as-is`. I did not find a correctness bug, a performance problem worth fixing now, or a contract weakness large enough to justify escalating this into a rewrite or a required code change.

What does not hold up is the third-pass explanation. Its framing around tolerant parsing, approximation, and heuristics appears to be stale or template-contaminated. For this file, that is not a harmless wording issue; it obscures the real preservation constraints.

### 2. What the current third-pass packet got right

The third-pass packet was right about one important thing: resist cleanup-for-cleanup’s-sake. This module is small, readable, and already does its job clearly. The right default remains to preserve behavior.

It was also directionally right to avoid turning a tiny helper into a broader redesign surface. Nothing in the live code justifies a cache layer, typed wrapper, or broader API churn as immediate work.

### 3. What it missed or underweighted

The biggest miss is that the real contract here is not “tolerant parsing.” It is split normalization plus shape gating.

`lookup_ma_info` does an ASCII case-insensitive lookup, but it does not trim (`crates/app_core/src/double_ma_ma_kind.rs:1-7`). `validate_ma_kind` does trim before lookup and before constructing its error strings (`crates/app_core/src/double_ma_ma_kind.rs:9-23`). That asymmetry is fine today because the visible entrypoints either normalize before calling the helper or call `validate_ma_kind` directly. But it is still a real contract boundary worth documenting and testing. The earlier packets noticed case-insensitive behavior but did not fully surface this split.

The earlier packets also under-specified the negative-test backlog. Saying “add a multi-output rejection test” is not precise enough. Because `validate_ma_kind` checks `period_based` first and `single_output` second, a test that picks an MA with both flags false will only exercise the first rejection branch. To actually pin both branches, the test matrix needs:

- one registry-driven example where `period_based == false`; and
- one registry-driven example where `period_based == true && single_output == false`.

That is a more useful sharpening than the prior generic “if present” advice.

Finally, the third-pass packet underweighted how many downstream behaviors depend on this registry lookup. `lookup_ma_info` is not used only for acceptance/rejection. It also influences CPU/CUDA support checks, candle-based input selection, and MA-series construction in the surrounding Double-MA engine (`context__02__crates__app_core__src__double_ma.rs:5222-5230`, `5306-5314`, `8045-8047`, `10757-10764`). That means any broadening of accepted MA shapes is not a local refactor; it can spill into backend selection and validation behavior.

### 4. What the current third-pass packet overstated

The current third-pass packet overstates the need to preserve “heuristic” or “approximation” behavior because this file does not implement those concepts.

It also implies that future tightening should center on policy heuristics. For this file, the smaller and more accurate guidance is different: preserve the actual normalization and rejection contract, and pin it with focused tests. That is a materially narrower claim.

## Implementation Priority

### Safe-now hardening

The smallest justified hardening remains docs-and-tests work, not runtime behavior change.

First, add tests that pin the actual current normalization contract. The file should explicitly test case-insensitive lookup and trimmed validation acceptance for a dynamically chosen MA that is both period-based and single-output. That would finally back up the second-pass claim that case-insensitive acceptance is deliberate contract behavior.

Second, add two registry-driven reject-path tests with selectors that match the real branch structure:

- a non-period-based MA to pin the first rejection branch; and
- a period-based but multi-output MA to pin the second rejection branch.

Do not hard-code example ids copied from external docs. The exact `vector-ta` version used by this repo is not present in this review bundle, so the tests should derive candidates from the runtime registry. If the project wants those tests to stay resilient across upstream version bumps, dynamic selection is the least regression-prone way to do it.

Third, add a short module-level ownership comment or function comment clarifying the split contract: `lookup_ma_info` performs ASCII-case-insensitive registry lookup, while `validate_ma_kind` adds trimming and Double-MA suitability checks.

I would keep empty-input behavior out of the immediate change set. Today, optimization and validation entrypoints perform explicit empty checks before calling `validate_ma_kind`, while drilldown and local stability rely on the helper and currently surface empty input as “unknown moving average type” (`context__02__crates__app_core__src__double_ma.rs:5047-5048`; `context__03__crates__app_core__src__double_ma_validate.rs:1858-1859`; compare `context__02__crates__app_core__src__double_ma.rs:4491-4494`, `4663-4666`). That inconsistency is real, but changing it would alter user-visible wording. It is not large enough to disturb the current bucket.

### Larger redesign still deferred

A few ideas may look tempting, but they should stay deferred.

A local cache or `OnceLock<HashMap<_, _>>` for MA lookup is not justified by the evidence in this bundle. The upstream registry already exposes a static slice, the helper is tiny, and there is no profiling evidence here that this lookup is a meaningful bottleneck. Introducing a second registry representation would add code and initialization behavior for very uncertain gain.

Likewise, a richer typed error or a `ValidatedMaKind` wrapper could be reasonable as part of a larger cleanup that removes repeated stringly-typed MA handling across `double_ma.rs`. But that is a cross-file redesign, not a surgical improvement to this helper. It would touch error wording, call sites, and possibly serialization-facing request flows, so it is not appropriate as stand-alone hardening for a Tier-3 accept-as-is file.

Finally, do not broaden the eligibility rule beyond “known + period-based + single-output” without auditing every `lookup_ma_info` consumer in the Double-MA engine. The current downstream code uses registry metadata for backend capability and input-shape planning, so relaxing this gate is not a free improvement.

## Verification If Changed

If the only changes are comments and local unit tests, the narrow verification target is still appropriate:

- `cargo test -p ta_app_core validate_ma_kind -- --nocapture`

If behavior changes are made instead of just hardening tests/docs, that is no longer enough. In that case, rerun targeted coverage for the four entry surfaces that call this helper:

- Double-MA optimization request handling
- drilldown request handling
- local-stability request handling
- Double-MA validation request handling

The reason is simple: this helper is shared across those surfaces, and seemingly small normalization or error-message edits can show up differently in each one.

## Regression And Feature-Preservation Constraints

Preserve the current acceptance rule: only registry-known MAs that are both period-based and single-output are eligible here.

Preserve ASCII case-insensitive lookup. Tightening that to exact-match lookup would be a request-compatibility change, not cleanup.

Do not silently widen `lookup_ma_info` to trim whitespace unless that is an intentional contract change. The current split between raw lookup and trimmed validation is imperfect but stable; changing it affects more than tests.

Preserve current branch intent when writing new tests. A multi-output example that is also non-period-based does not verify the second rejection path.

Be careful with error-string assertions. These `String` errors flow through public optimization and validation entrypoints, so wording changes are cross-layer behavior changes. Focus new tests on branch meaning unless the product specifically wants exact message text pinned.

Do not overstate performance risk. There is no evidence in this bundle that this helper sits on a hot per-bar or per-candidate path. Premature caching would add tech debt more readily than it would remove it.

There is no direct GPU ABI or persistence risk in the file itself, but there is indirect backend-routing risk: changing what counts as an acceptable MA kind can leak into CPU/CUDA support checks and candle-input selection in surrounding code.

## Fourth-Pass Conclusion

The current third-pass **bucket** should stand: `Accept as-is` is still the right action bucket for the live source.

The current third-pass **packet** should not stand as written. Its rationale is materially off-target for this file and misses the real contract details that are worth preserving.

The real fourth-pass conclusion is narrower and more evidence-backed:

- the code is still fundamentally fine;
- the worthwhile near-term hardening is in focused tests and a brief ownership comment; and
- broader redesign ideas should stay deferred unless the project is already doing a wider cleanup of MA normalization and error handling across the Double-MA stack.
