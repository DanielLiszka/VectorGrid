# OX-06 Tail-risk, drawdown-path, and rare-event math

- package type: `overlay`
- copied context files: `5`
- missing context files: `0`

## Overlay Scope

### `OX-06` Tail-risk, drawdown-path, and rare-event math

Best attached to:

- `PR-01`
- `PR-06`
- `PR-09`

Most relevant repo surfaces:

- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`

What this overlay asks:

- emphasize path-dependent tail behavior rather than only mean or Sharpe-like
  summaries
- identify whether drawdown path shape, CVaR-style measures, or rare-event
  proxies would improve ranking or validation
- look for cheap retained summaries that capture tail information without full
  replay

Overlay notation seed:

- for equity curve `E_t`, define drawdown
  `D_t = 1 - E_t / \max_{u \le t} E_u`
  and rare-event/tail summaries such as
  `\mathrm{CVaR}_\alpha(L) = \mathbb{E}[L \mid L \ge \mathrm{VaR}_\alpha(L)]`

Overlay prompt:

Add a tail-risk and rare-event lens to this review. Focus on whether the owned
problem is under-measuring path-dependent downside or rare-event exposure, and
whether there are low-marginal-cost summaries that would capture the missing
information more faithfully.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-06` Tail-risk, drawdown-path, and rare-event math

Best attached to:

- `PR-01`
- `PR-06`
- `PR-09`

Most relevant repo surfaces:

- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`

What this overlay asks:

- emphasize path-dependent tail behavior rather than only mean or Sharpe-like
  summaries
- identify whether drawdown path shape, CVaR-style measures, or rare-event
  proxies would improve ranking or validation
- look for cheap retained summaries that capture tail information without full
  replay

Overlay notation seed:

- for equity curve `E_t`, define drawdown
  `D_t = 1 - E_t / \max_{u \le t} E_u`
  and rare-event/tail summaries such as
  `\mathrm{CVaR}_\alpha(L) = \mathbb{E}[L \mid L \ge \mathrm{VaR}_\alpha(L)]`

Overlay prompt:

Add a tail-risk and rare-event lens to this review. Focus on whether the owned
problem is under-measuring path-dependent downside or rare-event exposure, and
whether there are low-marginal-cost summaries that would capture the missing
information more faithfully.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-06` Tail-risk, drawdown-path, and rare-event math

Best attached to:

- `PR-01`
- `PR-06`
- `PR-09`

Most relevant repo surfaces:

- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`

What this overlay asks:

- emphasize path-dependent tail behavior rather than only mean or Sharpe-like
  summaries
- identify whether drawdown path shape, CVaR-style measures, or rare-event
  proxies would improve ranking or validation
- look for cheap retained summaries that capture tail information without full
  replay

Overlay notation seed:

- for equity curve `E_t`, define drawdown
  `D_t = 1 - E_t / \max_{u \le t} E_u`
  and rare-event/tail summaries such as
  `\mathrm{CVaR}_\alpha(L) = \mathbb{E}[L \mid L \ge \mathrm{VaR}_\alpha(L)]`

Overlay prompt:

Add a tail-risk and rare-event lens to this review. Focus on whether the owned
problem is under-measuring path-dependent downside or rare-event exposure, and
whether there are low-marginal-cost summaries that would capture the missing
information more faithfully.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-06` Tail-risk, drawdown-path, and rare-event math

Best attached to:

- `PR-01`
- `PR-06`
- `PR-09`

Most relevant repo surfaces:

- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`

What this overlay asks:

- emphasize path-dependent tail behavior rather than only mean or Sharpe-like
  summaries
- identify whether drawdown path shape, CVaR-style measures, or rare-event
  proxies would improve ranking or validation
- look for cheap retained summaries that capture tail information without full
  replay

Overlay notation seed:

- for equity curve `E_t`, define drawdown
  `D_t = 1 - E_t / \max_{u \le t} E_u`
  and rare-event/tail summaries such as
  `\mathrm{CVaR}_\alpha(L) = \mathbb{E}[L \mid L \ge \mathrm{VaR}_\alpha(L)]`

Overlay prompt:

Add a tail-risk and rare-event lens to this review. Focus on whether the owned
problem is under-measuring path-dependent downside or rare-event exposure, and
whether there are low-marginal-cost summaries that would capture the missing
information more faithfully.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/options_expression_gpu.rs`
- `crates/app_core/src/validation_pipeline.rs`
