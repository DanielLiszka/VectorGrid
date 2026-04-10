# Credible Benchmarking and Public Comparison Discipline for VectorGrid

## Executive summary

A credible public benchmark comparing a GPU trading optimizer like VectorGrid to a CPU-oriented VectorBT workflow should be structured like a “benchmark charter + disclosure package,” not a single headline speedup. The charter should (a) pin down semantic equivalence (same dataset, same strategy rules, same objective computation) before timing, (b) define explicit timing boundaries (cold vs warm, end-to-end vs compute-only), (c) require multi-trial statistics and noise awareness, and (d) publish “full disclosure” artifacts (scripts, configs, versions, hardware, reproduction steps) sufficient for an independent party to reproduce results—mirroring the disciplines used by established benchmark bodies. citeturn4view3turn3view3

For GPU timing, the benchmark must acknowledge that CUDA work is generally asynchronous with respect to the CPU host thread; without correct synchronization, a “wall-clock” timer can accidentally measure only kernel launch overhead rather than kernel execution. citeturn3view0turn3view2turn4view0

A README-grade disclosure model should be short but non-negotiably specific: what was measured, what was excluded, how warmup was handled (for both GPU runtime warmup and Python/Numba JIT), how many trials were run, which statistic is reported (median, min, dispersion), and which claims are explicitly out of scope (e.g., “GPU is always X faster”). citeturn10view0turn5view0turn9view0

A stricter internal policy should separate (1) “public comparison” scenarios that are frozen and audit-friendly from (2) “engineering microbenchmarks” (e.g., Criterion/variants) that track regressions. It should also enforce repeat/replicate/reproduce thinking (same system vs different software stack vs different hardware) and maintain a durable artifact trail. citeturn3view7turn5view1turn4view3

## Extracted question

From `VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md`, the seventh question (labeled **DR-07**) is:

> What is the most credible way to benchmark and publicly compare a GPU trading optimizer like VectorGrid against a CPU-oriented workflow like VectorBT? Define a benchmark charter covering warm versus cold runs, dataset loading boundaries, fairness rules, semantic alignment, disclosure wording, scenario selection, and what claims should be avoided. Recommend a README-grade disclosure model and a stricter internal benchmark policy for future comparisons. Keep the result focused on additive benchmarking discipline and disclosure improvements, not on changing the product to fit a benchmark. fileciteturn0file1

## Evidence base and transferable principles

Established benchmark ecosystems emphasize *comparability* and *reproducibility* over single-number reporting. For example, entity["organization","SPEC","benchmark consortium"]’s CPU2017 run rules explicitly frame published results as meaningful/comparable/reproducible only when accompanied by “full disclosure” of configuration and procedures sufficient for independent reproduction. citeturn4view3turn3view4

Similarly, entity["organization","MLCommons","mlperf org"] (the steward of MLPerf) states operating principles centered on “fair and useful benchmark results,” and explicitly reserves the right to exclude submissions judged deceptive or not of interest to the community—highlighting that benchmark governance is as much about preventing misleading comparisons as it is about measurement mechanics. citeturn3view3

For measurement methodology, modern benchmarking literature and tooling treats noise and heavy-tailed timing distributions as normal. The “robust benchmarking” approach from entity["organization","Massachusetts Institute of Technology","university cambridge ma"] researchers argues that timing distributions can be non-ideal due to timer error and OS jitter, motivating robust estimators and disciplined repetition. citeturn3view5turn6view3turn6view0 Meanwhile, Criterion’s analysis documentation (representative of statistically-aware benchmarking practice) emphasizes outlier detection, bootstrapped confidence intervals, and comparisons between runs rather than raw point estimates. citeturn5view1

Finally, GPU benchmarking introduces domain-specific pitfalls: entity["company","NVIDIA","gpu vendor"]’s CUDA best practices guide stresses that many CUDA API calls and all kernel launches are asynchronous with respect to the CPU, and that accurate CPU-side timing requires explicit synchronization; it also illustrates event-based GPU timing via `cudaEventRecord`/`cudaEventElapsedTime`. citeturn3view0turn4view1 The CUDA runtime API documents event timing resolution and semantics, and the CUDA programming guide describes stream-based asynchronous execution as a core model. citeturn4view0turn3view2

## Benchmark charter for VectorGrid vs VectorBT comparisons

A benchmark charter is a “constitution” for what counts as fair, what counts as measured, and what claims are permitted. The most credible public posture is to declare two layers: a **public-comparison charter** (frozen, disclosure-heavy, audit-friendly) and an **engineering charter** (more exhaustive, not necessarily public-facing). citeturn4view3turn3view3

### Charter scope and allowed claims

The charter should constrain the public comparison to claims that are defensible under full disclosure:

- **Allowed**: “Under scenario S (defined dataset, defined grid, defined objective, defined timing boundary), VectorGrid completed in X while VectorBT completed in Y, across N repeated trials; the median ratio is R; here is the reproduction package.” This aligns with the “results as observations + disclosure” philosophy in SPEC-style benchmarking. citeturn3view4turn4view3  
- **Disallowed**: generalized claims like “VectorGrid is R× faster than VectorBT” *without scenario qualifiers*, or “GPU makes backtesting R× faster in general.” Benchmark rule sets explicitly treat published results as conditional on methods and conditions, not universal truths. citeturn4view3turn3view4

### Semantic alignment requirements

Before timing, the charter should require a **semantic alignment checklist** and a **parity sanity artifact**:

- **Dataset identity alignment**: both tools must use the *same* dataset bytes (or a generator with a pinned seed and published hash). This matches the reproducibility intent emphasized in benchmark rule systems. citeturn4view3turn3view7  
- **Strategy and search-space equivalence**: same parameter grid, same constraints (e.g., whether `fast < slow` is enforced), and the same objective definition. MLPerf’s “semantic-level benchmark” approach is a useful model: specify the task and rules, but make the rules explicit and auditable. citeturn5view2  
- **Objective computation disclosure**: if metric values are not bit-identical, the benchmark should disclose that and describe why (e.g., floating-point differences, different intermediate rounding, different data type paths). SPEC explicitly distinguishes “results” from “estimates” and places weight on mature, disclosed methods. citeturn3view4turn4view3  

VectorGrid’s existing benchmark writeups already reflect parts of this discipline—e.g., distinguishing warm “app-path” timings from lower-level Criterion timings, and describing scenario boundaries and versions. fileciteturn0file5 This charter formalizes and generalizes those practices into a reusable framework.

### Timing boundaries and warm vs cold definitions

The most common way public comparisons become misleading is boundary mismatch: one side includes JIT compilation or dataset load while the other doesn’t. The charter should therefore define **at least two public timing boundaries**, and treat everything else as internal-only:

- **Cold start, end-to-end boundary**: Includes process start, imports/module load, GPU runtime initialization, dataset load/parse/validation, and first-run compilation (Python/Numba) where applicable. This measures “first run a new user experiences.” Python tooling explicitly distinguishes setup from timed statements (e.g., `timeit` excludes setup), which is a useful conceptual separation even when doing end-to-end timing. citeturn5view0  
- **Warm start, end-to-end boundary**: Dataset already loaded; JIT already compiled; GPU device selected; caches resident; then time only the repeated optimization run(s). This maps to real iterative research workflows and is commonly the most relevant boundary for “optimizer throughput.” GPU best practices and modern benchmark tooling both assume warmup and repeats are essential for stable comparisons. citeturn3view0turn9view0  

For CUDA specifically, the charter should state explicitly where synchronization occurs for timing. If you time on the CPU, you must synchronize around the region because kernel launches are asynchronous; if you time via CUDA events, you must ensure the events bracket the work in the relevant stream and synchronize appropriately for reading results. citeturn3view0turn4view1turn4view0

### Measurement protocol and statistics

The charter should require multi-trial testing and robust reporting:

- Run **warmup iterations** that are explicitly not included in the reported statistics, to reduce “first-run effects” (GPU warmup, Python/Numba JIT compilation, cache warming). citeturn3view0turn9view0turn10view0  
- Run **replicates** and report at least **median** plus a dispersion indicator (e.g., min/max or a confidence interval). entity["organization","PyTorch","ml framework"]’s benchmarking utilities explicitly recommend replicates, median computation, and handling warmups/synchronization for asynchronous accelerators—this is directly transferable guidance for GPU comparisons even outside PyTorch. citeturn9view0  
- Keep an explicit stance on which statistic is emphasized for which boundary:
  - For user-facing, end-to-end: median is typically most representative (and SPEC explicitly allows/reportable median practices in its run rules context). citeturn4view3  
  - For “best-case compute” tracking (internal), minimum-of-many can be defensible in the presence of positive noise/jitter (as argued in robust benchmarking work), but it should be labeled clearly as “best-case under minimal interference,” not “typical.” citeturn6view0turn6view3  

### Comparison table of benchmark boundaries

| Boundary (recommended naming) | Start condition | End condition | Captures | Best use | Common failure mode |
|---|---|---|---|---|---|
| Cold end-to-end | Fresh process; no caches assumed | Top-k/results available | Import/load/JIT/GPU init + compute | First-run user experience; “time to first result” | One side includes JIT/compilation and the other doesn’t (boundary mismatch) citeturn5view0turn10view0 |
| Warm end-to-end | Dataset loaded; JIT compiled; device selected | Top-k/results available | Orchestration + compute under realistic iterative workflow | Public speed claims most aligned with research iteration | Timing only GPU kernel while calling it “end-to-end” citeturn3view0turn4view1 |
| GPU compute-only (events) | Inputs resident where relevant | Kernel region complete | GPU execution time bracketed by events; high timer resolution | Internal kernel regression tracking; micro-optimization | Forgetting stream semantics / not bracketing the same work region citeturn4view1turn4view0turn3view2 |
| CPU compute-only (tight loop) | Inputs in memory | Loop complete | CPU compute excluding I/O | Internal CPU-path regressions | OS jitter/outliers misinterpreted as real changes citeturn3view5turn5view1 |

### Process overview diagram

```mermaid
flowchart TD
  A[Benchmark charter] --> B[Specify workload + search space]
  B --> C[Semantic alignment checklist]
  C --> D[Choose timing boundary: cold or warm]
  D --> E[Warmup protocol (not measured)]
  E --> F[Run N measured trials]
  F --> G[Summarize: median + dispersion]
  G --> H[Publish disclosure: scripts, configs, versions, hardware]
  H --> I[State limitations + claims to avoid]
```

## README-grade disclosure model for public comparisons

A public disclosure should be short enough for a README but strict enough to prevent misleading reuse. Benchmark rule systems summarize disclosure as “enough detail to independently reproduce results,” and MLPerf governance explicitly centers fairness and anti-deception. citeturn4view3turn3view3

A practical disclosure template:

**What we measured**  
We measured *(cold / warm)* end-to-end optimization time for scenario **S**, defined as: **start** = *(explicit event)*, **end** = *(explicit event)*. We ran **N** measured trials after **W** warmup runs and report **median** plus *(min/max or CI).* citeturn4view3turn9view0

**What we did not measure**  
We did not claim GPU-kernel-only timings as end-to-end, and we do not generalize results beyond scenario S. CUDA kernel launches are asynchronous on the host; any CPU-wall timing includes explicit synchronization points described below. citeturn3view0turn3view2turn4view1

**Fairness and semantic alignment**  
Both tools used the same dataset bytes (hash published) and the same parameter grid and objective definition. We include a parity sanity check: top parameters and objective values are reported; if values differ, we describe why. citeturn4view3turn5view2

**Environment**  
CPU model, GPU model, RAM, OS, versions of VectorGrid, VectorBT, Python, and key numeric/JIT dependencies; plus any non-default settings that affect performance (thread counts, GPU clocks/power limits if fixed). “Full disclosure” rules in established benchmarks treat these as required for reproducibility. citeturn4view3turn3view4

**Reproduction package**  
A single command (or small set of commands) that regenerates the dataset and re-runs the benchmark; plus pinned dependency versions and scripts in-repo. Artifact evaluation standards treat functional, reusable artifacts and durable availability as key steps toward reproducibility. citeturn3view6turn3view7

**Claims we avoid**  
We avoid “VectorGrid is X× faster than VectorBT” without the scenario label and boundary definition; we avoid implying that this benchmark represents all strategies or all data regimes; and we avoid implying that users will always see the minimum time as “typical.” citeturn4view3turn6view0

This disclosure model also aligns with VectorGrid’s existing practice of clearly labeling “warm app-path” numbers versus microbenchmarks and documenting scenario specifics and versions. fileciteturn0file5

## Stricter internal benchmark policy for future comparisons

An internal policy should be stricter than the README: it should reduce accidental benchmark drift, make regressions obvious, and prevent “silent redefinition” of timing boundaries.

### Policy structure

**Separate public benchmark tracks from engineering microbenches**

- **Public track (frozen)**: scenario files, fixed datasets or deterministic generators, fixed boundaries, and a “closed division” rule: do not modify the benchmarked algorithm beyond allowed configuration toggles. The MLPerf “closed vs open” division concept is highly transferable: closed = strict comparability; open = exploratory changes (still publishable, but clearly labeled). citeturn5view2turn3view3  
- **Engineering track (evolvable)**: Criterion-style and fine-grained kernel timing (CUDA events) for regression detection; results are for developers, not marketing. Criterion’s model—outlier awareness, bootstrapped confidence intervals, and run-over-run comparisons—is appropriate here. citeturn5view1turn0search7  

**Require a “benchmark bill of materials” (BOM)**  
Inspired by “full disclosure” norms: every internal benchmark run that is used for decisions should produce a machine-readable BOM (hardware, drivers, versions, settings). SPEC’s disclosure rules treat “all non-default settings” and reproduction steps as part of the benchmark’s credibility. citeturn4view3turn3view4

**Codify repeat / replicate / reproduce**

Adopt the HPC benchmarking framing:  
- *Repeat* (same system), *replicate* (different software stack), *reproduce* (different system). This gives a disciplined vocabulary for “is the win real, or an artifact of one machine/config?” citeturn3view7

### GPU- and Python-specific internal enforcement

**CUDA timing enforcement**  
Internal tooling should enforce correct timing primitives. For CUDA, that means explicitly documenting whether a measurement is CPU-wall time (with `cudaDeviceSynchronize()` bracketing) or event-based GPU time; NVIDIA’s best practices guide warns that kernel launches are asynchronous and requires synchronization for accurate CPU timing. citeturn3view0turn4view1turn4view0

**JIT/Numba warmup awareness**  
If the VectorBT side uses Numba, the internal policy should require either:
- a “cold” measurement that *includes* compilation, or  
- a “warm” measurement that explicitly *excludes* compilation by performing warmup calls (and/or using caching mechanisms where appropriate).

Numba’s documentation explicitly describes file-based caching via `cache=True` to avoid compilation times between runs (with known limitations), and VectorBT documentation emphasizes reliance on NumPy/Numba for performance and the broader “jitting” acceleration concept. citeturn10view0turn10view1turn10view3turn10view2

**Noise and regression thresholds**  
Use a consistent rule for what counts as a real change. Robust benchmarking research cautions that timing distributions can be non-normal/heavy-tailed due to jitter, motivating robust estimators; Criterion similarly includes noise thresholds and outlier warnings. citeturn6view3turn5view1

## Assumptions and unspecified details

The DR-07 question does not specify several implementation details; the recommendations above assume:

- The benchmark is intended to compare **equivalent semantic workloads** (same dataset, same exact-grid search space, same objective definition), not “best effort” with divergent strategy semantics. citeturn5view2turn4view3  
- Both tools can be configured to run **multiple trials** with a warmup phase and report robust statistics, rather than relying on a single observed run time. citeturn9view0turn5view1  
- The comparison will include at least **two boundaries** (cold end-to-end and warm end-to-end). If only one boundary is allowed, the disclosure must be even more explicit about what is included/excluded. citeturn4view3turn3view0  
- The hardware/OS environment is controllable enough to reduce extreme variance (background load, power/thermal throttling), and any remaining variance will be reported rather than hidden. citeturn6view3turn3view7  
- VectorBT performance depends materially on its acceleration stack (NumPy/Numba and, in PRO, jitting abstractions); therefore, the benchmark must specify whether compilation/caching is included as part of “cold” or excluded as part of “warm.” citeturn10view3turn10view2turn10view0  

## Prioritized sources

- entity["organization","SPEC","benchmark consortium"] CPU2017 Run & Reporting Rules (fairness + full disclosure requirements; reproducibility posture). citeturn4view3turn3view4  
- entity["company","NVIDIA","gpu vendor"] CUDA C++ Best Practices Guide (timing pitfalls; asynchrony; CPU sync; CUDA event timing example). citeturn3view0turn4view1  
- CUDA Runtime API Event Management (event functions; `cudaEventElapsedTime` semantics and resolution). citeturn4view0  
- entity["organization","MLCommons","mlperf org"] MLPerf policies submission rules (operating principles; fairness and anti-deception governance). citeturn3view3  
- MLPerf Inference Benchmark paper (closed vs open divisions; scenario-driven evaluation; benchmark methodology framing). citeturn5view2  
- entity["organization","PyTorch","ml framework"] `torch.utils.benchmark` documentation (warmups, synchronization, replicates, median robustness). citeturn9view0  
- Chen, Revels, Edelman: “Robust benchmarking in noisy environments” (noise/jitter; robust estimation arguments). citeturn3view5turn6view0turn6view3  
- Criterion.rs analysis documentation (outliers; bootstrapping; confidence intervals; run comparisons). citeturn5view1  
- Python `timeit` documentation (repeatable measurement patterns; setup excluded from timing; repeat semantics). citeturn5view0  
- ACM Artifact Review and Badging policy (replicability and artifact discipline as part of experimental credibility). citeturn3view6