# Startup Experience Replacement Implementation Tracking

Created: 2026-04-07

Prompt source: [startup_experience_replacement_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt7/startup_experience_replacement_direction_FINAL.md)

Scope: Prompt 7 objective implementation only. This tracker converts the grounded Prompt 7 findings into a frontend implementation plan without reopening shell architecture, inventing a richer runtime model, or broadening startup into a standalone product surface.

Status: Draft tracker. Objective implementation not started.

Owner posture: Objective-first, truth-preserving, startup-discipline-biased.

## Purpose

Turn the grounded Prompt 7 startup findings into a detailed implementation plan that:

- replaces the current faux-analysis startup scene with a thinner, more shell-native startup treatment
- preserves the current truthful early-mount startup model
- keeps [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts) minimal
- keeps [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) as the long-tail and error-state owner
- tightens reduced-motion and failure handoff behavior
- keeps the implementation narrow, local, and easy to verify

This is not a shell redesign, not a new splash-window project, and not a reason to add a broader animation system.

## Functionality Posture

This implementation must be additive and capability-preserving.

- Do not delay real readiness just to finish an animation.
- Do not hide real startup failure behind a decorative layer.
- Do not break the current early mounted-app startup truth model in [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts).
- Do not remove [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) or broaden it into a general shell status bar.
- Do not add a second startup-specific global state model beside [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts).
- Do not spread startup-only styling into a broad rewrite of [app.css](C:/Rust%20Projects/ta_desktop_demo/web/src/styles/app.css).
- Do not touch [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue) or [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue) unless a tiny layering correction proves unavoidable.
- Preserve skip behavior, reduced-motion support, and boot/error handoff reachability.

## Progress Ledger

- [ ] Phase 0 complete: baseline audit and evidence capture finished
- [ ] Phase 1 complete: startup ownership, timing, and state audit landed
- [ ] Phase 2 complete: overlay visual simplification landed
- [ ] Phase 3 complete: timing and runtime-banner handoff tightening landed
- [ ] Phase 4 complete: reduced-motion and error-state hardening landed
- [ ] Phase 5 complete: startup verification and evidence refresh landed
- [ ] Objective checkpoint complete and ready for clean commit
- [ ] Optional interpretive follow-up reviewed
- [ ] Targeted verification complete
- [ ] Final evidence and screenshots attached

## Implementation Notes (2026-04-07)

- [ ] Objective Prompt 7 implementation has not started yet
- [ ] The current tracker is intentionally limited to the grounded startup findings reviewed from [startup_experience_replacement_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt7/startup_experience_replacement_direction_FINAL.md)
- [ ] The current faux-analysis overlay in [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue) is the primary replacement target
- [ ] The runtime truth chain in [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts) and [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts) is expected to remain substantially unchanged
- [ ] Any [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) edits should remain narrow and conditional

## Implementation Outcome Target

At the end of the objective checkpoint:

- the startup experience should feel thinner, calmer, and more product-native
- the real shell should still be the first meaningful frame
- the full-screen startup layer should stop behaving like a fake analysis tableau
- the overlay should hand off quickly and credibly to the normal shell
- `booting` and `error` should remain truthful runtime facts, not visual states invented for animation
- reduced-motion startup should be nearly static
- long boot and error should clearly become [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) territory
- the change should stay local enough that it can be verified and reverted without destabilizing unrelated frontend surfaces

## Checkpoint Strategy

### Checkpoint A — Objective Improvements

Land only the grounded, high-confidence changes first:

- baseline audit and evidence capture
- startup ownership and timing audit
- overlay visual simplification
- truthful timing/handoff tightening
- reduced-motion and error-path hardening
- verification and evidence capture

Checkpoint A should be suitable for a clean commit on its own.

### Checkpoint B — Optional Interpretive Follow-Up

Only after Checkpoint A is stable:

- decide the exact final top-left rail geometry and visual weighting
- decide whether the startup asset should be static-only or a one-shot restrained motion accent
- decide whether visible boot-stage copy should be reduced to the minimal three-stage set
- decide whether [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) needs small visual alignment tweaks to match the new overlay more closely

Checkpoint B should be easy to split, stash, or revert without threatening startup truth or runtime ownership boundaries.

## Objective-First Execution Rule

Execute this work in the following order:

1. Phase 0
2. Phase 1
3. Phase 2
4. Phase 3
5. Phase 4
6. Phase 5
7. Objective verification
8. Clean objective commit checkpoint
9. Optional interpretive follow-up

Do not mix asset experimentation or broader shell restyling into the objective checkpoint.

## Source Basis

- [startup_experience_replacement_direction_FINAL.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Prompt7/startup_experience_replacement_direction_FINAL.md)
- [startup_animation_plan_2026-03-30.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/startup_animation_plan_2026-03-30.md)
- [frontend_theme_proposal_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/Unfinished/frontend_theme_proposal_2026-03-28.md)
- [frontend_design_system_spec_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_design_system_spec_2026-03-28.md)
- [frontend_target_comps_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_target_comps_2026-03-28.md)
- [frontend_functional_coverage_matrix_2026-03-28.md](C:/Rust%20Projects/ta_desktop_demo/Frontend%20Review/frontend_functional_coverage_matrix_2026-03-28.md)
- [REPO_MAP.md](C:/Rust%20Projects/ta_desktop_demo/REPO_MAP.md)
- [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)
- [VectorAlphaLogo.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VectorAlphaLogo.vue)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts)
- [app.css](C:/Rust%20Projects/ta_desktop_demo/web/src/styles/app.css)

## Execution Guardrails

- Preserve the current startup truth chain:
  - `markBooting(...)`
  - early `app.mount(root)`
  - real bootstrap work
  - `waitForNextPaint()`
  - `markReady()` or `markError(...)`
- Keep the mounted app alive on failure so the runtime error posture can render.
- Keep startup-only CSS local to [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue) unless a tiny shared token is unavoidable.
- Treat [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) as the only long-tail boot and error owner.
- Keep [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts) limited to real runtime facts, not animation progress semantics.
- Avoid introducing a new animation library or startup runtime dependency during the objective pass.
- Preserve reduced-motion accessibility and skip behavior.
- Preserve bundle-budget discipline.
- Avoid backend or Tauri command-surface changes during the objective pass.

## Explicit Working Decisions

### 1. Startup truth model

- [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts) remains the owner of startup truth.
- `booting`, `ready`, and `error` remain real runtime states.
- `bootStage` remains real status copy, not fake cinematic narration.
- The overlay may react to runtime truth, but it must not invent readiness.

### 2. Ownership boundary

- [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue) remains the primary implementation surface.
- [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts) remains the only place that should change boot timing or phase transitions.
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) remains the owner of long-boot and error messaging.
- [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts) remains minimal and startup-truth-driven.
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue) keeps the overlay mounted at the top of the tree.

### 3. Visual replacement model

- Replace the current faux-analysis scene with a thinner shell-first treatment.
- Remove startup elements that imply fake analysis or fake result generation:
  - large grid tableau
  - sweeping scene motion
  - fake result bars
  - pulsing decorative metrics
- Keep the shell visible underneath a brief interaction-blocking veil.
- Keep startup visual language aligned with the current Signal Foundry direction:
  - restrained surface treatment
  - short useful motion
  - overlay-as-instrument-sheet posture

### 4. Timing model

- Keep timing constants local to [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue).
- Shorten the current full-screen hold so it behaves like a startup prelude, not a scene.
- Preserve a short minimum-visibility threshold only to avoid flash/flicker.
- Preserve fast error exit and quick ready exit.
- Keep skip as dismissal of the full-screen layer only, not a fake boot-completion control.

### 5. Reduced-motion model

- Reduced motion should use the static brand asset path or a visually equivalent static presentation.
- Reduced motion should avoid sweep, pulsing, animated bar travel, and layered scene motion.
- Reduced motion may keep a simple dissolve if that remains the calmest transition.

### 6. Error and long-boot model

- On `error`, the overlay should stop being decorative and exit quickly.
- On long boot, the overlay should not remain a second persistent status surface.
- The runtime banner must remain the sole long-tail and error status owner.
- Raw error detail remains behind disclosure in [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue).

## Expected Edit Inventory

Primary expected edit targets:

- [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)
- [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts)

Likely review-only or narrow-touch targets:

- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts)

Avoid-touch surfaces unless a concrete blocker appears:

- [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue)
- [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue)
- [app.css](C:/Rust%20Projects/ta_desktop_demo/web/src/styles/app.css)

## Implementation Phases

## Phase 0 — Baseline Audit And Evidence Capture

Goal: record the current startup behavior and identify exactly what the replacement must preserve.

Checklist:

- [ ] Capture the current normal startup overlay state
- [ ] Capture the current reduced-motion startup overlay state
- [ ] Capture the current long-boot handoff state if reproducible
- [ ] Capture the current error-state handoff if reproducible
- [ ] Record the current overlay timing constants and their user-facing effect
- [ ] Record every current faux-analysis element rendered by [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)
- [ ] Record which current startup behaviors already satisfy the startup plan:
  - early mount
  - truthful boot stages
  - skip behavior
  - reduced-motion branching
  - fast error exit
- [ ] Confirm current overlay layering over the titlebar, runtime banner, and shell
- [ ] Confirm current runtime banner visibility rules during `booting` and `error`
- [ ] Record current baseline asset usage:
  - animated logo path
  - static logo path
  - reduced-motion fallback

Acceptance criteria:

- [ ] There is a stable before-state reference for normal, reduced-motion, and at least one long-tail or error path
- [ ] The current timing and faux-analysis surfaces are explicitly documented
- [ ] The startup truth chain is documented before any changes land

## Phase 1 — Startup Ownership, Timing, And State Audit

Goal: lock what the frontend already knows about startup so the implementation remains a presentation tightening, not a runtime rewrite.

Primary files:

- [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts)
- [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts)
- [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)

Checklist:

- [ ] Audit the exact boot-stage sequence emitted by [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts)
- [ ] Confirm `app.mount(root)` still occurs before expensive or failure-prone startup work
- [ ] Confirm `markReady()` still waits for `waitForNextPaint()`
- [ ] Confirm `markError(...)` preserves a mounted UI path for error rendering
- [ ] Audit whether [runtimeStore.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/stores/runtimeStore.ts) already exposes everything needed for the replacement without new state
- [ ] Audit how [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) behaves during active boot while the overlay is still visible
- [ ] Audit whether duplicate visible messaging during long-boot handoff can be solved in the overlay alone
- [ ] Confirm [App.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/App.vue) placement is already correct for the replacement
- [ ] Avoid runtime-store expansion unless a concrete missing truth signal appears

Acceptance criteria:

- [ ] The implementation knows exactly which startup facts already exist
- [ ] The replacement can proceed without inventing new global animation state
- [ ] The banner handoff boundary is understood before overlay edits start

## Phase 2 — Overlay Visual Simplification

Goal: replace the faux-analysis startup scene with a thinner shell-first startup treatment.

Primary file:

- [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)

Checklist:

- [ ] Remove the large faux-analysis tableau from the overlay template
- [ ] Remove fake result bars and pseudo-analysis copy from the overlay
- [ ] Remove sweeping scene motion and other decorative moving layers
- [ ] Replace the current composition with a thinner shell-first treatment:
  - shell veil
  - compact startup status surface
  - restrained brand cue
  - real boot-stage text
- [ ] Keep the overlay visually subordinate to the shell it introduces
- [ ] Keep startup-only CSS local to the component
- [ ] Reuse existing theme tokens and current surface language rather than inventing a startup-only palette
- [ ] Keep visual emphasis on quick comprehension, not spectacle
- [ ] Preserve click and key skip behavior
- [ ] Preserve the shell-visible-but-blocked posture while the overlay is active

Acceptance criteria:

- [ ] The overlay no longer reads like fake analysis or fake results generation
- [ ] The startup surface is materially thinner than the current implementation
- [ ] The replacement still clearly communicates startup state and brand identity
- [ ] No shell architecture files were widened unnecessarily

## Phase 3 — Timing And Runtime-Banner Handoff Tightening

Goal: make the startup sequence short, truthful, and cleanly handed off to the runtime banner when needed.

Primary files:

- [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)
- [main.ts](C:/Rust%20Projects/ta_desktop_demo/web/src/main.ts)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)

Checklist:

- [ ] Shorten the current `MIN_VISIBLE_MS` and `SIGNATURE_MS` posture so the overlay stops behaving like a long scene
- [ ] Preserve a small minimum-visibility floor to prevent flash/flicker
- [ ] Preserve quick `EXIT_MS` handoff behavior
- [ ] Keep timing constants local to [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)
- [ ] Ensure `ready` causes exit as soon as the minimum threshold is satisfied
- [ ] Ensure long boot exits the full-screen layer and leaves [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) as the visible long-tail surface
- [ ] Ensure `error` exits the decorative layer immediately or near-immediately without waiting to finish motion
- [ ] Ensure skip dismisses only the full-screen visual and does not fabricate readiness
- [ ] Evaluate whether current visible stage copy should remain as-is during the objective checkpoint or be reduced without losing trust
- [ ] Touch [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) only if duplicate visible messaging cannot be prevented from the overlay side

Acceptance criteria:

- [ ] The startup layer exits quickly on real readiness
- [ ] The startup layer does not linger as a second boot surface on long boot
- [ ] Long-boot and error ownership clearly belong to the runtime banner
- [ ] Timing changes remain local and easy to reason about

## Phase 4 — Reduced-Motion And Error-Path Hardening

Goal: make the replacement more accessible and more truthful under non-happy-path conditions.

Primary files:

- [StartupAnimationOverlay.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/StartupAnimationOverlay.vue)
- [VectorAlphaLogo.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VectorAlphaLogo.vue)
- [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [app.css](C:/Rust%20Projects/ta_desktop_demo/web/src/styles/app.css) if a tiny shared token proves unavoidable

Checklist:

- [ ] Remove or neutralize nonessential startup motion under reduced motion
- [ ] Ensure reduced-motion startup is nearly static
- [ ] Ensure the reduced-motion variant still looks intentional, not broken
- [ ] Ensure the brand asset path under reduced motion remains calm and credible
- [ ] Ensure the overlay becomes transitional only on error
- [ ] Ensure the error path leaves runtime error detail reachable through [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [ ] Ensure no decorative startup element continues animating after error
- [ ] Confirm no broad global-motion rewrite is required

Acceptance criteria:

- [ ] Reduced-motion startup meets the stricter startup posture, not just the generic shell posture
- [ ] Error handoff is fast, readable, and not visually awkward
- [ ] The implementation does not broaden beyond the narrow startup scope

## Phase 5 — Verification, Evidence Refresh, And Tracker Reconciliation

Goal: prove the replacement works on the frontend and in the desktop shell, then reconcile this tracker.

Checklist:

- [ ] Add or update focused startup component tests
- [ ] Verify normal startup enter and exit behavior
- [ ] Verify reduced-motion startup behavior
- [ ] Verify skip behavior
- [ ] Verify error-path exit behavior
- [ ] Verify long-boot handoff behavior if reproducible
- [ ] Refresh browser-side screenshot evidence
- [ ] Refresh desktop-side screenshot evidence
- [ ] Reconcile this tracker so completed work is checked off accurately
- [ ] Record any intentionally deferred interpretive items in the optional section

Acceptance criteria:

- [ ] The implementation is proven by tests and by visual evidence
- [ ] The tracker reflects reality rather than template leftovers
- [ ] Objective completion is checkpoint-ready

## Objective Verification Checklist

Run after the objective implementation lands:

- [ ] `npm --prefix web run lint`
- [ ] `npm --prefix web run typecheck`
- [ ] `npm --prefix web run test`
- [ ] `npm --prefix web run build`
- [ ] `npm --prefix web run a11y:smoke`
- [ ] `npm --prefix web run baseline:ui`
- [ ] `npm --prefix web run check:bundle-budget`
- [ ] `cargo check -p ta_desktop_demo_app`

## Targeted Test Additions

- [ ] Update or add `StartupAnimationOverlay` tests for `booting` enter behavior
- [ ] Update or add `StartupAnimationOverlay` tests for `ready` exit after the minimum threshold
- [ ] Update or add `StartupAnimationOverlay` tests for immediate or near-immediate error exit
- [ ] Update or add `StartupAnimationOverlay` tests for reduced-motion behavior
- [ ] Update or add `StartupAnimationOverlay` tests for skip behavior
- [ ] Update or add tests that prove the runtime banner becomes the visible owner on long boot or error paths
- [ ] Update any startup-baseline or smoke harness assumptions if the old full-screen scene selectors no longer apply

## Manual Review Checklist

- [ ] Confirm the shell is visible under the startup veil from the first meaningful frame
- [ ] Confirm the shell is not interactable while the overlay is active
- [ ] Confirm the startup surface feels thinner and calmer than the current implementation
- [ ] Confirm the startup surface does not visually overpower the shell
- [ ] Confirm the runtime banner is the only long-tail or error owner
- [ ] Confirm reduced-motion startup is calm and legible
- [ ] Confirm the overlay still sits above the custom titlebar cleanly
- [ ] Confirm no visual duplication remains between overlay and runtime banner during handoff

## Guardrail Review Checklist

- [ ] Confirm no new startup-specific global store model was introduced
- [ ] Confirm no new animation runtime dependency was introduced
- [ ] Confirm no broad [app.css](C:/Rust%20Projects/ta_desktop_demo/web/src/styles/app.css) rewrite was required
- [ ] Confirm [DesktopTitlebar.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/DesktopTitlebar.vue) and [VueWorkspaceShell.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/VueWorkspaceShell.vue) remained untouched unless a narrow layering fix proved necessary
- [ ] Confirm the runtime banner still only surfaces `booting` and `error`
- [ ] Confirm bundle discipline remains intact after the replacement lands

## UX Pass/Fail Criteria

Pass if:

- [ ] Startup feels like a brief product-native prelude rather than a separate scene
- [ ] Readiness remains visibly truthful
- [ ] Error posture remains clearly visible and not trapped behind decoration
- [ ] Reduced motion is calmer, not merely slower
- [ ] The replacement looks intentional in both light and dark themes

Fail if:

- [ ] The startup still reads like fake analysis or pseudo-results generation
- [ ] The startup holds the screen too long after the app is ready
- [ ] The startup competes with [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) for ownership
- [ ] Reduced-motion startup still contains decorative motion
- [ ] The change forces broad shell, titlebar, or global-style rewrites

## Explicit Defers And Rejects

Defer from the objective checkpoint:

- startup-specific SVG asset experiments
- user-facing startup preferences
- native splash window evaluation
- new animation-library evaluation
- broader shell or titlebar styling changes

Reject in this pass:

- multi-second startup holds designed to finish the animation
- looping full-screen hero-logo startup
- fake metrics, fake result cards, or pseudo-optimization theater
- a second persistent startup status surface beside the runtime banner
- a broad startup-driven rewrite of the shell or global style system

## Optional Interpretive Follow-Up Checklist

These items were reasonable but more interpretive than repo-proven. Keep them out of the objective checkpoint unless the objective pass reveals a concrete need.

- [ ] Decide the exact final geometry, spacing, and weighting of the compact startup surface relative to [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue)
- [ ] Decide whether the default startup asset should be static-only or a one-shot restrained animation accent when reduced motion is off
- [ ] Decide whether the current `Finalizing dataset readiness` stage should remain visible or be merged back into a smaller visible boot-stage set
- [ ] Decide whether [AppRuntimeBanner.vue](C:/Rust%20Projects/ta_desktop_demo/web/src/components/AppRuntimeBanner.vue) needs small visual-alignment refinements to better match the new startup treatment
- [ ] Decide whether the replacement should preserve any tiny accent reveal beyond a pure dissolve
