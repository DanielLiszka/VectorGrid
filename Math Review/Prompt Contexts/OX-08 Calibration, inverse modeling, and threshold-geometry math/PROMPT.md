# OX-08 Calibration, inverse modeling, and threshold-geometry math

- package type: `overlay`
- copied context files: `4`
- missing context files: `0`

## Overlay Scope

### `OX-08` Calibration, inverse modeling, and threshold-geometry math

Best attached to:

- `PR-02`
- `PR-07`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- inspect whether current thresholds are arbitrary constants standing in for an
  unmodeled inverse problem
- review calibration, monotone transforms, and threshold geometry
- identify whether current decision boundaries could be made more stable through
  explicit inversion or calibration logic

Overlay notation seed:

- let a calibrated map satisfy `\hat p = g(s)` for raw score `s`, with decision
  rule `a = \mathbf{1}\{\hat p \ge \tau\}` or a more general threshold geometry
  over policy parameters

Overlay prompt:

Add a calibration and threshold-geometry lens to this review. For the owned
problem, identify where hard thresholds or nonlinear transforms behave like
poorly calibrated inverse mappings. Propose repo-fit calibration or
reparameterization ideas that would make decisions more stable and interpretable.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-08` Calibration, inverse modeling, and threshold-geometry math

Best attached to:

- `PR-02`
- `PR-07`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- inspect whether current thresholds are arbitrary constants standing in for an
  unmodeled inverse problem
- review calibration, monotone transforms, and threshold geometry
- identify whether current decision boundaries could be made more stable through
  explicit inversion or calibration logic

Overlay notation seed:

- let a calibrated map satisfy `\hat p = g(s)` for raw score `s`, with decision
  rule `a = \mathbf{1}\{\hat p \ge \tau\}` or a more general threshold geometry
  over policy parameters

Overlay prompt:

Add a calibration and threshold-geometry lens to this review. For the owned
problem, identify where hard thresholds or nonlinear transforms behave like
poorly calibrated inverse mappings. Propose repo-fit calibration or
reparameterization ideas that would make decisions more stable and interpretable.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-08` Calibration, inverse modeling, and threshold-geometry math

Best attached to:

- `PR-02`
- `PR-07`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- inspect whether current thresholds are arbitrary constants standing in for an
  unmodeled inverse problem
- review calibration, monotone transforms, and threshold geometry
- identify whether current decision boundaries could be made more stable through
  explicit inversion or calibration logic

Overlay notation seed:

- let a calibrated map satisfy `\hat p = g(s)` for raw score `s`, with decision
  rule `a = \mathbf{1}\{\hat p \ge \tau\}` or a more general threshold geometry
  over policy parameters

Overlay prompt:

Add a calibration and threshold-geometry lens to this review. For the owned
problem, identify where hard thresholds or nonlinear transforms behave like
poorly calibrated inverse mappings. Propose repo-fit calibration or
reparameterization ideas that would make decisions more stable and interpretable.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-08` Calibration, inverse modeling, and threshold-geometry math

Best attached to:

- `PR-02`
- `PR-07`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/validation_stats.rs`
- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- inspect whether current thresholds are arbitrary constants standing in for an
  unmodeled inverse problem
- review calibration, monotone transforms, and threshold geometry
- identify whether current decision boundaries could be made more stable through
  explicit inversion or calibration logic

Overlay notation seed:

- let a calibrated map satisfy `\hat p = g(s)` for raw score `s`, with decision
  rule `a = \mathbf{1}\{\hat p \ge \tau\}` or a more general threshold geometry
  over policy parameters

Overlay prompt:

Add a calibration and threshold-geometry lens to this review. For the owned
problem, identify where hard thresholds or nonlinear transforms behave like
poorly calibrated inverse mappings. Propose repo-fit calibration or
reparameterization ideas that would make decisions more stable and interpretable.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/validation_stats.rs`
