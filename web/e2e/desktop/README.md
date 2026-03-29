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

Then install or download a matching EdgeDriver and ensure `msedgedriver.exe` is on `PATH`.

## Run locally

From the repo root:

```powershell
npm --prefix web run test:desktop:smoke
```

Run a single spec:

```powershell
npm --prefix web run test:desktop:smoke:spec -- .\\e2e\\desktop\\specs\\desktop-shell-readiness.spec.ts
```

Debug runner logging:

```powershell
npm --prefix web run test:desktop:smoke:debug
```

## What the harness does

The default smoke path:

1. verifies local `tauri-driver` and `msedgedriver`
2. runs `npm --prefix web run build`
3. runs `cargo build -p ta_desktop_demo_app`
4. launches `target\\debug\\ta_desktop_demo_app.exe` through `tauri-driver`
5. executes the desktop smoke specs against the real Tauri shell

The harness deliberately avoids `cargo tauri dev` so it does not depend on dev-server timing or Tauri CLI cwd behavior.

Optional local overrides:

- `VECTORGRID_DESKTOP_SMOKE_SKIP_BUILD=1`
  - skip the explicit frontend/Rust build step if you already have a trusted debug binary
- `VECTORGRID_DESKTOP_SMOKE_BINARY=<absolute-or-repo-relative-path>`
  - point the harness at a specific desktop executable instead of `target\\debug\\ta_desktop_demo_app.exe`

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
  - install a driver that matches the local Edge build and put it on `PATH`
- session hangs during startup:
  - verify Edge and EdgeDriver versions match
- app launches manually but smoke fails before shell readiness:
  - inspect the `tauri-driver.log` and the failure screenshot under `output/wdio/desktop-smoke`
- maximize/restore assertion fails intermittently:
  - treat that as a harness bug and keep flaky native assertions out of the default green path
