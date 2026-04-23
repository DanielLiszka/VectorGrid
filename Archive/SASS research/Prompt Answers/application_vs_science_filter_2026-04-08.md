# Application vs Science Filter

Date: 2026-04-08

Purpose: apply a stricter filter than the earlier broader-CS synthesis and ask:

> If the application is treated as only one instantiation, what actual science remains?

This memo is intentionally harsher than the broader-CS memos. The goal is to separate:

- application-specific engineering or evaluation work,
- from reusable scientific content that could survive outside this trading/strategy domain.

It is based only on the completed Question 1 to Question 8 answers. The math-focused prompt tracks are still in progress and may materially change the result.

## Science filter

A direction passes the filter only if most of the following are true:

1. **The problem statement survives domain substitution.**
   You should be able to replace “trading strategy optimization” with another expensive optimization application and still have the same scientific question.

2. **The main claim survives backend substitution.**
   CUDA, GPU residency, and the current exact runtime can help instantiate the problem, but they cannot be the conceptual core.

3. **The contribution is more than evaluation on one application.**
   A benchmark or case study can be useful, but if the main novelty is “here is an interesting application,” the science content is weak.

4. **The abstractions are reusable.**
   A theorem, algorithmic primitive, semantic IR, conformance method, or impossibility claim should transfer beyond this repo.

5. **The likely reviewer question “why should anyone outside this application care?” has a strong answer.**

If a direction fails those tests, it may still be valuable engineering or benchmarking, but it is not strong science.

## Applying the filter to the current directions

## 1. Multi-fidelity trustworthiness

### What survives

- There is a real abstract question:
  - when do cheap fidelities preserve final full-fidelity ordering well enough to guide search?
- That question is broader than trading.

### What does not survive cleanly

- The current strongest version still depends heavily on this application’s:
  - exact `DoubleMa` anchor
  - route-conditional execution
  - validation-coupled workflow
- Without a stronger abstract formulation, the direction still reads primarily as:
  - an application-grounded evaluation study
  - not a clean pure-CS scientific contribution

### Verdict

- **Not strong enough yet as pure science from the first 8 answers alone.**

## 2. Portfolio / solver selection

### What survives

- Portfolio selection is an abstract optimization problem.
- Exact-vs-approx gating is conceptually reusable.

### What does not survive cleanly

- The currently strongest formulation depends on this repo’s exact mix of:
  - exact grid
  - staged search
  - route previews
  - backend heterogeneity
- That makes it look more like:
  - application-specific scheduling
  - than a new scientific contribution

### Verdict

- **Not strong enough yet as the pure-science lead.**

## 3. Standalone batch-synchronous optimizer

### What survives

- Large synchronous expensive-batch optimization is a real abstract regime.

### What does not survive cleanly

- The prior-work collisions are too strong.
- The distinctive part leans too much on the repo’s execution substrate.

### Verdict

- **Fails the science filter.**

## 4. Broad exact pruning

### What survives

- Exact pruning / admissible elimination is pure algorithms research if it exists.
- This is the cleanest “science” category in principle.

### What does not survive cleanly

- The broad repo-wide version does not survive.
- The only plausible version is a stripped-down restricted lattice or a negative result.
- Right now, from the first 8 answers alone, that is still a conjectured theorem lane, not an established direction.

### Verdict

- **Possible science, but not yet a supported lead.**
- This depends heavily on the still-running math prompts.

## 5. Benchmark / platform

### What survives

- Benchmark science is possible in principle.

### What does not survive cleanly

- In this repo, the strongest benchmark framing still depends too much on the application itself.
- The first 8 answers correctly demote it to a support direction.

### Verdict

- **Fails the science filter as a primary direction.**

## 6. Optimization-contract IR / search-space normalizer

### What survives

- This is the only direction from the first 8 answers that naturally separates from the application.
- The scientific object is not “a trading strategy.”
- The scientific object is:
  - extracting optimization contracts from executable surfaces,
  - normalizing them into a canonical IR,
  - lowering them across solver backends,
  - and checking semantic conformance.

That object can exist in many domains besides this repo.

### What still makes it fragile

- It can still collapse into engineering cleanup if:
  - the IR is too repo-specific,
  - the extraction is mostly hand-authored,
  - or conformance checking is shallow.

So even this direction is not automatically science.

### What would be required for it to count

It must be reframed away from “supporting this app” and toward:

- a general method for extracting **optimization contracts** from executable application policy surfaces,
- a canonical IR with explicit semantics,
- multiple lowerings,
- and a conformance methodology that is reusable beyond this repo.

If that generalization does not hold, this direction should also be demoted.

### Verdict

- **This is the only direction from the first 8 answers that currently survives the science filter at all.**
- But it survives only provisionally.

## Strict conclusion

Using only the completed Question 1 to Question 8 answers:

- there is **not yet a confident pure-science lead** that is clearly stronger than the application,
- except for a **provisional** IR / conformance direction,
- and even that remains fragile until it is clearly generalized beyond this repo.

So the honest answer is:

1. **Best provisional science direction**
   - optimization-contract IR / semantic conformance

2. **Best possible theorem direction, but not yet supported enough**
   - restricted exact pruning or an impossibility / negative-result formulation

3. **Everything else from the first 8 answers**
   - stronger as application-grounded AutoML/systems/evaluation work than as pure science

## What this implies operationally

If you insist on separating application from science **before** reopening technical work, then the right move is:

- **do not yet commit to any mainline research implementation based only on Questions 1 to 8**
- wait for the math-focused prompts
- and use the current result only to keep one provisional systems-science direction alive:
  - optimization-contract IR / semantic conformance

If the math prompts fail to produce:

- a real theorem,
- a real impossibility result,
- or a cleaner generalized scientific abstraction,

then the honest conclusion may be:

- **this repo supports strong application-grounded research artifacts, but not a clearly defensible pure-CS science lead yet**

## Final verdict

Under a strict “separate the application from the science” standard:

- **the current first-8 answer set does not yet establish a confident pure-CS research lead**
- **the only provisional survivor is optimization-contract IR / semantic conformance**
- **the final answer should remain open until the math-focused threads finish**
