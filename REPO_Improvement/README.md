# REPO Improvement

This folder is the working home for the non-options repo-wide improvement-document program.

Current seed documents:

- [REPO_IMPROVEMENT_META_PLAN_2026-04-08.md](./REPO_IMPROVEMENT_META_PLAN_2026-04-08.md)
- [IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md](./IMPROVEMENT_FEATURE_INVENTORY_2026-04-08.md)
- [IMPROVEMENT_MASTER_PLAN_2026-04-08.md](./IMPROVEMENT_MASTER_PLAN_2026-04-08.md)

Canonical planned-slot meta docs:

- [00_program_index_2026-04-09.md](./00_meta/00_program_index_2026-04-09.md)
- [01_doc_creation_tracker_2026-04-09.md](./00_meta/01_doc_creation_tracker_2026-04-09.md)

Historical status snapshots:

- [program_index_2026-04-08.md](./00_meta/program_index_2026-04-08.md)
- [doc_creation_tracker_2026-04-08.md](./00_meta/doc_creation_tracker_2026-04-08.md)

Current execution-status companion doc:

- [execution_ledger_2026-04-09.md](./00_meta/execution_ledger_2026-04-09.md)

Canonical governance ownership:

- `REPO_IMPROVEMENT_META_PLAN_2026-04-08.md`
  - owns non-options scope, folder catalog, and planned-slot intent
- `00_meta/00_program_index_2026-04-09.md`
  - owns current-state routing, wave summary, and “what comes next” posture
- `00_meta/01_doc_creation_tracker_2026-04-09.md`
  - owns folder counts, planned-slot coverage, and corpus-completion audit notes
- `00_meta/execution_ledger_2026-04-09.md`
  - owns implementation completion status for the plan set; it does not replace creation counts
- `00_meta/05_status_legend_2026-04-08.md`
  - owns canonical lifecycle/status terminology
- `00_meta/11_archive_policy_2026-04-08.md`
  - owns archive/supersede rules and historical-pointer expectations

Canonical governance docs:

- [05_status_legend_2026-04-08.md](./00_meta/05_status_legend_2026-04-08.md)
- [11_archive_policy_2026-04-08.md](./00_meta/11_archive_policy_2026-04-08.md)

Active scope:

- every current repo/project surface except anything options-related

Current catalog status:

- `356` planned docs in the active `v7` catalog
- planned-doc creation is now complete for the current catalog
- second-pass implementation-doc hardening is now complete for the current catalog
- `363` Markdown files now exist under `REPO_Improvement/`
- `356` unique planned-doc slots are represented on disk
- `343` unique non-meta implementation-plan slots are represented on disk
- `343` raw non-meta implementation-plan files exist on disk

Program maintenance rules:

- if planned slots are merged, split, superseded, or archived in a way that changes program accounting, update the meta plan, program index, tracker, and execution ledger together
- historical snapshots under `00_meta/` stay on disk with forward pointers instead of being silently replaced
- `Frontend Review/`, `research/`, `SASS research/`, `CUDA kernel review/`, and `scripts/baselines/` are retained as historical/reference trees; they count as current authority only when the active routing docs explicitly point to them
- release/build and verification authority remains repo-local first; archived GitHub Actions definitions under `.github/workflows.disabled/` are reference material, not active governance

Wave-close checklist:

- confirm file counts and folder totals still match the active catalog
- confirm the current tracker and program index agree on the same state
- confirm the status legend and archive policy still match any supersede/archive changes made in the wave
- confirm any new historical snapshots point forward to the current canonical replacement
- confirm the next-state note is explicit so another contributor can continue without reconstructing process from commit history

## Governance Contract

- `REPO_IMPROVEMENT_META_PLAN_2026-04-08.md` owns the active non-options scope boundary and the planned-slot catalog.
- `00_program_index_2026-04-09.md` owns the current program-state summary and current-wave routing posture.
- `01_doc_creation_tracker_2026-04-09.md` owns file counts, created-doc coverage, and section-complete corpus audits.
- `execution_ledger_2026-04-09.md` owns whole-document implementation completion only; it does not track mere file existence.
- `05_status_legend_2026-04-08.md` owns lifecycle/status meanings such as `created`, `section-complete`, `completed`, `archived`, and `superseded`.
- `11_archive_policy_2026-04-08.md` owns archive, supersede, and forward-pointer rules for this doc tree.

## Required Sync Rules

- update the meta plan, program index, tracker, and this README together when planned-slot counts or program-phase labels change
- update the tracker, program index, and archive policy references in the same change when docs are merged, split, superseded, or archived
- update the execution ledger only when a document-level implementation is actually completed and verified
- keep historical snapshot files visible, but point current entry docs at the canonical current tracker and program index

## Cross-Tree Archive Posture

- `REPO_Improvement/` is the active planning and execution tree for the non-options improvement program
- `Frontend Review/`, `research/`, `CUDA kernel review/`, and `SASS research/` are retained reference or historical trees; keep them linkable, but route contributors through current entry docs when a newer canonical surface exists
- `scripts/baselines/` is a retained evidence tree; keep active baseline/sample packs small, reviewable, and explicitly referenced rather than silently expanding it into an unindexed archive

## Wave-Close Checklist

Before calling a creation or maintenance wave closed:

1. reconcile planned-slot counts and actual on-disk counts
2. confirm the tracker and program index name the same current state
3. rerun the section-complete corpus audit
4. record the next queue or maintenance posture in the current canonical tracker/index surfaces

Second-pass hardening highlights:

- every implementation doc now carries explicit public-repo code-quality framing
- every implementation doc now includes stronger acceptance-evidence and checklist detail
- all missing title/metadata blocks in the implementation-doc corpus were restored
- all `_DATE.md` placeholder links in the implementation-doc corpus were resolved
- legacy `npm --prefix web test -- ...` verification command forms in the implementation-doc corpus were normalized

External routing anchor:

- [REPO_MAP.md](../REPO_MAP.md)

Planned folder expansion:

- `00_meta/`
- `01_shell/`
- `02_data/`
- `03_opt/`
- `04_results/`
- `05_validation/`
- `06_artifacts/`
- `07_strats/`
- `08_ml/`
- `09_live/`
- `10_tests_perf/`
- `11_docs_ops/`
