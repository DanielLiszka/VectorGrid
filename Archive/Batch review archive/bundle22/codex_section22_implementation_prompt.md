# Codex implementation prompt — Section 22 startup data-root ownership and dataset-readiness latency

## Mission

Implement only the remaining repo-valid Section 22 work for startup data-root ownership and dataset-readiness latency.

Keep the first wave focused on:

1. making the startup-ready boundary truthful for critical dataset-readiness work
2. separating critical startup data work from deferred affordance work
3. preserving startup snapshot correctness when a Polygon cache root is moved
4. adding only the focused verification needed to prove the new startup behavior

This is not a general settings redesign and not a broad `polygon.rs` / `data_cache.rs` rewrite.

---

## Read first

Before editing code, read:

1. `bundle22/section22_startup_data_root_ownership_and_dataset_readiness_latency_plan.md`
2. `gpt54_feature_review_map.md` section `## 22. Startup data-root ownership and dataset-readiness latency`
3. `REPO_MAP.md`
4. `web/src/main.ts`
5. `web/src/services/workflowBootstrap.ts`
6. `web/src/stores/dataStore.ts`
7. `src-tauri/src/settings.rs`
8. `src-tauri/src/commands_data.rs`

Trust the current code and the repo-state corrections below over older review wording.

---

## Repo-state validation and scope corrections

### Already true in this repo

- [x] `src-tauri/src/settings.rs` already owns settings-backed market-data root resolution through `configured_market_data_root_dir(...)`, `resolve_market_data_root_dir(...)`, and `set_market_data_root_dir(...)`.
- [x] `src-tauri/src/commands_data.rs` already owns the real Tauri data-root and Polygon cache command boundary.
- [x] `src-tauri/src/state.rs` already has truthful load-source and in-memory reuse behavior; Section 22 must not reopen the earlier Section 14 load-truthfulness fixes.
- [x] `web/src/stores/dataStore.ts` already owns startup snapshot persistence, startup dataset restore, market-data-root hint state, and lazy Polygon cache loading.
- [x] `web/src/services/dataService.ts` already normalizes market-data-dir info and truthful load results correctly.
- [x] `web/src/stores/settingsStore.ts` already forwards market-data-root actions into `dataStore`; it is not the primary owner of the startup path.
- [x] `web/src/main.ts` already mounts the app before running the boot sequence, so first-wave startup changes do not need to block shell mount.
- [x] `src-tauri/src/data_cache.rs` is not the startup owner; it is part of explicit CSV load, not shell bootstrap.
- [x] `web/src/services/settingsBootstrap.ts` is not a primary data-root owner in the current repo; it restores UI/settings preferences, not market-data-root startup semantics.

### Repo-valid scope corrections

- [x] The raw review-map file list is too broad. The real first-wave owners are `web/src/main.ts`, `web/src/services/workflowBootstrap.ts`, and `web/src/stores/dataStore.ts`.
- [x] `src-tauri/src/polygon.rs` is supporting infrastructure here, not the primary startup owner.
- [x] `web/e2e/desktop/README.md`, `web/scripts/check-a11y-smoke.mjs`, and `web/scripts/check-tauri-parity.mjs` are not first-wave startup latency verification authorities.
- [x] `web/scripts/check-known-tauri-commands.mjs` matters only if the Tauri command surface changes.
- [x] The current startup path does not eagerly call `polygon_get_cache_info` unless frontend state restores `polygonOpen`; a mandatory backend combined startup snapshot command is therefore not automatically justified.

### Still-open repo-valid work

- [x] Make the startup-ready boundary truthful by letting `main.ts` await the critical startup data bootstrap work started from `workflowBootstrap.ts`.
- [x] Split critical startup work from deferred affordance work inside `web/src/stores/dataStore.ts`.
- [x] Prevent stale restored `polygonOpen` state from forcing an eager startup Polygon cache scan when the shell only needs dataset readiness.
- [x] Preserve startup snapshot correctness when `setMarketDataDir(..., moveExisting=true)` moves a loaded Polygon dataset to a new cache root.
- [x] Add or update focused tests for startup sequencing, deferred cache loading, and moved-root snapshot behavior.

### Explicit defer / exclude items

- [x] Do not use this section for a settings UX redesign or for broader `SettingsWorkspacePanel.vue` layout work.
- [x] Do not use this section for a broad `polygon.rs` rewrite, reconnect policy work, or cache-manifest redesign.
- [x] Do not use this section for a broad `data_cache.rs` redesign or CSV load-path optimization pass.
- [x] Do not make validation/repair/download flows part of the critical startup path.
- [x] Do not replace the existing startup snapshot format with a broader persistence schema in the first wave.
- [x] Do not add a combined backend startup snapshot command unless the frontend sequencing cleanup still leaves measured startup fanout worth removing.
- [x] Do not treat `settingsBootstrap.ts` as an active primary file unless a direct startup-boundary dependency appears while implementing.

---

## File scope

### Primary implementation files

- `web/src/main.ts`
- `web/src/services/workflowBootstrap.ts`
- `web/src/stores/dataStore.ts`

### Primary verification files

- `web/src/main.test.ts`
- `web/src/services/workflowBootstrap.test.ts`
- `web/src/stores/dataStore.actions.test.ts`
- `web/src/components/StartupAnimationOverlay.test.ts`
- `src-tauri/tests/settings_tests.rs`

### Inspect-only / conditional files

- `web/src/stores/settingsStore.ts`
- `web/src/stores/settingsStore.actions.test.ts`
- `web/src/components/StartupAnimationOverlay.test.ts`
- `web/src/services/dataService.ts`
- `src-tauri/src/commands_data.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/data_cache.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/lib.rs`

### Keep unchanged unless evidence forces it

- current settings-file shape in `src-tauri/src/settings.rs`
- current explicit data commands for market-data-dir and Polygon cache flows
- current lazy Polygon cache loading behavior outside the startup path
- current Section 14 load-source/cache-path truthfulness semantics

---

## Hard constraints

- Keep the first wave additive.
- Keep the app mount early; improve startup truthfulness without regressing shell mount behavior.
- Distinguish critical startup readiness from deferred affordance readiness.
- Preserve current command compatibility unless a small new startup command is proven necessary.
- Keep CPU-only builds and non-Tauri test contexts working.
- Do not turn this into a broad settings/data architecture refactor.

### Ownership guard

Treat these as the current owners:

- `web/src/main.ts`
  - real frontend boot-stage / ready boundary
- `web/src/services/workflowBootstrap.ts`
  - workflow bootstrap sequencing and startup hydration entrypoint
- `web/src/stores/dataStore.ts`
  - startup snapshot restore, market-data-root state, lazy Polygon cache behavior
- `src-tauri/src/settings.rs`
  - persisted market-data-root ownership
- `src-tauri/src/commands_data.rs`
  - market-data-dir / Polygon cache command boundary

---

## Implementation checklist

### Phase 1 — truthful startup sequencing

- [x] Make `web/src/services/workflowBootstrap.ts` return or otherwise expose the critical startup data hydration work it starts.
- [x] Update `web/src/main.ts` so `runtimeStore.markReady()` happens after critical startup data hydration completes.
- [x] Preserve early app mount and existing boot error handling.
- [x] Keep boot-stage messaging truthful once the startup-ready boundary moves.

### Phase 2 — critical vs deferred startup work

- [x] Split critical startup data-root / dataset-restore work from deferred affordance work in `web/src/stores/dataStore.ts`.
- [x] Keep market-data-dir info and startup dataset restore on the critical path.
- [x] Keep sample CSV discovery off the critical ready boundary unless it is proven necessary.
- [x] Prevent stale restored `polygonOpen` state from forcing an eager startup Polygon cache scan.
- [x] Preserve explicit lazy Polygon cache loading on modal open / explicit refresh.

### Phase 3 — data-root ownership correctness

- [x] Detect successful Polygon cache-root moves when `setMarketDataDir(..., moveExisting=true)` changes the effective cache directory.
- [x] Rewrite current Polygon path-bearing frontend state and the persisted startup snapshot to the new cache root when the loaded dataset actually moved.
- [x] Keep manual CSV datasets and non-move root changes unchanged.
- [x] Add focused tests for moved-root snapshot correctness.

### Phase 4 — tests / fallout

- [x] Add or update `workflowBootstrap` tests for awaited startup sequencing.
- [x] Add or update `dataStore` tests for deferred cache loading and moved-root startup snapshot behavior.
- [x] `settingsStore` tests did not need changes because mirroring behavior stayed the same.
- [x] Validate startup animation behavior after the ready-boundary change.

### Conditional verification

- [x] `cargo test -p ta_desktop_demo_app commands_data` was not required because command-layer behavior stayed unchanged.
- [x] `npm --prefix web run check:commands` was not required because the Tauri command surface stayed unchanged.
- [x] `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate`

## Verification checklist

- [x] `npm --prefix web run test -- --run src/services/workflowBootstrap.test.ts src/stores/dataStore.actions.test.ts src/main.test.ts src/components/StartupAnimationOverlay.test.ts`
- [x] `npm --prefix web run test -- --run src/stores/settingsStore.actions.test.ts` was not required because market-data-root mirroring behavior stayed unchanged
- [x] `npm --prefix web run typecheck`
- [x] `cargo test -p ta_desktop_demo_app --test settings_tests`

---

## Acceptance criteria

- [x] The app no longer declares runtime ready before critical startup data-root / dataset-restore work completes.
- [x] Startup dataset readiness is no longer coupled to eager Polygon cache scans caused only by stale restored modal state.
- [x] Moving a Polygon cache root with `moveExisting=true` no longer leaves a stale startup restore path behind.
- [x] Existing settings/data-root ownership remains where it already lives in the repo.
- [x] The bundle stays focused on startup ownership and readiness latency, not on broad settings or cache redesign.
