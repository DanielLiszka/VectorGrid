# Dataset catalogs, universes, and data-workflow parity for VectorGrid

## Executive summary

The highest-value dataset and universe-management capabilities in mature quantitative research tools cluster into three themes: (a) **explicit “dataset objects” with durable identity, metadata, and refresh history**, (b) **first-class “universe” objects that are reusable inputs across workflows**, and (c) **lineage/provenance that makes results reproducible and explainable**. These themes show up repeatedly across code-first quant platforms (notably universe selection in entity["company","QuantConnect","algorithmic trading platform"]’s LEAN, and ingested “bundles” in Zipline) and across research data systems (data catalogs/lineage standards like PROV, DCAT, and OpenLineage). citeturn0search0turn0search8turn0search1turn1search0turn1search1turn1search3

For VectorGrid specifically, the best-fit direction is an **additive “Catalog + Universes v1” layer** that sits above today’s CSV + entity["company","Polygon.io","market data api provider"] flows, preserves local-first caching, and improves cross-workflow reuse (Optimize → Validate → Meta/Holdout → Reports/History) by giving datasets and universes stable IDs and shared metadata. The implementation should borrow the *conceptual* patterns of (1) **ingest + cached store** (Zipline bundles), (2) **parameter-hash caching** (VectorBT Pro’s LMDB cache keys), and (3) **pools/universes as reusable instrument sets** (Qlib’s “instrument pools” notion) while deliberately staying bounded and desktop-friendly. citeturn0search1turn0search2turn3view1

Recommendation: **Implement Now** as a scoped v1, with “time-varying universes” and “full data-platform breadth (connector sprawl)” as explicit backlog/non-goals. Change shape: **Mostly Additive** (new catalog/universe domain modules + minimal touchpoints to reference IDs in artifacts and workflow requests).

## Exact question text

From `VECTORGRID_DEEP_RESEARCH_QUESTION_PACK_2026-04-09.md` (DR-11):

> “What are the highest-value dataset and universe-management capabilities in mature quant research tools, and which of those are worth bringing into VectorGrid? Focus on local dataset catalogs, provenance, refresh history, derived-series lineage, saved universes, benchmark baskets, and cross-workflow reuse. Separate true workflow gaps from nice-to-have data-platform breadth that does not fit the current product. Prefer additive catalog and reuse capabilities that preserve current CSV and Polygon flows.”

## Context and assumptions

VectorGrid’s repo documentation emphasizes a local-first desktop workflow with strong artifact discipline and existing CSV + Polygon data handling, including binary candle caching, derived dataset caching, and dataset identity/fingerprint metadata in artifacts (when available). For example, `REPO_MAP.md` states (select excerpts):

- “cached binary candle format is `.vta`” and “the binary cache lives under `app_local_data_dir/data_cache`”  
- “derived datasets are cached in memory as `base_id@timeframe`” and “when a base dataset changes, derived descendants are invalidated”  
- “run artifact save enriches artifacts with environment metadata plus dataset identity/content-fingerprint metadata when possible”  
- “repo-local CSV discovery for batch/meta flows comes from `list_repo_data_csvs`”  
- Polygon cache and refresh/repair semantics exist and live updates can mutate loaded datasets by `data_id` (not just file path). (All quotes are from `REPO_MAP.md`.)

Within this context, this answer assumes:

- **Dataset** means a concrete OHLCV-like time series (plus optional adjustments/metadata) with a stable identity, stored locally (raw CSV, provider cache, and/or internal `.vta` cache).
- **Derived dataset** means an internal transformation of another dataset (resample, filter session, adjust prices, etc.) that should be reproducible from a recorded transform spec, not “mystery state.”
- **Universe** means a reusable set of instruments/datasets used to run cross-asset validation, batch workflows, meta selection, and (eventually) portfolio/basket research. In v1, “universe” should be treated as a **named, saved list** (optionally with tags/roles like “benchmark” or “holdout”), not a fully programmable selection engine.

## Evidence from mature tools and standards

Mature quant tooling repeatedly separates **data ingestion/caching** from **strategy evaluation**, and treats “what instruments am I operating over?” as a first-class concept.

**Universe management patterns (dynamic selection and reuse).** LEAN explicitly models universe selection as a system that constructs “Universe objects” and notifies the algorithm when constituents change (via `OnSecuritiesChanged`), supporting reusable selection “models” and refresh intervals. citeturn0search0turn0search8turn0search4 This highlights an important generalizable idea even if VectorGrid remains UI-driven: **universes are reusable inputs**, and “what changed?” is part of the research record.

**Dataset ingestion + local canonical storage (bundles).** Zipline’s “data bundle” architecture requires an ingestion step (e.g., the `csvdir` bundle for OHLCV CSVs), after which Zipline uses the ingested bundle for backtests. This is a durable, locally stored “dataset package” concept, not ad hoc “load whatever file right now.” citeturn0search1turn0search33

**Cache keys derived from request parameters (hash-based identity).** VectorBT Pro documents disk caching using LMDB, with cache keys generated from a hash over the data-class identity and fetch parameters so different pulls cache separately. citeturn0search2 This is directly relevant to VectorGrid’s “dataset identity” and “refresh history” needs: if you can’t name a dataset version unambiguously, comparisons and reruns become untrustworthy.

**Instrument pools and append-only updates for time-series data.** Qlib’s paper formalizes two important ideas:

- Time-series financial data and “instrument pools” (universes) are core primitives; “S&P 500 Index” is cited as a typical pool example. citeturn2view0turn3view1  
- Data update is naturally **append-only** for historical time series, and Qlib’s storage/caching design supports efficient appends and reuse (including disk cache layers for computed expressions and combined datasets). citeturn3view1  

Even if VectorGrid does not adopt Qlib’s full “factor expression engine,” the *data update and pool/universe concepts* are portable.

**Provenance, catalog metadata, and lineage standards.** Three standards families are especially transferable because they give a vocabulary for what VectorGrid needs without prescribing a massive platform:

- **PROV (PROV-DM / PROV-O)**: a domain-agnostic model for provenance built around *Entity–Activity–Agent* and relations like “wasGeneratedBy” / “used.” citeturn1search0turn1search32  
- **DCAT v3**: a W3C recommendation for dataset catalog metadata and relationships (designed for interoperability between catalogs). citeturn1search1turn1search5  
- **OpenLineage**: an open standard that models *datasets, jobs, and runs*, with extensible facets—useful as a mental model for “dataset + run + metadata.” citeturn1search3turn1search11  
- Additionally, **Frictionless Data Packages** provide a pragmatic, lightweight “descriptor + resources” pattern for packaging datasets and metadata. citeturn1search2turn1search6  

### Comparison table

| Capability area | LEAN (QuantConnect) | Zipline | VectorBT Pro | Qlib | What it implies for VectorGrid |
|---|---|---|---|---|---|
| Universe as first-class concept | Universe selection produces Universe objects; constituents can change and are surfaced via events. citeturn0search0turn0search8 | Not primarily “universe objects,” but bundles support multi-asset datasets; universes often implicit via asset metadata and bundle choice. citeturn0search1 | Multi-symbol pulls exist; “universes” often user-defined groupings, not a core engine primitive in the same sense. (Data caching is first-class.) citeturn0search2 | “Instrument pools” are explicitly modeled; pools can vary over time; used in evaluation pipelines. citeturn2view0turn3view1 | Save universes as reusable objects; later optionally support time-varying membership. |
| Ingest → canonical local store | Platform-managed datasets; algorithms consume subscriptions. (Conceptually: ingestion is separate from evaluation.) citeturn0search32turn0search8 | Explicit bundle ingestion (e.g., `csvdir`); stored under a local Zipline directory. citeturn0search1turn0search33 | Pull + disk cache (LMDB) keyed by request hash. citeturn0search2 | Flat-file binary store and cache layers for reuse; designed for append updates. citeturn3view1 | Make “dataset registration/ingest” explicit for CSV + Polygon into `.vta` + manifest; unify access across workflows. |
| Refresh/update history | Universes refresh; data subscriptions are time-aware. citeturn0search4turn0search8 | Bundles can be re-ingested; refresh is conceptually an ingest event. citeturn0search5turn0search1 | Cache distinguishes pulls by parameters; refresh can be treated as a new cache entry/version. citeturn0search2 | Append-only updates are first-class; time-indexed store supports updates. citeturn3view1 | Add “RefreshEvent” records and dataset versions; expose “last refreshed” and “what changed” summaries. |
| Provenance/lineage vocabulary | Platform has implicit provenance via code + settings; not standardized. | Bundle metadata partially covers provenance; often ad hoc. | Cache key hash provides partial provenance but not a full lineage record. citeturn0search2 | Pipeline-driven system; provenance can be captured through config-driven components. citeturn0search3turn0search15 | Adopt a small PROV/OpenLineage-inspired internal schema; keep it local-first and exportable. citeturn1search0turn1search3 |

## Best fit for VectorGrid

### Direct answer

The most valuable dataset/universe features to bring into VectorGrid—while staying within its desktop, local-first, non-notebook constraints—are:

1. **Local Dataset Catalog (v1) with durable IDs and searchable metadata**  
   Minimal but high leverage: stable dataset identifiers, tags, coverage summary (time range, timeframe, bar count), source/provenance pointers (CSV path hash, Polygon request spec), and “quality state” (last validation/repair status). This aligns with DCAT/Data Package patterns (catalog metadata + dataset descriptors) while staying lightweight. citeturn1search1turn1search2

2. **Dataset versioning + refresh history as first-class records**  
   Ingest/refresh should produce a new “DatasetVersion” with its own fingerprint, and a “RefreshEvent” describing how it was produced (parameters, time, success/failure, appended range). This is consistent with append-oriented time-series assumptions described in the Qlib paper and with hash-key caching practice in VectorBT Pro. citeturn3view1turn0search2

3. **Derived-series lineage for resamples and computed variants**  
   Every derived dataset should have: parent pointer + transform spec + output fingerprint. This is a direct application of PROV’s “Entity derived from Entity via Activity,” and it prevents “silent transformations” from undermining reproducibility. citeturn1search32turn1search0

4. **Saved Universes and baskets (v1) as reusable workflow inputs**  
   Start with static universes: named lists of symbols/dataset IDs, taggable by role (benchmark, holdout, sector/theme). This mirrors the reusable “universe/instrument pool” idea in LEAN and Qlib without requiring code-based dynamic selection. citeturn0search0turn3view1

5. **Cross-workflow reuse by referencing catalog IDs in artifacts and requests**  
   The practical “parity win” is that a saved run can clearly say: “I used dataset version X and universe Y,” and the History/Compare UX can reuse that information without relying on fragile file-path re-discovery. This mirrors OpenLineage’s dataset/job/run triad as a conceptual framing for “dataset ↔ run ↔ metadata.” citeturn1search3turn1search11

### Why this matters for VectorGrid

VectorGrid already has strong optimization/validation, but the repo’s own competitive plan calls out dataset catalog and universe management as foundational parity gaps (e.g., “searchable local catalog,” “provenance metadata,” “last refresh time,” and “first-class saved universes … benchmark baskets … holdout baskets”). (Quotes from `VECTORGRID_COMPETITIVE_IMPROVEMENT_PLAN_VS_VECTORBT_PRO_2026-03-19.md`.) Closing this gap compounds value across nearly every workflow: it improves run comparability, enables consistent batch/meta inputs, reduces “what data did I actually run?” ambiguity, and sets the stage for future portfolio/basket work.

### Step-by-step reasoning

Step one is to separate **portable external patterns** from **repo-specific engineering choices**:

1. **Portable patterns (strong external precedent):**  
   - “Universe/pool” as a reusable object (LEAN, Qlib). citeturn0search0turn3view1  
   - “Ingested dataset package/bundle” as the unit of reuse (Zipline). citeturn0search1turn0search33  
   - Hash-based identity for cached pulls (VectorBT Pro). citeturn0search2  
   - Explicit provenance/lineage vocabulary (PROV, DCAT, OpenLineage, Data Package). citeturn1search0turn1search1turn1search3turn1search2  

2. **Repo-specific choices (needs local profiling/product constraints):**  
   - Whether catalog storage is JSON-on-disk, SQLite, LMDB, etc. External precedent exists, but the “right” choice depends on VectorGrid’s desktop constraints, schema evolution posture, and performance goals. (This part is product judgment rather than an established standard.)

3. **Filter by VectorGrid’s non-goals:**  
   Avoid “data platform breadth” such as connector sprawl or building a general-purpose factor store. That’s explicitly cautioned against in common data-platform practice (and is outside the question’s requested scope). Instead, emulate the *small* reusable abstractions: dataset descriptors, versions, and universes.

### A concrete design that stays additive

A minimal v1 can be expressed as a small internal schema that is **inspired by PROV/OpenLineage**, without adopting their entire ecosystems.

```mermaid
erDiagram
  DATASET ||--o{ DATASET_VERSION : has
  DATASET ||--o{ REFRESH_EVENT : has
  DATASET_VERSION ||--o{ DERIVATION : parent_of
  DATASET_VERSION ||--o{ RUN_INPUT : used_by
  UNIVERSE ||--o{ UNIVERSE_MEMBER : contains
  UNIVERSE ||--o{ RUN_INPUT : used_by

  DATASET {
    string dataset_id
    string display_name
    string source_kind   // csv | polygon | repo_sample
    string asset_class   // equity | fx | crypto
    string timezone
    string session_calendar
    string[] tags
  }

  DATASET_VERSION {
    string version_id
    string dataset_id
    string fingerprint_sha256
    datetime created_at
    datetime start_time
    datetime end_time
    int bar_count
    string timeframe
    string schema_version
  }

  REFRESH_EVENT {
    string refresh_id
    string dataset_id
    datetime started_at
    datetime finished_at
    string status      // success | failed
    string parameters_json
    string from_version_id
    string to_version_id
  }

  DERIVATION {
    string derivation_id
    string parent_version_id
    string child_version_id
    string transform_spec_json
  }

  UNIVERSE {
    string universe_id
    string name
    string role        // general | benchmark | holdout
    string[] tags
  }

  UNIVERSE_MEMBER {
    string universe_id
    string member_kind // symbol | dataset_id
    string member_key
    float weight
  }

  RUN_INPUT {
    string run_id
    string dataset_version_id
    string universe_id
  }
```

### Algorithms and code snippets needed

The central algorithmic requirement is **durable identity**.

A workable approach is to split identity into:

- **Dataset ID**: stable “logical dataset” identity (e.g., “AAPL 1m Polygon” or “CSV import from X”).  
- **Dataset version ID**: a specific materialization (and therefore what should be referenced by run artifacts and reports).

One minimal, robust pattern is:

- `dataset_id = base32(sha256(source_kind + canonical_source_locator))`  
  - For CSV: canonical locator could be normalized absolute path + file-size + mtime *plus* content hash once computed.  
  - For Polygon: canonical locator could be symbol + asset_class + requested timespan + provider settings.

- `version_id = base32(sha256(dataset_id + fingerprint_sha256 + schema_version))`

And define `fingerprint_sha256` as a stable hash over the internal `.vta` payload (or over a canonical serialization of OHLCV arrays + metadata).

Illustrative Rust-like pseudocode:

```rust
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

// Example: fingerprint over internal binary candle cache (.vta)
fn fingerprint_vta(vta_bytes: &[u8], schema_version: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(schema_version.as_bytes());
    hasher.update(vta_bytes);
    hex::encode(hasher.finalize())
}

// Example: stable dataset ID for a Polygon pull (logical dataset identity)
fn dataset_id_polygon(symbol: &str, asset_class: &str, timespan: &str, interval: &str) -> String {
    let canonical = format!("polygon|{}|{}|{}|{}", symbol, asset_class, timespan, interval);
    sha256_hex(canonical.as_bytes())
}
```

This is directly aligned with the “cache key generated based on the hash of … fetch parameters” pattern described in VectorBT Pro’s data caching doc. citeturn0search2

### Recommendation and change shape

**Recommendation:** Implement Now (v1)

**Change shape:** Mostly Additive  
Rationale: It introduces new catalog/universe data structures and UI surfaces while keeping existing CSV load, Polygon workflows, and artifact semantics intact. The main “invasiveness” risk is accidental workflow coupling—so v1 should keep a hard rule: **all existing flows still accept the current direct-path selection**, with catalog integration as an additional path and a metadata enrich step.

## Implementation slice, verification, risks, and open issues

### Smallest useful implementation slice

A credibly “useful v1” (and the fastest path to parity value) is:

- **Backend (Rust/Tauri)**
  - `catalog_list_datasets` (returns dataset summaries)
  - `catalog_register_csv_dataset` (imports/registers; emits dataset + version)
  - `catalog_register_polygon_dataset` (registers provider-backed dataset + version; refreshable)
  - `catalog_refresh_dataset` (appends/repairs; creates a new version and refresh event)
  - `catalog_get_dataset_detail` (versions + refresh history + quality summary)
  - `universes_list`, `universes_create`, `universes_update_members`, `universes_delete`
  - “Attach catalog references” to run artifacts: whenever a run is saved, store `dataset_version_id` (and optionally `universe_id`) alongside existing dataset identity/fingerprint metadata. (This builds on the repo’s existing artifact enrichment posture described in `REPO_MAP.md`.)

- **Frontend (Vue)**
  - Minimal “Dataset Library” panel inside Data workspace: searchable list + detail drawer.
  - “Universe picker” used by cross-ticker validation/meta flows, initially as a convenience wrapper over existing selection.

This slice achieves the question’s core targets: local catalog, provenance hooks, refresh history, derived-series lineage (initially for resamples), saved universes, and cross-workflow reuse.

### Verification and benchmark plan

Because this is a workflow-spanning change, verification should combine correctness, reproducibility, and regressions:

1. **Deterministic identity tests**
   - Same input dataset produces same `fingerprint_sha256`.
   - Same register parameters produce same `dataset_id`.
   - Derived dataset transform spec changes must change derived version fingerprints.

2. **Artifact compatibility tests**
   - Existing saved runs load correctly even without catalog entries.
   - When catalog is present, saved runs show dataset version linkage.
   - Run compare/diff surfaces (if present) can use catalog metadata but do not require it.

3. **Refresh semantics tests**
   - Append-only refresh updates: start/end time range and bar counts behave as expected (mirroring the append-oriented assumption emphasized in Qlib’s design discussion). citeturn3view1
   - Repair workflows do not silently mutate a dataset version; they create a new version and a refresh event record.

4. **Performance sanity checks**
   - Catalog queries are fast for typical local dataset counts.
   - Dataset fingerprint computation is incremental and cached (avoid re-hashing huge files repeatedly). (This specific engineering detail requires local profiling judgment.)

### Main risks or counterarguments

- **Scope creep into “data platform” territory.** DCAT/PROV/OpenLineage can tempt teams into over-design. The correct stance is to borrow *conceptual primitives* (dataset, version, run, provenance edges) while staying local-first and UI-driven. citeturn1search1turn1search0turn1search3
- **False confidence from shallow “quality state.”** A “green check” can mislead users if it’s not clear what was actually validated (missing bars? splits? duplicates? session alignment?). This is largely product/UX wording discipline, not just engineering.
- **Symbol identity and corporate actions complexity.** Mature platforms often embed adjustment logic; VectorGrid should clearly separate “raw bars” vs “adjusted bars” dataset versions, and record the adjustment policy in provenance metadata. (This is an open design issue not fully answerable from external precedent alone.)
- **Time-varying universes are deceptively complex.** LEAN’s universe refresh and Qlib’s time-varying pools show why it’s valuable, but implementing it prematurely risks destabilizing workflows. citeturn0search0turn3view1

### Limitations, open issues, and recommended next steps

**Limitations of this research answer**
- There is strong external precedent for *concepts* (catalogs, provenance, universes), but the best on-disk store and schema-evolution strategy for VectorGrid is ultimately a local engineering decision.
- Web sources describe patterns, not VectorGrid’s exact current code; the recommended integration points should be validated by local implementation review.

**Open issues (worth deciding early)**
- Should a “universe” reference **symbols** (and allow multiple dataset sources/timeframes), or reference **dataset IDs** directly?
- What is the canonical “dataset version” boundary: raw provider payload, internal `.vta` cache, or post-normalization bars?
- How should benchmark baskets be represented: as first-class universes with a `role=benchmark`, or as a separate entity with constraints?

**Recommended next steps**
- Write a short internal “Catalog + Universes v1 scope doc” that explicitly lists:
  - in-scope entities (dataset, version, refresh event, derivation, universe)
  - non-goals (connector sprawl, dynamic scripting, broad factor store)
  - artifact fields to add (dataset_version_id, universe_id)
- Implement backend catalog primitives first, then a minimal UI. Use a PROV/OpenLineage-inspired naming scheme internally (dataset + run + facets) to keep future export/interop plausible without committing to heavyweight standards. citeturn1search0turn1search3turn1search11