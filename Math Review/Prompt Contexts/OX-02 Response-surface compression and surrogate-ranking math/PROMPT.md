# OX-02 Response-surface compression and surrogate-ranking math

- package type: `overlay`
- copied context files: `4`
- missing context files: `0`

## Overlay Scope

### `OX-02` Response-surface compression and surrogate-ranking math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-07`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/meta_select.rs`

What this overlay asks:

- look for low-dimensional structure in high-dimensional score surfaces
- identify whether local response surfaces can be compressed, parameterized, or
  approximated without losing rank quality
- assess whether surrogate ranking, local interpolation, or lattice-aware
  summaries could reduce compute

Overlay notation seed:

- seek a reduced model `\tilde f(g) = \sum_{k=1}^r \alpha_k \phi_k(g)` or other
  compressed surrogate such that rank error
  `\Pr(\operatorname{sign}(f(g)-f(h)) \neq \operatorname{sign}(\tilde f(g)-\tilde f(h)))`
  stays acceptably small on the candidate lattice

Overlay prompt:

Add a response-surface compression lens to this review. Determine whether the
owned problem has exploitable low-dimensional structure, locally regular lattice
behavior, or surrogate-friendly ranking behavior. Focus on repo-fit mathematical
reductions that could cut candidate evaluations or improve ranking quality
without replacing exact final scoring with a generic black-box optimizer.

## First-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` first-pass prompt. Do not change the core prompt's ownership, non-goals, or file boundaries.

Use this overlay block exactly:

```md
### `OX-02` Response-surface compression and surrogate-ranking math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-07`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/meta_select.rs`

What this overlay asks:

- look for low-dimensional structure in high-dimensional score surfaces
- identify whether local response surfaces can be compressed, parameterized, or
  approximated without losing rank quality
- assess whether surrogate ranking, local interpolation, or lattice-aware
  summaries could reduce compute

Overlay notation seed:

- seek a reduced model `\tilde f(g) = \sum_{k=1}^r \alpha_k \phi_k(g)` or other
  compressed surrogate such that rank error
  `\Pr(\operatorname{sign}(f(g)-f(h)) \neq \operatorname{sign}(\tilde f(g)-\tilde f(h)))`
  stays acceptably small on the candidate lattice

Overlay prompt:

Add a response-surface compression lens to this review. Determine whether the
owned problem has exploitable low-dimensional structure, locally regular lattice
behavior, or surrogate-friendly ranking behavior. Focus on repo-fit mathematical
reductions that could cut candidate evaluations or improve ranking quality
without replacing exact final scoring with a generic black-box optimizer.
```

Overlay theorem obligations: require at least one new lemma candidate, one new counterexample family, or one new bound that the core prompt would otherwise miss.

## Second-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` second-pass prompt. Keep the core prompt's ownership and comparison structure fixed.

Use this overlay block exactly:

```md
### `OX-02` Response-surface compression and surrogate-ranking math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-07`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/meta_select.rs`

What this overlay asks:

- look for low-dimensional structure in high-dimensional score surfaces
- identify whether local response surfaces can be compressed, parameterized, or
  approximated without losing rank quality
- assess whether surrogate ranking, local interpolation, or lattice-aware
  summaries could reduce compute

Overlay notation seed:

- seek a reduced model `\tilde f(g) = \sum_{k=1}^r \alpha_k \phi_k(g)` or other
  compressed surrogate such that rank error
  `\Pr(\operatorname{sign}(f(g)-f(h)) \neq \operatorname{sign}(\tilde f(g)-\tilde f(h)))`
  stays acceptably small on the candidate lattice

Overlay prompt:

Add a response-surface compression lens to this review. Determine whether the
owned problem has exploitable low-dimensional structure, locally regular lattice
behavior, or surrogate-friendly ranking behavior. Focus on repo-fit mathematical
reductions that could cut candidate evaluations or improve ranking quality
without replacing exact final scoring with a generic black-box optimizer.
```

Overlay theorem obligations: require the response to show how the overlay changes exactness assumptions, bounds, or adversarial constructions without changing ownership.

## Third-Pass Overlay Prompt

Append this overlay to the end of the chosen core `PR-*` third-pass prompt. Keep implementation ownership inside the core prompt's boundary.

Use this overlay block exactly:

```md
### `OX-02` Response-surface compression and surrogate-ranking math

Best attached to:

- `PR-02`
- `PR-03`
- `PR-05`
- `PR-07`

Most relevant repo surfaces:

- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `crates/app_core/src/meta_select.rs`

What this overlay asks:

- look for low-dimensional structure in high-dimensional score surfaces
- identify whether local response surfaces can be compressed, parameterized, or
  approximated without losing rank quality
- assess whether surrogate ranking, local interpolation, or lattice-aware
  summaries could reduce compute

Overlay notation seed:

- seek a reduced model `\tilde f(g) = \sum_{k=1}^r \alpha_k \phi_k(g)` or other
  compressed surrogate such that rank error
  `\Pr(\operatorname{sign}(f(g)-f(h)) \neq \operatorname{sign}(\tilde f(g)-\tilde f(h)))`
  stays acceptably small on the candidate lattice

Overlay prompt:

Add a response-surface compression lens to this review. Determine whether the
owned problem has exploitable low-dimensional structure, locally regular lattice
behavior, or surrogate-friendly ranking behavior. Focus on repo-fit mathematical
reductions that could cut candidate evaluations or improve ranking quality
without replacing exact final scoring with a generic black-box optimizer.
```

Overlay theorem obligations: require implementation-facing implications of the new lemma, counterexample, or bound without redesigning adjacent PR ownership surfaces.

## Local Repo Copies

The copied files below are stored directly under this prompt folder, preserving repo-relative paths.

- `crates/app_core/src/composite_mo.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/meta_select.rs`
- `crates/optimizer/src/mo.rs`
