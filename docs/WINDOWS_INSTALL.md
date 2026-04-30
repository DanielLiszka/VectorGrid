# VectorGrid Windows Install Notes

Use these notes with a signed public VectorGrid Windows release.

## Requirements

- Windows 10 or Windows 11, x64.
- Microsoft Edge WebView2 Runtime. The installer is configured to use Tauri's `downloadBootstrapper` WebView2 install mode when WebView2 is missing.
- For CPU builds, no NVIDIA driver or CUDA Toolkit is required.
- For GPU builds, use the release notes for the exact NVIDIA driver/runtime expectations.

## Verify The Download

Before installing:

1. Download the setup `.exe` from the release page.
2. Verify the publisher shown by Windows matches the release notes.
3. Compare the file SHA-256 hash with `SHA256SUMS.txt`.

PowerShell hash check:

```powershell
Get-FileHash .\VectorGrid-setup.exe -Algorithm SHA256
```

## Install

Run the setup `.exe` and follow the installer prompts.

If Windows SmartScreen appears, verify the publisher, hash, and download source before continuing. New public builds can still need reputation time even when signed.

## First Run

Start in Simple Mode for the public workflow:

1. Load the bundled synthetic sample dataset, or browse to a local OHLCV CSV.
2. Choose Double MA crossover or Composite signal.
3. Run optimization and review the Simple run summary.
4. Optionally run validation to add holdout and robustness evidence.
5. Copy or save the Simple Markdown summary, or switch to Advanced Mode for detailed report exports.

The bundled sample is synthetic, redistributable research data for workflow verification. It is not market data, a trading recommendation, or financial advice.

## Local Data

VectorGrid stores local settings, provider configuration, caches, run artifacts, reports, and logs under the Windows application data location for the app identifier:

```text
dev.vectoralpha.vectorgrid
```

See `PRIVACY.md` for local-data and credential notes.

## Provider Setup

Provider-backed workflows require user-supplied provider credentials. Public builds must not ship with maintainer API keys or broker credentials.

## Trading Disclaimer

VectorGrid is research and analysis software. It is not financial advice, a recommendation to trade, or a production trading system.
