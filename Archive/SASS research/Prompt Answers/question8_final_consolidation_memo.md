# Question 8 — Final Consolidation Memo

Date: 2026-04-08

## Adopted direction

According to the repo’s own Phase 2 artifact planning and research memos, the active `double_crossover` line has already been shifted from open-ended tuning into a research-artifact workstream, with product-facing code changes no longer the default objective. The final answer for Question 8 should therefore choose a broader-CS direction that does **not** reopen more CUDA work by default.

The strongest remaining direction is a **benchmark/evaluation paper on multi-fidelity trustworthiness for heterogeneous, route-conditional strategy optimization**, with **AutoML benchmark/evaluation venues as the primary fit**, especially **AutoML 2026 ABCD**. That track explicitly solicits benchmark contributions that improve AutoML benchmarking quality and even names HPOBench/NASBench-style resources as the right comparison class, with reproducibility and accessible tooling as hard expectations. A stronger, more evaluation-theory version could later stretch to **NeurIPS 2026 E&D**, which explicitly welcomes benchmark failure-mode analysis, evaluation-design comparisons, tools, protocols, and negative results, and does **not** require a new model or a SOTA win.

This means the paper should **not** be framed as “a new optimizer beats brute force,” and it should **not** be framed as “our CUDA kernel is fast.” It should be framed as:

> **When do real fidelity ladders preserve full-fidelity ordering well enough to justify promotion, and when do they fail, on a mixed strategy workload with exact anchors and route-conditioned costs?**

## Exact problem statement

The concrete problem is:

> **Given a fixed wall-clock budget, a heterogeneous set of strategy search spaces, backend- and route-dependent execution costs, and at least one exact full-fidelity anchor, how reliably do application-derived cheap fidelities preserve final full-fidelity ranking or regret, and what benchmark design is needed to measure that reliably?**

That is narrower and more defensible than a new optimizer paper. It turns the repo into an **evaluation environment** rather than a single-kernel story. The repo is strong enough for that because it already exposes `24` real optimization kinds, with `11` Double-MA-family kinds, `11` composite/advanced kinds, plus `CompositeMo` and `MlSignal`; it also routes `CompositeTrend`, `RegimeHybrid`, and `BreakoutMomentum` through the `DoubleMa` path while still retaining non-DoubleMa families on the composite side.

## Why this is still plausible after prior work

The nearest prior-work neighbors are already strong:

- **Hyperband** formulates budgeted HPO as resource allocation over configurations.
- **BOHB** and **DEHB** are strong mixed-space multi-fidelity baselines.
- **HPOBench** and **YAHPO Gym** already provide large multi-fidelity benchmark suites.

So a new optimizer-methods paper would collide directly with mature literature.

The direction still survives because the novelty target is different: not “invent another BOHB/DEHB variant,” but **show that this application induces an evaluation setting current benchmark suites do not capture cleanly**. The repo gives three ingredients that matter here:

1. an exact anchor (`DoubleMa`)
2. routed composite profiles whose effective backend changes with strategy conditions such as ADX
3. a broader family surface large enough to test whether fidelity logic generalizes beyond one clean lattice

## Why the alternatives were rejected

A new **AutoML methods** paper is rejected as the lead because AutoML’s Methods track explicitly covers configuration/tuning, portfolios, constrained optimization, and pipeline automation, which means this work would be judged directly against a saturated HPO literature unless it had a much sharper formal delta than it currently does. It does not.

Reopening the current **CUDA/SASS** line as the broader-CS target is also rejected. The repo’s own artifact plan says that branch has already been frozen into a research-artifact workstream; its strongest publishable framing is the existing static-plus-dynamic attribution case study, and further code changes are only justified if they close a research-artifact gap. That is a real paper, but it is a **different** paper.

A broader **compact exact search-stage / score-first** systems lane is deferred, not adopted. The repo’s own external review says that lane is higher risk and should move only after missing shortlist-heavy `xlarge` attribution is surfaced; it is not justified by default now.

## Role of the custom CUDA implementation

CUDA is **substrate only** here. It is valuable because it makes full-fidelity anchors and large-scale experiments affordable, but the conceptual claim has to survive a backend swap. That is exactly the rule the broader-CS prompt pack imposes, and it matches the repo’s own current separation between product-facing GPU engineering and broader-CS framing.

## Minimum prototype

The minimum serious prototype is small and mostly evaluative:

- define two or more real fidelity operators already implicit in the product workflow
- measure their rank/top-k preservation against full-fidelity evaluation
- build one promotion policy or evaluation protocol around those fidelities
- evaluate only on **final full-fidelity** quality
- publish the resulting suite as a reproducible benchmark/evaluation package, not merely an optimizer demo

That is a **design-first** and **measurement-first** follow-up, not a broad implementation reopening.

## Mandatory baseline matrix

The baseline matrix should be fixed now:

- **Exact baseline where available:** current exact GPU `DoubleMa` route, plus exact CPU on smaller/reference surfaces
- **Simple pragmatic baseline:** random or Sobol
- **Classical heuristic baseline:** one strong evolutionary or bandit-style baseline
- **Modern multi-fidelity baselines:** Hyperband plus at least one of BOHB or DEHB
- **Ablation baseline:** the proposed fidelity-aware evaluation or promotion rule with its main idea removed

Measures should stay exactly as the prompt requires:

- wall-clock to target quality
- simple regret under fixed budget
- anytime incumbent quality
- robustness across seeds
- cross-family behavior rather than `DoubleMa` only

## Mandatory benchmark slate

The benchmark slate should be the narrowest one that still proves breadth:

- **Structured exact anchor:** `DoubleMa`
- **Routed mixed-family profile:** at least one of `CompositeTrend`, `BreakoutMomentum`, or `RegimeHybrid`
- **One genuinely non-DoubleMa family:** for example `ASR`, `ROCB`, `VEMR`, `MKCS`, `KQSR`, `CARM`, `KVDF`, or `CDMC`
- **Validation-coupled case:** one workload where shortlist or validation cost materially changes the usefulness of cheap fidelities

Without that third item, the paper is overstating heterogeneity. The repo is broad enough to support this slate, but the evaluation must actually use that breadth rather than merely cite it.

## Deferred directions

Two directions stay deferred.

First, **MLSys-style search-space compiler / structure-extractor work**. MLSys 2026 clearly welcomes ML programming models, abstractions, compilers, runtimes, and benchmarks/tooling, so that venue family could make sense later. But the repo’s current artifact plan and external review explicitly say broader `compact_exact_search_stage` work is not justified by default yet.

Second, **portfolio / algorithm-selection** work. That remains plausible, but only after the benchmark exists and exposes meaningful cross-family complementarity. Right now it is better treated as a later consumer of the benchmark than as the first paper.

## Rejected directions

Three directions should be treated as rejected for this decision:

- a new universal optimizer paper
- reopening the current CUDA/SASS/kernel line as the broader-CS target
- any direction that still depends on “beat brute-force `double_ma` GPU” as the universal win condition

Those either collide too hard with mature HPO literature or are already claimed by the repo’s current artifact lane.

## Kill test

Kill this direction early if either of these happens:

1. Cheap fidelities fail to preserve rank/top-k order once evaluation moves beyond the exact `DoubleMa` anchor into at least one routed composite profile and one non-DoubleMa family.
2. The resulting suite does not expose behavior that looks materially different from what HPOBench or YAHPO Gym already let reviewers study.

If either happens, the direction is not a paper lead and should be closed rather than stretched.

## Final verdict

**Outcome: design-only follow-up.**

The strongest remaining direction is **not** another optimizer-methods paper and **not** another CUDA paper. It is a **benchmark/evaluation direction centered on multi-fidelity trustworthiness**, with **AutoML ABCD** as the primary venue family and **NeurIPS E&D** as a stretch only if the evaluation-science framing becomes unusually strong. The repo is broad enough to justify a pilot, but the current evidence does **not** justify reopening broad implementation work yet. The right next step is to assemble the benchmark slate, run the fidelity-validity pilot, and reopen implementation only if that pilot clears the kill test.

---

## Source notes

### Repo sources

- `double_crossover_phase2_research_artifact_plan_2026-04-03.md`
- `double_crossover_phase2_research_findings_2026-04-03.md`
- `double_crossover_phase2_external_review_findings_2026-04-03.md`
- `double_crossover_phase2_paper_outline_2026-04-03.md`
- `README.md`
- `REPO_MAP.md`
- `AGENTS.md`
- `01_first_pass_prompt.md`
- `02_second_pass_prompt.md`
- `03_final_consolidation_prompt.md`
- `vectorgrid_broader_cs_prompt_bundle_index_2026-04-08.md`

### Web sources used in the original answer

- AutoML 2026 call for papers: https://2026.automl.cc/call-for-papers/
- Hyperband (JMLR): https://jmlr.org/papers/v18/16-558.html
- MLSys 2026 call for papers: https://mlsys.org/Conferences/2026/CallForPapers
- YAHPO Gym / benchmark comparison reference: https://arxiv.org/abs/2109.06716
