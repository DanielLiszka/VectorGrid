# Required CPU CI

The prepared working-tree GitHub Actions workflow is
`.github/workflows/cpu-quality.yml`. It is intended to run for pull requests,
pushes, merge-queue (`merge_group`) validation, and manual dispatches using the
`windows-2022` runner label. Remote activation and branch-protection
configuration remain pending until these working-tree files are committed and
pushed. The stable required check name is:

- `Required CPU quality`

Repository branch protection should require that exact check before merge.
Renaming the job requires updating branch protection at the same time.

## Contract

The workflow intentionally mirrors repo-owned commands instead of maintaining
a second test inventory:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/run_cpu_ci_gate.ps1
```

The wrapper fails fast and runs the Node/npm reproducibility and release-secret
checks, strict Clippy for `ta_strategies` and `ta_optimizer`, the canonical repo
quality gate, serialized Rust workspace tests, desktop and browser harness
typechecks, desktop-boundary contracts, serialized frontend tests, full
headless Chromium QA with WCAG 2.2/static/budget reporting, and deterministic
JSON/CSV/XLSX output parity. The repo quality gate also validates the canonical
`docs/release_authority_matrix.json` policy index with
`scripts/check_release_authority_matrix.mjs`; this is a non-benchmark contract
check and its deterministic report is staged with the current-run CPU evidence.

Workspace-wide Clippy remains advisory while the existing warning debt is
tracked and retired separately. The current snapshot is 184 `ta_app_core`
diagnostics and 66 Tauri diagnostics. New warning-free enforcement is fail-fast
for the currently clean `ta_strategies` and `ta_optimizer` crates; the required
job must not hide regressions in either crate behind the legacy workspace
warning backlog.

Rust compilation and tests use one Cargo job, incremental compilation is
disabled, `Cargo.lock` is checked with `--locked`, Rust test threads are
serialized, Vitest uses one worker without file parallelism, and the Playwright
configuration uses one headless Chromium worker.
The workflow installs the repo's stable Rust components, selects Node `22.12.0`
from the supported `web/package.json` engine range, installs the exact
`packageManager` npm version from that file, and installs dependencies with
`npm ci` from `web/package-lock.json`.
Every external action reference is pinned to a full commit SHA; the
GitHub-Actions entry in `.github/dependabot.yml` supplies weekly update pull
requests so those immutable pins can be reviewed and advanced intentionally.

At the start of each invocation, the wrapper creates a unique bounded evidence
root under `output/qa/required-cpu-quality/<run-id>`. Reproducibility and
secret-hygiene reports are written directly into that root. After their
respective gate steps, only the freshly generated frontend dashboard
JSON/Markdown, manifest/toolchain contract, release-authority matrix contract,
and headless output-parity manifest are copied into it, and a deterministic fixed-shape
`required-cpu-quality-summary.json` records the completed checks and hashes of
those explicit reports. The support builder receives only this run-specific
root; it never recursively bundles the accumulated `output/qa` directory.

The seven-day GitHub Actions artifact contains only the ZIP and checksum
produced by the exact-filename-allowlist, redacting support-evidence builder.
The builder itself performs no upload or network request. Its README and
manifest record the caller context and identify GitHub Actions—not the
builder—as the actor permitted to upload the final ZIP and checksum
automatically. A local invocation records that automatic upload is disabled.
Raw QA directories, browser traces, local paths, settings, datasets,
credentials, and crash dumps are not uploaded. This diagnostic evidence does
not replace release manifests, signatures, installer smoke evidence, or local
release authority.

Every bundle includes a redacted `reproduction/system-metadata.json`. The
disclosed fields are:

- product name and version;
- Git commit and dirty-working-tree boolean;
- process architecture, PowerShell version, .NET runtime description, current
  culture, and local time-zone identifier;
- operating-system caption, version, build number, and architecture;
- computer manufacturer and model;
- processor name, physical-core count, and logical-processor count;
- display-adapter name and driver version; and
- privacy flags confirming that computer/user names, environment dumps,
  credential-store contents, and absolute paths are excluded.

The metadata also contains its schema version and generation timestamp. It does
not contain a full environment dump, hostname, username, serial number, IP
address, credential value, dataset, or absolute local path.

## Separate lanes

The required CPU job deliberately does not run:

- CUDA builds, GPU correctness/performance gates, benchmarks, or profiling;
- native Tauri WebDriver or Computer Use tests;
- installer building, signing, timestamp verification, or updater publishing;
- clean-VM install, launch, shortcut, upgrade, or uninstall tests;
- credentialed Polygon/Alpaca sandbox integration.

Those require suitable hardware, credentials, foreground desktop ownership, or
release infrastructure and remain separate manual/dedicated-runner evidence.
The archived definitions under `.github/workflows.disabled/` are historical
references and must not be copied back as a substitute for these lanes.

### Polygon and Alpaca paper sandbox lane

The required CPU aggregator runs
`node --test scripts/provider_sandbox_preflight.test.mjs`, which uses loopback
mocks only. It does not read or contact a provider. Credentialed evidence stays
in the separate lane below.

Run the provider lane separately from the required CPU job:

```powershell
# Always safe for local/CI use: loopback mocks plus a credential-presence report.
# No provider network request is made.
powershell -File scripts/run_provider_sandbox_qa.ps1

# Opt in to credentialed GET requests. Polygon uses its read-only ticker
# reference endpoint. Alpaca is pinned to paper-api.alpaca.markets and reads
# account, clock, and a one-item order-reconciliation snapshot.
powershell -File scripts/run_provider_sandbox_qa.ps1 -IncludeCredentialed

# Release-evidence mode: fail unless every selected provider actually passes.
powershell -File scripts/run_provider_sandbox_qa.ps1 `
  -IncludeCredentialed `
  -RequireCredentialedEvidence
```

Polygon credentials are read from `POLYGON_API_KEY` or `POLYGON_KEY`. Alpaca
paper credentials are read from `APCA_API_KEY_ID` and
`APCA_API_SECRET_KEY`; the documented `ALPACA_*` aliases are also accepted.
Never put credential values in command-line arguments. Conflicting aliases or
an incomplete Alpaca pair fail closed.

The harness contains no order-submission route: it permits `GET` only, rejects
the Alpaca live host, rejects redirects, bounds response sizes and timeouts,
and persists only redacted/minimal evidence under
`output/qa/provider-sandbox/`. Missing credentials or unavailable network are
reported as explicit skips unless `-RequireCredentialedEvidence` is set.
Offline local-server tests cover authentication headers, rate limiting,
disconnect/reconnect, a closed market clock, response minimization, and secret
non-disclosure without contacting either provider.
