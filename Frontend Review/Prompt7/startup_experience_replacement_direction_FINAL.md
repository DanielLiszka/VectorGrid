# Frontend Review/startup_experience_replacement_direction_FINAL.md

## 1. Executive summary

VectorGrid should replace the current startup treatment with a **runtime-banner prelude over a short shell veil**. The real shell should be visible from frame one under a light interaction-blocking veil, while a compact startup rail sits in the same top-left status zone that `AppRuntimeBanner.vue` already owns. That rail should show only a small static VectorAlpha mark, the product label, one live boot-stage line, and one restrained mineral-blue accent. When readiness becomes real, the veil dissolves away; if boot continues past a short signature ceiling, the full-screen layer exits and `AppRuntimeBanner.vue` becomes the only remaining boot and error surface.

This direction preserves the repo’s truthful early-mount startup model, keeps `runtimeStore` minimal, and fits the Signal Foundry workstation tone much better than a standalone startup scene. It also aligns with current platform guidance: launch visuals should reinforce that the app is quick and immediately ready, large-surface transitions should fade quickly rather than perform theatrically, and reduced-motion handling should replace nonessential motion with calmer alternatives.

## 2. Final startup experience recommendation

### Adopted direction

Adopt a **shell-first readiness veil** with a **compact startup rail** that feels like the premium first frame of `AppRuntimeBanner.vue`, not like a separate splash product.

Keep:
- the current in-window model
- startup truth driven by `web/src/main.ts` and `web/src/stores/runtimeStore.ts`
- the runtime banner as the long-tail and error-state owner

The improvement is not “more animation.” The improvement is that startup becomes thinner, more credible, and more product-native.

The startup rail should feel like a small instrument sheet resting over the workbench, which matches the theme guidance that overlays should feel like raised instrument sheets, shell surfaces should feel fixed and machined, and motion should stay short, deliberate, and useful rather than decorative.

### Deferred ideas

Defer the following until after the shell-first rail ships and is judged on real launches:
- a startup-specific one-shot SVG variant
- a user-facing startup preference toggle
- any evaluation of a native splash window or lazy-loaded motion library

These are optional follow-ups, not first-line work.

## 3. Visual concept description

The visual concept is deliberately simple:

- **Frame one:** the real VectorGrid shell is already present underneath a translucent veil.
- **Status zone:** a compact rail appears in the top-left status area, visually aligned to the `AppRuntimeBanner.vue` corner family.
- **Rail contents:** a small static VectorAlpha mark, the product label, one live boot-stage line, and a single restrained accent underline or edge reveal.
- **Material treatment:** use the existing Signal Foundry palette and surface language — graphite/slate surfaces, mineral-blue action accent, restrained borders, instrument-sheet overlay treatment.
- **Motion:** one quick opacity-and-accent reveal only. No sweeping scene motion, no pseudo-analysis tableau, no multiple moving layers.

This should feel closer to the shell than to branding theater. Premium feel should come from typography, material discipline, and the immediacy of the real first frame, not from a fake analytical sequence.

## 4. Timing and handoff rules

Use the existing first-wave timing contract as the base, but treat `SIGNATURE_MS` as a **ceiling**, not as something the UI tries to “perform through.” Keep the constants local to `web/src/components/StartupAnimationOverlay.vue`:

- `MIN_VISIBLE_MS = 360`
- `SIGNATURE_MS = 900`
- `EXIT_MS = 180`
- `SKIP_UNLOCK_MS = MIN_VISIBLE_MS`

Behavior:

- On `bootPhase === 'booting'`, show the shell veil and startup rail.
- If `ready` arrives before `MIN_VISIBLE_MS`, hold only until the minimum threshold, then dissolve out.
- If `ready` arrives after `MIN_VISIBLE_MS` but before `SIGNATURE_MS`, exit immediately through the short dissolve.
- If boot is still active at `SIGNATURE_MS`, remove the full-screen veil and let `AppRuntimeBanner.vue` continue the story alone.
- If `error` occurs, stop any decorative motion immediately and exit the full-screen layer without waiting to “finish” the startup sequence.
- Keep click or key skip after `SKIP_UNLOCK_MS`, but make it clear that skip dismisses only the full-screen visual, not the real boot state.

The live startup copy visible in the rail should also be trimmed back to the original minimal set from the startup plan:
- `Restoring workspace state`
- `Initializing desktop services`
- `Mounting workspace shell`

The extra `Finalizing dataset readiness` stage currently present in `web/src/main.ts` should be merged back into that simpler set unless there is evidence that it materially improves trust.

## 5. Reduced-motion and error-state behavior

Reduced-motion handling should follow the shell’s existing posture. `web/src/styles/app.css` already disables certain UI transitions under `prefers-reduced-motion`; the startup replacement should do the same at a stricter level.

In reduced motion, the rail should use:
- the static mark
- static text
- only a subtle dissolve

Do not use:
- sweep
- pulsing
- bar travel
- depth-simulating blur choreography
- looping brand motion

On error, the rail should stop being decorative and become transitional only. The full-screen layer exits quickly, and `AppRuntimeBanner.vue` plus existing runtime diagnostics become the sole active status surfaces. Startup should never hold the shell hostage after failure just to preserve visual continuity.

## 6. Asset usage rules

### Use now

Use the existing **static** VectorAlpha mark as the default startup asset, together with current theme tokens from `web/src/styles/app.css` and the Signal Foundry palette. A single subdued accent line or contour reveal is allowed if it is local to the startup rail and disappears with it.

### Defer

Create a startup-specific one-shot or mono/mineral-blue SVG only if the shell-first rail lands correctly but still feels too plain. If that happens, the asset should play once and freeze; it should not become a looping hero object.

### Do not use

Do not use the current startup as:
- a looping full-screen brand hero
- a fake metric or pseudo-result scene
- a giant grid/lattice tableau
- a second persistent status strip
- a multi-color or purple-gradient showcase
- a new animation-library showcase

That prohibition is directly in line with the repo’s startup plan, which rejects fake metric cards and repeated ambient motion, and with the theme proposal, which rejects continuous shimmer/glow, decorative parallax, and consumer-fintech styling.

## 7. Implementation order by file, component, and store

### Phase 1 — `web/src/components/StartupAnimationOverlay.vue`

Rewrite this component as the primary implementation surface:

- remove the faux-analysis scene logic
- keep the local overlay state machine
- render the shell veil plus compact startup rail
- keep timing constants local
- keep all startup-only CSS local to the component
- implement the reduced-motion branch here
- keep skip behavior here
- keep handoff timing here

### Phase 2 — `web/src/main.ts`

Keep the current early-mount truth model, but tighten the stage language:

- preserve early `app.mount(root)`
- preserve the paint-safe `waitForNextPaint()` readiness handoff
- collapse boot-stage copy back to the intended minimal set
- keep `markReady()` and `markError()` tied to real runtime truth only

### Phase 3 — `web/src/components/AppRuntimeBanner.vue`

Edit only if needed:

- align the rail’s corner geometry, spacing, and accent grammar with the runtime banner
- prevent duplicate visible messaging during long-boot handoff
- keep the banner as the only long-tail and error owner

### Phase 4 — `web/src/stores/runtimeStore.ts`

Keep the store minimal:

- retain `idle | booting | ready | error`
- retain `bootStage`
- do not add a second animation-specific global model

### Phase 5 — `web/src/App.vue`

Keep topology unchanged:

- retain the startup overlay near the top of the tree
- retain its placement above titlebar, runtime banner, and shell
- do not turn this into a shell-architecture rewrite

### Explicitly avoid broadening scope

Do not make this change depend on `DesktopTitlebar.vue`, `VueWorkspaceShell.vue`, or a broad `app.css` rewrite unless a tiny shared token proves unavoidable. The startup plan explicitly treats those files as avoid-touch surfaces for this work.

## 8. Validation plan

Use the repo’s real frontend and desktop gates, plus focused startup-specific checks.

### Required command gates

- `npm --prefix web run lint`
- `npm --prefix web run typecheck`
- `npm --prefix web run test`
- `npm --prefix web run build`
- `npm --prefix web run a11y:smoke`
- `npm --prefix web run baseline:ui`
- `npm --prefix web run check:bundle-budget`
- `cargo check -p ta_desktop_demo_app`

### Focused component evidence

Update or add tests for `StartupAnimationOverlay.vue` that prove:

- it enters on `booting`
- it exits on `ready` after the minimum threshold
- it exits immediately on `error`
- reduced motion is effectively static
- it hands off to `AppRuntimeBanner.vue` at the signature ceiling
- skip dismisses only the full-screen layer
- it does not leave both startup rail and runtime banner competing on screen

### Browser-side proof

Keep baseline captures for:

- normal startup rail state
- reduced-motion startup rail state
- post-signature handoff with runtime banner visible

### Desktop proof

Use the real Tauri shell to confirm:

- the veil stays above the custom titlebar
- the shell is visible underneath but not interactable while the veil is active
- the handoff is quick and credible on fast launches
- the runtime banner cleanly owns long-boot and error posture
- no extra dependency or bundle regression was introduced

Future visual work should continue to meet the repo’s stricter bar: reduced-motion handling, scanability, first-paint discipline, and no decorative flourish without practical value.

## 9. Explicitly rejected startup ideas

Reject all of the following for this repo:

- a multi-window Tauri splash as the near-term fix
- a long fixed-duration startup sequence or any multi-second hold designed to “finish” the animation
- a faux-analysis scene with fake metrics, fake results, or pseudo-optimization theater
- a looping full-screen animated logo hero
- a second persistent boot surface that duplicates `AppRuntimeBanner.vue`
- a new animation runtime dependency for the near-term replacement
- any startup design that becomes visually louder than the shell it introduces

The repo should not try to rescue startup by making it more theatrical. The winning move is to make it more **truthful, faster to disappear, and more visibly part of the workstation**.

## Evidence basis

### Repo files consulted

- `Frontend Review/startup_animation_plan_2026-03-30.md`
- `Frontend Review/frontend_theme_proposal_2026-03-28.md`
- `web/src/main.ts`
- `web/src/App.vue`
- `web/src/components/StartupAnimationOverlay.vue`
- `web/src/components/VectorAlphaLogo.vue`
- `web/src/components/AppRuntimeBanner.vue`
- `web/src/stores/runtimeStore.ts`
- `web/src/styles/app.css`
- `Frontend Review/existing_stack_utilization_plan_2026-03-28.md`
- `Frontend Review/workstation_behavior_plan_2026-03-28.md`
- `Frontend Review/frontend_functional_coverage_matrix_2026-03-28.md`
- `Frontend Review/frontend_target_comps_2026-03-28.md`
- `Frontend Review/frontend_design_system_spec_2026-03-28.md`
- `web/README.md`
- `REPO_MAP.md`
- `AGENTS.md`
- `Frontend Review/frontend_ui_ux_research_prompt_pack_2026-04-02.md`

### External guidance consulted

- Apple Human Interface Guidelines, launching guidance
- Microsoft Fluent 2 motion guidance
- Carbon Design System loading patterns
- W3C WCAG reduced-motion guidance
