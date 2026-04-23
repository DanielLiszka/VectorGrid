# PR-07 Meta ranking, holdout triage, and ML confidence

- package type: `core`
- copied context files: `7`
- missing context files: `0`

## Prompt Scope

## `PR-07` Meta ranking, holdout triage, and ML confidence

Owned problem:
how fixed validation outputs are turned into meta ranking, holdout admission, and
ML-confidence features or labels

Source packets:
`MR-11`, `MR-12`

Primary ownership files:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`

Context files:

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/validation_common.rs`

Own these symbols or decisions:

- meta-score weighting on fixed validation outputs
- shortlist admission and holdout triage heuristics
- final-score blending after holdout evidence exists
- ML-confidence feature use, target design, and calibration

Open problem:

- characterize how to combine fixed validation outputs into meta ranking,
  holdout triage, and confidence estimates under selection bias and repeated
  screening

Why this is hard here:

- meta selection acts on already filtered candidates rather than an unbiased
  sample
- holdout triage and final-score blending can create feedback between selection
  policy and observed evidence quality

Mathematical object:

- a staged selection functional that maps validation outputs into meta scores,
  holdout allocation decisions, and calibrated confidence targets

Notation seed:

- let `Y_i` denote the fixed validation-output vector for candidate `i`
- let `m_i = \mu(Y_i)` be the meta score
- let `h_i \in \{0,1\}` be the holdout-allocation decision
- let `q_i = \Pr(Z_i = 1 \mid \phi(Y_i))` be the calibrated confidence target
- the core question is whether there exists a stable staged rule
  `(m_i, h_i, q_i) = \Gamma(Y_i)` under the repo's selection bias

Primary questions:

- what combination rule is stable under selection bias and repeated filtering?
- what holdout triage rule is mathematically defensible under limited budget?
- which confidence features remain identifiable after upstream screening?

Required deliverables:

- formal staged-selection model
- candidate lemma set for bias-aware ranking or triage
- counterexamples showing failure of naive point-estimate weighting
- complexity and sample-efficiency consequences

Acceptance bar:

- every triage or blending proposal must state its bias assumptions explicitly

Verification hooks:

- workflow-regression parity
- holdout allocation sensitivity checks
- calibration drift falsifiers

Non-goals:

- do not redesign validation primitive definitions, verdict logic, or
  cross-ticker math; that belongs to `PR-06`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat `ta_workflow_regression.rs` as the workflow-baseline evidence surface
- if a proposed change requires new validation outputs, record that as a
  dependency on `PR-06`

## First-Pass Prompt

Treat `PR-07` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
## `PR-07` Meta ranking, holdout triage, and ML confidence

Owned problem:
how fixed validation outputs are turned into meta ranking, holdout admission, and
ML-confidence features or labels

Source packets:
`MR-11`, `MR-12`

Primary ownership files:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`

Context files:

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/validation_common.rs`

Own these symbols or decisions:

- meta-score weighting on fixed validation outputs
- shortlist admission and holdout triage heuristics
- final-score blending after holdout evidence exists
- ML-confidence feature use, target design, and calibration

Open problem:

- characterize how to combine fixed validation outputs into meta ranking,
  holdout triage, and confidence estimates under selection bias and repeated
  screening

Why this is hard here:

- meta selection acts on already filtered candidates rather than an unbiased
  sample
- holdout triage and final-score blending can create feedback between selection
  policy and observed evidence quality

Mathematical object:

- a staged selection functional that maps validation outputs into meta scores,
  holdout allocation decisions, and calibrated confidence targets

Notation seed:

- let `Y_i` denote the fixed validation-output vector for candidate `i`
- let `m_i = \mu(Y_i)` be the meta score
- let `h_i \in \{0,1\}` be the holdout-allocation decision
- let `q_i = \Pr(Z_i = 1 \mid \phi(Y_i))` be the calibrated confidence target
- the core question is whether there exists a stable staged rule
  `(m_i, h_i, q_i) = \Gamma(Y_i)` under the repo's selection bias

Primary questions:

- what combination rule is stable under selection bias and repeated filtering?
- what holdout triage rule is mathematically defensible under limited budget?
- which confidence features remain identifiable after upstream screening?

Required deliverables:

- formal staged-selection model
- candidate lemma set for bias-aware ranking or triage
- counterexamples showing failure of naive point-estimate weighting
- complexity and sample-efficiency consequences

Acceptance bar:

- every triage or blending proposal must state its bias assumptions explicitly

Verification hooks:

- workflow-regression parity
- holdout allocation sensitivity checks
- calibration drift falsifiers

Non-goals:

- do not redesign validation primitive definitions, verdict logic, or
  cross-ticker math; that belongs to `PR-06`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat `ta_workflow_regression.rs` as the workflow-baseline evidence surface
- if a proposed change requires new validation outputs, record that as a
  dependency on `PR-06`
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

Attack the first-pass answer for `PR-07` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
## `PR-07` Meta ranking, holdout triage, and ML confidence

Owned problem:
how fixed validation outputs are turned into meta ranking, holdout admission, and
ML-confidence features or labels

Source packets:
`MR-11`, `MR-12`

Primary ownership files:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`

Context files:

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/validation_common.rs`

Own these symbols or decisions:

- meta-score weighting on fixed validation outputs
- shortlist admission and holdout triage heuristics
- final-score blending after holdout evidence exists
- ML-confidence feature use, target design, and calibration

Open problem:

- characterize how to combine fixed validation outputs into meta ranking,
  holdout triage, and confidence estimates under selection bias and repeated
  screening

Why this is hard here:

- meta selection acts on already filtered candidates rather than an unbiased
  sample
- holdout triage and final-score blending can create feedback between selection
  policy and observed evidence quality

Mathematical object:

- a staged selection functional that maps validation outputs into meta scores,
  holdout allocation decisions, and calibrated confidence targets

Notation seed:

- let `Y_i` denote the fixed validation-output vector for candidate `i`
- let `m_i = \mu(Y_i)` be the meta score
- let `h_i \in \{0,1\}` be the holdout-allocation decision
- let `q_i = \Pr(Z_i = 1 \mid \phi(Y_i))` be the calibrated confidence target
- the core question is whether there exists a stable staged rule
  `(m_i, h_i, q_i) = \Gamma(Y_i)` under the repo's selection bias

Primary questions:

- what combination rule is stable under selection bias and repeated filtering?
- what holdout triage rule is mathematically defensible under limited budget?
- which confidence features remain identifiable after upstream screening?

Required deliverables:

- formal staged-selection model
- candidate lemma set for bias-aware ranking or triage
- counterexamples showing failure of naive point-estimate weighting
- complexity and sample-efficiency consequences

Acceptance bar:

- every triage or blending proposal must state its bias assumptions explicitly

Verification hooks:

- workflow-regression parity
- holdout allocation sensitivity checks
- calibration drift falsifiers

Non-goals:

- do not redesign validation primitive definitions, verdict logic, or
  cross-ticker math; that belongs to `PR-06`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat `ta_workflow_regression.rs` as the workflow-baseline evidence surface
- if a proposed change requires new validation outputs, record that as a
  dependency on `PR-06`
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

Produce the strongest implementation-ready mathematical result for `PR-07` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
## `PR-07` Meta ranking, holdout triage, and ML confidence

Owned problem:
how fixed validation outputs are turned into meta ranking, holdout admission, and
ML-confidence features or labels

Source packets:
`MR-11`, `MR-12`

Primary ownership files:

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_campaign.rs`

Context files:

- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/validation_common.rs`

Own these symbols or decisions:

- meta-score weighting on fixed validation outputs
- shortlist admission and holdout triage heuristics
- final-score blending after holdout evidence exists
- ML-confidence feature use, target design, and calibration

Open problem:

- characterize how to combine fixed validation outputs into meta ranking,
  holdout triage, and confidence estimates under selection bias and repeated
  screening

Why this is hard here:

- meta selection acts on already filtered candidates rather than an unbiased
  sample
- holdout triage and final-score blending can create feedback between selection
  policy and observed evidence quality

Mathematical object:

- a staged selection functional that maps validation outputs into meta scores,
  holdout allocation decisions, and calibrated confidence targets

Notation seed:

- let `Y_i` denote the fixed validation-output vector for candidate `i`
- let `m_i = \mu(Y_i)` be the meta score
- let `h_i \in \{0,1\}` be the holdout-allocation decision
- let `q_i = \Pr(Z_i = 1 \mid \phi(Y_i))` be the calibrated confidence target
- the core question is whether there exists a stable staged rule
  `(m_i, h_i, q_i) = \Gamma(Y_i)` under the repo's selection bias

Primary questions:

- what combination rule is stable under selection bias and repeated filtering?
- what holdout triage rule is mathematically defensible under limited budget?
- which confidence features remain identifiable after upstream screening?

Required deliverables:

- formal staged-selection model
- candidate lemma set for bias-aware ranking or triage
- counterexamples showing failure of naive point-estimate weighting
- complexity and sample-efficiency consequences

Acceptance bar:

- every triage or blending proposal must state its bias assumptions explicitly

Verification hooks:

- workflow-regression parity
- holdout allocation sensitivity checks
- calibration drift falsifiers

Non-goals:

- do not redesign validation primitive definitions, verdict logic, or
  cross-ticker math; that belongs to `PR-06`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat `ta_workflow_regression.rs` as the workflow-baseline evidence surface
- if a proposed change requires new validation outputs, record that as a
  dependency on `PR-06`
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

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_confidence.rs`
- `src-tauri/src/bin/ta_workflow_regression.rs`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_meta.rs`
- `src-tauri/src/commands_reports_meta_xlsx.rs`
- `src-tauri/src/validation_common.rs`
