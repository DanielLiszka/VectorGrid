# Final Consolidated Review: `src-tauri/src/ma_registry.rs`

- Review date: `2026-04-16`
- Source file: [ma_registry.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/ma_registry.rs:1>)
- First-pass packet: [ma_registry.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews/src-tauri/src/ma_registry.rs.review.md:1>)
- Second-pass packet: [ma_registry.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-second-pass/src-tauri/src/ma_registry.rs.review.md:1>)
- Third-pass packet: [ma_registry.rs.review.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-third-pass/src-tauri/src/ma_registry.rs.review.md:1>)
- Fourth-pass packet: [review_fourth_pass.md](</C:/Rust Projects/ta_desktop_demo/Repo Review/backend-file-reviews-fourth-pass/Tier 3/013__src-tauri__src__ma_registry.rs/review_fourth_pass.md:1>)
- Fourth-pass validity: `Valid`
- Wrong `review_fourth_pass.md` present: `No`
- Canonical final action bucket: `Small surgical refactor`
- External research anchors:
  - <https://serde.rs/field-attrs.html>
  - <https://docs.rs/crate/vector-ta/latest/source/src/indicators/moving_averages/registry.rs>
  - <https://docs.rs/crate/vector-ta/latest/source/src/indicators/moving_averages/param_schema.rs>

## Consolidated Judgment

The fourth-pass packet is valid and materially sharper than the third-pass packet. [ma_registry.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/ma_registry.rs:1>) is not a command wrapper. It is a projection seam that maps upstream `vector-ta` moving-average registry facts plus app-local kernel policy overlays into the frontend-visible [MaInfo](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/ma_registry.rs:5>) shape.

The real issue is semantic compression:

- [supports_cpu](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/ma_registry.rs:9>) is populated from upstream `supports_cpu_single`;
- [supports_cuda](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/ma_registry.rs:10>) is populated from upstream `supports_cuda_single`;
- upstream also has separate batch/sweep axes that differ for rows like `buff_averages`, `vwap`, `tradjema`, `uma`, and `volume_adjusted_ma`.

So the field names are broader than the facts they currently encode. The other important clarification is that [params](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/ma_registry.rs:16>) is numeric-schema only; additional semantics can still live in `notes`.

## Safe-Now Priority

- Keep the bucket at `Small surgical refactor`.
- Add a short contract note clarifying:
  - `supports_cpu` means upstream single-call CPU support;
  - `supports_cuda` means upstream single-call CUDA support;
  - `supports_cuda_ma_sweep` is the broader sweep axis;
  - `supports_cuda_kernel` / `prefers_cuda_kernel` are app-local overlays;
  - `params` is not a complete machine-readable form contract.
- Add a direct projection test locking representative rows:
  - `buff_averages` or `tradjema`
  - `vwap`
  - `frama`
  - `n_order_ema`
  - `prefers_cuda_kernel => supports_cuda_kernel`

## Deferred Work

- Defer any DTO rename or expansion into explicit `*_single` / `*_batch` axes until the frontend contract moves with it.
- Defer any richer structured parameter-schema work until the upstream `vector-ta` model is ready.
- Do not add local sorting as a “cleanup”; current pass-through ordering is safer.

## Verification If Changed

- Add direct `list_mas()` projection tests in this module or the [commands_data.rs](</C:/Rust Projects/ta_desktop_demo/src-tauri/src/commands_data.rs:81>) command path.
- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- Re-run frontend catalog consumers:
  - `npm --prefix ./web run test -- src/services/movingAverageCatalogService.test.ts src/services/optimizeService.test.ts`

## Regression And Feature-Preservation Constraints

- Preserve field names unless the frontend contract is versioned and migrated.
- Do not add `skip_serializing_if` or other omission behavior casually; wire shape matters here.
- Do not let tests collapse single-call and batch/sweep capability into one meaning.
- Do not treat `params` as exhaustive in new docs or tests.

## Final Conclusion

The fourth-pass packet is valid, no wrong fourth-pass file is present, and the final bucket stays `Small surgical refactor`. The correct implementation is narrow: document the lossy projection honestly and add representative edge-case tests without redesigning the public payload yet.
