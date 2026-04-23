# PR-08 Runtime cache and execution economics

- package type: `core`
- copied context files: `7`
- missing context files: `0`

## Prompt Scope

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

## First-Pass Prompt

Treat `PR-08` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
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

Attack the first-pass answer for `PR-08` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
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

Produce the strongest implementation-ready mathematical result for `PR-08` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
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

- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/state.rs`
