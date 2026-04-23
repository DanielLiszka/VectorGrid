# Math Review Parallel Prompt Suite

Date: `2026-04-17`

This is the single prompt document for the VectorGrid math-review program.

It is optimized for fully parallel GPT-5.4 Pro execution. The earlier `MR-*`
packet model remains the exhaustive coverage and synthesis inventory, but the
execution model in this file is reduced to a smaller set of orthogonal `PR-*`
problem prompts. The prompt count is intentionally lower because several
backend surfaces turned out to be coupled strongly enough that splitting them
further would create conflicting ownership.

Use this file to run prompts in parallel.

Use these files for coverage and synthesis reference:

- [MASTER_MATH_REVIEW_TRACKER.md](</C:/Rust Projects/ta_desktop_demo/Math Review/MASTER_MATH_REVIEW_TRACKER.md>)
- [BACKEND_FILE_MATH_REVIEW_CHECKLIST.md](</C:/Rust Projects/ta_desktop_demo/Math Review/BACKEND_FILE_MATH_REVIEW_CHECKLIST.md>)

## Execution Model

There are two layers:

- `MR-*`:
  exhaustive repo coverage and final synthesis inventory
- `PR-*`:
  parallel-safe execution prompts with explicit ownership boundaries

Run the `PR-*` prompts in parallel.

Core parallel prompt items: `10`
Supplemental overlay prompt items: `8`
Total prompt units in this document: `18`

## Why The Prompt Count Is Smaller

A collision audit showed that some earlier prompt candidates were not actually
orthogonal:

- GPU metric contract semantics, canonical trade/risk semantics, and published
  metric-surface labels are one artifact boundary
- shortlist confidence reranking and slice robustness are one post-shortlist
  subsystem
- dynamic GPU executable algebra and residency economics share one execution
  chain
- validation utility, validation statistics, reopt/regime summaries, and
  cross-ticker aggregation share one ranking and verdict stack

Those surfaces are merged below on purpose.

## Parallel Safety Rules

Every `PR-*` prompt must follow these rules:

1. Own only the explicitly listed problem surface.
2. Treat `Primary ownership files` as the redesign surface.
3. Treat `Context files` as constraints and dependencies only.
4. Respect `Non-goals`. If an issue belongs to another `PR-*`, note it as a
   dependency instead of redesigning it.
5. Keep findings tagged with both the local `PR-*` ID and the source `MR-*`
   packet IDs.
6. When the same file appears in multiple prompts, ownership is by symbol family
   or decision surface, not by whole-file exclusivity.

## Math-First Prompt Doctrine

Every core `PR-*` prompt should be read as an open mathematical problem about a
discrete, grid-shaped, threshold-heavy, and path-dependent system, not as a
generic code-review task.

Each core prompt should force work in this order:

1. formalize the objects
2. state the exactness target
3. characterize the strongest plausible positive result
4. characterize the strongest plausible impossibility result
5. separate exact, approximate, and heuristic claims
6. derive algorithmic consequences only after the mathematical claim survives
   adversarial stress

## Mathematical Output Contract For Every Core Parallel Prompt

Every GPT-5.4 Pro core response should include:

1. `Formal problem statement`
2. `Notation and mathematical objects`
3. `Owned symbols or decisions`
4. `Fixed invariants and interfaces`
5. `Candidate lemmas or theorem claims`
6. `Counterexample family or impossibility angle`
7. `Approximation or exactness contract`
8. `Time, memory, and retained-state complexity`
9. `Dependencies on other PR prompts`
10. `Verification obligations and kill criteria`

## Recommended Parallel Workflow

1. Run all `First-Pass` prompts in parallel.
2. Run all `Second-Pass` prompts in parallel, feeding each model only its own
   `First-Pass` output.
3. Run all `Third-Pass` prompts in parallel, again keeping each thread inside
   its owned problem.
4. After you receive all parallel responses, consolidate them back onto the
   `MR-*` inventory.

## Prompt Index

1. `PR-01` Metric semantics and published surface contract
2. `PR-02` Shortlist post-ranking math
3. `PR-03` Static candidate-space pruning and route cost model
4. `PR-04` Dynamic GPU execution chain math
5. `PR-05` Composite search and utility math
6. `PR-06` Validation stack math
7. `PR-07` Meta ranking, holdout triage, and ML confidence
8. `PR-08` Runtime cache and execution economics
9. `PR-09` Options evaluation and options GPU economics
10. `PR-10` ML-signal policy math

## Overlay Index

11. `OX-01` Sensitivity, neighborhood geometry, and local fragility math
12. `OX-02` Response-surface compression and surrogate-ranking math
13. `OX-03` Sequential compute allocation and stopping-value math
14. `OX-04` Dependence-aware uncertainty and effective-sample-size math
15. `OX-05` Regime inference, changepoints, and latent-state math
16. `OX-06` Tail-risk, drawdown-path, and rare-event math
17. `OX-07` Fill, slippage, and execution-policy math
18. `OX-08` Calibration, inverse modeling, and threshold-geometry math

## Crosswalk

| `PR-*` | Parallel-safe problem | Source `MR-*` coverage |
| --- | --- | --- |
| `PR-01` | Metric semantics and published surface contract | `MR-01`, `MR-07`, `MR-14` |
| `PR-02` | Shortlist post-ranking math | `MR-02`, `MR-03` |
| `PR-03` | Static candidate-space pruning and route cost model | `MR-04` |
| `PR-04` | Dynamic GPU execution chain math | `MR-05`, `MR-06` |
| `PR-05` | Composite search and utility math | `MR-15`, `MR-04` |
| `PR-06` | Validation stack math | `MR-08`, `MR-09`, `MR-10`, `MR-13` |
| `PR-07` | Meta ranking, holdout triage, and ML confidence | `MR-11`, `MR-12` |
| `PR-08` | Runtime cache and execution economics | `MR-14` |
| `PR-09` | Options evaluation and options GPU economics | `MR-16` |
| `PR-10` | ML-signal policy math | `MR-17` |

## Shared File Symbol Boundaries

Use these boundaries during prompt execution and consolidation:

- `crates/app_core/src/double_ma.rs`
  `PR-01` owns metric field semantics and result-surface meaning.
  `PR-02` owns shortlist confidence, slice, omission, and plateau or local
  robustness logic.
  `PR-04` owns dynamic-GPU selection, preflight, uploaded-batch cache key, and
  residency-summary symbols.

- `crates/app_core/src/composite_cpu.rs`
  `PR-01` owns trade or risk field semantics and trade22 publication meaning.
  `PR-05` owns `objective_score`, scalarization, and optimizer-facing composite
  utility behavior.

- `src-tauri/src/commands_campaign.rs`
  `PR-07` owns holdout splitting, holdout triage, and admission heuristics.
  `PR-08` may use it only as runtime-economics context and must not redesign
  holdout math there.

- `src-tauri/src/state.rs`
  `PR-06` owns alignment and shared-horizon semantics when they affect
  cross-ticker validation behavior.
  `PR-08` owns retention, invalidation, and cache-economics heuristics.

- `src-tauri/src/run_artifacts.rs`
  `PR-01` owns metric-surface publication and provenance semantics tied to
  result meaning.
  `PR-08` may reference it only as runtime context.

## Standard First-Pass Prompt

Treat `PR-XX` as an open mathematical problem, not a code-review task. Stay
inside the owned problem and respect the non-goals. Use the primary ownership
files as the redesign surface and the context files as constraints only. Do not
propose implementation yet.

Deliver:

1. formal problem statement
2. notation for the search space, state space, path functional, ranking
   functional, and admissible approximations
3. fixed invariants and interface constraints
4. strongest plausible positive result
5. strongest plausible impossibility result
6. candidate lemmas or theorem statements
7. one explicit counterexample family
8. one lower-bound, identifiability, or impossibility angle
9. unresolved questions that block implementation

## Standard Second-Pass Prompt

Attack the first-pass answer as an adversarial mathematician. Try to break the
leading formalization, proposed sufficient statistics, pruning rules, ranking
claims, or approximation contracts with edge cases, dependence structure,
pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Deliver:

1. explicit failure modes
2. adversarial constructions or separating examples
3. corrected theorem or lemma candidates
4. weakest assumptions under which the result survives
5. distinction between exact, approximate, and heuristic claims
6. revised time, memory, and retained-state costs
7. dependency burden on other PR prompts

Narrow to the strongest surviving repo-fit result.

## Standard Third-Pass Prompt

Produce the strongest implementation-ready mathematical result for `PR-XX` that
survives the second pass. Stay inside the owned problem. Treat listed
dependencies as interfaces, not as permission to redesign adjacent prompts.
Any non-exact claim must come with an explicit approximation, error, regret,
rank-instability, or bias contract.

Deliver:

1. final formalization
2. final theorem or lemma list
3. proof sketch or proof plan
4. sufficient statistics, or a proof they do not exist under the stated
   constraints
5. exactness conditions and equivalence classes
6. error, regret, or bias bounds for every non-exact claim
7. time, memory, and retained-state complexity
8. file and symbol impact map
9. verification obligations and falsifiers
10. dependencies that must be reconciled during consolidation
11. kill criteria

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

## `PR-02` Shortlist post-ranking math

Owned problem:
everything that happens after initial candidate score generation but before the
final shortlisted ordering is accepted

Source packets:
`MR-02`, `MR-03`

Primary ownership files:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`

Context files:

- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/double_ma/mod.rs`

Own these symbols or decisions:

- effective-score algebra for shortlist post-processing
- score-first eligibility conditions
- confidence reranking formulas
- slice and omission summary mathematics
- benchmark-relative shortlist summaries
- plateau or local-neighborhood robustness summaries

Open problem:

- characterize when a discrete shortlist can be re-ranked exactly from
  post-score retained summaries, and when omission/slice robustness forces
  replay or stronger retained path state

Why this is hard here:

- the shortlist subsystem mixes reranking, omission diagnostics, benchmark
  comparisons, and local-neighborhood geometry in one path-dependent stage
- some summaries may be reconstructible from retained aggregates while others
  may depend irreducibly on exact path slices

Mathematical object:

- a finite candidate shortlist with path-dependent outcome sequences, plus a
  post-ranking functional that maps retained summaries to final shortlist order
  and robustness diagnostics

Notation seed:

- let `C = \{c_1,\dots,c_n\}` be the shortlisted candidate set
- let `X(c)` denote the full path-dependent evidence for candidate `c`
- let `S(c)` denote the retained post-score summary used for reranking
- let `\Pi(C; X)` be the exact final shortlist ordering and
  `\widehat{\Pi}(C; S)` the ordering induced by retained summaries only
- the exactness target is
  `\Pi(C; X) = \widehat{\Pi}(C; S)`;
  any relaxed claim must state a rank-loss or finalist-regret bound

Primary questions:

- what is the smallest retained summary that preserves exact shortlist ordering
  under the current rerank modes?
- when are slice/omission summaries identifiable from arithmetic-return
  aggregates alone?
- what lower bound forces pathwise replay for a class of robustness outputs?

Required deliverables:

- formal shortlist-ranking model
- theorem candidates for exact reranking sufficiency
- counterexamples separating exact and inexact omission/slice reconstruction
- rank-instability or regret bounds for every approximation

Acceptance bar:

- every replay-reduction claim must come with an exactness proof or an explicit
  instability bound

Verification hooks:

- shortlist order parity
- slice-summary parity
- omission-summary parity under adversarial edge cases

Non-goals:

- do not redesign base metric field meaning; that belongs to `PR-01`
- do not redesign validation pipeline statistics beyond what is explicitly used
  in shortlist reranking; that belongs to `PR-06`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly assess PSR or DSR use on shortlist-derived peer distributions
- explicitly assess reconstructed arithmetic-return summaries versus exact
  pathwise slice PnL
- treat confidence and slice diagnostics as one subsystem, not two

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

## `PR-04` Dynamic GPU execution chain math

Owned problem:
the full dynamic-GPU path from representable algebra through executable caching
to residency and uploaded-batch reuse

Source packets:
`MR-05`, `MR-06`

Primary ownership files:

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`

Primary ownership symbols in shared files:

- in `crates/app_core/src/double_ma.rs`:
  `prepare_dynamic_gpu_execution_selection`,
  `dynamic_gpu_preflight_required_for_backend`,
  `dynamic_gpu_batch_cache_key`,
  `get_or_build_dynamic_gpu_uploaded_batch`,
  `build_gpu_residency_summary`

Context files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_plan.rs`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

Own these symbols or decisions:

- representable dynamic strategy algebra
- executable ABI and fingerprint semantics
- executable-cache key sufficiency
- resident CUDA dataset ownership
- prepared-input caching and uploaded-batch reuse
- dynamic path residency and preflight economics

Open problem:

- characterize the expressive power and minimal state of the dynamic GPU
  execution chain under executable-cache, ABI, and uploaded-batch constraints

Why this is hard here:

- algebra, executable identity, uploaded-batch reuse, and residency economics
  are one coupled chain rather than separable optimizations
- every extra expressive feature potentially widens ABI state, cache keys, and
  resident-state obligations

Mathematical object:

- a discrete executable language with a compilation map, an execution-state
  cache, and a reuse relation over uploaded datasets and compiled programs

Notation seed:

- let `a \in \mathcal{A}` denote a dynamic strategy expression
- let `C(a) = e` be the compilation map into an executable object `e`
- let `D` denote the uploaded dataset/resident input state
- let `K(e, D)` be the cache identity used for exact executable/data reuse
- exact reuse requires that
  `(e_1, D_1) \equiv (e_2, D_2) \Rightarrow K(e_1, D_1) = K(e_2, D_2)`,
  while non-equivalent pairs must not alias

Primary questions:

- what is the minimal state needed to preserve exact executable reuse without
  aliasing?
- which algebra extensions are representable without forcing unbounded cache-key
  growth?
- when is exact uploaded-batch reuse impossible without stronger immutability or
  identity assumptions?

Required deliverables:

- formal executable-state model
- candidate minimal-state or cache-key sufficiency theorem
- aliasing counterexamples
- complexity and retained-state width implications

Acceptance bar:

- every reuse claim must either prove non-aliasing or state exact conditions
  under which aliasing can occur

Verification hooks:

- cache-key collision tests
- resident-state reuse parity
- executable identity falsifiers

Non-goals:

- do not redesign static route heuristics; that belongs to `PR-03`
- do not redesign runtime Tauri cache or execution economics; that belongs to
  `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly separate IR schema, compiled plan, executable fingerprint, and
  uploaded-batch identity
- cache aliasing risk is a first-class kill criterion

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

## `PR-06` Validation stack math

Owned problem:
the full validation ranking and verdict stack, including single-candidate
statistics, reopt/regime summaries, and cross-ticker aggregation

Source packets:
`MR-08`, `MR-09`, `MR-10`, `MR-13`

Primary ownership files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_shared.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/cscv_pbo.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `src-tauri/src/validation_common.rs`
- `src-tauri/src/commands_validation.rs`

Context files:

- `src-tauri/src/commands_reports_validation_common.rs`
- `src-tauri/src/bin/ta_double_ma_validation_cost.rs`
- `src-tauri/src/bin/ta_headless_validate.rs`
- `src-tauri/src/state.rs`
- `crates/app_core/src/bin/real_data_fidelity_audit.rs`

Own these symbols or decisions:

- robust-score and verdict contract
- PSR or DSR semantics
- bootstrap, CSCV/PBO, and superiority-test math
- surrogate slice-return assumptions
- reopt walk-forward summaries
- regime or bucket summaries
- cross-ticker aggregation and verdict rebuild math

Open problem:

- characterize a validation stack whose verdicts remain mathematically coherent
  under dependence, reruns, regime slicing, and cross-ticker aggregation

Why this is hard here:

- single-candidate statistics, walk-forward summaries, cross-ticker aggregation,
  and final verdict logic are coupled rather than separable post-processors
- dependence, overlapping windows, and surrogate slice-return constructions
  challenge naive exactness assumptions

Mathematical object:

- a layered validation functional that maps path data and rerun evidence into
  intermediate statistics, robust scores, and final verdict classes

Notation seed:

- let `E(c)` denote the full validation evidence collected for candidate `c`
- let `T(E(c))` be the vector of intermediate statistics
  `(psr, dsr, spa, cscv, bootstrap, reopt, regime, cross)`
- let `R(c) = \rho(T(E(c)))` be the robust-score map
- let `V(c) = \nu(T(E(c)), R(c))` be the final verdict class
- coherence requires that equivalent evidence implies equivalent verdicts:
  `T(E(c_1)) = T(E(c_2)) \Rightarrow V(c_1) = V(c_2)`

Primary questions:

- what sufficient statistics are needed for coherent verdicts across the full
  validation stack?
- when are current surrogate slice-return constructions exact, and when do they
  introduce identifiable bias?
- what dependence-aware aggregation rule yields stable verdicts across reruns,
  regimes, and tickers?

Required deliverables:

- formal verdict-stack model
- theorem or lemma candidates for verdict coherence
- one counterexample family for false confidence or incoherent verdicts
- explicit bias, regret, or instability bounds for any approximation

Acceptance bar:

- every non-exact validation shortcut must expose a measurable approximation
  contract

Verification hooks:

- verdict parity under rerun perturbations
- cross-ticker consistency checks
- adversarial dependence cases

Non-goals:

- do not redesign meta ranking or ML-confidence policy; that belongs to `PR-07`
- do not redesign ML-signal policy; that belongs to `PR-10`
- do not redesign report workbook structure; that is context only

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly assess the coupling between `robust_score`, `final_score`, and
  cross-ticker aggregation
- explicitly compare family differences in regime definitions

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

## `PR-08` Runtime cache and execution economics

Owned problem:
runtime cache-value heuristics, data refresh or invalidation economics, and live
execution sizing or bracket translation

Source packets:
`MR-14`

Primary ownership files:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`

Context files:

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`

Own these symbols or decisions:

- data cache value or refresh heuristics
- retention and invalidation economics in `state.rs`
- execution sizing, quantity rounding, and bracket translation economics
- runtime prefetch or cache-performance interpretation

Open problem:

- characterize runtime cache and execution policy as a constrained optimization
  problem over retention value, refresh cost, and execution distortion

Why this is hard here:

- cache, invalidation, prefetch, and execution sizing decisions are discrete and
  operational, but they still imply an objective over cost, reuse, and decision
  quality
- repo-wide metric semantics are fixed elsewhere, so this prompt must optimize
  economics without changing public result meaning

Mathematical object:

- a constrained policy over cache states, refresh actions, and execution-size
  outputs with explicit cost and distortion functionals

Notation seed:

- let `x_t` be the runtime cache/state snapshot at decision time `t`
- let `u_t \in \{\mathrm{retain}, \mathrm{evict}, \mathrm{refresh},
  \mathrm{prefetch}\}` be the cache action
- let `L(x_t, u_t) = c_{\mathrm{io}} + c_{\mathrm{mem}} + c_{\mathrm{stale}}`
  be the runtime cost functional
- let `q_t = \pi_{\mathrm{exec}}(b_t, r_t, \sigma_t)` be the execution-size rule
  from budget/exposure inputs to actionable size
- policy quality should be stated as minimizing
  `\mathbb{E}\left[\sum_t L(x_t, u_t)\right]` subject to operational constraints

Primary questions:

- what policy minimizes expected refresh/reload cost under current reuse
  patterns?
- when should invalidation or prefetch fire under a bounded-staleness or
  bounded-regret policy?
- what execution sizing rule best trades exposure control against rounding and
  bracket distortion?

Required deliverables:

- formal cache/execution policy model
- candidate sufficient conditions for good runtime policy decisions
- adversarial cache or sizing scenarios
- explicit cost/distortion tradeoff analysis

Acceptance bar:

- every policy claim must define the objective it is optimizing, not just a
  heuristic rule

Verification hooks:

- campaign-perf evidence
- prefetch control harnesses
- live sizing falsifiers under volatile or thin scenarios

Non-goals:

- do not redesign metric-surface meaning or artifact publication; that belongs
  to `PR-01`
- do not redesign route-selection math; that belongs to `PR-03`
- do not redesign options-specific GPU runtime economics; that belongs to
  `PR-09`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- keep user-facing interpretability and operational stability in scope

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

## Consolidation Protocol

After you receive all parallel responses, consolidate them in this order:

1. Normalize each response into:
   `problem`, `owned symbols`, `accepted invariants`, `proposed change`,
   `dependencies`, `verification`, `kill criteria`.
2. Group findings back onto the `MR-*` inventory in
   [MASTER_MATH_REVIEW_TRACKER.md](</C:/Rust Projects/ta_desktop_demo/Math Review/MASTER_MATH_REVIEW_TRACKER.md>).
3. Resolve conflicts by ownership:
   if two responses disagree, the prompt that owns the decision surface wins.
4. Treat dependencies as integration questions, not prompt failures.
5. Produce a final synthesis with:
   `accepted directions`, `rejected directions`, `required reconciliations`,
   `shared invariants`, and `recommended implementation order`.

## Consolidation Ownership Rules

Use these rules during synthesis:

- `PR-01` wins on metric-field meaning and published surface semantics
- `PR-02` wins on shortlist post-ranking and slice robustness math
- `PR-03` wins on static pruning and static route cost modeling
- `PR-04` wins on dynamic-GPU execution, ABI, caching, and residency math
- `PR-05` wins on composite utility and composite search math
- `PR-06` wins on validation statistics, validation utility, reopt/regime, and
  cross-ticker verdict math
- `PR-07` wins on meta ranking, holdout triage, and ML-confidence logic built on
  fixed validation outputs
- `PR-08` wins on runtime cache and execution economics
- `PR-09` wins on options evaluation and options-specific GPU economics
- `PR-10` wins on ML-signal policy math

## Supplemental Math Overlays

These overlays are not new ownership prompts. They are optional higher-math
lenses that you can append to one or more `PR-*` prompts to push the analysis
further without changing ownership boundaries.

Use overlays like this:

1. Choose a core `PR-*` prompt.
2. Attach one or more `OX-*` overlays to the end of that prompt.
3. Keep the core prompt's ownership and non-goals unchanged.
4. Treat the overlay as an additional mathematical lens, not a license to
   expand scope.

Discrete-first interpretation rules:

1. Assume the repo is primarily a discrete, grid-shaped, threshold-heavy, and
   path-dependent system unless a prompt explicitly identifies a genuinely
   continuous submodel.
2. Prefer finite differences, neighborhood comparisons, plateau-width analysis,
   rank stability, and piecewise-response reasoning over gradients, Hessians, or
   smooth convexity assumptions.
3. Treat surrogate models as pruning, compression, or ranking aids, not as
   replacements for exact final scoring unless a prompt explicitly justifies it.
4. Treat changepoint, latent-state, and control ideas as lightweight decision
   aids that must fit the current repo architecture, not as invitations to build
   a heavyweight forecasting or control stack.
5. If a higher-math idea requires continuity, differentiability, or iid
   assumptions that the current implementation does not plausibly satisfy, call
   that out explicitly and downgrade the idea rather than forcing it.

Overlay theorem obligations:

- every attached overlay must add at least one new lemma candidate, one new
  counterexample family, or one new bound that the core prompt would otherwise
  miss
- overlays are not allowed to add generic “further discussion”; they must add a
  mathematically testable obligation

### `OX-01` Sensitivity, neighborhood geometry, and local fragility math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-06`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- identify parameter directions with high local sensitivity
- estimate flat versus sharp optima, plateau width, and local second-difference
  or neighborhood fragility
- identify condition-number-like fragility where small input changes cause large
  score or verdict changes
- propose mathematically cheaper stability summaries than brute-force local
  re-evaluation when possible

Overlay notation seed:

- for a discrete score surface `f : G \to \mathbb{R}`, define first differences
  `\Delta_i f(g) = f(g + e_i) - f(g)` and local second differences
  `\Delta_i^2 f(g) = f(g + e_i) - 2 f(g) + f(g - e_i)`

Overlay prompt:

Add a sensitivity and neighborhood-geometry lens to this review. For the owned
problem, identify whether the current formulas, ranking surfaces, or thresholds
behave like flat plateaus, sharp ridges, unstable cliffs, or ill-conditioned
decision boundaries on the discrete search lattice. If possible, describe
tractable finite-difference, local second-difference, or neighborhood fragility
metrics that could be computed cheaply enough to guide ranking, validation, or
pruning.

### `OX-02` Response-surface compression and surrogate-ranking math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-07`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/meta_select.rs`

What this overlay asks:

- look for low-dimensional structure in high-dimensional score surfaces
- identify whether local response surfaces can be compressed, parameterized, or
  approximated without losing rank quality
- assess whether surrogate ranking, local interpolation, or lattice-aware
  summaries could reduce compute

Overlay notation seed:

- seek a reduced model `\tilde f(g) = \sum_{k=1}^r \alpha_k \phi_k(g)` or other
  compressed surrogate such that rank error
  `\Pr(\operatorname{sign}(f(g)-f(h)) \neq \operatorname{sign}(\tilde f(g)-\tilde f(h)))`
  stays acceptably small on the candidate lattice

Overlay prompt:

Add a response-surface compression lens to this review. Determine whether the
owned problem has exploitable low-dimensional structure, locally regular lattice
behavior, or surrogate-friendly ranking behavior. Focus on repo-fit mathematical
reductions that could cut candidate evaluations or improve ranking quality
without replacing exact final scoring with a generic black-box optimizer.

### `OX-03` Sequential compute allocation and stopping-value math

Best attached to:

- `PR-03`
- `PR-05`
- `PR-06`
- `PR-07`
- `PR-08`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_campaign.rs`

What this overlay asks:

- treat compute as a scarce resource allocated across candidates or datasets
- assess early stopping, sequential testing, or value-of-information math
- identify whether partial evidence can justify terminating, expanding, or
  deferring evaluation

Overlay notation seed:

- let `B` be the total compute budget and `a_t` the stage-`t` allocation action
- seek a policy maximizing expected marginal value
  `\mathbb{E}[\Delta V_t(a_t)] / \Delta C_t(a_t)` subject to
  `\sum_t \Delta C_t(a_t) \le B`

Overlay prompt:

Add a sequential compute-allocation lens to this review. Treat the owned problem
as a decision under limited budget. Identify where partial evidence could be
used to stop early, continue selectively, or allocate more compute only where
expected information gain justifies it.

### `OX-04` Dependence-aware uncertainty and effective-sample-size math

Best attached to:

- `PR-02`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- review independence assumptions that are likely false
- replace naive sample counts with effective sample size where possible
- consider autocorrelation, clustered outcomes, overlapping windows, and
  repeated use of the same bars

Overlay notation seed:

- a standard target is an effective sample size of the form
  `n_{\mathrm{eff}} = n \big/ \left(1 + 2 \sum_{k \ge 1} \rho_k\right)`
  or a cluster-adjusted analogue when iid assumptions fail

Overlay prompt:

Add a dependence-aware uncertainty lens to this review. Identify where the
owned problem implicitly assumes iid evidence, independent trades, or
independent windows when the repo structure likely violates that assumption.
Propose effective-sample-size or dependence-aware alternatives where they would
materially change decisions.

### `OX-05` Regime inference, changepoints, and latent-state math

Best attached to:

- `PR-03`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- assess whether current median splits or coarse buckets are standing in for
  latent-state structure
- consider changepoint or lightweight hidden-regime views where mathematically
  useful
- identify lightweight regime-inference ideas that fit the repo better than a
  heavyweight forecasting stack

Overlay notation seed:

- let changepoints satisfy
  `1 \le \tau_1 < \tau_2 < \cdots < \tau_m \le T`
  and let `r_t \in \{1,\dots,K\}` be a latent regime label when a hidden-state
  view is used

Overlay prompt:

Add a latent-state and changepoint lens to this review. Determine whether the
owned problem is currently using coarse heuristics where a more principled
regime or changepoint view would materially improve correctness, stability, or
compute allocation.

### `OX-06` Tail-risk, drawdown-path, and rare-event math

Best attached to:

- `PR-01`
- `PR-06`
- `PR-09`

Most relevant repo surfaces:

- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`

What this overlay asks:

- emphasize path-dependent tail behavior rather than only mean or Sharpe-like
  summaries
- identify whether drawdown path shape, CVaR-style measures, or rare-event
  proxies would improve ranking or validation
- look for cheap retained summaries that capture tail information without full
  replay

Overlay notation seed:

- for equity curve `E_t`, define drawdown
  `D_t = 1 - E_t / \max_{u \le t} E_u`
  and rare-event/tail summaries such as
  `\mathrm{CVaR}_\alpha(L) = \mathbb{E}[L \mid L \ge \mathrm{VaR}_\alpha(L)]`

Overlay prompt:

Add a tail-risk and rare-event lens to this review. Focus on whether the owned
problem is under-measuring path-dependent downside or rare-event exposure, and
whether there are low-marginal-cost summaries that would capture the missing
information more faithfully.

### `OX-07` Fill, slippage, and execution-policy math

Best attached to:

- `PR-08`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- treat execution and fill assumptions as controllable mathematical models
- identify where fixed-percentage or fixed-threshold behavior should instead be
  volatility-, liquidity-, or uncertainty-aware
- review execution-policy logic through a constrained decision lens where useful

Overlay notation seed:

- an execution distortion model can be written as
  `p_{\mathrm{fill}} = p_{\mathrm{mid}} + s(q,\sigma,\ell)`
  where `q` is size, `\sigma` is volatility, and `\ell` is a liquidity proxy

Overlay prompt:

Add a fill, slippage, and execution-policy lens to this review. Identify where
the owned problem uses fixed heuristics that are really implicit execution or
fill models, and assess whether more principled policy-style or
uncertainty-aware math would materially improve realism or ranking quality.

### `OX-08` Calibration, inverse modeling, and threshold-geometry math

Best attached to:

- `PR-02`
- `PR-07`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- inspect whether current thresholds are arbitrary constants standing in for an
  unmodeled inverse problem
- review calibration, monotone transforms, and threshold geometry
- identify whether current decision boundaries could be made more stable through
  explicit inversion or calibration logic

Overlay notation seed:

- let a calibrated map satisfy `\hat p = g(s)` for raw score `s`, with decision
  rule `a = \mathbf{1}\{\hat p \ge \tau\}` or a more general threshold geometry
  over policy parameters

Overlay prompt:

Add a calibration and threshold-geometry lens to this review. For the owned
problem, identify where hard thresholds or nonlinear transforms behave like
poorly calibrated inverse mappings. Propose repo-fit calibration or
reparameterization ideas that would make decisions more stable and interpretable.
