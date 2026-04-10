# VectorGrid Deep Research Program

Date: 2026-04-09

## Purpose

This folder is the working area for external deep-research requests that are meant to improve this repository.

The goal is not to treat outside research as automatically correct or automatically in scope.

The goal is to:

1. ask better repo-grounded questions
2. collect responses in a consistent format
3. translate valid findings into one of four outcomes:
   - implementation candidate
   - documentation insight
   - explicit rejection
   - open question that still needs local validation

## Why this lives under `research/`

These findings are still hypothesis-stage until they survive repo-fit review.

They should not go straight into `REPO_Improvement/` because that folder is closer to accepted execution work, while this folder is for:

- external comparative research
- theory that may or may not fit VectorGrid
- parity-gap discovery
- product and architecture insight gathering

## Repo-fit rules

Every external finding must be judged against the real product constraints of this repo:

- Rust workspace with a Tauri desktop app
- Vue 3 frontend
- Rust backend with optional CUDA paths
- CPU-only builds must remain supported
- benchmark-first and parity-safe engineering posture
- local-first artifact, report, and workflow model
- no unrestricted Python-style user code execution

## Additive-only rule

This research program is for additive improvements, not major product rewrites.

External research should bias toward:

- new supporting surfaces that preserve existing workflows
- stronger diagnostics, provenance, and comparison tooling
- better defaults, warnings, and documentation
- bounded feature additions that do not remove current capabilities
- small extensions to existing feature lanes

External research should not be used to justify, by default:

- removing existing features
- replacing a major workflow family
- broad architectural rewrites
- changing the product into a notebook or scripting platform
- introducing migration-heavy redesigns with high regression risk

If a research answer implies an invasive change, the default action is not "implement".

The default action is one of:

- reduce it to a smaller additive slice
- keep it as `Doc Only`
- mark it `Reject`

## Selective parity rule

VectorGrid should pursue parity with tools like VectorBT only where it materially improves one of these:

- research power
- correctness
- validation trust
- experiment lineage and comparison
- dataset and universe workflow
- operator understanding of route, fallback, and provenance

VectorGrid should not chase parity that would weaken its current product identity or reproducibility model.

## Negative findings are valid

A strong answer can conclude:

- the idea is not worth implementing
- the feature is already effectively covered in another VectorGrid form
- the parity gap is real but not important enough
- the idea conflicts with current product constraints

Those are acceptable outcomes and should still be documented.

## Folder layout

- `README.md`
  - workflow and decision rules
- `VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md`
  - stable question IDs and prompt text
- `VECTORGRID_DEEP_RESEARCH_SYNTHESIS_TEMPLATE_2026-04-09.md`
  - template for turning completed research into concrete repo decisions
- `responses/`
  - one Markdown file per completed deep-research response

## Suggested workflow

1. Pick one question from the question pack.
2. Give the deep-research model the repo context plus the answer contract from the question pack.
3. Save the returned response in `responses/` using the naming convention in `responses/README.md`.
4. Read the response as a proposal, not as truth.
5. Fill the synthesis template for that question:
   - what the model claimed
   - whether it fits VectorGrid
   - what is already present in repo docs or code
   - whether the result should become code, docs, or explicit rejection
6. Only after that, open or update an implementation-plan document if the finding is worth acting on.
7. Validate with repo-local evidence before implementation:
   - code inspection
   - tests
   - parity checks
   - perf gates
   - benchmark or artifact checks where relevant

## Preferred output classes

Each question should end in one of these statuses:

- `Adopt`
  - good fit and worth converting into a concrete repo slice
- `Adopt Later`
  - valid idea, but not current priority
- `Doc Only`
  - useful for product philosophy, guardrails, or contributor guidance
- `Reject`
  - not worth doing or not compatible with VectorGrid
- `Needs Local Validation`
  - plausible, but still too abstract or risky

## Translation rules

If a research finding is accepted, translate it into one of these repo forms:

- targeted implementation slice under `REPO_Improvement/`
- update to an existing roadmap or implementation guide under `research/` or `options_research/`
- short maintenance or policy note in an existing canonical doc

If a research finding is rejected, keep the reasoning in the synthesis document so the same idea is not re-litigated later without new evidence.

Accepted implementation slices should usually satisfy all of these:

- additive
- reversible
- locally testable
- narrow in file touchpoints
- low risk of changing semantics outside the target surface

## Suggested first pass

After a web-grounded quality pass, the strongest externally researchable first batch is:

- `DR-01`
- `DR-02`
- `DR-03`
- `DR-04`
- `DR-07`
- `DR-09`
- `DR-10`
- `DR-11`
- `DR-15`

That set should clarify:

- where parity with VectorBT actually matters
- which parity gaps are real versus cosmetic
- which experiment-lineage and compare flows are worth productizing
- which portfolio and data workflow gaps are strategically important
- which validation ideas are worth productizing
- how benchmark claims should be framed
- what the minimum useful options-parity target should be
- how realism should be described without blurring semantics
- which analysis and visualization gaps are most worth closing

## Better asked later or handled locally

These question IDs are still useful, but they are more dependent on local repo evidence than broad external research:

- `DR-05`
  - still valuable, but the answer will partly rely on product judgment rather than a canonical external standard
- `DR-06`
  - should only be asked after reviewing current local profiling and cache behavior
- `DR-08`
  - valuable, but should be timed closer to live-execution cleanup work
- `DR-12`
  - much of the answer already exists in official Tauri docs and repo release notes
- `DR-13`
  - mostly an internal architecture/design review question
- `DR-14`
  - worthwhile only if multi-objective productization is actually near-term

## Recommended context docs to attach or summarize

For general VectorGrid questions:

- `README.md`
- `REPO_MAP.md`
- `ARCHITECTURE_NOTES.md`
- `research/VECTORGRID_FEATURE_PARITY_ROADMAP_VS_VECTORBT_PRO_2026-03-19.md`
- `research/VECTORGRID_COMPETITIVE_IMPROVEMENT_PLAN_VS_VECTORBT_PRO_2026-03-19.md`

For options-expression questions:

- `options_research/options_expression_optimizer_viable_plan_2026-04-02.md`
- `options_research/options_expression_future_scope_roadmap_2026-04-07.md`

## Practical rule for this folder

Do not convert a research answer directly into code work without first writing down the repo-fit decision.
