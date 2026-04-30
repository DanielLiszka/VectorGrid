# Validation And Evidence

Validation adds evidence after optimization. It helps users avoid relying only on one in-sample ranking result.

Validation does not prove that a strategy is safe, profitable, or deployable.

## When To Run Validation

Run validation after:

- loading a dataset;
- completing optimization;
- selecting a candidate or shortlist;
- reviewing obvious data-quality issues.

Validation is especially important when:

- the optimization grid is large;
- the best metric is unusually high;
- the dataset is short;
- the candidate is sensitive to a narrow parameter range;
- the strategy could be overfit;
- you plan to compare or share the result.

## Simple Mode Validation

Simple Mode presents validation as a recommendation state:

- blocked;
- recommended;
- passed;
- review;
- rejected.

The state summarizes the available evidence. Open Advanced Mode or reports when you need the full detail.

## Advanced Validation Modules

Advanced validation surfaces can include:

- fixed-candidate validation;
- walk-forward validation;
- reoptimization walk-forward validation;
- CSCV/PBO style probability evidence;
- SPA-style checks;
- perturbation checks;
- trade bootstrap checks;
- cross-dataset or cross-ticker reruns;
- hard-gate filtering;
- probability gate filtering;
- route metadata and warnings.

Not every strategy, backend, and candidate type supports every module. The UI and route notes should name unsupported modules or fallback routes.

## Holdout And Walk-Forward Evidence

Holdout-style evidence tests how a candidate behaves outside the exact optimization slice.

Walk-forward evidence splits the dataset into sequential windows. It is useful for seeing whether a candidate remains viable across time rather than only in one aggregate period.

Reoptimization walk-forward goes further by allowing optimization to repeat across windows, then evaluating the resulting behavior. This is more expensive and more complex to interpret.

## CSCV/PBO

CSCV/PBO-style evidence is designed to estimate overfit risk under combinatorially generated train/test partitions.

Use it as a warning signal, not a final verdict. It depends on how slices are built, how many observations exist, and whether the selected strategy/result surface supports the module.

## SPA, Perturbation, And Trade Bootstrap

SPA-style checks, perturbation checks, and trade bootstrap checks add robustness evidence from different angles:

- SPA-style checks help evaluate whether apparent performance could be data-mined.
- Perturbation checks help show whether small changes meaningfully alter the result.
- Trade bootstrap checks help inspect behavior under resampled trade sequences.

These checks can be useful, but they are still historical simulations.

## Cross-Dataset Checks

Cross-dataset validation reruns selected candidates against other datasets when available.

Use cross-dataset checks to ask:

- Does the candidate survive on related symbols?
- Does the result depend on one market?
- Does a strategy fail immediately when market regime changes?

Cross-dataset evidence is only meaningful when the comparison datasets are relevant and clean.

## GPU Validation

Some validation paths can use GPU execution. Others intentionally fall back to CPU or use hybrid routes.

The result should be read with its route metadata:

- `CpuOnly` means CPU validation.
- `GpuKernel` means a supported GPU kernel route.
- `GpuHybridKernel` means a hybrid route.
- route notes may explain unsupported modules, fallback behavior, or CPU-preferred paths.

Do not describe a validation result as GPU-executed unless the run evidence says so.

## Warnings Matter

Warnings are part of the evidence. Common warning categories include:

- missing or invalid input data;
- unsupported route/module combination;
- CPU fallback;
- insufficient observations;
- duplicate or missing timestamp coverage;
- score-cache provenance concerns;
- validation recommendation requiring review.

Do not remove warnings when sharing a result. They help others interpret the evidence boundary.

## How To Read A Validation Result

A good review process asks:

1. Was the input dataset clean and relevant?
2. Was the candidate selected before validation?
3. Which validation modules actually ran?
4. Which modules were skipped or unsupported?
5. Did any route fall back from GPU to CPU?
6. Did the result pass hard gates?
7. Did evidence agree across windows or datasets?
8. Are warnings acceptable for the intended research question?
9. Does the report include enough provenance to reproduce or reopen the result?

