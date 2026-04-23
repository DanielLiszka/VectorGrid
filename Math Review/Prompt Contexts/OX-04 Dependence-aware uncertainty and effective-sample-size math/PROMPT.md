# OX-04 Dependence-aware uncertainty and effective-sample-size math

- package type: `overlay`
- copied context files: `5`
- missing context files: `0`

## Overlay Scope

### `OX-04` Dependence-aware uncertainty and effective-sample-size math

Best attached to:

- `PR-02`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- review independence assumptions that are likely false
- replace naive sample counts with effective sample size where possible
- consider autocorrelation, clustered outcomes, overlapping windows, and
  repeated use of the same bars

Overlay notation seed:

- a standard target is an effective sample size of the form
  `n_{\mathrm{eff}} = n \big/ \left(1 + 2 \sum_{k \ge 1} \rho_k\right)`
  or a cluster-adjusted analogue when iid assumptions fail

Overlay prompt:

Add a dependence-aware uncertainty lens to this review. Identify where the
owned problem implicitly assumes iid evidence, independent trades, or
independent windows when the repo structure likely violates that assumption.
Propose effective-sample-size or dependence-aware alternatives where they would
materially change decisions.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-04` Dependence-aware uncertainty and effective-sample-size math

Best attached to:

- `PR-02`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- review independence assumptions that are likely false
- replace naive sample counts with effective sample size where possible
- consider autocorrelation, clustered outcomes, overlapping windows, and
  repeated use of the same bars

Overlay notation seed:

- a standard target is an effective sample size of the form
  `n_{\mathrm{eff}} = n \big/ \left(1 + 2 \sum_{k \ge 1} \rho_k\right)`
  or a cluster-adjusted analogue when iid assumptions fail

Overlay prompt:

Add a dependence-aware uncertainty lens to this review. Identify where the
owned problem implicitly assumes iid evidence, independent trades, or
independent windows when the repo structure likely violates that assumption.
Propose effective-sample-size or dependence-aware alternatives where they would
materially change decisions.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-04` Dependence-aware uncertainty and effective-sample-size math

Best attached to:

- `PR-02`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- review independence assumptions that are likely false
- replace naive sample counts with effective sample size where possible
- consider autocorrelation, clustered outcomes, overlapping windows, and
  repeated use of the same bars

Overlay notation seed:

- a standard target is an effective sample size of the form
  `n_{\mathrm{eff}} = n \big/ \left(1 + 2 \sum_{k \ge 1} \rho_k\right)`
  or a cluster-adjusted analogue when iid assumptions fail

Overlay prompt:

Add a dependence-aware uncertainty lens to this review. Identify where the
owned problem implicitly assumes iid evidence, independent trades, or
independent windows when the repo structure likely violates that assumption.
Propose effective-sample-size or dependence-aware alternatives where they would
materially change decisions.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-04` Dependence-aware uncertainty and effective-sample-size math

Best attached to:

- `PR-02`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- review independence assumptions that are likely false
- replace naive sample counts with effective sample size where possible
- consider autocorrelation, clustered outcomes, overlapping windows, and
  repeated use of the same bars

Overlay notation seed:

- a standard target is an effective sample size of the form
  `n_{\mathrm{eff}} = n \big/ \left(1 + 2 \sum_{k \ge 1} \rho_k\right)`
  or a cluster-adjusted analogue when iid assumptions fail

Overlay prompt:

Add a dependence-aware uncertainty lens to this review. Identify where the
owned problem implicitly assumes iid evidence, independent trades, or
independent windows when the repo structure likely violates that assumption.
Propose effective-sample-size or dependence-aware alternatives where they would
materially change decisions.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/sharpe_stats.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/validation_stats.rs`
