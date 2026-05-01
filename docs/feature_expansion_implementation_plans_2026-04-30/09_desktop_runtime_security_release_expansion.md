# Desktop Runtime, Security, And Release Expansion Plan

Date: 2026-04-30

## Goal

Expand the desktop runtime and release posture with explicit command boundary checks, capability hardening, secret hygiene, release provenance, signing/updater readiness, and public documentation alignment.

## Current Owner Files

Runtime and command boundary:

- `src-tauri/src/lib.rs`
- `src-tauri/src/commands.rs`
- `src-tauri/src/command_bindings.rs`
- `src-tauri/capabilities/main.json`
- `src-tauri/capabilities/dev.json`
- `src-tauri/tauri.conf.json`
- `src-tauri/tauri.windows.conf.json`
- `tauri.conf.json`
- `tauri.windows.conf.json`
- `web/src/tauri/commands.ts`
- `web/src/tauri/generatedBindings.ts`
- `web/src/tauri/invoke.ts`
- `web/scripts/generate-tauri-commands.mjs`
- `web/scripts/check-known-tauri-commands.mjs`
- `web/scripts/check-tauri-parity.mjs`
- `web/scripts/check-tauri-capabilities.mjs`
- `web/scripts/check-tauri-command-risk-policy.mjs`
- `src-tauri/command-risk-policy.json`

Release:

- `docs/PUBLIC_RELEASE.md`
- `docs/RELEASE.md`
- `docs/public/*`
- `src-tauri/src/release_readiness.rs`
- `src-tauri/src/bin/ta_release_readiness.rs`
- `scripts/run_release_doctor.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/run_public_release_gate.ps1`
- `scripts/generate_release_manifest.ps1`
- `scripts/capture_release_provenance.ps1`
- `scripts/check_tauri_config_sync.mjs`
- `scripts/check_release_secret_hygiene.mjs`
- `scripts/check_node_reproducibility.mjs`
- `scripts/sign_windows_artifacts_with_signtool.ps1`

Secrets and privacy:

- `src-tauri/src/settings.rs`
- `src-tauri/src/redaction.rs`
- `src-tauri/src/local_path_policy.rs`
- `src-tauri/src/polygon.rs`
- `src-tauri/src/commands_execution.rs`
- `web/src/utils/redaction.ts`
- `web/src/utils/storage.ts`
- `web/src/utils/core/storageKeys.ts`
- `web/src/services/settingsService.ts`
- `web/src/stores/settingsStore.ts`

## Research Basis

- Tauri Runtime Authority enforces capabilities, permissions, and scopes before invoking commands: <https://v2.tauri.app/security/runtime-authority/>
- Tauri security guidance emphasizes the Rust/WebView trust boundary and validating data crossing IPC: <https://v2.tauri.app/security/>
- Tauri capability, CSP, deep-link, single-instance, WebDriver, and updater docs should be treated as release-surface inputs, not implementation afterthoughts: <https://v2.tauri.app/security/csp/>, <https://v2.tauri.app/plugin/deep-linking/>, <https://v2.tauri.app/plugin/single-instance/>, <https://v2.tauri.app/develop/tests/webdriver/>, and <https://v2.tauri.app/plugin/updater/>
- Tauri documents platform WebView versions separately; Windows releases should include a WebView2 availability/diagnostic plan: <https://v2.tauri.app/develop/webview-versions/>
- Tauri Windows signing docs distinguish code signing routes and SmartScreen implications: <https://v2.tauri.app/distribute/sign/windows/>
- Microsoft WebView2 deployment guidance and Smart App Control signing guidance should inform installer prerequisites and signing evidence: <https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution> and <https://learn.microsoft.com/en-us/windows/apps/develop/smart-app-control/code-signing-for-smart-app-control>
- Tauri updater requires signed updates and private-key management outside the repo: <https://v2.tauri.app/plugin/updater/>
- NIST SSDF supports integrating secure development, vulnerability risk mitigation, and release evidence into the SDLC: <https://csrc.nist.gov/publications/detail/sp/800-218/final>
- OWASP Secure Coding Practices provides technology-agnostic secure coding checklist guidance: <https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/>

## Expansion Track A: Command Boundary Risk Policy

### Problem

Command count and risk can grow faster than capability review.

### Implementation

Current owners:

- `src-tauri/src/lib.rs`
- `src-tauri/capabilities/main.json`
- `src-tauri/capabilities/dev.json`
- `src-tauri/command-risk-policy.json`
- `web/scripts/check-tauri-command-risk-policy.mjs`
- `web/scripts/check-tauri-capabilities.mjs`

Add or complete risk policy for every command:

- read-only;
- filesystem write;
- shell/open path;
- credential/security;
- broker/trading;
- live stream;
- release/system;
- debug/internal.

Each command should have:

- owner module;
- frontend call sites;
- capability expectation;
- redaction requirement;
- test coverage expectation;
- public release status.

### Tests

- every generated handler command appears in risk policy;
- every capability-exposed command has an allowed reason;
- every broker/security/write command has redaction coverage or explicit not-applicable reason.

## Expansion Track B: IPC Type Coverage

### Problem

Specta bindings exist but cover a pilot subset. Expansion should make command contracts harder to drift.

### Implementation

Current owners:

- `src-tauri/src/command_bindings.rs`
- `src-tauri/tests/specta_bindings_export.rs`
- `web/src/tauri/generatedBindings.ts`
- `web/scripts/check-specta-bindings.mjs`

Add coverage incrementally:

1. low-risk settings/system/data commands;
2. artifact/report commands;
3. validation/optimization request/response types;
4. options/ML/live execution only after schema review.

### Acceptance Criteria

- Generated bindings do not replace runtime validation where trust boundary risk remains.
- Optional fields preserve legacy artifacts.
- Frontend services consume typed contracts through app-specific adapters.

## Expansion Track C: Secret Hygiene And Local Data Inventory

### Problem

Release and support workflows need clear privacy posture and log redaction.

### Implementation

Current owners:

- `src-tauri/src/redaction.rs`
- `web/src/utils/redaction.ts`
- `scripts/check_release_secret_hygiene.mjs`
- `web/src/services/settingsService.ts`
- `web/src/stores/settingsStore.ts`
- `docs/public/13_privacy_security_local_data.md`

Add:

- local data inventory command;
- user-facing data directory summary;
- redacted support bundle dry run;
- secret scan across ignored release evidence and output folders;
- legacy browser-storage credential alias purge checks.

### Tests

- redaction removes API keys, bearer tokens, signing keys, private local paths where configured;
- data inventory does not read raw market data contents by default;
- support bundle is dry-run by default.

## Expansion Track D: Public Release Manifest And Signing Evidence

### Problem

Public release needs evidence that staged assets, resources, signatures, hashes, and variant labels match.

### Implementation

Current owners:

- `scripts/run_public_release_gate.ps1`
- `scripts/run_release_candidate_gate.ps1`
- `scripts/generate_release_manifest.ps1`
- `scripts/capture_release_provenance.ps1`
- `docs/PUBLIC_RELEASE.md`

Add:

- variant-specific resource requirements;
- signed installer evidence;
- staged artifact hash verification;
- bundled synthetic sample proof;
- PTX/native DLL manifest entries by variant;
- public docs coverage check.

### Tests

- CPU variant does not require CUDA resources at runtime;
- GPU-kernel variant requires double-crossover PTX;
- options PTX is only release-required when options GPU route is public/release-supported;
- native ML DLL requirement matches release variant.

## Expansion Track E: Updater Readiness Gate

### Problem

Updater is disabled by default. Enabling it needs a separate signed-update gate.

### Implementation

Before enabling updater:

- generate updater key outside repo;
- set `bundle.createUpdaterArtifacts`;
- configure public key and HTTPS endpoint;
- keep private key in release environment only;
- add update rehearsal from previous signed version;
- add rollback/withdraw docs.

Do not enable production updater endpoints for local dev builds.

### Acceptance Criteria

- Updater artifacts and signatures appear in release evidence.
- Private key never appears in repo, logs, or evidence.
- Manual clean-machine update test is documented and passed.

## Second-Pass Expansion: Capability Profiles And WebView2 Diagnostics

### Problem

Desktop expansion will add more commands and resources. A single capability profile and no WebView2 diagnostic surface makes it harder to distinguish development-only behavior from release behavior.

### Implementation

Capability profiles:

- keep `src-tauri/capabilities/main.json` release-focused;
- keep development-only commands in `src-tauri/capabilities/dev.json`;
- require `web/scripts/check-tauri-capabilities.mjs` to compare generated command inventory against both profiles;
- fail if a high-risk command is exposed in the release profile without a matching `src-tauri/command-risk-policy.json` entry.

WebView2 diagnostics:

- add a release-doctor check for WebView2 runtime assumptions;
- include WebView2 runtime status in local diagnostics, not in normal app chrome;
- document installer behavior for Evergreen runtime versus fixed version only after release packaging chooses one;
- include actionable copy for clean-machine smoke failures.

### Acceptance Criteria

- release and dev capability profiles are intentionally different;
- config sync check covers `src-tauri/tauri.windows.conf.json` and `tauri.windows.conf.json`;
- release doctor can explain missing WebView2 prerequisites without exposing private paths.

## Second-Pass Expansion: Deep-Link And File-Open Handoff

### Problem

Deep links and file-open events are useful for artifacts and support workflows, but they are untrusted inputs crossing the desktop boundary.

### Implementation

Only add after command policy and path policy are complete:

- define allowed URI schemes and file extensions;
- route all paths through `src-tauri/src/local_path_policy.rs`;
- parse deep-link payloads with a strict schema;
- reject network URLs unless a later product decision approves them;
- log only redacted payload summaries;
- treat duplicate app instances through single-instance handoff rather than parallel mutation.

Initial use cases:

- open a saved artifact;
- import a supported CSV;
- open release notes or local support bundle location.

Non-use cases:

- broker actions;
- execution start/stop;
- credential import;
- updater private key configuration.

## Second-Pass Expansion: Signing, Updater, And Support Bundle Readiness

### Problem

Release readiness spans signing reputation, updater keys, resources, logs, and support evidence. These should be separately gated.

### Implementation

Signing:

- separate unsigned local rehearsal, signed beta, and public signed release labels;
- record signer subject, timestamping status, and staged artifact hashes;
- keep PFX/private keys outside the repo and out of release evidence.

Updater:

- keep updater disabled until signed update artifacts are generated and rehearsed;
- require old-version-to-new-version clean-machine update proof;
- document rollback and withdrawal procedure.

Support bundle:

- provide a redacted dry-run manifest first;
- include config, command-risk summary, environment summary, release manifest, and recent logs;
- exclude raw market data, credentials, signing keys, updater keys, and broker secrets by default.

## Non-Goals

- No telemetry without privacy/product decision.
- No enabling updater as a side effect of unrelated packaging work.
- No weakening capabilities to make tests pass.
- No committing signing keys, PFX files, or updater private keys.

## Verification

Targeted:

- `npm --prefix web run security:desktop-boundary`
- `npm --prefix web run check:release-secret-hygiene`
- `npm --prefix web run check:node-reproducibility`
- `cargo test -p ta_desktop_demo_app --test specta_bindings_export export_specta_bindings -- --ignored`

Release:

- `powershell -File scripts/run_release_doctor.ps1 -Variant cpu`
- `powershell -File scripts/run_public_release_gate.ps1 -Variant cpu -AllowUnsignedBeta` for local rehearsal only.

Broad:

- `powershell -File scripts/run_repo_quality_gate.ps1`
