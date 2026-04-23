# OX-03 Sequential compute allocation and stopping-value math

- package type: `overlay`
- copied context files: `5`
- missing context files: `0`

## Overlay Scope

### `OX-03` Sequential compute allocation and stopping-value math

Best attached to:

- `PR-03`
- `PR-05`
- `PR-06`
- `PR-07`
- `PR-08`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_campaign.rs`

What this overlay asks:

- treat compute as a scarce resource allocated across candidates or datasets
- assess early stopping, sequential testing, or value-of-information math
- identify whether partial evidence can justify terminating, expanding, or
  deferring evaluation

Overlay notation seed:

- let `B` be the total compute budget and `a_t` the stage-`t` allocation action
- seek a policy maximizing expected marginal value
  `\mathbb{E}[\Delta V_t(a_t)] / \Delta C_t(a_t)` subject to
  `\sum_t \Delta C_t(a_t) \le B`

Overlay prompt:

Add a sequential compute-allocation lens to this review. Treat the owned problem
as a decision under limited budget. Identify where partial evidence could be
used to stop early, continue selectively, or allocate more compute only where
expected information gain justifies it.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-03` Sequential compute allocation and stopping-value math

Best attached to:

- `PR-03`
- `PR-05`
- `PR-06`
- `PR-07`
- `PR-08`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_campaign.rs`

What this overlay asks:

- treat compute as a scarce resource allocated across candidates or datasets
- assess early stopping, sequential testing, or value-of-information math
- identify whether partial evidence can justify terminating, expanding, or
  deferring evaluation

Overlay notation seed:

- let `B` be the total compute budget and `a_t` the stage-`t` allocation action
- seek a policy maximizing expected marginal value
  `\mathbb{E}[\Delta V_t(a_t)] / \Delta C_t(a_t)` subject to
  `\sum_t \Delta C_t(a_t) \le B`

Overlay prompt:

Add a sequential compute-allocation lens to this review. Treat the owned problem
as a decision under limited budget. Identify where partial evidence could be
used to stop early, continue selectively, or allocate more compute only where
expected information gain justifies it.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-03` Sequential compute allocation and stopping-value math

Best attached to:

- `PR-03`
- `PR-05`
- `PR-06`
- `PR-07`
- `PR-08`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_campaign.rs`

What this overlay asks:

- treat compute as a scarce resource allocated across candidates or datasets
- assess early stopping, sequential testing, or value-of-information math
- identify whether partial evidence can justify terminating, expanding, or
  deferring evaluation

Overlay notation seed:

- let `B` be the total compute budget and `a_t` the stage-`t` allocation action
- seek a policy maximizing expected marginal value
  `\mathbb{E}[\Delta V_t(a_t)] / \Delta C_t(a_t)` subject to
  `\sum_t \Delta C_t(a_t) \le B`

Overlay prompt:

Add a sequential compute-allocation lens to this review. Treat the owned problem
as a decision under limited budget. Identify where partial evidence could be
used to stop early, continue selectively, or allocate more compute only where
expected information gain justifies it.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-03` Sequential compute allocation and stopping-value math

Best attached to:

- `PR-03`
- `PR-05`
- `PR-06`
- `PR-07`
- `PR-08`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/meta_select.rs`
- `src-tauri/src/commands_campaign.rs`

What this overlay asks:

- treat compute as a scarce resource allocated across candidates or datasets
- assess early stopping, sequential testing, or value-of-information math
- identify whether partial evidence can justify terminating, expanding, or
  deferring evaluation

Overlay notation seed:

- let `B` be the total compute budget and `a_t` the stage-`t` allocation action
- seek a policy maximizing expected marginal value
  `\mathbb{E}[\Delta V_t(a_t)] / \Delta C_t(a_t)` subject to
  `\sum_t \Delta C_t(a_t) \le B`

Overlay prompt:

Add a sequential compute-allocation lens to this review. Treat the owned problem
as a decision under limited budget. Identify where partial evidence could be
used to stop early, continue selectively, or allocate more compute only where
expected information gain justifies it.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/app_core/src/validation_pipeline.rs`
- `src-tauri/src/commands_campaign.rs`
