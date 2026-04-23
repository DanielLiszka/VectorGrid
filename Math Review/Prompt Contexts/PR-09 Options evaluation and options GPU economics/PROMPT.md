# PR-09 Options evaluation and options GPU economics

- package type: `core`
- copied context files: `10`
- missing context files: `0`

## Prompt Scope

## `PR-09` Options evaluation and options GPU economics

Owned problem:
options split rigor, ranking utility, options-specific GPU chunking or cache
economics, and exactify-reduction math

Source packets:
`MR-16`

Primary ownership files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_types.rs`

Context files:

- `crates/app_core/src/options_data_store.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_parent_artifact.rs`

Own these symbols or decisions:

- options split design and leakage control
- options ranking utility and adequacy thresholds
- signal-manifest timing semantics
- fill-model and price-semantics compatibility
- options-specific GPU runtime cache usage and chunk planning
- device-side retained summary design

Open problem:

- characterize an options-evaluation functional on sparse, invalid, path-coupled
  candidate tuples that preserves ranking integrity while reducing exactify cost

Why this is hard here:

- options tuples are sparse, partially invalid, timing-sensitive, and dependent
  on fill-model and price-semantics choices
- options GPU economics are not the same as repo-wide runtime economics because
  candidate validity and chunk shape are part of the mathematical problem

Mathematical object:

- a tuple-evaluation map from sparse option-event sequences to ranked candidate
  outputs, together with a device-side retained-summary contract and an
  exactification operator

Notation seed:

- let `\omega \in \Omega` denote an options tuple candidate
- let `I(\omega) \in \{0,1\}` be the validity indicator under timing and
  fill-model rules
- let `U(\omega)` be the exact tuple utility used for finalist ranking
- let `S_{\mathrm{gpu}}(\omega)` be the device-side retained summary
- let `E(\omega)` be the exactification operator; any reduction in exactify
  volume must control
  `U(\omega^\star) - U(\widehat{\omega}^\star)`

Primary questions:

- what split and leakage-control rule is mathematically sound for the current
  event structure?
- what retained device summary is sufficient for stable ranking under sparse or
  partially invalid tuples?
- when can exactification volume be reduced without changing finalist order?

Required deliverables:

- formal tuple-evaluation and exactification model
- candidate sufficiency theorem for device-side summaries
- counterexamples for invalid timing or fill-model assumptions
- complexity and exactify-volume tradeoffs

Acceptance bar:

- every exactify-reduction claim must preserve final ranking exactly or provide
  an explicit finalist-regret bound

Verification hooks:

- feasibility and smoke harness parity
- compare-artifact parity
- timing/fill-model falsifiers

Non-goals:

- do not redesign generic runtime cache or execution economics; that belongs to
  `PR-08`
- do not redesign generic validation stack math; that belongs to `PR-06`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly review persisted versus inferred validation verdicts
- keep options-specific GPU economics separate from repo-wide runtime economics

## First-Pass Prompt

Treat `PR-09` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
## `PR-09` Options evaluation and options GPU economics

Owned problem:
options split rigor, ranking utility, options-specific GPU chunking or cache
economics, and exactify-reduction math

Source packets:
`MR-16`

Primary ownership files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_types.rs`

Context files:

- `crates/app_core/src/options_data_store.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_parent_artifact.rs`

Own these symbols or decisions:

- options split design and leakage control
- options ranking utility and adequacy thresholds
- signal-manifest timing semantics
- fill-model and price-semantics compatibility
- options-specific GPU runtime cache usage and chunk planning
- device-side retained summary design

Open problem:

- characterize an options-evaluation functional on sparse, invalid, path-coupled
  candidate tuples that preserves ranking integrity while reducing exactify cost

Why this is hard here:

- options tuples are sparse, partially invalid, timing-sensitive, and dependent
  on fill-model and price-semantics choices
- options GPU economics are not the same as repo-wide runtime economics because
  candidate validity and chunk shape are part of the mathematical problem

Mathematical object:

- a tuple-evaluation map from sparse option-event sequences to ranked candidate
  outputs, together with a device-side retained-summary contract and an
  exactification operator

Notation seed:

- let `\omega \in \Omega` denote an options tuple candidate
- let `I(\omega) \in \{0,1\}` be the validity indicator under timing and
  fill-model rules
- let `U(\omega)` be the exact tuple utility used for finalist ranking
- let `S_{\mathrm{gpu}}(\omega)` be the device-side retained summary
- let `E(\omega)` be the exactification operator; any reduction in exactify
  volume must control
  `U(\omega^\star) - U(\widehat{\omega}^\star)`

Primary questions:

- what split and leakage-control rule is mathematically sound for the current
  event structure?
- what retained device summary is sufficient for stable ranking under sparse or
  partially invalid tuples?
- when can exactification volume be reduced without changing finalist order?

Required deliverables:

- formal tuple-evaluation and exactification model
- candidate sufficiency theorem for device-side summaries
- counterexamples for invalid timing or fill-model assumptions
- complexity and exactify-volume tradeoffs

Acceptance bar:

- every exactify-reduction claim must preserve final ranking exactly or provide
  an explicit finalist-regret bound

Verification hooks:

- feasibility and smoke harness parity
- compare-artifact parity
- timing/fill-model falsifiers

Non-goals:

- do not redesign generic runtime cache or execution economics; that belongs to
  `PR-08`
- do not redesign generic validation stack math; that belongs to `PR-06`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly review persisted versus inferred validation verdicts
- keep options-specific GPU economics separate from repo-wide runtime economics
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

Attack the first-pass answer for `PR-09` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
## `PR-09` Options evaluation and options GPU economics

Owned problem:
options split rigor, ranking utility, options-specific GPU chunking or cache
economics, and exactify-reduction math

Source packets:
`MR-16`

Primary ownership files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_types.rs`

Context files:

- `crates/app_core/src/options_data_store.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_parent_artifact.rs`

Own these symbols or decisions:

- options split design and leakage control
- options ranking utility and adequacy thresholds
- signal-manifest timing semantics
- fill-model and price-semantics compatibility
- options-specific GPU runtime cache usage and chunk planning
- device-side retained summary design

Open problem:

- characterize an options-evaluation functional on sparse, invalid, path-coupled
  candidate tuples that preserves ranking integrity while reducing exactify cost

Why this is hard here:

- options tuples are sparse, partially invalid, timing-sensitive, and dependent
  on fill-model and price-semantics choices
- options GPU economics are not the same as repo-wide runtime economics because
  candidate validity and chunk shape are part of the mathematical problem

Mathematical object:

- a tuple-evaluation map from sparse option-event sequences to ranked candidate
  outputs, together with a device-side retained-summary contract and an
  exactification operator

Notation seed:

- let `\omega \in \Omega` denote an options tuple candidate
- let `I(\omega) \in \{0,1\}` be the validity indicator under timing and
  fill-model rules
- let `U(\omega)` be the exact tuple utility used for finalist ranking
- let `S_{\mathrm{gpu}}(\omega)` be the device-side retained summary
- let `E(\omega)` be the exactification operator; any reduction in exactify
  volume must control
  `U(\omega^\star) - U(\widehat{\omega}^\star)`

Primary questions:

- what split and leakage-control rule is mathematically sound for the current
  event structure?
- what retained device summary is sufficient for stable ranking under sparse or
  partially invalid tuples?
- when can exactification volume be reduced without changing finalist order?

Required deliverables:

- formal tuple-evaluation and exactification model
- candidate sufficiency theorem for device-side summaries
- counterexamples for invalid timing or fill-model assumptions
- complexity and exactify-volume tradeoffs

Acceptance bar:

- every exactify-reduction claim must preserve final ranking exactly or provide
  an explicit finalist-regret bound

Verification hooks:

- feasibility and smoke harness parity
- compare-artifact parity
- timing/fill-model falsifiers

Non-goals:

- do not redesign generic runtime cache or execution economics; that belongs to
  `PR-08`
- do not redesign generic validation stack math; that belongs to `PR-06`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly review persisted versus inferred validation verdicts
- keep options-specific GPU economics separate from repo-wide runtime economics
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

Produce the strongest implementation-ready mathematical result for `PR-09` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
## `PR-09` Options evaluation and options GPU economics

Owned problem:
options split rigor, ranking utility, options-specific GPU chunking or cache
economics, and exactify-reduction math

Source packets:
`MR-16`

Primary ownership files:

- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_types.rs`

Context files:

- `crates/app_core/src/options_data_store.rs`
- `src-tauri/src/commands_options_expression.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_parent_artifact.rs`

Own these symbols or decisions:

- options split design and leakage control
- options ranking utility and adequacy thresholds
- signal-manifest timing semantics
- fill-model and price-semantics compatibility
- options-specific GPU runtime cache usage and chunk planning
- device-side retained summary design

Open problem:

- characterize an options-evaluation functional on sparse, invalid, path-coupled
  candidate tuples that preserves ranking integrity while reducing exactify cost

Why this is hard here:

- options tuples are sparse, partially invalid, timing-sensitive, and dependent
  on fill-model and price-semantics choices
- options GPU economics are not the same as repo-wide runtime economics because
  candidate validity and chunk shape are part of the mathematical problem

Mathematical object:

- a tuple-evaluation map from sparse option-event sequences to ranked candidate
  outputs, together with a device-side retained-summary contract and an
  exactification operator

Notation seed:

- let `\omega \in \Omega` denote an options tuple candidate
- let `I(\omega) \in \{0,1\}` be the validity indicator under timing and
  fill-model rules
- let `U(\omega)` be the exact tuple utility used for finalist ranking
- let `S_{\mathrm{gpu}}(\omega)` be the device-side retained summary
- let `E(\omega)` be the exactification operator; any reduction in exactify
  volume must control
  `U(\omega^\star) - U(\widehat{\omega}^\star)`

Primary questions:

- what split and leakage-control rule is mathematically sound for the current
  event structure?
- what retained device summary is sufficient for stable ranking under sparse or
  partially invalid tuples?
- when can exactification volume be reduced without changing finalist order?

Required deliverables:

- formal tuple-evaluation and exactification model
- candidate sufficiency theorem for device-side summaries
- counterexamples for invalid timing or fill-model assumptions
- complexity and exactify-volume tradeoffs

Acceptance bar:

- every exactify-reduction claim must preserve final ranking exactly or provide
  an explicit finalist-regret bound

Verification hooks:

- feasibility and smoke harness parity
- compare-artifact parity
- timing/fill-model falsifiers

Non-goals:

- do not redesign generic runtime cache or execution economics; that belongs to
  `PR-08`
- do not redesign generic validation stack math; that belongs to `PR-06`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly review persisted versus inferred validation verdicts
- keep options-specific GPU economics separate from repo-wide runtime economics
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

- `crates/app_core/src/options_data_store.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/options_signal_manifest.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/bin/ta_options_expression_compare.rs`
- `src-tauri/src/bin/ta_options_expression_feasibility.rs`
- `src-tauri/src/bin/ta_options_expression_smoke.rs`
- `src-tauri/src/bin/ta_options_parent_artifact.rs`
- `src-tauri/src/commands_options_expression.rs`
