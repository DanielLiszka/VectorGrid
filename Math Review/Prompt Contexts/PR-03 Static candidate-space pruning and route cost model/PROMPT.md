# PR-03 Static candidate-space pruning and route cost model

- package type: `core`
- copied context files: `12`
- missing context files: `0`

## Prompt Scope

## `PR-03` Static candidate-space pruning and route cost model

Owned problem:
static Double-MA style pruning and route selection for the non-dynamic path

Source packets:
`MR-04`

Primary ownership files:

- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `src-tauri/src/commands_optimize.rs`

Context files:

- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`

Own these symbols or decisions:

- static route selection heuristics or cost models
- VDPS rung targeting and local-anchor thinning
- predictive prescan economics
- command-boundary route coercion for static optimization requests

Open problem:

- derive a sound pruning and route-selection rule for discrete candidate grids
  that minimizes compute subject to explicit regret constraints on missed
  finalists

Why this is hard here:

- the search is discrete, finite, and often exact-grid-shaped, but route choice
  and pruning quality depend on workload shape, metric contract, and replay
  cost
- heuristic pruning can silently trade compute savings for hidden selection
  regret

Mathematical object:

- a finite candidate lattice with a cost functional, a true final-score
  functional, and a pruning/route policy that decides which subsets receive
  exact evaluation

Notation seed:

- let `G = A_1 \times \cdots \times A_d` be the finite candidate lattice
- let `J(g)` be the exact final score of candidate `g \in G`
- let `\kappa(g, r)` be the compute cost of evaluating `g` under route `r`
- let a pruning/route policy `\pi` produce a retained subset
  `H_\pi \subseteq G` and route map `r_\pi : H_\pi \to \mathcal{R}`
- define finalist regret by
  `\Delta_\pi = \max_{g \in \arg\max J} \left(J(g) - \max_{h \in H_\pi} J(h)\right)`

Primary questions:

- what pruning rule is sound under the current workload model?
- can a route policy be stated as an optimization problem over compute cost and
  finalist-regret probability?
- what sufficient conditions guarantee that a thinned candidate set still
  contains the true finalist set?

Required deliverables:

- formal pruning and route-selection model
- theorem candidates for safe pruning or bounded-regret pruning
- adversarial grids where intuitive heuristics fail
- compute/regret tradeoff characterization

Acceptance bar:

- any pruning claim must prove soundness or state an explicit regret contract

Verification hooks:

- exact-vs-pruned finalist overlap
- route-regret harness evidence
- counterexample workloads

Non-goals:

- do not redesign dynamic-GPU execution selection or residency; that belongs to
  `PR-04`
- do not redesign composite search or `composite_mo.rs`; that belongs to
  `PR-05`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat the bin harnesses as empirical route-regret evidence surfaces
- keep static-route economics separate from dynamic execution economics

## First-Pass Prompt

Treat `PR-03` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
## `PR-03` Static candidate-space pruning and route cost model

Owned problem:
static Double-MA style pruning and route selection for the non-dynamic path

Source packets:
`MR-04`

Primary ownership files:

- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `src-tauri/src/commands_optimize.rs`

Context files:

- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`

Own these symbols or decisions:

- static route selection heuristics or cost models
- VDPS rung targeting and local-anchor thinning
- predictive prescan economics
- command-boundary route coercion for static optimization requests

Open problem:

- derive a sound pruning and route-selection rule for discrete candidate grids
  that minimizes compute subject to explicit regret constraints on missed
  finalists

Why this is hard here:

- the search is discrete, finite, and often exact-grid-shaped, but route choice
  and pruning quality depend on workload shape, metric contract, and replay
  cost
- heuristic pruning can silently trade compute savings for hidden selection
  regret

Mathematical object:

- a finite candidate lattice with a cost functional, a true final-score
  functional, and a pruning/route policy that decides which subsets receive
  exact evaluation

Notation seed:

- let `G = A_1 \times \cdots \times A_d` be the finite candidate lattice
- let `J(g)` be the exact final score of candidate `g \in G`
- let `\kappa(g, r)` be the compute cost of evaluating `g` under route `r`
- let a pruning/route policy `\pi` produce a retained subset
  `H_\pi \subseteq G` and route map `r_\pi : H_\pi \to \mathcal{R}`
- define finalist regret by
  `\Delta_\pi = \max_{g \in \arg\max J} \left(J(g) - \max_{h \in H_\pi} J(h)\right)`

Primary questions:

- what pruning rule is sound under the current workload model?
- can a route policy be stated as an optimization problem over compute cost and
  finalist-regret probability?
- what sufficient conditions guarantee that a thinned candidate set still
  contains the true finalist set?

Required deliverables:

- formal pruning and route-selection model
- theorem candidates for safe pruning or bounded-regret pruning
- adversarial grids where intuitive heuristics fail
- compute/regret tradeoff characterization

Acceptance bar:

- any pruning claim must prove soundness or state an explicit regret contract

Verification hooks:

- exact-vs-pruned finalist overlap
- route-regret harness evidence
- counterexample workloads

Non-goals:

- do not redesign dynamic-GPU execution selection or residency; that belongs to
  `PR-04`
- do not redesign composite search or `composite_mo.rs`; that belongs to
  `PR-05`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat the bin harnesses as empirical route-regret evidence surfaces
- keep static-route economics separate from dynamic execution economics
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

Attack the first-pass answer for `PR-03` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
## `PR-03` Static candidate-space pruning and route cost model

Owned problem:
static Double-MA style pruning and route selection for the non-dynamic path

Source packets:
`MR-04`

Primary ownership files:

- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `src-tauri/src/commands_optimize.rs`

Context files:

- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`

Own these symbols or decisions:

- static route selection heuristics or cost models
- VDPS rung targeting and local-anchor thinning
- predictive prescan economics
- command-boundary route coercion for static optimization requests

Open problem:

- derive a sound pruning and route-selection rule for discrete candidate grids
  that minimizes compute subject to explicit regret constraints on missed
  finalists

Why this is hard here:

- the search is discrete, finite, and often exact-grid-shaped, but route choice
  and pruning quality depend on workload shape, metric contract, and replay
  cost
- heuristic pruning can silently trade compute savings for hidden selection
  regret

Mathematical object:

- a finite candidate lattice with a cost functional, a true final-score
  functional, and a pruning/route policy that decides which subsets receive
  exact evaluation

Notation seed:

- let `G = A_1 \times \cdots \times A_d` be the finite candidate lattice
- let `J(g)` be the exact final score of candidate `g \in G`
- let `\kappa(g, r)` be the compute cost of evaluating `g` under route `r`
- let a pruning/route policy `\pi` produce a retained subset
  `H_\pi \subseteq G` and route map `r_\pi : H_\pi \to \mathcal{R}`
- define finalist regret by
  `\Delta_\pi = \max_{g \in \arg\max J} \left(J(g) - \max_{h \in H_\pi} J(h)\right)`

Primary questions:

- what pruning rule is sound under the current workload model?
- can a route policy be stated as an optimization problem over compute cost and
  finalist-regret probability?
- what sufficient conditions guarantee that a thinned candidate set still
  contains the true finalist set?

Required deliverables:

- formal pruning and route-selection model
- theorem candidates for safe pruning or bounded-regret pruning
- adversarial grids where intuitive heuristics fail
- compute/regret tradeoff characterization

Acceptance bar:

- any pruning claim must prove soundness or state an explicit regret contract

Verification hooks:

- exact-vs-pruned finalist overlap
- route-regret harness evidence
- counterexample workloads

Non-goals:

- do not redesign dynamic-GPU execution selection or residency; that belongs to
  `PR-04`
- do not redesign composite search or `composite_mo.rs`; that belongs to
  `PR-05`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat the bin harnesses as empirical route-regret evidence surfaces
- keep static-route economics separate from dynamic execution economics
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

Produce the strongest implementation-ready mathematical result for `PR-03` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
## `PR-03` Static candidate-space pruning and route cost model

Owned problem:
static Double-MA style pruning and route selection for the non-dynamic path

Source packets:
`MR-04`

Primary ownership files:

- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `src-tauri/src/commands_optimize.rs`

Context files:

- `crates/app_core/src/strategy_plan.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`

Own these symbols or decisions:

- static route selection heuristics or cost models
- VDPS rung targeting and local-anchor thinning
- predictive prescan economics
- command-boundary route coercion for static optimization requests

Open problem:

- derive a sound pruning and route-selection rule for discrete candidate grids
  that minimizes compute subject to explicit regret constraints on missed
  finalists

Why this is hard here:

- the search is discrete, finite, and often exact-grid-shaped, but route choice
  and pruning quality depend on workload shape, metric contract, and replay
  cost
- heuristic pruning can silently trade compute savings for hidden selection
  regret

Mathematical object:

- a finite candidate lattice with a cost functional, a true final-score
  functional, and a pruning/route policy that decides which subsets receive
  exact evaluation

Notation seed:

- let `G = A_1 \times \cdots \times A_d` be the finite candidate lattice
- let `J(g)` be the exact final score of candidate `g \in G`
- let `\kappa(g, r)` be the compute cost of evaluating `g` under route `r`
- let a pruning/route policy `\pi` produce a retained subset
  `H_\pi \subseteq G` and route map `r_\pi : H_\pi \to \mathcal{R}`
- define finalist regret by
  `\Delta_\pi = \max_{g \in \arg\max J} \left(J(g) - \max_{h \in H_\pi} J(h)\right)`

Primary questions:

- what pruning rule is sound under the current workload model?
- can a route policy be stated as an optimization problem over compute cost and
  finalist-regret probability?
- what sufficient conditions guarantee that a thinned candidate set still
  contains the true finalist set?

Required deliverables:

- formal pruning and route-selection model
- theorem candidates for safe pruning or bounded-regret pruning
- adversarial grids where intuitive heuristics fail
- compute/regret tradeoff characterization

Acceptance bar:

- any pruning claim must prove soundness or state an explicit regret contract

Verification hooks:

- exact-vs-pruned finalist overlap
- route-regret harness evidence
- counterexample workloads

Non-goals:

- do not redesign dynamic-GPU execution selection or residency; that belongs to
  `PR-04`
- do not redesign composite search or `composite_mo.rs`; that belongs to
  `PR-05`
- do not redesign runtime cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat the bin harnesses as empirical route-regret evidence surfaces
- keep static-route economics separate from dynamic execution economics
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

- `crates/app_core/src/bin/real_data_fidelity_audit.rs`
- `crates/app_core/src/bin/real_data_optimizer_eval.rs`
- `crates/app_core/src/bin/stage6b_kernel_report.rs`
- `crates/app_core/src/double_ma_backend.rs`
- `crates/app_core/src/double_ma_mode.rs`
- `crates/app_core/src/double_ma_vdps.rs`
- `crates/app_core/src/ma_policy.rs`
- `crates/app_core/src/prescan_predictive.rs`
- `crates/app_core/src/strategy_plan.rs`
- `src-tauri/src/bin/ta_app_path_faceoff.rs`
- `src-tauri/src/bin/ta_profile_route_matrix.rs`
- `src-tauri/src/commands_optimize.rs`
