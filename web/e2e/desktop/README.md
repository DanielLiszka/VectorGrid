# Desktop Smoke Harness

Windows-first desktop end-to-end smoke coverage for the Tauri shell.

## Local prerequisites

Required:

- Rust stable
- Node version compatible with [package.json](C:\Rust Projects\ta_desktop_demo\web\package.json)
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
npm --prefix web run test:desktop:smoke
```

The default smoke suite intentionally excludes the opt-in geometry resize spec until it proves stable enough for wider use.

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

Optional local overrides:

- `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1`
  - skip the explicit frontend/Rust build step if you already have a trusted debug binary
  - this is now just a local speed optimization; the managed fresh-build path works again
- `VECTORGRID_DESKTOP_SMOKE_BINARY=<absolute-or-repo-relative-path>`
  - point the harness at a specific desktop executable instead of `target\\debug\\ta_desktop_demo_app.exe`
- `VECTORGRID_DESKTOP_SMOKE_APP_URL=http://127.0.0.1:1420/`
  - override the preview/bootstrap URL if needed

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
  - inspect the `tauri-driver.log`, `preview.log`, and the failure screenshot under `output/wdio/desktop-smoke`
- desktop automation attached to a blank shell:
  - this harness already bootstraps `about:blank` to the managed preview host; if that still fails, inspect `preview.log` and verify `http://127.0.0.1:1420/` is reachable
- maximize/restore assertion fails intermittently:
  - treat that as a harness bug and keep flaky native assertions out of the default green path

## Safe local cleanup

If a prior run left processes behind, do not kill every `node` process.

Use scoped cleanup instead:

```powershell
Get-Process ta_desktop_demo_app,tauri-driver,msedgedriver -ErrorAction SilentlyContinue | Stop-Process -Force
```
