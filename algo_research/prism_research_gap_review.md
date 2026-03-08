# PRISM-SIG-HVCOST: gap re-check, publishability checklist, and next-step contributions

## What the literature already covers

1. GPU/tensorized EMO exists.
   - Tensorized EMO / TensorNSGA-III already covers GPU-native NSGA-III, MOEA/D, and HypE style acceleration.
   - Generic claim "we accelerated a MOEA on GPU" is therefore not novel by itself.

2. Gray-box partial-evaluation optimization exists.
   - RV-GOMEA / MO-RV-GOMEA and follow-up work already exploit linkage models and partial evaluations.
   - GPU-accelerated parallel GOM in gray-box settings also exists.

3. Conformal surrogates for optimization exist.
   - Conformal quantile regression has already been used for HPO and multi-fidelity aggregation.

4. Cost-aware / partial-information multi-objective optimization exists.
   - Hypervolume Knowledge Gradient handles decoupled objectives with resource differences and multi-fidelity proxies.

5. Mixed-space local optimization exists.
   - Mixed continuous/categorical BO methods already exist.

6. Beyond-VRAM GPU scheduling exists.
   - Reordered pipelining / grouping / padding for beyond-GPU-memory execution exists in systems work.

## Where PRISM still appears differentiated

PRISM is still plausibly research-worthy if the paper claim is narrowed to:

> A conservative, signature-conditioned, gray-box multi-objective optimizer for
> exact GPU-resident backtest search on consumer hardware, with graph-stable
> batching, partial reevaluation, and beyond-VRAM streaming.

The differentiating combination is:
- exact backtests are already cheap enough that optimizer overhead matters;
- fidelities are real execution stages of the same CUDA/Rust pipeline;
- search units are strategy-signature-conditioned cells, not generic vectors;
- partial reevaluation is tied to affected tensor/kernel groups;
- scheduling explicitly optimizes hypervolume gain per millisecond;
- out-of-core execution is a first-class design constraint on a 24 GB GPU;
- Tensor Cores are used only for the dense screening layer, not the exact path.

## Strongest paper framing

### Preferred title shape
"Conservative Hypervolume-per-Cost Pareto Racing for Exact GPU-Resident Strategy Search Beyond Device Memory"

### Core claim
When exact GPU-resident backtests become very fast, the right optimization target is not sample efficiency alone, but *Pareto hypervolume gained per millisecond under batching, graph, and VRAM constraints*.

### What the algorithm is NOT
- not a generic new MOEA competing on standard ZDT/DTLZ only;
- not a Tensor-Core-accelerated exact backtester;
- not a BO method for arbitrary black-box functions;
- not a new conformal predictor in isolation.

## Minimal theorems / propositions worth proving

1. **Conservative prune safety (empirical finite-sample statement)**
   Given a split-conformal surrogate with marginal coverage 1-alpha on each objective,
   define optimistic bounds by upper confidence values for both normalized objectives.
   Show that pruning only candidates whose optimistic point is dominated by the archive's
   pessimistic frontier controls false-prune rate by a function of alpha and calibration size.

2. **Hypervolume-per-cost monotonicity surrogate lemma**
   If scheduler priority is optimistic-HV-gain / predicted-cost and the cost model is
   multiplicatively bounded within [1-eps, 1+eps], derive regret of mis-prioritization
   relative to an oracle scheduler over one batch.

3. **Graph-stable batching efficiency proposition**
   Formalize that batching by (signature, stage, changed_group_signature, shape) weakly
   dominates unclustered exact submission in host-launch count and never increases the
   number of exact evaluations.

4. **Partial reevaluation savings decomposition**
   Given module factorization G = {regime, entry, risk}, show expected saved work equals
   the proportion of proposals whose changed group excludes one or more modules times
   the cached module cost. This can be empirical if a closed-form theorem is too brittle.

## Required experiments for a convincing paper

### A. Main comparison
Compare against:
- brute-force GPU (where feasible)
- current progressive app search
- NSGA-II
- TPE
- QMC / Sobol / Halton
- PRISM without screen
- PRISM without cost model
- PRISM without partial reevaluation
- PRISM full

### B. Main plots
- Pareto hypervolume vs wall-clock
- regret to brute-force frontier vs wall-clock (small feasible spaces)
- exact eval count vs wall-clock
- screen false-prune rate vs calibration alpha
- graph replay hit rate vs batch-shape policy
- partial reuse rate vs proposal locality
- host-device bytes transferred vs baseline

### C. Ablations
- no conformal calibration
- global screen vs signature-conditioned screen
- cost-aware scheduler vs optimistic-score-only scheduler
- graph-stable batching vs irregular batching
- latent valid coordinates vs repaired invalid vectors
- with vs without low-fidelity real execution stages

### D. Stress tests
- mixed discrete-heavy signatures
- very large numeric cells
- low rank-correlation fidelities
- stage-cost skew across signatures
- screen under deliberate distribution shift

## Practical integration checklist

1. Instrument real exact runtime per:
   - strategy signature
   - candle count
   - asset count
   - changed_group_signature
   - batch shape
   - fidelity stage

2. Fit a multiplicative cost model and store calibration error.

3. Implement exact changed-group manifests:
   - regime-only
   - entry-only
   - risk-only
   - regime+entry
   - entry+risk
   - full

4. Capture CUDA graphs for each stable key.

5. Record all paper metrics in one schema:
   - hv
   - hv/ms
   - false-prune rate
   - joint interval coverage
   - exact evals saved
   - partial reuse rate
   - graph replay hit rate
   - bytes avoided

## What would invalidate the paper direction

- False-prune rate cannot be driven low enough without collapsing prune savings.
- Low-fidelity stages have poor rank correlation with full exact evaluation.
- Partial reevaluation savings are too small because most proposals touch all modules.
- Graph-stable batching fragments the workload so much that it loses throughput.
- Brute-force remains superior on all search sizes that matter in practice.

## Suggested claim hierarchy for the final paper

1. **Main contribution:** a systems-aware multi-objective optimizer for exact GPU search.
2. **Secondary contribution:** conservative conformal screening in a signature-conditioned mixed space.
3. **Systems contribution:** graph-stable batching plus partial reevaluation for VRAM-resident indicator banks.
4. **Hardware contribution:** Tensor Cores used only where algebraically appropriate (screen stage).

## Recommendation

Do not market this as "a new optimizer" only.
Market it as **optimizer + runtime co-design for exact GPU search under VRAM constraints**.
That is where the literature overlap appears weakest and where the current artifact is strongest.
