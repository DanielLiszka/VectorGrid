# Review Packet: `src-tauri/src/ma_registry.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/ma_registry.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Lean and useful catalog adapter, but not yet well-specified enough to treat as a stable frontend capability contract without a direct projection test and clearer flag semantics`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `Public-surface-sensitive`, `Frontend-contract-sensitive`, `Capability-matrix-sensitive`

## What This File Owns

`ma_registry.rs` owns the desktop crate's moving-average catalog DTO and the projection from upstream registry/policy sources into that DTO.

This file owns:

- the serialized moving-average catalog row shape:
  - `MaInfo`: `src-tauri/src/ma_registry.rs:5-19`
- the registry-to-DTO projection:
  - `list_mas`: `21-40`

This file does not own:

- the Tauri command wrapper
  - that lives in `src-tauri/src/commands_data.rs:80-83`
- the underlying moving-average registry
  - that lives in `vector_ta::...::registry::list_moving_averages()`
- GPU-kernel capability policy
  - that lives in `crates/app_core/src/ma_policy.rs`
- frontend normalization/caching
  - that lives in `web/src/services/movingAverageCatalogService.ts`

## Classification

- Public surface: yes
  - serialized backend catalog contract consumed by the frontend
- Hot path: no
  - low-frequency catalog lookup
- Runtime-critical: low to moderate
  - not computationally heavy, but capability flags influence UI choices and backend-route expectations
- CUDA/runtime-critical: indirect only
  - exposes kernel/policy metadata, but does not execute CUDA work
- Strategy-local: no
- Low-risk support code: yes, with contract sensitivity

## Direct Interactions

### Backend interaction map

- Tauri command wrapper:
  - `src-tauri/src/commands_data.rs:80-83`
- GPU-kernel policy lookups used in this adapter:
  - `crates/app_core/src/ma_policy.rs:239-258`
- upstream registry/policy tests exist in `ta_app_core`:
  - `crates/app_core/src/ma_policy.rs:362-470`

### Frontend contract anchors

- frontend catalog model and normalizer:
  - `web/src/services/movingAverageCatalogService.ts:16-29`
  - `web/src/services/movingAverageCatalogService.ts:101-152`
- frontend tests for shape normalization and caching:
  - `web/src/services/movingAverageCatalogService.test.ts:21-175`
- command allowlist includes `list_moving_averages`:
  - `web/src/tauri/commands.ts:105`

## Standards And Research Log

This packet was written after targeted external review focused on Rust/Tauri serialized API shape and public DTO clarity.

### Sources consulted

1. Tauri calling Rust from the frontend
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why it was consulted:
     - this DTO is serialized across the Tauri boundary
   - Standard informed:
     - returned data structures are part of the frontend API contract and should stay explicit and stable

2. Rust API Guidelines checklist
   - Source: <https://rust-lang.github.io/api-guidelines/checklist.html>
   - Why it was consulted:
     - this is a small public contract adapter
   - Standard informed:
     - naming, clarity, and representative tests matter even for thin adapter modules

3. Serde serialization guidance
   - Source: <https://serde.rs/>
   - Why it was consulted:
     - this file's output is defined entirely by `Serialize`
   - Standard informed:
     - field names and optional/null behavior become part of the effective frontend schema

## Findings

## Correctness And Safety

### 1. No file-local safety or runtime bug stands out, but the adapter merges multiple upstream sources without a direct projection test

- Evidence:
  - base MA metadata comes from the VectorTA registry: `22-29`
  - kernel support/preference flags come from `ta_app_core::ma_policy`: `30-31`
  - parameter schema comes from a separate lookup: `35-36`
  - frontend consumers trust the merged capability matrix directly:
    - `web/src/services/movingAverageCatalogService.ts:109-141`
- Why it matters:
  - if upstream registry metadata and app policy drift, the frontend can receive a capability combination that is technically serializable but semantically wrong
- Smallest justified fix:
  - add a direct backend projection test for representative invariants in `list_mas()`

## API And Naming

### 1. The meaning of the GPU-related flags is not documented where the projection happens

- Evidence:
  - the DTO exposes:
    - `supports_cuda`
    - `supports_cuda_ma_sweep`
    - `supports_cuda_kernel`
    - `prefers_cuda_kernel`
  - refs: `src-tauri/src/ma_registry.rs:9-13`
- Why it matters:
  - these fields are close enough in meaning that downstream readers can easily misinterpret them without checking both VectorTA registry rules and app-core policy
- Smallest justified fix:
  - add short field comments or a module-level note describing how the flags differ

### 2. Ordering semantics are implicit

- Evidence:
  - `list_mas()` preserves upstream iteration order and does not sort: `21-40`
  - frontend caching preserves received order: `web/src/services/movingAverageCatalogService.ts:147-152`
- Why it matters:
  - UI ordering will drift if the upstream registry order changes
- Review judgment:
  - low severity, but it should be either documented or deliberately sorted

## Error Handling

### 1. The adapter is intentionally infallible

- Evidence:
  - `list_mas()` returns `Vec<MaInfo>` directly with no `Result`
- Why it matters:
  - there is no file-local error-handling defect to fix here
  - the review focus is contract clarity and projection verification

## Module Boundaries And Technical Debt

### 1. This file is the only place where registry metadata and app policy are merged into one frontend-facing capability row

- Evidence:
  - VectorTA registry fields and app-core policy fields are stitched together in `24-37`
- Why it matters:
  - this is exactly the layer where compatibility drift should be tested
- Review judgment:
  - the module boundary is correct; it just needs stronger contract pinning

## Docs And Examples

### 1. The adapter needs a short contract note for frontend consumers

- Missing documentation topics:
  - what each capability field means
  - whether ordering is intentional
  - that `params` come from the MA parameter schema, not the registry row directly

## Tests And Missing Verification

### 1. Backend coverage exists, but not at the exact projection layer

- Verified in this review pass:
  - `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
    - passed `19` tests
  - relevant direct backend test:
    - `src-tauri/src/commands_data.rs:1852-1866`
    - only checks non-empty, unique IDs
  - `cargo test -p ta_app_core ma_policy -- --nocapture`
    - passed `9` tests
    - policy behavior is covered in `crates/app_core/src/ma_policy.rs:362-470`

### 2. Frontend coverage is normalization-focused, not backend-projection-focused

- `npm --prefix ./web run test -- src/services/movingAverageCatalogService.test.ts`
  - passed `3` tests
- those tests validate:
  - payload normalization
  - boolean coercion
  - client-side caching
- they use mocked payloads rather than the real backend projection:
  - `web/src/services/movingAverageCatalogService.test.ts:9-13`

### 3. Missing verification

- no direct backend test that:
  - `prefers_cuda_kernel` never contradicts `supports_cuda_kernel`
  - parameter schema rows are present for representative MA IDs
  - ordering is stable or intentionally sorted
- no end-to-end test that exercises the real serialized DTO through `list_moving_averages`

## Performance, Allocation, And CUDA Concerns

### 1. No meaningful performance issue exists here

- single registry projection, low frequency, no allocation pressure worth optimizing

## Cross-File Interaction Map

### Most likely coordination points if this file changes

- `src-tauri/src/commands_data.rs`
- `crates/app_core/src/ma_policy.rs`
- `web/src/services/movingAverageCatalogService.ts`
- `web/src/services/optimizeService.ts`

### Downstream consumers needing verification

- moving-average picker/catalog UI
- optimize-service MA capability summaries
- any backend logic that assumes catalog flags are coherent with app-core routing policy

## Code-Quality Improvement Register

### Safe now

- add a small backend projection test for representative flag invariants
- add short comments documenting flag meanings
- decide whether ordering should be documented or explicitly sorted

### Coordination required

- changing the semantics of any capability flag
- changing whether kernel support/preference is derived from app-core policy vs upstream registry

### Defer until later pass

- any broader catalog redesign spanning frontend and backend

## Technical-Debt Reduction Register

### Debt safe to reduce now

- missing documentation for flag semantics
- missing projection-specific backend tests

### Debt that spans multiple files

- keeping VectorTA registry semantics and app-core kernel policy aligned in the serialized catalog

### Debt that should not be “cleaned up” casually

- the split between upstream registry metadata and app-local GPU-kernel policy

## Performance Opportunity Register

- Safe structural opportunities:
  - none important
- Benchmark-sensitive opportunities:
  - none
- Not the right place to chase performance:
  - yes

## Observability And Diagnostics Gaps

- no direct signal if the merged capability matrix becomes semantically inconsistent

## Open Questions And Assumptions

- assumption:
  - preserving upstream registry order is acceptable unless the product wants a curated UI order
- open question:
  - whether the repo wants the MA catalog contract to be explicitly stable for frontend consumers, or only stable enough for current workspace use

## Regression Risk And Feature-Preservation Constraints

- Do not change the meaning of `supports_cuda*` / `prefers_cuda_kernel` casually; frontend capability summaries depend on them.
- Do not sort or reorder the catalog without checking whether any UI intentionally relies on upstream order.

## Exact Local Verification Required If Changed

1. `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
2. `cargo test -p ta_app_core ma_policy -- --nocapture`
3. `npm --prefix ./web run test -- src/services/movingAverageCatalogService.test.ts src/services/optimizeService.test.ts`

## Recommended Action

`Small surgical refactor`

This file does not need a redesign. It needs one direct backend projection test and a small amount of documentation so the frontend-facing capability matrix remains trustworthy as the upstream registry and app-local GPU policy evolve.
