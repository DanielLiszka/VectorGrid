# VectorGrid Deep Research Question Pack

Date: 2026-04-09

## How to use this document

Each question has a stable ID so that:

- the prompt can be reused
- the saved response can be named consistently
- the later synthesis step can reference it cleanly

Use the global context block and answer contract below for every prompt unless a specific question says otherwise.

## Global context block

Use this as the prefix before any prompt:

`Project context: VectorGrid is a Windows-first Tauri desktop app with a Vue 3 frontend and a Rust backend with optional CUDA paths. It focuses on exact-grid trading strategy optimization and validation, local-first artifacts and reports, route/fallback transparency, and CPU-only support when CUDA is unavailable. The product is not trying to become an unrestricted Python strategy engine, but it does care about selective parity with VectorBT where that parity materially improves research power, correctness, validation trust, data workflow, or experiment ergonomics.`

## Required answer contract

Ask the research model to structure every answer with these sections:

1. `Direct answer`
2. `Why this matters for VectorGrid`
3. `What comparable tools already do`
   - especially VectorBT, and LEAN when options research is involved
4. `Best fit for VectorGrid`
   - what clearly fits
   - what conflicts with the repo's constraints
5. `Recommendation`
   - choose exactly one: `Implement Now`, `Backlog`, `Doc Only`, `Reject`
6. `Change shape`
   - choose exactly one: `Additive`, `Mostly Additive`, `Invasive`
7. `Smallest useful implementation slice`
8. `Verification and benchmark plan`
9. `Main risks or counterarguments`
10. `Open questions, if any`

Also require the model to be explicit when it is inferring rather than citing a standard or established practice.

For architecture-heavy prompts, require one extra rule:

- if the answer depends more on local profiling or local product constraints than on external precedent, the model must say so explicitly

For every prompt, require these guardrails:

- preserve existing user-visible capabilities by default
- prefer additive improvements over replacement proposals
- if the best answer would require removing or substantially reworking a current feature, mark that answer `Invasive` and provide a smaller additive fallback instead
- do not recommend broad rewrites unless the question explicitly asks for them

## Prompt ordering

### Highest-confidence deep-research questions

These have strong external anchors in current VectorBT docs, LEAN docs, MLOps experiment-tracking practice, or established validation literature.

- `DR-01`
- `DR-02`
- `DR-03`
- `DR-04`
- `DR-07`
- `DR-09`
- `DR-10`
- `DR-11`
- `DR-15`

### Good second-wave questions

These are still worthwhile, but they lean a bit more on repo-specific judgment.

- `DR-05`
- `DR-08`

### Use selectively

These can still be good questions, but they are less ideal for external deep research because local repo evidence should dominate.

- `DR-06`
- `DR-12`
- `DR-13`
- `DR-14`

## Questions

### DR-01 - Selective parity with VectorBT

`Identify the highest-value selective parity gaps between VectorGrid and VectorBT Pro for a local-first, GPU-aware desktop research product. Do not assume parity means cloning VectorBT. Rank the top gaps by impact on research power, correctness, validation trust, dataset workflow, experiment comparison, and overall product credibility. For each gap, say whether VectorGrid should implement it, backlog it, document why it should not do it, or explicitly reject it. Favor additive parity targets that preserve existing VectorGrid workflows rather than proposals that require major redesign or feature removal.`

### DR-02 - Experiment comparison and lineage

`What are the best current patterns for experiment comparison, lineage, and run intelligence in serious quantitative research tools? Focus on request diffing, provenance tracking, parent-child relationships, rerun comparability, and candidate-to-validation lineage. Recommend the smallest high-value set of additive comparison and lineage capabilities that would close a meaningful parity gap with VectorBT-style research ergonomics without turning VectorGrid into a notebook platform or replacing current artifact workflows.`

### DR-03 - Portfolio and basket research parity

`What is the smallest first-class portfolio or basket-research surface that would materially improve VectorGrid's competitiveness with VectorBT while staying compatible with a guardrailed desktop product? Compare equal-weight, fixed-weight, score-weighted, and top-k portfolio models; basket-level validation; and multi-asset compare flows. Distinguish what should be treated as a real feature gap versus what is too broad or too open-ended for VectorGrid's current product philosophy. Prefer bounded additive surfaces that can coexist with current single-dataset workflows rather than replacing them.`

### DR-04 - Validation stack and overfitting defense

`What is the most statistically defensible validation stack for a local exact-grid strategy optimizer that can evaluate tens of thousands to hundreds of thousands of candidates quickly? Compare walk-forward, nested cross-validation, CSCV/PBO, SPA, White's Reality Check, deflated Sharpe, bootstrap families, and holdout variants. Recommend a fast default validation stack and a stricter optional stack for VectorGrid, including what should be operator-visible and how confidence should be described without overstating certainty. Favor additive improvements to the current validation workflow over proposals that would replace or destabilize it.`

### DR-05 - Route provenance, fallback honesty, and artifact metadata

`How should a GPU-aware desktop optimizer communicate route provenance and fallback behavior honestly? Use examples from scientific computing tools, quant platforms, and benchmarking/reporting practice to recommend a user-facing and artifact-facing taxonomy for CpuOnly, GpuOnly, GpuKernel, GpuHybridKernel, and Auto that makes exact versus staged versus fallback execution clear. Include recommended wording, metadata fields, warning thresholds, and report/artifact provenance rules that would help users understand what actually happened during a run. Be explicit about which parts of the answer are product judgment rather than external standard.`

### DR-06 - GPU runtime architecture for repeated optimization and validation

`Survey the best 2024-2026 architecture patterns for GPU-accelerated indicator sweeps and backtests when repeated runs reuse the same dataset. For a Rust + CUDA engine with warm in-memory and VRAM-resident execution paths, compare per-indicator kernels, fused kernels, staged device graphs, caller-owned device output buffers, and runtime cache reuse strategies. Recommend only the externally mature patterns that are realistic to borrow into VectorGrid, and explicitly separate those from ideas that are likely too repo-specific to answer without local profiling.`

### DR-07 - Benchmark methodology and public comparison discipline

`What is the most credible way to benchmark and publicly compare a GPU trading optimizer like VectorGrid against a CPU-oriented workflow like VectorBT? Define a benchmark charter covering warm versus cold runs, dataset loading boundaries, fairness rules, semantic alignment, disclosure wording, scenario selection, and what claims should be avoided. Recommend a README-grade disclosure model and a stricter internal benchmark policy for future comparisons. Keep the result focused on additive benchmarking discipline and disclosure improvements, not on changing the product to fit a benchmark.`

### DR-08 - Live execution operator safety

`What operator-safety patterns are best for a desktop quant app that connects research results to live execution using Alpaca and Polygon? Cover dry-run versus live separation, delayed versus real-time data warnings, stale-data handling, order-confirmation models, kill switches, audit logs, runner state transitions, and degraded-mode UX. Recommend the smallest set of safety improvements that should exist before expanding live execution breadth.`

### DR-09 - Minimum useful options parity envelope

`For an options-expression optimizer that wants LEAN-like credibility without inheriting the full complexity of LEAN, what is the minimum useful parity envelope? Focus on fixed-shape contract selection, deterministic structure compilation, template exits, assignment semantics, execution assumptions, and validation credibility. Distinguish the parity targets that are worth pursuing from those that should remain intentionally out of scope for a GPU-first desktop product. Favor additive semantics and reporting improvements that build on the current options lane instead of widening scope into a simulator rewrite.`

### DR-10 - Options realism lanes and semantic clarity

`For intraday options strategy optimization, what are the most defensible execution semantics across aggregate-bar realism, derived-mid realism, and quote-aware top-of-book realism? Define the terminology, artifact wording, user-facing warnings, and product-policy boundaries that keep these lanes clear and honest. Recommend how VectorGrid should separate a fast canonical lane from a more realistic lane without confusing users about what each result means. Favor additive lane clarification and reporting over proposals that would replace the current options workflow.`

### DR-11 - Dataset catalog, universes, and data workflow parity

`What are the highest-value dataset and universe-management capabilities in mature quant research tools, and which of those are worth bringing into VectorGrid? Focus on local dataset catalogs, provenance, refresh history, derived-series lineage, saved universes, benchmark baskets, and cross-workflow reuse. Separate true workflow gaps from nice-to-have data-platform breadth that does not fit the current product. Prefer additive catalog and reuse capabilities that preserve current CSV and Polygon flows.`

### DR-12 - Packaging, supportability, and optional-CUDA desktop delivery

`For a Windows-first Tauri desktop app with optional CUDA acceleration, what are the best practices for packaging, capability detection, crash diagnosis, support policy, and user messaging? Compare prebuilt PTX, optional local CUDA toolchain assumptions, GPU capability checks, CPU fallback posture, installer disclosure, and log/telemetry collection. Focus on user-visible supportability and release policy rather than code-level implementation detail, and say clearly which recommendations already follow from official Tauri guidance versus broader desktop-app practice.`

### DR-13 - Capability modeling for constrained strategy building

`What capability-model pattern is most likely to reduce frontend/backend drift in a constrained strategy builder that must keep controls honest about candle-field requirements, indicator support, route eligibility, validation support, and fallback behavior? Compare patterns from compilers, query planners, analytics engines, and typed feature registries, but keep the answer grounded in practical schemas and ownership boundaries rather than abstract framework design. If external precedent is weak, say so explicitly and fall back to a smaller set of transferable design rules.`

### DR-14 - Multi-objective workflow productization

`Should a CPU-bound multi-objective optimization workflow be productized further in VectorGrid right now, or mainly clarified and constrained? Compare shortlist construction, Pareto presentation, candidate recommendation, metric explanation, and confidence language in serious optimization tools. Recommend what VectorGrid should surface now, what should remain hidden, and what product behaviors tend to cause misuse or false confidence.`

### DR-15 - Analysis and visualization parity that actually matters

`Which analysis and visualization capabilities in mature tools like VectorBT most materially improve research productivity, candidate inspection, and trust, and which of those would translate well into VectorGrid's desktop workflow? Focus on trade-history inspection, benchmark-relative views, MAE/MFE-style diagnostics, parameter-surface exploration, multi-panel candidate analysis, and cross-run visual comparison. Distinguish high-value parity targets from notebook-native or overly open-ended capabilities that VectorGrid should not imitate. Prefer additive analysis surfaces that enrich current result workflows rather than replacing them.`

## Notes for future expansion

If this pack grows later, preserve the stable IDs above and append new IDs instead of renumbering the list.
