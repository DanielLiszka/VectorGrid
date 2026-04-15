# Second-Pass Review: `src-tauri/src/ma_registry.rs`

- Review date: `2026-04-14`
- Source file: `src-tauri/src/ma_registry.rs`
- First-pass packet: `Repo Review/backend-file-reviews/src-tauri/src/ma_registry.rs.review.md`
- First-pass action bucket: `Small surgical refactor`
- Second-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## First-Pass Recap

The first pass already treated this registry projection as mostly sound but contract-sensitive because frontend consumers depend on its serialized field set and policy overlays.

## Stale Check

The live source still exposes:

- moving-average catalog projection
- nullable notes field
- app-policy CUDA kernel flags layered over the upstream registry

The first-pass packet remains current.

## Second-Pass Research Log

1. Tauri v2 command guidance
   - Source: <https://v2.tauri.app/develop/calling-rust/>
   - Why consulted in second pass:
     - verify the serialization-contract sensitivity of the returned catalog DTO
   - Additional constraint informed:
     - field presence and nullability remain part of the public command contract

2. Serde field attributes
   - Source: <https://serde.rs/field-attrs.html>
   - Why consulted in second pass:
     - confirm omission vs explicit-null behavior
   - Additional constraint informed:
     - `notes` currently serializes as explicit `null`, not omission

## Delta vs First Pass

### 1. `supports_cuda_kernel` and `prefers_cuda_kernel` are authoritative app-policy overlays, not derivable implications

- Why this matters:
  - downstream consumers should not treat them as redundant mirror fields
- Practical implication:
  - preserve them as independent policy signals unless the policy model is intentionally redesigned

### 2. `notes` remains a nullable field contract, not an absent-only field

- Practical implication:
  - frontend null handling must remain in place if this surface changes

## Verification If Changed

- `cargo test -p ta_desktop_demo_app commands_data -- --nocapture`
- `cargo test -p ta_app_core ma_policy -- --nocapture`
- `npm --prefix ./web run test -- src/services/movingAverageCatalogService.test.ts src/services/optimizeService.test.ts`

## Regression And Feature-Preservation Constraints

- Preserve current field names and nullability behavior
- Preserve the kernel-policy overlay flags as authoritative app policy
- Do not collapse policy overlays back into upstream-registry assumptions

## Second-Pass Conclusion

`Bucket unchanged`

The first-pass `Small surgical refactor` bucket remains correct. The second pass mainly clarifies that nullable `notes` and the two CUDA kernel policy flags are deliberate serialized contract, not redundant projection noise.
