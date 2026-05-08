# Advanced Mode Reference

Advanced Mode exposes the full research workbench. For releases that implement Simple Mode, it is for users who need more control than the streamlined first-run workflow. In the current checked-in frontend, Advanced Mode is the default implemented workbench.

## When To Use Advanced Mode

Use Advanced Mode when you need:

- advanced strategy families;
- explicit CPU/GPU backend control;
- exact-grid controls;
- heatmap settings;
- export-all or include-all outputs;
- shortlist confidence, union, slice, or neighborhood diagnostics where supported;
- validation modules beyond the Simple recommendation;
- cross-dataset validation;
- saved-run history and artifact replay;
- report exports;
- ML-signal workflows;
- options-expression workflows;
- live/paper execution workspaces.

## Workbench Areas

Advanced Mode may include these major areas:

| Area | Purpose |
|---|---|
| Data | Load CSVs, samples, Polygon datasets, and market-data cache settings. |
| Optimize | Configure strategy, backend, objective, ranges, and outputs. |
| Validation | Validate selected candidates and inspect evidence modules. |
| History | Reopen saved artifacts, inspect replay status, and access reports. |
| Aggregate/Meta | Compare saved artifacts or run meta-level selection workflows where available. |
| Candidate detail | Inspect selected candidate charts, trades, drilldown, and local-stability follow-ups where supported. |
| ML Signal | Export feature datasets, import score caches, and validate signal policies. |
| Options Expression | Evaluate options-expression workflows from supported parent artifacts. |
| Live | Dry-run, local paper, or broker-connected workflows where enabled. |
| Portfolio | Run portfolio feasibility analysis where the release exposes that workspace. |
| Settings | App configuration, data roots, risk policy, notification settings, and diagnostics. |
| Diagnostics | Review runtime, GPU, command, route, and app-state diagnostics when available. |

The exact workspace set can differ by release.

## Advanced Route Awareness

Advanced users should pay attention to:

- requested backend;
- resolved backend;
- fallback reasons;
- unsupported controls;
- route notes;
- evidence tier;
- artifact replay class;
- validation warnings.

Route metadata is not noise. It is how the app tells you what actually ran.

## Advanced Output Controls

Advanced outputs can be large. Use them intentionally:

- Top-K for normal review.
- Include-all for broader retained candidates.
- Export-all for complete supported result surfaces.
- Heatmap for supported Double-MA-style sensitivity review.
- XLSX/JSON/CSV reports for durable evidence.

Large exports can increase runtime, memory use, and disk use.

## Advanced Mode Safety

Advanced Mode makes it easier to create misleading evidence if controls are misunderstood.

Before acting on any result, confirm:

- the data is clean;
- the strategy supports the selected route;
- validation actually ran;
- warnings are understood;
- exported files include enough provenance;
- the result is not being treated as trading advice.

## Command Palette And Navigation

Advanced users may use the command palette and workspace rail to move between major workspaces, start common actions, open reports, switch Simple/Advanced modes, and reach settings.

Keyboard and command-palette shortcuts are convenience features. They should not bypass disabled states, confirmation dialogs, or live-execution risk controls.
