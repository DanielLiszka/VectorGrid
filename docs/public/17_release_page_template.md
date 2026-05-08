# Release Page Template

Use this template for a public VectorGrid Windows release page.

Before using the Simple Mode wording below, confirm the release implements and verifies Simple Mode as the shipped first-run surface. Replace those sections with Advanced-workbench first-run instructions if that is what the release actually ships.

```markdown
# VectorGrid <version>

Release date: <YYYY-MM-DD>

This is a <CPU-only / GPU MA / GPU kernel> Windows x64 release of VectorGrid.

VectorGrid is local research software for strategy optimization, validation evidence, and exportable run artifacts. It is not financial advice and not a production trading system.

## Download

| Artifact | Description | SHA-256 |
|---|---|---|
| `VectorGrid-<version>-setup.exe` | Signed Windows x64 installer | `<hash>` |
| `SHA256SUMS.txt` | Hash list | `<hash>` |

Publisher/signature: `<publisher>`

## Requirements

- Windows 10/11 x64.
- Microsoft Edge WebView2 Runtime.
- CPU-only variant: no NVIDIA GPU or CUDA required.
- GPU variant: <driver/runtime requirements>.

## Install

1. Download the installer.
2. Verify the SHA-256 hash.
3. Confirm the Windows publisher/signature.
4. Run the setup executable.
5. Start in <Simple Mode / the documented first-run workspace>.

Install guide: <link>

## First Run

Recommended first workflow:

1. Load the bundled synthetic sample.
2. Run Double MA optimization in <Simple Mode / the documented first-run workspace>.
3. Optionally run validation.
4. Copy or save the Simple Markdown summary.

## What's Included

- <Simple Mode / documented first-run> workflow for data -> optimize -> validate -> summary.
- Local CSV loading.
- Bundled synthetic sample.
- Polygon.io historical cache workflows for supported markets.
- Advanced workbench for deeper strategy and validation controls.
- Local reports and saved artifacts.
- <GPU route summary if applicable>.

## Known Limitations

- Research software only; not trading advice.
- Windows x64 only.
- Polygon.io access depends on user plan and API key.
- Polygon live streaming is stocks-only.
- GPU acceleration is route-specific and not universal.
- <release-specific limitations>.

## Verification

Release gate evidence:

- Public release gate: `<passed / link / summary>`
- Release doctor: `<passed / summary>`
- Packaged smoke: `<passed / summary>`
- Output parity: `<passed / summary>`
- Signature verification: `<passed>`
- Clean-machine smoke: `<passed / pending>`

## Privacy

VectorGrid stores settings, caches, artifacts, and reports locally. Provider and broker workflows use user-supplied credentials. Review generated files before sharing.

Privacy notes: <link>

## Disclaimer

VectorGrid is research and analysis software. It is not financial advice, investment advice, legal advice, tax advice, or a recommendation to trade.

Disclaimer: <link>
Known limitations: <link>
License: <link>

## Support

For bugs and support questions, use GitHub Issues. For security reports, use private vulnerability reporting.

Support: <link>
Security: <link>
```

## Maintainer Publishing Checklist

Before publishing:

- release was built from a clean worktree;
- installer exists and is staged;
- installer is Authenticode signed;
- `SHA256SUMS.txt` matches staged artifacts;
- public release gate passed;
- packaged desktop smoke passed;
- clean-machine smoke was completed or explicitly marked pending;
- release notes name the exact variant;
- website/docs match the release;
- no maintainer credentials or private paths are bundled;
- known limitations are honest and visible.
