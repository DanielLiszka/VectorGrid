# Fourth-Pass Review: `crates/app_core/src/indicator_factory.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/indicator_factory.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file warranted a fourth pass because the third-pass packet kept the right bucket but got there with reasoning that was more generic than this source really supports. Reopening the live file shows a tiny public wrapper with only two local tests, a public matrix DTO, and a narrow `compute_moving_average_batch_cpu()` entrypoint. That means the real question is not whether the bucket should change; it is whether the rationale behind that bucket is still precise enough to be useful.

After re-reading the live source, the second-pass framing remains closer to the truth than the third-pass framing. The live file is still a small CPU-only moving-average batch facade: `IndicatorMatrixF64` is publicly constructible and deserializable, `validate_uniform_step()` enforces an arithmetic period grid, and `compute_moving_average_batch_cpu()` trims and normalizes input before delegating the actual MA work to `crate::double_ma` through `resolve_alma_params_for_side()` and `ma_matrix_cpu_f64()` (source lines 6-115). That still points to small, contract-sensitive hardening, not redesign.

So the current third-pass bucket should **stand**, but for narrower and more concrete reasons than the third-pass packet gave.

## Fourth-Pass Research Log

I re-read the live source first, then compared it against the first-, second-, and third-pass packets to identify where the review logic had drifted.

Local source checks that mattered most:

- `IndicatorMatrixF64` still exposes `pub rows`, `pub cols`, and `pub values`, while `try_new()` is only an optional checked path (lines 6-34).
- `row()` still returns `None` for zero-column matrices, even when `row < rows` (lines 28-33).
- `MovingAverageBatchRequest` still accepts `ma_params`, but `MovingAverageBatchResponse` does not echo either the raw params or any normalized/resolved parameter state (lines 37-50, 96-114).
- `compute_moving_average_batch_cpu()` still lowercases `ma_type`, trims `ma_source`, enforces a uniform-step period grid, and delegates the real computation (lines 79-115).
- The module still has only two local tests, covering one smoke path and one non-uniform rejection case (lines 118-164).

Repo-context checks that mattered:

- `indicator_factory` is still a public module exported from `crates/app_core/src/lib.rs`, so this is a real crate surface even though it is not a Tauri command or GPU path.
- `REPO_MAP.md` still places this file in the support-module layer, not a hot-path or GPU-runtime ownership layer, which argues against performance chasing or architectural churn here.

External references consulted to pressure-test the public-surface recommendations:

1. Rust API Guidelines — Documentation  
   <https://rust-lang.github.io/api-guidelines/documentation.html>  
   Used to re-check whether the current doc comment and type framing are honest enough for a public support-module API.

2. Rust API Guidelines — Naming  
   <https://rust-lang.github.io/api-guidelines/naming.html>  
   Used to evaluate whether “indicator factory” language still overstates what this module really does.

3. Rust API Guidelines — Dependability  
   <https://rust-lang.github.io/api-guidelines/dependability.html>  
   Used to sanity-check the “preserve behavior, add tests first” posture on a public contract seam.

4. Serde — Field attributes  
   <https://serde.rs/field-attrs.html>  
   Used to distinguish real serde-visible behavior from generic boilerplate concerns.

5. Serde — Container attributes  
   <https://serde.rs/container-attrs.html>  
   Used to verify that `try_from`-style deserialization hardening is available, but would only solve the deserialize path unless field visibility changes too.

6. Serde — Implementing `Deserialize`  
   <https://serde.rs/impl-deserialize.html>  
   Used to confirm that invariant-preserving deserialization is possible if the project later decides this DTO must become harder to misuse.

7. `serde_json` docs (`Number::from_f64`, `Value::from(f64)`)  
   <https://docs.rs/serde_json/latest/serde_json/struct.Number.html>  
   <https://docs.rs/serde_json/latest/serde_json/enum.Value.html>  
   Used to validate a future wire-format risk: JSON does not preserve non-finite floats as ordinary JSON numbers.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep this file in `Small surgical refactor`. The live source is still too small, too delegated, and too contract-sensitive to justify a larger cleanup campaign. It was also right that tests and documentation are more valuable here than representational churn.

That part still stands.

### What the third-pass packet overstated

The third-pass packet reads as if this file were mainly a defaults-heavy, omission-sensitive serde DTO seam. The live source does not really support that emphasis.

There are no serde renames, no `skip_*` attributes, no `flatten`, no `deny_unknown_fields`, and no enum label surface here. The only clearly optional serde field is `ma_params: Option<HashMap<String, f64>>`. So language like “defaults, sentinels, and exported labels” or “defaults, omission behavior, and exported labels” is noticeably more generic than the actual file.

That does not make the third-pass conclusion wrong. It just makes the reasoning weakly tailored. The concrete public-contract issues in this file are still the ones earlier packets already identified more directly: invariant leakage, narrow-but-underdocumented wrapper semantics, and thin boundary tests.

### What the third-pass packet underweighted or missed

#### 1. The real invariant problem is still `IndicatorMatrixF64`, and serde-only hardening would not fully solve it

The second-pass packet was more precise here than the third-pass packet. `IndicatorMatrixF64::try_new()` checks `rows * cols == values.len()`, but the same type still has public fields and derives `Deserialize`. That makes the constructor advisory rather than authoritative.

The important fourth-pass refinement is this: a custom `Deserialize` or `#[serde(try_from = ...)]` path would harden **deserialization**, but it would still leave **direct public construction** open unless the fields become private. So any future recommendation must separate:

- **partial hardening**: validate on deserialize
- **full hardening**: make direct invalid construction impossible too

That distinction matters because the first option is smaller, while the second is a public-API shape change. Earlier packets hinted at this, but none said it explicitly enough.

#### 2. The zero-width row contract is still a real semantics question, and the safe move is documentation or tests, not behavior churn

`try_new()` accepts `rows > 0, cols == 0, values.is_empty()`, while `row()` returns `None` whenever `cols == 0`, even for an in-range row. That makes “empty but in-range row” indistinguishable from “out of range.”

The first pass already spotted this. The third pass effectively dropped it.

Fourth-pass view: this is still worth calling out, but it is not a reason to change behavior immediately. Returning `Some(&[])` instead would be observable behavior change; rejecting zero-width matrices would also be behavior change. The safe-now move is to document whichever contract the project intends, then add a focused test so later cleanup does not accidentally pick a different one.

#### 3. The wrapper’s observable normalization behavior is more important than the third-pass packet acknowledged

The source still normalizes `ma_type` with `trim().to_ascii_lowercase()`, but only trims `ma_source` without lowercasing it. The response then echoes those normalized values, not the raw request. That is subtle but public behavior.

This matters more than the third-pass packet’s generic serde wording, because it is live today and visible through the function result. If changed casually, it could break callers that compare echoed fields or expect source identifiers to remain case-sensitive.

Safe-now action is documentation and a thin boundary test, not normalization churn.

#### 4. The response is not fully self-describing for parameterized moving averages

This is the clearest new gap the earlier packets did not spell out.

`MovingAverageBatchRequest` accepts `ma_params`, and `compute_moving_average_batch_cpu()` uses them directly and also feeds them through ALMA-parameter resolution. But `MovingAverageBatchResponse` returns only `ma_type`, `ma_source`, `periods`, and `matrix`; it does **not** include `ma_params`, nor any normalized/resolved parameter payload.

That omission is not automatically wrong. A thin compute API can reasonably expect the caller to retain its request. But it does mean the response is not self-describing for parameterized MAs. If a caller serializes or caches only the response, the exact parameter provenance is gone.

That is not a safe-now DTO change because adding params to the response is a public serialized-shape change. But it is important enough to document, and it is a better file-specific observation than the third-pass packet’s generic “exported labels” language.

#### 5. There is a plausible future wire-format hazard around non-finite float values

The first-pass packet cited an existing delegated test named `cpu_ma_matrix_rowwise_scalar_fallback_fills_skipped_rows_with_nan`, which strongly suggests the underlying MA matrix surface intentionally uses `NaN` in warmup regions. Separately, the `serde_json` docs say non-finite floats are not JSON numbers, and `Value::from(f64)` converts `NaN`/infinite values to `Null`.

That combination means these derives should not be mistaken for a drop-in JSON contract if this DTO ever crosses JSON IPC, persistence, or export surfaces. I do **not** see evidence of that today in the provided bundle, so this is not a current bug report. But it is a real future compatibility constraint, and the third-pass packet missed it entirely.

If this surface ever becomes a real wire DTO, the project should revisit both:

- how non-finite matrix values are represented
- whether `usize` dimensions are appropriate for a portable serialized shape

That is deferred work, not a present refactor ask.

#### 6. The current tests are too thin for the actual boundary behavior this wrapper owns

The live module still tests only one success case and one non-uniform-period failure case. That is enough to prove the file is wired up, but not enough to pin its actual public contract.

The most valuable missing tests are still small and local:

- empty `ma_type`
- empty `ma_source`
- empty `periods`
- duplicate or descending periods
- single-period success
- normalization echo behavior (`" SMA "` -> `"sma"`, trimmed source behavior)
- invalid ALMA params if the delegated error contract is stable enough
- zero-width `row()` semantics if that representable state is kept
- zero-period rejection, but only as a wrapper-level test if the real rule intentionally stays delegated

### Safe-now hardening

The safest next changes are still small and local.

First, rewrite the docs to describe the file honestly: this is a CPU-only moving-average batch facade over `double_ma`, not a general indicator factory. The docs should also say that the batch is restricted to a strictly increasing **uniform-step** period grid, and that `matrix.row(i)` corresponds to `periods[i]`.

Second, document the DTO contract more explicitly. `IndicatorMatrixF64` should say whether direct field construction is supported, what invariant callers must preserve if it is, and what `row()` means for zero-width matrices. `MovingAverageBatchResponse` should say that it echoes normalized `ma_type` and trimmed `ma_source`, but does not carry request parameter provenance.

Third, add boundary tests before touching representation:
- invalid empty fields
- duplicate/decreasing periods
- single-period acceptance
- normalization behavior
- zero-width row semantics
- invalid parameter passthrough behavior where stable

These changes reduce ambiguity without changing the DTO shape, serialized field names, normalization policy, delegated math, or error ownership.

### Larger redesign still deferred

A few bigger ideas remain reasonable, but they should stay deferred.

Making `IndicatorMatrixF64` fields private and enforcing construction through checked paths would be the cleanest invariant story, but it is a public-API change. A custom or `try_from`-based deserialize path is smaller, but it only closes one bypass path unless visibility changes too.

Likewise, renaming the module or types away from the broader “indicator factory” framing could improve public clarity, but that is also a compatibility decision, not a cleanup patch.

A typed error enum is not justified for a wrapper that mostly forwards deeper errors.

And if this surface ever becomes JSON-facing or artifact-facing, that work should be handled as a coordinated contract redesign, not as an incidental refactor in this file.

## Implementation Priority

**Priority 1 — safe now**

1. Tighten rustdoc on the module/types/function so the actual contract is explicit:
   - CPU-only
   - moving-average-only
   - uniform-step grid only
   - row-major matrix orientation
   - normalization behavior
   - response is not self-describing for `ma_params`

2. Add boundary tests for the wrapper-specific behavior the file itself owns.

3. If the team wants a tiny extra hardening step without changing public shape, prefer documentation plus tests over code-path churn.

**Priority 2 — only with explicit compatibility review**

1. Make `IndicatorMatrixF64` fields private and force checked construction.
2. Add custom/`try_from` deserialization for invariant preservation.
3. Revisit response shape if parameter provenance needs to survive serialization or caching.
4. Revisit float and dimension types if this ever becomes a real wire/persistence DTO.

## Verification If Changed

At minimum, keep the existing local coverage and add focused wrapper tests.

Existing checks already cited in earlier packets remain the right baseline:

- `cargo test -p ta_app_core indicator_factory::tests:: -- --nocapture`
- `cargo test -p ta_app_core cpu_ma_params_gaussian_poles_changes_output -- --nocapture`
- `cargo test -p ta_app_core cpu_ma_matrix_rowwise_scalar_fallback_fills_skipped_rows_with_nan -- --nocapture`

Add wrapper-local tests for:

- empty `ma_type`
- empty `ma_source`
- empty `periods`
- duplicate and descending periods
- single-period success
- normalization echo behavior
- zero-width `row()` semantics if representable
- invariant-preserving deserialize behavior, but only if deserialize hardening is actually changed

If the response shape or serde behavior changes, add round-trip tests for the exact chosen serialization format rather than assuming the derives are enough.

## Regression And Feature-Preservation Constraints

Preserve the current delegated MA behavior in `double_ma`. This file is a facade, not the owner of MA math, warmup semantics, or performance behavior.

Do not casually “clean up” the saturating arithmetic in `row()` unless the invariant story changes too. In the current shape, that defensive arithmetic helps malformed states fail harmlessly with `None` rather than relying on unchecked multiplication or slicing.

Do not change `ma_source` normalization opportunistically. Lowercasing it, canonicalizing it, or trying to echo raw input instead of normalized output would all be observable contract changes.

Do not change zero-width row behavior without treating it as a semantics decision. Returning `Some(&[])`, rejecting zero-column matrices, or preserving the current `None` behavior are all plausible choices, but they are not equivalent.

Do not add `ma_params` to the response as a “small improvement” without acknowledging that it changes the serialized/public DTO shape.

Do not treat `Serialize` / `Deserialize` derives here as proof that this is a ready-made JSON or IPC DTO. Non-finite float handling and architecture-sized integers both need explicit thought before that claim would be safe.

Performance, GPU ABI, and CUDA risks are minimal here because the live file is CPU-only and delegates the hot work elsewhere. The real risks are public-contract drift, serialized-shape drift, and boundary-test undercoverage.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `crates/app_core/src/indicator_factory.rs` is still a `Small surgical refactor`, not a redesign target.

But the third-pass reasoning should be tightened. Its “defaults / omission / labels” framing is too generic for this live file. The sharper file-specific story is:

- public invariant leakage in `IndicatorMatrixF64`
- ambiguous zero-width row semantics
- underdocumented normalization and matrix-shape behavior
- response DTOs that are not fully self-describing for parameterized MAs
- thin wrapper-local tests
- a deferred, but real, future wire-format caveat if these float-heavy DTOs ever become JSON-facing

So the right fourth-pass recommendation is conservative:

**safe now**: improve docs, add boundary tests, and make the current behavior explicit.

**still deferred**: representation changes, deserialize hardening that changes public shape, response-shape expansion, renames, and any wire-format redesign.
