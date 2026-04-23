# Fourth-Pass Review: src-tauri/src/commands_analysis.rs

- Review date: `2026-04-15`
- Source file: `src-tauri/src/commands_analysis.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Accept as-is`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

`commands_analysis.rs` deserved a fourth pass because the third-pass packet was directionally careful, but it still mixed together three different questions that should be separated more sharply for a public Tauri boundary file:

1. what the live file objectively does today,
2. what the repo’s current runtime/task model says `cancel_current_run` is for, and
3. what Tokio’s `spawn_blocking` guidance says in the abstract versus what is evidenced here.

The live source is still only four thin command wrappers. Each command resolves `req.data_id` through `AppState::get_candles`, captures an `Arc<Candles>` snapshot, and then offloads the actual analysis call through `tauri::async_runtime::spawn_blocking(...)` before returning the app-core result unchanged (`src-tauri/src/commands_analysis.rs`, lines 8-57). That narrow shape still matters.

The third pass got two important things right.

First, it correctly preserved the must-not-break invariant: snapshot acquisition happens before the blocking handoff. That matters because `AppState::get_candles` clones the resident candle `Arc` (`src-tauri/src/state.rs`, lines 408-420). In this repo, residency/eviction logic also explicitly treats `Arc::strong_count(&entry.candles) > 1` as a reason to avoid evicting that dataset first (`src-tauri/src/state.rs`, lines 886-909). So the current wrapper does more than just “read candles”; it also pins in-flight data in a way the residency policy already understands.

Second, the third pass was right not to recommend routing this file through unrelated optimization or artifact-persistence machinery. Repo guidance is explicit that `AppState` owns “the single-run guard used by optimization/validation flows,” and that `cancel_current_run` is a run-level control distinct from `cancel_background_task` (`REPO_MAP.md`, Tauri runtime state section). That makes “just wire analysis into the shared run/cancel path” materially riskier than the third-pass packet acknowledged.

That is why the bucket should change. The best adversarial reading is no longer “small surgical refactor in this file.” It is “accept this file as-is, and only change adjacent task/UI semantics if a cross-layer revalidation confirms the affordance mismatch still exists.”

## Fourth-Pass Research Log

I re-checked the live source and local runtime context first, then re-checked the official runtime docs that actually govern the third-pass claims.

On the repo side:

- `src-tauri/src/commands_analysis.rs` still contains only four async commands, each following the same pattern: `get_candles` first, `spawn_blocking` second, return app-core result unchanged.
- `src-tauri/src/state.rs` still shows that `get_candles` clones an `Arc<Candles>` and updates access sequencing, while `try_begin_run` / `cancel_token` / `request_cancel` remain part of the run-level state surface.
- `REPO_MAP.md` still documents that the single-run guard is for optimization/validation flows and that `cancel_current_run` is a run-level control rather than a generic long-task bus.

On the official docs side:

- Tauri’s current “Calling Rust from the Frontend” guide still says async commands are preferred for heavy work so the UI does not freeze, and that non-async commands otherwise run on the main thread.
- Tauri’s current state-management guide still treats `State` as shared managed state and keeps mutation/locking concerns separate from the command boundary itself.
- Tokio’s current `spawn_blocking` docs still say the closure runs on a dedicated blocking pool, that the pool grows until the configured blocking-thread limit is hit, that the default upper limit is intentionally large, that a semaphore or similar limit is only recommended when running many CPU-bound computations, and that started `spawn_blocking` tasks cannot be aborted.
- Tokio’s runtime builder docs still show the default `max_blocking_threads` value as `512`, with queued blocking tasks remaining in an unbounded queue once the limit is reached.

Those official docs support two conclusions at once:

- keeping `spawn_blocking` here is still the correct posture for heavy synchronous work, and
- using those docs alone is not enough to prove that this specific file has a present-tense concurrency defect that justifies a local semaphore or a forced coupling to the optimization/validation run guard.

## What Fourth Pass Sharpened

### 1. The third-pass concurrency finding is real in theory, but overstated as a file-local safe-now defect

The third pass leaned on the correct Tokio warning: CPU-bound `spawn_blocking` work can oversubscribe threads if you submit many jobs concurrently. But the packet jumped too quickly from “Tokio warns about this in general” to “this file should locally bound analysis concurrency now.” That is not fully supported by the evidence currently in the bundle.

Why the claim is too strong:

- this file has no internal fan-out loop, queue, batching, retry loop, or recursive scheduling;
- the bundle does not include profiler evidence, saturation traces, or user-facing starvation reports for analysis follow-up work;
- Tokio already queues blocking work after the runtime’s blocking-thread limit is reached, and the documented default limit is very large;
- the same Tokio docs recommend a semaphore when running **many** CPU-bound computations, which is a workload claim, not an automatic property of every `spawn_blocking` use.

So the third pass identified a plausible systems concern, but not one that is strong enough to drive a source-file bucket by itself. Treat it as deferred design pressure, not current proof of a `commands_analysis.rs` defect.

### 2. The third-pass packet underweighted that the obvious backend-side “cancel fix” is repo-semantic drift

The live repo guidance matters here. `REPO_MAP.md` is explicit that the single-run guard belongs to optimization/validation flows, and the `AppState` implementation still names the failure mode accordingly: `try_begin_run()` returns `"an optimization run is already in progress"` (`src-tauri/src/state.rs`, lines 559-567). That is a strong signal that the run guard is not a neutral generic task primitive.

So while the second and third passes were reasonable to worry about a fake-cancel affordance if the frontend still exposes one, that mismatch is also not fully re-proven by the live bundle here because the cited `taskStore` / frontend invoke code is not part of the local packet. That makes the proposed backend-side repair path even looser than the third-pass framing implied. Wiring these read-only analysis commands into `try_begin_run()` or the shared run cancel token would:

- couple analysis follow-ups to optimization/validation exclusivity,
- potentially serialize benign read-only analysis behind active runs, or vice versa,
- blur the semantics of a run-level cancel path that the repo currently documents as belonging elsewhere,
- and still not produce true abortability for started `spawn_blocking` work.

That is regression-prone, not “small and safe.”

### 3. The third pass missed the more concrete runtime invariant: current snapshot timing already interacts with dataset residency

The current wrapper pattern is not only about stale-snapshot safety. It also lines up with the state layer’s retention policy.

Because `get_candles` clones an `Arc` before handoff, in-flight analyses temporarily increase `Arc::strong_count`, and the eviction policy explicitly prefers evicting unshared entries before shared ones (`src-tauri/src/state.rs`, lines 886-909). That makes the current order of operations materially meaningful.

This matters because several “simple” fixes would perturb more than cancellation semantics:

- moving the snapshot later changes retention behavior,
- holding an analysis semaphore **before** `get_candles` changes hotness/access timing,
- holding a semaphore **after** `get_candles` may pin `Arc`s longer while jobs wait,
- and routing analysis through a different task abstraction may accidentally narrow the current guarantee that an in-flight snapshot survives invalidation or live updates.

The third-pass packet preserved the snapshot idea, but it did not fully spell out how much repo behavior currently hangs off that detail.

### 4. The safest local hardening was already identified earlier and remains underweighted: thin wrapper tests, not task-system surgery

The first pass was right that this file’s main local debt is missing direct wrapper coverage. The third pass largely moved past that and focused on runtime policy, but the more defensible backend-local improvement is still narrow regression coverage if the file is ever touched.

That is especially true because this file is a public IPC surface but only 50-ish lines of wrapper code. A tiny test harness around request forwarding, snapshot-before-handoff behavior, and basic error propagation is a far safer hardening move than adding a semaphore or reusing run-level cancellation infrastructure.

### 5. The third-pass packet also underweighted a smaller but real observability gap

Each command still flattens `JoinError` via `.map_err(|e| e.to_string())?`. That is acceptable for repo consistency, but it loses command-specific context. This is not a bucket-driving bug, yet it is a more realistic “touch-it-if-you-are-already-here” cleanup than concurrency gating.

Even here, caution is required: changing string text can break frontend tests or user-visible expectations if any layer compares exact error messages. So this is optional hardening, not a must-fix.

## Implementation Priority

### Safe-now hardening

For this file itself, the current third-pass bucket should **not** stand. The safer immediate posture is to leave `commands_analysis.rs` behavior unchanged.

If a real user-facing issue still exists, the smallest honest fix is probably outside this file:

- revalidate whether the frontend still marks these analysis commands as cancelable work;
- if it does, remove only the cancel affordance or mark the work as tracked-but-not-cancelable;
- keep the backend wrapper as a thin snapshot-plus-dispatch layer.

If this source file is touched anyway, the highest-confidence hardening is:

- add thin command-boundary tests close to the Tauri layer;
- preserve the current `get_candles`-before-`spawn_blocking` ordering;
- optionally improve join-error context only if exact error strings are not contract-sensitive.

### Larger redesign still deferred

If the product truly needs cancelable analysis follow-ups, that is no longer a `commands_analysis.rs` micro-refactor. It is a wider task-system change that should be deferred until scoped explicitly.

That redesign would need all of the following, not just a wrapper tweak:

- an analysis-specific task model rather than reuse of the optimization/validation run guard,
- a cooperative cancel contract inside the app-core analysis functions themselves,
- or a dedicated executor model for CPU-bound analytics if empirical profiling shows real contention.

Likewise, if analysis concurrency eventually proves to be a desktop-capacity problem, the fix should be evidence-driven. A task-system or dedicated-executor design can be justified later; a local semaphore in this file is too easy to add and too easy to get subtly wrong.

## Verification If Changed

If only frontend task semantics are adjusted, the priority checks should stay frontend-heavy:

- `npm --prefix web run test -- src/services/analysisService.test.ts src/stores/taskStore.test.ts src/stores/taskStore.lifecycle.test.ts`
- `npm --prefix web run lint`

If this Rust file changes at all, run at minimum:

- `cargo test --workspace`

For higher-risk cross-layer work, use the repo’s broader non-options gate:

- `powershell -File scripts/run_repo_quality_gate.ps1`

If anyone does attempt concurrency gating or cancellation redesign later, that work should also include targeted runtime evidence rather than only static review. At that point, bench/profiling or workflow-gate evidence becomes relevant, because the change would be task-system behavior, not just wrapper correctness.

## Regression And Feature-Preservation Constraints

Any future change should preserve all of the following unless there is an explicitly approved cross-layer migration:

- the four public command names and current request/response DTO shapes;
- off-main-thread execution for the synchronous analysis work;
- pre-handoff candle snapshotting through `AppState::get_candles`;
- the current ability for in-flight analysis to remain insulated from later dataset invalidation or live-bar mutation;
- the current residency interaction where shared `Arc` ownership helps avoid evicting in-use data.

The main regression risks if the third-pass recommendations were implemented too aggressively are:

- **Compatibility / IPC risk:** renaming commands, changing DTOs, or even casually changing error strings can ripple into frontend invoke code and tests.
- **Runtime / task-model risk:** reusing `try_begin_run()` would improperly couple analysis follow-ups to optimization/validation exclusivity.
- **Performance risk:** a local semaphore may add latency or distort legitimate read-only follow-up interactivity without proof that contention is currently harming users.
- **State / residency risk:** changing when `get_candles` is called can alter access-sequence and eviction behavior.
- **Cancellation risk:** `spawn_blocking` jobs cannot be truly aborted once started, so wrapper-level “cancel” mechanisms are easy to over-promise.
- **Persistence / GPU ABI / validation risk:** this file does not own persistence, report generation, GPU ABI, or validation semantics, so fixes should not sprawl into those areas.

## Fourth-Pass Conclusion

The current third-pass bucket should change.

`Small surgical refactor` was too aggressive for `src-tauri/src/commands_analysis.rs` as currently evidenced. The live file still looks like a correctly scoped, thin Tauri boundary: it resolves candle snapshots first, offloads heavy synchronous work with `spawn_blocking`, and returns app-core results without smuggling in unrelated runtime policy (`src-tauri/src/commands_analysis.rs`, lines 8-57).

What the third-pass packet got right was the preservation story: keep the snapshot-before-handoff invariant, keep the wrapper read-only, and do not route it through unrelated optimization/artifact machinery.

What it missed or overstated was where the safe-now fix actually belongs.

- The concurrency concern is theoretically valid but not sufficiently evidenced here to justify a local semaphore.
- The cancellation concern may still be real at the product surface, but the repo’s own runtime documentation says the shared run guard/cancel path is for optimization/validation flows, so forcing analysis into that model would be regression-prone.
- The best backend-local hardening remains narrow wrapper tests and, at most, optional error-context cleanup if this file is already being touched.

So the conservative fourth-pass conclusion is:

- **Accept this file as-is.**
- **If the cancel affordance is still misleading, fix the frontend/task semantics first.**
- **Defer any analysis-specific cancellation or concurrency architecture until there is scoped task-model work and real runtime evidence.**
