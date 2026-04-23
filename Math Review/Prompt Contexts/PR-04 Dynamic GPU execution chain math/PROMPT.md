# PR-04 Dynamic GPU execution chain math

- package type: `core`
- copied context files: `10`
- missing context files: `0`

## Prompt Scope

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

## First-Pass Prompt

Treat `PR-04` as an open mathematical problem, not a code-review task. Stay inside the owned problem and respect the non-goals. Use the primary ownership files as the redesign surface and the context files as constraints only. Do not propose implementation yet.

Use the following prompt scope block exactly:

```md
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

Attack the first-pass answer for `PR-04` as an adversarial mathematician. Try to break the leading formalization, proposed sufficient statistics, pruning rules, ranking claims, or approximation contracts with edge cases, dependence structure, pathologies, and finite-grid effects. Do not redesign out-of-scope surfaces.

Use the following prompt scope block exactly:

```md
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

Produce the strongest implementation-ready mathematical result for `PR-04` that survives the second pass. Stay inside the owned problem. Treat listed dependencies as interfaces, not as permission to redesign adjacent prompts. Any non-exact claim must come with an explicit approximation, error, regret, rank-instability, or bias contract.

Use the following prompt scope block exactly:

```md
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

- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`
- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_plan.rs`
- `src-tauri/src/commands_dynamic.rs`
