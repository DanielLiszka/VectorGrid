# OX-05 Regime inference, changepoints, and latent-state math

- package type: `overlay`
- copied context files: `4`
- missing context files: `0`

## Overlay Scope

### `OX-05` Regime inference, changepoints, and latent-state math

Best attached to:

- `PR-03`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- assess whether current median splits or coarse buckets are standing in for
  latent-state structure
- consider changepoint or lightweight hidden-regime views where mathematically
  useful
- identify lightweight regime-inference ideas that fit the repo better than a
  heavyweight forecasting stack

Overlay notation seed:

- let changepoints satisfy
  `1 \le \tau_1 < \tau_2 < \cdots < \tau_m \le T`
  and let `r_t \in \{1,\dots,K\}` be a latent regime label when a hidden-state
  view is used

Overlay prompt:

Add a latent-state and changepoint lens to this review. Determine whether the
owned problem is currently using coarse heuristics where a more principled
regime or changepoint view would materially improve correctness, stability, or
compute allocation.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-05` Regime inference, changepoints, and latent-state math

Best attached to:

- `PR-03`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- assess whether current median splits or coarse buckets are standing in for
  latent-state structure
- consider changepoint or lightweight hidden-regime views where mathematically
  useful
- identify lightweight regime-inference ideas that fit the repo better than a
  heavyweight forecasting stack

Overlay notation seed:

- let changepoints satisfy
  `1 \le \tau_1 < \tau_2 < \cdots < \tau_m \le T`
  and let `r_t \in \{1,\dots,K\}` be a latent regime label when a hidden-state
  view is used

Overlay prompt:

Add a latent-state and changepoint lens to this review. Determine whether the
owned problem is currently using coarse heuristics where a more principled
regime or changepoint view would materially improve correctness, stability, or
compute allocation.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-05` Regime inference, changepoints, and latent-state math

Best attached to:

- `PR-03`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- assess whether current median splits or coarse buckets are standing in for
  latent-state structure
- consider changepoint or lightweight hidden-regime views where mathematically
  useful
- identify lightweight regime-inference ideas that fit the repo better than a
  heavyweight forecasting stack

Overlay notation seed:

- let changepoints satisfy
  `1 \le \tau_1 < \tau_2 < \cdots < \tau_m \le T`
  and let `r_t \in \{1,\dots,K\}` be a latent regime label when a hidden-state
  view is used

Overlay prompt:

Add a latent-state and changepoint lens to this review. Determine whether the
owned problem is currently using coarse heuristics where a more principled
regime or changepoint view would materially improve correctness, stability, or
compute allocation.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-05` Regime inference, changepoints, and latent-state math

Best attached to:

- `PR-03`
- `PR-06`
- `PR-07`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- assess whether current median splits or coarse buckets are standing in for
  latent-state structure
- consider changepoint or lightweight hidden-regime views where mathematically
  useful
- identify lightweight regime-inference ideas that fit the repo better than a
  heavyweight forecasting stack

Overlay notation seed:

- let changepoints satisfy
  `1 \le \tau_1 < \tau_2 < \cdots < \tau_m \le T`
  and let `r_t \in \{1,\dots,K\}` be a latent regime label when a hidden-state
  view is used

Overlay prompt:

Add a latent-state and changepoint lens to this review. Determine whether the
owned problem is currently using coarse heuristics where a more principled
regime or changepoint view would materially improve correctness, stability, or
compute allocation.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/composite_validate.rs`
- `crates/app_core/src/double_ma_validate.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/ml_signal.rs`
