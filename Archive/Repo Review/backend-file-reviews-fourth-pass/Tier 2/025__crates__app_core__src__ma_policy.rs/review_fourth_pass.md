# Fourth-Pass Review: `crates/app_core/src/ma_policy.rs`

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/ma_policy.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/ma_policy.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ma_policy.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ma_policy.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet kept the right bucket, but its reasoning was too generic for the live file. `ma_policy.rs` is not mainly a serde/default/DTO seam. The live source is a small route-policy module that owns three behavior-critical things at once: static kernel-capability policy (`GPU_KERNEL_SUPPORTED_MAS`, `GPU_KERNEL_PREFERRED_MAS`), build-embedded speedup-table parsing (`include_str!` + `OnceLock`), and the public helpers/decision structs that downstream route selection can consume (`supports_gpu_kernel_ma`, `prefers_gpu_kernel_ma`, `plan_hybrid_indicator_route`, and friends) (`crates/app_core/src/ma_policy.rs:4-327`).

That distinction matters because the real next-step hardening is narrower than the earlier packet implied. The highest-value work here is not broad compatibility churn. It is tightening the semantic contract around three concrete seams the current third-pass packet underweighted: helper naming versus actual behavior, inconsistent normalization across helpers and fallback paths, and first-row CSV parsing edge cases that can silently change which policy source becomes authoritative.

The current third-pass bucket should **stand**. This is still a `Small surgical refactor`. But the justification should shift away from generic “serde-visible behavior” language and toward precise policy-contract hardening.

## Fourth-Pass Research Log

1. Live source and prior review packets
   - Files consulted:
     - `source__crates__app_core__src__ma_policy.rs`
     - `review__third_pass.md`
     - `review__second_pass.md`
     - `review__first_pass.md`
     - `context__01__crates__app_core__src__lib.rs`
     - `context__REPO_MAP.md`
   - Why consulted:
     - verify the live route-policy semantics, identify what the current packet actually challenged, and separate public-surface risk from internal-only cleanup.

2. Rust Reference, input format
   - Source:
     - `https://doc.rust-lang.org/reference/input-format.html`
   - Why consulted:
     - verify whether `include_str!` removes a UTF-8 BOM before the CSV parser sees the embedded file.
   - Constraint informed:
     - `include_str!` does **not** apply BOM removal, so the first-row header check in `parse_gpu_speedup_ids` is BOM-sensitive.

3. `include_str!` documentation
   - Source:
     - `https://doc.rust-lang.org/std/macro.include_str.html`
   - Why consulted:
     - confirm this file really embeds a compile-time string contract rather than loading a runtime artifact.
   - Constraint informed:
     - parser hardening is preferable to runtime file-recovery logic, because the policy source is compiled into the binary.

4. RFC 4180
   - Source:
     - `https://www.rfc-editor.org/rfc/rfc4180.html`
   - Why consulted:
     - reassess the current custom CSV header detection against normal quoted-header behavior.
   - Constraint informed:
     - quoted CSV fields are legitimate, so a raw `starts_with("indicator_id,")` gate is a weaker header check than comparing the parsed first field.

5. Rust API Guidelines, documentation and naming
   - Sources:
     - `https://rust-lang.github.io/api-guidelines/documentation.html`
     - `https://rust-lang.github.io/api-guidelines/naming.html`
   - Why consulted:
     - check whether the public helper names and public decision fields are accurately conveying semantics.
   - Constraint informed:
     - names and docs matter more here than representation churn, because the live behavior is policy-heavy and slightly broader than the exported helper names suggest.

6. `OnceLock` documentation
   - Source:
     - `https://doc.rust-lang.org/std/sync/struct.OnceLock.html`
   - Why consulted:
     - re-check whether the earlier packet’s caching ideas were still worth prioritizing.
   - Constraint informed:
     - the current lazy-init pattern is already appropriate; additional caching is not the leading hardening need without profiling evidence.

## What Fourth Pass Sharpened

The current third-pass packet got two important things right.

First, the bucket should remain `Small surgical refactor`. This file is still small enough that broad redesign would create more compatibility risk than value. Second, invariant tests and source-local documentation are still more valuable than “cleanup” refactors. The file’s deterministic `BTreeSet` behavior in partition planning (`crates/app_core/src/ma_policy.rs:261-285`) is an example of something worth preserving, not “simplifying” casually.

What the current packet missed is **which** invariants matter most.

### 1. The third-pass serde/default framing is weakly supported by the live source

The strongest challenge to the current third-pass packet is that its main rationale is not well matched to this file. The packet repeatedly talks about serde omission/default behavior and DTO-visible semantics (`review__third_pass.md:24-36`, `40-67`), but `ma_policy.rs` has no serde derives, no serialization attributes, and no direct IPC or persistence code in the live source (`crates/app_core/src/ma_policy.rs:1-327`). The public risk here is not wire-shape drift. It is route-policy drift and policy-bit misinterpretation.

That does not make the third-pass bucket wrong. It makes the third-pass *reasoning* too generic.

### 2. The biggest live semantic ambiguity is not “partial CSV fallback”; it is that the preference bit is broader than kernel-route eligibility

This is the most important missed point.

`prefers_gpu_kernel_ma` is named like a kernel-route truth function (`crates/app_core/src/ma_policy.rs:147-157`), and `prefers_gpu_by_speedup_table` is just an alias to it (`160-162`). But the file’s own test suite asserts that `"ehma"` is GPU-preferred by the speedup table (`419-423`) even though `"ehma"` is **not** in `GPU_KERNEL_SUPPORTED_MAS` (`5-39`).

That means the exported “GPU preferred” bit is broader than “kernel-capable and preferred for the direct kernel route.” Inside `plan_hybrid_indicator_route`, that broader bit is harmless because route selection still gates on `HybridCapabilityTier::GpuKernel` before selecting `HybridRouteTier::GpuKernel` (`293-305`). But outside that function, the name is easy to over-read.

This matters more than the current third-pass packet acknowledged because it creates a concrete downstream risk: any consumer that treats `prefers_gpu_kernel_ma` or `gpu_preferred_by_speedup` as a kernel-route claim can present misleading capability metadata. The safe-now response is not to rename the public helper immediately. It is to document the broader semantics and, if consumer audit justifies it, add a narrower helper for “kernel-route preferred” rather than changing the existing helper’s behavior silently.

### 3. Input normalization is inconsistent today, and the inconsistency is behaviorally observable

This was not called out in the earlier packets and is a cleaner safe-now fix than most of the broader recommendations.

`supports_gpu_kernel_ma` performs a case-insensitive lookup but does **not** trim input (`142-143`). By contrast, `prefers_gpu_kernel_ma` trims and lowercases first (`148-150`) before consulting the parsed speedup table (`152-155`). That means whitespace handling is inconsistent across the public helpers.

There is also a second inconsistency inside `prefers_gpu_kernel_ma` itself. When the parsed CSV set is non-empty, the function checks the normalized `id` (`152-155`). When the parsed set is empty, it falls back to `contains_case_insensitive(GPU_KERNEL_PREFERRED_MAS, ma_type)` using the original untrimmed input (`156`). So whitespace normalization changes depending on whether the embedded CSV was parsed as empty.

That is a real contract smell, not a theoretical style issue. The smallest justified fix is to normalize once and use the normalized value on both the parsed-table path and the static-fallback path, then add a unit test that the support/preference helpers treat surrounding whitespace consistently.

### 4. The parser risk is more specific than the first-pass packet described, and that makes the safer fix narrower too

The earlier packet was directionally right that a non-empty parsed set becomes authoritative (`crates/app_core/src/ma_policy.rs:152-156`), but its recommended response was too broad. “Validate the whole parsed table and fall back if it looks partial” risks hiding a broken benchmark artifact in production.

The live source shows a narrower and better-targeted problem: header detection is fragile because it is based on the raw first line index and a string prefix check (`111-118`), not on the first parsed field.

Concretely:

- a UTF-8 BOM on the embedded file can defeat `starts_with("indicator_id,")`, because `include_str!` does not strip BOMs;
- leading blank lines make `line_idx == 0` miss the actual header row;
- a quoted header field such as `"indicator_id"` is legal CSV but will not satisfy the raw prefix check.

Those are easier to fix safely than the prior packet’s more open-ended “partial malformed CSV” fallback. The right small change is to track the first **non-empty** row, parse that row with the existing field parser, normalize the first field (including BOM stripping), and then skip it when it equals `indicator_id`. That hardens the current design without adding a silent broad fallback that could mask real artifact drift.

### 5. The missing invariant tests are narrower and more concrete than “serde/default round-trips”

The third-pass packet was correct that invariant tests are valuable, but the wrong invariants were emphasized.

The highest-value missing tests are:

- `GPU_KERNEL_PREFERRED_MAS` stays a subset of `GPU_KERNEL_SUPPORTED_MAS` (`43-76` versus `5-39`);
- every statically listed kernel-supported MA resolves to a live registry entry before route classification relies on it (`239-258`);
- a non-kernel MA can be speedup-preferred without producing `HybridRouteTier::GpuKernel`, so consumer semantics stay explicit;
- support/preference helpers normalize surrounding whitespace consistently once the normalization fix lands;
- CSV header parsing skips first-row headers robustly across BOM / quoted-field / leading-blank-line cases.

Those tests directly protect route-policy meaning. They are more valuable here than the current packet’s generic serde/default suggestions.

### 6. Some earlier recommendations are still too weakly supported to move into “safe now”

Two recommendations from the earlier packets should remain deferred.

The first is narrowing public route types and planners to `pub(crate)`. `ma_policy` is publicly exported from `app_core` (`context__01__crates__app_core__src__lib.rs:35`), but this bundle does not include a full external-consumer audit. That makes visibility tightening a semver-sensitive cleanup, not a safe-now change.

The second is prioritizing another `OnceLock` cache for registry classification. `classify_hybrid_capability` does a linear search over the moving-average registry (`crates/app_core/src/ma_policy.rs:239-258`), but this file is not a numeric hot path, and the live packet shows larger semantic issues than lookup cost. It is reasonable tech debt, but it should not lead the change list.

## Implementation Priority

### Safe-now hardening

The current third-pass bucket should stand, but the safe-now work should be narrower and more concrete than the earlier packet proposed.

Start with helper and parser contract hardening:

1. Normalize helper inputs consistently.
   - Trim in `supports_gpu_kernel_ma`.
   - Use the normalized `id` on both branches of `prefers_gpu_kernel_ma`, including the static fallback branch.
   - Add a small test covering surrounding whitespace and mixed case for both support and preference helpers.

2. Harden CSV header detection without changing the policy model.
   - Skip the first non-empty row when its parsed first field normalizes to `indicator_id`.
   - Strip a leading BOM from that first field before the comparison.
   - Keep the existing embedded-file + `OnceLock` model.

3. Document the live semantics that are currently easy to misread.
   - `prefers_gpu_kernel_ma` / `prefers_gpu_by_speedup_table` are preference bits, not direct proof that the route planner will choose `GpuKernel`.
   - `gpu_preferred_by_speedup` can be true for non-kernel-capable IDs.
   - `CpuStaged` is a supported route outcome, not a capability tier.
   - `indicator_id` and partition outputs are normalized to lowercase, and partition ordering is deterministic/deduplicated rather than input-order-preserving.

4. Add policy invariants that lock the intended maintenance posture.
   - preferred list stays a subset of supported list;
   - statically supported IDs exist in the registry;
   - a non-kernel speedup-preferred MA does not become `HybridRouteTier::GpuKernel`;
   - parser header handling works for the concrete edge cases above.

### Larger redesign still deferred

The following changes are still real ideas, but they should remain deferred because they carry more compatibility or policy risk than the live source justifies right now.

1. Renaming or changing the behavior of the public preference helpers.
   - Changing `prefers_gpu_kernel_ma` to return `false` for non-kernel IDs would alter current public behavior.
   - Renaming/removing `prefers_cpu_by_speedup_table` or the route-planner structs is semver-sensitive.

2. Broad runtime fallback for “partial” speedup tables.
   - That can hide a broken embedded benchmark artifact instead of surfacing it.
   - Prefer parser sanitation plus invariant tests over silent production fallback.

3. Public-surface narrowing (`pub(crate)`), helper removal, or route-struct redesign.
   - These need a full repo/external-consumer audit.

4. Unifying the static support lists, static preferred list, and speedup CSV into a single generated policy source.
   - This is plausible longer-term debt reduction, but it is beyond a small surgical hardening pass.

5. Registry caching as a lead change.
   - Low urgency compared with the semantic fixes above.

## Verification If Changed

At minimum, keep the source-local unit lane green:

- `cargo test -p ta_app_core ma_policy::tests:: -- --nocapture`

If the safe-now changes above are implemented, add or tighten targeted tests for:

- whitespace normalization in `supports_gpu_kernel_ma` and `prefers_gpu_kernel_ma`;
- parser/header handling with BOM, quoted header fields, and leading blank lines;
- preferred-list subset and supported-list registry membership invariants;
- non-kernel speedup-preferred route behavior.

If any downstream call sites are changed to use a narrower “kernel-route preferred” semantic, rerun representative metadata and route consumers as well:

- `cargo test -p ta_app_core capability_report_marks_gpu_supported_for_plain_sma_request -- --nocapture`
- `cargo test -p ta_app_core`
- if route-selection behavior is touched under CUDA-featured builds, rerun representative Double-MA optimization/validation route tests under `--features cuda,cuda-backtest-kernel`.

The important verification boundary here is not GPU ABI or persistence. It is route-choice and metadata consistency across optimization, validation, and capability-report surfaces.

## Regression And Feature-Preservation Constraints

Do not silently change current route selection for known kernel-capable IDs while “cleaning up” helper names. The route planner already gates `GpuKernel` selection on `HybridCapabilityTier::GpuKernel` (`crates/app_core/src/ma_policy.rs:293-305`), and that behavior should remain the source of truth.

Do not convert the current broader preference bit into a narrower one without auditing consumers first. The live file already locks in that non-kernel IDs can be speedup-preferred (`419-423`). If that behavior is undesirable, treat it as a compatibility change, not a trivial rename.

Do not add a wide runtime fallback that treats any surprising non-empty CSV parse as a reason to ignore the embedded speedup table. That would reduce observability and could hide real benchmark-artifact drift. Prefer narrow parser hardening and invariant tests.

Do not remove deterministic lowercase/deduplicated partition behavior (`261-285`) or change the diagnostic shape of the public structs casually. Those are observable semantics even if they are not serde-serialized today.

Do not overstate IPC, persistence, or GPU-ABI risk here. This file does not define the kernel ABI or a persisted wire format. The real blast radius is behavioral: optimization/validation route choice and any user-visible capability/preference metadata derived from these helpers.

## Fourth-Pass Conclusion

`Bucket unchanged`

The third-pass action bucket should **stand**: `crates/app_core/src/ma_policy.rs` is still a `Small surgical refactor`.

What the current third-pass packet got right is the overall posture: keep changes narrow, preserve deterministic behavior, and prefer documentation/tests over broad refactor churn. What it missed or overstated is the center of gravity of the live risk. The real issues are not generic serde/default compatibility concerns. They are a broader-than-advertised preference bit, inconsistent normalization across helpers and fallback branches, and fragile first-row CSV header detection.

The best next pass is therefore smaller and sharper than the earlier packet suggested:

- normalize helper inputs consistently;
- harden first-row header parsing against BOM / blank-line / quoted-header cases;
- document that the speedup-preference bit is advisory and not equivalent to direct kernel-route eligibility;
- add invariant tests that lock policy-source assumptions in place.

API narrowing, helper renames/removals, broad runtime fallback, and policy-source unification remain valid longer-term topics, but they are still deferred. Acting on those now without a consumer audit would introduce more regression risk than this file’s current evidence supports.
