# PR-01 Metric Semantics and Published Surface Contract

Source prompt: `PROMPT.md` (uploaded prompt file).

This document consolidates the first-pass, second-pass, and third-pass answers
for PR-01.

---

## `PR-01` Metric semantics and published surface contract

Owned problem:
the meaning of published metric fields across GPU contracts, CPU finalizers, and
runtime metric-surface publication

Source packets:
`MR-01`, `MR-07`, `MR-14`

Primary ownership files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/composite_gpu_metric_decode.rs`
- `crates/app_core/src/double_ma.rs`
- `crates/app_core/src/composite_cpu.rs`
- `crates/strategies/src/double_ma/mod.rs`
- `src-tauri/src/metric_surface.rs`
- `src-tauri/src/run_artifacts.rs`

Context files:

- `crates/app_core/src/iterative_sim.rs`
- `crates/app_core/src/asr.rs`

Own these symbols or decisions:

- `legacy`, `core17`, and `trade22` meaning
- exact-tail versus placeholder field semantics
- GPU row-width or bundle meaning as a published contract
- result-surface resolution and projection semantics
- trade/risk field meaning that is visible to downstream consumers

Open problem:

- characterize the minimal sufficient state needed to publish every current
  metric surface exactly, and prove which published fields are mathematically
  derivable versus irreducibly primitive under the repo's path-dependent
  semantics

Why this is hard here:

- GPU row contracts, CPU finalizers, and runtime publication all expose the
  same public semantics from different execution paths
- exact trade and path-dependent tail meaning competes with tight retained-state
  budgets and artifact compatibility

Mathematical object:

- a discrete path-to-surface emission problem with exact publication
  requirements across multiple metric surfaces

Notation seed:

- let `\mathcal{P}_T` denote the set of admissible discrete backtest paths
  `p = ((z_t, a_t, \delta_t))_{t=1}^T`, where `z_t` is the market/bar state,
  `a_t` is the strategy action/position state, and `\delta_t` is any
  event-level trade or fill update needed by the current product semantics
- for each published surface `s \in \{\mathrm{legacy}, \mathrm{core17},
  \mathrm{trade22}\}`, let
  `F_s : \mathcal{P}_T \to \mathbb{R}^{d_s}` be the exact emitted metric row
- seek a retained-state map
  `S : \mathcal{P}_T \to \mathbb{R}^k \times \mathbb{Z}^m`
  and decoders `G_s` such that
  `F_s(p) = G_s(S(p))` for all `p \in \mathcal{P}_T`
- define semantic equivalence on paths by
  `p \sim q \iff \forall s,\; F_s(p) = F_s(q)`;
  an exact retained state must separate the quotient classes induced by `\sim`
- a published field `F_{s,j}` is derivable from the retained state iff there
  exists `g_{s,j}` such that
  `F_{s,j}(p) = g_{s,j}(S(p))` for all `p`; otherwise it is primitive under the
  admitted state class

Primary questions:

- does a finite sufficient-statistic basis exist for exact `legacy`, `core17`,
  and `trade22` publication?
- which currently published fields are exact derivations and which force
  path-dependent retained state?
- when do two different retained-state bundles induce identical published
  surfaces?

Required deliverables:

- formal state/output model
- candidate minimal-sufficiency theorem
- one counterexample family showing where compression fails
- explicit lower bound or identifiability argument for the minimal retained
  state width `(k, m)`
- exactness criterion of the form `F_s(p) = \hat F_s(p)` for all `s, p`, or an
  explicit approximation contract such as
  `\lVert F_s(p) - \hat F_s(p) \rVert_\infty \le \varepsilon_s`
  together with any rank-preservation requirement

Acceptance bar:

- separate exact equivalence results from heuristic compression ideas
- any proposed compression must either preserve public semantics exactly or
  attach an explicit approximation contract with a stated norm, bias, or
  rank-instability bound

Verification hooks:

- artifact parity on `legacy`, `core17`, and `trade22`, meaning either
  `F_s(p) = \hat F_s(p)` exactly for every tested path or satisfaction of the
  stated approximation contract
- proof or falsifier for every claimed derivation

Non-goals:

- do not redesign ranking or utility formulas that consume these metrics; those
  belong to `PR-02`, `PR-05`, `PR-06`, or `PR-07`
- do not redesign cache or execution economics; that belongs to `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- preserve or explicitly redefine public metric semantics instead of silently
  drifting them
- identify primitive versus derivable published fields
- keep artifact-surface compatibility visible as a first-class constraint
- use the symbols `\mathcal{P}_T`, `F_s`, `S`, `G_s`, and `\sim` explicitly in
  the answer rather than replacing them with prose only

---

## First-pass answer

### Formal model

Treat the repo as a path-to-surface emission problem. For each admissible path
\(p \in \mathcal{P}_T\), let \(M(p)\) be the internal semantic `Metrics`
object and let the public row projection be determined by
\(s \in \{\mathrm{legacy}, \mathrm{core17}, \mathrm{trade22}\}\).

The initial row model was:

- \(F_{\mathrm{legacy}}(p) = ()\)
- \(F_{\mathrm{core17}}(p) = \Pi_{17}(M(p))\)
- \(F_{\mathrm{trade22}}(p) = \Pi_{22}(M(p))\)

with `core17` field order fixed by `CORE17_FIELDS`, `trade22` extending that
order by
`closed_trades`, `winning_trades`, `losing_trades`, `gross_profit`,
`gross_loss`, and public `mean` defined by the historical sidecar contract
`mean = sharpe * volatility`, `std = volatility`.

### Candidate sufficient statistics

The first-pass retained state split was:

- \(S_{\mathrm{pub}}\): publication evidence needed to determine the public
  surface kind.
- \(S_{\mathrm{core}}\): bar-aggregate risk state sufficient for the 17
  published core coordinates.
- \(S_{\mathrm{trade}}\): exact trade-tail summary
  \((c,w,\ell,g^+,g^-)\).
- \(S_{\mathrm{open}}\): transient open-trade state needed only to update the
  trade tail online.

A concrete \(S_{\mathrm{core}}\) was modeled on the existing running
aggregates:
\[
S_{\mathrm{core}} =
(n, o, \mu, M_2, \Sigma^-_2, \Sigma^-_3, n_-, \Sigma^+, \Sigma^-_{\mathrm{abs}},
 n_L, n_S, \Sigma_L, \Sigma_S, \log E, \mathrm{DD}^{\log}_{\max},
 \Sigma \mathrm{DD}^2, U, U_{\max}, \Sigma |a|, \Sigma a, N_{\mathrm{trades}})
\]
where the symbols denote counts, return moments, downside partial moments,
drawdown accumulators, underwater counters, exposure sums, and trade count.

### Strongest positive result claimed in the first pass

The first-pass positive theorem was:

- exact `core17` publication should be derivable from \(S_{\mathrm{core}}\);
- exact `trade22` publication should be derivable from
  \((S_{\mathrm{core}}, S_{\mathrm{trade}})\);
- no full trade ledger should be required because the published trade tail uses
  only five closed-trade summaries.

### Strongest impossibility result claimed in the first pass

The initial negative result was:

- `Metrics` alone cannot determine the public surface kind because runtime
  hints, current metadata, route class, and kernel evidence also matter;
- \(S_{\mathrm{core}}\) alone cannot determine the five trade-tail fields.

### Counterexample family used in the first pass

The main separator was the surface-evidence family:

- same market path,
- same strategy path,
- same `best_metrics`,
- same exact-GPU-looking route,
- but different route diagnostics or runtime hints.

That family separated `legacy` from `core17` while keeping the numerical
metrics payload fixed.

### Initial lower-bound angle

Relative to a `core17`-exact state, the first pass argued that exact `trade22`
publication needs at least the extra summary coordinates for

- `closed_trades`
- `winning_trades`
- `losing_trades`
- `gross_profit`
- `gross_loss`

plus a discrete publication-state coordinate if the public surface kind must
also be recovered.

### What the first pass left unresolved

The first pass did not yet settle:

- whether the full sidecar artifact, not just one row, was the true public
  object;
- whether `legacy` absence versus explicit `legacy` sidecar mattered;
- whether placeholder trade-tail zeros could be distinguished from exact zeros;
- whether the fixed-dimensional \(S : \mathcal{P}_T \to \mathbb{R}^k \times \mathbb{Z}^m\)
  target was valid for the whole artifact.

---

## Second-pass answer

### Main attacks on the first pass

The second pass broke the original formalization in seven main ways.

First, the public object is not just a row. The sidecar carries `surface`,
`fields`, an optional `best_metrics_row`, and an ordered variable-length
`top_metric_rows` list.

Second, the full artifact therefore does not fit the codomain
\(\mathbb{R}^{d_s}\). It is a partial JSON-like object with optional rows and
skip-serialization behavior.

Third, public surface resolution is not determined by metrics alone. Runtime
hint wins, then current metadata, then heuristics driven by route, strategy
support, requested contract, and `gpu_kernel.metrics_count`.

Fourth, the non-double-MA GPU decoder emits placeholder zeros for the five
trade-tail fields because that raw row contract does not export them. So a
numeric zero trade tail can mean either “exact zero” or “not exported”.

Fifth, there is a precedence mismatch risk between kernel-side contract
selection and publication-side heuristic reconstruction when explicit
`Full17` and `include_trade_metrics` interact without exact `metrics_count`
evidence.

Sixth, non-finite values break a pure row model in two distinct ways:
row-cell `null` on direct serialization versus row omission when the
input `best_metrics` payload no longer parses into `Metrics`.

Seventh, the first-pass “no fixed-dimensional state for full artifact parity”
was only implementation-plausible, not mathematically sharp, unless the
admitted state class excludes opaque set-theoretic encodings of arbitrary
finite row sequences into a fixed number of scalars.

### Corrected surviving result after the second pass

The strongest surviving result was narrower:

- there is no nontrivial fixed-dimensional sufficient statistic for exact full
  artifact parity unless one narrows the admissible state class to typed,
  implementation-fit retained states;
- on a restricted domain where the surface kind is fixed, the row exists, the
  row is finite, and the trade tail is exact rather than placeholder, a finite
  sufficient statistic survives for best-row semantics:
  - `core17` from bar-aggregate risk state,
  - `trade22` from that plus exact closed-trade summaries.

### Weakest assumptions identified in the second pass

The surviving row theorem required all of the following:

- compare canonical row semantics, not raw artifact JSON;
- treat publication evidence as primitive interface state;
- restrict `trade22` to exact-tail-supporting routes;
- separate placeholder-zero from exact-zero semantics;
- avoid claiming ranking preservation from public-row equality.

### Final narrowed second-pass statement

The second pass therefore ended with:

- no fixed-dimensional theorem for the full current sidecar as first written;
- a surviving exact theorem only for the restricted semantic best-row target.

---

## Third-pass answer

### 1. Final formalization

I keep the prompt notation \(\mathcal{P}_T\), \(F_s\), \(S\), \(G_s\), and
\(\sim\), but I explicitly refine the public object.

Let \(p \in \mathcal{P}_T\) denote an admissible backtest path together with the
publication evidence actually used by the repo: request flags, resolved route,
runtime hint, current sidecar metadata, and any exact-GPU row-width evidence.

The implementation-ready public target is not just a row. It is the **canonical
normalized sidecar artifact**
\[
P^\#(p)=\big(\sigma(p),\Phi_{\sigma(p)},B(p),T(p)\big),
\]
where

- \(\sigma(p)\in\{\mathrm{legacy},\mathrm{core17},\mathrm{trade22}\}\),
- \(\Phi_{\mathrm{legacy}}=[]\),
  \(\Phi_{\mathrm{core17}}=\texttt{CORE17\_FIELDS}\),
  \(\Phi_{\mathrm{trade22}}=\texttt{TRADE22\_FIELDS}\),
- \(B(p)\in\{\varnothing\}\cup \mathbb J^{d_{\sigma(p)}}\) is the optional
  `best_metrics_row`,
- \(T(p)\in (\mathbb J^{d_{\sigma(p)}})^*\) is the ordered `top_metric_rows`
  sequence,
- \(\mathbb J := \mathbb R_{\mathrm{fin}} \cup \{\texttt{null}\}\).

This matches the current sidecar shape in
`src-tauri/src/metric_surface.rs:98-107, 439-487` and the normalization/backfill
boundary in `src-tauri/src/run_artifacts.rs:1699-1707`.

The canonical surface meanings are:

\[
\mathrm{legacy}:
\quad
\Phi_{\mathrm{legacy}}=[],
\quad
B=\varnothing,
\quad
T=()
\]
after canonical normalization.

\[
\mathrm{core17}:
\quad
\Phi_{\mathrm{core17}}=
(\text{pnl},\text{trades},\text{max\_dd},\text{mean},\text{std},\text{exposure},
\text{net\_exposure},\text{downside\_deviation},\text{ulcer\_index},\text{omega},
\text{downside\_frequency},\text{downside\_severity},\text{long\_mean\_return},
\text{short\_mean\_return},\text{kappa\_tau},\text{underwater\_fraction},
\text{max\_underwater\_duration\_bars})
\]

\[
\mathrm{trade22}:
\quad
\Phi_{\mathrm{trade22}}=
(\Phi_{\mathrm{core17}},
\text{closed\_trades},\text{winning\_trades},\text{losing\_trades},
\text{gross\_profit},\text{gross\_loss})
\]

and public `mean` is **not** the hidden running mean; it is the sidecar contract
value
\[
\text{mean} = \text{sharpe}\cdot \text{volatility},
\qquad
\text{std} = \text{volatility},
\]
exactly as projected in `metric_surface.rs:312-369`.

To model exact-tail versus placeholder semantics, define a provenance label
\[
\tau(p)\in\{\mathrm{none},\mathrm{placeholder},\mathrm{exact}\}.
\]
The public rule is:

- `core17` makes no claim about the five trade-tail coordinates.
- `trade22` is semantically valid only when \(\tau(p)=\mathrm{exact}\).
- Any row whose tail is placeholder-zero is **not** a valid witness for
  `trade22`.

This distinction is forced by
`crates/app_core/src/composite_gpu_metric_decode.rs:49-62, 127-131`, where the
non-double-MA GPU decode explicitly fills those five fields with placeholder
zeros.

For the prompt’s row maps \(F_s\), the correct exact domain is the finite,
row-present, provenance-valid domain
\[
D_s^{\mathrm{pub}} \subseteq \mathcal{P}_T.
\]
Then
\[
F_s : D_s^{\mathrm{pub}} \to \mathbb J^{d_s}
\]
is the canonical row projection for surface \(s\). For `trade22`,
\(D_{\mathrm{trade22}}^{\mathrm{pub}}\) includes only exact-tail paths.

The exact public equivalence relation is no longer the seed
\(\forall s,\;F_s(p)=F_s(q)\). The correct canonical-public equivalence is
\[
p \sim q \iff P^\#(p)=P^\#(q).
\]
For row-only reasoning on a fixed surface, define
\[
p \sim_s q \iff F_s(p)=F_s(q).
\]

To keep the retained-state problem mathematically meaningful and
implementation-fit, I also make the admissible-state class explicit.

A retained state is **repo-admissible** if it is built from:

- a fixed tuple of scalar counters, accumulators, booleans, and enums in
  \(\mathbb R^k \times \mathbb Z^m\),
- optional explicit row buffers,
- constant-time online updates for the scalar part,
- coordinate-wise decoders and explicit serialization of any stored row buffers.

Opaque set-theoretic encodings of arbitrary finite row sequences into one real
number are excluded. Without that admissibility condition, the fixed-width
problem becomes mathematically vacuous and implementation-irrelevant.

### 2. Final theorem and lemma list

#### Theorem 1: canonical surface theorem

On canonicalized artifacts, the public contract is exactly \(P^\#\), not merely
a numeric row.

- `legacy` means empty field list and no rows.
- `core17` means the fixed 17-field order from `CORE17_FIELDS`.
- `trade22` means the fixed 22-field order from `TRADE22_FIELDS`.
- `fields` is derivable from \(\sigma\).
- `best_metrics_row` and `top_metric_rows` are optional and ordered.

Repo references:
`src-tauri/src/metric_surface.rs:7-49, 98-107, 317-487`;
`src-tauri/src/run_artifacts.rs:1699-1707, 2855, 2883, 2924`.

#### Theorem 2: core17 finite-sufficiency theorem

On \(D_{\mathrm{core17}}^{\mathrm{pub}}\), there exists a finite
repo-admissible sufficient statistic \(S_{\mathrm{core}}\) and a decoder
\(G_{\mathrm{core17}}\) such that
\[
F_{\mathrm{core17}}(p)=G_{\mathrm{core17}}(S_{\mathrm{core}}(p))
\qquad
\forall p\in D_{\mathrm{core17}}^{\mathrm{pub}}.
\]

One exact choice is
\[
S_{\mathrm{core}}=
(\mathrm{obs},\mathrm{count},\mu,M_2,\Sigma^-_2,\Sigma^-_3,n_-,\Sigma^+,
\Sigma^-_{\mathrm{abs}},n_L,n_S,\Sigma_L,\Sigma_S,\log E,
\mathrm{DD}^{\log}_{\max},\Sigma \mathrm{DD}^2,U,U_{\max},
N_{\mathrm{trades}},\Sigma |pos|,\Sigma pos),
\]
where these are exactly the running quantities maintained by the double-MA
evaluation loops and consumed by `finalize_risk_metrics`.

Every core17 field is derivable from this state. In particular:

- `pnl` from \(\log E\),
- `max_dd` from \(\mathrm{DD}^{\log}_{\max}\),
- `ulcer_index` from \(\Sigma \mathrm{DD}^2\) and `obs`,
- `underwater_fraction` from \(U/\mathrm{obs}\),
- `max_underwater_duration_bars` from \(U_{\max}\),
- `exposure` from \(\Sigma |pos|/\mathrm{denom}\),
- `net_exposure` from \(\Sigma pos/\mathrm{denom}\),
- downside and directional fields from the running return aggregates,
- public `mean` from the sidecar rule
  \(G_{\mathrm{mean}}=G_{\mathrm{sharpe}}\cdot G_{\mathrm{volatility}}\).

Repo references:
`crates/strategies/src/double_ma/mod.rs:142-306, 2718-2954`;
`src-tauri/src/metric_surface.rs:312-369`.

#### Theorem 3: trade22 finite-sufficiency theorem

On
\[
D_{\mathrm{trade22}}^{\mathrm{pub,exact}}
:=
\{p \in D_{\mathrm{trade22}}^{\mathrm{pub}} : \tau(p)=\mathrm{exact}\},
\]
there exists a finite repo-admissible sufficient statistic
\[
S_{\mathrm{trade22}} = (S_{\mathrm{core}}, S_{\mathrm{trade}})
\]
with
\[
S_{\mathrm{trade}}=(c,w,\ell,g^+,g^-)
\]
such that
\[
F_{\mathrm{trade22}}(p)=G_{\mathrm{trade22}}(S_{\mathrm{trade22}}(p))
\qquad
\forall p\in D_{\mathrm{trade22}}^{\mathrm{pub,exact}}.
\]

The exact trade-tail update rule is:

- on every closed trade with realized return \(r\):
  \[
  c \leftarrow c+1,
  \]
  \[
  w \leftarrow w+\mathbf 1_{\{r>0\}},
  \qquad
  \ell \leftarrow \ell+\mathbf 1_{\{r<0\}},
  \]
  \[
  g^+ \leftarrow g^+ + \max(r,0),
  \qquad
  g^- \leftarrow g^- + \max(-r,0).
  \]

Break-even closed trades are allowed, so \(c\) need not equal \(w+\ell\).

The first 17 coordinates are \(G_{\mathrm{core17}}(S_{\mathrm{core}})\), and the
last five are exactly \((c,w,\ell,g^+,g^-)\).

Repo references:
`crates/strategies/src/double_ma/mod.rs:1632-1698`;
`src-tauri/src/metric_surface.rs:347-369`.

#### Theorem 4: full-artifact nonexistence theorem under repo-admissible states

Under the repo-admissible state class, there is **no fixed-size scalar-only**
retained state \(S : \mathcal P_T \to \mathbb R^k \times \mathbb Z^m\) whose
decoder recovers the full canonical artifact \(P^\#\) for all \(p\).

Exact full artifact publication requires a variable-size explicit row buffer:
\[
S_{\mathrm{full}}(p)=\big(S_{\mathrm{pub}}(p),B(p),T(p)\big),
\]
where \(B(p)\) is the optional best row and \(T(p)\) is the ordered list of top
rows.

So the right statement is:

- fixed-size sufficient statistics exist for row semantics,
- fixed-size sufficient statistics do not exist for the whole artifact,
- exact whole-artifact publication requires explicit variable-size row storage.

The obstruction is not the core metrics. It is the ordered sequence
`top_metric_rows`, plus row-presence and null/omission behavior.

Repo references:
`src-tauri/src/metric_surface.rs:98-107, 439-487`.

#### Theorem 5: primitive publication-state theorem

The following are primitive interface state, not derivable from numeric metrics
alone:

- the resolved public surface kind \(\sigma\),
- exact-tail provenance \(\tau\),
- best-row presence versus omission,
- `null`-cell versus row-omission behavior,
- the ordered top-row payload.

This is forced by the resolver precedence in
`metric_surface.rs:142-309`, by placeholder zeros in
`composite_gpu_metric_decode.rs:49-62, 127-131`, and by the canonicalization
boundary in `run_artifacts.rs:1699-1707`.

#### Theorem 6: semantic-width theorem

`metrics_count` is the semantic GPU row-width witness for public metric surface
resolution. `tile_metrics_stride` is transport-only and must be excluded from
the public equivalence relation \(\sim\).

Repo references:
`crates/app_core/src/gpu_backtest_kernel.rs:150-158, 977-1000, 4662-4707`;
`src-tauri/src/metric_surface.rs:168-179, 250-275`.

#### Theorem 7: optional approximation theorem for gross-sum reconstruction

If an implementation chooses not to retain direct gross sums and instead
reconstructs them from
\[
\hat g^+ = \mathrm{fl}(\mathrm{fl}(\mathrm{avg\_win}) \cdot w),
\qquad
\hat g^- = \mathrm{fl}(\mathrm{fl}(-\mathrm{avg\_loss}) \cdot \ell),
\]
then exactness is lost. Under IEEE-754 binary64, round-to-nearest, and no
overflow/underflow, the relative error obeys
\[
|\hat g^\pm - g^\pm| \le \gamma_2 |g^\pm|,
\qquad
\gamma_2 = \frac{2u}{1-2u},
\qquad
u = 2^{-53}.
\]

This is the only non-exact claim I allow, and it has no rank-stability promise.
If PR-01 requires exact publication, this approximation is **not acceptable**;
retain \(g^+\) and \(g^-\) directly instead.

Repo references:
`crates/strategies/src/double_ma/mod.rs:1637-1679`;
`crates/app_core/src/composite_cpu.rs:647-655`.

### 3. Proof sketch / proof plan

#### Proof sketch for Theorem 1

Read the public field orders and surface labels directly from
`metric_surface.rs:7-64`.
Read the sidecar object shape from `metric_surface.rs:98-107`.
Read row projection and the `mean = sharpe * volatility` rule from
`metric_surface.rs:312-369`.
Read builder semantics, including ordered `top_metric_rows` and `pareto`
fallback, from `metric_surface.rs:439-487`.
Read canonical backfill into artifacts from
`run_artifacts.rs:1699-1707`.

That establishes the codomain and shows that the true public object is \(P^\#\).

#### Proof sketch for Theorem 2

The double-MA evaluation loop maintains all of the scalar accumulators listed in
\(S_{\mathrm{core}}\) explicitly:
`count`, `mean`, `m2`, downside sums, directional sums, exposure sums,
drawdown accumulators, underwater counters, trade count, and `log_eq`
in `mod.rs:2718-2954`.

`finalize_risk_metrics` is then an explicit deterministic decoder from those
aggregates to the risk metrics in `mod.rs:142-306`.

Finally, `project_metrics_row` in `metric_surface.rs:325-346` is an explicit
coordinate map from `Metrics` to the published core17 row. So composing those
two decoders yields \(G_{\mathrm{core17}}\).

#### Proof sketch for Theorem 3

The strategy module already computes the exact trade summary ingredients from
closed trades in `mod.rs:1632-1698`.
The public trade22 tail needs only
\((c,w,\ell,g^+,g^-)\), not the full ledger.

So if those five summaries are retained directly, the last five published
coordinates are exact. Appending them to the first 17 coordinates from
\(G_{\mathrm{core17}}\) yields \(G_{\mathrm{trade22}}\).

`composite_cpu.rs:1134-1187` confirms that the backend already treats exact
trade22 enrichment as a publication-level concern, and
`composite_cpu.rs:15830-15952` shows a route where GPU-enriched trade22 is
checked against CPU trade stats.

#### Proof sketch for Theorem 4

The artifact builder preserves an ordered, variable-length sequence
`top_metric_rows` in `metric_surface.rs:449-472`.
Under repo-admissible states, a fixed tuple of scalar accumulators is not
allowed to secretly encode that whole sequence.
Therefore exact full artifact parity requires explicitly storing the rows.

This is an implementation-fit impossibility, not a set-theoretic one.

#### Proof sketch for Theorem 5

Surface resolution uses runtime hint, then current sidecar metadata, then route-
and strategy-sensitive heuristics in `metric_surface.rs:142-309`.
Placeholder trade tails are produced on a raw GPU path in
`composite_gpu_metric_decode.rs:49-62, 127-131`.
Therefore neither \(\sigma\) nor \(\tau\) can be derived from numeric metrics
alone.

#### Proof sketch for Theorem 6

The GPU kernel code distinguishes `metrics_count` from `tile_metrics_stride`:

- semantic contracts are defined by the 10/17/22 widths and by
  `exact_eval_metrics_count_with_contract` in
  `gpu_backtest_kernel.rs:150-158, 990-1000`,
- route diagnostics expose both `metrics_count` and `tile_metrics_stride` in
  `gpu_backtest_kernel.rs:4662-4707`,
- the publication resolver reads only `gpu_kernel.metrics_count` in
  `metric_surface.rs:168-179, 250-275`.

So `tile_metrics_stride` is outside the published semantic contract.

### 4. Sufficient statistics, or proof they do not exist

#### Exact sufficient statistics that do exist

For exact core17 row semantics:
\[
S_{\mathrm{core}}=
(\mathrm{obs},\mathrm{count},\mu,M_2,\Sigma^-_2,\Sigma^-_3,n_-,\Sigma^+,
\Sigma^-_{\mathrm{abs}},n_L,n_S,\Sigma_L,\Sigma_S,\log E,
\mathrm{DD}^{\log}_{\max},\Sigma \mathrm{DD}^2,U,U_{\max},
N_{\mathrm{trades}},\Sigma |pos|,\Sigma pos).
\]

For exact trade22 row semantics:
\[
S_{\mathrm{trade22}} = (S_{\mathrm{core}}, c,w,\ell,g^+,g^-).
\]

If online exact tail updates are required during strategy execution, then the
execution engine also needs a transient open-trade accumulator
\(S_{\mathrm{open}}\), but that is strategy-local and not part of the final
published sufficient statistic.

#### Exact sufficient statistics that do not exist

No fixed-size scalar-only sufficient statistic exists for the full canonical
artifact \(P^\#\) under the repo-admissible state class, because exact
publication preserves the full ordered `top_metric_rows` payload. The exact full
state is necessarily
\[
S_{\mathrm{full}} = (S_{\mathrm{pub}}, B, T),
\]
which is variable-size through \(T\).

### 5. Exactness conditions and equivalence classes

The exactness conditions are:

For a fixed surface row on its valid domain,
\[
\hat F_s(p)=F_s(p)
\qquad \forall p\in D_s^{\mathrm{pub}}.
\]

For full canonical artifact parity,
\[
\hat P^\#(p)=P^\#(p)
\qquad \forall p\in\mathcal P_T.
\]

For raw artifacts, exactness is checked only after canonical normalization:
\[
\mathcal N(\hat P_{\mathrm{raw}}(p))=\mathcal N(P_{\mathrm{raw}}(p)).
\]

The public equivalence classes are:

- canonical artifact equivalence:
  \[
  p \sim q \iff P^\#(p)=P^\#(q),
  \]
- fixed-surface row equivalence:
  \[
  p \sim_s q \iff F_s(p)=F_s(q).
  \]

A critical semantic rule is:
\[
\tau(p)=\mathrm{placeholder}
\implies
p \notin D_{\mathrm{trade22}}^{\mathrm{pub,exact}}.
\]
So numeric equality of a 22-vector is not enough to certify trade22 semantics.

### 6. Error, regret, or bias bounds for every non-exact claim

There is only one sanctioned non-exact claim:

If gross sums are reconstructed from average win/loss rather than retained
directly, then the implementation must carry the explicit bound
\[
|\hat g^\pm - g^\pm| \le \gamma_2 |g^\pm|,
\qquad
\gamma_2 = \frac{2u}{1-2u},
\qquad
u = 2^{-53}.
\]

No other approximation contract is accepted for PR-01.

In particular:

- there is **no** accepted \(\varepsilon\)-contract for public `core17`;
- there is **no** accepted \(\varepsilon\)-contract for public `trade22`;
- there is **no** accepted rank-stability guarantee derived from public-row
  equality alone.

### 7. Time, memory, and retained-state complexity

For exact core17 row semantics:

- update time: \(O(1)\) per bar,
- retained scalar memory: \(O(1)\),
- decode time: \(O(17)\).

For exact trade22 row semantics:

- update time: \(O(1)\) per bar plus \(O(1)\) per close event,
- retained scalar memory: \(O(1)\),
- decode time: \(O(22)\).

For exact full canonical artifact publication with \(K := |T(p)|\):

- retained memory:
  \[
  \Theta\!\big((1+\mathbf 1_{B\neq\varnothing}+K)\, d_{\sigma(p)}\big)
  \]
  numeric cells plus \(O(1)\) primitive publication state,
- serialization time:
  \[
  \Theta\!\big((1+\mathbf 1_{B\neq\varnothing}+K)\, d_{\sigma(p)}\big).
  \]

So PR-01 has a clean split:

- row semantics are fixed-width,
- artifact semantics are variable-width.

### 8. File and symbol impact map

#### `src-tauri/src/metric_surface.rs`

Owns the public contract directly.

Symbols and decisions:
- `CORE17_FIELDS`
- `TRADE22_FIELDS`
- `OptimizationMetricSurfaceKind`
- `artifact_requested_trade22`
- `artifact_requested_full17`
- `artifact_gpu_kernel_metrics_count`
- `detect_exact_gpu_kernel_surface_kind`
- `heuristic_surface_kind`
- `rebuild_surface_kind`
- `project_metrics_row`
- `build_optimization_metric_surface_artifact`
- `build_optimization_metric_surface_artifact_value`

Impact:
- this file must define the canonical surface semantics,
- the final `\sigma` resolver,
- the field order,
- the `mean = sharpe * volatility` rule,
- the top-row ordering contract.

#### `src-tauri/src/run_artifacts.rs`

Owns the canonicalization boundary.

Symbols and decisions:
- normalization/backfill of `optimization_metric_surface`
- detail response threading
- hydration payload threading

Impact:
- raw absence versus explicit `legacy` must be collapsed here if canonical
  artifact parity is the public target.

#### `crates/app_core/src/composite_gpu_metric_decode.rs`

Owns placeholder-tail semantics on non-double-MA GPU decode.

Symbols and decisions:
- `NON_DOUBLE_MA_GPU_CORE17_EXTENDED_METRICS_STRIDE`
- `decode_non_double_ma_gpu_metrics_row`

Impact:
- this file must never be treated as exact trade22 evidence unless the row
  contract itself changes.

#### `crates/app_core/src/composite_cpu.rs`

Owns exact trade22 enrichment for composite strategies.

Symbols and decisions:
- `composite_trade22_requested`
- `composite_metric_surface_kind`
- `apply_trade_stats_to_metrics`
- `maybe_enrich_trade22_backend_result`
- `evaluate_*_trade22_metrics`

Impact:
- this file determines whether exact trade22 is available after enrichment,
- it currently exposes the avg-times-count approximation risk for gross sums.

#### `crates/app_core/src/double_ma.rs`

Owns request-side contract intent and route-sensitive surface hints.

Symbols and decisions:
- `DoubleMaMetricContract`
- `resolved_metric_surface_kind_for_result`
- `maybe_enrich_nonexact_trade22_result`

Impact:
- this file must agree with `metric_surface.rs` on precedence and on when a
  route may legitimately emit `trade22`.

#### `crates/app_core/src/gpu_backtest_kernel.rs`

Owns semantic GPU row width versus transport stride.

Symbols and decisions:
- `PRODUCT_REDUCED_METRICS_COUNT`
- `METRICS_COUNT`
- `TRADE_METRICS_COUNT`
- `fixed_metrics_contract_rows`
- `exact_eval_metrics_count_with_contract`
- `search_metrics_count_for_optimization_with_contract`
- `decode_tm_flex_metrics_row`
- `build_gpu_kernel_route_diagnostics`

Impact:
- `metrics_count` is public evidence,
- `tile_metrics_stride` is not.

#### `crates/strategies/src/double_ma/mod.rs`

Owns the semantic meaning of the metric fields themselves.

Symbols and decisions:
- `Metrics`
- `finalize_risk_metrics`
- evaluation loops that accumulate the core risk state
- trade-stat construction from the closed-trade set

Impact:
- this file is the source of truth for which fields are derivable from which
  path aggregates.

### 9. Verification obligations and falsifiers

The consolidation must satisfy all of the following.

1. **Canonical surface correctness**
   - For every normalized artifact, emitted `surface` and `fields` must match
     the canonical definitions.
   - Falsifier: a normalized artifact says `core17` but carries a 22-field list.

2. **Exact trade22 gating**
   - Any published `trade22` row must have \(\tau=\mathrm{exact}\).
   - Falsifier: a non-double-MA GPU placeholder row publishes as `trade22`.

3. **Core17 sufficiency**
   - Recompute all 17 public coordinates from \(S_{\mathrm{core}}\) and match
     the canonical row exactly.
   - Falsifier: any coordinate requires retained curve history or a trade ledger.

4. **Trade22 sufficiency**
   - Recompute the five tail fields from \((c,w,\ell,g^+,g^-)\) exactly.
   - Falsifier: any tail field requires the full ledger.

5. **Break-even trade behavior**
   - Verify cases with `closed_trades > winning_trades + losing_trades`.
   - Falsifier: code silently assumes equality.

6. **Resolver coherence**
   - `metric_surface.rs` and `double_ma.rs` must resolve the same authoritative
     surface under the same request/route evidence.
   - Falsifier: `metric_contract=Full17` plus `include_trade_metrics=true`
     resolves differently across the two layers.

7. **Stride irrelevance**
   - Changing `tile_metrics_stride` without changing `metrics_count` must not
     change the public surface.
   - Falsifier: any consumer derives `core17`/`trade22` from stride.

8. **Normalization alias rule**
   - Raw absent sidecar and canonical explicit `legacy` must compare equal only
     after normalization.
   - Falsifier: raw absence remains a distinct public state in canonical mode.

9. **Non-finite policy**
   - Canonical behavior for non-finite rows must be tested and documented.
   - Falsifier: two code paths produce different canonical artifacts for the
     same underlying non-finite metric payload.

10. **Approximation quarantine**
    - If avg-times-count gross-sum reconstruction remains, it must be labeled as
      approximate and checked against the stated floating-point bound.
    - Falsifier: approximate gross sums are presented as exact.

### 10. Dependencies that must be reconciled during consolidation

1. **Resolver precedence**
   - `double_ma.rs:344-390` and `metric_surface.rs:142-309` must share one
     authoritative precedence order.

2. **Exact-tail provenance**
   - `composite_gpu_metric_decode.rs` placeholder semantics and
     `composite_cpu.rs` enrichment semantics must meet at a single publication
     rule for `trade22`.

3. **Canonicalization boundary**
   - `run_artifacts.rs` must be the single place where raw artifacts are turned
     into canonical public artifacts for parity checks.

4. **Gross-sum exactness**
   - `apply_trade_stats_to_metrics` in `composite_cpu.rs:647-655` must either
     retain direct gross sums or explicitly remain an approximation with the
     stated bound.

5. **Semantic width versus stride**
   - GPU kernel diagnostics and runtime publication must continue to treat
     `metrics_count` as semantic and `tile_metrics_stride` as transport-only.

6. **Dynamic and hybrid routes**
   - Any route that wants to publish `trade22` must either expose exact 22-row
     evidence or undergo exact enrichment before publication.

### 11. Kill criteria

PR-01 should be considered failed if any of the following remain true after
consolidation.

1. A published `trade22` row can still be produced from placeholder tail data.
2. Canonical artifact parity is not defined on normalized artifacts.
3. `core17` or `trade22` best-row publication requires an approximation without
   an explicit error contract.
4. `tile_metrics_stride` is observable as part of the public metric-surface
   semantics.
5. Resolver precedence remains split across `double_ma.rs` and
   `metric_surface.rs`.
6. Full artifact parity is demanded while explicit variable-size row buffering
   is forbidden.
7. Gross-profit/gross-loss reconstruction remains approximate but is treated as
   exact.

## Final surviving implementation-ready result

The strongest surviving repo-fit result is:

\[
\boxed{
\text{Exact PR-01 semantics split into two layers.}
}
\]

Layer 1 is the **canonical public artifact**
\[
P^\#(p)=\big(\sigma(p),\Phi_{\sigma(p)},B(p),T(p)\big),
\]
which is the true public contract after normalization.

Layer 2 is the **fixed-surface row contract**
\[
F_s(p),
\]
for which exact finite sufficient statistics do exist:

\[
\boxed{
F_{\mathrm{core17}} \text{ is exactly derivable from } S_{\mathrm{core}}.
}
\]

\[
\boxed{
F_{\mathrm{trade22}} \text{ is exactly derivable from }
(S_{\mathrm{core}}, c,w,\ell,g^+,g^-)
\text{ on exact-tail-supporting paths only.}
}
\]

And the full-artifact result is:

\[
\boxed{
\text{There is no fixed-size scalar-only sufficient statistic for exact } P^\#
\text{ under repo-admissible retained states.}
}
\]

That is the strongest implementation-ready mathematical statement I can defend
from the current repo without redesigning adjacent prompts.