# OX-07 Fill, slippage, and execution-policy math

- package type: `overlay`
- copied context files: `4`
- missing context files: `0`

## Overlay Scope

### `OX-07` Fill, slippage, and execution-policy math

Best attached to:

- `PR-08`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- treat execution and fill assumptions as controllable mathematical models
- identify where fixed-percentage or fixed-threshold behavior should instead be
  volatility-, liquidity-, or uncertainty-aware
- review execution-policy logic through a constrained decision lens where useful

Overlay notation seed:

- an execution distortion model can be written as
  `p_{\mathrm{fill}} = p_{\mathrm{mid}} + s(q,\sigma,\ell)`
  where `q` is size, `\sigma` is volatility, and `\ell` is a liquidity proxy

Overlay prompt:

Add a fill, slippage, and execution-policy lens to this review. Identify where
the owned problem uses fixed heuristics that are really implicit execution or
fill models, and assess whether more principled policy-style or
uncertainty-aware math would materially improve realism or ranking quality.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-07` Fill, slippage, and execution-policy math

Best attached to:

- `PR-08`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- treat execution and fill assumptions as controllable mathematical models
- identify where fixed-percentage or fixed-threshold behavior should instead be
  volatility-, liquidity-, or uncertainty-aware
- review execution-policy logic through a constrained decision lens where useful

Overlay notation seed:

- an execution distortion model can be written as
  `p_{\mathrm{fill}} = p_{\mathrm{mid}} + s(q,\sigma,\ell)`
  where `q` is size, `\sigma` is volatility, and `\ell` is a liquidity proxy

Overlay prompt:

Add a fill, slippage, and execution-policy lens to this review. Identify where
the owned problem uses fixed heuristics that are really implicit execution or
fill models, and assess whether more principled policy-style or
uncertainty-aware math would materially improve realism or ranking quality.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-07` Fill, slippage, and execution-policy math

Best attached to:

- `PR-08`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- treat execution and fill assumptions as controllable mathematical models
- identify where fixed-percentage or fixed-threshold behavior should instead be
  volatility-, liquidity-, or uncertainty-aware
- review execution-policy logic through a constrained decision lens where useful

Overlay notation seed:

- an execution distortion model can be written as
  `p_{\mathrm{fill}} = p_{\mathrm{mid}} + s(q,\sigma,\ell)`
  where `q` is size, `\sigma` is volatility, and `\ell` is a liquidity proxy

Overlay prompt:

Add a fill, slippage, and execution-policy lens to this review. Identify where
the owned problem uses fixed heuristics that are really implicit execution or
fill models, and assess whether more principled policy-style or
uncertainty-aware math would materially improve realism or ranking quality.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-07` Fill, slippage, and execution-policy math

Best attached to:

- `PR-08`
- `PR-09`
- `PR-10`

Most relevant repo surfaces:

- `src-tauri/src/commands_execution.rs`
- `crates/app_core/src/options_types.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/ml_signal.rs`

What this overlay asks:

- treat execution and fill assumptions as controllable mathematical models
- identify where fixed-percentage or fixed-threshold behavior should instead be
  volatility-, liquidity-, or uncertainty-aware
- review execution-policy logic through a constrained decision lens where useful

Overlay notation seed:

- an execution distortion model can be written as
  `p_{\mathrm{fill}} = p_{\mathrm{mid}} + s(q,\sigma,\ell)`
  where `q` is size, `\sigma` is volatility, and `\ell` is a liquidity proxy

Overlay prompt:

Add a fill, slippage, and execution-policy lens to this review. Identify where
the owned problem uses fixed heuristics that are really implicit execution or
fill models, and assess whether more principled policy-style or
uncertainty-aware math would materially improve realism or ranking quality.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_types.rs`
- `src-tauri/src/commands_execution.rs`
