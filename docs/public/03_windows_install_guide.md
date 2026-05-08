# Windows Install Guide

Use this guide with a signed public VectorGrid Windows release.

First-run instructions that mention Simple Mode apply only to a release whose shipped UI implements and verifies Simple Mode. The current checked-in frontend defaults to the Advanced workbench, so do not use this page as current repository run instructions.

## Before You Install

Confirm that the release page includes:

- release version;
- variant label, such as CPU-only or GPU kernel;
- Windows architecture;
- publisher/signature identity;
- SHA-256 hashes;
- known limitations;
- privacy and disclaimer links;
- release notes for GPU requirements if you download a GPU variant.

Do not install a build from an untrusted source.

## Verify The Download

After downloading the setup `.exe`, verify the file hash:

```powershell
Get-FileHash .\VectorGrid-setup.exe -Algorithm SHA256
```

Compare the output with the release page or `SHA256SUMS.txt`.

Also inspect the Windows publisher/signature prompt. A public release should be Authenticode signed. Unsigned builds should be treated as private test builds unless the release page clearly labels them otherwise.

## Install

1. Run the setup `.exe`.
2. Follow the installer prompts.
3. If Windows prompts for WebView2, allow the bootstrapper to install it.
4. Launch VectorGrid from the Start menu or desktop shortcut if one was created.

New signed Windows applications can still trigger SmartScreen reputation prompts. Verify the publisher, hash, and download source before continuing.

## First Launch

On first launch for a Simple Mode release:

1. Start in Simple Mode.
2. Load the bundled synthetic sample or a local CSV.
3. Run a small CPU-safe optimization.
4. Optionally run validation.
5. Copy or save the Simple Markdown summary.

The bundled sample is synthetic workflow-verification data. It is not market data and not a trading recommendation.

## Updates

The updater is disabled by default in the repository configuration unless a signed release explicitly enables it.

If updates are enabled for a public release, the release notes should state:

- update channel;
- update host;
- whether updates are automatic or manual;
- how update signatures are verified;
- how to disable or skip update checks.

If no updater is enabled, install updates by downloading the next signed installer from the release page and running it manually.

## Uninstall

Use Windows Settings:

1. Open Settings.
2. Go to Apps.
3. Find VectorGrid.
4. Select Uninstall.

Uninstalling the app may not remove user-created local data such as provider caches, reports, saved run artifacts, and settings. See [Privacy, Security, And Local Data](13_privacy_security_local_data.md) for data locations and safe cleanup guidance.

## Install Troubleshooting

If installation fails:

- confirm the installer matches your Windows architecture;
- install all pending Windows updates;
- install or repair Microsoft Edge WebView2 Runtime;
- check antivirus or enterprise application-control policy;
- verify that the downloaded installer hash matches the release page;
- try the CPU-only variant if the GPU variant fails on launch.
