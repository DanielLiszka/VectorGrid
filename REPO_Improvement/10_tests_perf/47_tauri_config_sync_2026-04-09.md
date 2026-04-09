# Tauri Config Sync Plan

Last updated: 2026-04-09

## Purpose

Define the mirror-sync contract between the canonical Tauri config and the root mirror file.

## Scope Boundaries

In scope:

- the sync checker `scripts/check_tauri_config_sync.mjs`
- the canonical config `src-tauri/tauri.conf.json`
- the root mirror `tauri.conf.json`
- shared-field expectations including windows, build, and security-related fields

Out of scope:

- options-related planning distinctions
- desktop smoke behavior itself
- broader build/release docs beyond config sync
- runtime command parity, which is a separate concern

## Current User-Facing Behavior

The repo already keeps a script that compares the canonical Tauri config under `src-tauri/` with a root mirror config, allowing for the expected `frontendDist` path difference while checking shared fields structurally. This is valuable because dual configs are easy to let drift, but the contract needs a doc that explains why the mirror exists and what sync actually proves.

## Current Code Surface

- `scripts/check_tauri_config_sync.mjs` performs the canonical-vs-mirror comparison.
- `src-tauri/tauri.conf.json` is the canonical config for the app.
- `tauri.conf.json` is the root mirror file that must stay aligned on shared fields.
- `RELEASE.md` and build docs rely on these configs remaining consistent enough for the documented workflows to work.

## Current Verification Surface

- `node scripts/check_tauri_config_sync.mjs`
- manual diff review of both config files when app windows, build commands, or security capabilities change
- desktop build/smoke workflows when config changes are substantial

## Strengths Already Present

- the sync checker already normalizes and compares the important shared fields structurally
- the expected `frontendDist` difference is documented in code rather than left as a mystery
- config drift can be caught quickly without building the whole app first
- the script keeps mirror-sync concerns separate from runtime smoke concerns

## Weaknesses / Failure Modes

- two config files remain a maintenance burden even with a sync script
- the checker only proves shared-field sync, not that the config choices are semantically correct
- contributors can still forget to run the script when config files change
- if Tauri config structure evolves, the checker shape may need to evolve too

## Regressions To Avoid

- letting the root mirror and canonical config drift on shared fields
- assuming config sync implies runtime behavior is automatically correct
- editing one config and updating docs without checking the other
- loosening the checker shape until real drift is tolerated silently

## External Research Summary

- [Tauri config reference](https://v2.tauri.app/reference/config/) is relevant because both checked-in config files are supposed to represent the same core Tauri app contract.
- [Tauri capabilities overview](https://v2.tauri.app/security/capabilities/) is relevant because security capabilities are part of the compared config shape.
- [Node.js `path` API docs](https://nodejs.org/api/path.html) are relevant because the sync script reasons about canonical vs root-relative paths.
- [Cargo build command docs](https://doc.rust-lang.org/cargo/commands/cargo-build.html) are relevant because build-command and frontend-dist config values directly affect the documented release paths.

## Proposed Improvement Approach

1. Keep one canonical config and one constrained mirror rather than letting both behave as independent authorities.
2. Preserve structural sync checking over the shared fields that matter for build and runtime setup.
3. Document clearly what the sync script proves and what still requires smoke or build verification.
4. Review any config-structure change against the checker, the build docs, and the desktop harness together.

## Checklist

- [ ] Document why both config files exist and which one is canonical.
- [ ] Document the expected `frontendDist` difference explicitly.
- [ ] Keep the sync checker current with Tauri config shape changes.
- [ ] Keep this config-sync plan non-options only.
- [ ] Run the sync checker whenever config files or build commands change.
- [ ] Avoid implying that sync alone proves runtime correctness.

## Verification Plan

- run `node scripts/check_tauri_config_sync.mjs` after config changes
- inspect both JSON files directly when the checker fails
- run relevant build or desktop smoke paths when config changes are behaviorally significant
- confirm docs still match the synced config state

## Rollout / Fallback / Compatibility Notes

- preserve the canonical-plus-mirror pattern while it still serves build/documentation needs
- prefer tightening the checker over relying on convention
- if the mirror becomes unnecessary in the future, remove it deliberately rather than letting it rot

## Related Docs

- `REPO_Improvement/10_tests_perf/42_tauri_command_parity_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/01_release_build_variants_2026-04-09.md`
- `REPO_Improvement/11_docs_ops/07_local_vs_ci_authority_2026-04-09.md`
