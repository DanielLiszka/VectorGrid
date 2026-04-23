# PR-05 Composite search and utility math

- package type: `core`
- copied context files: `7`
- missing context files: `0`

## Prompt Scope

## `PR-05` Composite search and utility math

Owned problem:
composite utility, composite prescan, exact-grid or candidate-set search math,
and optimizer-facing heatmap semantics

Source packets:
`MR-15`, `MR-04`

Primary ownership files:

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`
- `crates/strategies/src/composite_cpu/mod.rs`

Context files:

- `none`

Own these symbols or decisions:

- composite objective scoring and scalarization
- exact-grid versus candidate-set search behavior
- composite prescan and seed-pair generation
- heatmap summary semantics
- optimizer search telemetry invariants

Open problem:

- characterize a composite utility and search geometry on discrete candidate
  lattices that preserves rank stability while supporting exact-grid and
  candidate-set search modes

Why this is hard here:

- composite search mixes scalarization, Pareto structure, prescan, and telemetry
  on finite irregular grids rather than smooth objective manifolds
- rank quality, heatmap semantics, and search cost are coupled

Mathematical object:

- a discrete multi-objective lattice with a scalarized ranking functional, a
  Pareto geometry, and a telemetry map from sampled search paths to published
  search summaries

Notation seed:

- let `G` be the discrete candidate lattice and `M(g) \in \mathbb{R}^q` the
  objective vector for candidate `g`
- let `\sigma : \mathbb{R}^q \to \mathbb{R}` be the scalarization used for
  winner selection
- let `P^\star = \{g \in G : \nexists h \in G \text{ with } M(h) \succ M(g)\}`
  be the exact Pareto set
- let `\widehat{P}` and `\widehat{g}^\star` denote the sampled/search-limited
  Pareto set and winner; any non-exact method must bound
  `\sigma(g^\star) - \sigma(\widehat{g}^\star)`

Primary questions:

- what scalarization preserves stable ordering under the current composite
  metric family?
- when can prescan or candidate-set search preserve the exact-grid winner set?
- what heatmap summary is mathematically faithful on sparse or unevenly sampled
  candidate regions?

Required deliverables:

- formal scalarization and search model
- theorem candidates for exact-grid/candidate-set equivalence or bounded regret
- counterexamples for unstable scalarizations
- complexity and telemetry consequences

Acceptance bar:

- every sampled-search improvement must state how rank error or finalist regret
  is controlled

Verification hooks:

- exact-grid versus candidate-set overlap
- scalarization stability checks
- heatmap falsifiers on sparse candidate regions

Non-goals:

- do not redefine underlying metric field meaning; that belongs to `PR-01`
- do not redesign static Double-MA route selection; that belongs to `PR-03`

Files to provide as context:

- all primary ownership files

Prompt-specific addenda:

- explicitly cover `CompositeGpuPrescanConfig` and predictive seed-pair logic
- keep composite utility math separate from metric field semantics

## First-Pass Prompt

Treat `PR-05` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
## `PR-05` Composite search and utility math

Owned problem:
composite utility, composite prescan, exact-grid or candidate-set search math,
and optimizer-facing heatmap semantics

Source packets:
`MR-15`, `MR-04`

Primary ownership files:

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`
- `crates/strategies/src/composite_cpu/mod.rs`

Context files:

- `none`

Own these symbols or decisions:

- composite objective scoring and scalarization
- exact-grid versus candidate-set search behavior
- composite prescan and seed-pair generation
- heatmap summary semantics
- optimizer search telemetry invariants

Open problem:

- characterize a composite utility and search geometry on discrete candidate
  lattices that preserves rank stability while supporting exact-grid and
  candidate-set search modes

Why this is hard here:

- composite search mixes scalarization, Pareto structure, prescan, and telemetry
  on finite irregular grids rather than smooth objective manifolds
- rank quality, heatmap semantics, and search cost are coupled

Mathematical object:

- a discrete multi-objective lattice with a scalarized ranking functional, a
  Pareto geometry, and a telemetry map from sampled search paths to published
  search summaries

Notation seed:

- let `G` be the discrete candidate lattice and `M(g) \in \mathbb{R}^q` the
  objective vector for candidate `g`
- let `\sigma : \mathbb{R}^q \to \mathbb{R}` be the scalarization used for
  winner selection
- let `P^\star = \{g \in G : \nexists h \in G \text{ with } M(h) \succ M(g)\}`
  be the exact Pareto set
- let `\widehat{P}` and `\widehat{g}^\star` denote the sampled/search-limited
  Pareto set and winner; any non-exact method must bound
  `\sigma(g^\star) - \sigma(\widehat{g}^\star)`

Primary questions:

- what scalarization preserves stable ordering under the current composite
  metric family?
- when can prescan or candidate-set search preserve the exact-grid winner set?
- what heatmap summary is mathematically faithful on sparse or unevenly sampled
  candidate regions?

Required deliverables:

- formal scalarization and search model
- theorem candidates for exact-grid/candidate-set equivalence or bounded regret
- counterexamples for unstable scalarizations
- complexity and telemetry consequences

Acceptance bar:

- every sampled-search improvement must state how rank error or finalist regret
  is controlled

Verification hooks:

- exact-grid versus candidate-set overlap
- scalarization stability checks
- heatmap falsifiers on sparse candidate regions

Non-goals:

- do not redefine underlying metric field meaning; that belongs to `PR-01`
- do not redesign static Double-MA route selection; that belongs to `PR-03`

Files to provide as context:

- all primary ownership files

Prompt-specific addenda:

- explicitly cover `CompositeGpuPrescanConfig` and predictive seed-pair logic
- keep composite utility math separate from metric field semantics
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

Attack the first-pass answer for `PR-05` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
## `PR-05` Composite search and utility math

Owned problem:
composite utility, composite prescan, exact-grid or candidate-set search math,
and optimizer-facing heatmap semantics

Source packets:
`MR-15`, `MR-04`

Primary ownership files:

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`
- `crates/strategies/src/composite_cpu/mod.rs`

Context files:

- `none`

Own these symbols or decisions:

- composite objective scoring and scalarization
- exact-grid versus candidate-set search behavior
- composite prescan and seed-pair generation
- heatmap summary semantics
- optimizer search telemetry invariants

Open problem:

- characterize a composite utility and search geometry on discrete candidate
  lattices that preserves rank stability while supporting exact-grid and
  candidate-set search modes

Why this is hard here:

- composite search mixes scalarization, Pareto structure, prescan, and telemetry
  on finite irregular grids rather than smooth objective manifolds
- rank quality, heatmap semantics, and search cost are coupled

Mathematical object:

- a discrete multi-objective lattice with a scalarized ranking functional, a
  Pareto geometry, and a telemetry map from sampled search paths to published
  search summaries

Notation seed:

- let `G` be the discrete candidate lattice and `M(g) \in \mathbb{R}^q` the
  objective vector for candidate `g`
- let `\sigma : \mathbb{R}^q \to \mathbb{R}` be the scalarization used for
  winner selection
- let `P^\star = \{g \in G : \nexists h \in G \text{ with } M(h) \succ M(g)\}`
  be the exact Pareto set
- let `\widehat{P}` and `\widehat{g}^\star` denote the sampled/search-limited
  Pareto set and winner; any non-exact method must bound
  `\sigma(g^\star) - \sigma(\widehat{g}^\star)`

Primary questions:

- what scalarization preserves stable ordering under the current composite
  metric family?
- when can prescan or candidate-set search preserve the exact-grid winner set?
- what heatmap summary is mathematically faithful on sparse or unevenly sampled
  candidate regions?

Required deliverables:

- formal scalarization and search model
- theorem candidates for exact-grid/candidate-set equivalence or bounded regret
- counterexamples for unstable scalarizations
- complexity and telemetry consequences

Acceptance bar:

- every sampled-search improvement must state how rank error or finalist regret
  is controlled

Verification hooks:

- exact-grid versus candidate-set overlap
- scalarization stability checks
- heatmap falsifiers on sparse candidate regions

Non-goals:

- do not redefine underlying metric field meaning; that belongs to `PR-01`
- do not redesign static Double-MA route selection; that belongs to `PR-03`

Files to provide as context:

- all primary ownership files

Prompt-specific addenda:

- explicitly cover `CompositeGpuPrescanConfig` and predictive seed-pair logic
- keep composite utility math separate from metric field semantics
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

Produce the strongest implementation-ready mathematical result for `PR-05` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
## `PR-05` Composite search and utility math

Owned problem:
composite utility, composite prescan, exact-grid or candidate-set search math,
and optimizer-facing heatmap semantics

Source packets:
`MR-15`, `MR-04`

Primary ownership files:

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/stream.rs`
- `crates/optimizer/src/mo.rs`
- `crates/strategies/src/composite_cpu/mod.rs`

Context files:

- `none`

Own these symbols or decisions:

- composite objective scoring and scalarization
- exact-grid versus candidate-set search behavior
- composite prescan and seed-pair generation
- heatmap summary semantics
- optimizer search telemetry invariants

Open problem:

- characterize a composite utility and search geometry on discrete candidate
  lattices that preserves rank stability while supporting exact-grid and
  candidate-set search modes

Why this is hard here:

- composite search mixes scalarization, Pareto structure, prescan, and telemetry
  on finite irregular grids rather than smooth objective manifolds
- rank quality, heatmap semantics, and search cost are coupled

Mathematical object:

- a discrete multi-objective lattice with a scalarized ranking functional, a
  Pareto geometry, and a telemetry map from sampled search paths to published
  search summaries

Notation seed:

- let `G` be the discrete candidate lattice and `M(g) \in \mathbb{R}^q` the
  objective vector for candidate `g`
- let `\sigma : \mathbb{R}^q \to \mathbb{R}` be the scalarization used for
  winner selection
- let `P^\star = \{g \in G : \nexists h \in G \text{ with } M(h) \succ M(g)\}`
  be the exact Pareto set
- let `\widehat{P}` and `\widehat{g}^\star` denote the sampled/search-limited
  Pareto set and winner; any non-exact method must bound
  `\sigma(g^\star) - \sigma(\widehat{g}^\star)`

Primary questions:

- what scalarization preserves stable ordering under the current composite
  metric family?
- when can prescan or candidate-set search preserve the exact-grid winner set?
- what heatmap summary is mathematically faithful on sparse or unevenly sampled
  candidate regions?

Required deliverables:

- formal scalarization and search model
- theorem candidates for exact-grid/candidate-set equivalence or bounded regret
- counterexamples for unstable scalarizations
- complexity and telemetry consequences

Acceptance bar:

- every sampled-search improvement must state how rank error or finalist regret
  is controlled

Verification hooks:

- exact-grid versus candidate-set overlap
- scalarization stability checks
- heatmap falsifiers on sparse candidate regions

Non-goals:

- do not redefine underlying metric field meaning; that belongs to `PR-01`
- do not redesign static Double-MA route selection; that belongs to `PR-03`

Files to provide as context:

- all primary ownership files

Prompt-specific addenda:

- explicitly cover `CompositeGpuPrescanConfig` and predictive seed-pair logic
- keep composite utility math separate from metric field semantics
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

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/grid.rs`
- `crates/optimizer/src/lib.rs`
- `crates/optimizer/src/mo.rs`
- `crates/optimizer/src/stream.rs`
- `crates/strategies/src/composite_cpu/mod.rs`
