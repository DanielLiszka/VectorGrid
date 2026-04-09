# Prompt Answers Synthesis — Questions 1 To 8

Date: 2026-04-08

Scope: review of the eight final consolidation memos in `SASS research/Prompt Answers/`, limited to the broader-CS question pack. This synthesis does **not** include the still-in-progress math-focused prompts.

## Executive conclusion

A viable research direction **does** still exist, but it is much narrower than the original “new optimizer” framing.

The strongest surviving direction is:

- **a measurement-first benchmark/evaluation study on trustworthy fidelity ladders for exact-anchor, route-conditional, validation-coupled strategy optimization**, with
- **a bounded reliability-gated scheduler** as an optional follow-on layer, not as the headline claim.

That means:

- do **not** reopen broad implementation for a new universal optimizer,
- do **not** reopen broad implementation for a standalone benchmark/platform paper,
- do **not** reopen broad implementation for exact pruning,
- and do **not** reopen more CUDA/SASS work as the broader-CS target.

The first 8 answers collectively support one main line and two secondary lines:

1. **Primary direction**
   - trustworthy multi-fidelity / evaluation-science study
2. **Secondary follow-on if the fidelity audit passes**
   - exact-anchor-aware portfolio / solver selection
3. **Secondary tooling/support direction**
   - optimization-contract IR / search-space normalizer with conformance checks

## Question-by-question validity

## Question 1

### What is valid

- The memo is correct that **“beat brute force on `DoubleMa`” is the wrong universal target**.
- It is also correct that the repo’s exact `DoubleMa` route should be treated as an **anchor baseline**, not as the main thesis.
- The proposed budgeted objective framing is valid:
  - full-fidelity simple regret under wall-clock budget
  - anytime incumbent quality
- The memo is also right that the surviving claim is closer to:
  - benchmark/problem formulation
  - solver selection
  - or systems/AutoML evaluation
  than to a new optimizer paper.

### Caveat

- Q1 is still a little too broad because it mixes:
  - fidelity selection
  - solver selection
  - and benchmark framing
  in one umbrella. It is a good framing answer, but not yet the cleanest implementation target.

### Disposition

- **Valid as the framing reset.**
- Not the final implementation target by itself.

## Question 2

### What is valid

- This is the strongest answer in the set.
- It is valid that the repo exposes **real candidate fidelity ladders**:
  - `GpuVdps` rung budgets on the optimization side
  - walk-forward / reopt-walk-forward / CSCV / SPA budget knobs on the validation side
- It is also valid that the right claim is **not** “we invented a new generic MFBO algorithm.”
- The strongest surviving claim is correctly narrowed to:
  - **which repo-native fidelities are trustworthy enough to guide exact, validation-coupled search**
  - and **when they fail**
- The memo correctly keeps:
  - exact finalization authoritative
  - CUDA as substrate only
  - and a small reliability-gated scheduler as optional follow-on rather than the main thesis.

### Caveat

- Not every cheaper knob is a real fidelity.
- The memo mostly handles this correctly, but this needs to stay strict:
  - some validation settings are budget reductions,
  - some are changed semantics,
  - and some “pseudo-fidelities” should be rejected early.

### Disposition

- **Adopt as the leading research direction.**

## Question 3

### What is valid

- The memo is correct that a **universal optimizer** is less plausible than a small **portfolio / scheduler / selector**.
- It is also valid that this repo has genuine heterogeneity:
  - exact `DoubleMa`
  - approximate/staged search
  - route-conditional GPU/hybrid/CPU behavior
  - multiple strategy families
- The “exact-vs-approx gating” idea is a real and repo-specific question.
- The recommendation to keep the controller small:
  - static features
  - optional probe
  - at most one switch
  is sensible and appropriately bounded.

### Caveat

- Q3 is strongest **after** Q2, not before it.
- Without a clean fidelity audit and benchmark protocol, Q3 risks becoming:
  - generic algorithm selection
  - or weakly evaluated scheduling.

### Disposition

- **Strongest follow-on direction after Q2.**

## Question 4

### What is valid

- The memo is right to **close the standalone batch-synchronous optimizer thesis**.
- The repo does expose that regime, but the memo is correct that the literature collision is too strong for this to stand alone.
- It is also correct that any useful residue belongs under:
  - portfolio/scheduling
  - or benchmark/evaluation
  not under “new optimizer.”

### Caveat

- None material. This is a clean closure answer.

### Disposition

- **Close as a primary direction.**

## Question 5

### What is valid

- The memo is right to reject **broad exact pruning** as a repo-wide thesis.
- It is also right that the only remotely credible version is:
  - stripped-down `DoubleMa`
  - theorem-first
  - with permission for a negative result
- The kill test is valid:
  - if no nontrivial admissible bound exists on paper first, the lane should close.

### Caveat

- This is mathematically attractive but strategically weak.
- As a repo-level direction, it is correctly ranked below Q2 and Q3.

### Disposition

- **Design-only side note, not a main research line.**

## Question 6

### What is valid

- This is the strongest non-optimizer alternative.
- The repo really does already encode an optimization contract across:
  - request shaping
  - route legality
  - output-contract legality
  - capability previews
  - backend constraints
- The memo is correct that the strongest claim is **not “compiler” yet**.
- The narrower and more honest claim is:
  - **optimization-contract IR / search-space normalizer**
  - plus **semantic conformance checking**
- The requirement to compare extracted IR against hand-authored spaces is exactly right.

### Caveat

- Novelty risk is still real.
- Without:
  - strong conformance checking
  - multiple lowering targets
  - and measurable optimizer portability / invalid-trial reduction
  this can collapse into engineering cleanup.

### Disposition

- **Best support/tooling direction.**
- Worth keeping alive, but behind Q2 as the primary paper lead.

## Question 7

### What is valid

- The memo is right to reject a **standalone benchmark/platform paper right now**.
- The benchmark/platform field is already too crowded for VectorGrid to enter without a sharper empirical gap.
- It is also correct that benchmark packaging becomes much stronger **after** another direction lands.

### Caveat

- None material. This is another clean closure answer.

### Disposition

- **Keep only as a deferred packaging direction.**

## Question 8

### What is valid

- The venue-aware answer mostly reinforces Q2.
- It is valid that the best venue family fit is closer to:
  - AutoML benchmark/evaluation
  - or evaluation/benchmark tracks
  than to methods-heavy optimizer venues.
- It is also valid that the strongest broader direction is:
  - multi-fidelity trustworthiness
  - on heterogeneous, route-conditional workloads
  - with exact anchors
  rather than a new optimizer paper.

### Caveat

- The venue-specific naming is less important than the direction itself.
- Treat the venue advice as soft guidance, not as the reason to choose the direction.

### Disposition

- **Valid reinforcement of Q2, not a separate research program.**

## Cross-question synthesis

The answers are not equally important.

The high-signal pattern is:

- **Q1** resets the target correctly.
- **Q2** identifies the strongest surviving primary direction.
- **Q3** identifies the strongest follow-on if Q2 succeeds.
- **Q6** identifies the strongest support/tooling contribution.
- **Q4, Q5, and Q7** mostly close weaker branches.
- **Q8** confirms that the surviving direction is evaluation/benchmark-oriented rather than optimizer-methods-oriented.

In other words:

- Q2 is the paper lead.
- Q3 is the next paper or second phase.
- Q6 is the most useful enabling systems/tooling lane.

## Final decision

## Adopted research direction

Pursue:

- **trustworthy fidelity ladders for exact-anchor, route-conditional, validation-coupled optimization**

More concretely:

- build a **bounded fidelity-audit harness**
- measure which cheaper stages preserve:
  - rank
  - top-k retention
  - regret ordering
  - and downstream validation decisions
- keep **full-fidelity exact evaluation authoritative**
- only add a **small reliability-gated scheduler** if the fidelity audit succeeds

## What not to pursue as the lead

- not a universal optimizer
- not a standalone batch-synchronous optimizer thesis
- not broad exact pruning
- not a standalone benchmark/platform paper
- not more CUDA/SASS work as the broader-CS target

## Why this is the right choice

This direction wins because it is the only one that is:

- strongly supported by multiple question answers
- grounded in repo-native structure
- different enough from generic optimizer literature
- still meaningful if CUDA is only substrate
- and able to produce a publishable negative result if the answer is “the fidelities are not trustworthy”

That last point matters. A clean negative result here is still useful. Several other directions only work if they win positively.

## Recommended next step

Reopen work only for a **bounded Phase 0 fidelity audit**, not for a broad new implementation branch.

That Phase 0 should do three things:

1. define the exact endpoint and a short list of candidate fidelities,
2. measure rank/top-k/regret preservation across:
   - `DoubleMa`
   - at least one routed composite-profile case
   - at least one genuinely non-`DoubleMa` family if feasible in the current workflow,
3. stop immediately if the fidelities do not generalize beyond the exact `DoubleMa` anchor.

## Kill condition for the overall direction

Close the broader-CS direction if:

- the audited fidelities are not trustworthy outside the exact `DoubleMa` anchor,
- or the resulting evaluation does not expose behavior meaningfully different from current benchmark suites.

If that happens, downgrade the result to:

- a benchmark/methodology negative-result memo,
- or stop entirely.

## Final verdict

**Yes, a viable research direction still exists.**

But it is **not** the original optimizer thesis.

It is:

- **a measurement-first, benchmark/evaluation-style study of trustworthy fidelity ladders**, with
- **a small reliability-gated scheduler** as the only justified follow-on,
- and **optimization-contract IR / conformance tooling** as the best auxiliary systems direction.
