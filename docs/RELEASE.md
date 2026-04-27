# Release Notes

This document tracks release-time desktop integration requirements for VectorGrid.

## Desktop Updater

The updater integration is intentionally dormant in development and local builds.

- Rust plugin: `tauri-plugin-updater`
- Frontend bridge: `web/src/services/desktopUpdater.ts`
- Productization hook: `web/src/services/desktopProductization.ts`
- Current config: `src-tauri/tauri.conf.json` has `plugins.updater.active = false`

Before enabling update checks for a signed release:

1. Generate and store the Tauri updater signing key outside the repository.
2. Publish signed update artifacts and the update manifest to an HTTPS endpoint.
3. Set `plugins.updater.active = true`.
4. Add the production endpoint to `plugins.updater.endpoints`.
5. Set `plugins.updater.pubkey` to the public key matching the signing key.
6. Run `cargo check -p ta_desktop_demo_app`, `npm --prefix web run typecheck`, and the frontend QA gates before packaging.

Do not point local or test builds at a production updater endpoint. Keep update errors non-fatal at startup so a missing endpoint cannot block normal offline use.

## Native Desktop Plugins

Native shell integrations should use Tauri plugins instead of handwritten shell fallbacks:

- File and folder pickers: `tauri-plugin-dialog`
- Clipboard writes: `tauri-plugin-clipboard-manager`, with browser fallbacks for web-mode tests
- Opening or revealing files/directories: `tauri-plugin-opener`
- Single-instance focus handoff: `tauri-plugin-single-instance`
- Restart after an installed update: `tauri-plugin-process`

When adding frontend calls to a plugin, update `src-tauri/capabilities/main.json` and `web/scripts/check-tauri-capabilities.mjs` together.
