# Roadmap And Future Scope

This page describes direction, not commitments. A feature is public only when a signed release includes it and the release notes document its support boundary.

## Near-Term Public Priority

The highest-value public work is release hardening:

- signed Windows installer;
- clean-machine install smoke;
- Simple Mode first-run polish;
- clear release notes;
- stable support and troubleshooting docs;
- honest known limitations;
- consistent report/artifact behavior.

## Simple Mode Direction

Simple Mode should remain the default public experience.

Future improvements should make it easier to:

- load data;
- choose a small set of understandable strategy workflows;
- run optimization;
- run optional validation;
- understand warnings;
- export a useful summary.

Simple Mode should not become another advanced control panel.

## Data Direction

Useful future data work may include:

- clearer cache cleanup tools;
- stronger data-quality explanations;
- better provider-plan error handling;
- richer options data quality summaries;
- safer import flows for external datasets.

Adding a new provider should be treated as a product decision, not a small technical change.

## Validation Direction

Useful future validation work may include:

- clearer evidence-tier labels;
- stronger cross-dataset interpretation;
- more visible route/fallback summaries;
- better report explanations;
- more deterministic reproducibility evidence.

Validation improvements should preserve warnings and provenance rather than hiding complexity.

## GPU Direction

Useful future GPU work may include:

- expanding GPU validation where measurements justify it;
- improving GPU residency for supported workflows;
- profiling route-specific bottlenecks;
- keeping CPU fallback reliable;
- documenting when CPU is expected to be faster.

Tensor cores, ray-tracing cores, and highly experimental GPU ideas should remain research topics until there is a concrete workload and evidence.

## ML And Options Direction

Useful future research work may include:

- stricter score-cache provenance;
- clearer ML Signal evidence tiers;
- safer options dataset provenance;
- more transparent options validation interpretation;
- better artifact replay diagnostics.

These areas should not be marketed as production trading intelligence without substantial additional evidence.

## Live Execution Direction

Live execution should remain conservative.

Future work should prioritize:

- safer confirmation flows;
- clearer risk-policy previews;
- stronger evidence linking source data to actions;
- better failure/recovery documentation;
- paper-first workflows.

Broker-connected automation should never become the first-run path.

## Website Direction

The website should mature around:

- signed downloads;
- release integrity;
- first-run docs;
- transparent limitations;
- privacy and disclaimer clarity;
- support/reporting quality;
- benchmark context.

Avoid building a marketing site that overstates release readiness.

