# Third-Pass Review: `src-tauri/src/commands_meta.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/commands_meta.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/commands_meta.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/src-tauri/src/commands_meta.rs.review.md`
- Second-pass action bucket: `Small surgical refactor`
- Third-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Third Pass

The second pass found real ML-confidence command-contract problems, but this file remained high ROI because the meta command family is large and the desktop task center treats several of these commands as cancelable tracked work.

## Third-Pass Research Log

1. Tokio `spawn_blocking`
   - Source: <https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html>
   - Why consulted in third pass:
     - confirm how strongly tracked desktop cancellation must be backed by cooperative polling
   - Third-pass constraint informed:
     - blocking work needs explicit cooperative cancel wiring if the UI advertises cancelability

2. Tauri calling Rust
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in third pass:
     - re-check the command boundary as a public runtime contract
   - Third-pass constraint informed:
     - command correctness here includes task-store and notification behavior

3. Rust `create_dir_all`
   - Source: <https://doc.rust-lang.org/std/fs/fn.create_dir_all.html>
   - Why consulted in third pass:
     - keep testing whether status-style helpers are still mutating disk
   - Third-pass constraint informed:
     - `ml_confidence_model_status` remains a read-only contract bug until its directory helper stops mutating storage

## What Third Pass Sharpened

### 1. The cancellation inconsistency is broader than the second pass stated

- Evidence:
  - `run_meta_selection` still owns the real shared cancel path
  - `run_meta_stage4_experimental`, `run_meta_stage5_ml_confidence`, and `evaluate_ml_confidence_policy` are still tracked/cancel-mapped in the frontend without the same cooperative cancel wiring
  - `train_ml_confidence_model` and `apply_ml_confidence_model` remain in the same inconsistent family
- Why this matters:
  - the desktop currently over-promises cancel behavior across more commands than the second-pass packet listed
- Practical implication:
  - the next surgical fix should address the whole tracked ML-confidence/meta family in one contract pass instead of patching only train/apply

### 2. The read-only status bug still stands, but it is a secondary fix next to task-contract honesty

- Evidence:
  - the status path still goes through a directory helper that eagerly creates storage
- Practical implication:
  - fix it, but do not route status into the shared run slot just to make it "consistent"

### 3. Atomic replacement remains the best bounded write hardening if model mutation is touched

- Evidence:
  - train/apply/delete still target the same final model path
- Practical implication:
  - temp-write-plus-rename remains the cleanest safe-now write-hardening primitive short of larger workflow redesign

## Implementation Priority

- Highest-ROI hardening:
  - make tracked meta/ML-confidence cancellation semantics honest across the whole command family
- Secondary hardening:
  - make `ml_confidence_model_status` truly read-only

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`
- `npm --prefix web run test -- src/services/metaService.test.ts src/stores/taskStore.test.ts`

Add targeted regression coverage for:

- tracked meta commands that currently map to `cancel_current_run`
- status inspection leaving the confidence-model directory absent when it did not exist
- overlapping model mutation flows if write semantics change

## Regression And Feature-Preservation Constraints

- Preserve `run_meta_selection`'s existing cooperative cancel behavior
- Keep read-only status inspection outside the shared run slot
- If the fix is contract cleanup rather than real cancellation, update frontend task-store mappings and notifications in lockstep
- Tighten model mutation safety without narrowing current workflow support

## Third-Pass Conclusion

`Bucket unchanged, but the contract cleanup scope is larger`

The third pass keeps this file in `Small surgical refactor`, but it widens the concrete target. The misleading tracked-cancel behavior is not limited to `train_ml_confidence_model` and `apply_ml_confidence_model`; it spans a broader meta command family and should be corrected as one bounded contract cleanup.
