# OX-08 calibration / threshold-geometry review — first, second, and third pass

Source prompt overlay: `OX-08 Calibration, inverse modeling, and threshold-geometry math`. fileciteturn13file0

I only had the OX-08 overlay and the four copied repo files, not the attached core `PR-*` prompts, so the three passes below apply the overlay directly to:

- `crates/app_core/src/ml_confidence.rs`
- `crates/app_core/src/ml_signal.rs`
- `crates/app_core/src/options_expression.rs`
- `crates/app_core/src/validation_stats.rs`

---

## First-pass answer

The short first-pass verdict is:

`ml_confidence.rs` already has a real score-to-probability calibration layer, but the label being calibrated is semantically unstable. `ml_signal.rs`, `validation_stats.rs`, and several gates in `options_expression.rs` still use hard thresholds or fixed monotone transforms that look like stand-ins for unmodeled inverse problems rather than calibrated decisions.

### `ml_confidence.rs`

This is the healthiest surface from a calibration standpoint. It explicitly fits Platt scaling on predicted probabilities, retains calibration only when calibrated CV Brier is no worse than raw CV Brier, applies that calibration at prediction time, and marks the model unhealthy when eval AUC drops below `0.55` or eval Brier is worse than baseline. fileciteturn14file1

But the target geometry is unstable.

The defaults are already a thresholded policy: `min_holdout_pass_rate = 0.60`, `min_holdout_score_p10 = 0.0`, `min_holdout_datasets = 30`. Then the code defines two different labels:

- `label_from_holdout` says positive iff aggregate holdout `pass_rate >= 0.60` and `score_p10 >= 0.0`, plus optional DD filtering. fileciteturn15file3
- `label_from_holdout_row` says positive iff the per-dataset `score_median >= min_holdout_score_p10`, with optional `max_dd_worst` filtering. It does not use aggregate pass rate. fileciteturn15file3

Then `resolve_target_from_prepared_rows` switches between those two meanings when `target = Auto`, based only on whether holdout dataset rows exist in the chosen prefix. The test `prepared_row_prefix_auto_target_preserves_meta_usage` confirms that prefix `1` resolves to `HoldoutGate`, while prefix `2` resolves to `HoldoutDatasetPass`. fileciteturn15file11

So the main first-pass issue here is not missing calibration of scores. It is that `\hat p = g(s)` is being asked to approximate different events depending on artifact availability. That makes the resulting “probability” less interpretable than it looks.

Repo-fit fix: make the target explicit and split the config fields. In particular, `min_holdout_score_p10` is doing two jobs right now. It should become separate knobs for aggregate holdout quantile logic and per-dataset median logic. If both targets are wanted, train two models, or define a two-stage inversion: first estimate per-dataset pass probability, then map that to aggregate gate probability with a separate calibrated monotone map keyed by dataset count.

### `ml_signal.rs`

This surface already treats scores as probabilities in several places. Raw model margins are pushed through sigmoid or isotonic calibration, native score-cache builds write calibrated `p_long` and `p_short`, and CSV ingestion explicitly prefers calibrated probability columns like `p_long_cal` / `p_short_cal`. fileciteturn15file2 fileciteturn14file4 fileciteturn14file12

But the action rule is not itself calibrated. It is a wedge in `(p_long, p_short)` space:

- defaults `t_long = t_short = 0.55`, `delta = 0.05`
- long if `p_long >= t_long` and `p_long - p_short_cmp >= delta`
- short if `p_short >= t_short` and `p_short - p_long >= delta`
- else flat. fileciteturn14file4

Also, training labels already come from a deadbanded threshold geometry: `y_long = 1` when `r_long > deadband`, `y_short = 1` when `r_short > deadband`, and samples in `[0, deadband]` are dropped. fileciteturn15file2

That means this pipeline is calibrating component probabilities, but the final decision rule is still an ad hoc inverse map from those probabilities to a trading action. Because long and short come from separate calibrated channels, they are not constrained to form a coherent simplex; `delta` is effectively acting as a tie-breaker and coherence patch.

Repo-fit fix: reparameterize the policy. Instead of `(t_long, t_short, delta)`, use something like:

- a presence threshold `τ` on “any action is warranted”
- an advantage threshold `κ` on directional preference

Concretely, a minimal reparameterization is:

- `q = max(p_long, p_short)`
- `m = logit(p_long) - logit(p_short)`

Then long if `q >= τ` and `m >= κ`, short if `q >= τ` and `m <= -κ`, else flat.

### `validation_stats.rs`

This file is the cleanest example of a hard threshold behaving like a hidden inverse map.

`sanitize_probability_gate` clamps any finite threshold into `[0, 1]`, so `-1.0` and `0.0` become identical, and `1.0` and `2.0` become identical. Then `passes_probability_gates` compares missing PSR/DSR as `0.0` whenever a threshold is present. The tests explicitly lock in the discontinuity: missing stats fail when threshold is positive, but missing stats pass when threshold is exactly `0.0`. fileciteturn15file0

So the acceptance rule for a missing stat is literally:

\[
A(\tau) = \mathbf{1}\{\tau \le 0\}.
\]

That is a jump discontinuity at `0`. Changing a threshold from `0.0` to `1e-12` flips every missing-stat candidate from pass to fail.

Repo-fit fix: separate threshold transport from policy semantics. Either reject out-of-range thresholds instead of silently clamping them, or at least surface a warning and preserve the raw input. More importantly, add an explicit missing-value policy such as `MissingAsFail`, `MissingAsPass`, or `MissingAsUnknown`. Right now that choice is encoded implicitly in `unwrap_or(0.0)`.

### `options_expression.rs`

This file has multiple threshold geometries that are clearly hand-tuned.

The backend auto-selection is an orthant classifier over fixed constants:

- candidate density `>= 0.25`
- invalid slot rate `<= 0.75`
- compiled tuples `>= 256`
- search events `>= 8`
- projected work `>= 10_000`
- measured speedup `>= 1.25` for promotion. fileciteturn16file15

Coverage adequacy uses another threshold stack around overlap floors, coverage floors, and dominant invalid-reason share. fileciteturn15file17

Stress modes are fixed monotone transforms on entry and exit prices. Robust ranking uses

\[
R = 0.45u_s + 0.25u_h + 0.15u_c + 0.10u_{hh} + 0.05u_e,
\quad
u_i = (1-\text{invalid\_rate}_i)\cdot \text{expectancy\_per\_premium}_i
\]

with missing holdout/harsher/extreme lanes imputed from search/stress. fileciteturn15file13

The test around measured speedup `1.23x` shows how sharp these transitions are: runtime auto can recommend GPU, while promotion logic flips to CPU solely because `1.23 < 1.25`. fileciteturn16file18

Repo-fit fixes:

- learn a calibrated “GPU is worth it” score from existing workload features and historical CPU/GPU timings, then threshold that score
- replace fixed stress multipliers with empirical quantile transforms conditioned on price semantics and fill model
- if `robust_score` is meant to approximate sealed-stage quality, calibrate it directly against sealed outcomes instead of relying on fixed lane weights forever

### New theorem-level material from first pass

**Lemma candidate.**  
For `ml_signal`, let the boundary set be

\[
B=\{p_l=t_l\}\cup\{p_s=t_s\}\cup\{p_l-p_s=\delta\}\cup\{p_s-p_l=\delta\}.
\]

If a score point is farther than `ε` from `B` in `\ell_\infty`, then any perturbation of `(p_l,p_s)` by at most `ε` leaves the chosen action unchanged.

**Counterexample family.**  
In `ml_confidence`, two histories with the same feature distribution and same aggregate holdout summary can still induce different target semantics because `Auto` switches from aggregate holdout-gate labels to per-dataset pass labels when dataset rows become available. fileciteturn15file11

**Bound.**  
For `options_expression`, since `robust_score` is linear in lane utilities,

\[
|\Delta R|
\le
0.45|\Delta u_s|
+0.25|\Delta u_h|
+0.15|\Delta u_c|
+0.10|\Delta u_{hh}|
+0.05|\Delta u_e|.
\]

So any calibration error in the search lane matters `9x` as much as the extreme lane.

### First-pass priority order

1. Make `ml_confidence` target semantics explicit and stop letting `Auto` silently change the meaning of the label.
2. Reparameterize `ml_signal` action selection from three raw probability thresholds into a confidence-plus-advantage geometry.
3. Add explicit missing-stat policy in `validation_stats` and stop overloading threshold `0.0` with semantic meaning.
4. Replace `options_expression` hard backend/promotion/stress constants with calibrated monotone maps derived from existing runtime and evaluation data.

The sharpest first-pass call is: this repo already calibrates scores in some places, but several important decisions are still being made by constants that stand in for probabilities of the thing the system actually cares about.

---

## Second-pass answer

The second-pass verdict is stronger than the first-pass one:

The repo does not just have some arbitrary thresholds. In a few places, the evidence used to justify those thresholds is itself not exact to the deployed decision problem. The biggest upgrade from first pass is that `ml_confidence.rs` and `options_expression.rs` each have a pre/post mismatch: the model or score is calibrated or ranked on one event / approximation, then later interpreted as if it were exact for another. `ml_signal.rs` has calibrated component probabilities but an uncalibrated decision wedge, and `validation_stats.rs` has a literal step discontinuity at threshold `0`. fileciteturn14file1 fileciteturn15file13 fileciteturn15file0

### What changes at second pass

At first pass, the main observation was “some constants look hand-tuned.”  
At second pass, the sharper observation is:

- `ml_confidence` changes the meaning of the label under `Auto`, and in dataset-pass mode it also changes the CV grouping geometry, so calibration quality can be overstated for the real “new meta artifact” problem. fileciteturn15file3 fileciteturn14file8
- `options_expression` ranks and truncates candidates before the later exactify stage recomputes missing lanes, so the pre-truncation robust score is not the same score as the post-exactify robust score. fileciteturn15file13 fileciteturn16file15
- `ml_signal` calibrates margins to probabilities, but the action map is still a raw threshold wedge over two probabilities, so decision stability depends on boundary mass rather than just Brier quality. fileciteturn15file2 fileciteturn14file4
- `validation_stats` overloads threshold `0.0` with missing-value policy, producing a jump discontinuity at zero. fileciteturn15file0

### `ml_confidence.rs`: the new exactness problem is CV grouping, not just target semantics

The code does contain a real calibration path: it fits Platt scaling on CV probabilities, keeps calibration only if calibrated CV Brier is no worse than raw CV Brier, and marks the model unhealthy if evaluation AUC is below `0.55` or evaluation Brier is worse than baseline. fileciteturn14file1

But second pass reveals a deeper issue than first pass.

The target switch is already real:

- `label_from_holdout` defines a positive aggregate holdout gate using `pass_rate >= min_holdout_pass_rate` and `score_p10 >= min_holdout_score_p10`, with optional drawdown filtering. fileciteturn15file3
- `label_from_holdout_row` instead defines a per-dataset positive using `score_median >= min_holdout_score_p10`, again with optional drawdown filtering. fileciteturn15file3
- `resolve_target_from_prepared_rows` makes `Auto` choose `HoldoutDatasetPass` whenever dataset rows exist in the selected prefix, and the test `prepared_row_prefix_auto_target_preserves_meta_usage` shows prefix `1` resolves to `HoldoutGate` while prefix `2` resolves to `HoldoutDatasetPass`. fileciteturn15file11

The second-pass issue is that dataset-pass mode also changes the CV unit.

When prepared rows are built in dataset-pass mode, every holdout-row dataset becomes its own row with `group_id = ds`, but the feature vector `x` is cloned from the same meta group for each of those rows. Then training computes CV folds from `fnv1a64(&r.group_id)` and uses the count of unique `group_id`s to decide whether CV is available. fileciteturn15file3 fileciteturn14file8

So the actual second-pass counterexample family is:

Take one meta artifact with one feature group and at least two valid holdout datasets. Dataset-pass mode will produce multiple rows with the same `x` but different dataset-name `group_id`s. CV then runs because there are multiple unique groups, even though there is only one meta artifact. Out-of-fold predictions are effectively trained on exact copies of the same feature vector from the same historical artifact. That means CV calibration evidence is not exact to the real “generalize to a new meta artifact” claim.

A repo-fit fix that stays inside `ml_confidence.rs` is:

- keep the current prepared-row surface if needed,
- but add a separate CV/calibration grouping key tied to meta artifact or meta group, not dataset name,
- and make `Auto` target resolution explicit in metrics and model metadata, rather than letting artifact availability silently change both target semantics and fold geometry.

I would also split `min_holdout_score_p10` into separate config fields for aggregate-holdout quantile logic and per-dataset median logic, because it is currently doing two conceptually different jobs.

### `ml_signal.rs`: the calibration is real, but the action surface is not

This file does the score-calibration part correctly enough:

- raw margins can be mapped through sigmoid or isotonic calibration in `apply_calibration_probability`, with fallback to a raw-margin probability map. fileciteturn15file2
- native score-cache construction writes `p_long` and `p_short` after applying calibration. fileciteturn14file4
- CSV ingestion prefers `p_long_cal` / `p_short_cal` and treats those as probability-space inputs. fileciteturn14file12

It also already admits that labels are thresholded, via the documented policy:

- `y_long = 1 when r_long > deadband`
- `y_short = 1 when r_short > deadband`
- samples in `[0, deadband]` are dropped. fileciteturn15file2

But the final policy is still the wedge:

- long if `p_long >= t_long` and `p_long - p_short_cmp >= delta`
- short if `p_short >= t_short` and `p_short - p_long >= delta`
- else flat. fileciteturn14file4

The tests confirm that this wedge, not calibration quality itself, decides action: `0.68/0.45` goes long, `0.44/0.69` goes short, `0.58/0.56` stays flat. fileciteturn15file10

So the second-pass lemma here is:

Let

\[
B=\{p_l=t_l\}\cup\{p_s=t_s\}\cup\{p_l-p_s=\delta\}\cup\{p_s-p_l=\delta\}.
\]

If a point \((p_l,p_s)\) is farther than \(\varepsilon\) from \(B\) in \(L_\infty\), then any perturbation of size at most \(\varepsilon\) leaves the chosen action unchanged.

That means global calibration metrics are not enough. What matters for action stability is the mass of score points near the threshold boundary set \(B\). Even a well-calibrated channel can produce unstable decisions if many examples sit near `t_long`, `t_short`, or the `delta` indifference lines.

A useful bound is:

\[
\Pr[\text{action flip under perturbation } \Delta]
\le
\Pr[d((p_l,p_s),B)\le \|\Delta\|_\infty].
\]

That is the right second-pass object here.

A repo-fit reparameterization that stays within current ownership is to keep two calibrated channels but replace `(t_long, t_short, delta)` with:

- a presence threshold \( \tau \) on \( q=\max(p_l,p_s) \),
- and an advantage threshold \( \kappa \) on \( m=\logit(p_l)-\logit(p_s) \).

Then:

- long if \(q \ge \tau\) and \(m \ge \kappa\),
- short if \(q \ge \tau\) and \(m \le -\kappa\),
- else flat.

That keeps the existing long/short decomposition but makes the decision surface easier to tune and easier to audit for boundary instability.

### `validation_stats.rs`: the missing-value policy is a discontinuity disguised as a gate

This file is mathematically simple, and second pass makes that simplicity more damning.

`sanitize_probability_gate` clamps any finite threshold into `[0,1]`, so `-1.0` becomes `0.0` and `2.0` becomes `1.0`. Then `passes_probability_gates` compares missing PSR/DSR as `0.0` whenever a threshold is set. The tests lock in that missing stats fail for positive thresholds but pass at zero threshold. fileciteturn15file0

So for a missing PSR value, the acceptance rule is exactly

\[
A(\tau)=\mathbf{1}\{\tau\le 0\},
\]

after sanitization.

That means the gate has a jump discontinuity at `0`. Moving a threshold from `0.0` to `10^{-12}` flips every missing-stat candidate from pass to fail. This is not a soft calibration choice. It is an implicit missing-data policy encoded as a threshold.

A repo-fit fix is to separate three concerns that are currently conflated:

- threshold validation / normalization,
- probabilistic gate value,
- missing-stat policy.

An explicit enum like `MissingAsFail | MissingAsPass | MissingAsUnknown` would remove the hidden discontinuity.

### `options_expression.rs`: exactify proves the ranking score is only provisional

This is where second pass changes the most.

First pass already noted the hand-set constants, including auto GPU gating and the `1.23x` vs `1.25x` promotion flip. fileciteturn16file15 fileciteturn16file18

Second pass adds a stricter claim: the score used to truncate candidates is not the same score used after exactification.

The file itself says the GPU ranking is “aggregate-first” and that `exactify_ranked_results` is the boundary where retained rows regain exact summary metrics, enrichment, and exact extreme-stress scoring before robust scores are recomputed. fileciteturn15file13

The robust score also has explicit fallback semantics:

- missing holdout behaves like search,
- missing harsher behaves like stress,
- and sealed metrics are excluded from tuple ranking. fileciteturn15file13 fileciteturn16file18

But ranking happens before exactify, and both CPU and GPU paths truncate to `top_k` before finalization. fileciteturn15file18

That gives an exact second-pass drift formula.

Let lane utilities be

\[
u_s,u_h,u_t,u_{hh},u_e
\]

for search, holdout, stress, harsher, and extreme respectively.

Then on the CPU path, before exactify:

- holdout is present if available,
- stress is present if available,
- harsher is missing and therefore falls back to stress,
- extreme is missing and therefore also falls back to stress.

So the pre-exactify CPU score is

\[
R_{\text{cpu,pre}}=0.45u_s+0.25u_h+0.30u_t.
\]

After exactify, the score becomes

\[
R_{\text{cpu,post}}=0.45u_s+0.25u_h+0.15u_t+0.10u_{hh}+0.05u_e.
\]

Therefore

\[
\Delta R_{\text{cpu}}
=
0.10(u_{hh}-u_t)+0.05(u_e-u_t).
\]

On the GPU aggregate-first path, harsher is often present before exactify but extreme is not, so the pre-exactify score is

\[
R_{\text{gpu,pre}}
=
0.45u_s+0.25u_h+0.15u_t+0.15u_{hh},
\]

and the post-exactify score is

\[
R_{\text{gpu,post}}
=
0.45u_s+0.25u_h+0.15u_t+0.10u_{hh}+0.05u_e,
\]

hence

\[
\Delta R_{\text{gpu}}
=
0.05(u_e-u_{hh}).
\]

That is the most valuable second-pass bound in the file, because it tells you exactly how much provisional ranking can drift when exact lanes are filled in later.

It also yields a concrete counterexample family: two tuples can tie or order one way before exactify and reverse after exactify, meaning pre-truncation can drop the eventual winner.

### Second-pass priority order

1. `ml_confidence.rs`: fix dataset-pass CV grouping so calibration evidence matches the real generalization unit.
2. `options_expression.rs`: stop letting pre-exactify robust scores determine irreversible truncation without a retention buffer or conservative proxy.
3. `ml_signal.rs`: reparameterize the action boundary from raw `(t_long,t_short,delta)` wedges into a presence-plus-advantage geometry.
4. `validation_stats.rs`: split missing-value policy from thresholding so threshold `0` stops acting as a hidden semantic switch.

The sharpest second-pass takeaway is:

This repo already has some real calibration machinery, but several important decisions are still made on provisional surrogates whose exactness changes later. The OX-08 lens does not just say “these thresholds are arbitrary.” It says some of the probabilities and scores are being interpreted more strongly than the current training, CV, or ranking geometry actually justifies.

---

## Third-pass answer

Based on the OX-08 third-pass overlay, the implementation-facing version is: do not try to solve this with a repo-wide redesign. Fix the places where current code claims more exactness than its present calibration, CV, or ranking geometry really supports, and keep ownership inside these four files. fileciteturn13file0

### Third-pass verdict

The smallest safe third-pass slice is not “learn new models everywhere.” It is four local patches:

1. **`ml_confidence.rs`** — decouple dataset-row identity from CV grouping.
2. **`options_expression.rs`** — retain a pre-exactify buffer, then exactify and rerank before the final `top_k` cut.
3. **`validation_stats.rs`** — make missing-value policy explicit rather than encoding it in threshold `0.0`.
4. **`ml_signal.rs`** — add boundary-mass diagnostics first, then optionally add an alternate confidence/advantage decision geometry.

Those are direct implementations of the new counterexample, bound, and stability lemma from the earlier passes.

### 1) `ml_confidence.rs`: patch CV grouping first

This is the highest-risk exactness problem.

Right now, prepared dataset-pass rows are built by cloning the same feature vector `x` across many per-dataset rows while assigning `group_id = ds`; then both logistic and GBDT CV use hashed `group_id` values to define folds. That lets one meta artifact masquerade as multiple unique CV groups if it has multiple holdout datasets. fileciteturn15file3 fileciteturn14file8

**Implementation-facing implication of the counterexample:**  
the training row needs two notions of identity:

- **row identity / provenance**: dataset name if you want to keep it for debugging
- **CV grouping identity**: meta-run identity, i.e. the thing that should stay together across folds

The clean local patch is:

- replace the internal `type DatasetRow = MlConfidenceDatasetRow` training alias with a private training struct, e.g.
  - `cv_group_id: String`
  - `x: Vec<f64>`
  - `y: f64`
  - optional `source_dataset: Option<String>` if desired for diagnostics
- keep `MlConfidencePreparedArtifactRows` and `MlConfidenceDatasetRow` unchanged on the public prepared-row surface
- in `build_dataset_from_prepared_rows`, when target is `HoldoutDatasetPass`, map every retained row from the same artifact to the same `cv_group_id`, using `artifact.ts` or a stable fallback like `artifact:{index}`
- route both logistic and GBDT fold assignment through `cv_group_id` instead of the public prepared-row `group_id`

That keeps external shape stable while fixing the exactness leak locally.

**Patch consequence:**  
if a prefix contains only one meta artifact but many dataset rows, CV should now skip with the existing “need at least 2 unique meta runs” warning rather than pretending to have multiple groups. That is the right outcome.

**Tests to add:**

- `dataset_pass_cv_groups_by_meta_run_not_dataset_name`
- `single_artifact_many_datasets_skips_cv_after_group_fix`
- keep `prepared_row_prefix_auto_target_preserves_meta_usage`, but add a warning assertion if `Auto` target resolution changes across prefixes

**Small phase-2 semantic cleanup inside the same file:**  
split `min_holdout_score_p10` into:

- `min_holdout_aggregate_score_p10`
- `min_holdout_dataset_score_median`

with backward-compatible fallback from the old field. Right now one config knob is covering two different label semantics. fileciteturn15file3

### 2) `options_expression.rs`: exactify before the irreversible cut

This is the cleanest third-pass patch because it is highly local and the second-pass drift bound tells you exactly why it matters.

The file explicitly says `exactify_ranked_results` is where retained rows regain exact summary metrics, enrichment, and exact extreme-stress scoring before robust scores are recomputed. But CPU and GPU ranking both truncate to `top_k` before finalization. fileciteturn15file13 fileciteturn15file18

**Implementation-facing implication of the bound:**

- CPU provisional ranking is missing `0.15` lane weight (`0.10` harsher + `0.05` extreme)
- GPU provisional ranking is usually missing only `0.05` lane weight (extreme)

So the CPU path is more drift-prone than the GPU path. That means the fix does not require new kernels or new models. It requires a **retention buffer before exactify**.

**Local patch shape:**

Add a helper like:

```rust
fn pre_exactify_retain_k(top_k: usize, sealed_finalist_count: usize, cpu_like: bool, total: usize) -> usize
```

and use it to retain more than `top_k` before `finalize_ranked_results`.

Recommended shape:

- CPU path: retain materially more than `top_k` because provisional missing-lane mass is `0.15`
- GPU path: retain a smaller buffer because provisional missing-lane mass is `0.05`
- after `finalize_ranked_results`, sort/resequence again if needed, then apply the final `top_k` truncate

Concretely:

- in `rank_compiled_tuples_cpu`, replace the pre-finalization `truncate(req.top_k.max(1))` with `truncate(retain_k)`
- then, after `finalize_ranked_results`, do the final `truncate(req.top_k.max(1))`
- in the GPU combined path, change `shortlisted_indices.truncate(...)` from `top_k` to `retain_k`
- in the GPU fallback path, do the same thing the CPU path does

This stays entirely inside `options_expression.rs`.

**What not to do in this slice:**

- do not redesign robust scoring
- do not change the backend defaulting rules
- do not try to learn a backend model in the same patch

The exactify-buffer patch is lower risk and directly addresses the rank-reversal counterexample.

**Tests to add:**

- `cpu_pre_exactify_buffer_preserves_post_exactify_winner`
- `gpu_pre_exactify_buffer_preserves_extreme_only_rank_flip`
- a small helper test that verifies `retain_k >= top_k` and `retain_k >= sealed_finalist_count`

**Optional instrumentation worth adding in the same file:**

- store `pre_exactify_robust_score` beside final `robust_score`
- compute `exactify_robust_score_delta = final - provisional`

That makes post-hoc calibration drift visible without changing selection logic.

### 3) `validation_stats.rs`: add an explicit missing policy enum

This file is small, so the third-pass fix should be small too.

Right now `sanitize_probability_gate` clamps into `[0,1]`, and `passes_probability_gates` compares missing PSR/DSR as `0.0`, which means threshold `0.0` silently means “missing passes” while any positive threshold means “missing fails.” fileciteturn15file0

**Implementation-facing implication of the discontinuity:**  
do not overload threshold value with missing-data semantics.

The local patch is:

```rust
pub enum ProbabilityGateMissingPolicy {
    CompareAsZero,
    MissingAsFail,
    MissingAsPass,
}
```

and then:

```rust
pub fn passes_probability_gates_with_policy(...)
pub fn passes_probability_gates(...) -> bool {
    passes_probability_gates_with_policy(..., ProbabilityGateMissingPolicy::CompareAsZero)
}
```

That preserves current callers while giving future callers a way to be explicit.

If desired, add a second helper:

```rust
pub fn sanitize_probability_gate_with_status(v: Option<f64>) -> (Option<f64>, bool)
```

where the boolean indicates clamping occurred. That lets upstream code surface warnings without breaking existing call sites.

**Tests to add:**

- `legacy_probability_gates_match_compare_as_zero_policy`
- `missing_as_fail_is_threshold_stable_at_zero`
- `missing_as_pass_is_threshold_stable_at_zero`
- `sanitize_probability_gate_with_status_reports_clamping`

This is the cleanest implementation of the second-pass step-discontinuity finding.

### 4) `ml_signal.rs`: diagnostics-first, geometry second

`ml_signal` already insists that threshold validation runs in probability space, and it already calibrates raw scores to probabilities. The unmodeled part is the decision geometry, not the score space. fileciteturn15file14 fileciteturn15file2

Because the second-pass lemma said action instability is driven by mass near the boundary set, the first implementation slice here should be **measurement**, not immediate redesign.

#### Step 1: add boundary-mass diagnostics

Add a helper inside `ml_signal.rs`:

```rust
fn decision_boundary_distance(point: ScorePoint, params: &MlSignalPolicyParams) -> Option<f64>
```

that returns the minimum distance to the active boundary set:

- `|p_long - t_long|`
- `|p_short - t_short|` when two-sided
- `|(p_long - p_short_cmp) - delta|`
- `|(p_short - p_long) - delta|` when two-sided

Then extend `MlSignalValidationCandidateResult` with fields like:

- `near_boundary_bars_eps_01`
- `near_boundary_bars_eps_02`
- `mean_boundary_distance`
- `min_boundary_distance`

and populate them inside `validate_ml_signal_candidates`, where `fill_requested_targets` is already iterating the score surface. fileciteturn15file18

That gives the repo the right stability diagnostic without changing the current policy contract.

#### Step 2: optional alternate geometry behind a default-preserving flag

After diagnostics exist, the next local slice is an **optional** policy geometry, not a replacement.

Add something like:

```rust
enum MlSignalDecisionGeometry {
    LegacyWedge,
    ConfidenceAdvantage,
}
```

and optional params:

- `presence_threshold`
- `advantage_threshold`

Then `score_target` branches:

- default: current wedge
- alternate: `q = max(p_long, p_short)` and `m = logit(p_long) - logit(p_short)`

This can stay entirely within `ml_signal.rs` because:

- score caches are already probability-space
- sweeps already expand threshold axes locally
- the result types already carry candidate params locally. fileciteturn15file14 fileciteturn15file18

**Tests to add:**

- `decision_boundary_distance_matches_known_examples`
- `validation_reports_near_boundary_mass`
- `confidence_advantage_geometry_preserves_long_only_contract`
- keep `score_target_respects_thresholds_and_delta` unchanged for the legacy default. fileciteturn15file10

### Third-pass implementation priority

If I were turning the third pass into code, I would do it in this order:

1. **`options_expression.rs`** — add pre-exactify retention buffers and final truncation after exactify.
2. **`ml_confidence.rs`** — fix dataset-pass CV grouping so calibration evidence matches the true generalization unit.
3. **`validation_stats.rs`** — add explicit missing-policy support while preserving legacy wrapper behavior.
4. **`ml_signal.rs`** — add boundary-mass diagnostics first; only then add an optional alternate decision geometry.

### Sharp third-pass takeaway

The third-pass implementation lesson is:

- `ml_confidence` needs a truer CV unit
- `options_expression` needs a truer truncation point
- `validation_stats` needs a truer missing-value contract
- `ml_signal` needs truer stability diagnostics before it needs a new policy surface

That is how the OX-08 calibration / inverse-modeling lens should cash out in code without redesigning adjacent ownership surfaces.
