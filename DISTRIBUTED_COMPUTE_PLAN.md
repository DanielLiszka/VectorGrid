# Distributed Compute Plan (LAN Leader/Workers)

This document plans a **local-network distributed computing** feature for the VectorTA Optimizer Tauri app.

## Status (v1 implemented)

As of this repo state, a first working implementation exists:

- UI: **Controls → Distributed** tab (`dist/index.html` + `dist/app.js`).
- Worker: embedded HTTP server (off by default) started/stopped from the UI.
- Leader: two modes (Double MA only):
  - **Assets**: dispatch one Double MA optimization per CSV path and aggregate results.
  - **Tiles**: split a single Double MA **Grid** run into tiles, dispatch, and merge reductions (best/top‑K).
- Dataset distribution: v1 assumes workers can read the same dataset paths (recommended: UNC / Windows file share).

### Worker HTTP API (v1)

- `GET /health` (no auth)
- `GET /capabilities` (Bearer token)
- `POST /v1/double_ma/run` (Bearer token)
- `POST /v1/cancel` (Bearer token) — cancels the currently running job on that worker

We want two (complementary) distribution modes:

1) **Asset-level distribution (preferred / default)**  
   Run many independent optimizations (e.g., 1,000 tickers/pairs) across a pool of workers, and aggregate results on a leader.

2) **Parameter-space tiling (optional / “huge grid” mode)**  
   Split a *single* enormous optimization run into tiles (chunks of the parameter grid), dispatch tiles to workers, and merge partial reductions (best/top‑K/heatmap) on the leader.

Both should be implemented **without regressions** to the current single-node workflows and without compromising the GPU kernel path (VectorTA CUDA remains a differentiator).

---

## Goals

- Keep current local behavior unchanged unless “Distributed” mode is enabled.
- Support **one leader** (the user’s machine) and **N workers** (same app on LAN).
- Workers run the same binaries/features as the leader (ideally identical machines; mixed capability is supported).
- Leader aggregates:
  - per-asset results (asset-level mode)
  - per-tile results merged into an exact final result (tile mode)
- Operate offline on LAN (no cloud services required).
- Maintain reproducibility:
  - stable “column id ↔ params” mapping where applicable
  - dataset/config hashing
  - record of code/version and build features (cuda / cuda-backtest-kernel)

---

## Non-goals (for v1)

- Internet-facing worker servers.
- Full “distributed samplers” for adaptive MO/TPE/NSGA‑II (can be added later; v1 focuses on deterministic grid workloads).
- Shipping `include_all=true` payloads over the network (too large; use artifact export instead).

---

## Key Concepts

### Roles

- **Leader**: UI-driven orchestrator that owns the job queue and aggregation.
- **Worker**: exposes a LAN RPC interface and executes one job at a time (or a limited concurrency configured by the user).

In v1, *both roles are the same Tauri app*, controlled by UI toggles.

### Work Units

- **Asset job**: “Run optimization X on dataset Y.”
- **Tile job**: “Run optimization X on (dataset Y, tile spec T).”

### Deterministic Column Mapping

For Double-MA grid searches we already have a stable index:

- `DoubleMaFastLtSlowIndex` in `crates/app_core/src/matrix_index.rs`
- Returned in results as `matrix_index`, with `best_col` and `top_cols`.

This is useful for:

- tile partitioning (by col ranges or by period chunks)
- stable export formats
- repeatable aggregation

---

## Architecture Overview

### Worker RPC Server (embedded)

Each worker runs a small HTTP+JSON server bound to a configurable address/port (default “off”):

- Bind address defaults to `127.0.0.1` unless the user explicitly enables LAN mode.
- When enabled, bind to `0.0.0.0` (or a selected interface) and show the URL in the UI.

Suggested endpoints (v1):

- `GET /health`
- `GET /capabilities`
  - build flags: `cuda`, `cuda-backtest-kernel`
  - runtime: `cuda_available`, device list
  - concurrency limit
  - dataset cache summary (optional)
- `POST /datasets/register`
  - register a dataset reference (shared path / UNC path / local path)
  - or “upload” mode (phase 2)
- `POST /jobs/submit`
  - returns `job_id`
- `GET /jobs/:job_id/status`
- `GET /jobs/:job_id/result`
- `POST /jobs/:job_id/cancel`

Implementation choices:

- Prefer **HTTP+JSON** (serde) for fastest iteration + easy debugging.
- Keep the worker engine in `ta_app_core` (pure Rust) and keep networking glue in `src-tauri`.

### Leader Scheduler (embedded)

Leader maintains:

- configured list of workers (manual list + discovery)
- worker heartbeats + health
- job queue + dispatch
- retries/backoff
- aggregation + export

Leader does not need to run a server; it can be client-only.

---

## Distribution Mode 1: Asset-Level (recommended)

### Use case

- You have 1,000 polygon datasets (tickers/pairs).
- You want to run 1,000 optimizations (same configuration).
- You have 10 machines on LAN; run ~100 jobs per machine and aggregate.

### Why it’s ideal

- Each job is independent (no tricky merge semantics).
- Most robust to worker failures (retry a single asset).
- Great throughput scaling.

### Result aggregation

Leader stores:

- per-asset `BackendOptimizationResult` + metadata:
  - dataset id/path/hash
  - runtime
  - backend used
  - best/top/heatmap (optional)

Leader can produce:

- cross-asset leaderboard (best Sharpe, best PnL, etc.)
- export to JSON/CSV/Parquet (later)
- feed into existing `meta_select` / cross-ticker tooling (later)

---

## Distribution Mode 2: Parameter-Space Tiling (for huge grids)

### Scope (v1)

Only **deterministic Grid** mode for:

- Double-MA (CPU / GPU-sweep / GPU-kernel)
- Composite CPU (Grid only)

### Tile partitioning options

1) **Period chunk tiles** (recommended)
   - tile = `(fast_periods chunk) × (slow_periods chunk)`
   - best MA-matrix reuse inside a worker
   - matches existing tiling loops

2) **Column range tiles**
   - tile = `col_start..col_end` based on `DoubleMaFastLtSlowIndex`
   - easier to describe, but may lead to worse locality unless mapped back to period chunks

### Exactness and merge logic

Workers return a reduced `OptimizationResult` for their tile:

- `best_params`, `best_metrics`
- `top` with `top_k = global_top_k`
- `heatmap` with the same bin mapping as leader requested (optional)
- `num_combos` processed

Leader merges:

- best = max(best score)
- top-K = merge of all tile top lists, then global top-K
  - safe if each tile returns `top_k = global_top_k`
- heatmap = elementwise max (same binning)

Constraints:

- `include_all` is disabled for distributed tiling.
- If any worker cannot compute the tile (missing feature/cuda), leader can reassign or fall back locally.

---

## Dataset Distribution (LAN)

We need a reliable way for workers to access the same candles:

### Option A: Shared network path (preferred)

- Store CSVs in a Windows file share (UNC path).
- Leader sends UNC path to workers.

Pros: simplest, fastest.  
Cons: requires setting up shares + permissions.

### Option B: Worker-local cache keyed by content hash

- Leader computes a dataset hash (sha256).
- If worker doesn’t have it cached, leader uploads it once (compressed) to the worker.

Pros: no share required; robust.  
Cons: implement upload + storage; network heavy initially.

### Option C: Worker self-download (Polygon)

- Leader sends polygon request parameters.
- Workers download using their own API key.

Pros: avoids large transfers.  
Cons: complicated key management; non-goal for v1.

Recommendation: implement **A first**, design protocol to allow **B** later.

---

## Discovery

v1: manual worker list

- User enters worker URLs (e.g., `http://192.168.1.50:48991`) in the Distributed tab.
- Leader pings `/health` and `/capabilities`.

v2: optional discovery

- mDNS/zeroconf or UDP broadcast “who is a worker?”
- Show discovered workers and allow “Add”.

---

## Security / Safety

Even on a LAN, running an RPC server needs guardrails:

- Default “Worker server” is **OFF**.
- Default bind `127.0.0.1`.
- Enabling LAN bind shows a warning (Windows Firewall prompt likely).
- Require a **pre-shared auth token** for all non-GET endpoints.
- Optionally restrict to private subnets and/or allowlist leader IPs.

---

## Scheduling & Load Balancing

Asset-level:

- simple work stealing: dispatch next asset to next idle worker
- optional “weighted workers” (GPU nodes get the bigger jobs)

Tile-level:

- tile sizes should be chosen to:
  - keep network overhead negligible vs compute
  - avoid VRAM oversubscription on GPU kernels
  - keep retries cheap

Leader should record per-worker throughput and adapt chunk sizing over time (v2).

---

## UI Plan (Tauri) — “Distributed” Tab

All distributed-compute UI lives under a dedicated tab in the main app.

### Distributed Tab layout

Sub-tabs inside Distributed:

1) **Leader**
   - Worker list (add/remove, ping, status)
   - Run mode selector:
     - Asset-level
     - Parameter tiling
   - Dataset set (e.g., polygon cache selection / list of CSVs)
   - Job config (strategy kind, backend preference, objective, ranges, top‑K, heatmap)
   - Queue controls (start/pause/cancel)
   - Aggregation view (progress, per-worker throughput, failures)
   - Export aggregated results

2) **Worker**
   - Toggle “Enable worker server”
   - Bind address + port
   - Auth token (generate/copy)
   - Capabilities panel (CUDA availability, devices)
   - Current job status + cancel
   - Dataset cache management (later)

### Integration rule

No distributed code runs unless the user explicitly enables it in this tab.

---

## Implementation Phases (recommended)

### Phase 0: UI skeleton + types

- Add “Distributed” tab with placeholder.
- Add shared Rust types for:
  - `WorkerCapabilities`
  - `JobSpec` (`AssetJobSpec`, `TileJobSpec`)
  - `JobResult` (reduced)
  - `AggregatedRun` state

### Phase 1: Worker server MVP + leader client MVP (asset-level only)

- Start/stop worker server from UI.
- Add manual worker list on leader.
- Leader dispatches asset jobs for **Double-MA** only.
- Aggregates per-asset results into a table + export JSON.

### Phase 2: Dataset access strategy A (shared paths) + caching metadata

- Leader verifies workers can access dataset paths (preflight check).
- Add dataset validation errors to UI.

### Phase 3: Tile-level distribution for Double-MA grid

- Add tile job type for Double-MA grid only.
- Parity test: distributed tile merge == local run on small grids.

### Phase 4: Extend to Composite CPU grid

- Same tiling/asset dispatch patterns.

### Phase 5: (Optional) discovery + upload caching + advanced scheduling

---

## Testing Plan

Unit tests (Rust):

- tile merge logic:
  - top‑K correctness
  - heatmap merge correctness
  - best correctness

Integration tests (local machine):

- spawn N worker servers on different ports locally
- leader dispatches jobs and aggregates
- verify deterministic equality vs local single-run (small grid)

Manual tests (LAN):

- firewall prompts + token auth
- disconnect/reconnect a worker mid-run; ensure retry works

---

## Open Questions

- Should the leader be allowed to run jobs locally while also scheduling workers? (likely yes)
- How do we represent “dataset identity” in UI?
  - ticker + timeframe + start/end + source
  - or file path + content hash
- How do we manage worker concurrency on GPU nodes? (default 1 GPU job at a time)
