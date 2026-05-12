# Workflow Visibility Model

This document defines how to keep VectorGrid cohesive while supporting many workflows.

The guiding rule: the active workflow chooses the visible UI. Backend capability alone is not a reason to occupy screen space.

## Visibility Layers

Each UI element should belong to one layer.

### Primary

Visible by default because it is required for the active task.

Examples:

- load data action on a missing-data screen
- run optimization on a ready optimization screen
- selected candidate summary during validation
- validation gate status after validation runs

### Secondary

Visible but visually quiet because it supports the active task without being the main action.

Examples:

- compact strategy snapshot
- row count
- selected artifact timestamp
- report destination

### Advanced

Collapsed by default or behind an explicit advanced section.

Examples:

- validation thresholds
- bootstrap settings
- re-optimization knobs
- route override controls
- cross-ticker configuration

### Inspector

Available in the right-side inspector or details panel when the user selects an object.

Examples:

- selected candidate metrics
- gate explanations
- artifact provenance
- selected report details
- selected run lineage

### Diagnostics

Hidden from normal workflow UI unless diagnostics are open.

Examples:

- raw backend enum values
- CUDA build flags
- `gpu_speedups.csv` policy details
- command payload details
- timing internals
- raw IDs and hashes

## Workflow Modes

### Data Mode

Primary:

- choose/load dataset
- dataset readiness
- active source summary
- repair/resample actions when relevant

Secondary:

- row count
- timeframe
- source path/provider
- cache state

Advanced:

- cache root management
- Polygon repair/download policies
- derived dataset controls

Diagnostics:

- raw command status
- cache internals
- provider debug output

### Conventional Optimization Mode

Primary:

- strategy selection
- search-space shape
- objective
- run optimization
- top result summary

Secondary:

- backend route summary
- estimated combinations
- current preset

Advanced:

- full sweep ranges
- route overrides
- GPU/CPU policy controls
- less-common indicator settings

Diagnostics:

- raw strategy kind
- exact request payload
- CUDA/build flags
- speedup-policy file details

### Validation Mode

Primary:

- validation state
- blocked reason or readiness
- selected candidate
- run validation
- gate outcome
- report/export result

Secondary:

- gate thresholds
- shortlist size
- report path
- validation modules selected

Advanced:

- bootstrap settings
- walk-forward settings
- CSCV/PBO settings
- Hansen SPA settings
- cross-ticker settings
- auto-export settings

Diagnostics:

- raw final recommendation enum
- raw row counts
- backend route internals
- command payload/result fragments

Blocked-state rule:

If no dataset is loaded, show the blocked state and selected-candidate summary. Collapse validation settings until data exists.

### Meta-Analysis Mode

Primary:

- selected artifact group
- analysis status
- key comparison/result
- run/analyze action

Secondary:

- artifact counts
- cohort/holdout summary
- selected ranking mode

Advanced:

- cohort filters
- holdout policy
- ranking policy
- evidence thresholds

Diagnostics:

- raw artifact IDs
- runtime metadata
- replay/provenance internals

### ML Signal Mode

Primary:

- dataset/model readiness
- feature policy
- train/score action
- signal result summary

Secondary:

- feature count
- split summary
- model artifact path

Advanced:

- CatBoost settings
- feature selection
- calibration/scoring options
- validation policy

Diagnostics:

- raw model metadata
- command payload
- score-cache internals

### Options Optimization Mode

Primary:

- options dataset/readiness
- expression setup
- parent artifact/context
- run optimization
- validation follow-up state

Secondary:

- provider/source summary
- quote quality
- retained tuple/split summary

Advanced:

- expression internals
- walk-forward plan
- holdout plan
- validation replay policy

Diagnostics:

- Polygon quote internals
- provenance hashes
- raw retained tuple data

### Live / Paper Execution Mode

Primary:

- execution support status
- active symbol/feed
- dry-run/live mode
- arm/start/stop action
- risk-policy status

Secondary:

- broker connection state
- current position
- recent runner event

Advanced:

- bracket policy
- risk overrides
- execution adapter support matrix

Diagnostics:

- broker request IDs
- raw runner event payloads
- local paper ledger details

### Library / History Mode

Primary:

- saved runs/artifacts
- open/replay/compare action
- selected artifact summary

Secondary:

- artifact type
- timestamp
- workflow family
- report availability

Advanced:

- filters
- compare settings
- export/replay options

Diagnostics:

- raw artifact IDs
- hashes/provenance
- replay payloads

### Settings / Diagnostics Mode

Primary:

- user-configurable preferences
- data root
- runtime status
- diagnostic export/copy action

Secondary:

- version/build summary
- GPU availability summary
- desktop integration status

Advanced:

- cache migration
- window/runtime effects
- notification permissions

Diagnostics:

- all raw environment/build/command detail

## Cross-Workflow Rules

1. Do not show workflow-specific controls until that workflow is selected or required.
2. Preserve quick access through navigation, command palette, or inspector links.
3. Prefer a compact snapshot for previous steps when the user advances to a later step.
4. Keep advanced settings editable, but collapse them unless they are the current task.
5. Keep diagnostics available, but make them look diagnostic.
6. Convert raw backend states to public-facing language before they appear in primary UI.
7. Use tests to protect the intended visibility state for each workflow.
