# Experiment Comparison and Lineage Patterns for VectorGrid

```text
### DR-02 - Experiment comparison and lineage

`What are the best current patterns for experiment comparison, lineage, and run intelligence in serious quantitative research tools? Focus on request diffing, provenance tracking, parent-child relationships, rerun comparability, and candidate-to-validation lineage. Recommend the smallest high-value set of additive comparison and lineage capabilities that would close a meaningful parity gap with VectorBT-style research ergonomics without turning VectorGrid into a notebook platform or replacing current artifact workflows.`
```

## Executive summary

The second top-level question in `VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md` is **DR-02**, reproduced verbatim above (from `/mnt/data/VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md`, lines 92–94). The question asks for **current best practices** in five concrete areas—request diffing, provenance, parent–child relationships, rerun comparability, and candidate→validation lineage—and then asks for the **smallest additive set** of capabilities that meaningfully narrows a VectorBT-style “research ergonomics” gap **without** turning VectorGrid into a notebook platform or replacing the existing artifact workflow.

Across mature tooling, the most transferable “best pattern” is a **two-store model**: a lightweight **metadata index** (run headers, params, relationships, dataset pointers) plus a **file-based artifact store** (reports, logs, arrays, tables). MLflow documents this explicitly (metadata in a backend store; artifacts in an artifact store), and it’s compatible with local-first desktop products. (`https://mlflow.org/docs/latest/ml/tracking/`). citeturn0search6

For VectorGrid specifically, the smallest high-value additive slice is:

1. **Canonical “request + dataset + environment” fingerprints** (hashes) recorded for every run, enabling fast request diffing and rerun comparability checks, modeled after MLflow’s emphasis on recording parameters/code version and its dataset digest approach. (`https://mlflow.org/docs/latest/ml/tracking/`, `https://mlflow.org/docs/latest/ml/dataset/`). citeturn4search1turn4search4  
2. **Explicit parent–child run links** (optimize → validate → meta/holdout), following well-established nested-run patterns (MLflow) and DAG lineage patterns (W&B artifacts lineage graphs). (`https://mlflow.org/docs/latest/ml/tracking/tracking-api/`, `https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`). citeturn0search5turn2search11  
3. **A “compare” surface that is diff-first**: show only changed request fields by default and numeric deltas for a curated set of metrics, analogous to DVC’s `exp diff` / `metrics diff` and “only-changed” table patterns. (`https://dvc.org/doc/command-reference/exp/diff`, `https://dvc.org/doc/command-reference/exp/show`). citeturn0search2turn4search3  
4. **Candidate-to-validation linkage** keyed by stable candidate IDs associated to an optimization run, so a user can traverse from a saved candidate to its validation and holdout evidence (W&B-style “inputs/outputs of a run” translated into “candidates in / results out”). (`https://docs.wandb.ai/models/registry/lineage`). citeturn2search8

Recommendation: **Implement Now** (additive) because it increases research trust and speed-of-iteration while aligning with VectorGrid’s “guardrailed desktop” posture stated in the global context block. (`/mnt/data/VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md`, line 19). The main risks are UI/UX complexity creep and metadata drift; both can be controlled by making the index schema small, versioned, and derived from (or validated against) existing artifact manifests.

## Locating the second question and defining the parsing rule

I opened the file at `/mnt/data/VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md` and extracted DR-02 by applying an explicit “top-level question entry” rule.

Rule used to detect “top-level question entry” in this file:

- Identify the `"## Questions"` section.
- Within that section, treat any Markdown heading matching `^### DR-\d+\s+-\s+` (at column 0) as a **top-level question entry**.
- The entry’s “exact text” is defined as:
  - the `### DR-XX - ...` heading line, plus
  - the immediately following backtick-quoted prompt paragraph (the line starting with a backtick and ending with a backtick),
  - stopping before the next `### DR-` heading.

Under this rule, the first entry is DR-01 and the **second entry is DR-02** (lines 92–94). This also matches the intent in the “stable ID” design of the question pack (questions are anchored by DR IDs rather than numeric markdown lists). (`/mnt/data/VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md`, multiple lines including 92–96).

### Ambiguity assessment

This file contains an additional “Prompt ordering” section (a curated list of IDs) that could confuse a naive “second question” detector. In this specific document, however, all reasonable interpretations still point to **DR-02** as “second” because the ordering list begins with DR-01, DR-02, DR-03… and the canonical “Questions” section also begins with DR-01, DR-02… (so there is no conflict in outcome).

| Possible interpretation of “second question” | Detection scheme | Would it pick DR-02? | Why it’s weaker or stronger |
|---|---|---:|---|
| Second entry under “Questions” | Second `### DR-XX` block after `## Questions` | Yes | Strongest: it targets the explicit prompt text, not a mention list. |
| Second item in “Highest-confidence deep-research questions” | Second bullet in the curated list | Yes | Weaker: the list is not the full prompt; it’s only IDs. |
| Second “DR-XX” occurrence anywhere in the file | Global scan for `### DR-` headings | Yes | Usually works here, but less robust if later editors add non-question DR headings elsewhere. |

## What comparable tools already do and what constitutes “best current patterns”

The request is explicitly about **comparison, lineage, and run intelligence**. “Best patterns” are those that (a) are widely adopted, (b) have clear user-facing affordances, and (c) map cleanly to a local-first artifact-bound workflow.

image_group{"layout":"carousel","aspect_ratio":"16:9","query":["MLflow experiment run comparison UI screenshot","Weights and Biases artifact lineage graph screenshot","DVC exp diff output table screenshot","OpenLineage lineage diagram facets"],"num_per_query":1}

### Request diffing

A “request diff” is the ability to answer: “What changed between run A and run B?” Mature tools treat this as a first-class operation by storing **structured parameters** and emitting a diff that defaults to “only what changed.”

- DVC’s experiment tooling includes `dvc exp diff`, which prints a table of differences between experiments across parameters and metrics, and explicitly supports a “show everything vs only changed” posture. (`https://dvc.org/doc/command-reference/exp/diff`). citeturn0search2  
- DVC also has `dvc exp show`, which renders experiments in a table that includes parent relationships and colored columns for metrics and parameters, plus an “only changed” option (`--only-changed`). (`https://dvc.org/doc/command-reference/exp/show`). citeturn4search3  

Translatable pattern: store the request as a structured object and provide a default diff view that suppresses noise unless expanded.

### Provenance tracking

Provenance usually includes: dataset identity (what data), code identity (what version), environment identity (where/how), and transformation identity (what produced what).

- MLflow’s Tracking concept is explicitly about logging parameters, code versions, metrics, and output files (artifacts) and then exploring/comparing them later. (`https://mlflow.org/docs/latest/ml/tracking/`). citeturn0search6  
- MLflow also provides dataset tracking where a dataset object includes a digest (hash/fingerprint) and source lineage metadata, and can be logged as an “input” to a run. (`https://mlflow.org/docs/latest/ml/dataset/`). citeturn4search4  

Translatable pattern: treat a dataset as an addressable object with a stable digest and source metadata, and record it as a run input. For a desktop quant app, that becomes “base dataset + derived-series lineage + split assignment + timeframe,” recorded as a small manifest.

### Parent-child relationships

Parent–child relationships matter whenever one workflow step spawns multiple sub-runs or evaluations (hyperparameter sweeps, cross-validation folds, optimize→validate expansions).

- MLflow documents nested runs (“hierarchical runs with parent-child relationships”) and stores the parent run ID for nested runs. (`https://mlflow.org/docs/latest/ml/tracking/tracking-api/`). citeturn0search5  
- OpenLineage’s standard lineage model is built around **run / job / dataset** entities and allows extensible metadata (“facets”) attached to those entities; its spec includes a notion of parent linkage via facets in examples. (`https://openlineage.io/docs/spec/facets/`). citeturn4search0  

Translatable pattern: keep explicit pointers, not inferred ones. Parent IDs should be first-class metadata fields, not “best effort” string parsing.

### Rerun comparability

Comparability is more nuanced than “can I rerun it.” Good tools distinguish “equivalent request” from “equivalent outcome,” and they preserve enough metadata to explain differences.

- MLflow’s core pitch includes logging code version and metadata so you can later analyze runs and reproduce them; it also uses system tags (for example git commit info where available) to capture execution context. (`https://mlflow.org/docs/latest/ml/tracking/tracking-api/`). citeturn0search5  
- DVC’s experiment system assumes determinism and is deeply tied to versioned state (Git), which is an explicit posture that makes “comparability” definable. (`https://dvc.org/doc/command-reference/exp`). citeturn4search9  
- QuantConnect’s local LEAN workflow stores backtest results in a timestamped directory and includes raw result files; the local results directory includes a `code/` directory copying the algorithm files, plus config/log outputs—an explicit “freeze what ran” pattern. (`https://www.quantconnect.com/docs/v2/lean-cli/api-reference/lean-backtest`, `https://www.quantconnect.com/docs/v2/local-platform/backtesting/results`). citeturn1search0turn1search1  

Translatable pattern: store a “run capsule” that can be reopened and audited locally (files), plus an index (metadata) that supports cross-run searching and comparisons.

### Candidate-to-validation lineage

Classic ML lineage graphs track datasets and models flowing through runs. For VectorGrid, the analogous “atomic objects” include **candidates** (strategy configs) and **validation evidence**.

- W&B documents lineage graphs as DAGs of artifacts and runs, explicitly showing “artifacts used as inputs” and “artifacts created as outputs.” (`https://docs.wandb.ai/models/registry/lineage`). citeturn2search8  
- W&B also describes artifact lineage graphs as directed acyclic graphs that support reproducibility and version control by linking run inputs/outputs over time. (`https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`). citeturn2search11  

Translatable pattern: treat “candidate sets” and “validation results” as artifact types with explicit input/output edges:
- optimization run outputs candidate-set artifact
- validation run inputs candidate-set artifact and outputs validation-result artifact

## Best fit for VectorGrid and the smallest additive high-value capability set

VectorGrid’s context explicitly emphasizes a **guardrailed**, local-first desktop product that is not trying to become an unrestricted scripting/notebook platform. (`/mnt/data/VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md`, line 19). It also has strong internal invariants: preserve existing workflows (optimize/validate/meta/holdout/reports/artifacts) and keep CPU-only builds working. (`/mnt/data/ARCHITECTURE_NOTES.md`, lines 10–12).

This section answers DR-02 using the question pack’s requested contract, but expressed as concise labeled paragraphs rather than additional headings.

**Direct answer**  
Implement a **local run index + lineage graph** on top of existing run artifacts, anchored by three stable fingerprints per run—(1) canonical request hash, (2) dataset hash (plus derived lineage), and (3) environment/build hash—and two explicit relationship layers—(a) parent/child run IDs and (b) candidate IDs that survive promotion from optimize → validate → holdout/meta. This mirrors stable, proven patterns in MLflow (runs/experiments with datasets + nested runs) and W&B (DAG lineage graphs of inputs/outputs), and it enables DVC-style diff-first comparisons without introducing a notebook runtime. (`https://mlflow.org/docs/latest/ml/tracking/`, `https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`, `https://dvc.org/doc/command-reference/exp/diff`). citeturn0search6turn2search11turn0search2

**Why this matters for entity["organization","VectorGrid","tauri quant desktop app"]**  
VectorGrid already persists local artifacts and emphasizes transparency and local-first workflows; adding formal comparison and lineage reduces “research friction” (what changed?), increases user trust (what actually ran?), and improves “promotion discipline” (why is this candidate chosen?) without changing the underlying optimization/validation semantics. Internally, the repo already treats artifacts as canonical cross-workflow handoff objects and enriches them with environment metadata and dataset fingerprints “when possible,” which is a strong foundation to build an index/lineage layer on top of, rather than replacing anything. (`/mnt/data/REPO_MAP.md`, lines 1213–1222).  

**What comparable tools already do**  
- **entity["organization","MLflow","open source experiment tracking"]**: organizes work into runs and experiments; logs parameters, metrics, artifacts, and code version context; supports nested runs with parent IDs; supports dataset tracking with digests and source lineage. (`https://mlflow.org/docs/latest/ml/tracking/`, `https://mlflow.org/docs/latest/ml/tracking/tracking-api/`, `https://mlflow.org/docs/latest/ml/dataset/`). citeturn4search1turn0search5turn4search4  
- **entity["company","Weights & Biases","mlops platform"]**: exposes lineage graphs that visualize inputs/outputs of runs and represent artifact–run relationships as DAGs, emphasizing reproducibility and auditability. (`https://docs.wandb.ai/models/registry/lineage`, `https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`). citeturn2search8turn2search11  
- **entity["company","DVC","data version control"]**: makes comparison operational with diff commands (`exp diff`, `metrics diff`) and experiment tables that can hide unchanged fields; parent relationships are also surfaced in experiment views. (`https://dvc.org/doc/command-reference/exp/diff`, `https://dvc.org/doc/command-reference/exp/show`). citeturn0search2turn4search3  
- **entity["organization","OpenLineage","open standard lineage metadata"]**: standardizes lineage around run/job/dataset entities and extensible facets, providing a vocabulary for capturing lineage metadata that can scale as needs grow. (`https://openlineage.io/docs/1.26.0/`, `https://openlineage.io/docs/spec/facets/`). citeturn0search9turn4search0  
- **entity["company","QuantConnect","algorithmic trading platform"]** (LEAN local platform): stores run results to timestamped directories and includes raw results, logs, and code snapshots, supporting local audit and reproducibility. (`https://www.quantconnect.com/docs/v2/lean-cli/api-reference/lean-backtest`, `https://www.quantconnect.com/docs/v2/local-platform/backtesting/results`). citeturn1search0turn1search1  
- **entity["organization","VectorBT PRO","quant research library"]** (as parity reference): emphasizes high-dimensional parameter exploration and caching; provides hash-based data caching and chunk caching; supports saving/loading computed objects, which helps researchers iterate and compare without rebuilding everything. (`https://vectorbt.pro/features/data/`, `https://vectorbt.pro/features/performance/`, `https://vectorbt.pro/features/productivity/`). citeturn3search1turn3search2turn3search7  

**Best fit for VectorGrid**  
A VectorGrid-fit design should borrow **the shape** of these patterns, not their deployment model:

- Prefer an MLflow-like split between a **small metadata index** (SQLite is enough) and the **existing artifact store** (directories/files). MLflow explicitly distinguishes “backend store” (run metadata) from “artifact store” (files), and this maps cleanly to VectorGrid’s local-first posture. (`https://mlflow.org/docs/latest/ml/tracking/`). citeturn0search6  
- Use W&B-style DAG semantics, but with VectorGrid-specific nodes: *Dataset* → *Optimize Run* → *Candidate Set* → *Validation Run* → *Validation Evidence* → *Meta/Holdout*, so that “candidate-to-validation lineage” is navigable. (`https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`). citeturn2search11  
- Use DVC-style “diff-first” UX: show only changed request fields and key metric deltas by default, with expandable full tables. (`https://dvc.org/doc/command-reference/exp/diff`). citeturn0search2  
- Treat OpenLineage/PROV as **conceptual anchors** if needed for naming and metadata organization (run/job/dataset; entity/activity/agent). This can prevent schema drift without fully adopting a heavyweight standard. (`https://openlineage.io/docs/1.26.0/`, `https://www.w3.org/TR/2013/NOTE-prov-overview-20130430/`). citeturn0search9turn2search13  

This answer necessarily includes product judgment because the question is about “smallest additive set” under a guardrailed desktop constraint; those constraints and UX tradeoffs are not fully determined by external precedent. (That is, the “what” is externally grounded; parts of the “how far” are repo-fit judgment.)

**Recommendation**  
Implement Now.

**Change shape**  
Additive (new metadata/index layer and UI surfaces; no workflow removal).

**Smallest useful implementation slice**  
Deliver a “Comparison + Lineage MVP” that consists of:

1. **Canonical run header + hashes** written into each run artifact’s manifest (or a sidecar) and also indexed:  
   - `request_digest`: hash of canonicalized request JSON  
   - `dataset_digest`: hash/fingerprint of dataset content/identity + derived lineage key(s)  
   - `env_digest`: build flags + app version + OS + hardware summary (CPU/GPU)  
   Rationale: dataset digests and run context are explicitly core to MLflow dataset tracking and run metadata. (`https://mlflow.org/docs/latest/ml/dataset/`, `https://mlflow.org/docs/latest/ml/tracking/`). citeturn4search4turn4search1  

2. **Parent/child links**:
   - `parent_run_id` on validation/meta/holdout runs that were spawned or derived from an optimization run  
   - optional: `root_run_id` for whole “experiment trees”  
   Rationale: nested runs are a proven holistic pattern for sweep→subrun organization. (`https://mlflow.org/docs/latest/ml/tracking/tracking-api/`). citeturn0search5  

3. **Candidate identity and linkage**:
   - define a stable `candidate_id` (e.g., hash of normalized strategy kind + parameters + timeframe + dataset scope)  
   - optimization artifacts output candidate sets containing `candidate_id`s  
   - validation artifacts record the `candidate_id` they evaluated and the upstream `optimize_run_id`  
   Rationale (inference): this is the direct translation of W&B “artifact input/output DAGs” to VectorGrid’s “candidate→validation evidence” world. The external precedent is the DAG concept; the candidate-ID composition is repo-specific and must be validated against existing candidate key/serialization rules. (`https://docs.wandb.ai/models/registry/lineage`). citeturn2search8  

4. **Two-run compare view (diff-first)**:
   - request diff table (only changed fields default)  
   - dataset/environment comparability badges (see below)  
   - metric delta table for a curated set of headline stats  
   Rationale: DVC demonstrates that diffing both metrics and parameters is an expected primitive; “only changed” is especially important at scale. (`https://dvc.org/doc/command-reference/exp/diff`, `https://dvc.org/doc/command-reference/exp/show`). citeturn0search2turn4search3  

5. **Comparability classification** (operator-visible)  
   Compute and display a deterministic comparability label, e.g.:  
   - “Exact comparable” (same request_digest + dataset_digest + env_digest)  
   - “Same request, different data” (same request_digest, different dataset_digest)  
   - “Same data, different request”  
   - “Different build/environment”  
   Rationale: QuantConnect LEAN’s result folders and MLflow’s tracking emphasize keeping enough context to interpret rerun differences. (`https://www.quantconnect.com/docs/v2/local-platform/backtesting/results`, `https://mlflow.org/docs/latest/ml/tracking/`). citeturn1search1turn4search1  

A minimal lineage view can be text-first (parents/children list) before implementing a full interactive graph; W&B shows the value of the graph, but VectorGrid can ship value earlier with a simple lineage panel. (`https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`). citeturn2search11  

**Verification and benchmark plan**  
This is partly repo-specific (contracts, artifact compatibility, and performance overhead), so verification should be anchored to existing repo gates and parity tooling:

- Add unit tests for canonicalization and hashing stability (same logical request ⇒ same digest; semantically different request ⇒ different digest).  
- Add artifact-compatibility tests: older artifacts should still load; new fields should be optional and versioned.  
- Run the existing repo quality gate and output parity tooling so report/artifact shape remains stable: the repo’s `scripts/run_output_parity_check.mjs` is described as the canonical deterministic headless parity regeneration plus structural checking. (`/mnt/data/REPO_MAP.md`, lines 1385–1388).  
- Performance check: ensure hashing/index writes are negligible relative to run time; if not, compute hashes at save-time, not during hot-path compute. This depends on local profiling and should be stated as such.

**Main risks or counterarguments**  
- **Scope creep risk**: “run intelligence” can balloon into a full experiment platform. Mitigation: treat the index as a thin layer over artifacts; do not introduce arbitrary scripting.  
- **Metadata drift**: if request schemas evolve, diffs become misleading. Mitigation: canonicalization must be versioned, and the diff UI must show schema version and unknown fields.  
- **UX complexity**: compare + lineage can become a dense UI. Mitigation: diff-first defaults (only changed), and progressive disclosure (expand for details), mirroring DVC’s patterns. (`https://dvc.org/doc/command-reference/exp/show`). citeturn4search3  

**Open questions**  
- What is the canonical “candidate identity” in current VectorGrid artifacts (what fields truly define a candidate)? This is a repo-local question that must be answered by inspecting current artifact schemas and candidate key structures, not by external precedent.  
- Which comparability fields must be treated as “hard breaks” (e.g., dataset_digest changes) vs “soft breaks” (e.g., UI-only metadata)?

### Suggested lineage model

```mermaid
flowchart LR
  D[Dataset\n(base + derived lineage)] --> O[Optimize Run]
  O --> CS[Candidate Set\n(candidate_id list)]
  CS --> V[Validation Run]
  V --> VE[Validation Evidence\n(metrics + artifacts)]
  VE --> M[Meta / Holdout Selection]
  O -->|parent_run_id| V
  V -->|parent_run_id| M
```

This mirrors the core idea common to MLflow/W&B/OpenLineage: *runs produce outputs and consume inputs*, and parent/child relationships make multi-stage workflows navigable. (`https://mlflow.org/docs/latest/ml/tracking/tracking-api/`, `https://docs.wandb.ai/models/registry/lineage`, `https://openlineage.io/docs/1.26.0/`). citeturn0search5turn2search8turn0search9  

## Implementation-oriented details that keep the scope bounded

To keep the result “VectorBT-style ergonomic” without becoming a notebook runtime, the system should remain declarative and artifact-centric:

- **Index derived from artifacts**: treat the SQLite index as a fast query layer; the artifact directory remains source-of-truth. This matches MLflow’s “backend store + artifact store” separation while preserving local-first file ownership. (`https://mlflow.org/docs/latest/ml/tracking/`). citeturn0search6  
- **Diff is structured, not textual**: request diff should be JSON-path based, stable-order, and schema-versioned. DVC’s ability to compare metrics/params is effective because it’s structured. (`https://dvc.org/doc/command-reference/exp/diff`). citeturn0search2  
- **Lineage is explicit**: do not infer parents from filenames; store `parent_run_id` and `candidate_id` edges, as MLflow does for nested runs and W&B does for artifact graphs. (`https://mlflow.org/docs/latest/ml/tracking/tracking-api/`, `https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`). citeturn0search5turn2search11  
- **Rerun comparability is a first-class badge**: this prevents users from over-trusting results that look “the same” but are not comparable. The precedent is the pervasive emphasis on capturing enough context (MLflow run context; LEAN result directories with code/config/logs). (`https://mlflow.org/docs/latest/ml/tracking/`, `https://www.quantconnect.com/docs/v2/local-platform/backtesting/results`). citeturn4search1turn1search1  

## Primary sources prioritized

Official/primary references used most heavily in this answer (URLs shown as inline code):

- `https://mlflow.org/docs/latest/ml/tracking/` citeturn4search1  
- `https://mlflow.org/docs/latest/ml/tracking/tracking-api/` citeturn0search5  
- `https://mlflow.org/docs/latest/ml/dataset/` citeturn4search4  
- `https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph` citeturn2search11  
- `https://docs.wandb.ai/models/registry/lineage` citeturn2search8  
- `https://dvc.org/doc/command-reference/exp/diff` citeturn0search2  
- `https://dvc.org/doc/command-reference/exp/show` citeturn4search3  
- `https://openlineage.io/docs/1.26.0/` citeturn0search9  
- `https://openlineage.io/docs/spec/facets/` citeturn4search0  
- `https://www.quantconnect.com/docs/v2/lean-cli/api-reference/lean-backtest` citeturn1search0  
- `https://www.quantconnect.com/docs/v2/local-platform/backtesting/results` citeturn1search1  
- `https://vectorbt.pro/features/data/` citeturn3search1  
- `https://vectorbt.pro/features/performance/` citeturn3search2  
- `https://vectorbt.pro/features/productivity/` citeturn3search7  
- `https://www.w3.org/TR/2013/NOTE-prov-overview-20130430/` citeturn2search13  

## Suggested follow-up research tasks

1. Extract and document the **current run artifact schema(s)** (optimize/validate/meta/holdout) and define the minimal additive fields required for `request_digest`, `dataset_digest`, `env_digest`, `parent_run_id`, and `candidate_id` mapping; verify backward compatibility by loading older artifacts.

2. Prototype the **request canonicalization + diff algorithm** on real stored requests (especially those with defaulted/normalized fields) and measure false positives/negatives (diff shows changes that aren’t semantically meaningful, or misses meaningful changes). Use DVC-style “only changed” as the UX baseline. (`https://dvc.org/doc/command-reference/exp/show`). citeturn4search3  

3. Design-test a minimal **compare + lineage UI** with progressive disclosure: two-run diff, comparability badge, and a text-first lineage panel; only after that, decide whether a full interactive DAG view is worth the complexity (W&B shows DAG value, but it isn’t required for the MVP). (`https://docs.wandb.ai/models/artifacts/explore-and-traverse-an-artifact-graph`). citeturn2search11