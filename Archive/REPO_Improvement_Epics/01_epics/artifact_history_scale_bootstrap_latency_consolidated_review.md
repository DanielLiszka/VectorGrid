# Artifact History Scale And Bootstrap Latency — Consolidated Review

Bundle under review: **Artifact History Scale And Bootstrap Latency**

## Surface understanding

This epic is aimed at three related but distinct costs: latest-run reopen, full artifact-library enumeration, and startup/bootstrap critical-path latency. The provided code already separates some of those paths more than the plan says. The frontend latest-run sync now calls `load_latest_run_artifact_summary`, then hydrates only that returned path with `load_run_artifact_hydration`; the full-library path still scans and parses the runs directory; and bootstrap already restores shell state immediately, starts startup data hydration, and only then chains artifact sync. The plan should describe those paths separately rather than treating them as one undifferentiated artifact-sync cost. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Current User-Facing Behavior”; [workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts), `syncWorkflowArtifactsFromLegacyRuntime`; [commands_artifacts.rs](sandbox:/mnt/data/commands_artifacts.rs), `load_latest_run_artifact_summary`, `load_run_artifact_hydration`, `list_artifact_library`; [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `load_latest_run_artifact_entry_blocking`, `scan_artifact_library`, `list_artifact_library_blocking`; [workflowBootstrap.ts](sandbox:/mnt/data/workflowBootstrap.ts), `applyWorkflowBootstrapFromStorage`)

The other major repo truth is that saved-run truth is already broad and user-visible. The Rust artifact owner, workflow-artifact store, and history UI all carry more than req/res plus tags: config fingerprint/version, dataset/resample/source identity, route provenance, lineage, optimization metric surface, ML fields, and validation payloads. Any performance plan that talks about summary/index paths or deferred bootstrap work without freezing those truth fields is under-specified. ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `WorkflowArtifactRunPayload`, `RunArtifactDetailResponse`, `save_run_artifact_blocking`, `build_workflow_artifacts_hydration_payload`, `build_run_artifact_detail_response`, `build_run_artifact_summary`; [workflowArtifactsStore.ts](sandbox:/mnt/data/workflowArtifactsStore.ts), `WorkflowRunArtifact`, `WorkflowMlSignalArtifact`, `workflowRunSignature`, `normalizeRunArtifact`; [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue), selected-run detail block)

## Consolidated must-fix findings

1. **The plan’s current-state description is already stale.**

   The doc says the current workflow-artifact sync path “asks the runtime for the full run-artifact list, takes the newest row, and then loads a separate hydration payload.” The current code does not do that anymore. `syncWorkflowArtifactsFromLegacyRuntime` calls `load_latest_run_artifact_summary`, gets one path, and then calls `load_run_artifact_hydration` only for that artifact. The full scan still exists for library listing and for latest-run fallback, but not as the normal latest-run frontend path. The doc should be corrected before implementation starts, otherwise it will benchmark and optimize the wrong baseline. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Current User-Facing Behavior”; [workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts), `syncWorkflowArtifactsFromLegacyRuntime`; [commands_artifacts.rs](sandbox:/mnt/data/commands_artifacts.rs), `load_latest_run_artifact_summary`, `load_run_artifact_hydration`; [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `load_latest_run_artifact_entry_blocking`, `load_latest_run_artifact_summary_blocking`)

2. **The compatibility/truth contract is too narrow.**

   The checklist currently freezes tags, validation hydration, dataset identity, and legacy aliases. That is not enough for the code that actually exists. Reopen/detail truth today includes config fingerprint/version, dataset meta including resample/base/derived/source fields, route provenance, lineage, optimization metric surface, ML-specific metadata, and req/res plus validation req/res. The history panel renders many of these directly. Those fields need to be called out explicitly as compatibility-frozen for any fast path, summary/index, or deferred bootstrap work. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), checklist and rollout/compatibility sections; [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `WorkflowArtifactRunPayload`, `RunArtifactDetailResponse`, `save_run_artifact_blocking`, `build_workflow_artifacts_hydration_payload`, `build_run_artifact_detail_response`, `build_run_artifact_summary`; [workflowArtifactsStore.ts](sandbox:/mnt/data/workflowArtifactsStore.ts), `WorkflowRunArtifact`, `workflowRunSignature`; [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue), selected-run detail block)

3. **The plan is missing an explicit freshness/mutability contract.**

   The current frontend already has a trust shortcut: it skips re-hydration when path and timestamp match, validation is present, and the summary’s report path, csv path, and tags match. But the store’s real signature includes broader truth such as dataset meta, config fingerprint, route provenance, lineage, ML state, metric surface, req, and res. That shortcut is only safe if artifacts are effectively immutable after save except for known tag edits, or if the fast path has a stronger freshness token and falls back to hydration on mismatch. The doc warns about stale indexes, but it does not define the actual freshness contract that would make skip/load logic trustworthy. ([workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts), `latestSummaryMatchesCurrentStore`, `syncWorkflowArtifactsFromLegacyRuntime`; [workflowArtifactsStore.ts](sandbox:/mnt/data/workflowArtifactsStore.ts), `workflowRunSignature`, `applyNormalizedArtifacts`; [01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Regressions To Avoid”, checklist, rollout/fallback notes)

4. **Fallback-to-full-scan rules need to be explicit, not implied.**

   The backend latest-run path already has a two-step rule: prefer newest standard run filename prefixes, then fall back to a full `scan_artifact_library()` walk for nonstandard filenames. That is a correctness rule, not an implementation detail, and there are explicit tests for both the fast path and the fallback path. The plan mentions keeping a fallback, but it does not freeze the actual rule strongly enough. Acceptance criteria and rollback criteria should name this case directly. ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `load_latest_run_artifact_entry_blocking`, `scan_artifact_library`, tests `load_latest_run_artifact_summary_prefers_latest_run_file_without_scanning_meta_history` and `load_latest_run_artifact_summary_falls_back_to_full_scan_for_nonstandard_run_filenames`; [01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), proposed approach, checklist, rollout/fallback notes)

5. **The scope boundary around shared library families is ambiguous.**

   The doc says options-related artifacts or workflow history are out of scope, but the shared `ArtifactLibrarySnapshot` already contains `options_expression`, and the history panel embeds `OptionsExpressionPanel` under the selected saved-run surface. If this epic touches the shared scan/index/listing layer behind `list_artifact_library_blocking()`, it is not truly isolated to runs unless the doc says so explicitly. The plan should either narrow itself to run-specific latest-run/saved-run paths only, or admit shared-path changes and freeze non-run families as compatibility-preserved. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Scope Boundaries”; [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `ArtifactLibrarySnapshot`, `list_artifact_library_blocking`; [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue), embedded `OptionsExpressionPanel`)

6. **The plan omits real frontend owners.**

   The current code surface and verification surface list the history panel and dock, but the routing map says the real saved-run owners are `historyStore.ts` and `historyService.ts`, and the panel itself delegates actions to `useHistoryStore()`. Those files are not optional implementation details; they are the owner layer for saved-run search, selected-run detail hydration, and library actions. They should be added to the doc’s owner list and verification list. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Current Code Surface”, “Current Verification Surface”; [REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md), “If the task is artifacts, batch summaries, or run history”, “Important stores”, “Important services”, “Saved run rehydration and history bridge”; [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue), imports and store-driven actions; [HistoryContextDock.vue](sandbox:/mnt/data/HistoryContextDock.vue), store-driven actions)

## Consolidated should-fix findings

1. **The verification plan should use the repo’s actual gate order, not a generic substitute.**

   The repo’s documented flow is: run the repo quality gate first for broad non-options changes, then run the workflow gate when staged workflow timing can change, and run output parity when artifact/report-path behavior changes. The epic doc currently falls back to more generic `cargo check` plus lint/typecheck wording. That is weaker than the repo’s own routing guidance. ([AGENTS.md](sandbox:/mnt/data/AGENTS.md), “Common commands”, “Coding conventions”; [REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md), “Build / Dev / Release Workflow”, “Current local quality gate”, “Current verification order”, “Verification Map”; [01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Verification Plan”)

2. **The timing baseline needs named milestones, not just broad scenarios.**

   Bootstrap is already partly split: shell/workspace restoration happens immediately, startup data hydration starts first, and artifact sync is chained after that promise resolves. The doc should therefore define milestone timings such as shell restored, startup data hydrate resolved, latest summary loaded, artifact hydration applied, analysis refresh applied, and history library refreshed. Without that, the epic can report “startup improvement” while optimizing the wrong phase. ([workflowBootstrap.ts](sandbox:/mnt/data/workflowBootstrap.ts), `applyWorkflowBootstrapFromStorage`; [workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts), `syncWorkflowArtifactsFromLegacyRuntime`; [01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Proposed Improvement Approach”, checklist, verification plan)

3. **The scenario matrix needs branch cases, not only size tiers.**

   “Small / medium / large library” is not enough for this repo. The real branches include standard-name latest run, nonstandard-filename fallback, with-validation vs without-validation reopen, ML-signal reopen, mixed-family library scans, and legacy/resampled dataset metadata. Those cases should be explicit in baseline and acceptance evidence. ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `load_latest_run_artifact_entry_blocking`, `build_workflow_artifacts_hydration_payload`, `build_run_artifact_summary`, latest-run tests; [workflowArtifactsStore.ts](sandbox:/mnt/data/workflowArtifactsStore.ts), `WorkflowMlSignalArtifact`, `normalizeRunArtifact`; [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue), selected-run detail block)

4. **Artifact identity should be defined explicitly as something other than `config_fingerprint`.**

   The current fingerprint intentionally strips `data_id`, strips validation candidates and admission provenance, and normalizes cross-ticker dataset order. The test suite explicitly proves two distinct artifacts can share the same fingerprint. Any summary/index/dedupe plan that treats it as unique identity would be wrong. The doc should state that `config_fingerprint` is comparison metadata only, not artifact identity. ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `compute_config_fingerprint_from_artifact`, test `config_fingerprint_ignores_data_ids_candidates_and_cross_dataset_order`)

5. **Some cited verification surfaces are packet-unverified, not proven stale.**

   I do not see a clearly nonexistent path in the epic doc from the provided files. The sharper issue is omitted owners. `HistoryWorkspacePanel.test.ts` and `analysisService.ts` are named in the plan, but those files were not part of this packet, so stronger claims about their exact current contents would be unsupported. The doc should either add those owner files to the packet when reviewed in the future or phrase their role more carefully. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “Current Verification Surface”; [CONTEXT_FILE_LIST.txt](sandbox:/mnt/data/CONTEXT_FILE_LIST.txt))

## Optional improvements worth considering

1. **Keep UI/render ideas explicitly secondary until measurements justify them.**

   The doc cites MDN `content-visibility`, but the owner files provided here point much more strongly to Tauri invoke flow, file IO, and JSON scan/parse cost as the first-order problem. UI deferral may help later, but it should not become the first optimization target without measurement evidence. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), “External Research Summary”, “Proposed Improvement Approach”; [workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts); [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `scan_artifact_library`, `list_artifact_library_blocking`)

2. **State directly that hydration remains authoritative and any summary/index is derivative only.**

   That would make the trust model easier to reason about and would align the epic with the existing code shape, where summary paths exist but detail/hydration remains the source of reopen truth. ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `build_workflow_artifacts_hydration_payload`, `build_run_artifact_detail_response`, `build_run_artifact_summary`; [workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts))

3. **Prefer a narrow command split before a maintained index unless measurements prove otherwise.**

   The repo already has a latest-run summary command. It is plausible that the remaining pain sits mostly in `list_artifact_library_blocking()` and `list_run_artifacts_blocking()` rather than in latest-run lookup itself. The doc would be stronger if it explicitly said the maintained index is conditional, not assumed. ([commands_artifacts.rs](sandbox:/mnt/data/commands_artifacts.rs), `list_run_artifacts`, `load_latest_run_artifact_summary`, `list_artifact_library`; [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `list_run_artifacts_blocking`, `list_artifact_library_blocking`, `load_latest_run_artifact_summary_blocking`)

## Suggestions to reject as out of scope or unsupported

1. **Reject any broad schema rewrite or architecture rewrite.**

   Nothing in the provided files requires replacing the artifact system, widening into options work, or redesigning the app shell. The epic itself is framed as additive and non-options scoped. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), purpose and scope boundaries; [PROMPT_01_first_pass.md](sandbox:/mnt/data/PROMPT_01_first_pass.md); [PROMPT_02_second_pass.md](sandbox:/mnt/data/PROMPT_02_second_pass.md); [PROMPT_03_consolidation.md](sandbox:/mnt/data/PROMPT_03_consolidation.md))

2. **Reject any proposal that uses `config_fingerprint` as a unique artifact key.**

   The code and tests contradict that. ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `compute_config_fingerprint_from_artifact`, fingerprint collision test)

3. **Reject any attempt to optimize dense-table rendering first without baseline evidence.**

   The provided owner files do not show that as the current bottleneck. The doc itself says not to widen into unrelated UI redesign. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), scope boundaries and rollout/fallback notes)

4. **Reject any strong claim about exact saved-run library refresh behavior inside the history panel alone.**

   The packet omits `historyStore.ts` and `historyService.ts`, which `REPO_MAP.md` identifies as the actual owners, and the panel itself only proves a `refreshSessionHistory()` call on mount. More specific claims about saved-run refresh timing would not be grounded from this packet alone. ([HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue), mount logic and store usage; [REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md), history owners)

5. **Reject any claim that the plan currently references a clearly nonexistent file path.**

   From the provided files, that is not proven. The stronger critique is that the doc omits real owner files, not that it names a definitely invalid one. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), current code/verification surfaces; [REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md), history owners)

## Recommended doc edits

1. **Rewrite “Current User-Facing Behavior.”**

   Say explicitly:
   - latest-run reopen already uses `load_latest_run_artifact_summary` followed by `load_run_artifact_hydration`
   - full-library browsing still uses directory-wide scan/parse
   - latest-run summary still falls back to a full scan for nonstandard filenames

   ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), current wording; [workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts); [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `load_latest_run_artifact_entry_blocking`)

2. **Expand “Current Code Surface” and “Current Verification Surface.”**

   Add:
   - `web/src/stores/historyStore.ts`
   - `web/src/services/historyService.ts`

   Keep `analysisService.ts` if reopen follow-up remains relevant, but note it as follow-up behavior rather than primary history ownership. ([REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md), history owners; [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue); [HistoryContextDock.vue](sandbox:/mnt/data/HistoryContextDock.vue))

3. **Add a new “Artifact Truth and Compatibility Freeze” section.**

   Freeze:
   - config fingerprint/version
   - dataset/resample/source identity
   - route provenance
   - lineage
   - optimization metric surface
   - ML fields
   - validation payload presence/absence
   - req/res and validation req/res behavior
   - tags and legacy aliases

   ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), payload/detail/summary builders; [workflowArtifactsStore.ts](sandbox:/mnt/data/workflowArtifactsStore.ts); [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue))

4. **Add a new “Freshness, Identity, and Fallback Contract” section.**

   State:
   - artifact identity is not `config_fingerprint`
   - summary/index freshness depends on either artifact immutability or a stronger freshness token
   - hydration/detail remains authoritative
   - nonstandard filenames must still recover through full scan fallback

   ([workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts); [workflowArtifactsStore.ts](sandbox:/mnt/data/workflowArtifactsStore.ts); [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs))

5. **Replace size-only scenarios with a scenario matrix.**

   Include:
   - standard-name latest run with validation
   - latest run without validation
   - nonstandard-filename latest run
   - mixed-family artifact library
   - ML-signal reopen
   - legacy/resampled artifact reopen

   ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs); [workflowArtifactsStore.ts](sandbox:/mnt/data/workflowArtifactsStore.ts); [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue))

6. **Tighten the verification plan to repo-authoritative commands and order.**

   Name:
   - `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1`
   - targeted Rust/frontend tests near touched files
   - `powershell -File scripts/run_workflow_gate.ps1 -Mode candidate` when staged timing changes
   - `node scripts/run_output_parity_check.mjs` when artifact/report-path behavior changes

   Also say where timing baselines and traces will be stored. ([AGENTS.md](sandbox:/mnt/data/AGENTS.md); [REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md), verification sections)

7. **Clarify the scope boundary around shared artifact families.**

   Add one explicit sentence choosing between:
   - “this epic only changes run-specific latest-run and saved-run paths,” or
   - “this epic may touch shared library-scan code, but non-run families including options-expression remain compatibility-frozen.”

   ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), scope boundaries; [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), `ArtifactLibrarySnapshot`; [HistoryWorkspacePanel.vue](sandbox:/mnt/data/HistoryWorkspacePanel.vue))

## Recommended implementation order

1. **Correct the doc first.**

   Fix the stale current-state description, add the missing history owners, and freeze the truth/freshness/scope contract before implementation. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md); [REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md))

2. **Measure the existing paths separately.**

   Capture baseline timings for:
   - shell restore
   - startup data hydrate
   - latest-run summary
   - latest-run hydration
   - analysis refresh after hydration
   - full library snapshot / run list

   ([workflowBootstrap.ts](sandbox:/mnt/data/workflowBootstrap.ts); [workflowArtifactsSync.ts](sandbox:/mnt/data/workflowArtifactsSync.ts); [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs))

3. **Exploit the existing narrow latest-run path before adding new infrastructure.**

   Since the repo already has a latest-run summary command, first determine whether the remaining pain is mostly in full-library scan/listing or in bootstrap sequencing, not in latest-run selection itself. ([commands_artifacts.rs](sandbox:/mnt/data/commands_artifacts.rs); [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs))

4. **Only then decide whether a maintained summary/index is justified.**

   If measurements still point to directory-scan cost, add the index as a strictly derivative, additive path with explicit fallback and rollback triggers. ([01_artifact_history_scale_bootstrap_latency_2026-04-10.md](sandbox:/mnt/data/01_artifact_history_scale_bootstrap_latency_2026-04-10.md), proposed approach and rollout notes; [run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs))

5. **Apply bootstrap-path changes after the measurement split is clear.**

   Because bootstrap is already partially staged, defer sequencing changes until the timings show which milestone is actually hurting first-interactive behavior. ([workflowBootstrap.ts](sandbox:/mnt/data/workflowBootstrap.ts))

6. **Finish with branch-specific regression coverage and acceptance evidence.**

   Add the scenario-matrix tests and keep stored before/after timing artifacts, so the epic closes on measured evidence rather than intuition. ([run_artifacts.rs](sandbox:/mnt/data/run_artifacts.rs), latest-run tests and artifact tests; [REPO_MAP.md](sandbox:/mnt/data/REPO_MAP.md), verification map)
