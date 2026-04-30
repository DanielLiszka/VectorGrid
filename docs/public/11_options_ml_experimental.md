# Options, ML, And Experimental Workflows

VectorGrid includes research workflows beyond the primary Simple Mode optimization path. These surfaces are useful, but they require stricter interpretation.

## Public Boundary

Options-expression and ML-signal workflows should be documented as research workflows unless a release explicitly promotes a narrower supported slice.

Do not present them as turnkey trading, forecasting, or production deployment systems.

## ML Signal Workflow

The ML Signal workflow is a CPU research path. It is designed around:

- exporting feature datasets;
- importing external scores;
- building score caches;
- validating signal policies;
- recording score-cache metadata and evidence;
- distinguishing research cache behavior from stricter release-style score-cache contracts.

ML Signal does not use the normal global optimization-mode selector. It emits dataset-export, score-cache, validation, manifest, and artifact evidence rather than normal optimizer heatmaps.

## ML Signal Score Caches

Score caches should be reviewed for:

- timestamp coverage;
- duplicate timestamps;
- score scope;
- dataset fingerprint;
- schema or bundle metadata;
- threshold policy;
- overlap metadata where walk-forward scores are involved;
- warnings.

Research-mode score caches may be useful for exploration. Release-strict score-cache behavior is more demanding and should reject incomplete provenance.

## Options Expression Workflow

The options-expression workflow is a follow-up research lane. It is scoped around supported parent artifacts and Polygon.io options data.

Important boundaries:

- Polygon.io is the only supported options data provider.
- Direct provider download is currently minute-aggregate oriented.
- Saved options artifacts use provenance guards.
- Revalidation depends on matching parent artifact identity, signal manifest identity, dataset identity, price semantics, and dataset provenance.
- Broader option-chain, quote-lane, and per-window roadmap items should be described as future scope unless implemented in the release.

## Options Validation

Options validation can include:

- retained-tuple original-split replay;
- session-chronological replay;
- date-range holdout;
- anchored or rolling walk-forward union-holdout plans where available;
- artifact-scoped compare/reopen behavior.

Users should review quote quality, provider coverage, time alignment, and artifact provenance before trusting any options result.

## Experimental Does Not Mean Unsupported Chaos

Experimental workflows should still:

- preserve local data privacy;
- show warnings;
- save enough provenance;
- avoid silently weakening validation;
- avoid presenting exploratory results as production decisions.

If a release includes these workflows, release notes should name the exact supported scope.

