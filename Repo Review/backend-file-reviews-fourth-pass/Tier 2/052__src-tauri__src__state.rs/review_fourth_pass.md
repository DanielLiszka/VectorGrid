# Fourth-Pass Review: `src-tauri/src/state.rs`

| Field | Value |
| --- | --- |
| Review date | `2026-04-16` |
| Source file | `src-tauri/src/state.rs` |
| First-pass packet | `Repo Review/backend-file-reviews/src-tauri/src/state.rs.review.md` |
| Second-pass packet | `Repo Review/backend-file-reviews-second-pass/src-tauri/src/state.rs.review.md` |
| Third-pass packet | `Repo Review/backend-file-reviews-third-pass/src-tauri/src/state.rs.review.md` |
| Third-pass action bucket | `Small surgical refactor` |
| Fourth-pass action bucket | `Small surgical refactor` |
| Packet status | `Reviewed` |

## Why This File Got A Fourth Pass

This file deserved a fourth pass because the live source still sits on a sensitive contract surface, but the current third-pass packet drifted toward the wrong risk model. The bucket size is still right, yet the justification in the current packet is not. `state.rs` is primarily an in-memory runtime-state coordinator for loaded datasets, derived caches, live-bar mutation, run exclusivity, and live-stream status. It is not a persistence or durable-overwrite module in production code.

That distinction matters. The third-pass packet correctly resisted broad refactor churn, and it correctly warned that lexical path cleanup is not the same as real filesystem identity. But its sharpened emphasis on “overwrite semantics” and “persistence timing” is weakly supported by the live file. In production logic, this module does not own whole-file rewrites. The only `std::fs::write` calls in the file are test helpers (`state.rs:1000`, `state.rs:1026`). The real review target is shared mutable runtime coordination.

Accordingly, the current third-pass bucket should **stand**, but the reason it stands is different: this file wants a small set of targeted correctness hardenings around timestamp/epoch assumptions, overlapping load/resample calls, and a few implicit API contracts. It does **not** need a persistence redesign, a lock-model rewrite, or an abstraction split.

## Fourth-Pass Research Log

This pass rechecked the live source against a small set of external standards and the bundled local context.

First, Tauri’s state-management guidance was re-read to confirm the intended shape of managed shared state (`https://v2.tauri.app/develop/state-management/`). The important constraint is that app-managed state is explicitly shared across commands and typically wrapped with interior mutability such as `Mutex`; a standard mutex remains acceptable unless guards are held across `await` points. That supports keeping the current synchronous lock approach rather than drifting toward an async-mutex redesign.

Second, Rust’s `Arc::make_mut` documentation was revisited (`https://doc.rust-lang.org/std/sync/struct.Arc.html`). It still supports the current clone-on-write mutation model as the correct conservative choice for infrequent live-bar mutation of resident candles. There is no evidence here for replacing that pattern with more invasive shared-mutable designs.

Third, Rust’s atomic-ordering documentation was rechecked (`https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html`). The current `SeqCst` usage on run/background guards remains conservative and correctness-first. There is still no evidence for relaxing those orderings as a cleanup.

Fourth, Rust’s path and canonicalization docs were re-read (`https://doc.rust-lang.org/std/fs/fn.canonicalize.html`, `https://doc.rust-lang.org/std/path/struct.Path.html`). `canonicalize` resolves symlinks, requires filesystem access, and on Windows returns extended-length paths; `normalize_lexically` is not a drop-in substitute because lexical cleanup can change meaning when symlinks are involved. That directly limits how aggressive any “normalize the path key” recommendation can be.

Fifth, Rust’s mutex poisoning documentation was rechecked (`https://doc.rust-lang.org/std/sync/poison/struct.Mutex.html`). It does not create a new top-priority bug here, but it reinforces that partial multi-lock state transitions should be reviewed as runtime-state consistency issues, not as generic error-message polish.

Locally, `src-tauri/src/lib.rs` and `src-tauri/src/commands.rs` were re-opened to confirm that `AppState` and `BackgroundTaskState` are Tauri-managed shared state and that this file sits under a broad command surface, not an optimize-only one. That matters for how conservative IPC-visible changes should be.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The third-pass packet got three important things right.

It was right to keep the action bucket at `Small surgical refactor`. Nothing in the live source justifies a module split, actor rewrite, or broader runtime redesign.

It was also right that path identity should be handled carefully and that lexical normalization should not be oversold as true filesystem identity.

And it was right not to reopen `Arc::make_mut` or `SeqCst` ordering as casual cleanup targets. Those are the kinds of “optimization” suggestions that would create risk without evidence.

### What the current third-pass packet missed or underweighted

The biggest miss is that the real production-risk surface is **runtime coordination**, not persistence. The live file’s most meaningful edge cases are all in timestamp handling, live-bar mutation semantics, and non-transactional multi-step state updates.

The timestamp issue is also broader than the current packet described. The earlier packet correctly called out the pre-1980 load-time normalization bug in `timestamps_look_like_seconds()` (`state.rs:1512-1521`), but it underweighted a second and arguably sharper mismatch: `upsert_live_bar()` does **not** use the same heuristic. It derives units from `timestamp_uses_millis()` (`state.rs:1508-1510`), which treats values as milliseconds only once they reach `1_000_000_000_000`, i.e. roughly **2001-09-09T01:46:40Z**, while load-time normalization uses a **1980-01-01T00:00:00Z** cutoff (`state.rs:1474-1475`, `state.rs:1514-1518`). That means legitimate millisecond datasets from **1980-01-01 through 2001-09-09** can survive load-time normalization correctly but still be misread as second-based during live-bar upsert (`state.rs:474-484`).

There is a second timestamp contract weakness behind that one. `upsert_live_bar()` accepts an absolute Unix-millisecond input (`time_open_unix_ms`) but never verifies that the resident dataset is itself epoch-based. The test suite intentionally loads and normalizes a relative-seconds CSV into relative milliseconds (`state.rs:1030-1055`). That dataset is fine for load/resample tests, but it is not a semantically valid live-upsert target. Even a “fixed” milliseconds/seconds branch would still mix an epoch timestamp with a relative timeline. So the safe contract is narrower than the current code expresses: live upsert is only coherent for epoch-based datasets, and today that rule is implicit.

The file also has a more important atomicity problem than the third-pass packet described. The real issue is not durable file-overwrite atomicity; it is non-transactional state observation under overlapping commands.

`load_price_data()` performs a reuse check, then runs `load_or_build_with_identity()`, then inserts into the state map (`state.rs:215-223`, `state.rs:233-267`). There is no second in-memory reuse check before insertion. Two overlapping calls for the same file can therefore both miss reuse, both do the work, and both create separate base IDs. With the default base-retention limit of `1`, the later insert can immediately evict the earlier dataset. That is not a theory-heavy concern; it is a straight check-then-act race on a Tauri-managed shared state surface.

`resample_loaded_data()` has a related snapshot problem. It captures `base_version` under one lock scope (`state.rs:299-342`), then later clones the base candles (`state.rs:355`) and loaded context (`state.rs:356`), then later inserts a derived entry using the earlier `base_version` (`state.rs:365-376`). A concurrent base mutation can therefore yield a derived dataset computed from newer candles but stamped with an older `source_version`, causing needless future invalidation. A concurrent base invalidation can also let a derived entry be inserted after the parent disappears, leaving an orphaned derived node keyed to a missing parent ID.

A smaller but still real miss is that `upsert_live_bar()` is only safe by convention for base datasets. The method accepts any `data_id` and invalidates only descendants (`state.rs:460`, `state.rs:546-553`). If a derived dataset ID is ever passed, the derived series can be mutated without updating or invalidating its parent base. The file’s own parent/child freshness model strongly suggests that live-bar mutation is intended for base datasets only, but the method does not enforce that.

Finally, the current packet underweighted a small IPC-facing mismatch: `try_begin_run()` returns the optimize-specific error string `"an optimization run is already in progress"` (`state.rs:559-565`) even though this state module is managed under a much broader command surface than optimization alone. That is not a redesign issue, but it is a real user-visible wording bug on a shared runtime guard.

### What the current third-pass packet overstated

The main overstatement is the idea that this file’s next move should be “overwrite semantics or identity-sensitive path handling before anything else.” That ordering is not well supported by the live source. Path handling does matter, but it is not the first thing to fix.

The earlier path-normalization advice is also too strong as a safe-now recommendation. Today, base reuse requires both `ctx.csv_path == path` and a matching `source_identity` (`state.rs:736-738`). Without the definition of `SourceIdentity` in this review bundle, it is not safe to assume that dropping raw-path equality would be correctness-improving. The extra path check may be deliberately compensating for a weaker metadata identity. On top of that, Rust’s own docs warn that `canonicalize()` resolves symlinks and produces extended-length Windows paths, while lexical normalization can change meaning across symlink boundaries. So “just canonicalize the key” is not an evidence-backed small fix yet.

The earlier recommendation to change `upsert_live_bar()` into an explicit outcome enum is also somewhat too large for the current bucket if interpreted literally. That method is part of a shared backend surface, and any signature change can cascade into command modules and IPC assumptions. The safer immediate step is to keep the `Result<(), String>` shape unless and until the command boundary actually needs structured mutation outcomes.

## Implementation Priority

### Safe-now hardening

The safest immediate hardening is to unify the live-bar timestamp contract. That should do **two** things, not one: first, use a seconds-vs-milliseconds decision that is consistent with load-time normalization; second, reject live-bar upserts for resident datasets that are not clearly epoch-based. Without that second guard, relative timestamp series remain invalid live targets even if unit inference is fixed. This change is local, high-value, and backed directly by the live code.

The next safe-now improvement is a narrow concurrency hardening for overlapping loads and resamples. For `load_price_data()`, the practical small fix is a second in-memory reuse check after `load_or_build_with_identity()` returns but before a new base entry is inserted. That does not remove duplicate work entirely, but it does prevent duplicate IDs and immediate self-eviction churn. For `resample_loaded_data()`, the practical small fix is to snapshot candles/version/context more coherently and re-check parent freshness before inserting the derived entry. This can stay inside `state.rs`; it does not require an actor rewrite.

A third safe-now hardening is to make the live-bar mutation contract explicit for dataset kind. If current callers only ever pass base IDs, reject `DatasetKind::Derived` up front. If there is any existing caller ambiguity, document the rule first and add a regression test before tightening behavior.

A fourth safe-now cleanup is the run-guard error text. It should become generic, for example “a run is already in progress,” because that is what the state surface actually represents. This is low risk, but because exact strings cross the IPC boundary, it should still be treated as a compatibility-visible change and tested accordingly.

### Larger redesign still deferred

A generation-scoped cancellation token is still deferred. The current shared `AtomicBool` model assumes that all workers tied to a run are gone before the next run starts. That assumption may be acceptable today, but replacing it with per-run generation tokens or a structured run handle would be a broader cross-module contract change.

A stronger filesystem identity model is also deferred. If the repo eventually wants same-file dedupe across alternate path spellings or symlinks, that should be designed around an explicit identity contract in the cache/loader layer, not improvised inside `state.rs` with `canonicalize()` alone.

Likewise, a structured live-bar outcome type remains deferred unless the command boundary actually needs append/update/ignore distinctions as data, not just diagnostics. That is a good design direction if UI or telemetry asks for it, but it is larger than the narrow hardening justified by the current evidence.

Finally, a full transactional or actor-style state owner is deferred. The overlapping-call issues found here are real, but the smallest justified response is targeted snapshot and second-check hardening, not architectural replacement.

## Verification If Changed

If any of the safe-now recommendations are implemented, the minimum local verification should stay centered on `state.rs` and the command families that consume its runtime contracts.

Run the existing `state_` tests first, but extend them materially. The current suite is good on residency, retention, and basic live mutation, yet it still needs focused regression coverage for: legitimate pre-1980 millisecond timestamps on load; legitimate 1980-to-2001 millisecond timestamps on live upsert; rejection or explicit handling of non-epoch relative timestamp datasets in live upsert; exact-timestamp overwrite versus unmatched stale-bar ignore; and any newly added base-only mutation guard.

Then run the data and live-execution command tests, because timestamp handling and live-bar mutation cross those boundaries directly. If the overlapping-load or overlapping-resample behavior changes, add at least one concurrency-oriented regression test or deterministic double-check test around the new second-check logic.

If the run-guard error text changes, re-run any command or frontend tests that assert exact error text. This is a small change, but it is IPC-visible.

No GPU ABI, report-shape, or workbook-format verification expansion is required by the recommendations in this packet. The risk surface here is runtime-state correctness and command behavior, not CUDA contract shape.

## Regression And Feature-Preservation Constraints

Preserve the current `LoadPriceDataResponse` schema and dataset-ID shape unless command and frontend consumers are changed together. The safe-now recommendations do not require a response-schema change.

Preserve clone-on-write mutation via `Arc::make_mut`. The current live-bar path is semantically conservative in the right way; the issue is not the clone-on-write mechanism, but the timestamp and dataset-kind assumptions around it.

Do not weaken `SeqCst` ordering on run/background guards without a deliberate concurrency proof. This file is too central a runtime boundary for speculative atomic tuning.

Do not collapse path identity to `canonicalize()` alone as a small refactor. On Windows that changes path representation materially, and in all environments it resolves symlinks and requires filesystem-backed normalization. Any such change needs an explicit compatibility review with the cache identity contract.

If live upsert is restricted to base datasets or epoch-based timelines, preserve current append-at-tail and exact-timestamp overwrite behavior for the supported cases. Those are the mutation modes the file’s existing test posture already implies.

If concurrent load/resample hardening is added, preserve today’s retention and descendant-invalidation semantics. The goal is to remove duplicate/orphan/stale-snapshot behavior, not to change eviction policy or derived-cache freshness rules.

## Fourth-Pass Conclusion

The third-pass action bucket should **stand** as `Small surgical refactor`, but the current third-pass packet is not sharp enough on where the real work is.

What it got right: no broad redesign, no casual atomic-ordering cleanup, and no simplistic path-normalization story.

What it missed: the file’s most important remaining risks are the live-upsert timestamp/epoch contract and the non-transactional load/resample flows under overlapping calls. Those are better-supported, more file-specific findings than the packet’s current emphasis on overwrite semantics.

What it overstated: path normalization and persistence-style concerns are not the first safe-now move here, and changing `upsert_live_bar()` into a structured public outcome API is larger than this bucket needs.

The practical conclusion is narrow and conservative. Keep the bucket. Tighten the live timestamp contract. Add a second-check/snapshot hardening pass for overlapping loads and resamples. Make the run-guard wording accurate. Defer larger cancellation, identity, and API-shape redesign until the broader command surface is reviewed together.
