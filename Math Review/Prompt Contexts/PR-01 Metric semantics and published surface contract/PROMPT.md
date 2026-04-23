# PR-01 Metric semantics and published surface contract

- package type: `core`
- copied context files: `9`
- missing context files: `0`

## Prompt Scope

## `PR-01` Metric semantics and published surface contract

Owned problem:
the meaning of published metric fields across GPU contracts, CPU finalizers, and
runtime metric-surface publication

Source packets:
`MR-01`, `MR-07`, `MR-14`

Primary ownership files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/run_artifacts.rs`

Context files:

- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/asr.rs`

Own these symbols or decisions:

- `legacy`, `core17`, and `trade22` meaning
- exact-tail versus placeholder field semantics
- GPU row-width or bundle meaning as a published contract
- result-surface resolution and projection semantics
- trade/risk field meaning that is visible to downstream consumers

Open problem:

- characterize the minimal sufficient state needed to publish every current
  metric surface exactly, and prove which published fields are mathematically
  derivable versus irreducibly primitive under the repo's path-dependent
  semantics

Why this is hard here:

- GPU row contracts, CPU finalizers, and runtime publication all expose the
  same public semantics from different execution paths
- exact trade and path-dependent tail meaning competes with tight retained-state
  budgets and artifact compatibility

Mathematical object:

- a discrete path-to-surface emission problem with exact publication
  requirements across multiple metric surfaces

Notation seed:

- let `\mathcal{P}_T` denote the set of admissible discrete backtest paths
  `p = ((z_t, a_t, \delta_t))_{t=1}^T`, where `z_t` is the market/bar state,
  `a_t` is the strategy action/position state, and `\delta_t` is any
  event-level trade or fill update needed by the current product semantics
- for each published surface `s \in \{\mathrm{legacy}, \mathrm{core17},
  \mathrm{trade22}\}`, let
  `F_s : \mathcal{P}_T \to \mathbb{R}^{d_s}` be the exact emitted metric row
- seek a retained-state map
  `S : \mathcal{P}_T \to \mathbb{R}^k \times \mathbb{Z}^m`
  and decoders `G_s` such that
  `F_s(p) = G_s(S(p))` for all `p \in \mathcal{P}_T`
- define semantic equivalence on paths by
  `p \sim q \iff \forall s,\; F_s(p) = F_s(q)`;
  an exact retained state must separate the quotient classes induced by `\sim`
- a published field `F_{s,j}` is derivable from the retained state iff there
  exists `g_{s,j}` such that
  `F_{s,j}(p) = g_{s,j}(S(p))` for all `p`; otherwise it is primitive under the
  admitted state class

Primary questions:

- does a finite sufficient-statistic basis exist for exact `legacy`, `core17`,
  and `trade22` publication?
- which currently published fields are exact derivations and which force
  path-dependent retained state?
- when do two different retained-state bundles induce identical published
  surfaces?

Required deliverables:

- formal state/output model
- candidate minimal-sufficiency theorem
- one counterexample family showing where compression fails
- explicit lower bound or identifiability argument for the minimal retained
  state width `(k, m)`
- exactness criterion of the form `F_s(p) = \hat F_s(p)` for all `s, p`, or an
  explicit approximation contract such as
  `\lVert F_s(p) - \hat F_s(p) \rVert_\infty \le \varepsilon_s`
  together with any rank-preservation requirement

Acceptance bar:

- separate exact equivalence results from heuristic compression ideas
- any proposed compression must either preserve public semantics exactly or
  attach an explicit approximation contract with a stated norm, bias, or
  rank-instability bound

Verification hooks:

- artifact parity on `legacy`, `core17`, and `trade22`, meaning either
  `F_s(p) = \hat F_s(p)` exactly for every tested path or satisfaction of the
  stated approximation contract
- proof or falsifier for every claimed derivation

Non-goals:

- do not redesign ranking or utility formulas that consume these metrics; those
  belong to `PR-02`, `PR-05`, `PR-06`, or `PR-07`
- do not redesign cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- preserve or explicitly redefine public metric semantics instead of silently
  drifting them
- identify primitive versus derivable published fields
- keep artifact-surface compatibility visible as a first-class constraint
- use the symbols `\mathcal{P}_T`, `F_s`, `S`, `G_s`, and `\sim` explicitly in
  the answer rather than replacing them with prose only

## First-Pass Prompt

Treat `PR-01` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
## `PR-01` Metric semantics and published surface contract

Owned problem:
the meaning of published metric fields across GPU contracts, CPU finalizers, and
runtime metric-surface publication

Source packets:
`MR-01`, `MR-07`, `MR-14`

Primary ownership files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/run_artifacts.rs`

Context files:

- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/asr.rs`

Own these symbols or decisions:

- `legacy`, `core17`, and `trade22` meaning
- exact-tail versus placeholder field semantics
- GPU row-width or bundle meaning as a published contract
- result-surface resolution and projection semantics
- trade/risk field meaning that is visible to downstream consumers

Open problem:

- characterize the minimal sufficient state needed to publish every current
  metric surface exactly, and prove which published fields are mathematically
  derivable versus irreducibly primitive under the repo's path-dependent
  semantics

Why this is hard here:

- GPU row contracts, CPU finalizers, and runtime publication all expose the
  same public semantics from different execution paths
- exact trade and path-dependent tail meaning competes with tight retained-state
  budgets and artifact compatibility

Mathematical object:

- a discrete path-to-surface emission problem with exact publication
  requirements across multiple metric surfaces

Notation seed:

- let `\mathcal{P}_T` denote the set of admissible discrete backtest paths
  `p = ((z_t, a_t, \delta_t))_{t=1}^T`, where `z_t` is the market/bar state,
  `a_t` is the strategy action/position state, and `\delta_t` is any
  event-level trade or fill update needed by the current product semantics
- for each published surface `s \in \{\mathrm{legacy}, \mathrm{core17},
  \mathrm{trade22}\}`, let
  `F_s : \mathcal{P}_T \to \mathbb{R}^{d_s}` be the exact emitted metric row
- seek a retained-state map
  `S : \mathcal{P}_T \to \mathbb{R}^k \times \mathbb{Z}^m`
  and decoders `G_s` such that
  `F_s(p) = G_s(S(p))` for all `p \in \mathcal{P}_T`
- define semantic equivalence on paths by
  `p \sim q \iff \forall s,\; F_s(p) = F_s(q)`;
  an exact retained state must separate the quotient classes induced by `\sim`
- a published field `F_{s,j}` is derivable from the retained state iff there
  exists `g_{s,j}` such that
  `F_{s,j}(p) = g_{s,j}(S(p))` for all `p`; otherwise it is primitive under the
  admitted state class

Primary questions:

- does a finite sufficient-statistic basis exist for exact `legacy`, `core17`,
  and `trade22` publication?
- which currently published fields are exact derivations and which force
  path-dependent retained state?
- when do two different retained-state bundles induce identical published
  surfaces?

Required deliverables:

- formal state/output model
- candidate minimal-sufficiency theorem
- one counterexample family showing where compression fails
- explicit lower bound or identifiability argument for the minimal retained
  state width `(k, m)`
- exactness criterion of the form `F_s(p) = \hat F_s(p)` for all `s, p`, or an
  explicit approximation contract such as
  `\lVert F_s(p) - \hat F_s(p) \rVert_\infty \le \varepsilon_s`
  together with any rank-preservation requirement

Acceptance bar:

- separate exact equivalence results from heuristic compression ideas
- any proposed compression must either preserve public semantics exactly or
  attach an explicit approximation contract with a stated norm, bias, or
  rank-instability bound

Verification hooks:

- artifact parity on `legacy`, `core17`, and `trade22`, meaning either
  `F_s(p) = \hat F_s(p)` exactly for every tested path or satisfaction of the
  stated approximation contract
- proof or falsifier for every claimed derivation

Non-goals:

- do not redesign ranking or utility formulas that consume these metrics; those
  belong to `PR-02`, `PR-05`, `PR-06`, or `PR-07`
- do not redesign cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- preserve or explicitly redefine public metric semantics instead of silently
  drifting them
- identify primitive versus derivable published fields
- keep artifact-surface compatibility visible as a first-class constraint
- use the symbols `\mathcal{P}_T`, `F_s`, `S`, `G_s`, and `\sim` explicitly in
  the answer rather than replacing them with prose only
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

Attack the first-pass answer for `PR-01` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
## `PR-01` Metric semantics and published surface contract

Owned problem:
the meaning of published metric fields across GPU contracts, CPU finalizers, and
runtime metric-surface publication

Source packets:
`MR-01`, `MR-07`, `MR-14`

Primary ownership files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/run_artifacts.rs`

Context files:

- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/asr.rs`

Own these symbols or decisions:

- `legacy`, `core17`, and `trade22` meaning
- exact-tail versus placeholder field semantics
- GPU row-width or bundle meaning as a published contract
- result-surface resolution and projection semantics
- trade/risk field meaning that is visible to downstream consumers

Open problem:

- characterize the minimal sufficient state needed to publish every current
  metric surface exactly, and prove which published fields are mathematically
  derivable versus irreducibly primitive under the repo's path-dependent
  semantics

Why this is hard here:

- GPU row contracts, CPU finalizers, and runtime publication all expose the
  same public semantics from different execution paths
- exact trade and path-dependent tail meaning competes with tight retained-state
  budgets and artifact compatibility

Mathematical object:

- a discrete path-to-surface emission problem with exact publication
  requirements across multiple metric surfaces

Notation seed:

- let `\mathcal{P}_T` denote the set of admissible discrete backtest paths
  `p = ((z_t, a_t, \delta_t))_{t=1}^T`, where `z_t` is the market/bar state,
  `a_t` is the strategy action/position state, and `\delta_t` is any
  event-level trade or fill update needed by the current product semantics
- for each published surface `s \in \{\mathrm{legacy}, \mathrm{core17},
  \mathrm{trade22}\}`, let
  `F_s : \mathcal{P}_T \to \mathbb{R}^{d_s}` be the exact emitted metric row
- seek a retained-state map
  `S : \mathcal{P}_T \to \mathbb{R}^k \times \mathbb{Z}^m`
  and decoders `G_s` such that
  `F_s(p) = G_s(S(p))` for all `p \in \mathcal{P}_T`
- define semantic equivalence on paths by
  `p \sim q \iff \forall s,\; F_s(p) = F_s(q)`;
  an exact retained state must separate the quotient classes induced by `\sim`
- a published field `F_{s,j}` is derivable from the retained state iff there
  exists `g_{s,j}` such that
  `F_{s,j}(p) = g_{s,j}(S(p))` for all `p`; otherwise it is primitive under the
  admitted state class

Primary questions:

- does a finite sufficient-statistic basis exist for exact `legacy`, `core17`,
  and `trade22` publication?
- which currently published fields are exact derivations and which force
  path-dependent retained state?
- when do two different retained-state bundles induce identical published
  surfaces?

Required deliverables:

- formal state/output model
- candidate minimal-sufficiency theorem
- one counterexample family showing where compression fails
- explicit lower bound or identifiability argument for the minimal retained
  state width `(k, m)`
- exactness criterion of the form `F_s(p) = \hat F_s(p)` for all `s, p`, or an
  explicit approximation contract such as
  `\lVert F_s(p) - \hat F_s(p) \rVert_\infty \le \varepsilon_s`
  together with any rank-preservation requirement

Acceptance bar:

- separate exact equivalence results from heuristic compression ideas
- any proposed compression must either preserve public semantics exactly or
  attach an explicit approximation contract with a stated norm, bias, or
  rank-instability bound

Verification hooks:

- artifact parity on `legacy`, `core17`, and `trade22`, meaning either
  `F_s(p) = \hat F_s(p)` exactly for every tested path or satisfaction of the
  stated approximation contract
- proof or falsifier for every claimed derivation

Non-goals:

- do not redesign ranking or utility formulas that consume these metrics; those
  belong to `PR-02`, `PR-05`, `PR-06`, or `PR-07`
- do not redesign cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- preserve or explicitly redefine public metric semantics instead of silently
  drifting them
- identify primitive versus derivable published fields
- keep artifact-surface compatibility visible as a first-class constraint
- use the symbols `\mathcal{P}_T`, `F_s`, `S`, `G_s`, and `\sim` explicitly in
  the answer rather than replacing them with prose only
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

Produce the strongest implementation-ready mathematical result for `PR-01` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
## `PR-01` Metric semantics and published surface contract

Owned problem:
the meaning of published metric fields across GPU contracts, CPU finalizers, and
runtime metric-surface publication

Source packets:
`MR-01`, `MR-07`, `MR-14`

Primary ownership files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/run_artifacts.rs`

Context files:

- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/asr.rs`

Own these symbols or decisions:

- `legacy`, `core17`, and `trade22` meaning
- exact-tail versus placeholder field semantics
- GPU row-width or bundle meaning as a published contract
- result-surface resolution and projection semantics
- trade/risk field meaning that is visible to downstream consumers

Open problem:

- characterize the minimal sufficient state needed to publish every current
  metric surface exactly, and prove which published fields are mathematically
  derivable versus irreducibly primitive under the repo's path-dependent
  semantics

Why this is hard here:

- GPU row contracts, CPU finalizers, and runtime publication all expose the
  same public semantics from different execution paths
- exact trade and path-dependent tail meaning competes with tight retained-state
  budgets and artifact compatibility

Mathematical object:

- a discrete path-to-surface emission problem with exact publication
  requirements across multiple metric surfaces

Notation seed:

- let `\mathcal{P}_T` denote the set of admissible discrete backtest paths
  `p = ((z_t, a_t, \delta_t))_{t=1}^T`, where `z_t` is the market/bar state,
  `a_t` is the strategy action/position state, and `\delta_t` is any
  event-level trade or fill update needed by the current product semantics
- for each published surface `s \in \{\mathrm{legacy}, \mathrm{core17},
  \mathrm{trade22}\}`, let
  `F_s : \mathcal{P}_T \to \mathbb{R}^{d_s}` be the exact emitted metric row
- seek a retained-state map
  `S : \mathcal{P}_T \to \mathbb{R}^k \times \mathbb{Z}^m`
  and decoders `G_s` such that
  `F_s(p) = G_s(S(p))` for all `p \in \mathcal{P}_T`
- define semantic equivalence on paths by
  `p \sim q \iff \forall s,\; F_s(p) = F_s(q)`;
  an exact retained state must separate the quotient classes induced by `\sim`
- a published field `F_{s,j}` is derivable from the retained state iff there
  exists `g_{s,j}` such that
  `F_{s,j}(p) = g_{s,j}(S(p))` for all `p`; otherwise it is primitive under the
  admitted state class

Primary questions:

- does a finite sufficient-statistic basis exist for exact `legacy`, `core17`,
  and `trade22` publication?
- which currently published fields are exact derivations and which force
  path-dependent retained state?
- when do two different retained-state bundles induce identical published
  surfaces?

Required deliverables:

- formal state/output model
- candidate minimal-sufficiency theorem
- one counterexample family showing where compression fails
- explicit lower bound or identifiability argument for the minimal retained
  state width `(k, m)`
- exactness criterion of the form `F_s(p) = \hat F_s(p)` for all `s, p`, or an
  explicit approximation contract such as
  `\lVert F_s(p) - \hat F_s(p) \rVert_\infty \le \varepsilon_s`
  together with any rank-preservation requirement

Acceptance bar:

- separate exact equivalence results from heuristic compression ideas
- any proposed compression must either preserve public semantics exactly or
  attach an explicit approximation contract with a stated norm, bias, or
  rank-instability bound

Verification hooks:

- artifact parity on `legacy`, `core17`, and `trade22`, meaning either
  `F_s(p) = \hat F_s(p)` exactly for every tested path or satisfaction of the
  stated approximation contract
- proof or falsifier for every claimed derivation

Non-goals:

- do not redesign ranking or utility formulas that consume these metrics; those
  belong to `PR-02`, `PR-05`, `PR-06`, or `PR-07`
- do not redesign cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- preserve or explicitly redefine public metric semantics instead of silently
  drifting them
- identify primitive versus derivable published fields
- keep artifact-surface compatibility visible as a first-class constraint
- use the symbols `\mathcal{P}_T`, `F_s`, `S`, `G_s`, and `\sim` explicitly in
  the answer rather than replacing them with prose only
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

- `crates/app_core/src/asr.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/run_artifacts.rs`
