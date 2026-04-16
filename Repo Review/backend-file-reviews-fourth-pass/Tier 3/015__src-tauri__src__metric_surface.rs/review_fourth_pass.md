# Fourth-Pass Review: src-tauri/src/metric_surface.rs

- Review date: `2026-04-16`
- Source file: `src-tauri/src/metric_surface.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

This file needed a fourth pass because the current third-pass packet is materially mis-scoped to the wrong subsystem.

The live bundle shows `metric_surface.rs` as a private support module nested under `commands.rs`, not a registered Tauri command seam: it is declared as `mod metric_surface;` in `src-tauri/src/commands.rs:31-32`, while `src-tauri/src/lib.rs:82-173` registers many invoke-handler commands and does not expose any `metric_surface` command at all. The file’s actual work is artifact-sidecar inference and projection: route/strategy fallback detection, current-metadata classification, metric-row projection, and sidecar construction in `src-tauri/src/metric_surface.rs:118-463`.

So the third-pass packet’s discussion of public IPC wrappers, command registration, runtime authority, cancellation scope, event identity, and `spawn_blocking` ownership is not evidence-backed for this source file. What *is* still correct is the conservative instinct behind its bucket call: this remains a contract-sensitive compatibility helper, not a good target for a broad rewrite.

That means the current third-pass **bucket should stand, but its rationale should not**. The file still belongs in `Small surgical refactor`, just for persisted artifact compatibility reasons rather than IPC-wrapper reasons.

## Fourth-Pass Research Log

This pass re-read the live source first, then checked the local crate context to confirm whether the file is actually an IPC boundary or an internal compatibility seam. After that, it did a targeted doc pass only where the earlier packets made claims that depended on `serde_json` behavior.

1. `serde_json::to_value`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/fn.to_value.html>
   - Why consulted:
     - the earlier packet treated `serde_json::to_value(...).ok()` in `build_optimization_metric_surface_artifact_value()` as a meaningful silent-failure surface.
   - Constraint informed:
     - `to_value` fails when serialization itself fails or when maps have non-string keys. For this file’s derived sidecar struct, that makes the `.ok()` path much less concerning than the earlier packet suggested.

2. `serde_json::value::Serializer` source
   - Source: <https://docs.rs/serde_json/latest/src/serde_json/value/ser.rs.html>
   - Why consulted:
     - to verify what happens when this sidecar serializes floating-point metric rows.
   - Constraint informed:
     - `serialize_f64` returns `Ok(Value::from(float))`, so this serializer does not error on non-finite floats in the way the earlier packet implicitly assumed.

3. `serde_json::Value` conversion for `f64`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why consulted:
     - to confirm the exact JSON representation of non-finite floats.
   - Constraint informed:
     - `Value::from(f64)` produces `Value::Null` for `NaN` or infinity. In this file, that means non-finite metric cells become JSON `null` entries inside `best_metrics_row` / `top_metric_rows`, not serialization failure.

4. `serde_json::Value::pointer`
   - Source: <https://docs.rs/serde_json/latest/serde_json/value/enum.Value.html>
   - Why consulted:
     - the file’s fallback logic is driven almost entirely by JSON Pointer lookups.
   - Constraint informed:
     - missing paths cleanly return `None`, which this module then normalizes into empty strings / `false` / absence-driven fallbacks. That makes path absence a first-class part of the compatibility contract here, not an exceptional case.

One important scope note: the first-pass packet made several downstream claims about report, hydration, and meta callers outside this bundle. Those may still be right, but they were not re-proven here because the actual `run_artifacts`, `commands_meta`, and `commands_reports` source files were not included in this fourth-pass bundle.

## What Fourth Pass Sharpened

### What the current third-pass packet got right

The current third-pass packet got one high-level judgment right: this file does **not** justify a large redesign. The live code is still a narrow compatibility layer, and the safest work remains local, additive, and test-driven.

That conservative bucket call is worth keeping.

### What the current third-pass packet missed

The packet missed the file’s real contract surface entirely.

`metric_surface.rs` is not a public command wrapper. Its real risk is persisted artifact interpretation. The core behaviors are all file-local and visible in the source:

- runtime hint parsing in `metric_surface.rs:181-191`
- exact-GPU fallback inference in `metric_surface.rs:233-275`
- broad strategy/route fallback in `metric_surface.rs:208-230` and `339-356`
- current-schema sidecar classification in `metric_surface.rs:368-384`
- row projection in `metric_surface.rs:278-337`
- sidecar rebuild in `metric_surface.rs:416-463`

Because the packet looked at the wrong abstraction, its verification guidance was also off. Manual desktop smoke for a “command path,” command-registration checks, and event/cancellation review are not the right proving surface for this module.

### What earlier passes underweighted, and what remains the most important safe-now hardening

The most important live issue is still the **precedence split between classification and rebuild**.

`artifact_metric_surface_kind()` honors current schema-2 top-level sidecar metadata when it exists and parses cleanly (`metric_surface.rs:375-384`). But `build_optimization_metric_surface_artifact()` does **not** reuse that decision; it rebuilds from `detect_surface_kind()` directly (`metric_surface.rs:416-419`), which only looks at runtime hints and heuristics (`metric_surface.rs:339-356`).

That means the module currently has two different notions of “effective surface”:

- one for *reading/summarizing* artifacts
- another for *rebuilding* artifacts

The second-pass packet noticed the practical risk here, and the fourth pass confirms it from the live source: if an artifact already has a valid current sidecar but the raw `run` payload no longer carries the same amount of route evidence, rebuild can reinterpret the artifact more conservatively than the reader path.

The safe-now hardening is narrow:

- add a dedicated helper for rebuild precedence that prefers:
  1. `/run/res/metric_surface_kind`
  2. valid current `optimization_metric_surface.surface`
  3. exact-GPU heuristics
  4. broad strategy fallback

That is deliberately narrower than swapping the builder over to `artifact_metric_surface_kind()`. Doing that blindly would be regression-prone because `artifact_metric_surface_kind()` currently lets persisted sidecar metadata outrank runtime hints whenever schema version 2 is present. For rebuild, runtime should remain the strongest source of truth.

### What the earlier review history likely overstated

The first-pass packet’s error-handling section was directionally right about `parse_metrics()` silently dropping malformed rows (`metric_surface.rs:410-413`), but it was too aggressive about `build_optimization_metric_surface_artifact_value()`.

The source keeps `serde_json::to_value(...).ok()` at `metric_surface.rs:461-463`, but for this particular struct that looks close to dead defensive code rather than a meaningful failure channel. The sidecar is a derived `Serialize` struct with strings, vectors, and options — no custom fallible serializer and no non-string-key maps. The more realistic edge case is not “serialization failed and the sidecar vanished,” but “row cells were non-finite and therefore serialized as JSON `null`.”

That is an important difference because the safest hardening changes:

- **weak recommendation**: add logging for a near-impossible `to_value` failure
- **stronger recommendation**: add an explicit regression test and local comment for non-finite metric cells in row payloads

### What the file still misses

There are still several concrete gaps that the current third-pass packet never got near.

First, the tests still miss at least one live branch that matters. `RegimeHybrid` is included in `strategy_kind_supports_exact_gpu_metric_surface()` at `metric_surface.rs:233-247`, but there is no direct `RegimeHybrid` test in the file. For a compatibility module, an untested admitted branch is exactly the kind of omission that later turns into stale fallback logic.

Second, accepted current-schema aliases are not directly locked by tests. `parse_surface_label()` accepts `core17`, `core_17`, `core-17`, `trade22`, `trade_22`, and `trade-22` at `metric_surface.rs:359-365`, but the test matrix only exercises canonical labels. That is a smaller issue than the precedence split, but it is still a real contract gap.

Third, the current-schema validity check remains very shallow. `has_current_metric_surface_metadata()` only checks `schema_version == 2` (`metric_surface.rs:368-373`). That means malformed current blobs are treated as “current metadata exists” for summary purposes even if the `surface` string is bad and the reader has to fall back anyway (`metric_surface.rs:375-384`, `387-407`). The first-pass packet already noticed this; the fourth pass agrees, but also notes that any fix changes serialized summary counts and therefore needs caller awareness.

Fourth, non-finite numeric row behavior is implicit and untested. `metrics_mean()` reconstructs `mean` as `sharpe * volatility` (`metric_surface.rs:278-279`), and `project_metrics_row()` writes raw `f64` values into the sidecar row vectors (`metric_surface.rs:290-337`). Combined with `serde_json`’s `f64 -> Value::Null` behavior, that means saved metric rows may legally contain JSON `null` elements. That may be acceptable, but it should be an explicit, tested contract if it remains.

## Implementation Priority

### Safe-now hardening

The current bucket should stay `Small surgical refactor`, and the highest-value work is still narrow.

1. **Unify rebuild precedence without changing runtime authority.**
   Add a builder-specific helper so rebuild uses runtime hint first, then valid current sidecar metadata, then heuristics. This removes the current split-brain between `artifact_metric_surface_kind()` and `build_optimization_metric_surface_artifact()` without letting stale persisted metadata override fresh runtime truth.

2. **Add direct tests for the missing live branches and edge contracts.**
   The minimum worthwhile additions are:
   - `RegimeHybrid` exact-GPU coverage
   - schema-2 artifact with invalid `surface` label
   - accepted alias spellings from `parse_surface_label()`
   - non-finite metric cells and the resulting JSON row representation

3. **Document the two implicit row rules that matter.**
   A short comment is warranted for:
   - `mean` being reconstructed as `sharpe * volatility`
   - row vectors potentially containing JSON `null` cells after serialization when metrics are non-finite

4. **Be careful if tightening “current metadata” semantics.**
   Improving `missing_metadata` behavior is reasonable, but it is not purely internal. `ArtifactMetricSurfaceSummary` is a serialized DTO (`metric_surface.rs:69-95`), so count semantics are effectively a contract. Any change should be test-backed and coordinated with the consumers that interpret those counts.

### Larger redesign still deferred

The broader fixes still do **not** belong in a file-local cleanup pass.

1. **Do not replace the JSON-pointer heuristic layer with a typed artifact model in this change.**
   That may be a worthwhile future cleanup, but it crosses artifact schema, call-site behavior, and backward-compatibility assumptions.

2. **Do not broaden the fallback tables based only on intuition.**
   This file is full of stringly route and strategy inference (`metric_surface.rs:118-275`). If any strategy support matrix is widened, it should be done from app-core/runtime truth, not because the table “looks incomplete.”

3. **Do not change row payload shape or remove rows because current consumers look sparse.**
   `best_metrics_row` and `top_metric_rows` are part of the persisted sidecar shape (`metric_surface.rs:98-107`, `452-458`). That is persistence-sensitive work, not cleanup.

## Verification If Changed

For a file-local hardening pass, the right proof is targeted automated coverage, not manual desktop smoke.

At minimum:

- `cargo test -p ta_desktop_demo_app metric_surface -- --nocapture`

And if the change touches rebuild callers or summary consumers outside this file, follow with the relevant artifact/meta/report tests in the owning modules.

The new regression cases that matter most are:

- rebuild precedence when runtime hint and current sidecar disagree
- rebuild precedence when current sidecar exists but runtime hint is absent
- `RegimeHybrid` exact-GPU branch coverage
- invalid schema-2 `surface` value fallback
- alias label acceptance
- non-finite row values producing the intended serialized shape

No special GPU ABI validation is required for a file-only change here, because the module does not define or cross a CUDA ABI boundary. The real risk surface is persistence and downstream artifact interpretation.

## Regression And Feature-Preservation Constraints

This file is not a hot path and not a direct IPC entrypoint, so performance and runtime-authority concerns are secondary here. The real constraints are persistence compatibility and interpretation stability.

Do not make these mistakes while “cleaning it up”:

- **Do not let current persisted sidecar metadata outrank runtime `metric_surface_kind` during rebuild.**
  That would make stale artifacts harder to repair and could preserve wrong history after a rerun or resave.

- **Do not silently change how non-finite row cells are represented.**
  Moving from implicit JSON `null` to omission, zero, string tags, or row-drop semantics would change the persisted artifact contract.

- **Do not rename or casually repurpose `missing_metadata`.**
  Even though the name is imperfect, it is part of a serialized summary shape. Tightening semantics is safer than renaming fields.

- **Do not widen heuristic support tables without evidence.**
  This module is already inference-heavy. Over-eager fallback broadening can misclassify historical artifacts, especially when route evidence is missing.

- **Do not overreact to the `.ok()` in `build_optimization_metric_surface_artifact_value()`.**
  The practical risk is row-content semantics, not a likely serialization panic path.

The compatibility rule should stay simple: preserve current artifact behavior unless a concrete contract weakness is visible in the live source, and when a weakness is real, prefer additive tests and precedence fixes over schema churn.

## Fourth-Pass Conclusion

The third-pass **bucket should stand**, but the third-pass **packet should not** be treated as the controlling rationale.

This file is not a Tauri command wrapper. It is a private artifact-compatibility seam whose real responsibilities are metric-surface inference, row projection, and sidecar rebuild. The best near-term work remains small and surgical.

What the fourth pass adds is a cleaner priority order:

- keep the bucket as `Small surgical refactor`
- discard the IPC/cancellation framing from the current third-pass packet
- focus instead on the real file-local risks:
  - rebuild precedence drifting away from current metadata classification
  - missing tests for admitted branches like `RegimeHybrid`
  - implicit, undocumented non-finite row serialization behavior

That is a conservative, evidence-backed hardening path that improves correctness without reopening artifact schema design or broader runtime ownership questions.
