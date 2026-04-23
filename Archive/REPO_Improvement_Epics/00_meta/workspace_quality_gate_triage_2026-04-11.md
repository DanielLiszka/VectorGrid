# Workspace Quality Gate Triage — 2026-04-11

## Scope

This note documents a do-no-harm triage pass against the failing broad Rust workspace gate before any source changes are attempted.

The objective was to answer two questions:

1. Is the broad `cargo test --workspace` failure real?
2. If it is real, are the reported `E0463` and related compiler errors primary source failures or secondary fallout from toolchain/resource pressure?

No product code or verification baselines were changed as part of this triage.

## Broad gate that failed

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1 -IncludeDesktopTypecheck -IncludeWorkspaceTests
```

Observed outcome:

- Rust format passed.
- Rust check passed.
- Rust clippy passed with warnings only.
- `cargo test --workspace` failed.

That means the gate failure is real at the workflow level, even if some individual diagnostics later prove to be secondary.

## Isolated reproductions

The following isolated commands were used to separate primary failures from cascading failures:

```powershell
cargo test -p ta_desktop_demo_app --bin ta_headless_validate --no-run
cargo test -p ta_desktop_demo_app --bin ta_app_path_faceoff --no-run
cargo test -p ta_desktop_demo_app --bin ta_options_expression_compare --no-run
cargo test -p ta_desktop_demo_app --bin ta_refactor_snapshots --no-run -j 1
cargo test -p ta_desktop_demo_app --bin ta_options_expression_feasibility --no-run
cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests --no-run -j 1
```

Observed outcome:

- `ta_headless_validate`: passed in isolation.
- `ta_app_path_faceoff`: passed in isolation.
- `ta_options_expression_compare`: passed in isolation.
- `ta_refactor_snapshots`: passed in isolation with `-j 1`.
- `ta_options_expression_feasibility`: compiled successfully, then failed at link time with `LINK : fatal error LNK1318: Unexpected PDB error`.
- `validation_xlsx_report_tests`: failed with the same `LNK1318` PDB linker failure even when reduced to `-j 1`.

## Findings

### 1. The gate failure is real, but the early `E0463` output is not a trustworthy primary diagnosis

The original broad workspace failure included:

- `E0463` "can't find crate"
- `E0786` invalid metadata errors
- `E0460` mixed crate version / newer crate metadata errors
- `STATUS_STACK_BUFFER_OVERRUN`
- Windows paging-file errors (`os error 1455`)
- `link.exe` `LNK1318` PDB failures

After isolated reruns, multiple targets that previously appeared broken compiled successfully. That rules out a simple "current source is missing crates" explanation for those targets.

Current interpretation:

- The full gate failure is real.
- Many of the `E0463` and metadata errors are secondary fallout after the build environment is already under stress.

### 2. The current evidence points first to host resource pressure, especially on the system drive

Direct host checks during triage showed:

- `C:` free space: about `0.07 GB`
- pagefile allocation present at `C:\pagefile.sys`
- earlier build logs already included:
  - `os error 112` "There is not enough space on the disk"
  - `os error 1455` "The paging file is too small for this operation to complete"
  - `LINK : fatal error LNK1318`

With only about 70 MB free on the system drive, linker, PDB, archive, metadata, and incremental-debug-artifact failures should be considered environment-first until proven otherwise.

### 3. The current evidence points second to Windows linker / debug-artifact pressure

Two heavier targets still fail cleanly in reduced isolated runs:

- `src-tauri/tests/validation_xlsx_report_tests.rs`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`

In both cases, the failure terminates at link time with:

```text
LINK : fatal error LNK1318: Unexpected PDB error; LIMIT (12)
```

That is materially different from a Rust source compile error. It means the build gets through Rust compilation and then fails while producing Windows debug artifacts.

### 4. The earlier options-expression suspicion was not confirmed

The first broad failure log made `src-tauri/src/bin/ta_options_expression_compare.rs` look suspect. That suspicion was not confirmed.

Isolated result:

- `src-tauri/src/bin/ta_options_expression_compare.rs` compiles successfully via `cargo test -p ta_desktop_demo_app --bin ta_options_expression_compare --no-run`

The related options-expression support-aware changes also look structurally closed on static review:

- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `src-tauri/src/commands_options_expression.rs`

So there is no current evidence that the present options-expression dirty-tree work is the primary cause of the workspace gate failure.

### 5. The workspace test surface is broader than it first appears

`cargo test --workspace` is compiling a large set of helper binaries, examples, and integration tests, including auto-discovered targets under:

- `src-tauri/src/bin/`
- `src-tauri/tests/`
- `crates/app_core/src/bin/`

That wider target surface increases debug-link pressure on Windows and makes cascading metadata and missing-crate diagnostics more likely after the environment starts failing.

## Classification

### Confirmed real failures

- The broad workspace test gate currently fails.
- `validation_xlsx_report_tests` currently fails in isolated `--no-run -j 1` mode at Windows link time with `LNK1318`.
- `ta_options_expression_feasibility` currently fails in isolated `--no-run` mode at Windows link time with `LNK1318`.

### Not confirmed as real source regressions

- `ta_headless_validate`
- `ta_app_path_faceoff`
- `ta_options_expression_compare`
- `ta_refactor_snapshots`

These compile successfully in isolation, so their earlier `E0463` output should be treated as cascading fallout, not primary evidence of broken source.

### Likely environmental or toolchain-pressure symptoms

- `E0463` missing-crate errors appearing after broader build stress
- `E0786` invalid metadata errors
- `os error 112` disk-full archive failures
- paging-file / `os error 1455`
- `STATUS_STACK_BUFFER_OVERRUN`
- mixed `windows_sys` version complaints appearing mid-failure

These are consistent with a stressed Windows debug build/link environment rather than a single coherent code-level regression.

## Recommended next steps

1. Treat this as a real workspace-gate failure, but not as proof that the current codebase broadly stopped compiling.
2. Avoid changing application logic until the remaining failures are narrowed to either:
   - reproducible source-level breakage in a specific target, or
   - reproducible Windows linker/debug-artifact limitations.
3. The first operational fix should be host cleanup, not source edits. The current free space on `C:` is too low for trustworthy Windows debug-link conclusions.
4. After free space is recovered, rerun the isolated failing targets before changing code.
5. Next diagnostic step after host cleanup should be build-surface reduction, not feature edits. Good candidates:
   - rerun the failing isolated targets with cleaner target directories
   - inspect Windows PDB/debug settings for the heavy `src-tauri` test/bin targets
   - consider whether some helper binaries should stop being part of the default `cargo test --workspace` surface
6. Keep this note separate from implementation ledgers until an actual fix is chosen and verified.

## Current conclusion

The failing workspace regression gate is real.

The strongest current evidence says the primary problem is host resource exhaustion, especially near-zero free space on `C:`, followed by Windows debug-link / artifact pressure on heavier `src-tauri` targets. It does not currently look like a broad Rust source regression across the repository. Some earlier reported missing-crate and metadata diagnostics were real outputs, but they do not currently look like trustworthy primary causes.

## Re-evaluation after disk cleanup

After reclaiming disk space, host checks showed:

- `C:` free space: about `117.47 GB`

The previously failing isolated commands were then rerun:

```powershell
cargo test -p ta_desktop_demo_app --test validation_xlsx_report_tests --no-run -j 1
cargo test -p ta_desktop_demo_app --bin ta_options_expression_feasibility --no-run -j 1
cargo test --workspace --no-run -j 1
```

Observed outcome:

- `validation_xlsx_report_tests` now passes in isolated `--no-run -j 1` mode.
- `ta_options_expression_feasibility` now passes in isolated `--no-run -j 1` mode.
- `cargo test --workspace --no-run -j 1` now passes across the full workspace compile surface.

That confirms the earlier linker / metadata / missing-crate failures were resource-conditioned, not persistent code failures.

## Remaining reproducible issue after host cleanup

With the compile surface restored, a full serial workspace test run:

```powershell
cargo test --workspace -j 1
```

now fails in exactly one place:

- `asr::tests::asr_metrics_emit_core17_and_trade22_fields_from_desired_positions`
  in [crates/app_core/src/asr.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/asr.rs:2881>)

The failing assertion is:

```rust
assert!(metrics.short_mean_return.abs() > 0.0);
```

## Likely classification of the remaining failure

Current evidence says this remaining failure is likely a stale test assumption, not a fresh regression in the metric pipeline.

Why:

1. The test builds its request with `StrategyConfig::default()` in [crates/app_core/src/asr.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/asr.rs:2886>).
2. `StrategyConfig::default()` currently sets `long_only: true` in [crates/strategies/src/double_ma/mod.rs](</C:/Rust Projects/ta_desktop_demo/crates/strategies/src/double_ma/mod.rs:781>).
3. The ASR metric evaluation explicitly clamps negative desired positions to flat when `cfg.long_only` is true in [crates/app_core/src/asr.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/asr.rs:2277>).

That means the test is currently asking for a non-zero short-side mean while simultaneously using a default strategy config that disables shorts. Under the current product semantics, a zero short-side mean is therefore expected.

## Updated conclusion

After host cleanup:

- the earlier workspace build/link failures are no longer reproducible
- the broad compile surface is healthy again
- the remaining reproducible failure is one likely-outdated ASR unit test assumption tied to `StrategyConfig::default()` being long-only

This is no longer a broad workspace instability problem. It has narrowed to a single, source-level test expectation that should be reviewed before any implementation changes are made.

## Follow-up fix

The ASR test was corrected by making its strategy intent explicit in [crates/app_core/src/asr.rs](</C:/Rust Projects/ta_desktop_demo/crates/app_core/src/asr.rs:2881>):

- the test now sets `strategy.long_only = false` before calling `eval_asr_metrics_from_desired`
- product/runtime behavior was not changed
- only the stale test assumption was corrected

Verification after the fix:

```powershell
cargo test -p ta_app_core asr::tests::asr_metrics_emit_core17_and_trade22_fields_from_desired_positions -- --exact
cargo test --workspace -j 1
```

Observed outcome:

- the exact ASR test passed
- the full serial workspace test suite passed

At this point the previously identified reproducible issue is resolved.
