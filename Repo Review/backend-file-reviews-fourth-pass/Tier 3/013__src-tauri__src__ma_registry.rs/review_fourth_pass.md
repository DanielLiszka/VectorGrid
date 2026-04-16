# Fourth-Pass Review: `src-tauri/src/ma_registry.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/ma_registry.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file warranted one more pass because the current third-pass packet drifted into risks that are real at the app level but not actually owned by this module. The live source is only a DTO/projection seam: `MaInfo` defines the serialized row shape, and `list_mas()` maps upstream moving-average registry facts plus app-local kernel-policy overlays into that row (`src-tauri/src/ma_registry.rs:5-39`). It does not define a Tauri command, does not touch runtime state, and does not participate in cancellation or event flow. The command exposure lives elsewhere: `commands.rs` re-exports `list_moving_averages` from `commands_data.rs`, and `lib.rs` registers that command in the Tauri invocation surface (`context__02__src-tauri__src__commands.rs:61-64`; `context__01__src-tauri__src__lib.rs:185-188`).

So the right fourth-pass question was narrower than the third-pass framing: not whether a thin IPC seam needs generic command hardening, but whether this projection is semantically faithful enough for the frontend capability contract. The answer is that the current bucket should stand, but the real unresolved issue is sharper than the third-pass packet said: this adapter silently collapses some upstream capability distinctions that already matter in the source data.

## Fourth-Pass Research Log

I re-read the live file first, then the third-pass packet, then the nearby command-registration context to confirm ownership boundaries. After that, I checked the upstream sources that feed this adapter and the serialization rules that make its wire shape compatibility-sensitive.

The external references that informed this pass were:

1. [Tauri IPC / commands](https://v2.tauri.app/concept/inter-process-communication/): Tauri commands are delivered over message-passing IPC, and command arguments/return values must be JSON-serializable. That keeps this DTO in the category of real frontend contract, not internal glue.
2. [Serde field attributes](https://serde.rs/field-attrs.html): field names, omission rules, and nullability are wire-shape decisions. That matters here because `MaInfo` derives `Serialize` directly and currently does not use `rename` or `skip_serializing_if`.
3. [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html): useful mostly as a restraint on naming and documentation claims for a public data shape.
4. [`vector-ta` moving-average registry source](https://docs.rs/crate/vector-ta/latest/source/src/indicators/moving_averages/registry.rs): this is the upstream source for the capability facts being projected here.
5. [`vector-ta` moving-average param schema source](https://docs.rs/crate/vector-ta/latest/source/src/indicators/moving_averages/param_schema.rs): this is the upstream source for the `params` payload copied into `MaInfo`.

That upstream check materially changed the review. It showed representative rows that the earlier packets did not pin down: `buff_averages`, `vwap`, `tradjema`, `uma`, and `volume_adjusted_ma` all separate single-call support from batch/sweep support in ways the current DTO only partially preserves. It also showed that the parameter schema is intentionally numeric-only while still pushing some meaningful configuration into free-form notes.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet was right to keep this file out of any broad rewrite bucket. The file is still small, low-frequency, and not the place to chase performance. It was also right that the serialized shape is contract-sensitive because it is exposed to the frontend through a Tauri command path.

That core bucket call still holds. The safe work here remains narrow: documentation, a better projection test, and clearer constraints on what not to “clean up.”

### What it missed or underweighted

The main miss is that the third-pass packet described risks from the wrong layer. It talked about preserving command identities, state-owner boundaries, cancellation semantics, and event behavior. Those are important in this repository, but they are not file-local concerns here. `ma_registry.rs` does not own them. Treating this file as though it were a command wrapper obscures the more specific contract issue that does live here.

That contract issue is the way this adapter names and projects support flags. At `src-tauri/src/ma_registry.rs:27-31`, `supports_cpu` is projected from `m.supports_cpu_single`, `supports_cuda` from `m.supports_cuda_single`, and `supports_cuda_ma_sweep` from `m.supports_cuda_sweep`. The problem is not that the code is inconsistent with upstream. The problem is that the field names are broader than the upstream facts they encode.

Upstream `vector-ta` does not model a single broad “CPU support” bit or a single broad “CUDA support” bit. It models at least four separate axes in the registry row: `supports_cpu_single`, `supports_cpu_batch`, `supports_cuda_single`, and `supports_cuda_sweep`. The fourth pass found concrete cases where those axes differ. `buff_averages` is batch/sweep-capable but not single-call-capable. `tradjema`, `uma`, and `volume_adjusted_ma` are available through CPU batch and CUDA sweep even though their single-call CPU/CUDA flags are false. `vwap` keeps CPU single support, drops CUDA single support, but still keeps CUDA sweep support. That means the current DTO is semantically lossy: `supports_cpu = false` does **not** mean “no CPU path,” and `supports_cuda = false` does **not** mean “no CUDA path.” It only means “no single-call path,” which is a much narrower statement.

The first-pass packet broadly said the flag meanings needed clearer documentation. That was directionally right, but it did not pin the concrete edge cases that make the ambiguity real. The third-pass packet missed them entirely.

A second underweighted point is `params`. The earlier packets treated `params` mostly as a projected schema needing tests. The fourth pass found that this is only partially true. `params` comes from `ma_param_schema(m.id)` (`src-tauri/src/ma_registry.rs:35-36`), and upstream `MaParamInfo` only models numeric parameter kinds (`Float` and `Int`). Upstream already uses notes to carry non-numeric semantics. `n_order_ema` is the cleanest example: the schema exposes numeric `order`, but the parameter note explicitly says there are additional enum parameters. Separately, registry notes for dual-output rows such as `buff_averages` and other batch-oriented rows carry output-selection semantics in prose. So `params` is not a full machine-readable form contract. It is a numeric parameter schema plus notes. Any review packet that implies otherwise is overstating what this adapter currently guarantees.

A third correction is on ordering. The first pass suggested the file should “decide whether ordering should be documented or explicitly sorted.” After checking the upstream registry, the safer conclusion is narrower: pass-through ordering is fine, and documenting that fact is much safer than introducing local sorting. This adapter currently preserves upstream literal-slice order. Sorting here would be a user-visible behavior change, not neutral cleanup.

### Safe-now hardening

The current bucket should stand, and the safe-now work remains small.

First, add a short contract note on `MaInfo` or at the module level that clarifies the support axes precisely:

- `supports_cpu` currently means upstream **single-call CPU** support, not broad CPU availability.
- `supports_cuda` currently means upstream **single-call CUDA** support, not broad CUDA availability.
- `supports_cuda_ma_sweep` is the broader CUDA sweep/batch axis.
- `supports_cuda_kernel` and `prefers_cuda_kernel` are app-local kernel-policy overlays, not fields originating from the upstream registry.
- `params` is a numeric schema surface; additional enum/output semantics may still live in `notes`.

That documentation is the smallest change that makes the current lossy DTO honest without breaking the wire contract.

Second, add a focused projection test at this layer. The right test is not “the catalog is non-empty” and not a broad manual smoke. It should pin a small set of representative rows that would catch semantic drift if upstream metadata or app policy changes. The best candidates are:

- `tradjema` or `buff_averages`, to lock the distinction between single-call support and batch/sweep support;
- `vwap`, to lock the `supports_cuda = false` but `supports_cuda_ma_sweep = true` case;
- `frama`, to lock a candles-required row with a meaningful note;
- `n_order_ema`, to lock that notes are preserved on parameter rows and to avoid future consumers assuming `params` is complete;
- and the invariant `prefers_cuda_kernel => supports_cuda_kernel`, which is a real projection-level sanity check for the app-local kernel overlay.

Third, tighten any nearby documentation or consumer wording that currently paraphrases `supports_cpu` / `supports_cuda` as broad capability. That can be fixed conservatively now without changing the DTO.

### Larger redesign still deferred

There is a real redesign worth doing later, but it is not a small file-local edit.

If the repository wants this catalog to be a precise capability matrix, then the current DTO shape is underspecified. A future coordinated contract revision should expose explicit axes such as `supports_cpu_single`, `supports_cpu_batch`, `supports_cuda_single`, and `supports_cuda_sweep`, or otherwise version/rename the current fields so the wire names stop implying broader meaning than the source data supports. That is a cross-layer change touching the Tauri response shape, frontend normalization, UI copy, and probably tests that assume today’s payload.

Likewise, if the repository wants the MA catalog to drive a fully structured dynamic builder, the parameter surface needs more than `Float`/`Int` plus note strings. That almost certainly starts upstream in `vector-ta` by making non-numeric choices and output-selection semantics machine-readable, then projecting that richer schema here. Doing that inside `ma_registry.rs` alone would be the wrong level and likely create partial, regression-prone shadow semantics.

### Claims from earlier packets that are too weak or too strong

The strongest overstatement in the third-pass packet is the claim that the main risk here is “state ownership and cancellation scope.” That is not supported by the live file. Those concerns are real in the application, but they are not owned here and should not dominate this packet.

The weakest-supported verification guidance in the third-pass packet is `cargo test -p ta_desktop_demo_app ma_registry -- --nocapture` plus a manual command/IPC smoke. This file has no meaningful runtime/stateful behavior of its own. The more trustworthy verification is a direct projection test plus the existing command/frontend service tests that consume the catalog.

The first-pass packet’s suggestion that the adapter might want explicit sorting is also too aggressive for a thin projection seam. Documentation of pass-through order is safe; introducing a sort is not.

## Implementation Priority

Priority one is to keep the current bucket and do only the safe-now hardening: document the actual meaning of the existing fields and add a projection-specific test that locks the representative edge cases the upstream registry already contains.

Priority two is to avoid “cleanup” that quietly changes the public payload. In this file, field renames, nullability changes, and local sorting are more dangerous than they look because they alter a frontend-facing command result.

Priority three is to defer the real model cleanup until the repository is willing to make a coordinated frontend/backend catalog-contract change. That larger redesign is valid, but it is not this pass’s safe recommendation.

## Verification If Changed

If this file changes only in comments or local projection tests, the strongest verification set is still narrow and contract-focused:

1. `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
2. A new direct test for `list_mas()` or the `list_moving_averages` command path that asserts representative rows preserve the intended distinctions (`tradjema`/`buff_averages`, `vwap`, `frama`, `n_order_ema`)
3. `npm --prefix ./web run test -- src/services/movingAverageCatalogService.test.ts src/services/optimizeService.test.ts`

If the serialized field set, field names, or ordering changes, the verification bar rises. In that case I would also run the broader frontend tests that consume the catalog and do a targeted UI smoke on the moving-average picker/catalog surface. But that should be treated as coordinated contract work, not the default path for this pass.

## Regression And Feature-Preservation Constraints

Preserve the current field names unless the frontend contract moves with them. Tauri command payloads are serialized IPC output, so field-name changes are not benign refactors.

Do not change `notes` omission/null behavior casually. Adding `skip_serializing_if = "Option::is_none"` would change the wire shape.

Do not add local sorting in this adapter unless the product explicitly wants a new UI order. Today’s behavior is a straightforward pass-through of upstream registry order.

Do not write tests that incorrectly collapse capability axes. In particular, do **not** assert that `supports_cpu = false` means “CPU unsupported” or that `supports_cuda = false` means “no CUDA route.” Upstream registry data already disproves those stronger readings.

Do not treat `params` as a complete form-building contract in new tests or docs. The current schema is numeric-only, and some additional semantics already live in notes.

There is no direct persistence, GPU ABI, or validation-pipeline risk in this file. The indirect risk is capability-matrix drift: if this adapter misstates support semantics, the frontend can present wrong availability, wrong kernel preference messaging, or wrong legality hints. That is a contract risk, not a hot-path or ABI risk.

Also avoid pointless allocation micro-optimizations here. Replacing owned `String` / `Vec` fields with borrowed or lifetime-heavy variants would complicate the boundary for negligible gain on a low-frequency catalog call.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**: `src-tauri/src/ma_registry.rs` is still a `Small surgical refactor`, not a local redesign candidate.

But the third-pass packet did not identify the most important file-local weakness. The real issue is not generic IPC/state/cancellation drift. It is that this adapter projects upstream single-call support into broader-looking field names and therefore loses some capability precision that already exists upstream. It also exposes a parameter schema that is useful but not fully structured.

That does **not** justify changing the bucket today. It does justify a sharper recommendation set:

- safe now: document the true semantics of the existing fields and add a projection test that locks representative edge cases;
- still deferred: any contract redesign that introduces explicit single-vs-batch axes or richer structured parameter metadata.

That is the conservative, evidence-backed fourth-pass call: keep the bucket, narrow the immediate work, and make sure future reviewers do not mistake this adapter for either a generic command wrapper or a complete capability model.
