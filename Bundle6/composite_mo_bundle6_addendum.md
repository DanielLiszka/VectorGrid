# CompositeMo Bundle 6 Addendum — Missed Items, Corrections, and Optional Extensions

This addendum supplements the original `composite_mo_bundle6_implementation_plan.md`.

It focuses on items that were either:

1. missing from the first plan,
2. worth tightening because they directly affect performance/correctness,
3. or optional but high-value improvements that still preserve functionality.

The highest-value additions in this addendum are:

- fixing **generic categorical parameter decoding** end-to-end,
- rejecting **unknown or duplicate search-space parameters**,
- removing **structural waste from MACZ fast/slow ordering**,
- making evaluation accounting and cancellation semantics accurate,
- reducing memory pressure in `Random` / `Qmc` / `Tpe`,
- and making type-aware prescan also narrow the downstream **bank-build footprint**, not just evaluation eligibility.

---

## 1. What I would add or amend in the original plan

## 1.1 Critical amendment: exact-grid and shape-aware search depend on fixing categorical parameter handling first

My original plan recommended an exact-grid fast path. That recommendation is still good, but it depends on a missing prerequisite:

- the frontend currently emits some `Categorical` choices as **numbers or booleans**,
- while the Rust optimizer domain expects `Categorical { choices: Vec<String> }`,
- and the backend only correctly decodes categorical choices for MA types and MA lengths, not for the other numeric/bool params.

That means:

- some custom/range-built search spaces can fail to deserialize,
- and even if they deserialize, some categorical numeric/bool params can be interpreted as the **category index** instead of the actual value.

So the dependency order should become:

1. fix categorical representation and decoding,
2. add unknown/duplicate param rejection,
3. then enable exact-grid auto-switch with confidence.

---

## 1.2 Critical amendment: type-aware prescan should also narrow MA-bank construction by type

My original plan replaced global `allowed_pairs` with a type-aware map. That is necessary, but not sufficient.

Once prescan becomes type-aware, the downstream MA-bank build should also become type-aware.

Otherwise, the code still overbuilds banks using the **global union** of fast/slow periods across all MA-type pairs, even when each MA type only uses a smaller subset of those periods.

That means the original plan should be amended as follows:

- make prescan eligibility type-aware,
- make repair type-aware,
- and make bank construction type-aware.

---

## 1.3 Critical amendment: actual evaluation count semantics need to be corrected before dedup/cancel work lands

My original plan added dedup and telemetry, but it did not explicitly fix the meaning of `CompositeMoResult.total_evaluations` for `Random`, `Qmc`, and `Tpe`.

Right now the backend reports configured trial counts for those samplers, not actual evaluator invocations. That is acceptable today only because the current implementation mostly evaluates exactly that many points.

But once you add:

- dedup,
- exact-grid auto-switch,
- chunked frontier merging,
- or cancellation,

then `config.trials` is no longer the truth.

The source of truth should become the actual evaluation counter driven by `eval_with_progress(...)`.

---

## 2. Newly identified confirmed issues

## A1. Generic categorical numeric/bool params are not decoded correctly end-to-end

**Severity:** critical  
**Type:** correctness / performance  
**Location:**
- `crates/app_core/src/composite_mo.rs`
- `web/src/services/optimizeRequestBuilder.ts`
- `crates/optimizer/src/mo.rs`  
**Timing boundary affected:** app-path

### Why it matters

Today the backend has specialized categorical decoding for:

- MA length domains (`decode_len(...)` + `period_choice_map(...)`)
- MA type domains (`decode_ma_type(...)`)

But the rest of the parameter decoders still use:

- `get_u16(...)`
- `get_f64(...)`
- `get_bool(...)`

Those helpers interpret `ParamValue::Cat(ci)` as the **index** `ci`, not the categorical choice payload.

That is wrong for custom search-space categorical params such as:

- `halftrend_amplitude`
- `halftrend_channel_deviation`
- `macz_gamma`
- `ott_percent`
- `macz_use_lag`
- `macz_use_volume`
- `strategy_long_only`
- `strategy_allow_flip`

The frontend makes this worse because it currently emits some categorical choices as:

- numbers in `parseMoRangeFloat(...)`
- booleans in `parseMoBoolDomain(...)`

while the Rust optimizer expects `Categorical { choices: Vec<String> }`.

So there are actually **two bugs**:

1. frontend categorical shapes can be incompatible with Rust deserialization,
2. backend generic categorical decode can misread category indices as real values.

### Recommended change

#### Backend: add typed categorical decoders

Add generic helpers that inspect the domain and decode the underlying categorical choice string when the runtime `ParamValue` is `Cat(...)`.

```rust
fn categorical_choice<'a>(
    space: &'a SearchSpace,
    values: &[ParamValue],
    idx: usize,
) -> Result<Option<&'a str>, String> {
    let Some(def) = space.params.get(idx) else {
        return Err("param idx invalid".to_string());
    };

    match (&def.domain, values.get(idx)) {
        (ParamDomain::Categorical { choices }, Some(ParamValue::Cat(ci))) => {
            let Some(choice) = choices.get(*ci) else {
                return Err(format!("categorical index out of range for '{}'", def.name));
            };
            Ok(Some(choice.as_str()))
        }
        (_, Some(_)) => Ok(None),
        (_, None) => Err("param missing".to_string()),
    }
}

fn parse_f64_choice(s: &str) -> Result<f64, String> {
    s.trim()
        .parse::<f64>()
        .map_err(|_| format!("invalid float choice '{s}'"))
}

fn parse_bool_choice(s: &str) -> Result<bool, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("invalid bool choice '{s}'")),
    }
}

fn get_u16_param(space: &SearchSpace, values: &[ParamValue], idx: usize) -> Result<u16, String> {
    if let Some(choice) = categorical_choice(space, values, idx)? {
        return parse_u16_from_choice(choice);
    }
    get_u16(values, idx)
}

fn get_f64_param(space: &SearchSpace, values: &[ParamValue], idx: usize) -> Result<f64, String> {
    if let Some(choice) = categorical_choice(space, values, idx)? {
        return parse_f64_choice(choice);
    }
    get_f64(values, idx)
}

fn get_bool_param(space: &SearchSpace, values: &[ParamValue], idx: usize) -> Result<bool, String> {
    if let Some(choice) = categorical_choice(space, values, idx)? {
        return parse_bool_choice(choice);
    }
    get_bool(values, idx)
}
```

Then replace all non-length/non-MA-type parameter reads in `eval_one(...)` with these typed helpers.

Examples:

```rust
if let Some(i) = idx.halftrend_amplitude {
    if let Ok(v) = get_u16_param(&search_space, values, i) {
        halftrend.amplitude = v;
    }
}

if let Some(i) = idx.halftrend_channel_deviation {
    if let Ok(v) = get_f64_param(&search_space, values, i) {
        halftrend.channel_deviation = v;
    }
}

if let Some(i) = idx.macz_use_lag {
    if let Ok(v) = get_bool_param(&search_space, values, i) {
        macz.use_lag = v;
    }
}
```

#### Frontend: stringify categorical numeric/bool choices

Update `parseMoRangeFloat(...)` and `parseMoBoolDomain(...)` so `Categorical.choices` is always an array of strings.

```ts
function canonicalNumberString(value: number): string {
  if (!Number.isFinite(value)) throw new Error("categorical float choice must be finite");
  return Number(value.toFixed(10)).toString();
}
```

```ts
function parseMoRangeFloat(
  low: unknown,
  high: unknown,
  step: unknown,
  label: string,
): Record<string, unknown> {
  const lowNum = num(low, Number.NaN);
  const highNum = num(high, Number.NaN);
  const stepNum = num(step, Number.NaN);
  if (!Number.isFinite(lowNum) || !Number.isFinite(highNum) || !Number.isFinite(stepNum)) {
    throw new Error(`${label}: low/high/step must be finite numbers.`);
  }
  if (stepNum <= 0) throw new Error(`${label}: step must be > 0.`);
  if (lowNum > highNum) throw new Error(`${label}: low must be <= high.`);

  if (lowNum === highNum) {
    return { FloatRange: { low: lowNum, high: highNum, log: false } };
  }

  const count = Math.floor((highNum - lowNum) / stepNum) + 1;
  if (Number.isFinite(count) && count >= 2 && count <= 400) {
    const choices: string[] = [];
    for (let idx = 0; idx < count; idx += 1) {
      const value = lowNum + idx * stepNum;
      if (value > highNum + stepNum * 1e-9) break;
      choices.push(canonicalNumberString(value));
    }
    const last = choices.length ? Number(choices[choices.length - 1]) : Number.NaN;
    if (choices.length >= 2 && Number.isFinite(last) && Math.abs(last - highNum) > stepNum * 0.55) {
      choices.push(canonicalNumberString(highNum));
    }
    return { Categorical: { choices } };
  }

  return { FloatRange: { low: lowNum, high: highNum, log: false } };
}
```

```ts
function parseMoBoolDomain(
  mode: unknown,
): "Bool" | { Categorical: { choices: string[] } } {
  const normalized = normalizeText(mode).toLowerCase() || "both";
  if (normalized === "true") return { Categorical: { choices: ["true"] } };
  if (normalized === "false") return { Categorical: { choices: ["false"] } };
  return "Bool";
}
```

### Tests to add

- backend test: categorical float parameter decodes the actual string choice, not the category index
- backend test: categorical bool parameter decodes `"true"` / `"false"` correctly
- frontend test: `parseMoRangeFloat(...)` emits string choices
- frontend test: fixed bool mode emits `choices: ["true"]` or `choices: ["false"]`
- end-to-end request test: range-builder float categorical search space round-trips into Rust successfully

---

## A2. Unknown or duplicate search-space params can silently multiply work while doing nothing

**Severity:** high  
**Type:** correctness / performance  
**Location:**
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`
- `web/src/services/optimizeRequestBuilder.ts`  
**Timing boundary affected:** app-path

### Why it matters

`build_param_indices(...)` only looks up known names. Any unknown param in the search space is still sampled by the samplers, but it is ignored by the evaluator.

That means a user typo like:

- `macz_fast_lenght`
- `slow_lne`
- `strategy_allowflip`

can silently add a no-op axis to the search.

This is bad in two ways:

1. it wastes evaluations,
2. it makes the search look more exploratory than it really is.

Duplicate param names are similarly dangerous because the samplers still see both axes, while `find_param(...)` will only bind one of them.

### Recommended change

#### Generic optimizer-level duplicate-name validation

Add duplicate-name rejection to `SearchSpace::validate()` in `crates/optimizer/src/mo.rs`.

```rust
impl SearchSpace {
    pub fn validate(&self) -> Result<(), String> {
        if self.params.is_empty() {
            return Err("search space has no params".to_string());
        }

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for p in &self.params {
            let name = p.name.trim();
            if name.is_empty() {
                return Err("param name cannot be empty".to_string());
            }

            let key = name.to_ascii_lowercase();
            if !seen.insert(key) {
                return Err(format!("duplicate param '{}'", name));
            }

            match &p.domain {
                // existing validation
                _ => {}
            }
        }
        Ok(())
    }
}
```

#### CompositeMo-specific allowlist validation

Add a backend allowlist of supported parameter names and reject anything else before building indices.

```rust
fn validate_composite_mo_param_names(space: &SearchSpace) -> Result<(), String> {
    const ALLOWED: &[&str] = &[
        "fast_ma_type",
        "slow_ma_type",
        "fast_len",
        "slow_len",
        "halftrend_amplitude",
        "halftrend_channel_deviation",
        "halftrend_atr_period",
        "macz_fast_length",
        "macz_slow_length",
        "macz_signal_length",
        "macz_lengthz",
        "macz_length_stdev",
        "macz_a",
        "macz_b",
        "macz_gamma",
        "macz_use_lag",
        "macz_use_volume",
        "ott_period",
        "ott_percent",
        "ott_ma_type",
        "strategy_commission",
        "strategy_eps_rel",
        "strategy_long_only",
        "strategy_allow_flip",
    ];

    let allowed: std::collections::HashSet<&'static str> = ALLOWED.iter().copied().collect();
    for p in &space.params {
        let key = p.name.trim().to_ascii_lowercase();
        if !allowed.contains(key.as_str()) {
            return Err(format!(
                "unsupported CompositeMo search_space param '{}'",
                p.name
            ));
        }
    }
    Ok(())
}
```

Call it near the start of `run_composite_mo_optimization_blocking_with_candles(...)`.

#### Frontend mirror validation

Add a TS-side allowlist validator for `moSearchSpace` custom JSON so bad requests fail before crossing the Tauri boundary.

### Tests to add

- duplicate parameter names rejected by `SearchSpace::validate()`
- unknown `CompositeMo` params rejected by backend
- unknown `CompositeMo` params rejected by frontend parser
- typo axis does not silently survive into sampling

---

## A3. MACZ fast/slow ordering is another structural waste surface that should be repaired, not merely rejected

**Severity:** high  
**Type:** performance  
**Location:** `crates/app_core/src/composite_mo.rs`  
**Timing boundary affected:** app-path

### Why it matters

The current repair path only enforces `fast_len < slow_len` for the MA crossover lengths.

But the evaluator also rejects points where:

- `macz.fast_length >= macz.slow_length`

So there is a second ordered-pair constraint in the search space that is currently left to the evaluator to reject.

That is exactly the kind of structural waste that should be prevented earlier.

### Recommended change

Generalize the existing ordered-pair repair helper so it can repair both:

- `fast_len < slow_len`
- `macz_fast_length < macz_slow_length`

A good design is to extract a reusable helper that works for any two `u16`-valued domains.

```rust
fn repair_strictly_ordered_pair(
    values: &mut [ParamValue],
    left_idx: usize,
    left_domain: &ParamDomain,
    left_choice_idx_by_value: Option<&HashMap<u16, usize>>,
    left_values: &[u16],
    right_idx: usize,
    right_domain: &ParamDomain,
    right_choice_idx_by_value: Option<&HashMap<u16, usize>>,
    right_values: &[u16],
) {
    let left = decode_len(left_domain, values, left_idx, None).unwrap_or(0);
    let right = decode_len(right_domain, values, right_idx, None).unwrap_or(0);

    if left != 0 && right != 0 && left < right {
        return;
    }

    let max_less_than = |sorted: &[u16], x: u16| -> Option<u16> {
        let i = sorted.partition_point(|&v| v < x);
        if i == 0 { None } else { Some(sorted[i - 1]) }
    };

    let min_greater_than = |sorted: &[u16], x: u16| -> Option<u16> {
        let i = sorted.partition_point(|&v| v <= x);
        sorted.get(i).copied()
    };

    if let Some(new_left) = max_less_than(left_values, right.max(1)) {
        set_len_value(values, left_idx, left_domain, left_choice_idx_by_value, new_left);
        let left2 = decode_len(left_domain, values, left_idx, None).unwrap_or(0);
        let right2 = decode_len(right_domain, values, right_idx, None).unwrap_or(0);
        if left2 != 0 && right2 != 0 && left2 < right2 {
            return;
        }
    }

    if let Some(new_right) = min_greater_than(right_values, left.max(1)) {
        set_len_value(values, right_idx, right_domain, right_choice_idx_by_value, new_right);
    }
}
```

Then call it for MACZ if both domains are present and decodable as `u16` domains.

### Tests to add

- repaired MACZ pair always ends with `fast < slow`
- sampler duplicate/invalid rates drop when MACZ ordering repair is enabled
- categorical MACZ integer choices also repair correctly once typed categorical decode is fixed

---

## A4. Actual evaluation counting and cancellation responsiveness need their own explicit implementation plan

**Severity:** medium-high  
**Type:** performance / correctness  
**Location:**
- `crates/app_core/src/composite_mo.rs`
- `crates/optimizer/src/mo.rs`  
**Timing boundary affected:** app-path

### Why it matters

The current optimizer samplers do not support early stop.

Inside `CompositeMo`, cancellation is checked in `eval_one(...)`, but the sampler loops themselves continue to generate/evaluate batches until their configured work is exhausted.

That means cancellation is currently:

- logically visible to the evaluator,
- but not structurally respected by the sampler.

Also, once dedup/exact-grid/chunking lands, `total_evaluations` must reflect actual eval calls, not requested budget.

### Recommended change

#### Step 1: make `done_counter` the source of truth for actual evaluations

In `CompositeMo`, after the sampler finishes, use:

```rust
let actual_total_evals = done_counter.load(Ordering::Relaxed);
```

as the real total evaluation count for the result and telemetry.

Keep a separate field for requested/budgeted evaluations if needed.

#### Step 2: add chunk boundaries and stop checks in `mo.rs`

A low-risk path is to thread a `should_stop: impl Fn() -> bool` callback through the samplers.

For example:

```rust
pub fn random_mo_search<T, F, Repair, Stop>(
    space: &SearchSpace,
    cfg: RandomMoConfig,
    repair: Repair,
    eval: F,
    should_stop: Stop,
) -> Result<Vec<MoPoint<T>>, String>
where
    T: Clone + Send + Sync,
    F: Fn(&[ParamValue]) -> EvalOutcome<T> + Send + Sync,
    Repair: Fn(&mut [ParamValue]) + Send + Sync,
    Stop: Fn() -> bool + Send + Sync,
{
    // evaluate in chunks; break between chunks if should_stop()
}
```

Then:

- `Random` / `Qmc`: stop between generated/evaluated chunks
- `Tpe`: stop at the start of each batch loop
- `NSGA2`: stop between generations and return the best frontier so far

### Tests to add

- cancellation stops `Random` / `Qmc` / `Tpe` / `NSGA2` before full configured budget
- returned `total_evaluations` matches actual eval calls
- returned partial frontier remains valid under constrained-domination

---

## A5. `Random` and `Qmc` should stream frontiers instead of materializing all evaluated points

**Severity:** medium-high  
**Type:** performance / memory  
**Location:** `crates/optimizer/src/mo.rs`  
**Timing boundary affected:** app-path

### Why it matters

Today both samplers:

- generate all values up front,
- evaluate all of them,
- then compute the Pareto front from the full evaluated vector.

That is simple, but it inflates peak memory unnecessarily.

For `Random` and `Qmc`, you do **not** need the full history; you only need the final frontier.

### Recommended change

Evaluate in chunks and keep only an incremental frontier.

```rust
fn merge_pareto_front<T: Clone>(
    frontier: &mut Vec<MoPoint<T>>,
    mut batch: Vec<MoPoint<T>>,
) {
    if frontier.is_empty() {
        *frontier = pareto_front_constrained(&batch);
        return;
    }

    frontier.append(&mut batch);
    *frontier = pareto_front_constrained(frontier);
}
```

Then use chunked generation/evaluation:

```rust
const CHUNK_SIZE: usize = 4096;
let mut frontier: Vec<MoPoint<T>> = Vec::new();
let mut generated = 0usize;

while generated < cfg.trials {
    let batch_n = (cfg.trials - generated).min(CHUNK_SIZE);
    let mut values: Vec<Vec<ParamValue>> = (0..batch_n)
        .map(|_| space.sample(&mut rng))
        .collect();
    for v in &mut values {
        repair(v);
    }

    let batch_eval: Vec<MoPoint<T>> = values
        .par_iter()
        .map(|v| {
            let out = eval(v);
            let violation = overall_violation(&out.constraints);
            MoPoint {
                values: v.clone(),
                objectives: out.objectives,
                violation,
                payload: out.payload,
            }
        })
        .collect();

    merge_pareto_front(&mut frontier, batch_eval);
    generated += batch_n;
}

Ok(frontier)
```

This also makes cancellation much easier to honor.

### Tests to add

- streaming frontier matches current full-history frontier on deterministic toy problems
- memory use drops materially on large-trial random/qmc runs

---

## A6. `build_tpe_models(...)` clones full payload-bearing `MoPoint<T>` values even though it only needs objectives + violation

**Severity:** medium  
**Type:** performance  
**Location:** `crates/optimizer/src/mo.rs` — `build_tpe_models(...)`  
**Timing boundary affected:** app-path

### Why it matters

This line is expensive on large payloads:

```rust
let subset: Vec<MoPoint<T>> = idxs.iter().map(|&i| evaluated[i].clone()).collect();
```

For `CompositeMo`, the payload includes:

- `Metrics`
- `CompositeMoParams`
- two scalar objective views

That clone happens every time TPE rebuilds its model.

But the ranking path only uses:

- `objectives`
- `violation`

### Recommended change

Create a lightweight ranking-only struct for TPE model building.

```rust
#[derive(Clone, Copy, Debug)]
struct RankPoint {
    objectives: [f64; 2],
    violation: f64,
}
```

Then add ranking helpers that work on `RankPoint`.

```rust
let subset: Vec<RankPoint> = idxs
    .iter()
    .map(|&i| RankPoint {
        objectives: evaluated[i].objectives,
        violation: evaluated[i].violation,
    })
    .collect();
```

Then rank/crowd on `subset` instead of cloning the full `MoPoint<T>` payload.

### Tests / benchmarks to add

- benchmark `tpe_mo_search(...)` on a large payload type before/after lightweight ranking
- ensure ranking order is unchanged relative to the old implementation

---

## A7. Type-aware prescan should produce type-specific fast/slow period sets for bank construction

**Severity:** medium-high  
**Type:** performance  
**Location:** `crates/app_core/src/composite_mo.rs`  
**Timing boundary affected:** app-path

### Why it matters

Once prescan is type-aware, bank construction should not use global `fast_periods` / `slow_periods` unions for every MA type.

Instead, each MA type should only build the rows it actually needs.

### Recommended change

Derive per-type period sets from `allowed_pairs_by_type`.

```rust
let mut fast_periods_by_type: HashMap<String, BTreeSet<u16>> = HashMap::new();
let mut slow_periods_by_type: HashMap<String, BTreeSet<u16>> = HashMap::new();

if let Some(map) = allowed_pairs_by_type.as_ref() {
    for ((fast_t, slow_t), pairs) in map {
        for &(f, s) in pairs {
            fast_periods_by_type.entry(fast_t.clone()).or_default().insert(f);
            slow_periods_by_type.entry(slow_t.clone()).or_default().insert(s);
        }
    }
}
```

Then build:

```rust
for t in &fast_type_choices {
    let periods: Vec<u16> = fast_periods_by_type
        .get(t)
        .map(|set| set.iter().copied().collect())
        .unwrap_or_else(|| fast_periods.clone());

    let (values, index) = build_ma_bank(/* t, &periods, ... */)?;
    fast_banks.insert(t.to_string(), Bank { values, index });
}
```

Do the same for slow banks.

### Tests to add

- optimized MA-type search + type-aware prescan builds fewer rows than the global-union fallback
- results remain numerically identical to the old bank-union path

---

## A8. Prescan needs a candidate-pair cap and a density metric, not just period-count limits

**Severity:** medium  
**Type:** performance / benchmark-fidelity  
**Location:** `crates/app_core/src/composite_mo.rs`  
**Timing boundary affected:** app-path

### Why it matters

The current code caps:

- number of fast periods
- number of slow periods

But it does **not** cap the number of allowed pairs.

That means prescan can still produce a very dense pair set that is expensive to:

- store,
- repair against,
- validate against,
- and expand into type-specific bank needs.

Also, without a density metric, you cannot tell whether prescan is meaningfully pruning the space or simply recreating most of it.

### Recommended change

Add a hard cap per MA-type pair and record a density metric.

```rust
const MAX_PRESCAN_PAIRS_PER_TYPE: usize = 4096;

for ((fast_t, slow_t), pairs) in &allowed_pairs_by_type {
    if pairs.len() > MAX_PRESCAN_PAIRS_PER_TYPE {
        return Err(format!(
            "gpu_prescan produced too many candidate pairs for ({fast_t}, {slow_t}) ({} > {})",
            pairs.len(),
            MAX_PRESCAN_PAIRS_PER_TYPE,
        ));
    }
}
```

And telemetry:

```rust
pub struct CompositeMoPrescanTelemetry {
    pub candidate_fast_periods: usize,
    pub candidate_slow_periods: usize,
    pub candidate_pairs: usize,
    pub pair_density: f64,
    // existing fields...
}
```

Where:

```rust
let pair_density = if fast_candidates.is_empty() || slow_candidates.is_empty() {
    0.0
} else {
    pair_count as f64 / (fast_candidates.len() * slow_candidates.len()) as f64
};
```

High density means prescan is not narrowing much.

### Tests to add

- overly dense prescan result fails with clear error
- density telemetry matches expected small synthetic cases

---

## A9. Bank row lookup can be made cheaper than `HashMap<u16, usize>` in the uniform case

**Severity:** low-medium  
**Type:** performance  
**Location:** `crates/app_core/src/composite_mo.rs`  
**Timing boundary affected:** app-path

### Why it matters

Every evaluation currently does:

- one `fast_bank.idx_map.get(&fast_len)`
- one `slow_bank.idx_map.get(&slow_len)`

That is not catastrophic, but it is on the hot path.

For uniform or arithmetic-run banks, the row can be computed with simple arithmetic instead of a `HashMap` lookup.

### Recommended change

Introduce an index representation:

```rust
#[derive(Clone, Debug)]
enum PeriodIndex {
    Dense { min: u16, step: u16, len: usize },
    Sparse { periods: Vec<u16> },
}

impl PeriodIndex {
    fn row(&self, period: u16) -> Option<usize> {
        match self {
            PeriodIndex::Dense { min, step, len } => {
                if period < *min || *step == 0 {
                    return None;
                }
                let delta = period - *min;
                if delta % *step != 0 {
                    return None;
                }
                let row = (delta / *step) as usize;
                (row < *len).then_some(row)
            }
            PeriodIndex::Sparse { periods } => periods.binary_search(&period).ok(),
        }
    }
}
```

Then use `PeriodIndex` inside `Bank` instead of `HashMap<u16, usize>`.

This is not a first-phase change, but it is a good clean-up once the bigger waste is gone.

---

## 3. Extra code edits I would add to the implementation sequence

## 3.1 New backend validation stage ordering

I would insert this order near the top of `run_composite_mo_optimization_blocking_with_candles(...)`:

1. `req.search_space.validate()?`
2. `validate_composite_mo_param_names(&req.search_space)?`
3. `build_param_indices(&req.search_space)?`
4. typed categorical choice maps / decoders
5. prescan
6. type-aware bank build
7. sampler dispatch

That way bad search spaces fail before any expensive prep.

---

## 3.2 Frontend-side custom search-space normalization should include a name allowlist

Add a validator before `normalizeCompositeMoSearchSpace(...)` returns.

```ts
const COMPOSITE_MO_ALLOWED_PARAMS = new Set([
  "fast_ma_type",
  "slow_ma_type",
  "fast_len",
  "slow_len",
  "halftrend_amplitude",
  "halftrend_channel_deviation",
  "halftrend_atr_period",
  "macz_fast_length",
  "macz_slow_length",
  "macz_signal_length",
  "macz_lengthz",
  "macz_length_stdev",
  "macz_a",
  "macz_b",
  "macz_gamma",
  "macz_use_lag",
  "macz_use_volume",
  "ott_period",
  "ott_percent",
  "ott_ma_type",
  "strategy_commission",
  "strategy_eps_rel",
  "strategy_long_only",
  "strategy_allow_flip",
]);
```

Then reject anything else with a clear error.

---

## 3.3 Extend the original telemetry plan with pair density and actual-vs-requested evaluation counts

I would expand the telemetry block to include:

```rust
#[derive(Clone, Debug, Serialize)]
pub struct CompositeMoSearchTelemetry {
    pub requested_evaluations: usize,
    pub actual_evaluations: usize,
    pub unique_evaluations: usize,
    pub feasible_points: usize,
    pub rejected_decode_or_order: usize,
    pub rejected_prescan_pair: usize,
    pub rejected_other_constraints: usize,
    pub exact_grid_used: bool,
    pub cancelled_early: bool,
}
```

This makes benchmark interpretation much cleaner.

---

## 4. Optional changes that go beyond the original prompt scope but still fit your goal

These are optional because they are larger, but they still fit your stated priority:

- performance,
- optimization,
- correctness,
- no feature removal,
- no functionality breakage.

## 4.1 Introduce typed discrete domains in the optimizer instead of overloading string categorical choices

This is the clean long-term fix for the categorical decode problem.

### Example direction

Instead of forcing numeric/bool discrete values through `Categorical { choices: Vec<String> }`, add a typed discrete domain such as:

```rust
pub enum ParamDomain {
    IntRange { low: i64, high: i64, step: i64, log: bool },
    FloatRange { low: f64, high: f64, log: bool },
    Categorical { choices: Vec<String> },
    DiscreteInt { values: Vec<i64> },
    DiscreteFloat { values: Vec<f64> },
    DiscreteBool { values: Vec<bool> },
    Bool,
}
```

### Why this is attractive

- removes string parsing overhead and ambiguity
- makes exact-grid eligibility cleaner
- makes frontend request building less fragile

### Why this is optional

- it is a wider optimizer API change
- it needs serialization changes and broader test coverage

---

## 4.2 Replace `(fast_len, slow_len)` with a prescanned pair-index axis after prescan

This is a very attractive medium-risk optimization.

### Idea

After prescan, instead of keeping two independent axes plus repair logic, compress the valid prescanned configurations into one categorical axis:

- `fast_slow_pair_idx`

with a lookup table such as:

```rust
Vec<(u16, u16)>
```

For type-aware prescan, you can do the same per MA-type pair, or go one step further and compress:

- `(fast_ma_type, slow_ma_type, fast_len, slow_len)`

into a single candidate index.

### Why this is attractive

- removes invalid pair sampling completely
- makes exact-grid enumeration cheaper
- eliminates repeated repair for the crossover lengths

### Why this is optional

- it changes the logical shape of the search space
- it is a bigger refactor than the safe first-phase plan

---

## 4.3 Introduce lightweight frontier/history types in `mo.rs`

The optimizer currently uses full `MoPoint<T>` structures in places where only:

- `objectives`
- `violation`
- `values`

are needed.

A clean long-term optimization is to split:

- search/ranking history,
- final payload-bearing frontier,
- model-building history.

Example:

```rust
struct MoPointLite {
    values: Vec<ParamValue>,
    objectives: [f64; 2],
    violation: f64,
}
```

This is especially useful for:

- TPE model building,
- NSGA-II final frontier extraction,
- large-budget random/QMC exact-grid runs.

---

## 4.4 Add optimizer-level cancellation support across samplers

This is larger because it touches the generic optimizer crate API, but it is a strong quality-of-life and performance improvement.

If you do it, I would recommend:

- keep it callback-based (`Fn() -> bool`) rather than tying `mo.rs` directly to atomics,
- return partial frontier + `cancelled_early` telemetry,
- ensure deterministic results when cancellation does not fire.

---

## 4.5 Add a dedicated `CompositeMo` perf gate once telemetry exists

This is beyond the original review prompt, but it is the right follow-on.

The current bench coverage in `dynamic_strategy_cpu.rs` is too narrow for `CompositeMo` optimization decisions.

A lightweight gate could track:

- prescan on/off runtime on a large-range request,
- sparse-bank build time,
- sampler duplicate rate,
- actual-vs-requested evaluations,
- NSGA-II tail/frontier time.

---

## 5. Recommended updated implementation priority

If you want the most leverage with the least break risk, I would now prioritize in this order:

### Phase A — correctness blockers that also affect search quality

1. fix categorical numeric/bool representation in `optimizeRequestBuilder.ts`
2. add typed categorical decode helpers in `composite_mo.rs`
3. reject unknown and duplicate search-space params
4. add frontend allowlist validation for custom `moSearchSpace`

### Phase B — structural search waste

5. extend repair to `MACZ fast < slow`
6. make prescan type-aware everywhere
7. make bank construction type-aware after prescan
8. add prescan pair-count cap + pair-density telemetry

### Phase C — search accounting and responsiveness

9. make `done_counter` the source of truth for actual evaluations
10. add chunk boundaries and cancellation support to samplers
11. chunk/stream frontiers in `Random` and `Qmc`
12. make TPE ranking use lightweight structures

### Phase D — optional refinement / micro-opt

13. replace bank row `HashMap` lookups with `PeriodIndex`
14. consider typed discrete optimizer domains
15. consider prescanned pair-index compression

---

## 6. Additional checklist

## 6.1 Correctness / contract fixes

- [ ] Make all frontend `Categorical.choices` values strings for CompositeMo search-space generation
- [ ] Fix `parseMoRangeFloat(...)` so discrete float choices serialize as strings
- [ ] Fix `parseMoBoolDomain(...)` so fixed bool choices serialize as strings
- [ ] Add backend typed categorical decoders for `u16`, `f64`, and `bool`
- [ ] Replace non-length/non-MA-type `get_u16/get_f64/get_bool` reads with typed decoders
- [ ] Add frontend validation for unsupported CompositeMo search-space param names
- [ ] Add backend validation for unsupported CompositeMo search-space param names
- [ ] Add generic duplicate-param rejection to `SearchSpace::validate()`
- [ ] Add CompositeMo test coverage for categorical numeric/bool params

## 6.2 Structural search-efficiency fixes

- [ ] Extend repair logic to `macz_fast_length < macz_slow_length`
- [ ] Add counters for MACZ ordering rejections before/after repair
- [ ] Keep prescan allowed configs keyed by `(fast_ma_type, slow_ma_type)`
- [ ] Build type-specific fast period sets from prescan output
- [ ] Build type-specific slow period sets from prescan output
- [ ] Stop using global period unions for every MA type after type-aware prescan is available
- [ ] Add per-type prescan pair-count caps
- [ ] Add prescan pair-density telemetry

## 6.3 Evaluation accounting / cancellation / memory

- [ ] Make actual eval count come from the real evaluator invocation counter
- [ ] Add requested-vs-actual evaluation fields to telemetry
- [ ] Add chunked generation/evaluation for `Random`
- [ ] Add chunked generation/evaluation for `Qmc`
- [ ] Add streaming frontier merge for `Random`
- [ ] Add streaming frontier merge for `Qmc`
- [ ] Add sampler stop checks for cancellation
- [ ] Return valid partial frontier on cancellation
- [ ] Make NSGA-II stop between generations if cancellation fires
- [ ] Benchmark memory/runtime before vs after frontier streaming

## 6.4 TPE / sampler internals

- [ ] Replace payload-bearing TPE ranking subset clones with lightweight ranking structs
- [ ] Benchmark TPE model-build overhead before/after lightweight ranking
- [ ] Confirm ranking order is unchanged after lightweight subset refactor
- [ ] Keep dedup + actual-eval counting consistent with the new accounting model

## 6.5 Optional micro-optimizations

- [ ] Replace bank `HashMap<u16, usize>` with `PeriodIndex` where profitable
- [ ] Benchmark bank row lookup hot-path cost before/after `PeriodIndex`
- [ ] Consider typed discrete domains in `ParamDomain` as a longer-term cleanup
- [ ] Consider compressed valid-pair index domains after prescan

## 6.6 Tests to add specifically for items missed in the first plan

- [ ] Custom categorical float search-space param decodes the actual value
- [ ] Custom categorical bool search-space param decodes the actual value
- [ ] Duplicate param names are rejected at optimizer validation time
- [ ] Unknown CompositeMo param names are rejected by backend
- [ ] Unknown CompositeMo param names are rejected by frontend
- [ ] MACZ ordering repair prevents invalid fast/slow MACZ combos
- [ ] Type-aware prescan reduces bank rows per MA type
- [ ] Over-dense prescan candidate sets fail with a clear error
- [ ] `Random` streaming frontier matches full-history frontier
- [ ] `Qmc` streaming frontier matches full-history frontier
- [ ] Cancellation returns partial frontier and accurate actual evaluation count
- [ ] TPE lightweight ranking path preserves result equivalence

