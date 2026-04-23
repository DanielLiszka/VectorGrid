# OX-04 Dependence-aware uncertainty and effective-sample-size math  
Combined first-pass, second-pass, and third-pass answers

I only have the `OX-04` overlay prompt rather than the chosen `PR-*` core prompt, so these answers stay scoped to the five copied files named by the overlay: `validation_stats.rs`, `sharpe_stats.rs`, `validation_pipeline.rs`, `meta_select.rs`, and `ml_signal.rs`. fileciteturn19file0

## First-pass answer

I only have the OX-04 overlay prompt rather than the chosen PR-* core prompt, so this is a first-pass overlay review scoped to the five copied files. The main result is that the repo already has some dependence-aware machinery, but the main PSR/DSR/min-track-record path still treats raw counts as iid evidence, and that leaks into hard gates, run approval, ML-signal validation, and meta-selection summaries. fileciteturn19file0 fileciteturn21file0 fileciteturn20file1

### Where the iid assumption is still leaking

1. In `validation_stats.rs` and `sharpe_stats.rs`, the Sharpe-family confidence math is still raw-count math. `compute_psr_dsr` passes raw `obs` into PSR/DSR, `probabilistic_sharpe_ratio` scales with `sqrt(n-1)`, and `deflated_sharpe_ratio` uses raw `num_trials` through `expected_max_sharpe`. The min-track-record routine is also iid math; the dependence problem there is that later code compares its iid-required length against raw observed bars. So positive serial dependence or overlapping labels makes PSR/DSR too optimistic, while highly correlated candidate swarms can make DSR too harsh because raw `trials` overstates the number of genuinely distinct tests. Dependence is not pushing in only one direction here. fileciteturn21file15 fileciteturn21file13 fileciteturn21file17

2. In `validation_pipeline.rs`, `compute_probability_metrics` simply forwards `sample_obs`, counts all finite peer Sharpes as `trials`, and stores `track_record_bars = requested_obs`. Those numbers then feed candidate hard gates and run verdict logic. The tests explicitly show that min-track-record can flip a run from approved to unapproved, so this is decision-changing, not just reporting. The same file’s walk-forward code uses rolled windows with `window_bars.max(64)` and `step_bars.max(1)`, and the test case with `n = 66` yields windows `(0,64)`, `(1,65)`, `(2,66)`, which are almost the same evidence three times over. That is fine as a descriptive robustness scan, but not as an independent sample count. fileciteturn21file0 fileciteturn21file2 fileciteturn20file0

3. `ml_signal.rs` is the sharpest concrete failure. `build_ml_signal_split_windows` rolls train/calibration/test windows forward by `step_bars`, so overlap is allowed by construction. Later, `validate_ml_signal_candidates` sets `actual_score_obs` to the raw count of non-missing score points, writes that raw count into every candidate’s `track_record_bars`, and passes the same raw count into `compute_probability_metrics`. `score_scope_warning_flag` only adds warnings for `CalibrationSlice`, `FullDataset`, and `CustomPartial`; it treats `WalkforwardTest` as warning-free even though the split builder does not require non-overlapping test windows. That means `WalkforwardTest` is not automatically an iid scope. fileciteturn20file11 fileciteturn20file17 fileciteturn21file11

4. `meta_select.rs` is better on dataset coverage than on evidence weighting, but it is still not dependence-aware enough. `datasets_covered` and `pass_rate` dedupe by dataset key, which is good. But the train-stage quantiles and medians are still computed over every absorbed row, so repeated artifacts from one dataset can overweight that dataset in `robust_score_p10`, `max_dd_p90`, `psr_median`, and `dsr_median`. Evidence flags and `min_samples` also read raw `samples`. Today `w_psr` and `w_dsr` default to zero, so the immediate distortion is more about row-weighted summaries and sample-count gates than about the final weighted score, but the structure is already ready to consume miscalibrated PSR/DSR if those weights are turned on. fileciteturn20file6 fileciteturn20file4 fileciteturn23file1

### What already points in the right direction

The repo already has two good dependence-aware patterns: trade bootstrap has a stationary-bootstrap branch, and SPA validation resamples slice differences with a stationary bootstrap and configurable block length. So the missing piece is not conceptual acceptance of dependence-aware methods; it is that PSR/DSR/minTRL happen first and still run on raw counts. fileciteturn24file16 fileciteturn24file18 fileciteturn24file6 fileciteturn24file19

### New math this overlay should force into the review

**Lemma candidate: exact overlap lemma for h-bar ML labels.**  
If a label or outcome uses `horizon_bars = h` and is sampled every bar, take
\[
Y_t=\sum_{j=1}^{h}\varepsilon_{t+j}
\]
with iid one-bar innovations. Then
\[
\rho_k = 1-\frac{k}{h}\quad (1\le k<h),\qquad \rho_k=0\quad (k\ge h).
\]
So
\[
1 + 2\sum_{k=1}^{h-1}\rho_k = h,
\qquad
n_{\mathrm{eff}} = \frac{n}{h}.
\]
That means raw `actual_score_obs = n` overstates information by a factor of `h`, and PSR/DSR z-scores are inflated by `\sqrt{h}`. This is exact, not heuristic, and it is directly relevant to the ML signal label/split machinery. fileciteturn22file16 fileciteturn20file17

**Counterexample family: near-duplicate candidate swarms.**  
Take many candidate policies that differ only by tiny threshold or parameter nudges, so their return paths are nearly the same. `compute_probability_metrics` still sets `trials = sharpe_peers.len()`, and `deflated_sharpe_ratio` then behaves as if that whole swarm were independent multiplicity. In this family, raw `obs` is too optimistic because the underlying bar evidence is dependent, but raw `trials` is too pessimistic because the effective number of independent trials is near the cluster count, not the row count. So the current DSR path can mis-rank stable candidate clusters. fileciteturn21file0 fileciteturn21file13

**Bound: conservative independence bound for rolled windows.**  
If you have `W` rolled windows of width `w` and step `s`, then the number of pairwise disjoint windows is at most
\[
1+\left\lfloor \frac{(W-1)s}{w}\right\rfloor.
\]
That is a safe upper bound on “independent windows” before any autocorrelation estimate. In the test case with `w = 64`, `s = 1`, and `W = 3`, the bound is `1`, not `3`. So `windows` should stay descriptive unless you also compute an overlap-adjusted count. fileciteturn20file0

### Smallest implementation cut that stays inside these files

1. In `sharpe_stats.rs` / `validation_stats.rs`, add ESS-aware wrappers rather than rewriting everything at once. The right seam is a helper that takes `eff_obs` for PSR/minTRL comparisons and `eff_trials` for DSR. A simple fallback wrapper can still use `eff_obs = raw_obs` and `eff_trials = raw_trials`. For `eff_trials`, the best long-run target is a cluster-adjusted or correlation-adjusted count; even a caller-supplied cluster count is better than always using the raw peer length. fileciteturn21file15 fileciteturn21file13

2. In `validation_pipeline.rs`, change `compute_probability_metrics` so it can consume both raw and effective counts. Keep reporting `track_record_bars` as the raw count if you need backward compatibility, but also compare against `eff_obs`. The clean rule is: keep `minimum_track_record_length` as an iid-required length, and compare `eff_obs` against that requirement. fileciteturn21file0

3. In `ml_signal.rs`, the first conservative fix can be local. For overlapping `horizon_bars = h`, use `eff_obs ≈ floor(actual_score_obs / h)` as a lower-bound fallback. For rolled split windows, compute the union length or overlap-adjusted count of test spans instead of assuming that `WalkforwardTest` is automatically iid. The split builder already has the boundaries needed to do that without leaving this file. fileciteturn20file11 fileciteturn22file16

4. In `meta_select.rs`, aggregate to one row per `dataset_key` before computing train-stage quantiles and medians. That fixes the current row-weighting problem without redesigning surrounding ownership. Alongside that, add an `effective_samples_lower` concept equal to `datasets_covered`; it is a safe lower bound on independent evidence until richer time-span metadata is available. Use that lower bound for evidence flags, and keep raw `samples` as a coverage/debug number. fileciteturn20file6 fileciteturn20file4

5. For `compute_walk_forward_stats` and `compute_reopt_walk_forward_stats`, I would not force a hard redesign yet. Just publish raw window count and an overlap-adjusted upper bound on effective windows. That gives downstream reviewers the right uncertainty lens without turning descriptive robustness summaries into pretend hypothesis tests. fileciteturn20file0

### Priority

P0 is `compute_probability_metrics` plus the Sharpe-family wrappers, because that is the shared gate path.  
P1 is `ml_signal.rs`, because the exact `n_eff = n / h` overlap failure is already staring at the `horizon_bars` design.  
P1 is `meta_select.rs`, because per-row quantiles can currently overweight one dataset’s repeated artifacts.  
P2 is rolled-window reporting, because that is mostly a “do not over-interpret this count” fix unless a core PR wants those windows to become inferential evidence.

The single highest-value seam is to make the shared probability path consume effective sample size and effective trial count; once that exists, the ML and meta layers can plug into it without changing the surrounding workflow shape.

## Second-pass answer

Given only the `OX-04` overlay file, I’m keeping ownership and comparison fixed to the five files it names—`validation_stats.rs`, `sharpe_stats.rs`, `validation_pipeline.rs`, `meta_select.rs`, and `ml_signal.rs`—and answering the second-pass requirement the overlay spells out: show how dependence changes exactness assumptions, bounds, or adversarial constructions without changing ownership. fileciteturn19file0

The main second-pass change is that PSR, DSR, and minimum-track-record are not exact uncertainty objects for this repo as wired today; they are iid-calibrated surrogates. `compute_probability_metrics` builds `trials` from the raw count of finite peer Sharpes, takes raw `sample_obs` with only a `max(2)` clamp for the math path, forwards those into `compute_psr_dsr`, and separately computes `minimum_track_record_length`; `compute_psr_dsr` then passes raw `obs` and `trials` into `probabilistic_sharpe_ratio` and `deflated_sharpe_ratio`, whose formulas scale directly with `samples` and `num_trials`. Those values then feed candidate warnings, approval checks, and the run-level `EvidenceTooShortMinTrackRecord` decision. fileciteturn21file0 fileciteturn21file15 fileciteturn21file13 fileciteturn21file17 fileciteturn21file2

That changes the exactness assumption in a precise way: raw `track_record_bars` is only a raw exposure count, not a count of independent observations. The tests already preserve that distinction implicitly: one test keeps `track_record_bars` at raw values like `0` and `7` while still forcing PSR/DSR computation through the clamped observation path, and another test shows that failing min-track-record flips the final run recommendation. So the second-pass correction is not “nice to have”; it changes the semantics of existing hard and soft gates. fileciteturn22file7 fileciteturn21file2

The cleanest new lemma is the overlap lemma for ML labels. If a label uses `horizon_bars = h` and is sampled every bar, and the label is the sum of the next `h` one-bar shocks, then \(\rho_k = 1-k/h\) for \(1 \le k < h\), zero afterward, so \(1 + 2\sum_{k\ge1}\rho_k = h\) and \(n_{\mathrm{eff}} = n/h\). In this repo, ML schema/manifest structures carry `horizon_bars` and `label_horizon_bars`, but validation currently sets `actual_score_obs` to the raw number of scored rows, copies that into `track_record_bars`, and sends that raw count into `compute_probability_metrics`. Under the simplest overlapping-label model, that makes the current ML evidence count too large by a factor of about `h`. fileciteturn22file16 fileciteturn20file17

The overlay also changes what “walk-forward” can mean here. `build_ml_signal_split_windows` rolls windows by `step_bars`; a test explicitly expects windows starting at `0` and `50` for a 300-row example, so overlap is allowed by construction. Meanwhile `score_scope_warning_flag` treats `WalkforwardTest` as warning-free. So `WalkforwardTest` is not an independence certificate in this codebase; it is only a provenance label. fileciteturn20file11 fileciteturn22file8 fileciteturn21file11

The new bound the core prompt would likely miss is an overlap bound for rolled windows: if each window has width `w`, step `s`, and there are `W` rolled windows, then the maximum number of pairwise disjoint windows is \(1+\lfloor (W-1)s/w \rfloor\). In `compute_walk_forward_stats`, the test case with clamped `window=64`, `step=1`, and observed windows `(0,64)`, `(1,65)`, `(2,66)` has `W=3`, `w=64`, `s=1`, so the disjoint-window upper bound is only `1`, not `3`. The second-pass consequence is that `stats.windows` should be treated as descriptive coverage, not as three independent validations. fileciteturn20file0

The new adversarial family is a near-duplicate candidate swarm. In the shared probability path, `trials` is the raw count of finite peer Sharpes, and `deflated_sharpe_ratio` uses `expected_max_sharpe(num_trials, ...)`; `expected_max_sharpe` is also explicitly tested to be monotone in trial count. That means cloning a candidate into many tiny parameter perturbations can make DSR stricter even when the search has not really added distinct information. So under the overlay, raw `trials` is only an upper bound on multiplicity, not an exact count of distinct tests. fileciteturn21file0 fileciteturn21file13 fileciteturn21file1

`meta_select.rs` becomes a hybrid-weighting issue under this lens. The ingestion path pushes every absorbed row into `robust_scores`, `psrs`, and `dsrs`, while only `dataset_keys` is deduped. Later, `robust_score_p10`, `psr_median`, and `dsr_median` are computed over those row-level vectors, `datasets_covered` becomes `pass_rate`, and evidence flags still key off raw `group.samples`. So train-stage coverage is dataset-weighted, but the key quantiles and the sample-count evidence flags are artifact-weighted. Repeated artifacts from one dataset can therefore dominate the meta summaries even when `pass_rate` looks fine. fileciteturn20file6 fileciteturn20file1

What does not change is also important: the repo already contains dependence-aware machinery in the trade bootstrap and SPA paths. So the overlay is not arguing that this repo ignores dependence everywhere; it is showing a mismatch, where the later resampling layers are dependence-aware but the earlier PSR/DSR/min-track-record path is still raw-count iid math. fileciteturn24file16 fileciteturn24file18 fileciteturn24file6 fileciteturn24file19

So the second-pass bottom line is: any core comparison that treated PSR/DSR/min-track-record as exact evidence should now be downgraded to “iid-calibrated proxy,” any comparison that used `track_record_bars`, `samples`, or `windows` as independent evidence counts should now be read as an upper-bound-style raw count, and the most adversarial failure modes are overlapping ML labels/windows on the one hand and near-duplicate candidate swarms on the other. The ownership does not move; the comparison standard does.

## Third-pass answer

Because I still only have the `OX-04` overlay rather than the selected `PR-*` core prompt, I’m keeping implementation ownership inside the same five-file boundary the overlay names and turning the dependence lens into the smallest possible patch plan that does not redesign adjacent surfaces. The overlay explicitly asks for implementation-facing consequences inside `validation_stats.rs`, `sharpe_stats.rs`, `validation_pipeline.rs`, `meta_select.rs`, and `ml_signal.rs`. fileciteturn19file0

The highest-value implementation move is this: **keep raw counts for reporting, but run statistical gates on effective counts, then convert the iid-required minimum track record back into raw-bar units before storing it.** That fits the current wiring because `compute_probability_metrics` already sets `track_record_bars` and `min_track_record_length_bars`, while `build_run_verdict` already compares raw `track_record_bars` against the stored minimum requirement. In other words, the repo already has the right seam; it is just feeding that seam raw iid counts today. fileciteturn21file0 fileciteturn21file2

### 1) `validation_stats.rs` and `sharpe_stats.rs`: add ESS-aware wrappers, not new formulas

`validation_stats::compute_psr_dsr` is currently just a thin adapter over `probabilistic_sharpe_ratio` and `deflated_sharpe_ratio`, and those functions scale directly with `samples` and `num_trials`. `minimum_track_record_length` is also an iid sample-length calculation. That means the implementation-facing fix is not to rewrite the Sharpe math; it is to give those functions ESS-equivalent counts. fileciteturn21file15 fileciteturn21file13 fileciteturn21file17

The minimal patch is to add a new helper path, while leaving the old signature as a compatibility wrapper:

```rust
pub struct EffectiveCounts {
    pub raw_obs: usize,
    pub eff_obs: usize,
    pub raw_trials: usize,
    pub eff_trials: usize,
}

pub fn compute_psr_dsr_effective(
    sharpe: f64,
    counts: EffectiveCounts,
    peer_mean: f64,
    peer_std: f64,
) -> (Option<f64>, Option<f64>);
```

and a tiny conversion helper

```rust
pub fn inflate_mintrl_to_raw_bars(
    iid_mintrl: Option<usize>,
    raw_obs: usize,
    eff_obs: usize,
) -> Option<usize>;
```

with

\[
\text{stored\_mintrl\_raw}
=
\left\lceil
\text{iid\_mintrl} \cdot \frac{\max(raw\_obs,2)}{\max(eff\_obs,2)}
\right\rceil.
\]

That single conversion is the key third-pass implementation consequence. It lets the rest of the repo continue to compare raw bar counts against a stored bar threshold, while dependence adjustment happens invisibly inside the probability path. No new verdict interface is needed.

The counterexample-family implication also lands here: `expected_max_sharpe` is monotone in trial count, so the DSR path needs a place to accept `eff_trials` even if most callers still default to raw `trials` at first. The third-pass patch should therefore add the seam now, even if only some callers can exploit it immediately. fileciteturn21file1

### 2) `validation_pipeline.rs`: add an effective-count overload and preserve existing verdict wiring

`compute_probability_metrics` is the shared gate path. Today it counts finite peer Sharpes, clamps `requested_obs` to `obs.max(2)` for the statistical calculation, stores raw `track_record_bars = requested_obs`, and stores the iid minimum track record directly. That is the single most important place to patch. fileciteturn21file0

The smallest safe change is:

```rust
pub fn compute_probability_metrics_with_effective_counts<C: ValidationPostCandidate>(
    out: &mut Vec<C>,
    raw_sample_obs: &[usize],
    eff_sample_obs: Option<&[usize]>,
    eff_trials: Option<usize>,
)
```

and keep the existing `compute_probability_metrics` as a wrapper that calls the new function with `eff_sample_obs = None` and `eff_trials = None`.

Inside the new helper:

- use `requested_obs` for `cand.set_track_record_bars(requested_obs)`;
- use `stat_obs = eff_sample_obs[i].unwrap_or(requested_obs).max(2)` for PSR and DSR;
- use `stat_trials = eff_trials.unwrap_or(raw_trials).max(1)` for DSR;
- compute `iid_mintrl = minimum_track_record_length(...)`;
- store `inflate_mintrl_to_raw_bars(iid_mintrl, requested_obs.max(2), stat_obs)` as `min_track_record_length_bars`.

That is enough to make `build_run_verdict` dependence-aware **without editing `build_run_verdict` at all**, because it already treats `track_record_bars` as raw and compares that raw number against the stored threshold. The existing tests that preserve raw `track_record_bars` and the existing test that flips the run to `EvidenceTooShortMinTrackRecord` can stay conceptually intact; the new tests should assert that raw reporting is preserved while stored minTRL gets stricter when `eff_obs < raw_obs`. fileciteturn22file7 fileciteturn21file2

### 3) `ml_signal.rs`: implement the exact overlap lemma locally

This file is where the new lemma becomes a concrete patch. The export path already records `horizon_bars`, `label_horizon_bars`, and split-window metadata, and the validation path already computes `actual_score_obs` separately from candidate metrics. That means `ml_signal.rs` can supply dependence-aware counts to the shared helper without redesigning other modules. fileciteturn22file16 fileciteturn20file11 fileciteturn20file17

The exact overlap lemma says that for contiguous `h`-bar labels, \(n_{\mathrm{eff}} = n/h\). The direct implementation is:

```rust
fn effective_score_obs_from_overlap(
    raw_obs: usize,
    label_horizon_bars: usize,
) -> usize {
    raw_obs / label_horizon_bars.max(1)
}
```

For `WalkforwardTest`, use test-span union first, then horizon adjustment:

```rust
fn union_len_test_spans(windows: &[MlSignalSplitWindow]) -> usize;
```

and then

```rust
let unique_test_bars = min(raw_obs, union_len_test_spans(&split_windows));
let eff_obs = unique_test_bars / label_horizon_bars.max(1);
```

This is better than pretending every rolled window is new evidence, and it stays entirely inside `ml_signal.rs`. The split-window builder already exposes `test_start` and `test_end`, and the current test case proves overlap is allowed. fileciteturn20file11 fileciteturn22file8

The practical patch sequence is:

1. extend `LoadedScoreCache` with optional `label_horizon_bars` and optional `split_windows`;
2. populate them in `load_score_cache` by reading the export manifest and split manifest when available;
3. in `validate_ml_signal_candidates`, compute `raw_obs = actual_score_obs`, `eff_obs = overlap_adjusted_score_obs(...)`;
4. call `compute_probability_metrics_with_effective_counts` with raw and effective counts.

This keeps `track_record_bars` equal to the raw score count, so the existing “raw summary field” test should keep passing, but PSR/DSR and stored minTRL will become dependence-aware. fileciteturn20file17

The bound from the second pass also has a local implementation consequence: `score_scope_warning_flag` should not treat `WalkforwardTest` as automatically safe whenever split metadata is missing or overlapping. The lowest-friction change is not to redesign the existing scope enum; it is just to append a new warning such as `score_scope_walkforward_overlap_adjusted` whenever `score_scope == WalkforwardTest` and `eff_obs < raw_obs`. That reuses the current warning-flag pattern instead of adding a new output surface. fileciteturn21file11

### 4) `meta_select.rs`: dedupe at the dataset level before quantiles, but keep request shape unchanged

The dependence leak in `meta_select.rs` is not a formula problem; it is a weighting problem. The file already dedupes dataset coverage through `dataset_keys`, then computes `pass_rate` from that deduped set, but still computes `robust_score_p10`, `max_dd_p90`, `psr_median`, and `dsr_median` over every absorbed row. That means one dataset with many repeated artifacts can dominate the train-stage summaries even while `pass_rate` looks healthy. fileciteturn20file6 fileciteturn20file4

The minimal patch is to add a local collapse step right before the quantiles:

```rust
struct DatasetAggregate {
    robust_score: f64,
    max_dd: f64,
    cross_score: f64,
    psr: f64,
    dsr: f64,
}
```

Build one `DatasetAggregate` per `dataset_key` using a stable rule such as median robust score / median PSR / median DSR / worst max drawdown, then compute `robust_score_p10`, `max_dd_p90`, `cross_score_median`, `psr_median`, and `dsr_median` over those dataset-level vectors instead of raw row vectors.

That fixes the row-overweighting bug without changing `MetaSelectionRequest`, `min_samples`, or any external caller. The third-pass dependence-aware implication is: **leave `samples` as a raw debug/coverage count, but treat `datasets_covered` as the lower-bound effective sample count for evidence flags and other “thin evidence” messaging.** The file already has `datasets_covered`; it just is not yet the quantity driving those summaries. fileciteturn20file1

One subtle but important note: `w_psr` and `w_dsr` default to zero, so the immediate ranking change mostly comes from the already-active `robust_score_p10` and `max_dd_p90` channels, not from PSR/DSR weight terms. But if those weights are ever turned on, the bug becomes more visible, so it is worth fixing now while the ownership boundary is local. fileciteturn23file1

### 5) What I would not redesign in this pass

I would not try to infer a fully correlation-clustered `eff_trials` inside `compute_probability_metrics` yet, because this shared helper only sees metrics, not parameter similarity or return-path correlation. The right third-pass implementation move is just to create the optional `eff_trials` seam and default it to raw trial count until a caller can supply something better. That is the implementation consequence of the near-duplicate swarm counterexample, and it avoids fake precision.

I also would not redesign `WalkForwardStats` or `ReoptWalkForwardStats` outputs in this pass. The overlap bound is real, but those stats are currently descriptive summaries rather than shared gate inputs. A doc comment and perhaps an internal helper for future overlap-adjusted counts are enough for now. The P0 path is still the probability gate path. fileciteturn20file0

### Tests to add in this boundary

1. `validation_stats.rs` / `validation_pipeline.rs`: when `eff_obs == raw_obs` and `eff_trials == raw_trials`, outputs match the old path exactly.  
2. `validation_pipeline.rs`: with `raw_obs = 120` and `eff_obs = 12`, `track_record_bars` stays `120` but stored `min_track_record_length_bars` inflates by about `10x` relative to the iid case.  
3. `ml_signal.rs`: for `horizon_bars = 10`, raw score count remains unchanged in the result, but PSR/DSR are weaker than the raw-count path and the stored minTRL is stricter.  
4. `ml_signal.rs`: overlapping walk-forward test spans produce `eff_obs < raw_obs` and append the overlap-adjusted warning flag.  
5. `meta_select.rs`: twenty rows from one dataset plus one row from another should compute quantiles from two dataset aggregates, not twenty-one artifact rows.  
6. `meta_select.rs`: thin-evidence warnings should still appear when `datasets_covered` is small even if raw `samples` is large.  

### Priority

P0 is the new effective-count helper plus raw-bar minTRL inflation in `validation_pipeline.rs`.  
P1 is the ML overlap patch, because the exact `n/h` lemma is already encoded by `horizon_bars`.  
P1 is dataset-level collapse in `meta_select.rs`.  
P2 is any broader multiplicity clustering for DSR.

The important third-pass conclusion is that the implementation can become dependence-aware **without** changing the surrounding verdict interface: raw bars remain the displayed unit, but the probability math and the stored min-track-record threshold become effective-count aware under the hood.