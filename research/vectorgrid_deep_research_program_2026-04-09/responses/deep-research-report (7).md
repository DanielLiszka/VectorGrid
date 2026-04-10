# Minimum Useful Options Parity Envelope for a LEAN-Credible Options-Expression Optimizer

## Executive summary

A “minimum useful parity envelope” that earns entity["organization","LEAN","quantconnect algo engine"]‑like credibility without inheriting its full general‑purpose simulator complexity can be defined as a **small set of non-negotiable semantics** (contract selection correctness, execution modeling transparency, and validation discipline) plus a **deliberately bounded strategy space** (fixed‑shape structures and template exits). In practice, the credibility threshold is reached when a technically literate reviewer can (a) reproduce the contract-selection decision at each event time, (b) understand fill/fee/slippage/assignment assumptions, and (c) see an overfitting-resistant evaluation protocol for a large search space. citeturn3view3turn5view0turn6view0turn6view1

Authoritative references suggest that entity["company","QuantConnect","algorithmic trading platform"] positions options realism around: **universe selection and filtering**, **fill + slippage + fees**, **buying-power/margin checks**, and **exercise/assignment models** integrated into “brokerage models.” citeturn3view3turn5view0turn5view1turn5view2turn5view3turn3view0turn3view2

For an options-expression optimizer (especially a GPU-first, compiled-tensor design), the shortest credible path is typically to **avoid assignment complexity by construction** (long-only and/or forced close before expiration), while still implementing (and labeling) the execution and validation semantics that materially affect results. If the product later supports short premium, then assignment/exercise modeling becomes part of the parity envelope because assignment is fundamental to how options obligations are allocated and realized. citeturn4view0turn4view1turn3view2

## Extracted question text

Question 9 (DR-09) in `VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md`:

> “For an options-expression optimizer that wants LEAN-like credibility without inheriting the full complexity of LEAN, what is the minimum useful parity envelope? Focus on fixed-shape contract selection, deterministic structure compilation, template exits, assignment semantics, execution assumptions, and validation credibility. Distinguish the parity targets that are worth pursuing from those that should remain intentionally out of scope for a GPU-first desktop product. Favor additive semantics and reporting improvements that build on the current options lane instead of widening scope into a simulator rewrite.”

## Scope framing and assumptions

This answer assumes the “options-expression optimizer” is a system that **takes pre-defined entry events** (often derived from an underlying strategy) and then optimizes a bounded family of options expressions around those events (contract choice + exit template). The parity target is not “can run any event-driven options algorithm,” but “produces decision-grade, well-specified backtest results for its bounded expression family.”

Assumptions made due to the question’s lack of further product constraints:

- The target use case is **research credibility**, not brokerage-ready execution; therefore, correctness and provenance outrank microstructure completeness.
- The “minimum envelope” is scoped to **what materially changes P&L / risk / validity** for typical option backtests: contract eligibility, bid/ask vs mid assumptions, fees/slippage, expiry behavior, and overfitting control.
- The product can impose **scope constraints** (e.g., “no holding through expiration,” “no short premium”) as long as it is explicit and enforced.

## What LEAN treats as core option-reality plumbing

The most relevant “credibility anchors” in entity["organization","LEAN","quantconnect algo engine"] (as documented by entity["company","QuantConnect","algorithmic trading platform"]) map directly onto the DR-09 focus areas:

**Contract universe selection and filtering.** LEAN’s option universe model is explicit about how options contracts enter the tradable universe: an `OptionUniverseSelectionModel` is called on a schedule (refresh interval) and can apply an `OptionFilterUniverse` to narrow contracts. citeturn3view3turn0search12

**Deterministic contract filtering primitives.** LEAN exposes filtering methods that reflect common options-selection dimensions—Greeks, implied volatility, open interest, etc.—as explicit filter functions over candidate contracts (example shown in the source for `OptionFilterUniverse`). citeturn5view5

**Execution modeling: fills, slippage, brokerage realism.** LEAN formalizes fills via “fill models,” which can incorporate spread and interact with slippage models; slippage is defined as the deviation between expected and actual fill price, modeled to improve realism. citeturn5view0turn5view1  
LEAN also frames “brokerage models” as the composition of models that simulate live brokerage behavior and validate orders before submission. citeturn5view3

**Buying power and margin constraints.** LEAN uses buying power (margin) models to determine whether orders should be submitted, warning that calculations vary with brokerage and context. citeturn5view2

**Exercise and assignment semantics.** LEAN has explicit “exercise” and “assignment” models and describes default behaviors (e.g., scanning the portfolio periodically and applying rules around ITM thresholds and days-to-expiry). citeturn3view0turn3view2turn3view1

**Assignment as a real-world mechanism, not just a simulator detail.** Per entity["organization","The Options Clearing Corporation","us options clearinghouse"], exercise instructions eventually lead to OCC assigning exercised contracts to short positions, and the allocation process is procedural (including randomization/assignment “wheel” concepts). citeturn4view0turn4view1  
This matters because any optimizer that includes short options must define how it handles assignment, or it will systematically misstate tail risks and realized outcomes.

**Option pricing models (mainly for Greeks/IV).** LEAN documents default option pricing models and the existence of specific model families (e.g., binomial models for American-style options and Black-Scholes for European-style options) to compute Greeks/IV. citeturn5view4turn2search2  
An options-expression optimizer that relies on delta/IV-based filters should either (a) compute them with a well-defined model, or (b) restrict filters to observable quantities (moneyness, DTE) unless a trusted data source provides Greeks/IV.

## Minimum useful parity envelope

A “minimum useful parity envelope” is the smallest set of **required semantics + guardrails** that makes outputs comparable to a mature engine like LEAN, without building a general simulator. The envelope below is organized by the question’s required focus areas.

### Contract selection that is time-valid and reproducible

The minimum credible contract-selection contract is:

- **A time-indexed candidate universe**: at each entry event time, you can show which contracts were “available” and why. LEAN’s universe mechanism makes the selection step explicit and scheduled, which is part of what makes it auditable. citeturn3view3turn0search12
- **Deterministic selection rules** over that universe: given the same inputs, the same contract is selected (or the same “invalid reason” is produced).
- **Explicit filter dimensions** (minimum set):
  - DTE window / target DTE bucket
  - strike selection rule (ATM / fixed moneyness / strike steps)
  - liquidity gating (at least: “no quote / no tradeable price” handling)
  - optional: open interest / IV / delta only if those fields are trusted (LEAN exposes such filters, but the optimizer can defer them if its data doesn’t support them). citeturn5view5

A practical deterministic selector can be described (and implemented) as:

```python
def select_contract(contracts, as_of_ts, right, target_dte_days, moneyness_rule):
    # 1) eligibility: listed <= as_of < expiry, correct right (call/put)
    eligible = [c for c in contracts
                if c.listed_ts <= as_of_ts < c.expiry_ts and c.right == right]

    # 2) expiry choice: minimize |(expiry - as_of) - target_dte|
    # tie-breakers: prefer standard expiries, higher OI, tighter spread (if available)
    eligible.sort(key=lambda c: (abs((c.expiry_ts - as_of_ts).days - target_dte_days),
                                 c.is_weekly,              # false preferred
                                 -c.open_interest or 0,    # higher preferred
                                 c.spread or float("inf")))# lower preferred

    # 3) strike choice: closest strike to moneyness target (ATM/OTM steps/etc.)
    chosen_expiry = eligible[0].expiry_ts
    same_expiry = [c for c in eligible if c.expiry_ts == chosen_expiry]
    return choose_strike(same_expiry, moneyness_rule)
```

This is not “LEAN parity,” but it matches LEAN’s core idea: selection is an explicit, inspectable function over a known universe. citeturn3view3turn5view5

### Deterministic structure compilation with fixed-shape geometries

To remain GPU-credible and avoid simulator rewrite, the minimum envelope should **only include structures that compile into fixed-size leg bundles**.

A recommended “minimum useful” set (each with deterministic compilation):

- **Single-leg long call/put** (1 leg).
- **Same-expiry vertical debit spread** (2 legs).
- Optionally: **long straddle/strangle** (2 legs) if the product wants symmetric direction-agnostic exposure (still fixed-shape).

LEAN’s `OptionFilterUniverse` includes convenience methods for multi-leg “strategy-like” contract selection (e.g., straddles and butterflies). That’s not a requirement, but it is evidence that mature platforms treat “fixed-shape strategy templates” as first-class selection concepts. citeturn3view0turn0search8turn5view5

Minimum compilation requirements for credibility:

- The compiler must output a **fully specified leg bundle** (contract identifiers, expiry, strikes, quantity ratios, multipliers).
- If a leg cannot be resolved (missing strike, no quote, etc.) the compiler must produce an **invalid reason code**, not silently substitute.
- The compilation must reference **only information available as-of the entry time** (avoid implicit lookahead via “as-of-later” contract reference).

### Template exits

“Template exits” are one of the strongest levers for keeping scope bounded while still being comparable and auditable.

A minimum credible template set:

- Fixed hold: exit after *N* minutes/bars.
- Target / stop: exit when profit/loss thresholds are hit.
- “End-of-day or target/stop” (common intraday containment).
- Forced liquidation policy before expiry cutoff (see assignment section).

For each template, the engine must define:

- How it resolves ambiguous intra-bar ordering (e.g., both target and stop touched within one bar). A conservative rule (“take worst outcome”) is acceptable if disclosed; it reduces optimistic bias.
- Which price is used for exit evaluation under each execution lane (bid/ask vs mid vs aggregate) and whether slippage is applied. These are exactly the kinds of “reality modeling” semantics LEAN surfaces through fill and slippage modeling. citeturn5view0turn5view1

Minimum P&L formula for long options (per leg):

\[
\text{PnL} = (P_{\text{exit}} - P_{\text{entry}})\cdot \text{multiplier}\cdot q \;-\; \text{fees} \;-\; \text{slippage\_costs}
\]

For spreads/combos: sum leg P&L.

### Execution assumptions

To be “LEAN-like credible,” the engine does not need every LEAN model; it needs the **same kinds of levers and disclosures** that determine whether results are plausible.

Minimum required levers:

- **Fill model choice** (at least two lanes):
  - Conservative quote-side lane: entry at ask and exit at bid for long options (captures spread cost).
  - Mid lane: entry/exit at mid (research lane; must be labeled as optimistic relative to quote-side).  
  LEAN’s fill model concept explicitly allows spread costs and interacts with slippage models. citeturn5view0turn5view1
- **Slippage model** (even if minimal):
  - Constant ticks/percent of spread, or a simple constant model; the key is that it is explicit, repeatable, and used consistently with fills. citeturn5view1turn2search5
- **Fee model** (even if simple):
  - Per-contract commission + exchange/regulatory fee approximation. LEAN treats fees as part of its “reality modeling” stack. citeturn2search14turn5view3
- **Order validity / buying power checks** (if you model portfolio constraints at all):
  - If the optimizer assumes “infinite capital,” say so; otherwise implement at least a basic buying-power constraint. LEAN frames buying power as the mechanism that prevents orders that would be rejected by the brokerage. citeturn5view2turn2search12

In a compiled-tensor optimizer, these assumptions are best implemented as **job-level configuration**, not per-trade branching, to preserve determinism and GPU regularity.

### Assignment and exercise semantics

This is the most important “credibility fork”:

- If you support **short options** (including credit spreads), you must define assignment behavior, because assignment is intrinsic to short options obligations and is operationally realized through OCC assignment procedures. citeturn4view0turn4view1turn3view2
- If you support **long-only** positions and force liquidation well before expiry, you can credibly keep assignment modeling out of scope (because you never hold an exposure that can be assigned to you), but you must still define what happens at/near expiration if positions exist.

What LEAN documents is instructive for minimum parity semantics:

- LEAN has an explicit “exercise” model and a “DefaultExerciseModel” concept. citeturn3view0
- LEAN also documents default assignment model behavior for sold options (scan portfolio hourly; consider American options near expiry; exercise if sufficiently ITM and profitable after fees). citeturn3view2turn3view1
- LEAN’s option exercise order documentation highlights key constraints (exercise orders require a long position; European-style options cannot be exercised early) and describes an automated exercise/scan behavior in backtests. citeturn3view1

A minimum useful parity envelope therefore needs an explicit policy, one of:

**Policy A: No-expiry exposure rule (recommended minimal envelope).**  
Disallow holding options beyond a configured cutoff (e.g., “close all positions X minutes before market close on expiry date”), and treat inability to close as a modeled invalid/forced liquidation outcome. This avoids having to implement early exercise optimization and assignment allocation.

**Policy B: Expiry-aware long-only exercise rule.**  
If a long option is held to expiry, define whether it is (a) automatically exercised when ITM, and how settlement is represented (stock delivery vs cash settlement), or (b) automatically liquidated. LEAN’s documentation illustrates exercise leading to underlying positions and has examples for cash settlement via custom exercise models. citeturn3view1turn3view0

**Policy C: Short-options assignment model (only if you go beyond minimal).**  
Implement assignment/exercise triggers and resulting underlying delivery obligations (or cash settlement where appropriate). At minimum, document the exercise/assignment model, scan frequency, ITM thresholds, and fee treatment (LEAN documents these parameters in its default assignment descriptions). citeturn3view2turn3view1turn4view0turn4view1

### Validation credibility for a large search space

An options-expression optimizer is typically a **massive multiple-testing engine** (many tuples/candidates evaluated on the same finite set of events). That creates a credibility problem even if execution modeling is perfect.

Two authoritative anchors explain why:

- High-throughput backtesting increases false positives when repeated testing is performed on the same dataset; this is directly discussed in the literature on backtest overfitting and multiple testing. citeturn6view0
- “Data snooping” (reusing the same data across many specification searches) can produce spurious winners; the Reality Check framework exists specifically to account for this multiple-comparisons effect. citeturn6view1

Minimum credible validation discipline, tailored to an options-expression optimizer:

- **Train/holdout separation at the options layer** (not just at the underlying-signal layer). The options expression is a second optimizer; treat it as such.
- **Fold-aware scoring** (walk-forward or split-based), with reporting that separates in-sample selection metrics from out-of-sample holdout metrics.
- **Finalist-only “sealed” evaluation** on an untouched slice when feasible (prevents iterative tuning on the same holdout).
- **Stress rescoring** for top candidates under harsher assumptions (wider effective spread, more slippage, higher fees). This often delivers more credibility per engineering hour than broadening the simulator.

## Alternatives comparison

The DR-09 question allows multiple plausible “minimum envelopes.” The main decision is how much assignment/margin complexity you admit.

| Envelope | What you implement | What you explicitly keep out of scope | Pros | Cons | When it’s the right choice |
|---|---|---|---|---|---|
| Long-only, no-expiry-exposure (recommended minimal) | Deterministic contract selection; fixed-shape long structures (1–2 legs); template exits; quote-side vs mid fill lanes; fees + simple slippage; explicit forced-close/expiry cutoff; fold-aware validation & stress rescoring citeturn3view3turn5view0turn5view1turn6view0turn6view1 | Short premium; assignment/early exercise modeling; portfolio margin simulation; rolling; arbitrary combos; intrabar microstructure | Smallest credible envelope; stays compatible with compiled/GPU scoring; easy to explain and audit; avoids the hardest “gotchas” by construction | Cannot represent many popular income/short-premium strategies; may under-represent real-world exit friction if not stressed adequately | You want research credibility quickly and your product philosophy favors bounded, high-throughput optimization |
| Defined-risk spreads including short legs | Everything above **plus** short-leg handling, assignment/exercise triggers, at least basic margin/buying-power checks citeturn3view2turn5view2turn4view1 | Full brokerage fidelity; rolling; portfolio margin edge cases; complex multi-legs | Covers widely used strategies (credit spreads); improves realism where assignment risk is material | Engineering complexity rises sharply; correctness is harder; GPU regularity may suffer unless assignment is modeled as a post-pass | You need short exposure for product relevance and are willing to accept higher implementation/test burden |
| “LEAN parity” simulator breadth | Broad order types, brokerage models, settlement and margin variants, richer assignment/exercise, event-driven flexibility citeturn5view3turn3view2turn2search3 | Very little | Maximum familiarity for LEAN users; broadest strategy coverage | High scope; tends to collapse a compiled-tensor speed model; large surface area for subtle bugs; conflicts with “no simulator rewrite” intent | Only if the product goal is becoming a general engine (which DR-09 cautions against) |

### Mermaid diagram of the recommended minimal envelope

```mermaid
flowchart TD
  A[Underlying edge events\n(parent signals)] --> B[Event manifest\n(time + bar-index anchored)]
  B --> C[Contract universe as-of each event]
  C --> D[Deterministic contract selection\n(DTE + moneyness + filters)]
  D --> E[Fixed-shape structure compilation\n(1-2 leg bundles)]
  E --> F[Exit-template compiler\n(hold/target/stop/EOD)]
  F --> G[Execution lane scorer\nquote-side vs mid\n+ fees + slippage]
  G --> H[Validation stack\ntrain/holdout/sealed + stress]
  H --> I[Options-expression artifact\n+ provenance + diagnostics]
```

## Key prioritized references

- entity["company","QuantConnect","algorithmic trading platform"] docs on **options universes and filtering** (contract selection as an explicit step). citeturn3view3  
- entity["company","QuantConnect","algorithmic trading platform"] docs on **exercise and assignment models** and **option exercise orders** (default behaviors and requirements). citeturn3view0turn3view2turn3view1  
- entity["organization","The Options Clearing Corporation","us options clearinghouse"] primer and procedures on **exercise and assignment** (real-world allocation mechanism). citeturn4view0turn4view1  
- entity["company","QuantConnect","algorithmic trading platform"] docs on **fill models, slippage, brokerage, and buying power modeling** (reality-modeling semantics). citeturn5view0turn5view1turn5view2turn5view3  
- Research on **multiple testing / backtest overfitting / data snooping** as a validation credibility anchor (PBO/CSCV framing and Reality Check). citeturn6view0turn6view1