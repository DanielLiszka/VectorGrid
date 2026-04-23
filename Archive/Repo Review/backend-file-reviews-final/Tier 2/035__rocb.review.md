# Final Consolidated Review: `crates/app_core/src/rocb.rs`

- Review date: `2026-04-16`
- Source file: [rocb.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/rocb.rs:1>)
- First-pass packet: [rocb.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/crates/app_core/src/rocb.rs.review.md:1>)
- Second-pass packet: [rocb.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/crates/app_core/src/rocb.rs.review.md:1>)
- Third-pass packet: [rocb.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/crates/app_core/src/rocb.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 2/035__crates__app_core__src__rocb.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with strengthening`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://doc.rust-lang.org/std/vec/struct.Vec.html>
  - <https://doc.rust-lang.org/std/sync/struct.OnceLock.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>
  - <https://serde.rs/field-attrs.html>

## Consolidated Judgment

The first three passes were right to keep `rocb.rs` in `Small surgical refactor`, but the fourth pass is the first one that identifies the real next implementation targets. The file still owns several local correctness and contract edges that are narrower than a redesign and stronger than a documentation-only pass.

No wrong `review_fourth_pass.md` is present in this folder. The packet matches the live file and should govern the final review.

## Safe-Now Priority

- Fix the `Vec::reserve` misuse in `build_rocb_desired_positions_into()`.
  - `reserve(additional)` guarantees capacity for `len + additional`, and the current calculation based on `capacity` does not actually ensure room for `n` after `clear()`.
- Harden the malformed-shape early-return path in `build_rocb_desired_positions_into()`.
  - Today the function resizes to `start_t.min(n)` zeros before checking signal-length mismatch, so an early return can leave a truncated output vector instead of a full-length flat vector.
  - The narrow safe fix is either:
    - preserve full output length on mismatch, or
    - add a checked sibling/internal `Result` path where diagnostics matter.
- Tighten the Donchian output-resolution contract.
  - `resolve_output_id()` currently falls back to the first output on miss, which is especially risky for ROCB because Donchian upper and lower are semantically distinct.
  - Keep heuristic matching if needed, but add a strategy-specific guard that upper and lower cannot silently collapse to the same output ID.
- Revisit `first_joint_finite_idx_expected_or_scan()`.
  - The helper currently falls back to `max(first_finite(lhs), first_finite(rhs))`, which is weaker than a true joint finite scan.
  - Either implement the real scan or document/test the current monotonic-warmup assumption explicitly.
- Document that `compute_indicator_f64()` is GPU-preferred but not GPU-guaranteed and may silently fall back to CPU on indicator acquisition.

## Deferred Work

- Do not redesign the caller-owned `*_into` API or move to a full typed indicator-binding layer in this pass.
- Do not broad-brush change ROCB fallback policy from best-effort to strict failure without parity and product review.
- Do not split CPU semantics and CUDA sweep helpers into separate modules in the same patch.
- Do not attempt wider route-reporting or actual-backend-used surfacing from this file alone.

## Verification If Changed

- `cargo test -p ta_app_core rocb -- --nocapture`
- representative ROCB composite optimization and validation paths
- if descriptor binding or CUDA sweep helpers change:
  - ROCB CUDA-enabled coverage
  - related composite CPU/GPU parity or route-selection tests

## Regression And Feature-Preservation Constraints

- Preserve current serde-backed config defaults unless callers and artifacts are audited together.
- Preserve ROCB’s strategy-local organization.
- Treat current best-effort CPU fallback as compatibility-sensitive until redesigned.
- Keep the next pass focused on local correctness and contract truthfulness, not abstraction cleanup.

## Final Conclusion

Keep `rocb.rs` in `Small surgical refactor`, treat the fourth pass as valid, and prioritize the reserve fix, malformed-length handling, descriptor-binding hardening, and explicit route/warmup documentation. Typed binding redesign, route reporting, and module restructuring remain deferred.
