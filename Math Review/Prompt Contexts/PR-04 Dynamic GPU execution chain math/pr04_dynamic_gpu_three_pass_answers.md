# PR-04 Dynamic GPU execution chain math — first, second, and third pass

Source prompt file: `PROMPT.md`. fileciteturn13file0

This document packages the three-pass answer set for `PR-04`.

## Prompt scope block

```md
## `PR-04` Dynamic GPU execution chain math

Owned problem:
the full dynamic-GPU path from representable algebra through executable caching
to residency and uploaded-batch reuse

Source packets:
`MR-05`, `MR-06`

Primary ownership files:

- `crates/app_core/src/strategy_ir.rs`
- `crates/app_core/src/strategy_gpu_program.rs`
- `crates/app_core/src/strategy_gpu_runtime.rs`
- `crates/app_core/src/strategy_dynamic.rs`
- `crates/app_core/src/dynamic_indicator_adapter.rs`

Primary ownership symbols in shared files:

- in `crates/app_core/src/double_ma.rs`:
  `prepare_dynamic_gpu_execution_selection`,
  `dynamic_gpu_preflight_required_for_backend`,
  `dynamic_gpu_batch_cache_key`,
  `get_or_build_dynamic_gpu_uploaded_batch`,
  `build_gpu_residency_summary`

Context files:

- `crates/app_core/src/gpu_backtest_kernel.rs`
- `crates/app_core/src/strategy_plan.rs`
- `src-tauri/src/commands_dynamic.rs`
- `crates/app_core/src/bin/profile_gpu_dynamic_runner.rs`

Own these symbols or decisions:

- representable dynamic strategy algebra
- executable ABI and fingerprint semantics
- executable-cache key sufficiency
- resident CUDA dataset ownership
- prepared-input caching and uploaded-batch reuse
- dynamic path residency and preflight economics

Open problem:

- characterize the expressive power and minimal state of the dynamic GPU
  execution chain under executable-cache, ABI, and uploaded-batch constraints

Why this is hard here:

- algebra, executable identity, uploaded-batch reuse, and residency economics
  are one coupled chain rather than separable optimizations
- every extra expressive feature potentially widens ABI state, cache keys, and
  resident-state obligations

Mathematical object:

- a discrete executable language with a compilation map, an execution-state
  cache, and a reuse relation over uploaded datasets and compiled programs

Notation seed:

- let `a \in \mathcal{A}` denote a dynamic strategy expression
- let `C(a) = e` be the compilation map into an executable object `e`
- let `D` denote the uploaded dataset/resident input state
- let `K(e, D)` be the cache identity used for exact executable/data reuse
- exact reuse requires that
  `(e_1, D_1) \equiv (e_2, D_2) \Rightarrow K(e_1, D_1) = K(e_2, D_2)`,
  while non-equivalent pairs must not alias

Primary questions:

- what is the minimal state needed to preserve exact executable reuse without
  aliasing?
- which algebra extensions are representable without forcing unbounded cache-key
  growth?
- when is exact uploaded-batch reuse impossible without stronger immutability or
  identity assumptions?

Required deliverables:

- formal executable-state model
- candidate minimal-state or cache-key sufficiency theorem
- aliasing counterexamples
- complexity and retained-state width implications

Acceptance bar:

- every reuse claim must either prove non-aliasing or state exact conditions
  under which aliasing can occur

Verification hooks:

- cache-key collision tests
- resident-state reuse parity
- executable identity falsifiers

Non-goals:

- do not redesign static route heuristics; that belongs to `PR-03`
- do not redesign runtime Tauri cache or execution economics; that belongs to
  `PR-08`

Files to provide as context:

- all primary ownership files
- all context files

Prompt-specific addenda:

- explicitly separate IR schema, compiled plan, executable fingerprint, and
  uploaded-batch identity
- cache aliasing risk is a first-class kill criterion
```

---

## First pass

### 1. Formal problem statement

Let `a ∈ 𝒜` be a validated dynamic strategy IR. In the shipped repo this means a finite typed strategy object with profile metadata, MA source, fast/slow MA specs, a signal graph of indicator nodes, condition nodes, condition groups, and signal bindings, plus execution policy. The operand alphabet already includes indicator references, `MaSource`, `Close`, and exact `ConstantBits` payloads. The ambient IR validator allows up to 64 indicators, 256 conditions, 64 groups, and 64 group-condition references. fileciteturn23file2 fileciteturn23file0

The execution chain is not one map. It is a layered path:
- IR `a`
- compiled plan `p`
- compiled GPU predicate program / ABI `g`
- runtime executable identity `e`
- prepared CUDA resident input state `u`
- uploaded dynamic batch state `b`

The prompt asks for the minimal retained state that preserves exact reuse without aliasing across that whole chain.

### 2. Notation

Let the full request state be

\[
x = (a,r,d,\gamma,c)
\]

where:
- `a` is the dynamic strategy IR,
- `r` is the route / backend-resolution envelope,
- `d` is the candle dataset,
- `γ` is uploaded-batch geometry and MA staging shape,
- `c = (device, thread_namespace)` is the CUDA residency context.

Define the structural maps

\[
P(a,r)=p,\qquad G(p)=g,\qquad E(g,device)=e,
\]
\[
U_{\mathrm{prep}}(d,p,c)=u,\qquad U_{\mathrm{batch}}(d,e,\gamma,c)=b.
\]

Define the structural state space

\[
\Omega = \Omega_{\mathrm{ir}}\times \Omega_{\mathrm{plan}}\times \Omega_{\mathrm{exec}}
\times \Omega_{\mathrm{prep}}\times \Omega_{\mathrm{batch}}.
\]

A candidate cache-summary is any map

\[
K: \mathcal X \to \mathcal Z.
\]

The kill criterion is alias freedom:
if two inputs are not execution-equivalent at a given layer, their summary must not collide at that layer.

### 3. Fixed invariants and interface constraints

The repo already hard-separates IR identity, compiled-plan identity, and executable identity.

`CompiledStrategyPlan::ir_fingerprint_hex` is explicitly IR-level identity only; downstream GPU layers derive a compiled-plan fingerprint and then an executable fingerprint. There is also a planner test showing route overrides can preserve the IR fingerprint while changing both compiled-plan and executable fingerprints. fileciteturn20file6 fileciteturn20file13 fileciteturn21file2

The compiled-plan layer lowers value references to the finite operand alphabet
`IndicatorIndex | MaSource | Close | ConstantBits`. The GPU program layer carries the same separation into the ABI and executable fingerprints. fileciteturn20file6 fileciteturn21file2 fileciteturn21file9

The runtime layer then keys:
- compiled bundles by compiled-plan fingerprint + program schema version + ABI version,
- executables by executable fingerprint + device id + ABI version,
- uploaded executables in a thread-local cache keyed by the executable key. fileciteturn21file1 fileciteturn20file8 fileciteturn20file5

Prepared CUDA indicator inputs are intentionally sharper than semantic dataset identity: the docs say reuse is keyed by dataset pointer identity and length, not a broader mutation/revision contract. fileciteturn20file1

The current dynamic preview surface is broader than the narrower runnable GPU-kernel contract:
preview caps are 12 indicators, 24 conditions, 8 groups, 8 group-condition refs, bidirectional preview requires `short_entry_group`, and preview only allows floating scalar outputs; the narrower GPU-handle contract additionally requires `GpuUnified`, `DoubleMa`, no halftrend filter, exit mode `Cross`, exactly two indicator slots, and only indicator/constant operands in the GPU ABI. fileciteturn20file2 fileciteturn20file17

### 4. Strongest plausible positive result

A clean structural sufficient statistic exists at the level of canonical structural states, not at the level of the repo’s current compressed cache strings.

On the exact runnable sublanguage, under:
- canonical structural comparison instead of raw fingerprint strings,
- immutable or revisioned dataset identity,
- exact MA-source-series identity,
- fixed request-envelope parity,
- exact parameter identity or an explicit semantic quantization contract,

alias-free reuse can be factored by the tuple

\[
\Sigma^*(x)=
\big(
K_{\mathrm{bundle}},
K_{\mathrm{exec}},
K_{\mathrm{prep}},
K_{\mathrm{batch}}
\big)
\]

with:

\[
K_{\mathrm{bundle}}=
([\![p]\!]_{\mathrm{plan}}, v_{\mathrm{prog}}, v_{\mathrm{abi}})
\]

\[
K_{\mathrm{exec}}=
([\![e]\!]_{\mathrm{exec}}, device)
\]

\[
K_{\mathrm{prep}}=
([\![d]\!]_{\mathrm{dataset}}, \mu(p), device, thread)
\]

\[
K_{\mathrm{batch}}=
([\![e]\!]_{\mathrm{exec}}, [\![d_{\mathrm{close}}]\!], [\![d_{\mathrm{source}}]\!], \gamma, device, thread).
\]

This is the strongest positive result because it mirrors the actual repo layering: IR, compiled plan, executable, prepared resident state, and uploaded dynamic batch are different objects. fileciteturn20file6 fileciteturn21file2 fileciteturn20file4 fileciteturn20file7

### 5. Strongest plausible impossibility result

No unconditional exact sufficient statistic exists if one insists on using the current repo cache keys themselves as the mathematical identity.

There are three independent impossibility mechanisms.

First, the fingerprint layers are 64-bit hash surrogates over an infinite admissible domain that already includes arbitrary `ConstantBits` payloads and floating MA params, so they cannot be injective on the full domain. fileciteturn23file3 fileciteturn20file6 fileciteturn21file15

Second, prepared CUDA input reuse is keyed by pointer identity and length, not semantic dataset identity, so exactness fails without immutability or revision contracts. fileciteturn20file1

Third, the current dynamic batch key records close-pointer metadata and timestamps but not the actual `ma_source` series identity when `ma_source != close`, while the batch itself is built from MA matrices of `source_type(candles, ma_source)`. fileciteturn20file19 fileciteturn20file7

### 6. Candidate theorem or lemma list

1. **IR-insufficiency lemma.**  
   `ir_fingerprint_hex` is insufficient for executable identity.

2. **Compiled-plan necessity lemma.**  
   Any exact executable key must distinguish backend/routing decisions.

3. **Executable-layer sufficiency lemma.**  
   Canonical executable structure plus device is sufficient for alias-free executable reuse eligibility.

4. **Prepared-input necessity lemma.**  
   Exact prepared-input reuse needs immutable/revisioned dataset identity and required field-family identity.

5. **Batch-source necessity lemma.**  
   Exact uploaded-batch reuse must distinguish close-series identity from MA-source-series identity.

6. **Topology lemma.**  
   Equality of a sufficient statistic is not the same as an actual cache hit because uploads are thread-local and prepared CUDA state is resident and overwriteable.

7. **Hash lower-bound theorem.**  
   No 64-bit fingerprint family can be exact on the full PR-04 domain.

### 7. Explicit counterexample family

Take `ma_source = "high"` and hold:
- executable fingerprint,
- device id,
- MA types,
- normalized params,
- period arrays,
- `first_valid`,
- candle length,
- close-pointer identity,
- first/last timestamps

fixed.

Now change only the interior `high` series while keeping `close` unchanged.

Then the current dynamic batch key is unchanged, but the uploaded fast/slow MA matrices change because they are staged from `source_type(candles, ma_source)`, not from `close`. Therefore the current batch key aliases semantically different uploaded batches. fileciteturn20file19 fileciteturn20file7

### 8. One lower-bound / impossibility angle

A pure key-only exactness theorem is impossible on the shipped implementation because actual reuse is not a pure function of the key. Runtime topology matters:
- compiled bundle cache is process-global,
- executable cache is process-global,
- uploaded executable cache is thread-local,
- prepared CUDA dataset state is resident and replaceable,
- dynamic batch cache is thread-local. fileciteturn20file4 fileciteturn20file5

So actual hits depend on cache history, not only on the semantic request state.

### 9. Unresolved questions that block implementation

1. What is the intended exact dataset identity contract: content hash, immutable object id, or revision id?
2. Are the current fingerprint strings intended as probabilistic surrogates or exact identities?
3. Is 12-decimal normalization a semantic quantization contract or only a serialization convenience?
4. Is uploaded-batch identity supposed to include semantic MA-source-series identity?
5. Is the optimization target the broader preview slice or the narrower exact-compatible kernel slice?
6. Is CPU/CUDA parity intended to be exact, sign-stable only, or tolerance-bounded?

---

## Second pass

### 1. Explicit failure modes

The first pass was too strong in one critical way: it implicitly treated all reuse relations as if they were ordinary key-value caches. That is false.

Prepared CUDA resident inputs are not an arbitrary map of datasets; they are resident device state. Equality of semantic state may make reuse permissible, but an intervening resident replacement can eliminate an actual hit. Uploaded executables are thread-local even though compiled bundles and executable metadata are process-global. So a key can characterize alias-free eligibility without characterizing an actual hit. fileciteturn20file4 fileciteturn20file5

A second failure mode is overloading `ir_fingerprint_hex`. The code explicitly warns against this, and the planner test demonstrates that route overrides can leave the IR fingerprint unchanged while changing compiled-plan and executable fingerprints. fileciteturn20file6 fileciteturn20file13

A third failure mode is assuming the runnable language is determined by IR shape alone. The exact route also depends on the request envelope: `effective_sample_lcb_10`, `extra_shortlist_objectives`, `shortlist_confidence_mode`, `shortlist_union_mode`, `shortlist_slice_mode`, `metric_contract`, and the `variant_pairs == 1` restrictions can all force rejection or exact-route requirements. fileciteturn22file3 fileciteturn22file4 fileciteturn22file0 fileciteturn22file10

A fourth failure mode is assuming that the dynamic-indicator staging signature is exact. It is not. It uses only dataset length, first/last timestamps, first/last close bits, first/last selected-source bits, and a hashed source label. Interior OHLCV changes can evade the signature while changing indicator outputs. fileciteturn21file18

### 2. Adversarial constructions or separating examples

**A. Same IR fingerprint, different executable.**  
Route override one indicator from `GpuKernel` to `Cpu`. The same IR fingerprint survives, but compiled-plan and executable fingerprints differ. fileciteturn20file13

**B. Same dynamic-indicator staging key, different output.**  
Choose an indicator that consumes `HighLow` or `Ohlcv`. Hold endpoints and source label fixed; change interior `high`, `low`, or `volume`. The staging signature is unchanged, but the output changes. fileciteturn21file18

**C. Same batch key, different uploaded batch.**  
Set `ma_source="high"`, keep close/timestamps fixed, and change interior `high`. The key is unchanged, but the staged MA matrices change. fileciteturn20file19 fileciteturn20file7

**D. Same rounded parameter key, different semantics.**  
Take two parameter values whose `"{:.12}"` strings agree but whose full-precision values differ. The current key collides unless semantics are quantized at that same precision. fileciteturn20file15 fileciteturn20file19

**E. Same semantic identity, no hit.**  
Load dataset `d1`, then another dataset `d2`, then `d1` again on the same thread/device. The third call is semantically equal to the first but not a hit if the resident slot was replaced. fileciteturn20file4 fileciteturn20file5

### 3. Corrected theorem or lemma candidates

1. **Layer-separation lemma.**  
   IR, compiled plan, executable, prepared-input state, and uploaded-batch state are distinct identity layers.

2. **Eligibility theorem, not hit theorem.**  
   At best, a sufficient statistic can characterize alias-free reuse eligibility. It cannot characterize actual hits without cache-history state.

3. **Request-envelope lemma.**  
   The exact dynamic GPU route is a subset of `𝒜 × ℛ`, not just `𝒜`.

4. **Current-key impossibility theorem.**  
   The current shipped cache keys are not exact semantic quotients.

5. **Minimality non-theorem.**  
   The repo does not prove the current or proposed layered summaries are minimal in a formal quotient sense.

### 4. Weakest assumptions under which the result survives

- **A_struct**: compare canonical structural objects, not hash strings.
- **A_dataset**: dataset identity is immutable/revisioned.
- **A_source**: uploaded-batch identity includes both close-series and MA-source-series identity.
- **A_request**: request-envelope features that constrain the exact route are fixed.
- **A_quant12**: 12-decimal normalization is the semantic contract, or params are tracked exactly.
- **A_parity**: any CPU/CUDA fallback surface used in a non-exact claim comes with an explicit parity contract. The repo currently has a sign-parity-style dynamic-indicator test, not a full exact-equality theorem. fileciteturn20file16

### 5. Distinction between exact, approximate, and heuristic claims

**Exact today**
- IR/compiled-plan/executable identity layers are separate.
- Preview caps are broader than the exact-compatible kernel contract.
- Certain request-envelope features explicitly require the exact compatible route. fileciteturn20file6 fileciteturn20file17 fileciteturn22file3

**Approximate or conditional**
- 64-bit fingerprints as identities.
- 12-decimal parameter equality.
- CPU/CUDA fallback parity.
- Pointer-based prepared CUDA identity. fileciteturn20file1 fileciteturn20file15 fileciteturn20file16

**Heuristic only**
- dynamic-indicator dataset signature,
- current batch key when `ma_source != close`,
- any hit-rate claim that ignores thread-local or resident topology. fileciteturn21file18 fileciteturn20file19

### 6. Revised time, memory, and retained-state costs

Let:
- `I` = indicator slots,
- `C` = conditions,
- `G` = groups,
- `H` = total group-condition refs,
- `L` = candle length,
- `F = |fast_periods|`,
- `S = |slow_periods|`,
- `Q` = valid MA pair count,
- `M` = metrics row width for the chosen exact surface.

Then:
- executable description width is `Θ(I + C + G + H)`;
- prepared CUDA resident memory is `Θ(L·b + L·s)` where `b ≤ 5` is the saturated base OHLCV field count and `s` is the number of distinct derived-source overrides retained for the resident dataset;
- uploaded dynamic batch memory is `Θ(L(F+S+2) + F + S + Q)`;
- output scratch is `Θ(Q·M)`. The exact value of `M` depends on the metric surface (`METRICS_COUNT`, trade metrics, or a contract-specific width). fileciteturn20file1 fileciteturn20file7 fileciteturn21file4

### 7. Narrowed strongest surviving result

The strongest surviving repo-fit result is:

> The current system supports a layered **alias-free eligibility** formalization, but not a current-key exactness theorem and not a key-only actual-hit theorem.

That is the correct narrowing after adversarial attack.

---

## Third pass

I am answering the third-pass prompt from the uploaded `PROMPT.md` and narrowing to the strongest implementation-ready result that survives the second pass. fileciteturn13file0

### 1. Final formalization

Define the request state as

\[
x=(a,r,d,\gamma,c,h)
\]

with:
- `a ∈ 𝒜`: validated dynamic strategy IR,
- `r ∈ ℛ`: route and request-envelope state,
- `d ∈ 𝒟`: candle dataset,
- `γ ∈ Γ`: uploaded-batch geometry,
- `c=(g,\tau) ∈ 𝒞`: device id and thread-local namespace,
- `h ∈ 𝒣`: runtime cache state/history.

The extra state `h` is necessary because actual hits are path-dependent: compiled bundles and executable metadata are process-global, but uploaded executables are thread-local, prepared CUDA inputs are resident state, and dynamic uploaded batches are thread-local. fileciteturn20file4 fileciteturn20file5

Define the canonical layer maps:

\[
P(a,r)=p
\]
compiled plan,

\[
G(p)=g
\]
compiled GPU predicate program plus ABI,

\[
E(g,gpu\_device)=e
\]
runtime executable object,

\[
U_{\mathrm{prep}}(d,p,c,h)=u
\]
prepared CUDA resident input state,

\[
U_{\mathrm{batch}}(d,e,\gamma,c,h)=b
\]
uploaded dynamic batch.

Define the five identity layers:

\[
[a]_{\mathrm{IR}},\quad [p]_{\mathrm{plan}},\quad [e]_{\mathrm{exec}},
\quad [u]_{\mathrm{prep}},\quad [b]_{\mathrm{batch}}.
\]

These layers must remain distinct. The code explicitly says:
- `CompiledStrategyPlan::ir_fingerprint_hex` tracks IR identity only,
- `CompiledGpuPredicateProgram.compiled_plan_fingerprint_hex` tracks compiled-plan identity,
- `CompiledGpuPredicateProgram.executable_fingerprint_hex` tracks executable/cache identity,
- runtime caches key compiled bundles and executables on those downstream fingerprints,
- uploaded-batch identity is a different problem again. fileciteturn20file6 fileciteturn21file2 fileciteturn21file1 fileciteturn20file8

The exact-compatible runnable language is not just an IR subset. It is a subset of `𝒜 × ℛ`. The preview surface allows a broader product slice, while the exact-compatible `dynamic_strategy_v1` kernel route additionally requires:
- preview caps already satisfied,
- floating scalar outputs only,
- no pattern/matrix outputs,
- backend `GpuUnified`,
- profile `DoubleMa`,
- no halftrend filter,
- exit mode `Cross`,
- exactly two indicator slots,
- indicator/constant-only ABI operands,
- request-envelope features compatible with the exact route. fileciteturn20file2 fileciteturn20file17 fileciteturn22file3

### 2. Final theorem / lemma list

#### Theorem A — Structural alias-free reuse eligibility

On the exact-compatible language `𝓛_exact ⊂ 𝒜 × ℛ`, assume:

- **A_struct**: equality is taken on canonical structural objects, not current hash strings.
- **A_dataset**: datasets have immutable or revisioned semantic identity.
- **A_source**: uploaded-batch identity tracks both close-series identity and MA-source-series identity.
- **A_request**: the exact route-determining request envelope is fixed.
- **A_quant**: MA params are tracked exactly, or semantics are quantized to the same representation used in the key.
- **A_parity**: any non-exact CPU/CUDA fallback surface used in a reuse claim comes with an explicit verified parity contract.

Then there exists a finite layered sufficient statistic for **alias-free reuse eligibility**

\[
\Sigma_{\mathrm{elig}}(x)=
\big(
K_{\mathrm{bundle}},
K_{\mathrm{exec}},
K_{\mathrm{prep}},
K_{\mathrm{batch}}
\big)
\]

where:

\[
K_{\mathrm{bundle}}=
([\![p]\!]_{\mathrm{plan}}, v_{\mathrm{program}}, v_{\mathrm{abi}})
\]

\[
K_{\mathrm{exec}}=
([\![e]\!]_{\mathrm{exec}}, g)
\]

\[
K_{\mathrm{prep}}=
([\![d]\!]_{\mathrm{dataset}}, \mu(p), \sigma_{\mathrm{source\_set}}(p), g,\tau)
\]

\[
K_{\mathrm{batch}}=
([\![e]\!]_{\mathrm{exec}}, [\![d_{\mathrm{close}}]\!], [\![d_{\mathrm{ma\_source}}]\!],
\gamma, g,\tau).
\]

If two states differ at any of these layers, treating them as reusable is unsound.

#### Theorem B — No current-key exact sufficient statistic

Under the shipped repo keys, no exact sufficient statistic exists on the full PR-04 domain.

Reason:
1. `ir_fingerprint_hex`, compiled-plan fingerprints, and executable fingerprints are 64-bit hash surrogates over an infinite admissible domain that includes arbitrary `ConstantBits` payloads and floating MA parameters. fileciteturn23file3 fileciteturn20file6
2. prepared CUDA input reuse is keyed by pointer identity and length, not semantic dataset identity. fileciteturn20file1
3. the dynamic-indicator staging signature uses only endpoint summaries and a source hash. fileciteturn21file18
4. the dynamic batch key does not identify the actual MA-source series when `ma_source != close` and normalizes params to 12 decimals. fileciteturn20file19

So exactness claims can only be made over structural equivalence classes, not over today’s serialized cache strings.

#### Theorem C — No key-only actual-hit theorem

No theorem of the form

\[
K(x_1)=K(x_2) \iff \text{cache hit}
\]

is true for the shipped runtime.

Actual hits require runtime cache-state/history `h`, because:
- compiled bundles are process-global,
- executable metadata is process-global,
- uploaded executables are thread-local,
- prepared CUDA state is resident and overwriteable,
- dynamic uploaded batches are thread-local. fileciteturn20file4 fileciteturn20file5

The strongest true statement is eligibility, not guaranteed hits.

#### Lemma D — Exact route dependence on request envelope

The exact-compatible `DynamicDoubleMa GpuKernel(dynamic_strategy_v1)` route depends not only on the IR but also on route and request features such as:
- `effective_sample_lcb_10`,
- `extra_shortlist_objectives`,
- `shortlist_confidence_mode`,
- `shortlist_union_mode`,
- `shortlist_slice_mode`,
- `metric_contract`,
- and single-slice `variant_pairs == 1` constraints. fileciteturn22file3 fileciteturn22file4 fileciteturn22file0 fileciteturn22file10

#### Lemma E — Admissible algebra-extension criterion

An algebra extension is representable without forcing unbounded retained-state width iff it preserves all three properties:
1. bounded executable description width,
2. finitely generated prepared-input field/source obligations,
3. finitely described uploaded-batch geometry.

The current GPU-v1 slice satisfies this only for floating scalar outputs with `rows == 1`, no pattern ids, and the narrower kernel-handle restrictions above. fileciteturn20file17

### 3. Proof sketch / proof plan

#### Proof sketch for Theorem A

1. **Separate identity layers.**  
   The code documents and tests that IR identity, compiled-plan identity, and executable identity differ. Route overrides preserve `ir_fingerprint_hex` while changing compiled-plan and executable fingerprints. fileciteturn20file6 fileciteturn20file13

2. **Show executable eligibility factors by compiled structure.**  
   Runtime compiled-bundle caching is keyed by compiled-plan fingerprint plus schema/ABI versions, and executable caching is keyed by executable fingerprint plus device id plus ABI version. Thus backend/routing decisions and ABI versioning are necessary parts of alias-free executable eligibility. fileciteturn21file1 fileciteturn20file8

3. **Show prepared-input eligibility requires dataset and field/source obligations.**  
   Prepared CUDA inputs materialize base OHLCV fields according to required input kinds and may also upload derived source overrides. Therefore prepared-input identity depends on more than executable identity. fileciteturn20file1

4. **Show batch eligibility requires both close and MA-source identities.**  
   Dynamic batches are built from close-derived log returns and MA matrices built from `source_type(candles, ma_source)`. Therefore exact batch identity must track both close-series and MA-source-series identity. fileciteturn20file7

5. **Conclude layered sufficiency for eligibility.**  
   Under the structural assumptions, if all four keys match, reuse is alias-free at each layer.

#### Proof sketch for Theorem B

1. The admissible domain is infinite because IR supports raw `ConstantBits` and floating params. fileciteturn23file3
2. 64-bit hashes cannot be injective on an infinite domain.
3. Pointer-based dataset identity and endpoint-only staging signatures are not semantic identifiers.
4. The batch key omits MA-source series identity when `ma_source != close`.
5. Therefore the shipped keys are not exact sufficient statistics.

#### Proof sketch for Theorem C

1. Thread-local executable uploads and thread-local batch caches are separate from process-global compiled/executable metadata.
2. Prepared CUDA resident data is overwriteable.
3. Therefore equal semantic identity does not imply a hit without runtime history.

### 4. Sufficient statistics, or proof they do not exist

#### Exact sufficient statistics that do exist — but only structurally

A structural sufficient statistic exists for alias-free reuse eligibility under Theorem A:

- `K_bundle`
- `K_exec`
- `K_prep`
- `K_batch`

This is the strongest exact result.

#### Exact sufficient statistics that do not exist under shipped keys

No exact sufficient statistic exists if the statistic must be computed only from:
- current fingerprint strings,
- current prepared-CUDA pointer signatures,
- current dynamic-indicator endpoint signatures,
- current batch key serialization.

So the implementation-ready conclusion is:

> Preserve the existing layer split, but do not state or rely on any theorem that upgrades today’s serialized cache strings into exact semantic identities.

### 5. Exactness conditions and equivalence classes

Define the equivalence relations.

#### IR equivalence

\[
a_1 \sim_{\mathrm{IR}} a_2
\]

iff their canonical IR objects are equal modulo serialization details.

This is **not** the same as equality of `ir_fingerprint_hex` in an adversarial sense.

#### Compiled-plan equivalence

\[
p_1 \sim_{\mathrm{plan}} p_2
\]

iff backend choice, indicator routing, lowered operands, compiled groups/signals, and execution-policy payloads all match canonically.

This is the layer represented by `compiled_plan_fingerprint_hex` in the shipped GPU program stack. fileciteturn21file2

#### Executable equivalence

\[
e_1 \sim_{\mathrm{exec}} e_2
\]

iff executable structure, compiled-plan identity, ABI version, and schema version match canonically. Device id belongs to executable cache eligibility but not to source-language meaning. fileciteturn21file1

#### Prepared-input equivalence

\[
u_1 \sim_{\mathrm{prep}} u_2
\]

iff the semantic dataset identity, required base field family, derived source family, device, and thread namespace match.

This is stronger than the shipped pointer contract. fileciteturn20file1

#### Uploaded-batch equivalence

\[
b_1 \sim_{\mathrm{batch}} b_2
\]

iff:
- executable identity matches,
- close-series identity matches,
- MA-source-series identity matches,
- exact parameter identity or semantic quantized identity matches,
- period arrays and valid-pair geometry match,
- `first_valid`, device, and thread namespace match.

This is stronger than the shipped batch key whenever `ma_source != close` or params are compared only after `"{:.12}"` normalization. fileciteturn20file19

### 6. Error, regret, or bias bounds for every non-exact claim

#### 6.1 Hash-surrogate contract

If a 64-bit fingerprint is treated as a random digest and `N` distinct objects are compared, the birthday bound gives

\[
\Pr[\text{at least one collision}] \le \frac{N(N-1)}{2^{65}}.
\]

This is a probabilistic approximation contract only. It is not adversarially robust, and the implementation uses `DefaultHasher`, so this should be treated as a heuristic collision model, not a proof-grade exactness claim. fileciteturn20file6 fileciteturn21file15

#### 6.2 12-decimal parameter-normalization contract

If semantics are explicitly quantized to the same 12-decimal normalized representation used by the key, then parameter-key equality is exact.

If semantics are **not** quantized that way, there is no uniform finite bound on metric error or rank regret from key collisions alone, because arbitrarily close MA parameters can still induce different staged matrices and winner orderings on adversarial datasets. Therefore the only honest non-exact contract is:

- **approximation contract**: exact only after semantic quantization to the same normalized parameter space;
- **otherwise**: no general finite regret bound. fileciteturn20file15 fileciteturn20file19

#### 6.3 Prepared-CUDA pointer-identity contract

Current prepared-CUDA reuse is exact only under an external immutability/revision contract:
- candle arrays are immutable while resident,
- pointer reuse does not occur for semantically distinct live datasets in the reuse window.

Without that, no general error bound exists; aliasing can be arbitrarily bad. fileciteturn20file1

#### 6.4 Dynamic-indicator staging-signature contract

The current dynamic-indicator dataset signature is heuristic only. Because it ignores interior series content, there is no nontrivial universal error bound on indicator-output drift, metric bias, or rank regret under signature collisions. A single interior OHLCV perturbation can materially change indicator output while preserving the signature. fileciteturn21file18

#### 6.5 CPU/CUDA fallback parity contract

The repo contains a dynamic-indicator parity test that mostly checks sign agreement after a tolerance gate, not exact output equality. fileciteturn20file16

So any non-exact CPU/CUDA fallback claim must carry a contract of the form:

If there exists a verified bound

\[
\|f_{\mathrm{gpu}} - f_{\mathrm{cpu}}\|_{\infty} \le \varepsilon_{\mathrm{series}}
\]

and if the score functional `J` is locally Lipschitz with constant `L_J` over the compared candidate set, then

\[
|J_{\mathrm{gpu}} - J_{\mathrm{cpu}}| \le L_J \varepsilon_{\mathrm{series}}.
\]

Winner-rank stability then requires every decisive pairwise score margin to exceed `2 L_J ε_series`.

Without a verified `ε_series`, there is no defensible global score or rank bound.

### 7. Time, memory, and retained-state complexity

Let:
- `I` = number of indicator slots,
- `C` = number of compiled conditions,
- `G` = number of compiled groups,
- `H` = total group-condition indexes,
- `L` = candle length,
- `F = |fast_periods|`,
- `S = |slow_periods|`,
- `Q` = valid pair count,
- `M` = exact metric width for the chosen surface,
- `s` = number of distinct derived source overrides retained for one resident dataset.

Then:

#### Executable description width

\[
|\text{exec description}| = \Theta(I + C + G + H).
\]

This follows from the GPU program carrying indicator slots, predicates, groups, group-condition indexes, and signal bindings. fileciteturn21file2 fileciteturn21file17

#### Prepared-input resident memory

\[
|\text{prepared CUDA resident state}| = \Theta(L \cdot b + L \cdot s),
\]

where `b ≤ 5` is the number of materialized base OHLCV fields after field-mask saturation and `s` is the number of distinct derived source overrides retained for the resident dataset. This is bounded per resident dataset/device/thread, but not by the base field mask alone. fileciteturn20file1

#### Uploaded-batch memory

\[
|\text{uploaded batch}| = \Theta(L(F+S+2) + F + S + Q).
\]

The `+2` accounts for close-derived `lr` / `ret1`-style time-axis vectors, while the rest comes from fast/slow row-major matrices, period vectors, and pair maps. fileciteturn21file4 fileciteturn21file5

#### Output / scratch width

\[
|\text{metric scratch / output}| = \Theta(Q \cdot M).
\]

`M` depends on the chosen exact metric surface and contract. fileciteturn21file4

#### Retained-state topology

Retained state is not a single abstract cache. It is:
- process-global compiled bundle entries,
- process-global executable entries,
- current-thread uploaded executable entries,
- current-thread uploaded dynamic-batch entries,
- resident prepared-CUDA dataset state per device/thread. fileciteturn20file4 fileciteturn20file5

### 8. File and symbol impact map

#### Primary ownership

- **`strategy_ir.rs`**  
  Owns the ambient dynamic strategy algebra, `ValueRef::ConstantBits`, and IR caps. fileciteturn23file2 fileciteturn23file3

- **`strategy_plan.rs`**  
  Owns compiled-plan identity, lowering of `ValueRef`, backend/routing effects, and the explicit warning that `ir_fingerprint_hex` is IR-only. fileciteturn20file6

- **`strategy_gpu_program.rs`**  
  Owns compiled GPU program identity, compiled-plan fingerprint semantics, executable fingerprint semantics, and ABI encoding. fileciteturn21file2 fileciteturn21file3

- **`strategy_gpu_runtime.rs`**  
  Owns compiled-bundle keys, executable keys, and the topology split between process-global caches and thread-local uploads. fileciteturn21file1 fileciteturn20file4

- **`dynamic_indicator_adapter.rs`**  
  Owns prepared-CUDA dataset identity and the pointer-based prepared-input contract. fileciteturn20file1

- **`strategy_dynamic.rs`**  
  Owns preview-vs-runnable language separation, dynamic-indicator staging heuristics, and dynamic CPU evaluation/parity surfaces. fileciteturn20file2 fileciteturn20file17 fileciteturn21file18

#### Shared owned symbols in `double_ma.rs`

- `prepare_dynamic_gpu_execution_selection`
- `dynamic_gpu_preflight_required_for_backend`
- `dynamic_gpu_batch_cache_key`
- `get_or_build_dynamic_gpu_uploaded_batch`
- `build_gpu_residency_summary`

These are precisely where exact-route eligibility, preflight identity, batch identity, and residency summaries intersect. fileciteturn20file15 fileciteturn20file19

#### Context-only constraints

- `gpu_backtest_kernel.rs`
- `strategy_plan.rs`
- `commands_dynamic.rs`
- `profile_gpu_dynamic_runner.rs`

`gpu_backtest_kernel.rs` is especially important as a constraint because it fixes batch geometry validation and output-width surfaces. fileciteturn21file4

### 9. Verification obligations and falsifiers

1. **Executable identity falsifier**  
   A route-override test must keep `ir_fingerprint_hex` fixed while changing compiled-plan and executable fingerprints. This already exists conceptually in the planner tests. fileciteturn20file13

2. **Prepared-input alias falsifier**  
   Two datasets with the same pointer-style or endpoint-style signature but different interior OHLCV values must produce different indicator outputs for an affected indicator.

3. **Batch alias falsifier**  
   With `ma_source != close`, hold close/timestamps/key fields fixed and change the source series. Any implementation that still claims exact batch reuse is invalid.

4. **Rounded-param falsifier**  
   Two distinct params that normalize to the same 12-decimal string must be demonstrated to differ semantically unless quantization is the contract.

5. **Hit-vs-eligibility falsifier**  
   Same semantic request repeated after an intervening resident replacement must show eligibility without guaranteed hit.

6. **Exact-route falsifier**  
   Requests using `effective_sample_lcb_10`, shortlist confidence/union/slice, or exact metric-contract surfaces on non-exact dynamic routes must reject or fall back exactly as documented. fileciteturn22file3 fileciteturn22file10

7. **Parity falsifier**  
   Any non-exact CPU/CUDA fallback claim must fail if no explicit tolerance/bias contract is provided. The current sign-parity test is not enough to claim exactness. fileciteturn20file16

### 10. Dependencies that must be reconciled during consolidation

1. **Planner / registry normalization**  
   Any exact theorem depends on canonical indicator-id, output-ref, and param normalization from the IR/planner interface.

2. **Exact-route product contract**  
   Request-envelope restrictions in `double_ma.rs` are part of PR-04’s mathematical domain, not optional policy noise. They determine `ℛ_exact`. fileciteturn22file3

3. **GPU-kernel batch geometry**  
   Batch identity and memory complexity must stay compatible with `gpu_backtest_kernel.rs` upload validation and metric-width rules. fileciteturn21file4

4. **Numerical parity surface**  
   Any non-exact dynamic CPU fallback or CUDA-host staging claim needs an explicit parity contract. The current repo gives only partial evidence. fileciteturn20file16

### 11. Kill criteria

Any consolidation should be rejected if it does any of the following:

1. Equates `ir_fingerprint_hex` with compiled-plan or executable identity.  
2. Claims an exact current-key sufficient statistic using today’s serialized fingerprint and batch keys.  
3. Claims exact uploaded-batch reuse while omitting MA-source-series identity when `ma_source != close`.  
4. Claims exact prepared-CUDA reuse without an immutable/revisioned dataset identity contract.  
5. Claims actual cache-hit equivalence from key equality while ignoring thread-local and resident topology.  
6. Treats the broader preview caps as if they were the runnable exact-compatible kernel language.  
7. Uses 12-decimal param normalization in an exact theorem without a matching semantic quantization contract.  
8. Makes any non-exact CPU/CUDA or fallback reuse claim without an explicit error, regret, bias, or rank-instability contract.

## Final third-pass conclusion

The strongest implementation-ready result for PR-04 is:

> The dynamic GPU chain supports a layered **structural alias-free reuse-eligibility theorem**, but it does **not** support a current-key exactness theorem and it does **not** support a key-only actual-hit theorem.
>
> Exactness is defensible only over canonical structural equivalence classes that separately track IR, compiled plan, executable, prepared-input resident state, and uploaded-batch state. Any claim stronger than that fails on at least one shipped surface: hash surrogates, pointer-based prepared-input identity, endpoint-only dynamic-indicator staging signatures, or the current batch key’s omission of semantic MA-source-series identity.

---

## Evidence anchors used repeatedly

- `strategy_ir.rs`
  - ambient IR caps,
  - `ConstantBits`,
  - broad validator surface. fileciteturn23file2 fileciteturn23file3 fileciteturn23file0

- `strategy_plan.rs`
  - `CompiledValueRef`,
  - `ir_fingerprint_hex` IR-only warning,
  - route-dependent compiled-plan identity,
  - override test preserving IR fingerprint while changing downstream identity. fileciteturn20file6 fileciteturn20file13

- `strategy_gpu_program.rs`
  - plan / compiled-plan / executable fingerprint semantics,
  - ABI version and executable fingerprint input. fileciteturn21file2 fileciteturn21file3

- `strategy_gpu_runtime.rs`
  - compiled bundle key,
  - executable key,
  - process-global vs thread-local cache topology. fileciteturn21file1 fileciteturn20file4 fileciteturn20file5

- `dynamic_indicator_adapter.rs`
  - prepared CUDA pointer-identity contract. fileciteturn20file1

- `strategy_dynamic.rs`
  - preview-vs-runnable split,
  - heuristic dataset signature,
  - dynamic CPU metrics parity tests. fileciteturn20file2 fileciteturn20file17 fileciteturn21file18

- `double_ma.rs`
  - exact-route request-envelope restrictions,
  - dynamic preflight key,
  - dynamic batch key,
  - batch construction from `close` plus `ma_source`. fileciteturn22file3 fileciteturn22file0 fileciteturn20file15 fileciteturn20file19 fileciteturn20file7