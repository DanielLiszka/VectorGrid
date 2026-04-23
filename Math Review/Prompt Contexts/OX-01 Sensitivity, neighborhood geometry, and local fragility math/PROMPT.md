# OX-01 Sensitivity, neighborhood geometry, and local fragility math

- package type: `overlay`
- copied context files: `6`
- missing context files: `0`

## Overlay Scope

### `OX-01` Sensitivity, neighborhood geometry, and local fragility math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-06`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- identify parameter directions with high local sensitivity
- estimate flat versus sharp optima, plateau width, and local second-difference
  or neighborhood fragility
- identify condition-number-like fragility where small input changes cause large
  score or verdict changes
- propose mathematically cheaper stability summaries than brute-force local
  re-evaluation when possible

Overlay notation seed:

- for a discrete score surface `f : G \to \mathbb{R}`, define first differences
  `\Delta_i f(g) = f(g + e_i) - f(g)` and local second differences
  `\Delta_i^2 f(g) = f(g + e_i) - 2 f(g) + f(g - e_i)`

Overlay prompt:

Add a sensitivity and neighborhood-geometry lens to this review. For the owned
problem, identify whether the current formulas, ranking surfaces, or thresholds
behave like flat plateaus, sharp ridges, unstable cliffs, or ill-conditioned
decision boundaries on the discrete search lattice. If possible, describe
tractable finite-difference, local second-difference, or neighborhood fragility
metrics that could be computed cheaply enough to guide ranking, validation, or
pruning.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-01` Sensitivity, neighborhood geometry, and local fragility math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-06`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- identify parameter directions with high local sensitivity
- estimate flat versus sharp optima, plateau width, and local second-difference
  or neighborhood fragility
- identify condition-number-like fragility where small input changes cause large
  score or verdict changes
- propose mathematically cheaper stability summaries than brute-force local
  re-evaluation when possible

Overlay notation seed:

- for a discrete score surface `f : G \to \mathbb{R}`, define first differences
  `\Delta_i f(g) = f(g + e_i) - f(g)` and local second differences
  `\Delta_i^2 f(g) = f(g + e_i) - 2 f(g) + f(g - e_i)`

Overlay prompt:

Add a sensitivity and neighborhood-geometry lens to this review. For the owned
problem, identify whether the current formulas, ranking surfaces, or thresholds
behave like flat plateaus, sharp ridges, unstable cliffs, or ill-conditioned
decision boundaries on the discrete search lattice. If possible, describe
tractable finite-difference, local second-difference, or neighborhood fragility
metrics that could be computed cheaply enough to guide ranking, validation, or
pruning.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-01` Sensitivity, neighborhood geometry, and local fragility math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-06`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- identify parameter directions with high local sensitivity
- estimate flat versus sharp optima, plateau width, and local second-difference
  or neighborhood fragility
- identify condition-number-like fragility where small input changes cause large
  score or verdict changes
- propose mathematically cheaper stability summaries than brute-force local
  re-evaluation when possible

Overlay notation seed:

- for a discrete score surface `f : G \to \mathbb{R}`, define first differences
  `\Delta_i f(g) = f(g + e_i) - f(g)` and local second differences
  `\Delta_i^2 f(g) = f(g + e_i) - 2 f(g) + f(g - e_i)`

Overlay prompt:

Add a sensitivity and neighborhood-geometry lens to this review. For the owned
problem, identify whether the current formulas, ranking surfaces, or thresholds
behave like flat plateaus, sharp ridges, unstable cliffs, or ill-conditioned
decision boundaries on the discrete search lattice. If possible, describe
tractable finite-difference, local second-difference, or neighborhood fragility
metrics that could be computed cheaply enough to guide ranking, validation, or
pruning.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-01` Sensitivity, neighborhood geometry, and local fragility math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-06`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- identify parameter directions with high local sensitivity
- estimate flat versus sharp optima, plateau width, and local second-difference
  or neighborhood fragility
- identify condition-number-like fragility where small input changes cause large
  score or verdict changes
- propose mathematically cheaper stability summaries than brute-force local
  re-evaluation when possible

Overlay notation seed:

- for a discrete score surface `f : G \to \mathbb{R}`, define first differences
  `\Delta_i f(g) = f(g + e_i) - f(g)` and local second differences
  `\Delta_i^2 f(g) = f(g + e_i) - 2 f(g) + f(g - e_i)`

Overlay prompt:

Add a sensitivity and neighborhood-geometry lens to this review. For the owned
problem, identify whether the current formulas, ranking surfaces, or thresholds
behave like flat plateaus, sharp ridges, unstable cliffs, or ill-conditioned
decision boundaries on the discrete search lattice. If possible, describe
tractable finite-difference, local second-difference, or neighborhood fragility
metrics that could be computed cheaply enough to guide ranking, validation, or
pruning.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/composite_cpu.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/validation_pipeline.rs`
