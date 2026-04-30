# System Requirements

VectorGrid public desktop releases target Windows x64.

## Supported Operating System

Recommended:

- Windows 11 x64.
- Windows 10 x64 with current system updates.

Required:

- Microsoft Edge WebView2 Runtime.
- Permission to install desktop applications.
- Enough disk space for the app, local market-data caches, run artifacts, and reports.

The installer is expected to use Tauri's WebView2 bootstrapper behavior when WebView2 is missing.

## CPU-Only Variant

The CPU-only variant is the broadest compatibility path.

It should not require:

- an NVIDIA GPU;
- NVIDIA drivers;
- CUDA Toolkit;
- NVCC;
- GPU runtime setup.

Use the CPU-only variant when:

- you are evaluating the app for the first time;
- your machine does not have a supported NVIDIA GPU;
- you want the most predictable install path;
- you do not need GPU-specific performance claims.

## GPU Variants

GPU variants are release-specific. Follow the exact release notes for the build you download.

In general, GPU builds require:

- a supported NVIDIA GPU;
- a compatible NVIDIA driver;
- enough VRAM for the selected workload;
- the GPU variant that matches the release claim.

GPU builds should not be described as universally faster. Some strategy and validation surfaces are CPU-preferred because measured overhead can exceed GPU benefit on smaller or more complex workloads.

## CUDA Toolkit

Public GPU kernel builds may use prebuilt PTX resources. When the release notes state that prebuilt PTX is bundled, users should not need to install the CUDA Toolkit just to run the app.

Developers and maintainers may still need CUDA Toolkit or NVIDIA profiling tools for building, profiling, or validating new CUDA work.

## Memory And Storage

Recommended minimums:

- 16 GB system RAM for normal research workflows.
- 32 GB or more for large datasets, larger shortlist exports, or broad validation runs.
- Several GB of free disk space for provider caches, artifacts, and reports.

Storage can grow over time because VectorGrid may keep:

- Polygon.io cached datasets;
- binary CSV caches;
- saved run artifacts;
- JSON, CSV, XLSX, and Markdown reports;
- local logs and release evidence if run by maintainers.

Users can manage data growth by deleting old artifacts, reports, and unused market-data caches.

## Data Provider Accounts

Local CSV workflows do not require a provider account.

Polygon.io workflows require a user-supplied Polygon.io API key. Entitlements, rate limits, available history, and endpoint access depend on the user's Polygon.io plan.

Alpaca workflows, if enabled in the release, require user-supplied Alpaca credentials for non-dry-run broker calls. Dry-run and local-paper workflows are separate from real broker submission.

## Internet Access

Internet access is needed for:

- downloading the installer;
- WebView2 bootstrapper installation if WebView2 is missing;
- Polygon.io historical downloads;
- Polygon.io live stock streams;
- Alpaca broker workflows;
- updater checks if a signed release enables the updater;
- opening documentation or support links.

The core local CSV workflow can run offline after installation.

## Display

Recommended:

- 1280 x 720 or larger desktop resolution.
- 1024 x 640 minimum app window size.

Large tables, heatmaps, and advanced workspaces are easier to use on larger displays.

