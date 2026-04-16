# Final Consolidated Review: `src-tauri/src/commands_validation.rs`

- Review date: `2026-04-15`
- Source file: [commands_validation.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_validation.rs:1>)
- First-pass packet: [commands_validation.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/commands_validation.rs.review.md:1>)
- Second-pass packet: [commands_validation.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_validation.rs.review.md:1>)
- Third-pass packet: [commands_validation.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands_validation.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 1/045__src-tauri__src__commands_validation.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid with minor scope correction`
- Canonical final action bucket: `Larger refactor but defer`
- External research anchors:
  - <https://v2.tauri.app/develop/calling-rust/>
  - <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
  - <https://rust-lang.github.io/api-guidelines/dependability.html>

## Consolidated Judgment

The first three passes were right to defer a broad redesign. This file is still a Tauri command/orchestration hub that mixes request normalization, validation-context checks, cross-ticker execution, wrapper-side verdict rewriting, and app-core DTO reuse. A large split here without stronger characterization would be regression-prone.

The fourth pass is directionally correct and belongs to the right file. Its most important addition is not broad architecture advice, but a bounded correctness defect: after cross-ticker filtering, the wrapper rebuilds `run_verdict` using survivor counts rather than preserving the original validated candidate count semantics. That is a safe-now hardening target, but it is not enough to move the file out of the defer bucket. No wrong fourth-pass packet is present in this folder.

## Safe-Now Priority

- Preserve original tested-count semantics when rebuilding `run_verdict` after cross-ticker filtering so `candidates_tested` continues to mean candidates originally validated, not survivors after wrapper filtering.
- Add a regression test that freezes current `normalize_strategy_kind()` alias/default behavior before any tightening is attempted, especially for strategy families that are live in the repo but not explicitly mapped here.
- Add characterization tests and inline contract notes for the wrapper’s hybrid post-cross-ticker verdict behavior, where counts are rebuilt but base-run `spa` and `cscv_pbo` are still reused.
- Add characterization coverage for success-only cross-ticker accumulation so failures remain intentionally absent evidence unless and until product policy changes.

## Deferred Work

- Defer the larger DTO split between app-core validation results and Tauri-wrapper command results.
- Defer orchestration unification between the Double-MA and composite paths until the current wrapper semantics are pinned by tests.
- Defer any change from success-only accumulation to failure-penalizing accumulation unless product semantics are revisited explicitly.
- Defer rerunning SPA or CSCV-PBO inside cross-ticker reruns; that would alter semantics and runtime cost materially.

## Verification If Changed

- `cargo test -p ta_desktop_demo_app -- --nocapture`
- Add targeted tests for post-cross-ticker `run_verdict` reconstruction, including preserved tested-count semantics.
- Add regression coverage for current strategy-kind normalization aliases and unsupported-kind fallback behavior.
- Add characterization tests for cross-ticker aggregation when some reruns fail and when no overlap is available.

## Regression And Feature-Preservation Constraints

- Preserve the public Tauri command request and response shape.
- Keep current cross-ticker filtering and rerun semantics unless a test-backed policy change is explicitly intended.
- Do not change `spa` or `cscv_pbo` semantics in the same patch as the tested-count hardening.
- Avoid broad async/execution-model changes while touching only wrapper-level correctness.

## Final Conclusion

The authoritative implementation-facing conclusion is:

- keep `commands_validation.rs` in `Larger refactor but defer`
- make the narrow `run_verdict.candidates_tested` correctness fix now
- add characterization tests for strategy-kind normalization, hybrid verdict semantics, and success-only cross-ticker accumulation
- defer the broader wrapper/app-core split and orchestration unification until those semantics are pinned
