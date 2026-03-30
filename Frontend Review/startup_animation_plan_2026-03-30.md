# Startup Animation Plan

Date: 2026-03-30

Scope: repo-specific implementation plan for a restrained startup animation that uses the existing VectorAlpha brand assets without compromising startup honesty, bundle discipline, or dense-workflow UX.

Status: implemented for the first-wave in-window overlay scope

Implementation summary:

- startup boot truth is now driven from [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts)
- the startup overlay now ships in [StartupAnimationOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StartupAnimationOverlay.vue)
- the overlay is mounted from [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue) above the titlebar, runtime banner, and shell
- the final visual direction is a shell-first translucent fade-over, not a centered startup card
- browser-side smoke harnesses now understand the overlay and capture an intro screenshot before dismissing it
- first-wave verification is green across frontend, browser-smoke, bundle-budget, Cargo, and WDIO desktop smoke

This document is intentionally narrower than:

- [desktop_productization_plan_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\desktop_productization_plan_2026-03-28.md)
- [existing_stack_utilization_plan_2026-03-28.md](C:\Rust Projects\ta_desktop_demo\Frontend Review\existing_stack_utilization_plan_2026-03-28.md)

It exists because startup animation has a different failure mode than ordinary shell polish:

- it can easily become fake latency theater
- it can easily bloat the main entry chunk
- it can easily add repetitive motion to a frequently launched workstation app

The goal here is not “make startup flashy.” The goal is:

- brief brand polish
- honest startup signaling
- one clear implementation surface that is easy to iterate on in isolation

## Current Repo Baseline

Current evidence:

- [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts)
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts)
- [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)
- [VectorAlphaLogo.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VectorAlphaLogo.vue)
- [vectoralpha-animated.svg](C:\Rust Projects\ta_desktop_demo\web\src\assets\branding\vectoralpha-animated.svg)
- [vectoralpha-static.svg](C:\Rust Projects\ta_desktop_demo\web\src\assets\branding\vectoralpha-static.svg)
- [vite.config.ts](C:\Rust Projects\ta_desktop_demo\web\vite.config.ts)
- [check-bundle-budget.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-bundle-budget.mjs)
- [src-tauri/tauri.conf.json](C:\Rust Projects\ta_desktop_demo\src-tauri\tauri.conf.json)
- [src-tauri/src/lib.rs](C:\Rust Projects\ta_desktop_demo\src-tauri\src\lib.rs)

Current state summary:

- the app already has integrated static and animated VectorAlpha SVG assets
- the current brand mark already respects reduced-motion posture
- there is no current startup-animation implementation
- the app already has a boot/runtime status model in [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts), but it is not actively driven by the current production startup path
- the shell already mounts a compact runtime status surface in [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)
- the main window is manually built in Rust and shown after startup setup, so a true Tauri splash window is technically possible later
- the repo enforces strict bundle budgets and already relies on lazy chunking

Important current-state constraint:

- [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts) currently applies bootstrap work before `app.mount(root)`
- that means any startup failure before mount would prevent [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue) from rendering at all
- a truthful startup animation therefore needs the app mounted early enough for the overlay and error posture to actually exist

## Implementation Decision

The first implementation should be:

- an in-window startup overlay
- built as one primary Vue component file
- using the existing VectorAlpha SVG assets
- with no new animation runtime dependency in the first slice

The first implementation should not be:

- a multi-window Tauri splashscreen
- a GSAP/Rive/three.js stack
- a blocking cinematic sequence with fixed duration
- a global motion redesign

Reasoning:

- the repo already has a dense desktop shell and a working titlebar/runtime surface
- the app does not currently expose a rich multi-stage startup handshake worth splitting into a dedicated splash window
- the simplest high-quality version is an isolated overlay component plus narrow boot-state wiring
- a single-file component keeps design iteration easy without spreading animation logic across the shell

## Primary Design Goal

The startup animation should communicate:

- VectorAlpha / VectorGrid identity
- data in -> compute -> result out
- readiness, not mystery

It should feel:

- restrained
- technical
- premium
- brief

It should avoid:

- generic AI particles
- fake metric cards
- 3D spectacle in the first implementation
- repeated ambient motion after the main app is ready

## File-By-File Responsibilities

This plan should be implemented with a narrow, explicit write scope.

### Primary implementation file

- [StartupAnimationOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StartupAnimationOverlay.vue)
  - owns the full-screen visual
  - owns the intro timing state machine
  - owns skip behavior
  - owns reduced-motion branching
  - owns the lightweight grid/result accent
  - owns the transition from fullscreen intro to dismissed state

### Required integration files

- [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts)
  - creates and binds Pinia
  - marks the runtime as booting
  - mounts the app before expensive or failure-prone startup work
  - advances boot stage labels
  - marks ready or error
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
  - mounts the startup overlay near the top of the tree
- [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts)
  - remains the source of truth for `idle` / `booting` / `ready` / `error`
  - may receive small additive state only if strictly necessary for startup visualization

### Conditional integration files

- [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)
  - should only be edited if the overlay-to-banner handoff cannot be made clean with z-index and lifecycle timing alone
- [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts)
  - may add a dev/test-only debug hold hook if screenshot/testing determinism requires it

### Explicitly avoid touching

- [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue) beyond layering-safe coexistence
- [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue) beyond being visually covered during intro
- [app.css](C:\Rust Projects\ta_desktop_demo\web\src\styles\app.css) unless a tiny shared token is truly unavoidable

## Isolation Policy

The implementation should stay as self-contained as practical.

Primary new file:

- [StartupAnimationOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StartupAnimationOverlay.vue)

The component should own:

- template
- motion timing
- reduced-motion behavior
- inline SVG or DOM for any extra grid/result accents beyond the imported logo assets
- full-screen enter/exit behavior
- skip behavior
- handoff timing

Allowed narrow integration points:

- [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts)
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
- [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts)
- [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)

Avoid:

- spreading startup-animation CSS into the global stylesheet
- adding multiple startup-only composables before the concept is proven
- creating a second long-lived diagnostics/status UI beside the runtime banner

## Product Rules

1. Never delay the real app just to finish the animation.

2. The full-screen sequence is allowed only while real startup is still occurring or within a very short minimum visibility threshold.

3. If the animation is skipped or the app takes longer than the signature sequence, hand off to compact status rather than keeping a full-screen effect alive.

4. Reduced motion must switch to a nearly static presentation.

5. The startup animation must be visually compatible with the current titlebar, shell, and token system.

## Exact Startup Order

The first implementation should use this startup order:

1. Create Pinia.
2. Bind runtime Pinia through [runtimePinia.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimePinia.ts).
3. Resolve the runtime store and call `markBooting(...)`.
4. Create and mount the Vue app immediately.
5. While the fullscreen overlay is visible, run the current bootstrap work:
   - `applySettingsBootstrapFromStorage()`
   - `applyWorkflowBootstrapFromStorage()`
   - `applyRecommendedOptimizationDefaults()`
   - `initializeDesktopProductization()`
6. Mark ready only after the app is mounted and one paint-safe boundary has passed.
7. If any startup step throws, mark error and keep the mounted app alive so the error posture can render.

Rationale:

- this preserves a truthful startup signal
- this keeps errors visible
- this allows the shell to settle behind the overlay without exposing flicker to the user

Recommended paint-safe ready rule:

- call `runtime.markReady()` only after `app.mount(root)` and at least one `requestAnimationFrame`

Recommended first-wave stage labels:

- `Restoring workspace state`
- `Initializing desktop services`
- `Mounting workspace shell`

Do not add more stage labels until there is evidence they improve user trust.

## Runtime Boot Contract

The runtime contract for the startup animation should remain minimal and explicit.

### Source of truth

- [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts)

### Allowed phase values

- `idle`
- `booting`
- `ready`
- `error`

### Required transition sequence

- initial:
  - `idle`
- startup begins:
  - `booting`
- successful startup:
  - `ready`
- failed startup:
  - `error`

### Exact first-wave transition points

- `markBooting('Restoring workspace state')`
  - after Pinia exists and before app mount work begins
- `setBootStage('Initializing desktop services')`
  - before `initializeDesktopProductization()`
- `setBootStage('Mounting workspace shell')`
  - immediately before the final readiness handoff
- `markReady()`
  - after `app.mount(root)` plus one paint-safe boundary
- `markError(error)`
  - on any thrown startup exception after the runtime store exists

### Anti-bloat rule

Do not turn `runtimeStore` into a general animation store.

The startup overlay should derive almost all of its display behavior from:

- `bootPhase`
- `bootStage`
- local component timers

Not from a wide new startup state model.

## Recommended Motion Shape

Phase A: signature sequence

- target duration: about `1.2` to `1.6` seconds
- show the VectorAlpha animated mark
- add a subtle exact-grid or candidate-lattice accent
- add one restrained “best region” or “resolved result” beat
- keep the palette aligned with the current brand/theme tokens

Phase B: long-tail posture if still booting

- collapse the full-screen animation
- either:
  - hand off to the existing compact runtime banner
  - or downgrade to a minimal low-motion overlay strip within the same component

Recommended first implementation choice:

- hand off to [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)

That avoids maintaining two separate long-lived boot-status surfaces.

## Exact Overlay State Machine

The overlay should use a local component state machine, not a second global store.

Recommended local states:

- `hidden`
  - before booting is observed
- `intro`
  - fullscreen branded startup sequence is visible
- `exiting`
  - fade/resolve out of the fullscreen overlay
- `dismissed`
  - overlay is gone

Recommended transition rules:

- `hidden -> intro`
  - when `bootPhase === 'booting'`
- `intro -> exiting`
  - when any of the following becomes true:
    - `bootPhase === 'ready'` and the minimum visibility threshold is satisfied
    - `bootPhase === 'error'`
    - skip is requested after the minimum threshold
    - the signature duration ends while startup is still booting
- `exiting -> dismissed`
  - after the exit animation duration

Explicitly rejected first-wave states:

- a persistent fullscreen loop
- a second compact overlay strip that duplicates the runtime banner
- a decorative idle mode after the app is already ready

Long-tail behavior rule:

- if startup outlasts the signature duration, the fullscreen overlay should exit and let the existing runtime banner carry the remaining boot state

Error behavior rule:

- if `bootPhase === 'error'`, decorative motion stops immediately
- the overlay exits quickly
- the error posture is carried by the existing runtime banner and runtime diagnostics path

## Boot-Lifecycle Plan

The current repo has a runtime boot model but it is effectively dormant in production.

Before the startup animation can be truthful, the boot model needs to become real.

Recommended first-wave lifecycle:

1. `main.ts`
   - mark booting before app mount work starts
   - set one or two stage labels during:
     - storage/bootstrap application
     - desktop bridge initialization
     - app mount

2. `main.ts` or the startup overlay component
   - mark ready after the root app is mounted and the initial frame is painted

3. error path
   - mark error if initialization throws

Do not over-model startup in the first slice.

Avoid:

- building a fake multi-step startup engine
- inventing backend stages that are not actually observed

The initial boot-state contract only needs to distinguish:

- booting
- ready
- error
- one or two human-readable stage labels

## Overlay Behavior

The overlay should:

- mount above the shell and titlebar
- use existing branding assets rather than duplicating them
- fade/resolve away once the app is ready and the minimum visibility threshold has passed

Recommended timing rules:

- minimum show time:
  - about `320` to `450 ms`
- preferred full signature:
  - about `1.2` to `1.6 s`
- if ready arrives before the signature completes:
  - allow a fast graceful exit after the minimum show time
- if not ready after the signature:
  - downgrade from full-screen intro to compact status

Skip rules:

- allow skip on click or keypress after the minimum threshold
- skipping should only skip the full-screen visual
- skipping should not change real boot state
- if the app is still booting after skip, compact runtime status remains available

## Timing Contract

The first implementation should use explicit constants inside [StartupAnimationOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StartupAnimationOverlay.vue).

Recommended first-wave constants:

- `MIN_VISIBLE_MS = 360`
- `SIGNATURE_MS = 900`
- `EXIT_MS = 180`
- `SKIP_UNLOCK_MS = MIN_VISIBLE_MS`

Timing ownership rules:

- keep timing constants local to the component in the first slice
- do not add a shared animation-config file yet
- do not make timing user-configurable in the first slice

Acceptance rule for timings:

- the signature must feel intentional
- the overlay must never noticeably delay readiness
- the full-screen layer must be gone quickly once it has served its purpose

## Reduced-Motion Rules

Reduced motion must not simply hide the overlay and create visual discontinuity.

Reduced-motion version should:

- use the static brand mark
- keep only a subtle opacity or reveal transition
- avoid grid sweeps, bar travel, or camera-like movement
- preserve the same enter/exit timing contract

## Layering And Interaction Contract

Current relevant z-indexes in the repo:

- workspace shell:
  - [VueWorkspaceShell.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\VueWorkspaceShell.vue) uses `1200`
- task center:
  - [TaskCenterOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\TaskCenterOverlay.vue) uses `4000`
- runtime banner:
  - [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue) uses `4050`
- titlebar:
  - [DesktopTitlebar.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\DesktopTitlebar.vue) uses `4300`
- command palette:
  - [CommandPaletteOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\CommandPaletteOverlay.vue) uses `9998` and `9999`

Recommended startup overlay layering:

- fullscreen overlay root:
  - `z-index: 5000`

This keeps the overlay:

- above shell chrome
- above the runtime banner
- above the titlebar
- below the command palette if it were somehow opened

Interaction rules while fullscreen overlay is active:

- pointer events to the underlying shell must be blocked
- skip should be the only intentional user action
- no focus trap is required in the first slice, but the overlay root should be keyboard-reachable for skip
- command palette and shell controls should not be relied on during the intro window

## Testability Contract

The implementation should be test-friendly without spreading logic across multiple helper files.

Recommended first-wave testability choices:

- keep the state machine inside the component
- use fake timers in component tests
- derive reduced-motion through:
  - the real media query in production
  - a narrow override prop or stubbed media-query helper in tests

If deterministic browser screenshots are hard to capture because the overlay exits too quickly, add one small dev/test-only hold hook.

Recommended form if needed:

- localStorage or query-param driven
- only active in dev/test
- never exposed as a normal user-facing production setting in the first slice

## Recommended File Shape

### New primary file

- [StartupAnimationOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StartupAnimationOverlay.vue)

Recommended responsibilities:

- import:
  - [vectoralpha-static.svg](C:\Rust Projects\ta_desktop_demo\web\src\assets\branding\vectoralpha-static.svg)
  - [vectoralpha-animated.svg](C:\Rust Projects\ta_desktop_demo\web\src\assets\branding\vectoralpha-animated.svg)
- subscribe to runtime boot state
- compute:
  - local overlay stage
  - canSkip
  - reducedMotion
- contain all startup-only styles

### Expected narrow edits

- [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts)
  - early mount order
  - boot-state wiring
- [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue)
  - mount the overlay near the top of the app tree
- [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts)
  - keep the minimal boot contract clean
- [AppRuntimeBanner.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\AppRuntimeBanner.vue)
  - edit only if z-index and lifecycle handoff are not sufficient to prevent duplicate boot messaging

## Verification Requirements

Minimum required verification for implementation:

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`
- `npm --prefix web run check:bundle-budget`

Desktop-focused verification:

- `cargo check -p ta_desktop_demo_app`
- launch the Tauri app and verify:
  - startup overlay appears
  - overlay exits once ready
  - skip does not break startup
  - reduced-motion path is visually calm
  - error state still surfaces meaningful startup failure info

Recommended browser-side checks:

- focused component tests for overlay timing and handoff logic
- one screenshot for:
  - normal signature state
  - reduced-motion state
  - post-signature boot state while the runtime banner remains visible

## Verification Matrix

### Unit / component verification

Add targeted tests for:

- runtime boot transitions in [runtimeStore.ts](C:\Rust Projects\ta_desktop_demo\web\src\stores\runtimeStore.ts) if the transition contract changes
- [StartupAnimationOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StartupAnimationOverlay.vue):
  - enters on `booting`
  - exits on `ready` after minimum visibility
  - exits on `error`
  - exits on skip after unlock
  - renders reduced-motion posture correctly
  - does not remain fullscreen past `SIGNATURE_MS`

### Browser-side verification

Use the existing harnesses:

- [capture-baseline-ui.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\capture-baseline-ui.mjs)
- [check-a11y-smoke.mjs](C:\Rust Projects\ta_desktop_demo\web\scripts\check-a11y-smoke.mjs)

Required browser-side proof:

- screenshot of fullscreen intro
- screenshot of reduced-motion intro
- screenshot of post-signature handoff with runtime banner visible if boot is still pending

### Desktop verification

Use the real Tauri runtime to confirm:

- overlay appears above the custom titlebar
- shell does not become visible/interactable before the overlay exits
- skip works
- startup error still yields visible runtime failure information

### Bundle verification

The overlay implementation must not:

- add a new heavy dependency
- increase the main entry chunk beyond the existing guardrail
- cause a new lazy chunk to exceed the current budget without explicit approval

## Anti-Regression Rules

1. Early-mount startup wiring must not strand the app in a half-mounted state on failure.

2. The overlay must cover workspace flicker introduced by moving bootstrap work after mount.

3. The titlebar must not remain visually dominant above the startup animation.

4. The runtime banner must remain the long-tail and error-state source of truth.

5. The startup animation must not introduce a new always-visible status surface after ready.

6. The animation must not require a new library in the first slice.

7. If the component cannot be implemented cleanly as one primary file plus narrow integration edits, stop and re-evaluate before broadening scope.

## Acceptance Criteria

This work is complete when:

- the startup visual is primarily owned by one new component file
- the animation uses the existing VectorAlpha assets rather than a new animation stack
- the animation never blocks real readiness
- reduced-motion posture is explicit and clean
- the full-screen effect does not compete with the existing runtime banner
- the repo’s bundle budget remains green
- the animation feels product-native rather than decorative

## Explicit Non-Goals

This first implementation does not include:

- a new Tauri splash window
- GSAP
- Rive
- three.js
- PixiJS
- Spline
- Theatre.js
- 3D optimization landscapes
- a settings toggle on day one
- fake performance metrics or fake result cards

Those can be evaluated later only if the simple isolated implementation proves worthwhile.

## Later Optional Follow-Ups

Only consider these after the first slice is shipped and used:

- add a persisted “show startup animation” preference in Settings
- add a slightly richer data-grid accent if the current overlay feels too logo-only
- evaluate a true Tauri splash window only if there is real startup work worth masking honestly
- evaluate a narrowly lazy-loaded motion library only if the CSS/SVG implementation proves too limiting

## Implementation Checklist

### Phase 1: Boot Truthfulness

- [x] Activate the current runtime boot state in production startup.
- [x] Reorder [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts) so the Vue app mounts before startup work that can fail.
- [x] Mark boot start after Pinia is bound and before startup work begins in [main.ts](C:\Rust Projects\ta_desktop_demo\web\src\main.ts).
- [x] Use the first-wave stage labels:
  - `Restoring workspace state`
  - `Initializing desktop services`
  - `Mounting workspace shell`
- [x] Mark ready only after mount plus one paint-safe boundary.
- [x] Preserve meaningful error reporting if startup fails.

### Phase 2: Isolated Overlay

- [x] Create [StartupAnimationOverlay.vue](C:\Rust Projects\ta_desktop_demo\web\src\components\StartupAnimationOverlay.vue) as the primary implementation surface.
- [x] Keep startup-only visual logic inside that file wherever practical.
- [x] Import and use the existing static and animated VectorAlpha SVG assets.
- [x] Add one restrained grid/result accent that reinforces the product story.
- [x] Implement the local overlay state machine:
  - `hidden`
  - `intro`
  - `exiting`
  - `dismissed`
- [x] Keep timing constants local to the component.
- [x] Avoid any new animation/runtime dependency in the first slice.

### Phase 3: Shell Integration

- [x] Mount the overlay near the top of [App.vue](C:\Rust Projects\ta_desktop_demo\web\src\App.vue).
- [x] Ensure the overlay uses a z-index above the runtime banner and titlebar while active.
- [x] Block pointer interaction with the underlying shell while the fullscreen overlay is active.
- [x] Rely on the existing runtime banner for long-tail boot and error posture unless explicit suppression work proves necessary.
- [x] Hand off from fullscreen intro to the existing runtime banner if boot outlasts the signature sequence.
- [x] Keep the app usable immediately once startup is actually ready.

### Phase 4: Motion Rules

- [x] Keep the signature sequence brief.
- [x] Add a minimum visibility threshold so the overlay does not flicker on very fast launches.
- [x] Use first-wave timing targets:
  - `MIN_VISIBLE_MS = 360`
  - `SIGNATURE_MS = 900`
  - `EXIT_MS = 180`
- [x] Add click/keypress skip after the minimum threshold.
- [x] Ensure skip only dismisses the full-screen visual, not the underlying boot state.
- [x] Add a reduced-motion path that uses the static mark and minimal transition only.
- [x] Stop decorative motion immediately if startup enters the error state.

### Phase 5: Verification

- [x] Run `npm --prefix web run lint`.
- [x] Run `npm --prefix web run typecheck`.
- [x] Run `npm --prefix web run test`.
- [x] Run `npm --prefix web run build`.
- [x] Run `npm --prefix web run a11y:smoke`.
- [x] Run `npm --prefix web run baseline:ui`.
- [x] Run `npm --prefix web run check:bundle-budget`.
- [x] Run `cargo check -p ta_desktop_demo_app`.
- [x] Add component tests for intro, skip, ready, error, and reduced-motion behavior.
- [x] Capture at least one screenshot of the fullscreen intro state.
- [x] Capture at least one screenshot of the reduced-motion state.
- [x] Cover the post-signature banner handoff path with component-test evidence; a naturally slow live boot screenshot is not part of the first-wave proof because the production-like preview path now settles too quickly.
- [x] Verify the normal startup path in the Tauri app.
- [x] Verify the reduced-motion path.
- [x] Verify the error/handoff path.
