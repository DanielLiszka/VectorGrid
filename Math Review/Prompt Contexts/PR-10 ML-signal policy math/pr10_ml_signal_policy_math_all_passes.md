# PR-10 ML-signal policy math — all three passes

Compiled from this chat. The third-pass scope and deliverables came from the uploaded prompt file. fileciteturn21file0


## First-pass answer

Below is the first-pass answer for `PR-10`, staying inside the owned surface, treating the listed context files as constraints, and not proposing implementation yet. fileciteturn21file0

```md
## `PR-10` ML-signal policy math

Owned problem:
ML-signal label design, calibration transforms, threshold or delta rules, and
overlay or sweep behavior

Source packets:
`MR-17`

Primary ownership files:

- `crates/app_core/src/ml_signal.rs`

Context files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Own these symbols or decisions:

- label deadband and split-window design
- calibration interpolation and probability-contract checks
- threshold or delta policy geometry
- persistence and cooldown overlays
- sweep ranking and truncation rules

Open problem:

- characterize a policy-ranking geometry for ML-signal outputs that turns score
  caches into stable threshold/delta decisions under calibration error,
  dependence, and overlay logic

Why this is hard here:

- label design, score calibration, and threshold policies are all coupled
- the signal layer consumes validation-style primitives but must produce a
  policy surface rather than just a score surface

Mathematical object:

- a map from calibrated score sequences and overlay states to discrete
  threshold/delta actions, together with a sweep functional over candidate
  policy settings

Notation seed:

- let `p_t \in [0,1]` be the calibrated score at time `t`
- let `s_t` denote any overlay state (cooldown, persistence, position, etc.)
- let `\theta = (\tau_{\uparrow}, \tau_{\downarrow}, h, c, \dots)` denote a
  policy parameter tuple
- let `a_t = \pi_\theta(p_{1:t}, s_t)` be the induced action
- let `J(\theta)` be the sweep objective used to rank candidate policies

Primary questions:

- what threshold geometry yields stable policy ranking under calibration error?
- when can overlay or cooldown logic be compressed into a finite sufficient
  state?
- what sweep truncation rule preserves the true best policy under bounded rank
  error?

Required deliverables:

- formal policy-surface model
- candidate theorem set for stable threshold ranking
- counterexamples where calibration or dependence breaks naive thresholding
- explicit rank-instability or regret bounds for sweep truncation

Acceptance bar:

- every policy claim must separate score quality from decision-rule quality

Verification hooks:

- export/native-smoke parity
- calibration drift falsifiers
- threshold-order instability checks

Non-goals:

- do not redesign the generic validation stack; that belongs to `PR-06`
- do not redesign meta-select or ML-confidence policy; that belongs to `PR-07`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat score-cache coverage rules as first-class math contracts
- treat export and native-smoke flows as verification context, not redesign
  targets
```

### 1. Formal problem statement

The clean first-pass formalization is a two-layer search problem.

The upstream layer chooses a label-and-calibration contract
\[
u=(h,D,W,C,\mathcal K),
\]
where \(h\) is label horizon, \(D\) is the deadband rule, \(W\) is the split-window plan, \(C\) is the calibration map from raw margins to probabilities, and \(\mathcal K\) is the score-cache admissibility contract. The downstream layer chooses a signal policy
\[
\theta=(\tau_L,\tau_S,\Delta,H,e_{\mathrm{flat}},e_{\mathrm{opp}},\chi),
\]
where \(\tau_L,\tau_S\) are long/short thresholds, \(\Delta\) is the long-vs-short gap requirement, \(H\) is the maximum hold, \(e_{\mathrm{flat}}\) and \(e_{\mathrm{opp}}\) are exit rules, and \(\chi\) denotes fixed strategy-side overlay bits such as long-only, allow-flip, trade-on-next-bar, persistence, and cooldown.

The core object is therefore
\[
(u,\theta;X_{1:T}) \longmapsto \big(r_{1:T},a_{1:T},M(\theta),J(\theta),G(\theta)\big),
\]
on a fixed candle path \(X_{1:T}\), where \(r_t\) is the requested target symbol, \(a_t\) is the desired position after overlays, \(M(\theta)\) is the simulated metrics vector, \(J(\theta)\) is the ranking score, and \(G(\theta)\) is the approval or gating status.

### 2. Notation for the search space, state space, path functional, ranking functional, and admissible approximations

Let \(X_{1:T}=(O_t,H_t,L_t,C_t)\) be the candle path.

Define the upstream label contract by
\[
R_t^L(h)=\frac{O_{t+h+1}-O_{t+1}}{O_{t+1}},\qquad
R_t^S(h)=\frac{O_{t+1}-O_{t+h+1}}{O_{t+1}},
\]
and
\[
D_t(u)=\max\!\left(2\kappa,\ \lambda\frac{\mathrm{ATR}_{t,h}}{O_{t+1}}\right),
\]
where \(\kappa\) is round-trip cost in fraction form and \(\lambda\) is the ATR floor coefficient.

Define the split-window plan
\[
W=(n_{\mathrm{tr}},n_{\mathrm{cal}},n_{\mathrm{te}},g,s),
\]
where train, calibration, test, gap, and step are the rolling-window primitives.

Define the calibration family \(C_\psi\colon z_t\mapsto p_t\in[\varepsilon_0,1-\varepsilon_0]\), with current admissible forms including sigmoid and piecewise-linear monotone interpolation.

Define the admissible score-cache contract
\[
\mathcal K=(\text{space},\text{sides},\text{scope},\text{coverage}).
\]

For a fixed candidate \(\theta\), define the request alphabet
\[
\mathcal R=\{\bot,-1,0,+1\},
\]
where \(\bot\) means “no request because the row is unusable.”

Define the extended overlay state
\[
x_t=(q_t,h_t,c_t,u_t,n_t),
\]
where \(q_t\in\{-1,0,+1\}\) is current position, \(h_t\) is held bars, \(c_t\) is cooldown remaining, \(u_t\in\{-1,0,+1\}\) is pending sign, and \(n_t\) is pending persistence count.

The path functional is
\[
M(\theta)=\mathrm{Sim}\big(a_{1:T};X_{1:T}\big),
\]
and the ranking functional is
\[
J(\theta)=\mathrm{robustScore}(M(\theta)).
\]

### 3. Fixed invariants and interface constraints

The threshold geometry is a two-threshold plus delta-gap wedge.

Score caches are part of the mathematical domain, not just metadata. Probability space is required, side-presence rules are explicit, and scope affects what claims are admissible.

The overlay layer is signal-side, not execution-side. The desired-position path is materialized by the ML-signal layer before simulation.

The current ranking target is robust score, while approval is a separate hard-gate and verdict layer.

### 4. Strongest plausible positive result

The strongest plausible positive result is a cell decomposition theorem:

For fixed candles, fixed admissible score-cache support, and fixed policy \(\theta\), the calibrated score-path space decomposes into cells cut out by threshold/delta hyperplanes. Inside one cell, the request word is constant. Since the overlay layer is a deterministic finite-state transducer on that request word, the desired-position path is constant. Since the simulator only sees desired positions and candles, the metrics vector and robust score are constant on that cell.

A corollary is exact stability inside a margin-separated cell: if a calibration perturbation is uniformly smaller than the distance from all active bars to all active boundaries, the candidate-level policy output and score are unchanged.

### 5. Strongest plausible impossibility result

Without a boundary-margin assumption, there is no nontrivial uniform stability theorem for threshold ranking under calibration error.

There is also no general theorem recovering the “true best approved candidate” from a purely ranking-based surrogate unless approval is built into the objective itself.

### 6. Candidate theorem or lemma list

1. Finite sufficient state lemma.
2. Boundary-margin exactness lemma.
3. Exact action invariance corollary.
4. Missing-side admissibility lemma.
5. Scope/coverage admissibility proposition.
6. Approximate instability bound theorem candidate.
7. Top-k preservation theorem under uniform score error.
8. Approval-aware non-equivalence proposition.
9. Monotone-recalibration non-identifiability proposition.
10. Partial-cache non-identifiability proposition.

### 7. One explicit counterexample family

Take a long-only setting with \(\Delta=0\), no short side, hold \(H=1\), no cooldown, and no persistence. Let two candidate thresholds be
\[
\tau_A=\tau+\varepsilon,\qquad \tau_B=\tau+2\varepsilon,
\]
and let calibrated long scores alternate around those thresholds. Under one calibration, candidate \(A\) trades profitable even bars while candidate \(B\) trades none; under an arbitrarily small monotone shift, \(A\) begins trading all bars including the losing ones while \(B\) trades only the profitable bars. Ranking reverses.

### 8. One lower-bound, identifiability, or impossibility angle

Threshold policy quality is not identifiable from rank quality alone. Strictly increasing recalibrations preserve order but can move absolute-threshold crossings.

Similarly, full-dataset best-policy claims are not identifiable from partial caches without an explicit scope-qualified weakening.

### 9. Unresolved questions that block implementation

1. What calibration-error metric should theorem statements use?
2. What exact calibration-map contract should monotone interpolation satisfy?
3. Is the target of sweep truncation pure robust-score rank or approval-aware selection?
4. What simulator sensitivity assumption converts action-path perturbations into score regret?
5. What claim class is allowed for `CalibrationSlice` and `CustomPartial`?
6. Is the long-only short-side-as-zero comparison intended or incidental?
7. Should \(H=0\) be treated as unbounded hold at theorem level?
8. How should export-side keep masks interact with downstream score-cache admissibility?
9. What exact notion of “true best policy” is required for truncation claims?

The strongest first-pass conclusion was: the ML-signal policy surface should be modeled as a score-cache-contract-restricted, piecewise-constant cell complex in calibrated score space, composed with a finite-state overlay transducer and a separate approval layer.


---


## Second-pass answer

Here is the second-pass answer, following the prompt’s adversarial brief. fileciteturn21file0

```md
## `PR-10` ML-signal policy math

Owned problem:
ML-signal label design, calibration transforms, threshold or delta rules, and
overlay or sweep behavior

Source packets:
`MR-17`

Primary ownership files:

- `crates/app_core/src/ml_signal.rs`

Context files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Own these symbols or decisions:

- label deadband and split-window design
- calibration interpolation and probability-contract checks
- threshold or delta policy geometry
- persistence and cooldown overlays
- sweep ranking and truncation rules

Open problem:

- characterize a policy-ranking geometry for ML-signal outputs that turns score
  caches into stable threshold/delta decisions under calibration error,
  dependence, and overlay logic

Why this is hard here:

- label design, score calibration, and threshold policies are all coupled
- the signal layer consumes validation-style primitives but must produce a
  policy surface rather than just a score surface

Mathematical object:

- a map from calibrated score sequences and overlay states to discrete
  threshold/delta actions, together with a sweep functional over candidate
  policy settings

Notation seed:

- let `p_t \in [0,1]` be the calibrated score at time `t`
- let `s_t` denote any overlay state (cooldown, persistence, position, etc.)
- let `\theta = (\tau_{\uparrow}, \tau_{\downarrow}, h, c, \dots)` denote a
  policy parameter tuple
- let `a_t = \pi_\theta(p_{1:t}, s_t)` be the induced action
- let `J(\theta)` be the sweep objective used to rank candidate policies

Primary questions:

- what threshold geometry yields stable policy ranking under calibration error?
- when can overlay or cooldown logic be compressed into a finite sufficient
  state?
- what sweep truncation rule preserves the true best policy under bounded rank
  error?

Required deliverables:

- formal policy-surface model
- candidate theorem set for stable threshold ranking
- counterexamples where calibration or dependence breaks naive thresholding
- explicit rank-instability or regret bounds for sweep truncation

Acceptance bar:

- every policy claim must separate score quality from decision-rule quality

Verification hooks:

- export/native-smoke parity
- calibration drift falsifiers
- threshold-order instability checks

Non-goals:

- do not redesign the generic validation stack; that belongs to `PR-06`
- do not redesign meta-select or ML-confidence policy; that belongs to `PR-07`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat score-cache coverage rules as first-class math contracts
- treat export and native-smoke flows as verification context, not redesign
  targets
```

### 1. Explicit failure modes

The first-pass answer overmodeled the repo. The current sweep does not optimize over a live upstream object \(u=(h,D,W,C,\mathcal K)\) at runtime. It loads a frozen score cache, checks a narrow contract, and evaluates threshold candidates against cached `p_long/p_short`. Metadata about calibration or score contracts may be present, but much of it is only recommended, not enforced.

The first pass also understated the semantic difference between a flat request and no request. In the code, `score_target` returns `Some(0)` when finite scores exist but do not cross thresholds, while absent or unusable scores produce `None`. `fill_requested_targets` maps `None` to `ML_SIGNAL_NO_REQUEST`, and `fill_desired_positions` treats `0` as a possible close under `exit_on_flat` while `NO_REQUEST` leaves the current position unchanged.

A sharper bug is that support pattern alone is too weak. `actual_score_obs` is computed as `point.is_some()`, not as “number of usable finite request rows.” So gate metrics can change even when the request word and desired positions do not.

The biggest break is at sweep-level approval: the code evaluates every candidate, sorts by `robust_score`, truncates to `top_k`, and only then rebuilds `run_verdict` on that truncated slice. Therefore “best approved policy in the full evaluated set” is not the stored sweep object.

Another pathology is warning propagation: score-scope warnings are appended after `build_run_verdict`, so warning flags and final recommendation are not synchronized.

The coverage story is also scope-dependent: `FullDataset` is strict, `WalkforwardTest` is accepted without a scope warning, and `CalibrationSlice`/`CustomPartial` are warning-qualified.

Finite-grid effects were also underplayed. The sweep grid is finite, rounded, and multiset-like; candidate identity and shortlist contents can be sensitive to tie behavior and duplicate parameter tuples.

### 2. Adversarial constructions or separating examples

**A. Missing row versus flat row.**
With `exit_on_flat = true` and an open position, a finite row that yields `0` exits, while a missing row that yields `NO_REQUEST` does not.

**B. Truncated-slice approval reversal.**
If all candidates in ranks 1–25 fail approval but rank 26 passes, then `top_k = 25` produces a stored sweep result with no approved candidate even though the full evaluated set had one.

**C. Observation-count inflation without action change.**
A matched row with nonfinite probabilities contributes to `actual_score_obs` but may contribute nothing to the request word.

**D. Duplicate-grid crowding.**
Near-duplicate axis values can collapse onto the same effective threshold tuple and still occupy multiple shortlist slots.

**E. Non-unique split membership.**
With overlapping rolling windows, a row can belong to multiple logical windows while export writes only the first matching `split_id`.

### 3. Corrected theorem or lemma candidates

1. Request-word cell decomposition survives, but it is over the alphabet \(\{\bot,0,+1,-1\}\).
2. Finite-state overlay transducer survives for fixed candidate and fixed horizon.
3. Per-candidate metrics constancy survives once the desired-position path is fixed.
4. Missing-side admissibility survives.
5. Scope-conditioned claim classes survive.
6. No repo-fit pruning theorem exists yet, because the current implementation evaluates exhaustively.

### 4. Weakest assumptions under which the result survives

The result survives under:
- fixed candidate list after axis expansion,
- fixed support pattern at the level of matched rows and per-side finiteness,
- fixed base-strategy bits outside the sweep axes,
- fixed simulator config and candle path,
- claims restricted to per-candidate request words, desired positions, metrics, and robust scores,
- explicit tie assumptions for shortlist-order claims,
- shortlist-local—not full-set—semantics for approval claims.

### 5. Distinction between exact, approximate, and heuristic claims

**Exact:** per-candidate request-word cells, finite-state overlay evolution, and metrics constancy once desired positions are fixed.

**Approximate:** perturbation results that rely on threshold margins and fixed field/finiteness patterns.

**Heuristic only:** claims that infer sweep ranking from metadata not enforced by the runtime surface, or claims about pruning preserving the true best policy on a system that currently does no early pruning.

### 6. Revised time, memory, and retained-state costs

For \(N\) candidates and \(T\) bars, the current sweep is
\[
O(NT)+O(N\log N),
\]
with \(O(T)\) working buffers and \(O(N)\) retained results. The retained overlay state per candidate evaluation is constant size.

### 7. Dependency burden on other PR prompts

The dependency burden is narrower than first pass suggested.

`validation_pipeline.rs` is an interface defining robust-score, PSR/DSR/min-track-record, gates, and verdict logic.

`iterative_sim.rs` is an interface defining deterministic metrics from desired positions.

`commands_ml_signal.rs` constrains the theorem domain through validation-context checks.

The headless binaries remain verification context only.

### Strongest surviving repo-fit result

The strongest surviving result after attack is:

For a fixed expanded candidate list, fixed candle path, fixed base-strategy overlay configuration, and fixed score-cache support/finiteness pattern, each candidate induces a 4-symbol request word
\[
r_t \in \{\bot,0,+1,-1\}
\]
that is piecewise constant on a threshold/delta cell complex in calibrated-score space. The desired-position path is then an exact deterministic finite-state transform of that request word, and the simulated metrics plus robust score are exact on each cell. Everything stronger—especially end-to-end upstream identifiability, approval preservation under truncation, or pruning guarantees for the true best approved policy—fails on the present surface.


---

## Third-pass answer


Here is the third-pass answer, narrowed to the strongest implementation-ready result that survives the second pass. I followed the third-pass scope and deliverables from the uploaded prompt. fileciteturn21file0

```md
## `PR-10` ML-signal policy math

Owned problem:
ML-signal label design, calibration transforms, threshold or delta rules, and
overlay or sweep behavior

Source packets:
`MR-17`

Primary ownership files:

- `crates/app_core/src/ml_signal.rs`

Context files:

- `crates/app_core/src/validation_pipeline.rs`
- `crates/app_core/src/iterative_sim.rs`
- `src-tauri/src/commands_ml_signal.rs`
- `crates/app_core/src/bin/ta_ml_signal_export_dataset.rs`
- `crates/app_core/src/bin/ta_ml_signal_native_smoke.rs`

Own these symbols or decisions:

- label deadband and split-window design
- calibration interpolation and probability-contract checks
- threshold or delta policy geometry
- persistence and cooldown overlays
- sweep ranking and truncation rules

Open problem:

- characterize a policy-ranking geometry for ML-signal outputs that turns score
  caches into stable threshold/delta decisions under calibration error,
  dependence, and overlay logic

Why this is hard here:

- label design, score calibration, and threshold policies are all coupled
- the signal layer consumes validation-style primitives but must produce a
  policy surface rather than just a score surface

Mathematical object:

- a map from calibrated score sequences and overlay states to discrete
  threshold/delta actions, together with a sweep functional over candidate
  policy settings

Notation seed:

- let `p_t \in [0,1]` be the calibrated score at time `t`
- let `s_t` denote any overlay state (cooldown, persistence, position, etc.)
- let `\theta = (\tau_{\uparrow}, \tau_{\downarrow}, h, c, \dots)` denote a
  policy parameter tuple
- let `a_t = \pi_\theta(p_{1:t}, s_t)` be the induced action
- let `J(\theta)` be the sweep objective used to rank candidate policies

Primary questions:

- what threshold geometry yields stable policy ranking under calibration error?
- when can overlay or cooldown logic be compressed into a finite sufficient
  state?
- what sweep truncation rule preserves the true best policy under bounded rank
  error?

Required deliverables:

- formal policy-surface model
- candidate theorem set for stable threshold ranking
- counterexamples where calibration or dependence breaks naive thresholding
- explicit rank-instability or regret bounds for sweep truncation

Acceptance bar:

- every policy claim must separate score quality from decision-rule quality

Verification hooks:

- export/native-smoke parity
- calibration drift falsifiers
- threshold-order instability checks

Non-goals:

- do not redesign the generic validation stack; that belongs to `PR-06`
- do not redesign meta-select or ML-confidence policy; that belongs to `PR-07`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- treat score-cache coverage rules as first-class math contracts
- treat export and native-smoke flows as verification context, not redesign
  targets
```

## 1. Final formalization

The current repo does **not** search over a live upstream tuple of label policy, split plan, and calibration map at validation/sweep time. The exact runtime surface starts from a **loaded score cache** and a fixed candle path, then applies threshold/delta rules, overlay logic, iterative simulation, robust-score ranking, and post-hoc verdict building. That is the implementation-ready mathematical object. The score cache is hard-contracted to `MlSignal`, the expected data id, the expected feature-schema version, and deduplicated timestamps; for `FullDataset` scope it must also match the candle-span endpoints and clear a 0.95 coverage threshold. Probability-space scores are mandatory, long probabilities are always required, and short probabilities are additionally required whenever any candidate is not long-only. fileciteturn27file15 fileciteturn25file3 fileciteturn22file12

Fix candles
\[
X_{1:T}=(O_t,H_t,L_t,C_t)_{t=1}^T,
\]
a fixed iterative-sim config \(\iota\), and a fixed base strategy overlay configuration
\[
\chi=(\text{long\_only},\text{trade\_on\_next\_bar},\text{allow\_flip},\rho,c,\dots).
\]
Fix also a loaded admissible score cache
\[
K=(\text{point}_t,\sigma)_{t=1}^T,
\]
where \(\text{point}_t\) is either absent or contains optional finite/nonfinite \(p_t^L,p_t^S\), and \(\sigma\) is the cache summary. At the owned policy layer, a candidate is
\[
\theta=(\tau_L,\tau_S,\Delta,H,e_{\mathrm{flat}},e_{\mathrm{opp}}),
\]
with the remaining strategy bits inherited from \(\chi\). The current sweep expands exactly these axes, multiplies the axis lengths, rejects any expansion above 20,000, and then evaluates the entire expanded candidate multiset exhaustively. fileciteturn26file17

Define the local request alphabet
\[
\mathcal R=\{\bot,0,+1,-1\}.
\]
This four-symbol alphabet is essential. In repo semantics:
- \(\bot\) means **no request** because the row is absent or unusable,
- \(0\) means a finite score row exists but requests flat,
- \(+1\) requests long,
- \(-1\) requests short.

The distinction between \(\bot\) and \(0\) is exact because `fill_requested_targets` fills the request array with `ML_SIGNAL_NO_REQUEST = i32::MIN`, while `score_target` returns `Some(0)` for finite rows that clear neither threshold wedge. `fill_desired_positions` treats `0` as a possible close under `exit_on_flat`, but treats `NO_REQUEST` as “leave the current position alone and reset pending persistence state.” fileciteturn25file0 fileciteturn25file2 fileciteturn25file6

For each bar, define a field/finiteness state
\[
F_t \in \{\bot,L,LS\},
\]
where \(\bot\) means no usable finite long probability, \(L\) means finite long with missing/nonfinite short, and \(LS\) means both finite long and finite short. Then the current request map induced by `score_target` is
\[
u_t^\theta \in \mathcal R
\]
with
\[
u_t^\theta=
\begin{cases}
\bot, & \text{if point}_t=\bot \text{ or } p_t^L \text{ is missing/nonfinite},\\
+1, & p_t^L\ge \tau_L \text{ and } p_t^L-\tilde p_t^S\ge \Delta,\\
-1, & \neg\text{long\_only},\ p_t^S\ge \tau_S,\ p_t^S-p_t^L\ge \Delta,\\
0, & \text{otherwise},
\end{cases}
\]
where \(\tilde p_t^S=p_t^S\) if finite and \(\tilde p_t^S=0\) for the long comparison if short is absent or nonfinite. This long-side asymmetry is implementation reality, not an abstract convention. fileciteturn25file2

Let \(S_\chi\) be the trade-timing shift:
\[
r_t^\theta =
\begin{cases}
u_{t-1}^\theta, & \text{if trade\_on\_next\_bar is on and } t\ge 2,\\
u_t^\theta, & \text{otherwise},
\end{cases}
\]
with out-of-range values left at \(\bot\). That is exactly what `fill_requested_targets` does. fileciteturn25file0

Now define the exact owned overlay state
\[
x_t=(q_t,h_t,c_t,u_t,n_t),
\]
where:
- \(q_t\in\{-1,0,+1\}\) is current position,
- \(h_t\in\{0,\dots,H^\star\}\) is held bars,
- \(c_t\in\{0,\dots,c\}\) is cooldown remaining,
- \(u_t\in\{-1,0,+1\}\) is pending sign,
- \(n_t\in\{0,\dots,\rho\}\) is pending count.

On a fixed horizon \(T\), \(H^\star=H\) if \(H>0\) and \(H^\star=T\) if the interface semantics use \(H=0\) for unbounded hold. The transition
\[
x_{t+1}=F_{\theta,\chi}(x_t,r_t^\theta)
\]
is exactly the logic in `fill_desired_positions`: on-bar-start cooldown decrement; max-hold forced close; long-only clipping; persistence accumulation; entry cooldown blocking; entry from flat; optional exit on flat; optional exit/flip on opposite; pending reset on \(\bot\). The desired-position output is
\[
d_t^\theta = q_t.
\]
This is the exact policy layer. fileciteturn25file6 fileciteturn26file16

The simulator layer does **not** re-run this same persistence/cooldown policy, because `ml_signal.rs` explicitly neutralizes those execution overlays before calling the iterative simulator:
\[
\eta(\chi)=\chi \text{ with allow\_flip=true, cooldown\_bars=0, persistence\_bars=1.
\]
Then
\[
M(\theta)=\mathrm{Sim}(X_{1:T},d^\theta,\eta(\chi),\iota),
\qquad
J(\theta)=\mathrm{robustScore}(M(\theta)).
\]
That separation is exact and necessary: the owned policy layer builds the desired positions, while `iterative_sim.rs` then consumes those positions with a neutralized execution overlay configuration, plus commission/slippage/intrabar-stop logic from the sim config. fileciteturn25file6 fileciteturn25file8 fileciteturn25file12 fileciteturn27file4

There is then a separate gate layer. The repo defines
\[
N_{\mathrm{obs}}=\#\{t:\text{point}_t\text{ is present}\},
\]
not the number of usable finite requests. That value is stored as `track_record_bars`, duplicated across candidates as `sample_obs`, and then used downstream by PSR/DSR/min-track-record machinery. So the exact gate functional is
\[
G_{\mathrm{repo}}(\theta)=\mathrm{Gate}\big(M(\theta),N_{\mathrm{obs}}\big),
\]
not a pure function of the request word or desired-position path. fileciteturn25file8 fileciteturn26file5 fileciteturn27file13

Finally, the exact sweep output is
\[
\mathrm{Sweep}_{\mathrm{repo}}(K,\Theta,\text{top}_k),
\]
where \(\Theta\) is the fully expanded candidate multiset. The repo evaluates all of \(\Theta\), sorts by decreasing \(J(\theta)\), truncates to `top_k` if requested, and then rebuilds the run verdict **on the truncated shortlist**, with `candidates_tested = |\Theta|`. So the exact sweep target is **top-k robust-score shortlist plus shortlist-local verdict**, not “best approved candidate in the full evaluated set.” fileciteturn27file0

## 2. Final theorem or lemma list

### Theorem 1. Cache-admissibility domain theorem

The current policy geometry is defined only on score caches that satisfy the repo contract:
1. `score_space == Probability`,
2. long probabilities are present on every probability row,
3. short probabilities are present on every probability row whenever any candidate is not long-only,
4. duplicate timestamps are absent,
5. for `FullDataset`, endpoint span matches the candle span and coverage ratio is at least 0.95.

Outside this admissible set, validation/sweep is undefined because the code errors before policy evaluation. fileciteturn22file12 fileciteturn25file3 fileciteturn27file15

### Theorem 2. Four-symbol request-word cell decomposition theorem

Fix a candidate \(\theta\), a candle path, and a field/finiteness pattern \(F_{1:T}\). Then calibrated-score space decomposes into cells cut out by the active affine walls
\[
p_t^L=\tau_L,\qquad
p_t^S=\tau_S,\qquad
p_t^L-p_t^S=\Delta,\qquad
p_t^S-p_t^L=\Delta
\]
on bars where the corresponding fields are finite, together with the side-condition \(F_t\in\{\bot,L,LS\}\). On each cell, the local request word \(u_{1:T}^\theta\), and hence the shifted request word \(r_{1:T}^\theta\), is constant.

### Theorem 3. Exact finite-state sufficiency theorem for the policy layer

For a fixed candidate \(\theta\), fixed base strategy \(\chi\), and fixed horizon \(T\), the exact sufficient statistic for mapping the request word \(r_{1:T}^\theta\) to the desired-position path \(d_{1:T}^\theta\) is the five-component state
\[
x_t=(q_t,h_t,c_t,u_t,n_t).
\]
No deeper history is needed.

### Theorem 4. Per-candidate metrics equivalence theorem

If two admissible score-cache realizations induce the same desired-position path \(d_{1:T}^\theta\) for a fixed candidate \(\theta\), then they induce the same iterative metrics \(M(\theta)\) and the same robust score \(J(\theta)\), because the iterative simulator receives only candles, desired positions, normalized strategy, and sim config. fileciteturn25file8 fileciteturn25file12

### Theorem 5. Boundary-margin zero-error theorem

Fix \(\theta\). Let
\[
\gamma_\theta
=
\min_{t \in \mathcal I_\theta}
\operatorname{dist}\!\left((p_t^L,p_t^S), \mathcal B_\theta(t)\right),
\]
where \(\mathcal I_\theta\) is the set of bars whose finite fields are active for \(\theta\), and \(\mathcal B_\theta(t)\) is the union of the active threshold/delta walls at bar \(t\). If a perturbation of the calibrated scores preserves the field/finiteness pattern \(F_{1:T}\) and satisfies
\[
\|\delta p\|_{\infty} < \gamma_\theta,
\]
then the request word, desired-position path, metrics, and robust score are all unchanged for \(\theta\). Hence the per-candidate regret is exactly zero.

### Theorem 6. Policy-layer non-collapse theorem

There is no exact sufficient statistic for the policy layer that depends only on the current bar request \(r_t^\theta\), nor one that depends only on \((r_t^\theta,q_t)\). Cooldown, pending persistence, and held-bar count are semantically active. Therefore any attempt to compress the overlay map below \((q_t,h_t,c_t,u_t,n_t)\) loses exactness on the current surface.

### Theorem 7. Gate-layer non-pathwise theorem

There is no exact sufficient statistic for the full candidate post-processing layer based only on the desired-position path or request word. The repo gate layer depends on
\[
N_{\mathrm{obs}}=\#\{t:\text{point}_t\text{ is present}\},
\]
which can differ even when the request word and desired-position path are unchanged. Therefore approval, PSR, DSR, and minimum-track-record warnings are not pure decision-path functionals. fileciteturn25file8 fileciteturn26file5

### Theorem 8. Exact current-sweep theorem

The current sweep computes the exact exhaustive shortlist
\[
\operatorname{TopK}_J(\Theta)
\]
for the expanded candidate multiset \(\Theta\), where ranking is by descending robust score and shortlist size is controlled only after all candidates are evaluated. The verdict stored in the sweep result is the verdict of that **truncated sorted slice**, not of the full candidate set. There is no exact pruning theorem to prove for the current implementation, because the current implementation does not prune before evaluation. fileciteturn27file0

### Theorem 9. Warning/recommendation decoupling theorem

Because score-scope warning flags are appended after `build_run_verdict`, the presence of a score-scope warning flag is not logically equivalent to the stored `final_recommendation`. Any theorem equating “warning flags nonempty” with “recommendation must be ApprovedWithWarnings” is false on the current surface. fileciteturn22file12

## 3. Proof sketch or proof plan

Theorem 1 is immediate from `validate_score_cache_contract` and the `load_score_cache` checks. Theorem 2 follows by inspection of `score_target`: with fixed field/finiteness pattern, the only predicates are comparisons to \(\tau_L,\tau_S,\Delta\), so the request symbol is constant on each open cell of the induced arrangement. `trade_on_next_bar` only shifts the already constant word by one index. fileciteturn25file2 fileciteturn25file0

Theorem 3 follows directly from `fill_desired_positions`. Every branch reads only the current request symbol, the current position, held-bar count, cooldown-left, and pending persistence state; the next state is written deterministically from those values. No older history survives except through these five registers. fileciteturn25file6

Theorem 4 follows because `validate_ml_signal_candidates` passes the desired-position path into `simulate_iterative_from_signals` together with a normalized strategy and fixed sim config. If those inputs are equal, the simulator output is equal. fileciteturn25file8 fileciteturn25file12

Theorem 5 is the direct composition of Theorems 2–4. If the perturbation does not cross an active boundary and does not change the field/finiteness pattern, then the request word is identical, then the desired-position path is identical, then metrics and robust score are identical.

Theorem 6 is proved by separating examples. Hold everything fixed except one state component:
- same request \(r_t=+1\), same current position \(q_t=0\), but \(c_t>0\) versus \(c_t=0\) gives “blocked entry” versus “enter now”;
- same request \(r_t=+1\), same current position \(q_t=0\), but \(u_t=+1,n_t=\rho-1\) versus \(u_t=0,n_t=0\) gives “persistence threshold reached” versus “not yet”;
- same request \(r_t=\bot\), same current position \(q_t=+1\), but \(h_t=H\) versus \(h_t<H\) changes whether max-hold exits on the current bar.
So any smaller statistic fails.

Theorem 7 is proved by the explicit repo definition of `actual_score_obs = point.is_some()` and the later use of that count for probability metrics and min-track-record logic. A `Some(point)` with missing/nonfinite long can contribute to \(N_{\mathrm{obs}}\) while still producing \(\bot\) at the request layer. fileciteturn25file8 fileciteturn27file13

Theorem 8 is immediate from `run_ml_signal_policy_sweep_blocking_with_candles`: expand, validate, evaluate all, sort by `robust_score`, truncate, rebuild verdict on truncated slice. Theorem 9 follows from the order “build verdict, then append score-scope warning flag.” fileciteturn27file0 fileciteturn22file12

## 4. Sufficient statistics, or a proof they do not exist under the stated constraints

There are **two different answers**, one exact and one impossibility result.

For the **owned policy layer**, exact sufficient statistics do exist. The minimal repo-fit state is
\[
(q_t,h_t,c_t,u_t,n_t),
\]
and together with the current request symbol \(r_t^\theta\) it is sufficient to evolve the desired-position path exactly.

For the **full validation/sweep result**, such a compact sufficient statistic does **not** exist if one insists on exact repo output. The missing ingredients are:
1. the field/presence structure that determines `actual_score_obs`,
2. the full candidate multiset \(\Theta\),
3. the sorted robust-score order,
4. the `top_k` truncation boundary,
5. the shortlist-local verdict build.

So the strongest exact sufficient-statistics statement is:
- exact sufficient state exists for **per-candidate policy-path generation**,
- no request-word-only or desired-path-only statistic is sufficient for the **full repo validation/sweep result**.

## 5. Exactness conditions and equivalence classes

For fixed \(\theta\), define an equivalence relation on admissible score caches:
\[
K \sim_\theta K'
\]
iff they have the same field/finiteness pattern \(F_{1:T}\) and induce the same side of every active threshold/delta wall at every bar after the trade-timing shift. Equivalently, they induce the same request word \(r_{1:T}^\theta\). On each \(\sim_\theta\)-class:
- the desired-position path is identical,
- the metrics are identical,
- the robust score is identical.

For the full validation candidate record, a stricter equivalence relation is needed:
\[
K \approx_\theta K'
\]
iff \(K \sim_\theta K'\) and they also have the same `actual_score_obs`. On each \(\approx_\theta\)-class, the candidate’s metrics, robust score, PSR/DSR inputs, track-record bars, and hard-gate outcome are all identical.

For the full sweep result, an even stricter equivalence class is needed:
\[
(K,\Theta,\text{top}_k) \equiv (K',\Theta',\text{top}'_k)
\]
iff:
1. \(\Theta=\Theta'\) as expanded candidate multisets,
2. `top_k` matches,
3. every candidate’s robust score matches,
4. the resulting sort order matches,
5. every candidate’s gate inputs match.

Only on \(\equiv\)-classes is the stored sweep output exactly invariant.

## 6. Error, regret, or bias bounds for every non-exact claim

### 6.1 Calibration perturbation contract

For any fixed candidate \(\theta\), if
\[
\|\delta p\|_\infty < \gamma_\theta
\]
and the field/finiteness pattern is unchanged, then the request word does not change and the exact regret is
\[
J'(\theta)-J(\theta)=0.
\]
This is an exact zero-regret contract, not an asymptotic statement.

### 6.2 Observation-count bias contract

Suppose a surrogate layer replaces the repo observation count
\[
N_{\mathrm{obs}}^{\mathrm{repo}}=\#\{t:\text{point}_t\text{ is present}\}
\]
with a usable-row count \(N_{\mathrm{obs}}^{\mathrm{usable}}\). Then the exact bias satisfies
\[
\big|N_{\mathrm{obs}}^{\mathrm{repo}}-N_{\mathrm{obs}}^{\mathrm{usable}}\big|
\le B,
\]
where
\[
B=\#\{t:\text{point}_t \text{ is present but does not yield a usable finite request input}\}.
\]
This is the strongest safe explicit contract available from the repo. A tighter PSR/DSR error bound is **not** currently derivable from the owned surface alone because `compute_probability_metrics` is treated as an interface here. What survives exactly is the observation-count bias bound, not a numeric PSR/DSR perturbation bound. fileciteturn25file8

### 6.3 Future-pruning rank-preservation contract

This does **not** describe the current implementation, but it is the only admissible non-exact pruning contract for future consolidation. If a future surrogate ranking score \(\widehat J\) is introduced and satisfies
\[
\sup_{\theta\in\Theta} |\widehat J(\theta)-J(\theta)| \le \eta,
\]
then top-k preservation requires the true frontier gap
\[
J_{(k)}-J_{(k+1)} > 2\eta.
\]
Without that gap, no exact top-k preservation claim is valid. Under the current code, \(\eta=0\) because the sweep is exhaustive. fileciteturn27file0

### 6.4 Full-sweep approval contract

No nontrivial approval-preservation contract exists for the current stored sweep output unless it is stated on the **truncated shortlist**, not the full evaluated set. The exact shortlist-local statement survives; the full-sweep approved-policy statement does not.

## 7. Time, memory, and retained-state complexity

Let \(N=|\Theta|\) be the expanded candidate count and \(T\) the number of bars. The current implementation does:
- request-word generation in \(O(T)\) per candidate,
- desired-position generation in \(O(T)\) per candidate,
- iterative simulation in \(O(T)\) per candidate,
- final sort in \(O(N\log N)\).

So the exact runtime is
\[
O(NT)+O(N\log N),
\]
with \(N \le 20{,}000\) by explicit budget cap. Working memory is
\[
O(T)+O(N),
\]
because the request/desire buffers are reused across candidates while all candidate results are retained for ranking. The retained policy state during path generation is constant-size:
\[
(q_t,h_t,c_t,u_t,n_t).
\]
`top_k` does not reduce evaluation cost; it only truncates stored ranked results after exhaustive evaluation. fileciteturn26file17 fileciteturn25file8

## 8. File and symbol impact map

### Owned redesign surface: `crates/app_core/src/ml_signal.rs`

These are the symbols whose mathematics is genuinely owned by `PR-10`:
- `score_target`
- `fill_requested_targets`
- `fill_desired_positions`
- `neutralize_ml_signal_execution_overlays`
- `validate_score_cache_contract`
- `load_score_cache`
- `run_ml_signal_validation_blocking_with_candles`
- `run_ml_signal_policy_sweep_blocking_with_candles`
- `build_ml_signal_split_windows`
- dataset-export label/deadband/split assignment logic
- score-cache manifest/contract metadata classification surfaces

The label/export side currently writes `r_long`, `r_short`, `y_long`, `y_short`, `keep_long`, and `keep_short`, with split assignment taken from the **first** window whose interval contains the row. That means split provenance is not a unique partition when rolling windows overlap. fileciteturn28file10 fileciteturn26file13 fileciteturn22file16

### Interface-only constraints

`crates/app_core/src/iterative_sim.rs` is an interface constraint. It defines deterministic simulation from desired positions, plus execution-side persistence/cooldown logic that must remain neutralized by `ml_signal.rs` to avoid double application. fileciteturn25file12 fileciteturn27file10

`crates/app_core/src/validation_pipeline.rs` is an interface constraint. It owns robust-score folding/gating/verdict logic, including the fact that minimum-track-record approval is evaluated using `track_record_bars().max(2)` and the shortlist order passed into `build_run_verdict`. fileciteturn27file13

`src-tauri/src/commands_ml_signal.rs` is an interface constraint on domain validity. It enforces strategy kind and loaded dataset/timeframe compatibility whenever validation context is present. fileciteturn22file0 fileciteturn24file16

`ta_ml_signal_native_smoke.rs` and `ta_ml_signal_export_dataset.rs` are verification context, not redesign targets. The native smoke binary is a headless wrapper around score-cache generation from a bundle manifest with backend selection (`auto`, `native_cuda`, `native_cpu`) and explicitly rejects `score_cache_only`. fileciteturn28file14

## 9. Verification obligations and falsifiers

The result above is implementation-ready only if the following obligations are checked.

### Obligation A. No-request versus flat separation

Construct two caches that are identical on all usable finite rows, but on one bar one cache yields `Some(0)` and the other yields `None`. With `exit_on_flat = true` and an open position, the desired-position paths must differ. If they do not, the theorem’s four-symbol alphabet has been broken. fileciteturn25file6

### Obligation B. Boundary-margin invariance

For a candidate \(\theta\), perturb all active finite scores by less than the measured margin to every threshold/delta wall. The request word, desired-position path, metrics, and robust score must remain unchanged.

### Obligation C. Observation-count bias falsifier

Inject a present score point whose long/short fields are missing or nonfinite in a way that leaves the request word unchanged. `track_record_bars` should still change because the repo counts `point.is_some()`. If not, the gate-layer non-pathwise theorem needs revision. fileciteturn25file8

### Obligation D. Full-dataset coverage falsifier

A score cache marked `FullDataset` but missing either the first/last candle timestamp or sufficient coverage must fail before policy evaluation. There is already a repo test for this shape. fileciteturn26file5 fileciteturn25file3

### Obligation E. Scope-warning behavior

`CalibrationSlice` must add a score-scope warning; `WalkforwardTest` must not add one. The repo already contains tests for both behaviors. fileciteturn26file9

### Obligation F. One-sided versus two-sided admissibility

Long-only validation must accept one-sided `p_long` caches, while two-sided validation must reject missing short-side probabilities. The repo already tests both cases. fileciteturn26file11 fileciteturn27file7

### Obligation G. Shortlist-local verdict falsifier

Create a sweep in which the first approved candidate lies below `top_k`. The stored sweep result should still report no approved candidate, because verdicts are rebuilt only on the retained shortlist. If not, the exact sweep target has changed. fileciteturn27file0

### Obligation H. Export/native-smoke parity

The headless export wrapper and native-smoke wrapper should remain wrappers around the same owned contracts, not alternate math surfaces. Export should preserve the deadband/keep-rule semantics, and native-smoke should preserve bundle-backed score-cache metadata and backend reporting. fileciteturn28file14 fileciteturn28file10

## 10. Dependencies that must be reconciled during consolidation

1. **Overlay neutralization contract.** `ml_signal.rs` must continue to neutralize execution persistence/cooldown before calling `simulate_iterative_from_signals`; otherwise the exact owned transducer changes. fileciteturn25file6 fileciteturn25file12

2. **Gate-vs-policy distinction.** Consolidation must keep the separation between decision-path exactness and gate-layer dependence on `actual_score_obs`. Otherwise the theorem statements become silently false. fileciteturn25file8

3. **Shortlist-local verdict semantics.** Any cross-prompt consolidation must state clearly whether it is targeting full-set approval or stored shortlist verdicts. Current `PR-10` owns only the latter. fileciteturn27file0

4. **Recommended-versus-required metadata.** Manifest fields such as score contracts, training-window summaries, calibration summaries, and explainability artifacts are currently classified as recommended in bundle health, not enforced as hard validation prerequisites. Consolidation must not silently upgrade them into theorem assumptions without a code change. fileciteturn26file14

5. **Validation-context domain restriction.** Any consolidation with adjacent prompts must preserve the Tauri-side requirement that ML-signal validation/sweep only run against compatible loaded data when validation context is present. fileciteturn22file0 fileciteturn24file16

## 11. Kill criteria

Reject the implementation-ready result if any of the following happen:

1. `ML_SIGNAL_NO_REQUEST` is collapsed with `0` or otherwise loses its semantic distinction.
2. `fill_desired_positions` stops being the sole owner of signal persistence/cooldown/max-hold policy before simulation.
3. `neutralize_ml_signal_execution_overlays` stops neutralizing persistence/cooldown/flip before iterative simulation.
4. Sweep starts claiming “best approved candidate in the full evaluated set” without changing the current sort-then-truncate-then-verdict control flow.
5. Any future pruning/truncation is introduced without an explicit score-error and frontier-gap contract.
6. Full-dataset claims are made without the existing full-span and 0.95 coverage checks.
7. Split ids are treated as unique split provenance even though export currently assigns the first matching overlapping window.
8. Approval theorems are stated as pure functions of the desired-position path without accounting for `actual_score_obs`.

## Strongest surviving implementation-ready result

The strongest surviving result is this:

**On the current repo surface, an ML-signal candidate is exactly a four-symbol threshold/delta request-word generator composed with a five-register finite-state overlay transducer, followed by deterministic iterative simulation under a neutralized execution-overlay config, followed by robust-score ranking.** The exact candidate-level geometry is a piecewise-constant cell complex in calibrated-score space. Exact stability is available inside cells under a boundary-margin condition. The exact sweep output is the exhaustive top-k-by-robust-score shortlist together with a verdict rebuilt on that shortlist. Everything stronger—especially full-sweep approved-policy recovery, pruning guarantees, or approval as a pure function of the decision path—fails unless additional explicit contracts are added. fileciteturn25file2 fileciteturn25file6 fileciteturn25file8 fileciteturn27file0

