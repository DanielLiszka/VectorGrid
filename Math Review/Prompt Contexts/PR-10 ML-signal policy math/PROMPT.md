# PR-10 ML-signal policy math

- package type: `core`
- copied context files: `6`
- missing context files: `0`

## Prompt Scope

## `PR-10` ML-signal policy math

Owned problem:
ML-signal label design, calibration transforms, threshold or delta rules, and
overlay or sweep behavior

Source packets:
`MR-17`

Primary ownership files:

- `crates/app_core/src/ml_signal.rs`

Context files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Own these symbols or decisions:

- label deadband and split-window design
- calibration interpolation and probability-contract checks
- threshold or delta policy geometry
- persistence and cooldown overlays
- sweep ranking and truncation rules

Open problem:

- characterize a policy-ranking geometry for ML-signal outputs that turns score
  caches into stable threshold/delta decisions under calibration error,
  dependence, and overlay logic

Why this is hard here:

- label design, score calibration, and threshold policies are all coupled
- the signal layer consumes validation-style primitives but must produce a
  policy surface rather than just a score surface

Mathematical object:

- a map from calibrated score sequences and overlay states to discrete
  threshold/delta actions, together with a sweep functional over candidate
  policy settings

Notation seed:

- let `p_t \in [0,1]` be the calibrated score at time `t`
- let `s_t` denote any overlay state (cooldown, persistence, position, etc.)
- let `\theta = (\tau_{\uparrow}, \tau_{\downarrow}, h, c, \dots)` denote a
  policy parameter tuple
- let `a_t = \pi_\theta(p_{1:t}, s_t)` be the induced action
- let `J(\theta)` be the sweep objective used to rank candidate policies

Primary questions:

- what threshold geometry yields stable policy ranking under calibration error?
- when can overlay or cooldown logic be compressed into a finite sufficient
  state?
- what sweep truncation rule preserves the true best policy under bounded rank
  error?

Required deliverables:

- formal policy-surface model
- candidate theorem set for stable threshold ranking
- counterexamples where calibration or dependence breaks naive thresholding
- explicit rank-instability or regret bounds for sweep truncation

Acceptance bar:

- every policy claim must separate score quality from decision-rule quality

Verification hooks:

- export/native-smoke parity
- calibration drift falsifiers
- threshold-order instability checks

Non-goals:

- do not redesign the generic validation stack; that belongs to `PR-06`
- do not redesign meta-select or ML-confidence policy; that belongs to `PR-07`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat score-cache coverage rules as first-class math contracts
- treat export and native-smoke flows as verification context, not redesign
  targets

## First-Pass Prompt

Treat `PR-10` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
## `PR-10` ML-signal policy math

Owned problem:
ML-signal label design, calibration transforms, threshold or delta rules, and
overlay or sweep behavior

Source packets:
`MR-17`

Primary ownership files:

- `crates/app_core/src/ml_signal.rs`

Context files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Own these symbols or decisions:

- label deadband and split-window design
- calibration interpolation and probability-contract checks
- threshold or delta policy geometry
- persistence and cooldown overlays
- sweep ranking and truncation rules

Open problem:

- characterize a policy-ranking geometry for ML-signal outputs that turns score
  caches into stable threshold/delta decisions under calibration error,
  dependence, and overlay logic

Why this is hard here:

- label design, score calibration, and threshold policies are all coupled
- the signal layer consumes validation-style primitives but must produce a
  policy surface rather than just a score surface

Mathematical object:

- a map from calibrated score sequences and overlay states to discrete
  threshold/delta actions, together with a sweep functional over candidate
  policy settings

Notation seed:

- let `p_t \in [0,1]` be the calibrated score at time `t`
- let `s_t` denote any overlay state (cooldown, persistence, position, etc.)
- let `\theta = (\tau_{\uparrow}, \tau_{\downarrow}, h, c, \dots)` denote a
  policy parameter tuple
- let `a_t = \pi_\theta(p_{1:t}, s_t)` be the induced action
- let `J(\theta)` be the sweep objective used to rank candidate policies

Primary questions:

- what threshold geometry yields stable policy ranking under calibration error?
- when can overlay or cooldown logic be compressed into a finite sufficient
  state?
- what sweep truncation rule preserves the true best policy under bounded rank
  error?

Required deliverables:

- formal policy-surface model
- candidate theorem set for stable threshold ranking
- counterexamples where calibration or dependence breaks naive thresholding
- explicit rank-instability or regret bounds for sweep truncation

Acceptance bar:

- every policy claim must separate score quality from decision-rule quality

Verification hooks:

- export/native-smoke parity
- calibration drift falsifiers
- threshold-order instability checks

Non-goals:

- do not redesign the generic validation stack; that belongs to `PR-06`
- do not redesign meta-select or ML-confidence policy; that belongs to `PR-07`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat score-cache coverage rules as first-class math contracts
- treat export and native-smoke flows as verification context, not redesign
  targets
```

Required deliverables:

1. Formal problem statement
2. Notation for the search space, state space, path functional, ranking functional, and admissible approximations
3. Fixed invariants and interface constraints
4. Strongest plausible positive result
5. Strongest plausible impossibility result
6. Candidate theorem or lemma list
7. One explicit counterexample family
8. One lower-bound, identifiability, or impossibility angle
9. Unresolved questions that block implementation

## Second-Pass Prompt

Attack the first-pass answer for `PR-10` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
## `PR-10` ML-signal policy math

Owned problem:
ML-signal label design, calibration transforms, threshold or delta rules, and
overlay or sweep behavior

Source packets:
`MR-17`

Primary ownership files:

- `crates/app_core/src/ml_signal.rs`

Context files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Own these symbols or decisions:

- label deadband and split-window design
- calibration interpolation and probability-contract checks
- threshold or delta policy geometry
- persistence and cooldown overlays
- sweep ranking and truncation rules

Open problem:

- characterize a policy-ranking geometry for ML-signal outputs that turns score
  caches into stable threshold/delta decisions under calibration error,
  dependence, and overlay logic

Why this is hard here:

- label design, score calibration, and threshold policies are all coupled
- the signal layer consumes validation-style primitives but must produce a
  policy surface rather than just a score surface

Mathematical object:

- a map from calibrated score sequences and overlay states to discrete
  threshold/delta actions, together with a sweep functional over candidate
  policy settings

Notation seed:

- let `p_t \in [0,1]` be the calibrated score at time `t`
- let `s_t` denote any overlay state (cooldown, persistence, position, etc.)
- let `\theta = (\tau_{\uparrow}, \tau_{\downarrow}, h, c, \dots)` denote a
  policy parameter tuple
- let `a_t = \pi_\theta(p_{1:t}, s_t)` be the induced action
- let `J(\theta)` be the sweep objective used to rank candidate policies

Primary questions:

- what threshold geometry yields stable policy ranking under calibration error?
- when can overlay or cooldown logic be compressed into a finite sufficient
  state?
- what sweep truncation rule preserves the true best policy under bounded rank
  error?

Required deliverables:

- formal policy-surface model
- candidate theorem set for stable threshold ranking
- counterexamples where calibration or dependence breaks naive thresholding
- explicit rank-instability or regret bounds for sweep truncation

Acceptance bar:

- every policy claim must separate score quality from decision-rule quality

Verification hooks:

- export/native-smoke parity
- calibration drift falsifiers
- threshold-order instability checks

Non-goals:

- do not redesign the generic validation stack; that belongs to `PR-06`
- do not redesign meta-select or ML-confidence policy; that belongs to `PR-07`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat score-cache coverage rules as first-class math contracts
- treat export and native-smoke flows as verification context, not redesign
  targets
```

Required deliverables:

1. Explicit failure modes
2. Adversarial constructions or separating examples
3. Corrected theorem or lemma candidates
4. Weakest assumptions under which the result survives
5. Distinction between exact, approximate, and heuristic claims
6. Revised time, memory, and retained-state costs
7. Dependency burden on other PR prompts

Narrow to the strongest surviving repo-fit result.

## Third-Pass Prompt

Produce the strongest implementation-ready mathematical result for `PR-10` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
## `PR-10` ML-signal policy math

Owned problem:
ML-signal label design, calibration transforms, threshold or delta rules, and
overlay or sweep behavior

Source packets:
`MR-17`

Primary ownership files:

- `crates/app_core/src/ml_signal.rs`

Context files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Own these symbols or decisions:

- label deadband and split-window design
- calibration interpolation and probability-contract checks
- threshold or delta policy geometry
- persistence and cooldown overlays
- sweep ranking and truncation rules

Open problem:

- characterize a policy-ranking geometry for ML-signal outputs that turns score
  caches into stable threshold/delta decisions under calibration error,
  dependence, and overlay logic

Why this is hard here:

- label design, score calibration, and threshold policies are all coupled
- the signal layer consumes validation-style primitives but must produce a
  policy surface rather than just a score surface

Mathematical object:

- a map from calibrated score sequences and overlay states to discrete
  threshold/delta actions, together with a sweep functional over candidate
  policy settings

Notation seed:

- let `p_t \in [0,1]` be the calibrated score at time `t`
- let `s_t` denote any overlay state (cooldown, persistence, position, etc.)
- let `\theta = (\tau_{\uparrow}, \tau_{\downarrow}, h, c, \dots)` denote a
  policy parameter tuple
- let `a_t = \pi_\theta(p_{1:t}, s_t)` be the induced action
- let `J(\theta)` be the sweep objective used to rank candidate policies

Primary questions:

- what threshold geometry yields stable policy ranking under calibration error?
- when can overlay or cooldown logic be compressed into a finite sufficient
  state?
- what sweep truncation rule preserves the true best policy under bounded rank
  error?

Required deliverables:

- formal policy-surface model
- candidate theorem set for stable threshold ranking
- counterexamples where calibration or dependence breaks naive thresholding
- explicit rank-instability or regret bounds for sweep truncation

Acceptance bar:

- every policy claim must separate score quality from decision-rule quality

Verification hooks:

- export/native-smoke parity
- calibration drift falsifiers
- threshold-order instability checks

Non-goals:

- do not redesign the generic validation stack; that belongs to `PR-06`
- do not redesign meta-select or ML-confidence policy; that belongs to `PR-07`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat score-cache coverage rules as first-class math contracts
- treat export and native-smoke flows as verification context, not redesign
  targets
```

Required deliverables:

1. Final formalization
2. Final theorem or lemma list
3. Proof sketch or proof plan
4. Sufficient statistics, or a proof they do not exist under the stated constraints
5. Exactness conditions and equivalence classes
6. Error, regret, or bias bounds for every non-exact claim
7. Time, memory, and retained-state complexity
8. File and symbol impact map
9. Verification obligations and falsifiers
10. Dependencies that must be reconciled during consolidation
11. Kill criteria

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_ml_signal.rs`
