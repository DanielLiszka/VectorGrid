
# PR-08 Runtime cache and execution economics — all three passes

## First-pass answer

Below is the first-pass answer for `PR-08`. I am treating it as an open mathematical problem over the uploaded owned surfaces, with the campaign files only as constraints. fileciteturn0file0

```md
## `PR-08` Runtime cache and execution economics

Owned problem:
runtime cache-value heuristics, data refresh or invalidation economics, and live
execution sizing or bracket translation

Source packets:
`MR-14`

Primary ownership files:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`

Context files:

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`

Own these symbols or decisions:

- data cache value or refresh heuristics
- retention and invalidation economics in `state.rs`
- execution sizing, quantity rounding, and bracket translation economics
- runtime prefetch or cache-performance interpretation

Open problem:

- characterize runtime cache and execution policy as a constrained optimization
  problem over retention value, refresh cost, and execution distortion

Why this is hard here:

- cache, invalidation, prefetch, and execution sizing decisions are discrete and
  operational, but they still imply an objective over cost, reuse, and decision
  quality
- repo-wide metric semantics are fixed elsewhere, so this prompt must optimize
  economics without changing public result meaning

Mathematical object:

- a constrained policy over cache states, refresh actions, and execution-size
  outputs with explicit cost and distortion functionals

Notation seed:

- let `x_t` be the runtime cache/state snapshot at decision time `t`
- let `u_t \in \{\mathrm{retain}, \mathrm{evict}, \mathrm{refresh},
  \mathrm{prefetch}\}` be the cache action
- let `L(x_t, u_t) = c_{\mathrm{io}} + c_{\mathrm{mem}} + c_{\mathrm{stale}}`
  be the runtime cost functional
- let `q_t = \pi_{\mathrm{exec}}(b_t, r_t, \sigma_t)` be the execution-size rule
  from budget/exposure inputs to actionable size
- policy quality should be stated as minimizing
  `\mathbb{E}\left[\sum_t L(x_t, u_t)\right]` subject to operational constraints

Primary questions:

- what policy minimizes expected refresh/reload cost under current reuse
  patterns?
- when should invalidation or prefetch fire under a bounded-staleness or
  bounded-regret policy?
- what execution sizing rule best trades exposure control against rounding and
  bracket distortion?

Required deliverables:

- formal cache/execution policy model
- candidate sufficient conditions for good runtime policy decisions
- adversarial cache or sizing scenarios
- explicit cost/distortion tradeoff analysis

Acceptance bar:

- every policy claim must define the objective it is optimizing, not just a
  heuristic rule

Verification hooks:

- campaign-perf evidence
- prefetch control harnesses
- live sizing falsifiers under volatile or thin scenarios

Non-goals:

- do not redesign metric-surface meaning or artifact publication; that belongs
  to `PR-01`
- do not redesign route-selection math; that belongs to `PR-03`
- do not redesign options-specific GPU runtime economics; that belongs to
  `PR-09`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- keep user-facing interpretability and operational stability in scope
```

## 1. Formal problem statement

At first pass, `PR-08` is best formalized as a **constrained partially observed control problem** with five coupled runtime subsystems:

1. binary cache reuse for CSV-backed candles,
2. in-memory residency and invalidation of base and derived datasets,
3. Polygon historical refresh decisions,
4. runtime host or VRAM prefetch decisions,
5. live execution translation from a strategy signal to an actionable delta order.

The hidden state is not just future requests. It also includes the true source-file content, latent interior integrity of cached market-data CSVs, future route eligibility for staging, and future broker or market states. The observable runtime state is only the metadata actually exposed by the owned files: file size and mtime for source identity, dataset version and ancestry, last-access sequence, strong-count pinning, Polygon manifest coverage and integrity summaries, preload state, current position snapshot, last close, and optional stop-loss or take-profit overrides. The current owned surfaces already encode these observables and constraints. fileciteturn4file0 fileciteturn3file13 fileciteturn4file6 fileciteturn4file1

The optimization target is:

\[
\min_{\pi \in \Pi} \; \mathbb E_\pi\!\left[\Phi_T\right]
\]

over a horizon \(T\), where \(\pi\) is an admissible runtime policy and \(\Phi_T\) is a cumulative runtime-economic loss. The hard constraints are semantic, not heuristic:

- a derived dataset may not be reused across a parent-version change,
- a live base mutation must invalidate all descendants before any derived reuse,
- a Polygon refresh decision must still return exact requested date coverage,
- an execution action must preserve the current delta-to-target position semantics,
- no policy may redesign metric meaning, route-selection math, or out-of-scope GPU economics.

So the open problem is not “find a faster heuristic.” It is:

> characterize the exact safe action set induced by current repo semantics, then optimize retention, refresh, invalidation, prefetch, and order-translation choices **within** that safe set.

That split matters. In this repo, some decisions are already exact coherence constraints, while others are genuine economics.

## 2. Notation for the search space, state space, path functional, ranking functional, and admissible approximations

Let decision times be \(t=0,1,\dots,T-1\).

### State space

Write the true runtime state as

\[
x_t = (\Xi_t, G_t, M_t, P_t, E_t) \in \mathcal X.
\]

Here:

- \(\Xi_t\) is the source-file state. For each source path \(p\), let
  \[
  \Xi_t(p)=\big(\chi_t(p),\, s_t(p),\, m_t(p),\, f_t(p)\big),
  \]
  where \(\chi_t(p)\) is true content, \(s_t(p)\) is file size, \(m_t(p)\) is mtime, and \(f_t(p)\) is an optional content fingerprint. In the current runtime cache surface, the operative identity is only \((s_t,m_t)\), even though a content fingerprint routine exists. fileciteturn4file0

- \(G_t\) is the in-memory dataset DAG. Each active dataset node \(i\) carries
  \[
  g_i(t)=\big(k_i,\; \mathrm{par}_i,\; \tau_i,\; v_i,\; \sigma_i,\; a_i,\; h_i,\; b_i\big),
  \]
  where \(k_i \in \{\mathrm{base},\mathrm{derived}\}\), \(\mathrm{par}_i\) is parent id if derived, \(\tau_i\) is timeframe, \(v_i\) is node version, \(\sigma_i\) is source version tag, \(a_i\) is last-access sequence, \(h_i=\mathbf 1\{\text{Arc strong count}>1\}\), and \(b_i\) is retained memory cost. The current `state.rs` logic explicitly stores the exact version fields that matter for derived freshness. fileciteturn3file14 fileciteturn4file4

- \(M_t\) is Polygon cache state. For each manifest dataset \(j\),
  \[
  m_j(t)=\big(d_j^{\min}, d_j^{\max}, T_j^{\min}, T_j^{\max}, n_j, \iota_j, \ell_j\big),
  \]
  where \(d^{\min},d^{\max}\) are cached date bounds, \(T^{\min},T^{\max}\) are min or max timestamps, \(n_j\) is row count, \(\iota_j\) is integrity metadata, and \(\ell_j\) is last-updated time. Polygon refresh planning currently works on date-coverage plus minimal timestamp validity, with integrity tracked separately. fileciteturn4file6

- \(P_t\) is preload or prefetch state, including whether host preload is available, whether VRAM staging is allowed by the campaign policy surface, whether staging is already resident, and whether prior fatal staging feedback has disabled future attempts. This is constrained by the campaign files but not owned by them.

- \(E_t\) is live execution state:
  \[
  E_t = \big(d_t,\; q_t^{\rm req},\; \hat q_t,\; c_t,\; \ell_t,\; \rho_t\big),
  \]
  where \(d_t \in \{-1,0,1\}\) is desired position sign from the strategy, \(q_t^{\rm req}\) is requested quantity, \(\hat q_t\) is normalized executable quantity, \(c_t\) is current broker position, \(\ell_t\) is last close, and \(\rho_t=(\rho_t^{sl},\rho_t^{tp})\) are optional stop-loss and take-profit percentages. In the current repo-fit surface, execution sizing is mostly **translation of an exogenous qty**, not endogenous portfolio sizing.

### Observable state

The runtime does not observe all of \(x_t\). It sees

\[
o_t = \mathcal O(x_t) \in \mathcal O,
\]

where \(\mathcal O\) exposes only current metadata. This is why the problem is partially observed.

### Search space

Let the admissible action set at time \(t\) be

\[
\mathcal A^{\rm safe}(o_t)
=
\mathcal A^{\rm disk}(o_t)
\times
\mathcal A^{\rm mem}(o_t)
\times
\mathcal A^{\rm poly}(o_t)
\times
\mathcal A^{\rm pref}(o_t)
\times
\mathcal A^{\rm exec}(o_t).
\]

A policy is admissible if it maps observable state to safe actions:

\[
\Pi
=
\left\{
\pi=(\pi_t)_{t=0}^{T-1}
:
\pi_t(o_t)\in \mathcal A^{\rm safe}(o_t)
\right\}.
\]

Write

\[
u_t=\pi_t(o_t)=
\big(
u_t^{\rm disk},
u_t^{\rm mem},
u_t^{\rm poly},
u_t^{\rm pref},
u_t^{\rm exec}
\big).
\]

### Repo-fit execution notation

The seed notation in the prompt,

\[
q_t = \pi_{\rm exec}(b_t,r_t,\sigma_t),
\]

is not the exact repo-fit owned object, because `commands_execution.rs` does not currently expose a budget state \(b_t\) or a portfolio optimizer. The owned execution-economics object is lower-level:

\[
\hat q_t = \mathrm{round}_{10^{-4}}(q_t^{\rm req}),
\qquad
q_t^\star = d_t \hat q_t,
\qquad
\Delta_t = q_t^\star - c_t.
\]

Here \(q_t^\star\) is target broker quantity and \(\Delta_t\) is the submitted delta. Optional bracket payloads are then translated from \((d_t,\ell_t,\rho_t)\).

### Path functional

For a trajectory \(\gamma\), define the cumulative runtime-economic loss

\[
\Phi_T(\gamma)
=
\sum_{t=0}^{T-1}
\Big[
\alpha_{\rm io} C_t^{\rm io}
+
\alpha_{\rm cpu} C_t^{\rm cpu}
+
\alpha_{\rm net} C_t^{\rm net}
+
\alpha_{\rm wait} C_t^{\rm wait}
+
\alpha_{\rm mem} C_t^{\rm mem}
+
\alpha_{\rm stale} C_t^{\rm stale}
+
\alpha_{\rm churn} C_t^{\rm churn}
+
\alpha_{\rm track} D_t^{\rm track}
+
\alpha_{\rm round} D_t^{\rm round}
+
\alpha_{\rm bracket} D_t^{\rm bracket}
\Big].
\]

Interpretation:

- \(C^{\rm io}\): disk reads or writes, cache load costs.
- \(C^{\rm cpu}\): CSV parse, resample, manifest scan, repair costs.
- \(C^{\rm net}\): Polygon fetch or broker call cost.
- \(C^{\rm wait}\): user-visible latency and campaign stall time.
- \(C^{\rm mem}\): retained-state cost.
- \(C^{\rm stale}\): semantic penalty for stale reuse or insufficient refresh.
- \(C^{\rm churn}\): operational instability penalty for oscillatory evict/refresh/prefetch behavior.
- \(D^{\rm track}\): position-tracking distortion, for example
  \[
  D_t^{\rm track} = \big|(c_t+\Delta_t) - q_t^\star\big|.
  \]
- \(D^{\rm round}\): quantity-rounding distortion,
  \[
  D_t^{\rm round}=|\hat q_t - q_t^{\rm req}|.
  \]
- \(D^{\rm bracket}\): bracket-translation distortion, including one-sided bracket omission or latent mismatch between submitted bracket prices and intended risk bounds.

The optimization problem is then

\[
\min_{\pi\in\Pi}\; \mathbb E[\Phi_T].
\]

### Ranking functionals

For a retainable object \(i\) in memory, define its continuation value as

\[
V_t(i)
=
\mathbb E\!\left[\Phi_{t:T}\mid o_t,\; \text{evict } i\right]
-
\mathbb E\!\left[\Phi_{t:T}\mid o_t,\; \text{retain } i\right].
\]

Positive \(V_t(i)\) means retention saves expected future cost.

For a prefetch candidate \(j\), define

\[
W_t(j)
=
\mathbb E\!\left[\text{future wait saved by prefetching } j \mid o_t\right]
-
C_t^{\rm pref}(j).
\]

Positive \(W_t(j)\) means prefetch is economically worthwhile.

### Candidate sufficient statistics

At first pass, the strongest repo-fit candidate sufficient statistics are:

\[
S_t^{\rm derived} = (\mathrm{par},\tau,v_{\rm base}),
\]

\[
S_t^{\rm ret} = (k_i,a_i,h_i,\mathrm{prot}(i)),
\]

\[
S_t^{\rm poly} = (\mathbf 1_{\rm csv}, d^{\min}, d^{\max}, T^{\min}, T^{\max}, f, t),
\]

\[
S_t^{\rm exec} = (d_t,q_t^{\rm req},c_t,\ell_t,\rho_t).
\]

The first, third, and fourth are plausible exact sufficient statistics for their local subproblems under repo semantics. The second is only a plausible heuristic summary for retention value.

### Admissible approximations

An admissible approximation is a policy \(\tilde \pi\) that uses compressed observable summaries but still respects all hard safety constraints:

\[
\widetilde \Pi
=
\left\{
\tilde\pi :
\tilde\pi(o_t)\in \mathcal A^{\rm safe}(o_t)
\text{ for all } o_t
\right\}.
\]

Repo-fit admissible approximations include:

- replacing true file identity by `SourceIdentity=(size,mtime)`,
- replacing continuation value by recency-based ranking with ancestor protection,
- replacing Polygon latent integrity by interval coverage plus manifest metadata,
- replacing latent risk-exact brackets by last-close percentage translation.

These are admissible only because they operate **inside** the safe action set. They are not exact by default.

## 3. Fixed invariants and interface constraints

The following are hard invariants of the owned surface.

First, disk-cache reuse is currently identity-based, not content-based. `data_cache.rs` stores and checks `SourceIdentity { size, mtime_utc_ms }`; `.vta` reuse is allowed only when the header is valid and the stored size and mtime match the current source. Sidecar cache is preferred, fallback app-cache is second. A content-fingerprint routine exists, but it is not the cache-validity criterion. fileciteturn4file0

Second, in-memory base reuse is exact only under same path plus same source identity. Derived dataset identity is `base_id@timeframe`, and derived reuse is exact only when parent id, timeframe, and `source_version` match the current base version. If not, stale descendants are removed and recomputed. Live bar upserts are only allowed on base datasets with an epoch-based timeline, and a base mutation invalidates all descendants. Base and derived retention budgets are separate and default to 1 each, with ancestor protection expanded from the protected set and tree removal as the atomic eviction action. fileciteturn3file13 fileciteturn4file4 fileciteturn3file17

Third, Polygon refresh is constrained to a three-action interface: `FullHit`, `AppendOnly`, or `FullRefresh`. The current planner uses cached date coverage and timestamp-validity to choose among them, with a same-day-tail override that prevents stale “full hit” treatment at the right edge. `FullHit` is cache-only and can succeed without an API key; non-hit paths require network fetch. Manifest integrity survives only when bytes are unchanged. fileciteturn4file6

Fourth, live execution is currently not a portfolio-allocation optimizer. It normalizes a positive finite requested quantity, infers desired position sign from the strategy, targets the delta between desired and current position, and only attaches bracket orders when opening from flat and both stop-loss and take-profit are present. That is the owned execution-translation surface. fileciteturn4file1 fileciteturn4file18

Fifth, campaign preload and VRAM staging are context constraints, not redesign targets here. They matter only insofar as a runtime value model for host preload or VRAM staging must respect the existing campaign interfaces and verification harnesses.

These invariants are not optional tuning knobs. Any approximation that violates them is out of scope.

## 4. Strongest plausible positive result

The strongest plausible positive result is a **conditional decomposition theorem**:

> The global PR-08 problem decomposes into an exact safety projector plus an approximate continuation-value ranking problem.

More precisely, under the assumptions that:

1. resampling is deterministic given a base dataset and timeframe,
2. the action space is restricted to the current repo interfaces,
3. parse, resample, download, wait, and retained-memory costs are additive across decision epochs,
4. execution semantics are one-step delta-to-target market-order semantics over an exogenous normalized qty,
5. any hidden file-content or latent-integrity uncertainty not observable at runtime is charged to \(C^{\rm stale}\),

there exists a projector \(S\) from observable state to safe action subsets such that:

\[
\min_{\pi\in\Pi}\mathbb E[\Phi_T]
=
\min_{\rho}
\mathbb E[\Phi_T \mid u_t \in S(o_t),\; u_t=\rho_t(S(o_t),o_t)].
\]

The exact part is \(S(o_t)\). The heuristic part is \(\rho_t\), which ranks safe actions by expected continuation value.

Inside that decomposition, three sub-results are especially strong:

1. **Exact dataset-coherence result.**  
   Reuse of a derived dataset is exact iff its parent id, timeframe, and stored source version match the current base version. Immediate descendant invalidation after a base mutation is necessary and sufficient for semantic freshness of all derived caches.

2. **Exact Polygon range result.**  
   Within the current action class \(\{\mathrm{FullHit},\mathrm{AppendOnly},\mathrm{FullRefresh}\}\), the date-range trichotomy is the byte-minimizing exact-coverage policy: hit if the request is already covered, append if only the right tail is missing, refresh if the requested left edge precedes the cached left edge.

3. **Exact one-step execution-translation result.**  
   For exogenous normalized qty \(\hat q_t\), desired sign \(d_t\), and current broker position \(c_t\), the delta rule
   \[
   \Delta_t = d_t\hat q_t - c_t
   \]
   uniquely minimizes one-step target-tracking error among market-order policies, with zero trade when \(|\Delta_t|\) is below tolerance.

What remains non-exact is the economic ranking among safe retain, evict, refresh, and prefetch options. That residual problem is genuinely an online value-estimation problem.

## 5. Strongest plausible impossibility result

The strongest plausible impossibility result is:

> No policy based only on the current observable runtime metadata can be both globally exact and globally cost-optimal over all admissible traces.

There are three independent reasons.

First, file identity is partially observed. Current runtime cache validity depends on \((\text{size},\text{mtime})\), not true content. Two different source files can therefore be observationally identical to the controller.

Second, interval coverage is not full integrity. A cached Polygon dataset can have the same outer date coverage yet differ internally by duplicates, gaps, or other latent corruption states; interval coverage alone does not identify that.

Third, retention and prefetch are online problems with unknown future requests. Even if identity were perfect, optimal retain or evict decisions still depend on future reuse order, which is not observable at decision time.

So exact global optimality is impossible unless at least one of the following is added:

- stronger observability, such as integrated content identity or stronger integrity summaries,
- a stochastic model of future requests and mutations,
- a regret contract instead of exact optimality,
- a narrower policy class.

There is also a second impossibility specific to execution: **endogenous portfolio sizing is not identifiable in the current owned surface.** The current execution path does not expose portfolio value, buying-power constraints, volatility target, risk budget, or multi-position coupling. So PR-08 can rigorously optimize **quantity rounding and bracket translation economics**, but not a full capital-allocation law from first principles.

## 6. Candidate theorem or lemma list

1. **Derived Freshness Lemma.**  
   For deterministic resampling operator \(R_\tau\), a stored derived node \(i\) is semantically fresh for base \(b\) and timeframe \(\tau\) iff
   \[
   (\mathrm{par}_i,\tau_i,\sigma_i) = (b,\tau,v_b).
   \]

2. **Descendant Invalidation Lemma.**  
   After any base mutation, invalidating every descendant in the dataset DAG is necessary and sufficient for exact freshness of future derived reuse.

3. **Range-Trichotomy Optimality Lemma.**  
   Within the current Polygon action class, `FullHit`, `AppendOnly`, and `FullRefresh` are the only exact-coverage extremal actions, and the planner’s trichotomy is byte-minimizing among them.

4. **Same-Day-Tail Freshness Lemma.**  
   If the requested interval ends on the cached rightmost day and the request is a same-day tail, then treating the request as a static full hit is not exact under bounded-right-edge freshness; append-only is the minimally invasive safe action.

5. **Delta-Execution Optimality Lemma.**  
   For exogenous \(\hat q\), desired sign \(d\in\{-1,0,1\}\), and current position \(c\), the submitted delta
   \[
   \Delta=d\hat q-c
   \]
   minimizes absolute target-position error after one market-order step.

6. **Bracket-Representability Lemma.**  
   Under the current execution interface, exact bracket representation exists only when both stop-loss and take-profit are present and the action is an opening trade; one-sided or mid-position bracket semantics are not exactly representable in the current surface.

7. **Protected-LRU Conditional Optimality Lemma.**  
   If future reuse hazard is monotone in recency, costs are additive, and ancestor protection is hard, then ordering eviction candidates lexicographically by
   \[
   (\mathbf 1\{\text{pinned}\},\; \text{last\_access\_seq},\; \text{id})
   \]
   is optimal within the class of policies that only use current node metadata.

8. **Identity-Indistinguishability Lemma.**  
   `SourceIdentity=(size,mtime)` is not a sufficient statistic for exact cache correctness.

9. **Online-Paging Reduction Lemma.**  
   The retention subproblem contains weighted online paging as a special case, so clairvoyant optimality is unattainable without additional assumptions.

## 7. One explicit counterexample family

Consider a family \(\{F_n\}_{n\ge 1}\) constructed as follows.

For each \(n\), create two CSV contents \(A_n\) and \(B_n\) such that:

- they live at the same path,
- they have the same byte length \(n\),
- they have the same mtime \(m_n\),
- they produce different candle arrays, especially a different final close:
  \[
  \ell(A_n)\neq \ell(B_n).
  \]

Build a `.vta` cache from \(A_n\). Then replace the source content by \(B_n\) while preserving path, length, and mtime.

From the controller’s point of view, the observable state is identical in the world where the content is still \(A_n\) and in the world where it has changed to \(B_n\). Therefore any observable policy must choose the same action in both worlds.

- If it reuses the cache, it is wrong in the \(B_n\) world.
- If it refreshes unconditionally, it is suboptimal in the unchanged \(A_n\) world.

So no policy based only on current observables can simultaneously guarantee exact freshness and minimal rebuild cost.

This family also couples directly into execution distortion. If a live signal opens a position with stop or take percentages \((\rho^{sl},\rho^{tp})\), then stale cache reuse changes the submitted bracket anchors by

\[
|\ell(A_n)-\ell(B_n)|\rho^{sl},
\qquad
|\ell(A_n)-\ell(B_n)|\rho^{tp}.
\]

So the same hidden source-identity ambiguity creates both cache error and bracket-translation error.

## 8. One lower-bound, identifiability, or impossibility angle

A clean lower-bound angle is the reduction to **online paging**.

Ignore Polygon and execution. Restrict attention to base datasets only. Assume all datasets have the same reload cost, all strong counts are 1, there are no derived descendants, and the base retention capacity is \(k\). Then the PR-08 retention problem contains the classical online paging problem on \(k\) slots as a special case.

Therefore:

- any online retain or evict policy without future knowledge inherits paging-style impossibility,
- no such policy can match clairvoyant offline optimal cost on all request sequences,
- adding ancestor-coupled tree eviction, derived-state invalidation, and prefetch only makes the problem harder, not easier.

So even with perfect file identity, exact no-regret retention is impossible without either stochastic assumptions or an explicit competitive or regret contract.

## 9. Unresolved questions that block implementation

1. **What is the true objective weighting?**  
   How should parse cost, wait cost, memory cost, network cost, stale-risk cost, and execution distortion be weighted against each other?

2. **What identity level is intended?**  
   Is `(size,mtime)` only a fast approximation, or is exact source-content identity required for correctness-sensitive reuse?

3. **What role should integrity play in refresh planning?**  
   Should Polygon refresh decisions remain date-coverage-only, or should manifest integrity alter the safe action set?

4. **What future-request model is admissible?**  
   Are campaign traces and preload harnesses to be treated as a stationary reuse model, a worst-case online model, or only as falsifiers?

5. **What exactly is “prefetch value”?**  
   Is it reduction in total wall time, reduction in median wait, probability of residency, route-stability benefit, or some weighted combination?

6. **Is quantity fundamentally exogenous?**  
   If yes, PR-08 owns only rounding and bracket translation. If no, then missing budget or exposure interfaces must be specified before execution sizing can be formalized.

7. **What is the intended distortion metric for brackets?**  
   Absolute price error, expected fill distortion, reject probability, or latent risk-budget mismatch?

8. **How should one-sided risk overrides be scored?**  
   The current surface can warn and omit brackets, but the optimization objective for that omission is not defined.

9. **What stability contract is required for interpretability?**  
   How much action churn is acceptable before the runtime policy becomes too opaque or too operationally unstable?

10. **What constitutes admissible approximation error?**  
   Is the required contract one of bias, regret, rank-instability, stale-probability, or a mix of these?

The first-pass conclusion is:

> `PR-08` is not one monolithic heuristic-tuning problem. It is an exact-safe-set problem plus an online value-estimation problem. Exactness lives in version coherence, range coverage, and delta-to-target execution semantics. Approximation enters only when ranking safe retain, refresh, or prefetch choices under hidden future reuse.


## Second-pass answer

Below is the adversarial second-pass answer for `PR-08`, following the uploaded prompt file. fileciteturn5file0 fileciteturn11file17

```md
## `PR-08` Runtime cache and execution economics

Owned problem:
runtime cache-value heuristics, data refresh or invalidation economics, and live
execution sizing or bracket translation

Source packets:
`MR-14`

Primary ownership files:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`

Context files:

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`

Own these symbols or decisions:

- data cache value or refresh heuristics
- retention and invalidation economics in `state.rs`
- execution sizing, quantity rounding, and bracket translation economics
- runtime prefetch or cache-performance interpretation

Open problem:

- characterize runtime cache and execution policy as a constrained optimization
  problem over retention value, refresh cost, and execution distortion

Why this is hard here:

- cache, invalidation, prefetch, and execution sizing decisions are discrete and
  operational, but they still imply an objective over cost, reuse, and decision
  quality
- repo-wide metric semantics are fixed elsewhere, so this prompt must optimize
  economics without changing public result meaning

Mathematical object:

- a constrained policy over cache states, refresh actions, and execution-size
  outputs with explicit cost and distortion functionals

Notation seed:

- let `x_t` be the runtime cache/state snapshot at decision time `t`
- let `u_t \in \{\mathrm{retain}, \mathrm{evict}, \mathrm{refresh},
  \mathrm{prefetch}\}` be the cache action
- let `L(x_t, u_t) = c_{\mathrm{io}} + c_{\mathrm{mem}} + c_{\mathrm{stale}}`
  be the runtime cost functional
- let `q_t = \pi_{\mathrm{exec}}(b_t, r_t, \sigma_t)` be the execution-size rule
  from budget/exposure inputs to actionable size
- policy quality should be stated as minimizing
  `\mathbb{E}\left[\sum_t L(x_t, u_t)\right]` subject to operational constraints

Primary questions:

- what policy minimizes expected refresh/reload cost under current reuse
  patterns?
- when should invalidation or prefetch fire under a bounded-staleness or
  bounded-regret policy?
- what execution sizing rule best trades exposure control against rounding and
  bracket distortion?

Required deliverables:

- formal cache/execution policy model
- candidate sufficient conditions for good runtime policy decisions
- adversarial cache or sizing scenarios
- explicit cost/distortion tradeoff analysis

Acceptance bar:

- every policy claim must define the objective it is optimizing, not just a
  heuristic rule

Verification hooks:

- campaign-perf evidence
- prefetch control harnesses
- live sizing falsifiers under volatile or thin scenarios

Non-goals:

- do not redesign metric-surface meaning or artifact publication; that belongs
  to `PR-01`
- do not redesign route-selection math; that belongs to `PR-03`
- do not redesign options-specific GPU runtime economics; that belongs to
  `PR-09`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- keep user-facing interpretability and operational stability in scope
```

## 1. Explicit failure modes

The first pass treated `PR-08` as a mostly unified constrained-control problem. That is too coarse. The owned code splits into three different observability regimes: exact coherence state, partially observed refresh state, and heuristic continuation-value state. The first pass blurred those together, which made some “sufficient statistics” and “optimality” claims too strong. fileciteturn8file17 fileciteturn8file6 fileciteturn9file13 fileciteturn10file13

The retention formalization is the weakest part of the first pass. The code does **not** implement “protected LRU” in any exact sense. It evicts by retention kind, excludes protected ids after ancestor expansion, then orders candidates by `(Arc::strong_count > 1, last_access_seq, id)` ascending. So it prefers evicting unshared entries before shared ones, then older access sequence, then lexicographically smaller ids. Eviction is not per-node either: it removes the chosen root and all descendants as a tree. Any theorem claiming LRU optimality, even conditionally, is broken unless it explicitly includes tree removal, sharing bias, and lexicographic tie-breaks. fileciteturn7file5 fileciteturn7file0 fileciteturn13file15

The proposed retention statistic \(S_t^{\rm ret}=(k_i,a_i,h_i,\mathrm{prot}(i))\) is not sufficient for continuation value. It omits subtree size, descendant hotness, descendant rebuild cost, context-cleanup cost, and whether an external `Arc` clone is present only transiently. Because `strong_count` directly enters eviction priority, two observationally similar datasets can swap priority due to an irrelevant external clone. That is path dependent and extrinsic to future reuse value. fileciteturn7file5 fileciteturn13file15

The first pass overstated disk-cache exactness. Runtime validity uses `SourceIdentity { size, mtime_utc_ms }`, and `.vta` reuse checks only stored size and mtime from the header against the current source identity, plus basic payload-shape validity. A stronger content fingerprint function exists, but it is not used by runtime cache validity. So the current disk-cache equivalence relation is an approximation to content identity, not an exact representation of it. fileciteturn8file0 fileciteturn8file5 fileciteturn8file6

The first pass also overstated Polygon-refresh exactness. The observable planner distinguishes `FullHit`, `AppendOnly`, and `FullRefresh`, and the same-day-tail case is forced to `AppendOnly`. But repo context says dataset validation and repair can address duplicates, out-of-order rows, parse errors, and large same-day gaps, optionally by refetching gappy days. That means outer date coverage is not the whole truth of dataset validity. The trichotomy is exact only on a quotient state that ignores latent interior corruption and repair obligations. It is not globally exact over true dataset quality. fileciteturn9file13 fileciteturn13file18

The execution model in the first pass missed two strong pathologies. First, `qty` is quantized to \(10^{-4}\) before target sizing, while order payload quantities are serialized to 6 decimals. Second, the live runner is not a FIFO processor of bars. When a run is inflight, intermediate live bars are collapsed to the newest pending timestamp only; older pending bars are discarded. So any theorem that treats runner execution as exact bar-by-bar replay is false. fileciteturn10file2 fileciteturn10file13 fileciteturn12file0 fileciteturn12file1 fileciteturn12file11

The first pass also failed to separate dry-run and live semantics. In live mode, current position is fetched from Alpaca and `delta_qty = target_position_qty - current_position_qty`. In dry-run mode, no broker position is fetched, so `current_position_qty` remains 0.0 and the same strategy signal produces a different delta law. Therefore “exact delta-to-target execution” is only true after conditioning on mode. fileciteturn10file13 fileciteturn10file8

Finally, the first-pass state space was too broad for live mutation. `upsert_live_bar` is only valid on base datasets and only on epoch-based timelines; derived datasets and relative timelines are rejected. So a global state model over arbitrary loaded datasets is not the owned live-update surface. fileciteturn13file2 fileciteturn13file0 fileciteturn13file9

## 2. Adversarial constructions and separating examples

### A. Tree-eviction separation

Let `base-a` be older, unshared, and have many hot derived descendants. Let `base-b` be newer, unshared, and have no descendants. Protect neither. The current rule compares only root sharing status, root access sequence, and id; it does **not** score descendant rebuild value. So it can evict `base-a`, which removes the whole `base-a` tree, even when `base-b` has much lower continuation value. This separates the actual rule from any root-local value-optimal policy. The tree-removal behavior is explicit in `remove_dataset_tree_locked`, while the ordering key is root-local. fileciteturn7file5

### B. External-Arc bias separation

Take two base entries with identical future reuse, but keep an irrelevant external `Arc` clone to one of them. The current rule pushes `Arc::strong_count > 1` later in eviction order, so the externally cloned entry is protected relative to an unshared one, even if the clone has no economic relevance. The test suite explicitly codifies this preference. That breaks any theorem identifying the current policy with value-optimal residency under purely internal reuse assumptions. fileciteturn7file5 fileciteturn13file15

### C. Lexicographic tie-break separation

Construct two candidate roots of the same retention kind with identical sharing status and identical `last_access_seq`. The implementation breaks ties by `id`. So a renaming of dataset ids can change eviction behavior without changing workload, memory, or reuse. Any theorem that ignores id-order dependence is false. fileciteturn7file5

### D. Size-and-mtime cache ambiguity

Build a `.vta` cache from a CSV \(A\). Replace the source by another CSV \(B\) with the same file size and the same mtime, but different candles. Runtime identity is unchanged, so the `.vta` cache is still accepted. Because the code has a stronger content-fingerprint function but does not consult it for validity, the current cache-validity equivalence class is strictly coarser than true content equality. This is a genuine separating family against exact content-correctness claims. fileciteturn8file0 fileciteturn8file5 fileciteturn8file6 fileciteturn8file14

### E. Polygon latent-integrity separation

Let cached outer coverage be `[2024-01-01, 2024-01-31]`, but let the interior contain duplicates or a large same-day gap on `2024-01-15`. Request `[2024-01-10, 2024-02-15]`. The observable trichotomy says `AppendOnly`, but repo context allows validation and repair for duplicates, out-of-order rows, parse errors, and large same-day gaps. So exact refresh correctness depends on interior validity, not just interval coverage. This breaks any “byte-minimizing exact coverage” theorem unless interior integrity is assumed away. fileciteturn9file13 fileciteturn13file18

### F. Runner bar-loss separation

Let live bars arrive at timestamps \(t_1<t_2<t_3\). Suppose the runner starts processing \(t_1\), remains inflight, then receives \(t_2\) and \(t_3\). The pending queue stores only the latest timestamp, so \(t_2\) can be overwritten by \(t_3\). The runner therefore processes a subsequence, not the full bar stream. Any first-pass theorem treating runner execution as exact per-bar control is invalid. fileciteturn12file0 fileciteturn12file1 fileciteturn12file11

### G. Dry-run/live separation

Fix a strategy with desired position \(+1\) and normalized qty \(1\). If the actual broker position is \(-1\), live mode produces \(\Delta=2\). Dry-run mode never fetches broker position and uses \(0\), so \(\Delta=1\). Therefore one-shot dry-run and live execution do not share the same target-tracking semantics. The first pass needed a mode-conditioned observable \(c_{\rm obs}(\text{mode})\), not a single current-position variable. fileciteturn10file13 fileciteturn10file8

### H. Bracket discontinuity at tiny residual positions

Brackets are attached only when opening from flat, implemented as `current_position_qty.abs() < 1e-6 && target_position_qty.abs() > 1e-6`. So a residual position of \(9\times 10^{-7}\) gets bracket treatment, while \(1.1\times 10^{-6}\) does not, even with identical strategy signal and risk bounds. This finite-grid discontinuity breaks any smooth distortion claim for brackets. fileciteturn10file8 fileciteturn9file12

## 3. Corrected theorem or lemma candidates

The strongest statements that survive are narrower.

**Lemma 1. Exact descriptive disk-cache validity.**  
Current disk-cache reuse is exact only with respect to the runtime equivalence relation
\[
\text{CSV} \sim_{\rm disk} \text{CSV}'
\iff
(\text{size},\text{mtime})=(\text{size}',\text{mtime}')
\]
together with successful `.vta` header and payload-shape validation. It is not exact with respect to file content. fileciteturn8file5 fileciteturn8file6

**Lemma 2. Exact in-memory base-reuse criterion.**  
A loaded base dataset is reused in memory iff the stored context has the same `csv_path`, the same `SourceIdentity`, and the corresponding dataset kind is `Base`. That is an exact characterization of current base-reuse semantics. fileciteturn8file15 fileciteturn8file8

**Lemma 3. Exact derived-freshness and parent-version race criterion.**  
A derived dataset `base_id@timeframe` is reused iff its `parent_data_id`, `timeframe`, and `source_version` match the current base id, parsed timeframe label, and current base version. Even then, insertion after fresh resampling is rejected if the base version changed during the resample window. This is the strongest exact repo-fit coherence result. fileciteturn7file1 fileciteturn13file11

**Lemma 4. Exact descendant invalidation after base mutation.**  
Any successful live base mutation removes all descendants immediately, and active derived reuse therefore fails after mutation until recomputed. This is exact under current `upsert_live_bar` semantics. fileciteturn13file9 fileciteturn13file7

**Lemma 5. Exact description of the retention rule, but no optimality.**  
For each retention kind separately, the current rule chooses a non-protected root minimizing
\[
(\mathbf 1\{\mathrm{shared}\},\; \text{last\_access\_seq},\; \text{id})
\]
lexicographically, after expanding protected ids upward to ancestors, then evicts that root and all descendants. This is an exact descriptive lemma only. Any optimality suffix should be deleted. fileciteturn7file5 fileciteturn7file0 fileciteturn7file8

**Lemma 6. Exact quantized execution-translation law.**  
Let
\[
\tilde q=\mathrm{round}_{10^{-4}}(q_{\rm req}),\qquad
q^\star=d\,\tilde q,
\]
where \(d\in\{-1,0,1\}\) is the latest open-trade direction from drilldown output. Then current one-shot live execution computes
\[
\Delta=q^\star-c_{\rm obs},
\]
where \(c_{\rm obs}=0\) in dry-run and \(c_{\rm obs}\) is fetched broker position in live mode. If \(|\Delta|<10^{-6}\), it returns hold. This is exact for current code, but only relative to the **quantized** target and the mode-specific observable current position. fileciteturn10file2 fileciteturn10file13 fileciteturn10file8

**Lemma 7. Exact bracket-representability condition.**  
Current bracket translation is exact only on opening-from-flat transitions with both `stop_loss_pct` and `take_profit_pct` present after sanitization. Otherwise the execution surface either emits a plain market order or warns and omits the bracket. fileciteturn9file12 fileciteturn9file15 fileciteturn10file15

**Lemma 8. Exact runner-coalescing description.**  
Under an active runner, the processed live-bar sequence is a generation-gated, symbol-matched subsequence in which, during inflight work, at most the newest pending bar is retained. Intermediate bars may be dropped. This is exact and should replace any per-bar exactness claim. fileciteturn12file0 fileciteturn12file1 fileciteturn12file14

**Lemma 9. Polygon trichotomy is only conditional.**  
`FullHit`, `AppendOnly`, and `FullRefresh`, with same-day-tail forcing `AppendOnly`, are exact only on the observable quotient state defined by cached outer interval coverage and boundary validity, under the assumption that interior integrity defects and repair obligations are absent or irrelevant. fileciteturn9file13 fileciteturn13file18

## 4. Weakest assumptions under which the surviving result holds

For the coherence lemmas, the weakest assumptions are deterministic resampling given parent candles and timeframe, faithful base-version increments on mutation, and no hidden base mutation that bypasses versioning. Those assumptions are repo-fit because `source_version` is stored on derived entries, base versions are compared before reuse, and a second parent-version recheck rejects stale insertion windows. fileciteturn7file1 fileciteturn13file11

For the quantized execution law, the weakest assumptions are: desired position is taken from the latest open trade direction, qty is normalized exactly once to \(10^{-4}\), live mode observes current broker position via Alpaca, dry-run mode uses zero current position, and the objective is one-step tracking to the current code’s quantized target. No portfolio budget, exposure optimizer, or slippage model is assumed or identifiable here. fileciteturn10file15 fileciteturn10file2 fileciteturn10file13

For the runner-coalescing lemma, the weakest assumptions are just the implemented single-runner slot, generation gate, active live-stream precondition, symbol matching, and latest-only pending update. Those are explicit in the code and repo map. fileciteturn12file12 fileciteturn13file13 fileciteturn13file18

For the conditional Polygon lemma, one must assume that refresh correctness is judged only by observable boundary coverage and not by latent interior corruption, repair, or refetch obligations. Without that restriction, the trichotomy does not survive as an exact theorem. fileciteturn9file13 fileciteturn13file18

## 5. Exact, approximate, and heuristic claims

The exact claims are now clear. Exact are: disk-cache validity with respect to `(size,mtime)` equivalence; in-memory base reuse by `(csv_path, SourceIdentity)`; derived freshness by `(parent_id,timeframe,source_version)` plus the parent-version recheck; descendant invalidation after successful base mutation; the quantized target-position and delta law; the bracket-attachment condition; the live-update admissible domain; and runner latest-only coalescing. fileciteturn8file5 fileciteturn8file15 fileciteturn7file1 fileciteturn13file9 fileciteturn10file13 fileciteturn9file12 fileciteturn12file0

Approximate claims are anything that speaks about *value* rather than *current semantics*: expected retention value, expected prefetch value, expected refresh savings, expected stale-risk under current reuse patterns, and any economics that depend on future requests or latent data defects. Those can be modeled, but not proved exact from the current observables. The perf harnesses are verification hooks for such approximations, not proofs of exact optimality. `ta_campaign_perf.rs` measures preload and route outcomes, while the CUDA prefetch profiler measures paired first/second-run timings and stage outcomes under an exact `GpuKernel` route. fileciteturn11file1 fileciteturn11file3 fileciteturn11file15

Heuristic claims are the current eviction ranking and the observable Polygon trichotomy when interpreted as full truth about future rebuild cost or true dataset validity. The retention rule is plainly a hand-chosen order statistic over sharing, recency, and id. The Polygon trichotomy is a hand-chosen observable plan class over hit/append/refresh. Both may be good heuristics, but neither is a global optimum theorem. fileciteturn7file5 fileciteturn9file13

## 6. Revised time, memory, and retained-state costs

The first-pass cost \(c_{\rm io}+c_{\rm mem}+c_{\rm stale}\) is too thin. The owned surfaces force a more structured decomposition.

For disk caching, the right runtime cost is
\[
C_{\rm disk}
=
c_{\rm stat}
+c_{\rm open}
+c_{\rm header}
+c_{\rm payload}(n)\,\mathbf 1_{\rm hit}
+c_{\rm csv\_parse}(n)\,\mathbf 1_{\rm miss}
+c_{\rm cache\_write}(n)\,\mathbf 1_{\rm miss\&write}
+c_{\rm false\_hit}\,\mathbf 1_{\rm same\_identity\;different\;content}.
\]
The last term is not observable at runtime, but it is mathematically necessary because runtime identity is only `(size,mtime)`. fileciteturn8file5 fileciteturn8file12 fileciteturn8file6

For retention, the correct eviction unit is a root tree, not a node:
\[
C_{\rm evict}(r)
=
\sum_{v\in T(r)} m(v)
+
R(T(r))
+
X(T(r)),
\]
where \(T(r)\) is the evicted root-plus-descendants tree, \(m(v)\) is retained memory, \(R(T(r))\) is future rebuild cost, and \(X(T(r))\) is context-cleanup or coherence loss. Any root-local objective that ignores descendants is formally misspecified for `state.rs`. fileciteturn7file5

For live base mutation, the owned cost is
\[
C_{\rm live\_mut}
=
c_{\rm upsert}
+
c_{\rm timeline\_normalize}
+
c_{\rm invalidate}\cdot |D(b)|,
\]
because every successful base mutation can force descendant invalidation of all \(D(b)\). That invalidation is exact and immediate. fileciteturn13file9 fileciteturn13file7

For observed Polygon refresh planning, the best second-pass cost is only conditional:
\[
C_{\rm poly}^{\rm obs}
=
c_{\rm hit}\,\mathbf 1_{\rm FullHit}
+
c_{\rm append}(\Delta^+)\,\mathbf 1_{\rm AppendOnly}
+
c_{\rm refresh}(\Delta)\,\mathbf 1_{\rm FullRefresh}
+
c_{\rm repair}\,\mathbf 1_{\rm latent\ integrity\ defect}.
\]
The repair term is the adversarial spoiler that prevents global exactness from outer coverage alone. fileciteturn9file13 fileciteturn13file18

For execution, the owned distortion is mode-conditioned and quantized:
\[
\tilde q=\mathrm{round}_{10^{-4}}(q_{\rm req}),\quad
q^\star=d\tilde q,\quad
\Delta=q^\star-c_{\rm obs}.
\]
A repo-fit economic cost is
\[
C_{\rm exec}
=
c_{\rm posfetch}\,\mathbf 1_{\rm live}
+
c_{\rm submit}\,\mathbf 1_{|\Delta|\ge 10^{-6}\wedge \rm live}
+
\lambda_q|\tilde q-q_{\rm req}|
+
\lambda_b B
+
\lambda_s S,
\]
where \(B\) measures bracket omission or bracket-price quantization distortion and \(S\) measures skipped or coalesced live bars. This is the minimum cost structure consistent with the owned surfaces. fileciteturn10file2 fileciteturn10file13 fileciteturn9file12 fileciteturn12file0

## 7. Dependency burden on other PR prompts

`PR-08` cannot settle route-selection mathematics. Campaign and VRAM-prefetch files are verification and constraint surfaces only. `commands_campaign.rs` exposes preload-pack preparation and VRAM-prefetch feedback hooks, while `ta_campaign_perf.rs` records preload wall time, campaign runtime, and per-dataset route or prefetch artifacts. Those files let `PR-08` falsify prefetch heuristics, but not redesign route selection, backend policy, or report semantics. fileciteturn11file10 fileciteturn11file12 fileciteturn11file1 fileciteturn11file3

`PR-08` also depends on the external Alpaca and Polygon interfaces as fixed constraints. Live execution requires Alpaca credentials unless `dry_run=true`; live auto-runner requires an active Polygon live stream for the same symbol; only one active live stream slot and one active live execution runner slot exist. Those are hard interface burdens, not optimization variables owned here. fileciteturn13file18 fileciteturn13file13 fileciteturn12file12

Finally, `PR-08` inherits output and metric non-goals from the prompt. So third-pass results may only strengthen runtime coherence, bounded approximation contracts, or execution-translation math inside the owned files. They may not redesign product metrics, artifact meaning, route selection, or out-of-scope GPU economics. fileciteturn11file5

## Narrowed strongest surviving repo-fit result

After the second pass, the strongest surviving result is not a global optimal-policy theorem.

It is this:

> **The exact owned mathematical core of `PR-08` is a coherence-and-translation theorem, not a value-optimality theorem.**  
> Exactness survives for:
> 1. base and derived cache coherence on the equivalence classes the repo actually tracks,
> 2. descendant invalidation after successful base mutation,
> 3. quantized delta-to-target execution translation and bracket representability on the current live order surface,
> 4. latest-only coalescing semantics of the live runner.  
> Everything else—retention value, prefetch value, and refresh optimality beyond observable range planning—must be demoted to approximate or heuristic status with an explicit bias, regret, or falsification contract in the third pass. fileciteturn8file15 fileciteturn7file1 fileciteturn13file7 fileciteturn10file13 fileciteturn9file12 fileciteturn12file0

That is the narrowest statement I think still fully survives adversarial attack on the current repo-owned surface.

## Third-pass answer

Below is the third-pass answer for `PR-08`. This narrows to the strongest implementation-ready mathematical result that survives the second pass, and every non-exact claim carries an explicit contract. The acceptance bar and non-goals come directly from the uploaded prompt. fileciteturn15file10 fileciteturn13file13

```md
## `PR-08` Runtime cache and execution economics

Owned problem:
runtime cache-value heuristics, data refresh or invalidation economics, and live
execution sizing or bracket translation

Source packets:
`MR-14`

Primary ownership files:

- `src-tauri/src/data_cache.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `src-tauri/src/state.rs`

Context files:

- `src-tauri/src/commands_campaign.rs`
- `src-tauri/src/bin/ta_campaign_perf.rs`
- `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs`

Own these symbols or decisions:

- data cache value or refresh heuristics
- retention and invalidation economics in `state.rs`
- execution sizing, quantity rounding, and bracket translation economics
- runtime prefetch or cache-performance interpretation

Open problem:

- characterize runtime cache and execution policy as a constrained optimization
  problem over retention value, refresh cost, and execution distortion

Why this is hard here:

- cache, invalidation, prefetch, and execution sizing decisions are discrete and
  operational, but they still imply an objective over cost, reuse, and decision
  quality
- repo-wide metric semantics are fixed elsewhere, so this prompt must optimize
  economics without changing public result meaning

Mathematical object:

- a constrained policy over cache states, refresh actions, and execution-size
  outputs with explicit cost and distortion functionals

Notation seed:

- let `x_t` be the runtime cache/state snapshot at decision time `t`
- let `u_t \in \{\mathrm{retain}, \mathrm{evict}, \mathrm{refresh},
  \mathrm{prefetch}\}` be the cache action
- let `L(x_t, u_t) = c_{\mathrm{io}} + c_{\mathrm{mem}} + c_{\mathrm{stale}}`
  be the runtime cost functional
- let `q_t = \pi_{\mathrm{exec}}(b_t, r_t, \sigma_t)` be the execution-size rule
  from budget/exposure inputs to actionable size
- policy quality should be stated as minimizing
  `\mathbb{E}\left[\sum_t L(x_t, u_t)\right]` subject to operational constraints

Primary questions:

- what policy minimizes expected refresh/reload cost under current reuse
  patterns?
- when should invalidation or prefetch fire under a bounded-staleness or
  bounded-regret policy?
- what execution sizing rule best trades exposure control against rounding and
  bracket distortion?

Required deliverables:

- formal cache/execution policy model
- candidate sufficient conditions for good runtime policy decisions
- adversarial cache or sizing scenarios
- explicit cost/distortion tradeoff analysis

Acceptance bar:

- every policy claim must define the objective it is optimizing, not just a
  heuristic rule

Verification hooks:

- campaign-perf evidence
- prefetch control harnesses
- live sizing falsifiers under volatile or thin scenarios

Non-goals:

- do not redesign metric-surface meaning or artifact publication; that belongs
  to `PR-01`
- do not redesign route-selection math; that belongs to `PR-03`
- do not redesign options-specific GPU runtime economics; that belongs to
  `PR-09`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- keep user-facing interpretability and operational stability in scope
```

## 1. Final formalization

The strongest surviving formalization is a **hybrid exact-or-bounded policy decomposition**:

\[
\pi_t(o_t) = \rho_t\big(S(o_t),\, \hat\theta_t\big),
\]

where:

- \(o_t\) is the observable runtime state,
- \(S(o_t)\) is an **exact safe-set projector** determined by repo semantics,
- \(\hat\theta_t\) is a vector of empirical value estimates used only for non-exact ranking,
- \(\rho_t\) chooses among safe actions, never outside them.

The observable state is

\[
o_t = (\omega_t,\mathcal G_t,\mathcal P_t,\mathcal E_t,\mathcal R_t),
\]

with the following repo-fit components.

First, the disk-cache state is

\[
\omega_t = (p_t,s_t,m_t,h_t,f_t),
\]

where \(p_t\) is the source path, \(s_t\) is source size, \(m_t\) is source mtime in UTC ms, \(h_t\) is the `.vta` header validity state, and \(f_t\) is an optional stronger fingerprint if a consolidation later chooses to persist one. The current exact code surface uses only \((s_t,m_t)\) plus valid header or payload-shape checks for runtime reuse. `SourceIdentity` is exactly `(size,mtime_utc_ms)`, `load_or_build_with_identity` prefers sidecar then fallback cache, and `try_load_cache` rejects cached payloads when header size or mtime differs. A stronger `content_fingerprint_for_source` exists but is not part of current reuse validity. fileciteturn13file17 fileciteturn13file14 fileciteturn15file2 fileciteturn15file13

Second, the in-memory dataset state is a rooted forest

\[
\mathcal G_t = \{v=(id,k,parent,\nu,\sigma,a,\chi,b)\},
\]

where \(k\in\{\mathrm{Base},\mathrm{Derived}\}\), `parent` is the parent id for derived entries, \(\nu\) is the current dataset version, \(\sigma\) is the stored `source_version`, \(a\) is `last_access_seq`, \(\chi\) is the external-sharing flag \(\mathbf 1\{\mathrm{Arc::strong\_count}>1\}\), and \(b\) is retained bytes. For a root \(r\), let \(T_t(r)\) be the root-plus-descendants tree and let

\[
M_t(r)=\sum_{v\in T_t(r)} b(v)
\]

be its retained-state mass. Current base reuse scans loaded context for matching `(csv_path, source_identity)` and updates `last_access_seq`; current derived reuse is keyed by `base_id@timeframe` and is fresh iff `(parent_data_id,timeframe,source_version)` matches the current base id, timeframe label, and base version. Stale derived trees are removed before reinsertion. fileciteturn16file9 fileciteturn14file11 fileciteturn14file3

Third, the Polygon cache state is

\[
\mathcal P_t = (e_t,d_t^{\min},d_t^{\max},T_t^{\min},T_t^{\max},\kappa_t,u_t^{\rm last}),
\]

where \(e_t\) is CSV existence, \([d_t^{\min},d_t^{\max}]\) is outer date coverage, \([T_t^{\min},T_t^{\max}]\) is timestamp coverage, \(\kappa_t\) is integrity metadata, and \(u_t^{\rm last}\) is last update time. The current planner exposes `FullHit`, `AppendOnly`, and `FullRefresh`, with a same-day-tail override to `AppendOnly`, and `integrity_after_download` preserves integrity only when bytes are unchanged. Separate validation and repair surfaces exist and can count parse errors, duplicates, out-of-order rows, and large same-day gaps, with best-effort gap refetch by day when an API key is available. fileciteturn15file5 fileciteturn16file19 fileciteturn16file7

Fourth, the execution state is

\[
\mathcal E_t=(m_t^{\rm mode},d_t,q_t,\tilde q_t,c_t,\ell_t,\rho_t),
\]

where \(m_t^{\rm mode}\in\{\mathrm{dry\_run},\mathrm{live}\}\), \(d_t\in\{-1,0,1\}\) is desired position sign, \(q_t\) is requested quantity, \(\tilde q_t=\mathrm{round}_{10^{-4}}(q_t)\), \(c_t\) is observed current broker position, \(\ell_t\) is last close, and \(\rho_t\) is the stop or take pair after risk sanitization. The current one-shot translation computes

\[
q_t^\star=d_t\tilde q_t,\qquad
\Delta_t=q_t^\star-c_t.
\]

In dry-run, \(c_t=0\) because no broker position fetch occurs; in live mode, \(c_t\) is fetched from Alpaca. The action is `hold` when \(|\Delta_t|<10^{-6}\). When an order is emitted, quantity is serialized to 6 decimals, and bracket prices are serialized to 4 decimals. Brackets are attached only on opening-from-flat and only when both stop-loss and take-profit are present. fileciteturn13file6 fileciteturn16file8 fileciteturn16file1

Fifth, the runner state is

\[
\mathcal R_t=(g_t,\mathrm{active}_t,\mathrm{inflight}_t,\mathrm{pending}_t,\tau_t^{\rm last},s_t^{\rm skip}),
\]

where \(g_t\) is the generation, `active` is whether a runner exists, `inflight` is whether a run is executing, `pending` is the newest retained pending bar, \(\tau_t^{\rm last}\) is the latest processed open timestamp, and \(s_t^{\rm skip}\) is skip count. Current runner semantics are exact latest-only coalescing: if a matching bar arrives during inflight work, `update_pending_bar` keeps only the newest pending timestamp; older intermediate bars may be dropped. Starting a runner requires an active live candle stream for the same symbol. fileciteturn15file4 fileciteturn15file6

With that state, the implementation-ready objective is

\[
\min_{\pi\in\Pi_{\rm safe}}
\mathbb E\!\left[\sum_{t=0}^{T-1}
\Big(
C_t^{\rm exact}
+
\widehat C_t^{\rm retain}
+
\widehat C_t^{\rm pref}
\Big)\right].
\]

Here \(C_t^{\rm exact}\) contains only semantics that can be stated exactly from the owned surfaces:

\[
C_t^{\rm exact}
=
C_t^{\rm io}
+
C_t^{\rm cpu}
+
C_t^{\rm net}
+
C_t^{\rm wait}
+
\lambda_q |\tilde q_t-q_t|
+
\lambda_h \mathbf 1\{|\Delta_t|<10^{-6}\}|\Delta_t|
+
\lambda_{b,\rm omit}\mathbf 1_{\rm bracket\ omitted}
+
\lambda_{b,\rm px}\epsilon_t^{\rm bracket\_px}
+
\lambda_{r}\mathbf 1_{\rm skipped\ bars\ interpreted\ as\ replay}.
\]

The non-exact terms are restricted to retention ranking and prefetch admission, and they are only allowed with explicit contracts:

\[
\widehat C_t^{\rm retain}(r)=
\hat p_{h,r}\hat B_r
+\Gamma_{\rm shared}\mathbf 1_{\chi(r)=1}
-\lambda_M M_t(r),
\]

\[
\widehat C_t^{\rm pref}(j)=
-\hat\mu_j
+\Gamma_{\rm fatal}\hat p^f_j
+\lambda_{stage}m_j.
\]

Interpretation:

- \(\hat p_{h,r}\) is an empirical \(h\)-step reuse probability for root-tree \(r\),
- \(\hat B_r\) is an empirical cold-rebuild cost for root-tree \(r\),
- \(M_t(r)\) is subtree memory, not root-only memory,
- \(\Gamma_{\rm shared}\) is an explicit operational-bias term that protects externally shared roots,
- \(\hat\mu_j\) is estimated prefetch wall-time savings for candidate \(j\),
- \(\hat p^f_j\) is measured fatal-stage probability for candidate \(j\),
- \(m_j\) is stage memory footprint.

The safe-set projector \(S(o_t)\) is the exact part of the theorem:

1. disk reuse is safe only on the chosen equivalence class;
2. base reuse is safe only on matching `(csv_path, SourceIdentity)`;
3. derived reuse is safe only on matching `(parent_data_id,timeframe,source_version)` plus the parent-version recheck before insert;
4. live mutation is safe only on base datasets with epoch-based timelines;
5. Polygon `FullHit` and `AppendOnly` are exact only on validated clean integrity state;
6. execution translation is exact only for the quantized target and mode-specific observed current position;
7. bracket attachment is exact only on opening-from-flat with both risk bounds present;
8. latest-only runner coalescing is exact as a runtime semantics, but not as a claim of per-bar replay.

This decomposition is the strongest result that survives the adversarial pass.

## 2. Final theorem or lemma list

**Theorem 1. Safe-set decomposition theorem.**  
`PR-08` decomposes into an exact safe-set projector \(S\) and an estimation layer \(\rho\). Any admissible implementation-ready policy must first project to \(S(o_t)\) and only then rank within that safe set. No global optimality claim survives outside this decomposition. This is the core surviving theorem. It matches the current exact base or derived coherence checks, the live-update admissibility domain, the execution delta law, and the runner slot semantics. fileciteturn16file9 fileciteturn14file11 fileciteturn14file16 fileciteturn15file4

**Lemma 2. Weak disk-identity exactness.**  
Current `data_cache.rs` is exact only on the weak equivalence class

\[
[x]_{\rm disk}^{\rm weak}=[(s,m,h)],
\]

meaning identical source size, identical source mtime, and a valid `.vta` header or payload-shape. It is not exact on content identity. `try_load_cache` checks the stored `csv_size` and `csv_mtime_ms` against `SourceIdentity`; `content_fingerprint_for_source` does not currently participate in validity. fileciteturn13file17 fileciteturn15file2

**Lemma 3. Exact base-reuse equivalence class.**  
In-memory base reuse is exact on

\[
[x]_{\rm base}=[(\text{csv\_path},\text{SourceIdentity})].
\]

`find_matching_loaded_base` scans loaded context for that pair and reuses only base entries. This is the exact current semantic reuse class for loaded bases. fileciteturn16file9 fileciteturn16file11

**Lemma 4. Exact derived-reuse and race-guard theorem.**  
Derived reuse is exact on

\[
[x]_{\rm derived}=[(\text{parent\_data\_id},\text{timeframe},\text{base\_version})].
\]

If a cached derived entry matches those fields, it is reused; otherwise its whole stale tree is removed. Even after fresh resampling, insertion is rejected when the base version changed during the resample window. This is the strongest exact coherence result in `state.rs`. fileciteturn14file11 fileciteturn14file3

**Lemma 5. Exact descendant invalidation after live base mutation.**  
`upsert_live_bar` is exact only on base datasets with epoch-based timelines, and any successful mutation invalidates all descendants immediately. Relative-timeline and derived targets are rejected. So freshness after base mutation is restored only by descendant removal, not by lazy reuse. fileciteturn14file16 fileciteturn16file12

**Lemma 6. Exact quantized execution theorem.**  
Let \(q>0\) be requested qty, \(\tilde q=\mathrm{round}_{10^{-4}}(q)\), \(d\in\{-1,0,1\}\) the desired sign, and \(c_{\rm obs}\) the mode-specific observed current position. Then the exact one-shot target is

\[
q^\star=d\tilde q,\qquad \Delta=q^\star-c_{\rm obs}.
\]

If \(|\Delta|<10^{-6}\), action is `hold`; otherwise live mode submits an order and dry-run returns `dry_run_signal_only`. This theorem is exact relative to the quantized target, not the original unquantized request. fileciteturn13file6 fileciteturn16file8

**Lemma 7. Exact execution distortion bounds.**  
The current live execution surface induces exact deterministic distortion bounds:

\[
|\tilde q-q| \le 5\times 10^{-5},
\]

\[
|\Delta^{\rm submitted}-\Delta| \le 5\times 10^{-7},
\]

\[
\epsilon_{\rm hold}<10^{-6},
\]

\[
|p_{\rm stop}^{out}-p_{\rm stop}^\star| \le 5\times 10^{-5},
\qquad
|p_{\rm take}^{out}-p_{\rm take}^\star| \le 5\times 10^{-5},
\]

where the second line is the extra 6-decimal formatting error for submitted quantity, and the last line is the 4-decimal formatting error for stop-loss and take-profit payload prices. `normalize_execute_request` rounds to \(10^{-4}\), payload qty is formatted to 6 decimals, and bracket prices are formatted to 4 decimals. fileciteturn13file6 fileciteturn16file1

**Lemma 8. Exact bracket-representability theorem.**  
Current brackets are exactly representable only for opening-from-flat transitions with both stop-loss and take-profit present. Otherwise the order surface is a plain market order and may emit a warning about omitted brackets. There is no exact one-sided bracket representation in the current live execution surface. fileciteturn13file15 fileciteturn16file1

**Lemma 9. Exact latest-only runner theorem.**  
The live runner is exact as a latest-only event-throttled controller: if idle, it starts on the incoming matching bar; if inflight, it records at most the newest pending bar and may skip earlier ones; if the incoming timestamp is not newer than the latest seen bar, it increments `skip_count` and discards it. This is exact current behavior. It is not an exact theorem for bar-by-bar replay. fileciteturn15file4 fileciteturn15file7 fileciteturn12file0

**Lemma 10. Integrity-gated Polygon exactness theorem.**  
The observable trichotomy `FullHit/AppendOnly/FullRefresh` is exact only on the validated-clean quotient state

\[
[x]_{\rm poly}^{\rm clean}
=
[(e,d^{\min},d^{\max},T^{\min},T^{\max},\kappa,u^{\rm last})]
\]

with the conditions:

- `integrity.is_some()`,
- all integrity counters are zero,
- `integrity.checked_utc_ms \ge last_updated_utc_ms`,
- requested interval is fully covered for `FullHit`,
- same-day-tail requests are excluded from `FullHit`,
- append-only is used only when the left edge is already covered and boundary timestamps are valid.

If integrity is dirty, missing, or stale, exactness requires validation or repair before any hit claim. `polygon.rs` already contains the necessary validation and repair surfaces. fileciteturn15file5 fileciteturn16file19 fileciteturn16file7

**Theorem 11. No finite sufficient statistic for exact retention value.**  
Under the current observable interface, there is no finite-dimensional sufficient statistic that yields exact continuation value for retention or prefetch on arbitrary traces. This follows from the online paging reduction, subtree-removal dependence, external-`Arc` sharing bias, and unknown future reuse. Any retention or prefetch score must therefore be approximate and must carry an explicit contract. The current root-local ordering in `eviction_candidate_locked` is descriptive only, not optimal. fileciteturn15file3 fileciteturn15file0

**Theorem 12. Certified subtree-aware retention ranking under stationarity.**  
Assume a windowed stationary reuse model over a chosen horizon \(h\). For each candidate root-tree \(r\), define:

- \(Y_{r,i}\in\{0,1\}\): indicator that \(T(r)\) is reused within the next \(h\) relevant requests after observation \(i\),
- \(B_{r,j}\in[\underline B,\overline B]\): observed cold rebuild cost for \(T(r)\),
- \(M(r)\): current retained subtree mass,
- \(\chi(r)\in\{0,1\}\): current sharing flag.

Define the keep-score

\[
K_r
=
p_r \bar B_r
+
\Gamma_{\rm shared}\chi(r)
-
\lambda_M M(r),
\]

with estimator

\[
\hat K_r
=
\hat p_r \hat B_r
+
\Gamma_{\rm shared}\chi(r)
-
\lambda_M M(r).
\]

Then with probability at least \(1-\delta\), uniformly over \(N\) simultaneous candidate roots,

\[
|K_r-\hat K_r|
\le
\epsilon_r
:=
\overline B\sqrt{\frac{\log(4N/\delta)}{2n_r}}
+
(\overline B-\underline B)\sqrt{\frac{\log(4N/\delta)}{2m_r}}.
\]

The induced ordering between roots \(r\) and \(s\) is certified whenever

\[
|\hat K_r-\hat K_s| > \epsilon_r+\epsilon_s.
\]

When intervals overlap, any tie-break is heuristic only. This gives an explicit rank-instability contract rather than an optimality claim.

**Theorem 13. One-sided prefetch admission theorem.**  
Assume the prefetch profiler or campaign harness supplies repeated host-vs-staged wall-time samples for candidate \(j\). Let \(X_{j,i}=T^{host}_{j,i}-T^{staged}_{j,i}\in[-W,W]\) and let \(\mu_j=\mathbb E[X_{j,i}]\). Let \(F_{j,i}\in\{0,1\}\) indicate fatal stage failure. Then with probability at least \(1-\delta\),

\[
\mu_j \ge \hat\mu_j - W\sqrt{\frac{2\log(4K/\delta)}{n_j}},
\]

\[
p^f_j \le \hat p^f_j + \sqrt{\frac{\log(4K/\delta)}{2n_j}},
\]

uniformly over \(K\) candidates. Therefore a prefetch action is admissible only when

\[
\hat\mu_j - W\sqrt{\frac{2\log(4K/\delta)}{n_j}} > \eta_{\rm wall}
\]

and

\[
\hat p^f_j + \sqrt{\frac{\log(4K/\delta)}{2n_j}} < p_{\max}.
\]

This is a one-sided false-positive contract: prefetch is enabled only when measured latency savings remain positive after confidence correction and measured fatal-stage risk remains below threshold. The profiler already records stage outcomes and mean first/second/pair wall times, and it rejects runs that leave the exact `GpuKernel` route. fileciteturn16file3 fileciteturn16file4 fileciteturn16file16 fileciteturn16file18

**Theorem 14. Runner approximation bound, conditional only.**  
If a consolidation chooses to interpret latest-only runner output as an approximation to full per-bar replay, then a bound is available only for strategies whose decision map \(g\) is certified \(L_g\)-Lipschitz on a chosen candle-state metric \(d_{\mathcal C}\). In that case,

\[
|g(C^{full})-g(C^{coal})|
\le
L_g\, d_{\mathcal C}(C^{full},C^{coal}).
\]

Without a certified \(L_g\), no finite deterministic approximation bound exists. So latest-only runner semantics are exact as latest-only semantics, but approximate replay claims require explicit strategy sensitivity certification.

## 3. Proof sketch or proof plan

The proofs split cleanly into exact code-surface arguments and probabilistic approximation arguments.

For Lemmas 2 through 10, the proof plan is direct inspection of keys, guards, and update rules in the owned files. `data_cache.rs` proves weak disk identity because cache reuse compares header size and mtime to the current `SourceIdentity` and does not compare fingerprints. `state.rs` proves exact base reuse, exact derived freshness, and descendant invalidation because those conditions are the literal equality checks and tree-removal operations used before reuse and after live mutation. `commands_execution.rs` proves the quantized target law, the hold dead-zone, the mode-conditioned current-position observable, the opening-from-flat bracket predicate, and latest-only pending-bar retention because those are the exact branch conditions and string-formatting rules used at runtime. `polygon.rs` proves the observable trichotomy and separately proves that validation and repair exist, so exactness is recovered only after gating the plan on clean, fresh integrity state. fileciteturn15file2 fileciteturn16file9 fileciteturn14file16 fileciteturn16file8 fileciteturn15file4 fileciteturn16file19

The no-sufficient-statistic theorem is proved by counterexample. Retention depends on future reuse, subtree size, subtree rebuild cost, and external sharing, all of which can be varied while holding root-local recency and identity fixed. Prefetch depends on future route stability and next-dataset timing. So any exact continuation-value statistic over current local metadata fails on an adversarial pair of traces. The online paging reduction supplies the lower bound. The same style of indistinguishability argument rules out exact content correctness from weak disk identity and rules out exact replay from latest-only runner coalescing. fileciteturn15file3 fileciteturn12file0

The certified retention and prefetch theorems use standard concentration arguments. For retention, decompose the score error as

\[
|p_r\bar B_r-\hat p_r\hat B_r|
\le
\bar B_r |p_r-\hat p_r|
+
|\bar B_r-\hat B_r|,
\]

then apply Hoeffding bounds to the Bernoulli reuse indicators and bounded rebuild samples, followed by a union bound over simultaneously ranked roots. The score-certification margin condition follows immediately. For prefetch, apply Hoeffding to bounded paired wall-time differences and to Bernoulli fatal-stage indicators. The one-sided admission inequalities are just lower and upper confidence bounds, respectively.

The conditional runner bound is a short functional-analysis argument: if the decision map is Lipschitz under a chosen metric, then replacing the full bar suffix by the coalesced suffix perturbs the decision by at most \(L_g\) times the state-distance. Without that regularity assumption, no nontrivial bound is possible.

## 4. Sufficient statistics, or proof they do not exist under the stated constraints

Exact sufficient statistics **do** exist for the exact subproblems:

\[
S_t^{\rm base}=(\text{csv\_path},\text{SourceIdentity}),
\]

\[
S_t^{\rm derived}=(\text{parent\_data\_id},\text{timeframe},\text{base\_version}),
\]

\[
S_t^{\rm live\_mut}=(\text{DatasetKind},\text{timeline\_class}),
\]

\[
S_t^{\rm exec}=(m^{\rm mode},d,q,\tilde q,c_{\rm obs},\ell,\rho),
\]

\[
S_t^{\rm runner}=(\text{symbol},\text{generation},\text{inflight},\text{pending\_latest},\tau^{\rm last}).
\]

Those are exact because they match the owned guards and equality checks. Base reuse depends only on path and `SourceIdentity`; derived reuse depends only on parent id, timeframe label, and base version; live mutation admissibility depends only on base-vs-derived kind and epoch-vs-relative timeline; execution translation depends only on mode, sign, quantized qty, observed current position, last close, and risk pair; runner semantics depend only on symbol match, generation, inflight state, pending latest, and latest timestamp. fileciteturn16file9 fileciteturn14file11 fileciteturn14file16 fileciteturn16file8 fileciteturn15file4

Exact sufficient statistics **do not** exist for arbitrary-workload continuation value of retention or prefetch under the current constraints. There is no finite-dimensional exact statistic because future requests are unobserved, subtree rebuild cost is not root-local, external `Arc` clones perturb current sharing state without changing future reuse, and prefetch value depends on future dataset order and route residency. The strongest admissible result is therefore an approximate statistic with a contract:

\[
A_t^{\rm retain}(r)=
\big(M_t(r),\chi_t(r),\hat p_{h,r},\hat B_r,\epsilon_r\big),
\]

\[
A_t^{\rm pref}(j)=
\big(\hat\mu_j,\hat p^f_j,\epsilon^{\mu}_j,\epsilon^f_j\big).
\]

These are not exact sufficient statistics; they are only contract-bearing approximate summaries.

## 5. Exactness conditions and equivalence classes

The exactness conditions are now fully separated.

For disk caching there are two equivalence classes. The current code is exact only on the weak class

\[
[x]_{\rm disk}^{\rm weak}=[(size,mtime,header\ validity)].
\]

If consolidation requires exact content correctness, weak identity is insufficient. A stronger class

\[
[x]_{\rm disk}^{\rm strong}=[(size,mtime,fingerprint)]
\]

would require header or sidecar persistence of a full content fingerprint. Under the current owned code, sampled fingerprints for large files are available as a helper but are not an exact current equivalence class, so no exactness claim is allowed there. fileciteturn13file14 fileciteturn15file2

For in-memory bases, exactness is the `(csv_path, SourceIdentity)` class. For derived datasets, exactness is the `(parent_data_id,timeframe,base_version)` class plus the parent-version recheck before insertion. Those are the exact coherence classes that survive attack. fileciteturn16file9 fileciteturn14file3

For Polygon refresh, exactness requires the validated-clean class: clean integrity counters, integrity checked at or after last update, correct outer coverage, correct same-day-tail handling, and boundary timestamps valid. Without that gate, hit or append classification is heuristic only. `repair_cached_dataset` and `validate_cached_dataset` are therefore not optional if the consolidation wants exact refresh semantics. fileciteturn15file5 fileciteturn16file19 fileciteturn16file7

For execution, exactness is the class

\[
[x]_{\rm exec}=
[(m^{\rm mode},d,\tilde q,c_{\rm obs},\ell,\rho)].
\]

Nothing stronger is identifiable without portfolio budget, exposure, or slippage inputs. In particular, endogenous portfolio sizing is out of reach of the current execution interface.

For the runner, exactness is the class of latest-only coalesced event traces, not full bar sequences. Any claim beyond that must explicitly switch to the Lipschitz approximation contract.

## 6. Error, regret, or bias bounds for every non-exact claim

There are only four non-exact claim families that survive the second pass.

**A. Weak disk identity as a proxy for content identity.**  
No finite deterministic false-hit bound exists under arbitrary adversarial source edits that preserve file size and mtime. That is not a missing proof; it is an impossibility result. Therefore the only admissible contract is:

- weak identity is exact only on the weak equivalence class,
- if content-correctness is required, weak identity must be rejected,
- no content-correctness bias bound is valid unless a stronger identity surface is introduced.

**B. Retention value ranking.**  
Under the windowed stationarity assumptions of Theorem 12, the score error satisfies

\[
|K_r-\hat K_r|\le \epsilon_r
\]

with the explicit \(\epsilon_r\) given there. The pairwise rank-instability contract is:

\[
\mathrm{unstable}(r,s)
\iff
|\hat K_r-\hat K_s|\le \epsilon_r+\epsilon_s.
\]

Only certified pairs may be treated as statistically ordered. All uncertified pairs must be interpreted as heuristic ties, and any final ordering among them is explicit bias, not inferred value. Under arbitrary adversarial traces, no finite regret bound exists because the problem contains online paging.

**C. Prefetch admission.**  
Under the bounded paired-wall-time assumptions of Theorem 13, the false-positive contract is:

- enable prefetch only if the lower confidence bound on savings exceeds \(\eta_{\rm wall}\),
- disable prefetch if the upper confidence bound on fatal-stage probability exceeds \(p_{\max}\).

The failure probability of a mistaken positive decision is then controlled by the chosen \(\delta\) and union bound. Outside that sampling model, no exact claim is allowed.

**D. Latest-only runner as a proxy for per-bar replay.**  
The only admissible bound is conditional:

\[
|g(C^{full})-g(C^{coal})|
\le
L_g d_{\mathcal C}(C^{full},C^{coal})
\]

for strategies with certified \(L_g\). If a strategy does not provide that certificate, then no replay-approximation bound exists and the runner must be treated only as exact latest-only semantics.

Those four contracts exhaust the non-exact claims. Nothing else should be sold as more exact than it is.

## 7. Time, memory, and retained-state complexity

`data_cache.rs` has \(O(1)\) metadata work for identity and header checks, \(O(n)\) read cost on cache hits because the candle payload must still be loaded, and \(O(n)\) parse-plus-write cost on cache misses, where \(n\) is candle length. The current weak-identity check adds no extra content-scan cost beyond metadata. fileciteturn13file14 fileciteturn15file2

`state.rs` base lookup currently scans loaded context for a matching `(csv_path, SourceIdentity)`, so base-reuse discovery is \(O(N_{ctx})\) in the number of loaded context entries. Derived reuse is \(O(1)\) expected map lookup by `base_id@timeframe`, followed by \(O(n_{base})\) resampling on misses. Retention overflow is more expensive: `eviction_candidate_locked` sorts candidate roots, so one candidate pass is \(O(K\log K)\), but `remove_dataset_tree_locked` calls `descendant_ids_locked`, which recursively scans the map for children and can be \(O(N^2)\) in a degenerate chain. `upsert_live_bar` also pays that descendant-discovery cost plus the possible `Arc::make_mut` copy cost on shared candles. So the true worst-case retained-state management cost is tree-shaped, not root-local. fileciteturn15file3 fileciteturn15file0 fileciteturn14file16

Memory complexity in `state.rs` is \(O(\sum_{v\in\mathcal G_t}|v|)\) over active base and derived candles, plus \(O(N_{ctx})\) for loaded contexts. The exact retention budgets for base and derived counts are enforced separately, but subtree size still matters because eviction removes whole trees. fileciteturn15file18

`polygon.rs` current hit or append planning is \(O(1)\) on manifest metadata, but validation or repair is \(O(n)\) in dataset rows, with an explicit repair hard cap of 2,000,000 rows before forcing a delete-and-redownload recommendation. Repair may also make bounded per-day network requests for gappy days. fileciteturn16file19 fileciteturn16file7

`commands_execution.rs` one-shot translation is \(O(1)\) local computation plus external API cost for live position fetch and order submission. Runner state is \(O(1)\): one active runner slot, one pending latest bar, and scalar counters. Coalescing is therefore constant-memory but can trade away exact per-bar replay. fileciteturn15file6 fileciteturn15file4

The profiler and campaign harnesses add measurement complexity only in the number of trials. The prefetch profiler records `stage_attempts`, stage outcome counts, mean first or second or pair wall times, and per-iteration summaries, with measurement loops controlled by warmup and measured iterations. `ta_campaign_perf.rs` similarly records preload and route artifacts for campaign runs. fileciteturn16file4 fileciteturn14file8

## 8. File and symbol impact map

The mathematical result touches four owned files directly and three context files as verification interfaces.

In `src-tauri/src/data_cache.rs`, the impacted symbols are `SourceIdentity`, `identity_for_source`, `content_fingerprint_for_source`, `load_or_build_with_identity`, `try_load_cache`, and the cache header read or write helpers. The third-pass result says this file owns the choice of weak versus stronger disk equivalence class, and that no content-correctness claim is admissible while weak identity remains the runtime validator. fileciteturn13file17 fileciteturn15file2

In `src-tauri/src/state.rs`, the impacted symbols are `find_matching_loaded_base`, `load_price_data`, `resample_loaded_data`, `upsert_live_bar`, `remove_dataset_tree_locked`, `expanded_protect_ids_locked`, `eviction_candidate_locked`, and `enforce_dataset_retention_locked`. This file owns exact coherence, exact invalidation, and the redesign surface for any subtree-aware retention score with confidence radii. fileciteturn16file9 fileciteturn15file3 fileciteturn15file0

In `src-tauri/src/polygon.rs`, the impacted symbols are `plan_download_cache_strategy`, `integrity_after_download`, `validate_cached_dataset`, and `repair_cached_dataset`. This file owns the exactness gate for historical cache hits or appends. The third-pass result says that `FullHit` and `AppendOnly` may be exact only after clean integrity is known to be current. fileciteturn15file5 fileciteturn16file19

In `src-tauri/src/commands_execution.rs`, the impacted symbols are `normalize_execute_request`, `execute_strategy_signal_blocking`, `alpaca_order_payload`, `update_pending_bar`, `on_live_execution_bar`, `start_live_execution_runner`, and `live_execution_runner_status`. This file owns the exact quantized target law, the exact formatting distortion bounds, the bracket representability surface, and the runner coalescing theorem. fileciteturn13file6 fileciteturn15file4 fileciteturn12file0

The context files remain interfaces only. `src-tauri/src/commands_campaign.rs` exposes the prefetch request or feedback hooks. `src-tauri/src/bin/ta_campaign_perf.rs` is a preload and route verification harness. `crates/app_core/src/bin/profile_doublema_campaign_prefetch.rs` is the paired timing profiler that records stage outcomes and rejects runs that leave the exact `GpuKernel` route. None of those files give permission to redesign route-selection math or public metric surfaces. fileciteturn14file0 fileciteturn14file8 fileciteturn16file18

## 9. Verification obligations and falsifiers

The exact subtheorems require exact tests.

For disk-cache semantics, a same-size same-mtime different-content counterexample must remain a falsifier of any content-correctness claim while weak identity remains in force. If consolidation upgrades to stronger identity, that test must fail to hit the cache. For base or derived reuse, tests must continue to show same-id reuse on matching base identity, exact derived reuse on unchanged base version, and forced stale-tree removal when base version changes. `state.rs` already contains tests for reuse until base changes and invalidation after live mutation. fileciteturn16file11 fileciteturn16file12

For live mutation, verification must show that `upsert_live_bar` rejects derived targets and relative timelines, mutates base epoch datasets correctly, replaces the base `Arc` when needed, and invalidates active descendants. That is an exact theorem obligation, not a heuristic one. fileciteturn14file15 fileciteturn16file12

For execution, verification must include: normalization to \(10^{-4}\), hold dead-zone at \(10^{-6}\), live-vs-dry-run separation on current position, bracket attachment only on opening-from-flat with both bounds present, and price or quantity formatting distortions at their expected decimal grids. Thin or volatile edge cases belong here because they are exactly where the dead-zone and bracket discontinuities appear. fileciteturn13file6 fileciteturn16file1 fileciteturn16file8

For runner semantics, the falsifier is the three-bar inflight construction. If the consolidation still wants to call the runner “exact per-bar replay,” that construction kills the claim immediately. The only exact remaining statement is latest-only coalescing. fileciteturn15file4 fileciteturn12file0

For Polygon planning, the falsifier is any dataset with clean outer coverage but dirty or stale interior integrity. If the planner can still return `FullHit` without validation or repair, the exactness theorem fails. Validation or repair tests with parse errors, duplicates, out-of-order rows, and large same-day gaps are the right obligation. fileciteturn16file15 fileciteturn16file19

For the non-exact retention score, the verification obligation is rank certification on replay: log subtree mass, reuse outcomes, and rebuild times; compute \(\hat K_r\) and \(\epsilon_r\); verify that deployed evictions only lean on score gaps larger than confidence radii, or else explicitly tag them as heuristic ties. If most decisions are uncertified, the score is not earning the right to replace the current heuristic. The falsifier is high rank-instability on held-out traces.

For prefetch, the verification obligation is lower-confidence-bound positivity on saved wall time and upper-confidence-bound control on fatal stage risk. The existing prefetch profiler and campaign perf harness are already the right interfaces because they record stage outcome categories and mean wall times. The falsifiers are negative lower confidence bound on savings, stage fatal-rate upper bound above threshold, or route drift off the exact `GpuKernel` surface. fileciteturn16file4 fileciteturn16file18 fileciteturn14file8

## 10. Dependencies that must be reconciled during consolidation

The consolidation has four hard interface dependencies.

First, `PR-08` still depends on `commands_campaign.rs`, `ta_campaign_perf.rs`, and `profile_doublema_campaign_prefetch.rs` only as measurement and staging interfaces. Any attempt to use those files to redesign route selection, backend choice, or output semantics crosses into other prompts and must be rejected. fileciteturn14file0 fileciteturn14file8 fileciteturn16file18

Second, the live execution theorem depends on the external Alpaca interface and on the single active Polygon live-stream slot. If another prompt changes broker semantics, adds endogenous portfolio-sizing inputs, or turns the runner into a multi-stream coordinator, then this theorem’s exact state description must be reopened. The current repo map is explicit that live execution targets delta-to-current-position, depends on an active Polygon live stream, and allows only one active runner slot. fileciteturn15file19

Third, the Polygon exactness theorem depends on the availability of integrity metadata and validation or repair surfaces inside `polygon.rs`. If consolidation strips integrity freshness from the hit decision, the third-pass exactness theorem fails and only a heuristic remains. fileciteturn16file19

Fourth, the prompt’s non-goals remain binding: no redesign of metric-surface meaning, route-selection math, or options-specific GPU runtime economics. Any consolidation attempt that pushes `PR-08` into those surfaces should be rejected on scope grounds. fileciteturn15file10

## 11. Kill criteria

The third-pass result comes with explicit kill criteria.

Kill the **disk-cache correctness claim** immediately if stakeholders require content-correct reuse while retaining weak `(size,mtime)` identity as the runtime validator. There is no admissible bound there.

Kill the **Polygon exact-hit claim** immediately if `FullHit` or `AppendOnly` can be returned while integrity is dirty, missing, or stale relative to last update. At that point the theorem collapses back to a heuristic.

Kill the **retention-ranker replacement** if replay shows that most top-vs-runner-up score gaps are within confidence radii, or if the request stream is so nonstationary that the empirical score has no stable calibration window. In that case the ranker has not earned a statistical ordering.

Kill the **prefetch policy** whenever the lower confidence bound on saved wall time is nonpositive, the upper confidence bound on fatal-stage risk exceeds threshold, or the profiler can no longer stay on the exact `GpuKernel` route required by its own harness.

Kill the **runner replay approximation claim** unless the relevant live execution strategy ships a certified Lipschitz sensitivity bound. Without that, latest-only is exact as latest-only, and nothing more.

Kill the **endogenous sizing extension** unless new budget, exposure, or slippage inputs are added to the owned interface. The current execution surface identifies only quantized delta-to-target translation from an exogenous qty, not portfolio optimization.

The third-pass conclusion is:

> The strongest implementation-ready result for `PR-08` is a two-layer theorem: an exact semantics layer for cache coherence, invalidation, observable refresh admissibility, quantized live execution, bracket representability, and latest-only runner control; and a bounded estimation layer for retention ranking and prefetch admission. No stronger global optimality claim survives the second pass. The only admissible upgrades are those that either restore exactness by strengthening the observable state, or else carry explicit confidence, bias, or rank-instability contracts. fileciteturn16file9 fileciteturn16file8 fileciteturn15file5 fileciteturn15file4

