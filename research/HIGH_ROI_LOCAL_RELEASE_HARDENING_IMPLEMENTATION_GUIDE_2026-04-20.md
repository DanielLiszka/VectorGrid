# High-ROI Local Release Hardening Implementation Guide

Date: `2026-04-20`

## Objective

Improve release readiness through additive, non-frontend work that materially raises local verification quality, packaging confidence, and runtime safety without:

- enabling CI/CD
- removing any existing feature or workflow
- narrowing CPU-only support
- changing optimization, validation, or selection semantics
- introducing speculative refactors with unclear payoff

This guide is intentionally local-first. It assumes release quality is enforced by explicit local scripts, reproducible build tooling, packaged-artifact smoke checks, and stronger runtime/readiness diagnostics.

---

## Why this is the highest-ROI next step

The repo already has strong local primitives:

- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/check_tauri_config_sync.mjs`
- `npm --prefix web run parity`
- `npm --prefix web run parity:outputs`
- `npm --prefix web run test:browser:ux`
- `npm --prefix web run test:desktop:smoke`

The main gap is orchestration and release-specific proof, not missing low-level capability.

Today the repo can prove many individual things, but it still relies on the operator to:

- remember the right sequence
- remember which variant requires which checks
- remember which package artifact is the real ship target
- manually gather provenance and artifact metadata
- manually determine whether local runtime prerequisites are satisfied

That makes release quality too dependent on memory and habit. The highest-ROI additive improvement is to formalize the local release-candidate path without changing product behavior.

---

## Recommended implementation order

1. Release doctor and runtime readiness snapshot
2. Toolchain pinning and provenance capture
3. Local release-candidate gate wrapper
4. Release artifact manifest and integrity generation
5. Packaged-artifact smoke verification
6. Optional provider/live safety harness

Reasoning:

- item 1 prevents avoidable local release failures before expensive work starts
- item 2 improves reproducibility and release triage
- item 3 turns fragmented local gates into one authoritative release command
- item 4 makes shipped assets auditable
- item 5 closes the biggest remaining packaging blind spot
- item 6 is valuable, but should follow the release-process hardening first

---

## Primary files and surfaces

### Release and build surfaces

- `RELEASE.md`
- `README.md`
- `Cargo.toml`
- `Cargo.lock`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `tauri.conf.json`
- `tauri.windows.conf.json`
- `scripts/run_repo_quality_gate.ps1`
- `scripts/run_workspace_tests_stable.ps1`
- `scripts/run_workflow_gate.ps1`
- `scripts/run_cpu_perf_gate.ps1`
- `scripts/run_gpu_perf_gate.ps1`
- `scripts/check_tauri_config_sync.mjs`
- `scripts/post_release_telemetry_review.mjs`

### Desktop verification surfaces

- `web/package.json`
- `web/README.md`
- `web/e2e/desktop/README.md`
- `web/wdio.desktop.conf.ts`

### Runtime and provider surfaces

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands_system.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/state.rs`

### Existing output/evidence locations

- `output/benchmarks/`
- `output/wdio/desktop-smoke/`
- `output/playwright/`
- `output/logs/`

---

## Non-goals

This guide intentionally does not propose:

- re-enabling `.github/workflows`
- moving the repo to a hosted CI/CD release model
- broad frontend redesign
- changes to core strategy semantics
- removal of live execution, provider, or report functionality
- changing the supported public scope beyond Windows x64
- treating debug-shell proof as a substitute for packaged-artifact proof
- replacing existing local gates that already work

The goal is to compose and harden existing repo strengths, not to restart release engineering from scratch.

---

## Workstream 1: Release Doctor and Runtime Readiness Snapshot

### Problem

The repo has the data needed to reason about release readiness, but that information is split across:

- GPU status commands
- credential status commands
- provider/live-stream state
- Tauri config sync checks
- packaging-resource expectations
- docs in `RELEASE.md`

That increases operator error and slows triage.

### Objective

Add one authoritative local readiness check that reports whether a specific release variant can be trusted to proceed.

### Proposed implementation

#### Phase 1: Add a reusable release-readiness module

Add a small backend-side readiness module, for example:

- `src-tauri/src/release_readiness.rs`

Responsibilities:

- normalize variant input:
  - `cpu`
  - `gpu-ma`
  - `gpu-kernel`
- collect runtime/environment facts
- return structured readiness diagnostics

Suggested output shape:

- `variant`
- `supported`
- `errors`
- `warnings`
- `gpu`
- `resources`
- `tooling`
- `provider`
- `notes`

#### Phase 2: Reuse existing runtime facts rather than duplicating logic

Source facts from existing surfaces where practical:

- GPU capability:
  - `src-tauri/src/commands_system.rs`
- Alpaca credential status and connection preconditions:
  - `src-tauri/src/commands_execution.rs`
- Polygon key/storage behavior and stream status:
  - `src-tauri/src/commands_data.rs`
  - `src-tauri/src/polygon.rs`
- required release resources:
  - PTX presence for kernel lane
  - CatBoost DLL / bundled native resource presence where relevant
- Tauri config mirror sync:
  - `scripts/check_tauri_config_sync.mjs`

Do not add a frontend command for this first pass. Keep it local and low-risk.

#### Phase 3: Expose the readiness check through a dedicated local executable

Add one local CLI surface, for example:

- `src-tauri/src/bin/ta_release_readiness.rs`

Suggested CLI:

- `cargo run -p ta_desktop_demo_app --bin ta_release_readiness -- --variant cpu`
- `cargo run -p ta_desktop_demo_app --bin ta_release_readiness -- --variant gpu-ma`
- `cargo run -p ta_desktop_demo_app --bin ta_release_readiness -- --variant gpu-kernel --json output/release_candidates/<stamp>/readiness.json`

#### Phase 4: Add a PowerShell wrapper for operators

Add:

- `scripts/run_release_doctor.ps1`

Responsibilities:

- run `check_tauri_config_sync`
- call `ta_release_readiness`
- print concise pass/warn/fail output
- write JSON evidence under `output/release_candidates/<stamp>/readiness.json`

### Checks to implement

The doctor should check at minimum:

- variant name is valid
- Tauri config mirrors are in sync
- Rust release binary build prerequisites are present
- for GPU variants:
  - NVIDIA GPU visible
  - driver info readable
  - prebuilt PTX present for kernel lane
- Windows bundle-resource prerequisites are present
- required local tools are discoverable:
  - `cargo`
  - `npm`
  - `cargo-tauri` when bundle build requested
  - `tauri-driver` and `msedgedriver` when packaged smoke requested
- provider posture summary:
  - Alpaca credential configured or not
  - Polygon key configured or not
  - live-stream constraints and scope warnings

### Acceptance criteria

- one command answers “can I trust this machine and repo state for this release variant?”
- failures are specific and actionable
- GPU-specific issues do not block CPU-only release work
- no existing command or workflow is removed

### Verification

- run `scripts/run_release_doctor.ps1 -Variant cpu`
- run `scripts/run_release_doctor.ps1 -Variant gpu-ma`
- run `scripts/run_release_doctor.ps1 -Variant gpu-kernel`
- force at least one failure case locally:
  - invalid variant
  - missing `cargo-tauri`
  - missing PTX path check for kernel lane

### Checklist

- [ ] Add `src-tauri/src/release_readiness.rs`
- [ ] Add `src-tauri/src/bin/ta_release_readiness.rs`
- [ ] Add `scripts/run_release_doctor.ps1`
- [ ] Reuse existing GPU/provider logic instead of reimplementing it
- [ ] Emit JSON and operator-readable text output
- [ ] Document the doctor in `README.md` and `RELEASE.md`

---

## Workstream 2: Toolchain Pinning and Release Provenance Capture

### Problem

The repo is strong on local scripts, but weaker on reproducibility because the release environment is not fully pinned and captured.

That increases risk when:

- one machine builds with a different Rust toolchain than another
- local operator reports cannot be tied back to an exact toolchain
- release failures cannot be reproduced cleanly

### Objective

Make release-candidate builds locally reproducible and diagnosable without enabling CI/CD.

### Proposed implementation

#### Phase 1: Pin the Rust toolchain

Add:

- `rust-toolchain.toml`

Recommended content:

- stable channel only
- include `rustfmt`
- include `clippy`

Do not pin nightly. The repo docs already present the expected posture as stable.

#### Phase 2: Capture release-provenance metadata

Add:

- `scripts/capture_release_provenance.ps1`

Capture at minimum:

- `rustc --version --verbose`
- `cargo --version --verbose`
- `cargo tauri --version` if present
- `node --version`
- `npm --version`
- `tauri-driver --version` if present
- `msedgedriver` version if present
- GPU driver info if present
- important env toggles:
  - `VECTORBT_USE_PREBUILT_PTX`
  - `VECTORBT_REQUIRE_NVCC`
- current git commit and worktree dirty state
- target variant

Write output to:

- `output/release_candidates/<stamp>/provenance.json`
- optionally `provenance.txt` for quick operator reading

#### Phase 3: Attach provenance to every release-candidate run

The future release-candidate gate should always call provenance capture before build and again after build if variant-specific environment changes occur.

### Acceptance criteria

- every release-candidate output directory has provenance attached
- the repo’s documented stable-toolchain posture is encoded in the workspace
- local release triage can answer “what exact environment produced this artifact?”

### Verification

- confirm `cargo` respects the new toolchain pin
- run `scripts/capture_release_provenance.ps1 -Variant cpu`
- inspect JSON for all expected fields
- verify behavior on a dirty worktree and on a clean worktree

### Checklist

- [ ] Add `rust-toolchain.toml`
- [ ] Add `scripts/capture_release_provenance.ps1`
- [ ] Capture tool versions, driver info, env toggles, commit, and dirty state
- [ ] Emit machine-readable and human-readable provenance
- [ ] Reference the provenance step in `RELEASE.md`

---

## Workstream 3: Local Release-Candidate Gate Wrapper

### Problem

The repo already has strong local gates, but release proof is fragmented across separate commands and docs.

That creates three avoidable problems:

- inconsistent operator behavior
- inconsistent release evidence
- easier accidental omission of high-value checks

### Objective

Add one authoritative local release-candidate command that composes the repo’s existing gates in the correct order for a chosen variant.

### Proposed implementation

#### Phase 1: Add a wrapper script

Add:

- `scripts/run_release_candidate_gate.ps1`

Suggested parameters:

- `-Variant cpu|gpu-ma|gpu-kernel`
- `-IncludePerf`
- `-IncludeDesktopSmoke`
- `-OutDir output/release_candidates/<stamp>`
- `-SkipPackagedSmoke`
- `-SkipWorkspaceTests`

#### Phase 2: Encode the sequence explicitly

Recommended order:

1. release doctor
2. provenance capture
3. repo quality gate with:
   - `-IncludeDesktopTypecheck`
   - `-IncludeWorkspaceTests`
4. web parity:
   - `npm --prefix web run parity`
   - `npm --prefix web run parity:outputs`
5. workflow gate:
   - `scripts/run_workflow_gate.ps1 -Mode candidate`
6. optional perf gates:
   - CPU always for release-quality evidence
   - GPU only for GPU variants
7. build the chosen Tauri bundle variant
8. generate manifest and hashes
9. run packaged smoke
10. emit one summary JSON and one summary Markdown file

#### Phase 3: Make variant-aware decisions explicit

Variant-specific behavior should be encoded, not left to memory:

- `cpu`
  - no GPU perf gate
  - no GPU readiness requirement
- `gpu-ma`
  - require GPU visibility
  - run GPU perf gate only if requested
- `gpu-kernel`
  - require GPU visibility
  - require PTX availability
  - run GPU perf gate if requested
  - set `VECTORBT_USE_PREBUILT_PTX=1` for release-candidate build unless explicitly overridden

#### Phase 4: Emit one summary artifact

Write:

- `output/release_candidates/<stamp>/summary.json`
- `output/release_candidates/<stamp>/summary.md`

Summary should include:

- variant
- pass/fail by step
- command lines used
- build output paths
- hashes manifest path
- provenance path
- packaged smoke result
- key warnings

### Acceptance criteria

- one command can perform a full local release-candidate rehearsal
- the wrapper never runs multiple perf gates at the same time
- each step writes durable evidence under `output/release_candidates/`
- the release operator no longer has to remember the sequence manually

### Verification

- run CPU-only lane end to end
- run GPU-kernel lane end to end on a supported machine
- validate that a forced failure stops later steps and still writes partial summary evidence

### Checklist

- [ ] Add `scripts/run_release_candidate_gate.ps1`
- [ ] Compose existing gates instead of duplicating their logic
- [ ] Make variant-specific rules explicit
- [ ] Serialize perf work and benchmark work
- [ ] Emit `summary.json` and `summary.md`
- [ ] Update `RELEASE.md` with the new authoritative local release-candidate command

---

## Workstream 4: Release Artifact Manifest and Integrity Generation

### Problem

The current release guidance has naming rules, but there is no enforced local artifact manifest that answers:

- what exactly was built
- which variant it is
- what files belong to it
- whether the artifact names match policy
- whether expected native resources are actually present

### Objective

Make every release-candidate build auditable and less error-prone through manifest and hash generation.

### Proposed implementation

#### Phase 1: Add manifest generation

Add:

- `scripts/generate_release_manifest.ps1`

Inputs:

- variant
- version
- build output root
- provenance path

Outputs:

- `output/release_candidates/<stamp>/manifest.json`
- `output/release_candidates/<stamp>/SHA256SUMS.txt`

#### Phase 2: Record the exact shipped files

Manifest should include:

- product name
- version
- variant
- expected public asset naming
- actual output file names
- file sizes
- SHA-256 hashes
- build timestamp
- git revision
- provenance file reference
- presence of expected native resources:
  - PTX for kernel lane
  - CatBoost native DLL where applicable
- Tauri config identifiers:
  - bundle identifier
  - product name

#### Phase 3: Validate naming and required-resource policy

The generator should fail or warn when:

- artifact names do not match the variant naming policy from `RELEASE.md`
- expected bundle resources are missing
- multiple ambiguous primary artifacts are present

### Acceptance criteria

- each release-candidate directory contains a machine-readable manifest
- each release-candidate directory contains reproducible SHA-256 hashes
- artifact naming and required-resource policy are enforced locally

### Verification

- run manifest generation for each variant
- compare CPU-only vs GPU outputs
- force at least one naming mismatch or missing-resource failure

### Checklist

- [ ] Add `scripts/generate_release_manifest.ps1`
- [ ] Emit `manifest.json`
- [ ] Emit `SHA256SUMS.txt`
- [ ] Validate expected variant naming
- [ ] Validate required native resources per variant
- [ ] Link manifest output from the release-candidate summary

---

## Workstream 5: Packaged-Artifact Smoke Verification

### Problem

The repo already has desktop smoke, but the strongest current harness still centers on a debug-built app and managed preview host flow.

That leaves a real release blind spot:

- a packaged bundle can differ from a debug-shell success

### Objective

Smoke-test the actual packaged artifact produced by `cargo tauri build`, not just the debug shell.

### Proposed implementation

#### Phase 1: Reuse the existing desktop harness instead of creating a new one

The existing desktop harness already supports:

- `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1`
- `VECTORGRID_DESKTOP_SMOKE_BINARY=<path>`

Use that rather than building a separate automation stack.

#### Phase 2: Add a packaged-smoke wrapper

Add:

- `scripts/run_packaged_smoke.ps1`

Responsibilities:

- resolve the built release executable for the selected variant
- set:
  - `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1`
  - `VECTORGRID_DESKTOP_SMOKE_BINARY=<resolved-packaged-exe>`
- run:
  - `npm --prefix web run test:desktop:smoke`
- collect smoke artifacts into:
  - `output/release_candidates/<stamp>/desktop-smoke/`

#### Phase 3: Define the packaged smoke boundary clearly

Initial packaged-smoke scope should prove:

- app launches
- main shell is ready
- titlebar exists
- command palette opens
- task center opens
- settings/diagnostics surface opens
- app closes cleanly

Do not expand this first pass into broad screenshot or dense-results E2E assertions. The goal is packaging confidence.

### Acceptance criteria

- the release-candidate flow proves that the built artifact launches and reaches shell readiness
- packaged smoke uses the real artifact path, not an implicitly rebuilt debug path
- failures capture logs and screenshots under the release-candidate output directory

### Verification

- run packaged smoke for CPU-only build
- run packaged smoke for one GPU build
- force a wrong binary path and confirm failure is immediate and clear

### Checklist

- [ ] Add `scripts/run_packaged_smoke.ps1`
- [ ] Reuse `VECTORGRID_DESKTOP_SMOKE_BINARY`
- [ ] Reuse the existing WDIO desktop harness
- [ ] Copy smoke logs and screenshots into the release-candidate output directory
- [ ] Document packaged smoke in `web/e2e/desktop/README.md` and `RELEASE.md`

---

## Workstream 6: Optional Provider and Live-Execution Safety Harness

### Problem

The repo’s live/provider path is intentionally narrower and riskier than the optimize/validate path. The safety logic appears thoughtful, but it is not yet formalized into one local release-quality harness.

### Objective

Add a repeatable local harness for the highest-risk non-frontend runtime behaviors without expanding live-product scope.

### Proposed implementation

#### Phase 1: Add a local safety harness

Add either:

- `scripts/run_live_execution_safety_harness.ps1`

or a small dedicated bin plus wrapper:

- `src-tauri/src/bin/ta_live_execution_safety_harness.rs`
- `scripts/run_live_execution_safety_harness.ps1`

#### Phase 2: Cover the highest-value cases only

Recommended scenarios:

- missing Alpaca credentials with `dry_run=false`
- valid `dry_run=true` path
- emergency halt flips the system into explicit manual-review posture
- feed interruption disarms the runner
- symbol mismatch / invalid stream preconditions reject startup

#### Phase 3: Emit structured evidence

Write:

- `output/release_candidates/<stamp>/live-safety-summary.json`

Include:

- scenario
- expected outcome
- actual outcome
- any `requires_manual_review` or disarm transitions

### Acceptance criteria

- the repo has a repeatable local proof layer for live-execution guardrails
- failures are easy to inspect and preserve
- live scope is not widened

### Verification

- run all scenarios in `dry_run`
- force at least one negative case and confirm deterministic rejection

### Checklist

- [ ] Add a local live/provider safety harness
- [ ] Cover the highest-risk cases only
- [ ] Emit structured output under `output/release_candidates/`
- [ ] Keep the harness opt-in, not part of every default local dev loop

---

## Recommended first implementation slice

If this guide is implemented incrementally, the best first slice is:

1. `rust-toolchain.toml`
2. `scripts/capture_release_provenance.ps1`
3. `scripts/run_release_doctor.ps1`
4. `scripts/run_release_candidate_gate.ps1`

That first slice delivers the largest operator-confidence gain at the lowest regression risk because it mostly composes existing functionality and adds release evidence rather than changing runtime behavior.

After that, implement:

5. `scripts/generate_release_manifest.ps1`
6. `scripts/run_packaged_smoke.ps1`

Treat the live/provider safety harness as a follow-up slice after the release-candidate path is stable.

---

## Repo-level acceptance criteria for the whole guide

The guide should be considered complete when all of the following are true:

- one local command can perform a full release-candidate rehearsal for a chosen variant
- release-candidate runs emit durable evidence under `output/release_candidates/`
- the build environment is pinned and captured
- packaged artifact identity is recorded and hashed
- the packaged artifact, not only the debug shell, is smoke-tested
- all of the above are additive and leave existing product workflows intact

---

## Final implementation checklist

- [ ] Add `rust-toolchain.toml`
- [ ] Add `src-tauri/src/release_readiness.rs`
- [ ] Add `src-tauri/src/bin/ta_release_readiness.rs`
- [ ] Add `scripts/run_release_doctor.ps1`
- [ ] Add `scripts/capture_release_provenance.ps1`
- [ ] Add `scripts/run_release_candidate_gate.ps1`
- [ ] Add `scripts/generate_release_manifest.ps1`
- [ ] Add `scripts/run_packaged_smoke.ps1`
- [ ] Optionally add `scripts/run_live_execution_safety_harness.ps1`
- [ ] Update `README.md`
- [ ] Update `RELEASE.md`
- [ ] Update `web/e2e/desktop/README.md`
- [ ] Keep CPU-only support intact
- [ ] Keep existing release variants intact
- [ ] Avoid enabling CI/CD as part of this lane

