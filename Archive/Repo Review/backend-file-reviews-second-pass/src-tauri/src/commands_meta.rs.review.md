# Second-Pass Review: `src-tauri/src/commands_meta.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_meta.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_meta.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already identified safe-now issues around silently skipped artifacts, ambiguous holdout-success semantics, and unlocked ML-confidence model mutation against a shared final model path.

## Stale Check

The live source still aligns with the main first-pass concerns:

- unreadable run artifacts can still disappear during meta selection
- holdout evaluation can still succeed after zero effective datasets
- model train/apply/delete flows still touch the same final model path without stronger mutation coordination

The second pass found two additional contract issues in the ML-confidence command family rather than a reason to expand this into a larger redesign.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - confirm the public command-boundary expectations for the ML-confidence family
   - Additional constraint informed:
     - command correctness here includes matching task-store expectations, not only keeping the signatures stable

2. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in second pass:
     - refine how strongly the review should treat tracked/cancelable long-running meta workflows
   - Additional constraint informed:
     - started blocking tasks are not hard-abortable, so any tracked cancel affordance must be backed by real cooperative wiring

3. Rust `std::fs::create_dir_all`
   - Source: <https://doc.rust-lang.org/std/fs/fn.create_dir_all.html>
   - Why consulted in second pass:
     - verify whether the status path is truly read-only
   - Additional constraint informed:
     - `ml_confidence_model_status` currently performs a real filesystem mutation through its directory helper

4. Rust `std::fs::rename`
   - Source: <https://doc.rust-lang.org/std/fs/fn.rename.html>
   - Why consulted in second pass:
     - keep evaluating whether there is a bounded write-hardening improvement short of architectural redesign
   - Additional constraint informed:
     - temp-write-plus-same-filesystem-rename remains the cleanest safe-now primitive for model replacement

## Delta vs First Pass

## New Read-Only Contract Finding

### 1. `ml_confidence_model_status` is not truly read-only because it still creates the confidence-model directory on disk

- Evidence:
  - the status command still resolves through a helper that eagerly calls `create_dir_all`
- Why this matters:
  - status-style commands should not mutate the model store merely because the user inspected it
- Practical implication:
  - this is a small, bounded fix and should stay outside the shared run slot

## New Cancellation-Contract Finding

### 2. The ML-confidence command family is internally inconsistent about tracked cancellation semantics

- Evidence:
  - `train_ml_confidence_model` and `apply_ml_confidence_model` are frontend-tracked and mapped to `cancel_current_run`
  - unlike `evaluate_ml_confidence_policy` and `run_meta_selection`, they do not use the shared run guard and cooperative cancel token
- Why this matters:
  - the frontend currently advertises a cancellation behavior that these commands do not actually implement
- Practical implication:
  - the fix is still surgical:
    - either wire these commands into the same shared run/cancel model,
    - or stop presenting them as cancelable tracked work

## Refined Safe-Now Opportunity

### 3. The model-write concern still has a bounded low-regression hardening path

- Evidence:
  - the final model path remains a shared mutation target across train/apply/delete flows
- Why this matters:
  - temp-write-plus-rename reduces partial-write exposure without forcing a large workflow rewrite
- Practical implication:
  - if this area is touched, pair command-level coordination with atomic replacement semantics instead of introducing broader churn

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
- `npm --prefix web run test -- src/services/metaService.test.ts src/components/MetaWorkspacePanel.test.ts src/stores/taskStore.test.ts`

Add targeted regression coverage for:

- `ml_confidence_model_status` leaving the model directory absent when it did not exist
- tracked `train_ml_confidence_model` and `apply_ml_confidence_model` either truly honoring `cancel_current_run` or no longer advertising that behavior
- overlapping train/delete/status flows against the same model path if write semantics change

## Regression And Feature-Preservation Constraints

- Preserve current Stage 4 / Stage 5 and meta artifact contracts
- Preserve shared-run exclusivity and cooperative cancellation for the commands that already use it
- Do not route read-only status inspection through the shared run slot just to fix directory mutation
- Tighten model mutation safety without narrowing current workflow support

## Second-Pass Conclusion

`Bucket unchanged, but the ML-confidence command family has two sharper contract gaps`

The first-pass `Small surgical refactor` bucket remains correct. The second pass found that `ml_confidence_model_status` still mutates disk on a read-only path and that two tracked ML-confidence commands are still advertised as cancelable without actually participating in the shared run/cancel model.
