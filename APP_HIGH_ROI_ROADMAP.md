# High-ROI App Improvements Roadmap

_Last updated: 2026-01-28_

This roadmap tracks the **highest ROI** improvements for making a complicated optimizer feel **simple** in the
Tauri desktop app (across both strategies).

Guiding principles:
- Default to **hands-off** workflows (run → validate → report) with sensible presets.
- Surface **one recommended answer** (with evidence), but keep drilldowns available.
- Prefer improvements that add **clarity** without adding **compute cost**.

---

## Recently completed (already implemented)

- [x] Hands-off composite pipeline: optimize → validate → auto-save XLSX report (with top-N cap).
- [x] XLSX report polish: Dashboard + Shortlist (filters, freeze panes, conditional formatting).
- [x] XLSX report analytics: Distributions sheet (MA-pair counts, length histograms, fast-vs-slow scatter buckets).
- [x] Reports UX: “Open reports folder” + optional “reveal report after save”.
- [x] Presets: Quick/Balanced/Deep to set workflow defaults (without changing MA choices/ranges).

---

## P0 — UX that removes friction (highest ROI)

### 1) Run history + compare

- [x] Persist each run (req/res + validation + report path) to a local “runs” directory (not just localStorage).
- [x] Runs browser UI: list/load/reveal/delete + open runs folder.
- [x] Runs browser UI: search/tags + “open report” + “rerun”.
- [x] Compare UI: pick two runs → show deltas (recommended params + key metrics + robustness scores).

Value: makes iteration “fast” and reduces decision fatigue.

### 2) Progress/ETA + unified cancel

- [x] Standardize progress phases: optimize → validate → export → (optional) reveal.
- [x] ETA + throughput: backtests/sec (or evals/sec), rows/sec for exports.
- [x] A single Cancel button that cancels the current phase, across all long-running tasks.

Value: reduces uncertainty and improves perceived performance.

### 3) Batch runs (universe generator)

- [x] Batch runner: run optimize → validate → export XLSX across selected cached Polygon datasets.
- [x] Batch UX: dataset search + select all/none + “stop after current”.
- [x] Batch artifacts: auto-tag runs (per batch + per symbol) to make meta filtering easy.
- [x] Batch holdout split: tag a holdout subset and auto-run meta with holdout evaluation.
- [x] Batch summary artifact: save a JSON summary and allow “retry failed”.

Value: makes it practical to build a large validated pool for the Meta “set-and-forget” shortlist.

---

## P0 — Data reliability (Polygon)

### Polygon incremental updates (current status)

- [x] Append-only updates are implemented for Polygon 1m datasets:
  - Cache manifest tracks `min_ts`/`max_ts`.
  - If the requested range is fully covered, the downloader uses cache only.
  - If cache covers the start of the requested range, it fetches only **newer** bars and appends (skip duplicates by timestamp).

### Remaining improvements (recommended)

- [x] Gap detection + selective refetch: detect intra-day gaps and (on Repair) refetch affected days to fill missing minutes when possible.
- [x] Data integrity: detect duplicate timestamps and out-of-order rows; auto-repair.
- [x] “Adjusted vs unadjusted” clarity for stocks: UI toggle + separate caches to prevent mixing.
- [x] UI feedback per dataset: last updated timestamp, covered date range, row count, and “needs refresh” indicator.

Value: avoids silent data issues and makes “update data” trustworthy.

---

## P1 — Better decisions without more compute

### 1) In-app “sanity charts” for picks

- [x] For the recommended candidate (and selected rows): show equity + drawdown + trade list in-app.
- [x] Add a “rejection checklist” section (PASS/FAIL vs current Top filters): min trades, max dd cap, exposure cap, etc.

Value: quickly rejects obviously bad behavior without opening Excel.

### 2) Local stability check (cheap neighborhood)

- [x] For top-1/top-N: evaluate a small neighborhood around MA lengths (and optionally key composite knobs).
- [x] Show “peak vs plateau” indicator (robustness by smoothness).

Value: detects fragile parameter peaks with minimal additional compute.

---

## P1 — Presets (simple user experience)

- [x] Add presets: `Quick`, `Balanced`, `Deep` that set optimization + validation defaults.
- [x] Keep “Advanced” controls collapsed by default.

Value: reduces option paralysis while keeping power-user depth.

---

## Open questions (track decisions)

- [x] Define where run artifacts live (app data `runs/` directory).
- [x] Decide retention policy for saved runs (keep newest 200 untagged; tags preserve older runs).
- [x] Decide default compare metrics (best + validation recommended + WF/MC/perturb deltas when available).

---

## P0 — UX polish (user audit, Jan 2026)

Goal: make the “happy path” obvious and reduce avoidable misclicks, without adding compute cost or hiding dev details.

- [x] Show a dynamic “this run will…” checklist near `Run optimization` (Optimize → (Auto-)Validate → (Auto-)Export XLSX → (Optional) Reveal).
- [x] Surface dataset identity in the Data tab (label/source, symbol/market/adjusted, timeframe, date range).
- [x] Show a live pre-run cost preview near MA ranges (fast/slow pair count; warnings when huge / includeAll).
- [x] Make Validate tab context-aware (disable `Run validation` and show a hint when there’s no compatible current run).
- [x] Cross-ticker UX: add search + select all/none + selected count (and a “match current dataset” helper if applicable).
- [x] Add always-visible quick actions to open the latest report / latest meta report near Status.
- [x] Add lightweight tooltips (no new UI panels) for high-impact jargon (`eps_rel`, `allow flip`, `trade on next bar`, `CSCV/PBO`, etc.).

---

## P2 — Developer benchmarks (common workflows)

Purpose: keep performance regressions visible and provide “workflow” benchmarks that reflect real app usage.

- [x] Add workflow benches for composite CPU optimization (`ta_app_core` bench: `composite_cpu_opt`).
- [x] Add workflow benches for composite validation toggles (`ta_app_core` bench: `composite_validation_workflow`).
- [x] Add workflow benches for meta filter/rank across many artifacts (`ta_app_core` bench: `meta_select_workflow`).
- Data used: deterministic synthetic `Candles` (drift + sinusoidal noise) to keep benchmarks reproducible and avoid file I/O/licensing.
