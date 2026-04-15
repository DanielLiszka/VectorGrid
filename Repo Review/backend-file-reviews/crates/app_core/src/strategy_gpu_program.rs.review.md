# Review Packet: `crates/app_core/src/strategy_gpu_program.rs`

- Review date: `2026-04-14`
- Source file: `crates/app_core/src/strategy_gpu_program.rs`
- Packet status: `Reviewed`
- Public-review readiness: `Important GPU-contract module, but not fully review-ready because ABI invariants, fingerprint-stability assumptions, and version-bump rules are only implicit`
- Recommended action bucket: `Small surgical refactor`
- Sensitivity tags: `GPU-ABI`, `Unsafe`, `Serialization-contract`, `Semantics-sensitive`

## What This File Owns

`strategy_gpu_program.rs` owns the translation from `CompiledStrategyPlan` into a GPU-executable predicate program and its compact ABI representation:

- JSON-facing compiled program structs and enums:
  - `12-69`
- C-compatible ABI structs:
  - `71-120`
- CUDA `DeviceCopy` unsafe impls for ABI structs:
  - `122-130`
- fingerprint helpers:
  - `182-207`
- ABI compilation:
  - `209-275`
- predicate-program compilation and shape validation:
  - `300-432`
- file-local tests:
  - `434-603`

## Classification

- Public surface: effectively yes
  - consumed by `strategy_gpu_runtime.rs`
  - serialized program/ABI structs are part of internal runtime contracts
- Hot path: build-time/runtime-setup only
- CUDA/runtime-critical: yes
- Unsafe/FFI-adjacent: yes
  - `#[repr(C)]` plus `unsafe impl DeviceCopy`
- Low-risk support code: no
  - ABI mistakes here would be high-impact

## Direct Interactions

### Backend interaction map

- runtime caching and upload:
  - `crates/app_core/src/strategy_gpu_runtime.rs:1-355`
- plan compilation source:
  - `crates/app_core/src/strategy_plan.rs:195-317`
- kernel upload / probing path:
  - `crates/app_core/src/gpu_backtest_kernel.rs`

### Downstream verification impact if changed

- changing enum encodings or struct layout breaks kernel-side interpretation
- changing executable fingerprint inputs changes runtime cache keys
- changing validation rules alters which compiled plans can reach the GPU upload path

## Standards And Research Log

This packet was written after targeted web research focused on Rust layout guarantees, CUDA copy-safety requirements, and hash-stability constraints.

### Sources consulted

1. Rust Reference, type layout
   - Source: <https://doc.rust-lang.org/reference/type-layout.html>
   - Why it was consulted:
     - this file relies on `#[repr(C)]` layout assumptions for ABI structs
   - Standard informed:
     - ABI-facing structs need explicit and durable layout reasoning

2. `cust::memory::DeviceCopy`
   - Source: <https://docs.rs/cust/latest/cust/memory/trait.DeviceCopy.html>
   - Why it was consulted:
     - the file contains `unsafe impl DeviceCopy` for several ABI structs
   - Standard informed:
     - only plain-data ABI structs with safe bitwise copy semantics should implement `DeviceCopy`

3. `std::hash::DefaultHasher`
   - Source: <https://doc.rust-lang.org/std/hash/struct.DefaultHasher.html>
   - Why it was consulted:
     - fingerprint helpers use `DefaultHasher` and expose hex strings through serialized structs
   - Standard informed:
     - the hashing algorithm is intentionally unspecified, so cross-build stability should not be assumed implicitly

## Findings

## Correctness And Safety

### 1. The `unsafe impl DeviceCopy` blocks lack adjacent safety justification

- Evidence:
  - `unsafe impl cust::memory::DeviceCopy` appears for all ABI structs at `122-130`
  - the underlying structs are `#[repr(C)]`, copyable numeric fields only: `71-120`
- Why this matters:
  - the implementations are likely valid, but the proof currently lives only in the reader’s head
  - future field additions could silently invalidate the safety basis
- Smallest justified fix:
  - add short safety comments next to each `unsafe impl` or one shared comment documenting why these structs remain POD-safe for device copies

## API And ABI Contracts

### 2. Schema version, ABI version, and executable fingerprint rules are not documented where they are defined

- Evidence:
  - schema/version constants live at `8-10`
  - executable fingerprint input is defined inline inside `fingerprint_gpu_predicate_program_executable`: `186-207`
- Why this matters:
  - this file is the canonical place a maintainer will touch when changing GPU predicate shape
  - without local rules, it is easy to mutate a field and forget whether schema, ABI version, or both should change
- Smallest justified fix:
  - add rustdoc explaining:
    - when to bump `STRATEGY_GPU_PROGRAM_SCHEMA_VERSION`
    - when to bump `STRATEGY_GPU_EXECUTABLE_ABI_VERSION`
    - which fields intentionally participate in executable fingerprints

## Fingerprinting And Stability

### 3. Fingerprint hex strings currently look durable, but `DefaultHasher` does not promise stable output across builds

- Evidence:
  - `fingerprint_serialized` uses `DefaultHasher`: `182-187`
  - `plan_fingerprint_hex`, `compiled_plan_fingerprint_hex`, and `executable_fingerprint_hex` are stored on serialized structs: `56-69`, `106-120`
- Why this matters:
  - if these values are ever persisted, compared across binaries, or treated as long-term identifiers, the stability contract is ambiguous
  - the current implementation is appropriate for in-build cache keys, but that intent is not stated
- Smallest justified fix:
  - document these fingerprints as runtime/build-local identifiers unless and until a stable hash is chosen deliberately
  - defer any hash-algorithm migration until runtime cache compatibility is reviewed end to end

## Validation And Error Handling

### 4. The file validates index bounds well, but the invariants are only enforced procedurally

- Evidence:
  - indicator-index, condition-index, and group-binding checks exist in `compile_operand`, `validate_group_binding`, and `compile_gpu_predicate_program`: `277-432`
- Why this matters:
  - the logic is sound, but the ABI layer has no top-level “all invariants this compiler guarantees” summary
  - that raises review cost for kernel/runtime maintainers
- Smallest justified fix:
  - add a short module comment summarizing the invariants guaranteed by compilation

## Tests And Missing Verification

### 5. Tests are good on shape preservation, but there is no explicit safety-regression tripwire for ABI structs

- Evidence:
  - tests cover smoke, out-of-range rejection, ABI-shape preservation, and fingerprint reuse: `434-603`
- Missing coverage:
  - explicit layout-focused assertions for ABI structs
  - a documented review rule preventing non-POD fields from entering `DeviceCopy` structs
- Smallest justified fix:
  - add a layout/safety-focused test or comment gate before touching ABI struct fields

## Code-Quality Improvement Register

- Safe now:
  - add safety comments for `DeviceCopy`
  - document version-bump rules
  - document fingerprint scope/stability limits
- Defer:
  - replacing the hashing algorithm
  - broad ABI redesign

## Technical-Debt Reduction Register

- Safe now:
  - local contract documentation
  - explicit unsafe justification
- Defer:
  - stable-hash migration
  - kernel/ABI shape refactors
- Do not casually remove:
  - the existing index-range checks
  - the current fingerprint inputs without reviewing runtime cache implications

## Performance Opportunity Register

- Safe structural work:
  - none needed
- Benchmark-sensitive:
  - changing executable fingerprint composition or ABI shape

## Recommended Action

`Small surgical refactor`

The file is doing the right work. The missing piece is contract clarity: unsafe justification, ABI/version documentation, and an explicit statement that the fingerprint strings are not automatically durable identifiers across builds.

## Exact Local Verification Required If Changed

1. File-local tests:
   - `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`
2. Runtime integration surface:
   - `cargo test -p ta_app_core strategy_gpu_runtime::tests:: -- --nocapture`
3. Representative GPU parity path:
   - `cargo test -p ta_app_core gpu_backtest_kernel -- --nocapture`

### Verification run for this packet

- Passed:
  - `cargo test -p ta_app_core strategy_gpu_program::tests:: -- --nocapture`

## Regression Risk And Feature-Preservation Constraints

- Do not change numeric enum encodings or struct fields without reviewing the kernel-side ABI consumer
- Do not replace `DefaultHasher` casually; cache keys and downstream metadata surfaces depend on the current fingerprint shape
- Do not add non-POD fields to ABI structs while keeping `DeviceCopy`
