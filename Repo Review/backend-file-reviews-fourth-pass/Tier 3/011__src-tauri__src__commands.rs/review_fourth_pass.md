# Fourth-Pass Review: `src-tauri/src/commands.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/src-tauri/src/commands.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet was directionally conservative, but it still described this file a little too much like a thin IPC seam. The live code shows something narrower in behavior and broader in surface area than that framing suggests.

`src-tauri/src/commands.rs` has almost no file-local runtime logic. What it actually does is:

1. assemble a private child-module tree with `#[path = ...] mod ...;` declarations (`src-tauri/src/commands.rs:1-38`),
2. expose a large flat public Rust facade across command functions, headless helpers, and DTOs (`src-tauri/src/commands.rs:40-169`), and
3. feed a smaller frontend-invokable subset into Tauri only indirectly through `use commands::*;` plus `tauri::generate_handler![...]` in `src-tauri/src/lib.rs:8,82-192`.

That distinction matters because the right fourth-pass recommendation is even narrower than the third pass implied: preserve the structure, document the surface honestly, and avoid inventing cleanup work that mostly churns paths rather than reducing real risk.

## Fourth-Pass Research Log

1. I re-read the live `commands.rs` rather than relying on prior packets. The file is a private module root plus re-export hub, not an implementation file. It currently re-exports a very wide public surface from `src-tauri/src/commands.rs:40-169` while keeping the child modules private via `src-tauri/src/commands.rs:1-38`.

2. I compared that public facade with the actual Tauri registration surface in `src-tauri/src/lib.rs`. The live `generate_handler!` list (`src-tauri/src/lib.rs:82-192`) is the real frontend command registry, while `lib.rs` imports those names via `use commands::*;` (`src-tauri/src/lib.rs:8`). Counting the live source shows `109` frontend-registered command names, while the public facade in `commands.rs` exposes `124` public functions. The delta is `15` public Rust-only helpers, all headless or blocking variants such as `build_report_xlsx_bytes_headless`, research-campaign blocking helpers, options-data blocking helpers, options-expression blocking helpers, and the headless double-MA cross-ticker validation helper (`src-tauri/src/commands.rs:53-60,113-145`).

3. I checked the state-owner claim that the third pass emphasized. That concern is real, but it lives in adjacent files, not here: `src-tauri/src/lib.rs:36-37` separately manages `AppState` and `BackgroundTaskState`, and `src-tauri/src/state.rs:559-575,1413-1453` gives them separate guard and cancellation paths. So the warning against collapsing `cancel_current_run` and `cancel_background_task` is still valid, but it is an adjacent-contract point, not file-local logic.

4. I checked the repo guidance for drift. `REPO_MAP.md` correctly says to treat `src-tauri/src/lib.rs` as the authority for the public command registry, but its hard-coded command count is already stale: it says `77` commands at `REPO_MAP.md:552-554`, while the live `lib.rs` currently names `109`. That is a useful caution against adding any more duplicated numeric command counts to docs or tests.

5. I rechecked the language/framework semantics that matter here. Official Tauri guidance confirms that the actual command registration surface is the single `generate_handler!` call, not a module re-export list. The Rust Reference confirms that `pub use` is a real re-export boundary, and the Rust visibility rules explain why the private child modules remain hidden while their selected items stay publicly reachable through the facade. The rustdoc book also confirms that re-exports from private modules are inlined by default, and that `#[doc(no_inline)]` is the lowest-risk way to make ownership clearer when that matters.

## What Fourth Pass Sharpened

### What the third-pass packet got right

The third-pass packet was correct about the main bucket. This file still belongs in `Small surgical refactor`, not a redesign bucket.

It also got three substantive things right:

- broad structural cleanup is unjustified for a file with no hot-path logic and no file-local bug,
- command names and cancellation semantics are compatibility-sensitive, and
- any refactor that blurs distinct owner paths would be riskier than the current file merits.

That conservative posture should stay.

### What it missed or underweighted

The biggest miss is that the file is **thin in behavior but wide in API surface**. The third pass called it a thin public seam, which is only half true. The live file is behavior-thin, but it is also one of the widest flat Rust facades in the desktop crate:

- `19` private child-module declarations (`src-tauri/src/commands.rs:1-38`),
- `124` public function re-exports,
- `131` public type re-exports,
- `7` crate-visible helper re-exports, and
- only `109` of those public functions are actually part of the Tauri invoke surface.

That widening changes the review emphasis. The main risk is not that this file implements IPC behavior incorrectly. The main risk is that maintainers confuse four different surfaces:

- private module assembly,
- Tauri-invokable commands,
- Rust-only headless/blocking helper functions, and
- convenience type exports.

The third-pass packet also underweighted the compile-time protection that already exists for the actual Tauri surface. Because `lib.rs` imports from `commands::*` and then names commands explicitly inside `generate_handler!`, forgetting to re-export a command that `lib.rs` tries to register is already a compile-time failure. The more realistic drift case is the opposite one: a maintainer adds or renames a public re-export in `commands.rs` and assumes that alone made it frontend-visible. That is a documentation and contract-clarity issue more than a missing runtime test issue.

### What it overstated or weakly supported

The third-pass implementation advice was slightly too broad for this file.

First, “targeted tests around command registration, error mapping, and state-owner boundaries” overstates what `commands.rs` owns. This file does not perform error mapping. It does not implement cancellation logic. It does not own a Tauri handler body. Those belong to the child modules and adjacent state/runtime code. Adding behavior-heavy tests at the `commands.rs` layer would mostly duplicate checks that belong closer to the owning implementation.

Second, “manual desktop smoke for the command path or IPC seam this file owns” is low-yield here. `commands.rs` does not own a concrete UI path. For this file, automated compile/parity checks are more relevant than manual desktop exercise.

Third, earlier suggestions about moving internal sibling imports away from `crate::commands::...` should be treated as lower-value than they first sounded. Because `commands.rs` is also the private module root assembled via `#[path]`, changing those paths to `super::...` or another local spelling mostly reshuffles intra-tree references. It does not materially reduce public API blast radius.

Fourth, a namespace split such as `commands`, `headless`, and `types` is more invasive than a casual cleanup suggestion implies. This file is not just a re-export list; it is also the place where the child-module tree is mounted. A split would reshape module paths, rustdoc output, likely in-repo imports, and probably example/binary/test call sites. That is redesign work, not small hardening.

### Safe-now hardening

The current third-pass bucket should **stand**, but the safe-now work should be narrowed to the following:

**1. Add an honest module-level doc comment.**

A `//!` comment at the top of `src-tauri/src/commands.rs` would buy more than another round of generic tests. It should state, explicitly and in one place, that:

- this file is the private child-module assembly root for the command family,
- the frontend/Tauri invoke surface is only the subset registered in `src-tauri/src/lib.rs` via `generate_handler!`,
- several public functions here are intentionally Rust-only headless/blocking helpers, and
- the type re-exports are a convenience facade, not proof that `commands.rs` is their canonical owner.

That is the highest-value change because it attacks the real confusion directly.

**2. Add one minimal Rust-only facade smoke test, but scope it tightly.**

If the repo wants one extra guard, it should not snapshot the whole export list. It should only lock a small representative subset of intentionally Rust-consumed non-IPC exports, for example:

- `build_report_xlsx_bytes_headless`,
- `run_research_campaign_blocking_headless`,
- `run_options_expression_blocking_headless`,
- `download_polygon_options_dataset_blocking_headless`, and
- one or two representative DTO re-exports that repo-local headless tools actually use.

That catches accidental facade breakage without freezing a very large convenience surface.

**3. Optional doc-only refinement: selective `#[doc(no_inline)]`.**

If rustdoc output matters for this public repository, consider applying `#[doc(no_inline)]` to the large `pub use ta_app_core::{...};` block (`src-tauri/src/commands.rs:155-169`) and possibly the `run_artifacts` type block (`src-tauri/src/commands.rs:147-153`). That is the lowest-risk way to make those types read as re-exports rather than as if `commands.rs` were their natural implementation home.

### Larger redesign still deferred

The larger redesign remains deferred, and the fourth pass is more confident about that than the third pass was.

That deferred bucket includes:

- splitting the flat facade into separate namespaces,
- broad path rewrites inside sibling modules,
- narrowing the public facade aggressively, or
- trying to turn this Tauri app crate into a cleanly versioned app-layer library API.

Those are not impossible improvements. They are just not justified by a file whose current problem is mostly truthful documentation. They should wait for a deliberate app-layer API pass that can update helper binaries, examples, tests, and rustdoc expectations together.

## Implementation Priority

The priority order should be:

1. **Keep the bucket unchanged** and avoid structural churn.
2. **Add module docs first.** This is the highest signal-to-risk ratio.
3. **If another guard is desired, add a tiny Rust-only import/compile smoke** for the intentional non-IPC facade.
4. **Treat `#[doc(no_inline)]` as optional polish**, not required correctness work.
5. **Defer namespace cleanup and sibling-path rewrites** until there is an explicit wider API-surface effort.

If no one is actively using rustdoc for this crate, even the current file may be acceptable as-is after the documentation note. There is no evidence here of a file-local logic defect that demands code movement.

## Verification If Changed

Verification should match the actual ownership of the file.

For a docs-only change in `src-tauri/src/commands.rs`, a normal Rust compile/check is enough. If a small facade smoke test is added, then the best follow-up is:

- `cargo check -p ta_desktop_demo_app`
- the targeted new smoke test, if added
- `npm --prefix ./web run check:commands`
- `cd web && node ./scripts/check-tauri-parity.mjs`

If the headless workbook helper export is touched, `src-tauri/tests/validation_xlsx_report_tests.rs` is the highest-value existing integration surface called out in `REPO_MAP.md:1689-1694`.

What should *not* be the default here is broad manual desktop smoke or a generic “test the whole commands namespace” rule. This file itself is not where behavioral bugs would most likely originate.

## Regression And Feature-Preservation Constraints

- Do **not** remove or rename the public Rust-only headless/blocking helpers casually. The live code clearly exposes them intentionally outside the Tauri registry, and they are the most plausible place for repo-local Rust consumers to bind.

- Do **not** merge or relabel `cancel_current_run` and `cancel_background_task` as a cleanup step. `src-tauri/src/lib.rs:36-37` and `src-tauri/src/state.rs:559-575,1413-1453` show separate managed state and cancellation scopes.

- Do **not** freeze the full flat export list with a snapshot-style test. That would ossify incidental convenience re-exports and create noisy churn for little gain, especially across the `ta_app_core` and `run_artifacts` type blocks.

- Do **not** duplicate command counts in new docs. `REPO_MAP.md:552-554` is already stale relative to the live `lib.rs`, so new prose in this file should describe ownership, not promise a number.

- Performance, persistence, GPU ABI, and validation risk are only **indirect** here. `commands.rs` itself is not a hot path and does not define artifact schema or GPU behavior. The real regression risk is compatibility: renaming re-exported helpers or DTOs can break report, artifact, validation, or options-related Rust callers even though runtime behavior is unchanged.

## Fourth-Pass Conclusion

The current third-pass bucket should **stand**.

`src-tauri/src/commands.rs` is still a `Small surgical refactor`, but the fourth pass narrows that bucket further. The third-pass packet was right to stay conservative, yet it still over-framed this module as an IPC seam and over-suggested test and cleanup work that belongs in owning modules.

The best safe-now hardening is:

- one honest module-level contract comment, and
- at most one tiny Rust-only facade smoke test for the intentional non-IPC exports.

Everything larger than that is still deferred. For this file, a public-facing repository gets more value from accurate surface documentation than from structural churn.
