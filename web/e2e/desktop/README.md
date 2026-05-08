# Desktop Smoke Harness

Windows-first desktop end-to-end smoke coverage for the Tauri shell.

## Local prerequisites

Required:

- Rust stable
- Node version compatible with [`web/package.json`](../../package.json)
- `tauri-driver`
- `msedgedriver.exe` that matches the locally installed Edge version

Recommended install steps:

```powershell
cargo install tauri-driver --locked
```

Then install or download a matching EdgeDriver.

The harness accepts any of these locations:

- `VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER=<absolute-path>`
- repo-root `msedgedriver.exe`
- `PATH`

## Run locally

From the repo root:

```powershell
npm --prefix web run test:browser:ux
npm --prefix web run test:desktop:smoke
```

Use the browser UX suite for deterministic mocked-Tauri interaction proof, and use desktop smoke for real-shell/native verification.

For manual accessibility review, pair these desktop flows with the representative slices in [accessibility-review-matrix.md](../accessibility-review-matrix.md). Desktop review is only required when the touched change affects native-shell or OS-integrated behavior.

Run only desktop smoke:

```powershell
npm --prefix web run test:desktop:smoke
```

The default smoke suite intentionally excludes the opt-in geometry resize spec until it proves stable enough for wider use.

## Packaged smoke against a release-built executable

The desktop harness can already run against an existing desktop executable instead of rebuilding the debug app.

Use the repo wrapper for packaged smoke:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/run_packaged_smoke.ps1
```

That wrapper expects either:

- an explicit `-BinaryPath <path-to-release-exe>`, or
- a previously generated `manifest.json` in the selected release-candidate output directory

The release-candidate gate generates the manifest automatically before packaged smoke.
If the manifest says `artifact_source_kind: "release_binary"`, the smoke run is exercising the raw release-built executable rather than a bundle output. That is still useful evidence, but blank-shell failures in that mode need app-log confirmation because WebDriver may initially stay attached to `about:blank` even while the packaged shell is already loading.

Or run the harness manually against a specific executable:

```powershell
$env:VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD="1"
$env:VECTORGRID_DESKTOP_SMOKE_BINARY="C:\\path\\to\\ta_desktop_demo_app.exe"
$env:VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW="1"
npm --prefix web run test:desktop:smoke
```

`VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW=1` is important for packaged smoke because it disables the preview-host bootstrap fallback. In that mode the harness now gives the packaged shell a short grace period and, if needed, explicitly bootstraps the WebDriver context to `http://tauri.localhost/` instead of falling back to preview content.

Run a single spec:

```powershell
npm --prefix web run test:desktop:smoke:spec -- .\\e2e\\desktop\\specs\\desktop-shell-readiness.spec.ts
```

Debug runner logging:

```powershell
npm --prefix web run test:desktop:smoke:debug
```

Run only the opt-in geometry smoke:

```powershell
npm --prefix web run test:desktop:geometry
```

## What the harness does

The default smoke path:

1. verifies local `tauri-driver` and `msedgedriver`
2. runs `npm --prefix web run build`
3. runs `cargo build -p ta_desktop_demo_app`
4. starts a managed Vite preview host on `http://127.0.0.1:1420/`
5. launches `target\\debug\\ta_desktop_demo_app.exe` through `tauri-driver`
6. if the initial desktop context is blank, explicitly navigates that webview to the preview host
7. executes the desktop smoke specs against the real Tauri shell

The harness deliberately avoids `cargo tauri dev` so it does not depend on dev-server timing or Tauri CLI cwd behavior.

## Verification boundary

Desktop smoke should continue to own:

- real launch into the Tauri shell
- shell readiness in the native runtime
- titlebar and native window-control presence
- workflow, library, aggregate, diagnostics, live, settings, command-palette, and task-center reachability in the real shell
- native-shell portions of the manual accessibility review matrix when titlebar, geometry, notifications, or open/reveal path flows change

Desktop smoke should not become the primary proof layer for:

- general mocked-Tauri interaction journeys
- dense form/results interaction semantics
- broad screenshot assertions

Those app-level browser journeys belong in `web/e2e/browser/`, and exact interaction contracts still belong in component or unit tests.

Optional local overrides:

- `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1`
  - skip the explicit frontend/Rust build step if you already have a trusted debug binary
  - this is now just a local speed optimization; the managed fresh-build path works again
- `VECTORGRID_DESKTOP_SMOKE_BINARY=<absolute-or-repo-relative-path>`
  - point the harness at a specific desktop executable instead of `target\\debug\\ta_desktop_demo_app.exe`
- `VECTORGRID_DESKTOP_SMOKE_APP_URL=http://127.0.0.1:1420/`
  - override the preview/bootstrap URL if needed
- `VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW=1`
  - disable the preview host and blank-shell bootstrap path
  - use this when you want the smoke run to prove the packaged executable loaded its own frontend bundle

Recommended local command while geometry remains opt-in:

```powershell
npm --prefix web run test:desktop:geometry
```

Recommended full managed-build smoke command:

```powershell
npm --prefix web run test:desktop:smoke
```

## Failure artifacts

Artifacts are written under:

`output/wdio/desktop-smoke/`

Each run records:

- driver logs
- WDIO logs
- screenshots for failed specs
- a small environment summary

## Troubleshooting

- `tauri-driver` missing:
  - install it with `cargo install tauri-driver --locked`
- `msedgedriver` missing:
  - install a driver that matches the local Edge build and put it on `PATH`, in the repo root, or set `VECTORGRID_DESKTOP_SMOKE_EDGE_DRIVER`
- session hangs during startup:
  - verify Edge and EdgeDriver versions match
- app launches manually but smoke fails before shell readiness:
  - inspect the `tauri-driver.log`, `preview-server.log`, and the failure screenshot under `output/wdio/desktop-smoke`
- desktop automation attached to a blank shell:
  - in normal browser-backed smoke, the harness bootstraps `about:blank` to the managed preview host
  - in packaged smoke with `VECTORGRID_DESKTOP_SMOKE_DISABLE_PREVIEW=1`, the harness first gives the packaged shell a bounded grace period and then explicitly targets `http://tauri.localhost/`
  - if the packaged run still fails, inspect the copied app log under `desktop-smoke/logs/app/` before treating it as a packaging failure
- maximize/restore assertion fails intermittently:
  - treat that as a harness bug and keep flaky native assertions out of the default green path

## Safe local cleanup

If a prior run left processes behind, do not kill every `node` process.

Use scoped cleanup instead:

```powershell
Get-Process ta_desktop_demo_app,tauri-driver,msedgedriver -ErrorAction SilentlyContinue | Stop-Process -Force
```
