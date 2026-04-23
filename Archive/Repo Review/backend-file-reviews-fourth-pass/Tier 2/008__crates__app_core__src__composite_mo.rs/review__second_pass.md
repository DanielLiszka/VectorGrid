# Second-Pass Review: `crates/app_core/src/composite_mo.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/composite_mo.rs`
- First-pass packet: `Repo Review/backend-file-reviews/crates/app_core/src/composite_mo.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this file as a route- and telemetry-sensitive optimization surface whose sampler contract and GPU prescan diagnostics should remain explicit.

## Stale Check

The live source still owns:

- the public `CompositeMoSampler` enum
- prescan telemetry and device-resolution reporting
- multi-pair fallback summaries

The first-pass packet remains current.

## Second-Pass Research Log

1. Serde enum representations
   - Source: <https://serde.rs/enum-representations.html>
   - Why consulted in second pass:
     - verify the exact wire contract for the sampler enum
   - Additional constraint informed:
     - tag spellings are the Rust variant spellings exactly

2. Serde container attributes
   - Source: <https://serde.rs/container-attrs>
   - Why consulted in second pass:
     - confirm enum tagging behavior and compatibility implications
   - Additional constraint informed:
     - changing tag case or naming would be a public contract break

3. NVIDIA multi-GPU guide
   - Source: <https://docs.nvidia.com/cuda/cuda-programming-guide/03-advanced/multi-gpu-systems.html>
   - Why consulted in second pass:
     - reassess the importance of explicit device-selection diagnostics
   - Additional constraint informed:
     - telemetry should not look more certain than actual device resolution

## Delta vs First Pass

### 1. `CompositeMoSampler` has a stricter wire contract than the first pass emphasized

- Why this matters:
  - accepted tag spellings are serialized API, not internal enum style
- Practical implication:
  - keep sampler spelling stable unless compatibility aliases are added

### 2. Prescan telemetry can still imply stronger device certainty than the resolution path actually has

- Why this matters:
  - `resolved_device_id: None` can coexist with fallback execution via placeholder device selection
- Practical implication:
  - if telemetry is refined, prefer explicit uncertainty over optimistic labeling

## Verification If Changed

- serde decode tests for sampler tag spellings
- a prescan telemetry test where no preferred device resolves but fallback execution still happens

## Regression And Feature-Preservation Constraints

- Preserve accepted sampler tag spellings unless versioned or aliased
- Preserve current `require_gpu` hard-fail behavior
- Preserve current prescan narrowing behavior across multiple MA pairs

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly tightens two points: sampler tag spellings are a real public wire contract, and current prescan telemetry can still overstate device certainty.
