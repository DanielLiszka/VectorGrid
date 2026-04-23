# PR-02 Shortlist post-ranking math

- package type: `core`
- copied context files: `6`
- missing context files: `0`

## Prompt Scope

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

## First-Pass Prompt

Treat `PR-02` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
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

Attack the first-pass answer for `PR-02` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
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

Produce the strongest implementation-ready mathematical result for `PR-02` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
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
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/vemr_gpu_kernel.rs`
- `crates/strategies/src/double_ma/mod.rs`
