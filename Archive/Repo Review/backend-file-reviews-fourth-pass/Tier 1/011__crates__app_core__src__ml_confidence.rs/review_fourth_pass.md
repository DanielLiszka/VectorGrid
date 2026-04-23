# Fourth-Pass Review: `crates/app_core/src/ml_confidence.rs`

- Review date: `2026-04-15`
- Source file: `crates/app_core/src/ml_confidence.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/ml_confidence.rs.review.md`
- Second-pass packet: `Repo Review/backend-file-reviews-second-pass/crates/app_core/src/ml_confidence.rs.review.md`
- Third-pass packet: `Repo Review/backend-file-reviews-third-pass/crates/app_core/src/ml_confidence.rs.review.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file merited a fourth pass because the third-pass packet was directionally right on trust semantics, but it still centered the wrong “first fix.” Re-reading the live source shows that the sharper near-term risk is public API contract safety, not model-family choice or CV strategy redesign.

`ml_confidence` is not an incidental helper hidden behind one command; it is explicitly re-exported from `crates/app_core/src/lib.rs:190-198`, including the prepared-row and raw-row training entrypoints. That makes avoidable panics, scope drift, and persistence-metadata ambiguity more important than they looked in the third-pass packet. In a public-facing backend repo, that is the kind of issue that should be hardened before a broader modeling rethink.

The third-pass bucket should **stand**. This is still a `Small surgical refactor`, but the safer first patch is narrower and more contract-focused than the current packet suggests.

## Fourth-Pass Research Log

1. Re-read the live `ml_confidence.rs` source with emphasis on the public DTOs and entrypoints (`MlConfidenceDatasetRow`, `MlConfidencePreparedRows`, `train_ml_confidence_model_from_rows`, `train_ml_confidence_model_from_prepared_rows`, `train_ml_confidence_model_from_meta_artifacts`, and the health/calibration fallback logic).
2. Re-read the third-pass packet and prior review history to separate already-known issues from new ones.
3. Re-checked `crates/app_core/src/lib.rs:190-198` and `REPO_MAP.md` to confirm that this module is an intentionally exported meta/ML surface, not a private implementation detail.
4. Re-checked the Rust API Guidelines on argument validation and panic documentation. The relevant guidance is that Rust public APIs should validate inputs whenever practical, and panic conditions that remain should be documented.[^1][^2]
5. Re-checked scikit-learn’s calibration and grouped-CV documentation to ground the review of calibration evidence and fold-quality claims. The main takeaways are that calibration should ideally be fit on data independent from base-model fitting, grouped CV has specific guarantees about group non-overlap, and class coverage in each split matters for calibration quality.[^3][^4][^5]

One limitation of the bundle is that it did not include the live `src-tauri/src/commands_meta.rs` source. I therefore treated persistence/UI risk as real, but I did not assume a specific current command-side bug that I could not verify from the live file set.

## What Fourth Pass Sharpened

### What the third-pass packet got right

The third-pass packet was right to keep the action bucket small. It was also right that `healthy` can overstate evidence quality, and right that grouped validation here is heuristic enough that its outputs should not be oversold. It also correctly treated this as a persistence-sensitive public surface rather than a hot-path optimization target.

### What it missed or underweighted

#### 1. The strongest safe-now hole is the public training-entrypoint contract, not the splitter

`train_ml_confidence_model_from_rows` and `train_ml_confidence_model_from_meta_artifacts` both normalize and validate `scope` before training (`ml_confidence.rs:1648-1658`, `1693-1703`). `train_ml_confidence_model_from_prepared_rows` does not (`1673-1685`). It accepts a caller-provided `scope`, ignores `prepared.scope` (`352-357`), and passes the caller-provided scope straight into model construction.

That means the prepared-row public API can currently persist a model whose `scope` does not match the prepared data that was actually used. It can also persist a non-canonical timeframe label on the model even though the meta-artifact entrypoint normalizes timeframe first. In a codebase whose model files are named and routed by strategy/timeframe, that is a real contract weakness, not just a style nit.

This is a better first hardening target than a new CV splitter because it is cheap, local, and low-risk for well-formed callers.

#### 2. The raw/prepared row public APIs can panic or build a model that cannot predict

The file exposes `MlConfidenceDatasetRow` publicly (`338-342`) and accepts caller-supplied `x: Vec<f64>` through the public raw/prepared-row training paths. But the training code assumes consistent feature width and never validates it.

The model advertises a fixed seven-feature schema via `FEATURE_NAMES` (`374-382`) and always serializes those names into the model (`1582-1586`). The actual training path, however, just clones whatever `row.x` the caller supplied (`1118`, `1211`). `StandardScaler::fit` and `fit_logistic_regression` then index rows using the first row’s width (`803-838`, `842-899`). With inconsistent row widths, this can panic. With uniformly wrong widths or empty feature vectors, training can succeed but produce a model whose stored feature schema does not match its learned parameters, after which `predict_ml_confidence_for_features` returns `None` because the runtime width checks fail (`1713-1722`).

That is a stronger issue than anything in the third-pass packet. A public training function should not panic on malformed row width when it can cheaply reject the input up front, and it definitely should not persist a model that its own prediction API cannot use.

#### 3. The health fallback problem is broader than “CV produced no predictions”

The third-pass packet correctly called out the obvious fallback at `eval_auc = cv_auc.unwrap_or(train_auc)` and `eval_brier = cv_brier_cal.or(cv_brier).unwrap_or(train_brier)` (`1566-1567`). The sharper point is that the fallback is not limited to the “no CV predictions at all” case.

If CV predictions exist but CV AUC is unavailable because those predictions contain only one class, the code still falls back to train AUC (`1491-1495`, `1566-1573`). Separately, calibration bins also fall back to train-side predictions when CV predictions are absent (`1542-1564`). So the trust problem is broader than the third-pass wording suggested: not only can `healthy` be train-backed, the file can also surface train-backed calibration diagnostics without an explicit evidence-state boundary.

That does **not** mean the immediate safe patch should flip the `healthy` bit. It means the safe patch should add explicit warnings or metadata about train-side fallback and pin that behavior with tests.

#### 4. `Auto` target semantics are more contract-sensitive than the packet implied

The current `Auto` behavior has two different sharp edges.

First, `train_ml_confidence_model_from_rows` accepts `target_used: MlConfidenceTarget`, but silently coerces `MlConfidenceTarget::Auto` to `HoldoutDatasetPass` (`1663-1667`). That is not a meaningful inference on the raw-row API surface; it is a silent semantic choice.

Second, on the prepared-row path, `Auto` resolves to dataset rows on mere presence, not sufficiency (`519-539`). If even one prefixed artifact has any dataset rows, the trainer chooses `HoldoutDatasetPass`, even if that choice later yields too few rows or poor class balance while gate-level rows would have been trainable.

There is also a naming/config nuance the third pass missed. `label_from_holdout_row` does **not** use `min_holdout_pass_rate`; it repurposes `min_holdout_score_p10` as a per-dataset `score_median` threshold (`486-504`). The test fixtures even include a row-level `pass` field (`1791-1800`, `2047-2057`), but the implementation ignores it. That may be intentional, but it is not obvious from the public API names.

This is a documentation and API-semantics issue now, and a redesign issue later. It is not a good candidate for a silent behavior change in a small patch.

### What the third-pass packet overstated, or where the safer recommendation is narrower

#### 1. Reworking CV into something “time-aware” is not the safest next move

The third-pass packet framed the grouping weakness partly as “hash-based rather than explicitly time-aware.” That is too broad. The current splitter is a deterministic group-to-fold hash assignment (`1141-1145`, `1233-1237`). It already preserves group non-overlap, and each group still maps to exactly one test fold. What it does **not** guarantee is fold balance or class coverage.

That distinction matters. Compared with `GroupKFold` and `StratifiedGroupKFold`, the safer criticism is not “this is not time-aware,” but “this lacks explicit guarantees about fold balance and class presence, and the health surface currently degrades too quietly when those guarantees do not hold.”[^4][^5]

Switching the splitter in the next patch would change CV predictions, calibration, `healthy`, and possibly family fallback outcomes. That is redesign work, not first-aid hardening.

#### 2. “Fix `meta_artifacts_used`” is riskier than the packet suggested

The earlier packet was right that `meta_artifacts_used` currently counts prefixed artifacts, not only contributing artifacts (`709-737`, `1605`, `1630`). But changing that field’s meaning in place is not a trivial safe-now cleanup. The current tests already pin the existing semantics (`2013-2038`), and the field is persisted inside model metadata.

The safer near-term move is to document the current meaning, or add a new field in a versioned change. Silently redefining the existing field would be a compatibility change.

#### 3. Public-surface narrowing should stay deferred

The broader public-surface concern from earlier packets is directionally fair, but it should not be sold as low-risk cleanup. `lib.rs` deliberately re-exports the preparation DTOs and training/prediction entrypoints (`lib.rs:190-198`). Without a real consumer audit, removing or narrowing those exports is API redesign.

## Implementation Priority

The third-pass bucket should remain `Small surgical refactor`, but the implementation order should change.

### Safe-now hardening

1. **Unify scope validation across all three public training entrypoints.**
   Add a single normalization/validation helper and use it in `train_ml_confidence_model_from_prepared_rows` as well as the other two entrypoints. On the prepared-row path, reject a mismatch between the caller-provided scope and `prepared.scope` after normalization.

2. **Validate row shape before any training work starts.**
   Reject raw/prepared rows unless every `x` has exactly `FEATURE_NAMES.len()` entries and all feature values are finite. While touching validation, also reject obviously bad scalar config on the logistic path (`learning_rate <= 0`, non-finite `learning_rate`, negative/non-finite `l2`). This prevents both panics and “trained but unusable” models.

3. **Make the evidence fallback explicit without redefining `healthy` yet.**
   Add warnings when train-side AUC or calibration bins are being used as fallback evidence. The file already has warning channels in both `MlConfidenceTrainMetrics` and `MlConfidenceModelMeta`; use them before changing the boolean contract.

4. **Document the current target semantics instead of silently changing them.**
   The immediate safe gain is rustdoc and tests around `Auto`, `HoldoutDatasetPass`, and `meta_artifacts_used`. Do not silently reinterpret those fields in the same patch.

### Larger redesign still deferred

1. **Replace the single `healthy: bool` contract with a richer evidence model.**
   A future change may want an explicit validation-basis enum or additive metadata such as “cv_full”, “cv_partial”, or “train_fallback”. That is a persisted-model contract change and should be versioned.

2. **Rework `Auto` target selection to be sufficiency-aware.**
   A stronger design would attempt dataset-row training first only when dataset-row volume/class balance is sufficient, otherwise falling back to gate rows. That changes training outcomes and should be treated as behavior work, not cleanup.

3. **Revisit the CV splitter only with regression coverage.**
   If the repo wants `GroupKFold`-like or `StratifiedGroupKFold`-like guarantees, that is reasonable, but it should happen with explicit tests for fold coverage, class coverage, calibration, and saved-model drift.[^4][^5]

4. **Keep module splitting and public-surface narrowing deferred.**
   Those are useful debt items, but they are not the safest next patch in a persistence-sensitive, publicly re-exported module.

## Verification If Changed

At minimum, keep the existing verification from the prior packet:

- `cargo test -p ta_app_core ml_confidence::tests:: -- --nocapture`
- `cargo test -p ta_desktop_demo_app build_ml_confidence_status_prefers_dataset_rows_and_keeps_scan_error -- --nocapture`

If the file is changed along the safe-now path above, add or update tests for:

- prepared-row training rejects scope mismatches and normalizes scope consistently with the other entrypoints
- raw/prepared row training rejects inconsistent, empty, or wrong-width feature vectors without panicking
- bad logistic config (`NaN`, non-positive learning rate, negative/non-finite `l2`) is rejected explicitly
- health/calibration warnings distinguish CV-backed evidence from train-side fallback, including the case where CV predictions exist but AUC is unavailable
- `Auto` raw-row behavior is pinned explicitly, either as a documented coercion or as an error if that contract is tightened

If any persisted metadata field or warning contract changes, also run:

- `cargo test -p ta_desktop_demo_app commands::commands_meta::tests:: -- --nocapture`
- `cargo test --workspace --no-run`

## Regression And Feature-Preservation Constraints

- **No GPU ABI risk is in play here.** This is not a CUDA/runtime file. The real risks are persistence semantics, command/UI interpretation, and saved-model compatibility.
- **Do not silently reinterpret `healthy`.** Downstream status and policy-evaluation flows may already treat it as a boolean gate. Add evidence warnings first; version the contract before redefining the bit.
- **Do not silently reinterpret `meta_artifacts_used`.** Existing tests already pin the current meaning, and saved models may have been written with that interpretation.
- **Do not change the feature order or feature count casually.** `FEATURE_NAMES` is the de facto schema contract for models and predictions.
- **Do not swap the CV splitter in the same patch as entrypoint hardening.** That would mix a public API safety fix with a modeling-behavior change and make regressions harder to localize.
- **Any additive persisted metadata should be backward-compatible by construction.** If new fields are added, prefer optional/defaulted fields rather than repurposing existing ones in place.

## Fourth-Pass Conclusion

The third-pass `Small surgical refactor` bucket should **stand**, but the fourth pass changes what that patch should actually do.

The current packet got the evidence-quality concern basically right. What it still missed is that the live public API has two sharper, easier-to-fix contract holes: `train_ml_confidence_model_from_prepared_rows` is less validated than the other entrypoints, and the raw/prepared row APIs can currently panic or persist an unusable model when callers supply malformed feature vectors. Those are better next fixes than a splitter rewrite.

The fourth-pass recommendation is therefore:

- keep the bucket unchanged
- treat public-entrypoint hardening as the first patch
- keep the health concern, but narrow the immediate fix to additive evidence/warning semantics
- defer `Auto` target redesign, splitter redesign, public-surface narrowing, and any in-place reinterpretation of persisted metadata

That is the most conservative path to make this file more “public-review ready” without causing avoidable regressions.

[^1]: Rust API Guidelines, “Dependability / Functions validate their arguments (C-VALIDATE)”, <https://rust-lang.github.io/api-guidelines/dependability.html>.
[^2]: Rust API Guidelines, “Documentation”, especially panic documentation guidance, <https://rust-lang.github.io/api-guidelines/documentation.html>.
[^3]: scikit-learn User Guide, “Probability calibration”, including the guidance that calibration should ideally be fit on data independent from base-model fitting, and that class absence in train/test splits weakens calibration quality, <https://scikit-learn.org/stable/modules/calibration.html>.
[^4]: scikit-learn API Reference, `GroupKFold`, <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.GroupKFold.html>.
[^5]: scikit-learn API Reference, `StratifiedGroupKFold`, <https://scikit-learn.org/stable/modules/generated/sklearn.model_selection.StratifiedGroupKFold.html>.
