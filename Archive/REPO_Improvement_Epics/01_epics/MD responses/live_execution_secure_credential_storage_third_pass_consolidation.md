# Live Execution Secure Credential Storage — Third-Pass Consolidation

## Bundle under review

- `11_live_execution_secure_credential_storage_2026-04-11.md`

## Packet used for consolidation

- `11_live_execution_secure_credential_storage_2026-04-11.md`
- `05_live_execution_productization_2026-04-10.md`
- `REPO_MAP.md`
- `AGENTS.md`
- `executionService.ts`
- `executionStore.ts`
- `commands_execution.rs`
- `commands_data.rs`
- `polygon.rs`
- `storageKeys.ts`
- `lib.rs`

---

## 1. Surface understanding

This bundle is trying to harden one narrow live-execution weakness, not widen product scope: move remembered Alpaca credentials out of frontend local storage and into a backend-owned storage path while keeping the current live scope intact. The umbrella live plan and `REPO_MAP.md` both confirm that the repo’s live posture is still narrow: current non-options live execution only, Alpaca broker integration, Polygon live stocks feed, a single live stream slot, a single live runner slot, and an execution surface that still lives under the settings workspace rather than a fully separate shell surface. (`05_live_execution_productization_2026-04-10.md:7-24, 36-42, 55-63`; `REPO_MAP.md:901-903, 1589-1607`)

The real change is broader than “move remembered keys to the backend.” Today the frontend is both a persistence owner and an active request owner:

- `executionService.ts` reads and writes raw Alpaca credentials in frontend storage, resolves typed values over persisted values, and sends raw `alpaca_api_key` / `alpaca_api_secret` fields into account test, broker snapshot refresh, one-shot execution, and runner start requests. (`executionService.ts:4-6, 506-553, 609-818`)
- `executionStore.ts` hydrates those remembered raw values into visible form state on startup, derives `Persisted` / `Session only` / `Will persist` labels from raw-value comparisons, and clears the form optimistically before async clear completes. (`executionStore.ts:206-285, 348-388, 482-500, 876-889`)
- `commands_execution.rs` still requires raw Alpaca credentials in the request types and enforces them at runtime for non-dry-run execution, account test, and broker snapshot refresh. (`commands_execution.rs:39-61, 83-100, 914-1016, 1038-1117, 1402-1485`)

So the bundle needs to define secret ownership, command-resolution behavior, migration, UI truthfulness, degraded-storage policy, and verification scope. A pure storage-only framing is too small for the code that already exists.

No stale or nonexistent file references were confirmed among the files in this packet. The Related Docs listed inside the secure-storage plan were not independently validated here because those targets were not included in the packet. (`11_live_execution_secure_credential_storage_2026-04-11.md:123-128`)

---

## 2. Consolidated must-fix findings

### Must-fix 1 — Define the full secret-owner and command-resolution contract

- **Severity:** critical  
- **Type:** correctness / safety  
- **Basis:** confirmed doc gap  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:77-120`; `executionService.ts:609-818`; `commands_execution.rs:39-61, 83-100, 914-1016, 1038-1117, 1402-1485`; `lib.rs` command registry

**Why it matters**

The bundle adds backend credential commands for set / get-status / clear, but it does not say how the existing broker-facing commands stop depending on raw credentials supplied by the frontend. In the current code, all of these still take raw secrets from the web layer:

- `test_alpaca_connection`
- `get_live_execution_snapshot`
- `execute_strategy_signal_once`
- `start_live_execution_runner`

If the doc does not redefine those command contracts, the backend store becomes only a shadow persistence layer while the frontend remains the real secret owner.

**Recommended doc change**

Add a section named something like **Credential ownership and command resolution** that explicitly covers every current Alpaca consumer. The cleanest additive rule is:

- backend storage is the only persisted owner after migration
- broker-facing commands resolve stored backend credentials by default
- typed frontend credentials, if retained, are a transient per-action override only
- no permanent dual-owner model remains after migration

This section should explicitly name the four current Alpaca-consuming commands instead of referring only to “execution credential commands.”

---

### Must-fix 2 — Remove the raw-secret readback loophole

- **Severity:** critical  
- **Type:** safety / operator-truth  
- **Basis:** confirmed doc gap  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:79-86, 96-99`; `commands_data.rs:545-565`; `polygon.rs:57-61`

**Why it matters**

The current plan says backend commands should avoid returning raw secrets “unless explicitly required for a justified UX flow.” That exception is too loose for a bundle whose purpose is to establish one backend-owned secret owner. The Polygon precedent it points to is actually stricter than the bundle wording: the public `polygon_get_cache_info` path returns `has_api_key`, not the key itself. The moment Alpaca “get” returns raw secrets to the frontend, frontend secret ownership effectively survives.

**Recommended doc change**

Make the rule explicit: **backend retrieval means presence/status only, never raw secret readback**. Replace the current loophole wording with a direct ban. If the UI needs to show stored-state feedback, it should use presence/masked state, not raw-value retrieval.

---

### Must-fix 3 — Define a real precedence and migration matrix, including typed input and legacy alias keys

- **Severity:** high  
- **Type:** migration / correctness  
- **Basis:** confirmed doc gap plus confirmed repo edge case  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:84-86, 96-100, 118-120`; `executionService.ts:515-553, 624-666, 685-818`; `executionStore.ts:237-285`; `storageKeys.ts:33-44`

**Why it matters**

The bundle says it will define one precedence rule and one migration path, but current behavior already has more than two states:

1. typed-in credentials
2. current frontend-persisted credentials
3. future backend-stored credentials
4. legacy alias storage key `vectorbt_exec_keys_v1`

There is also a current sharp edge: `resolveApiKeys()` treats any typed input as authoritative. That means a partially typed key or secret suppresses fallback to persisted credentials. If the doc does not define this explicitly, contributors can implement conflicting behaviors around typed overrides, fallback, and migration cleanup.

**Recommended doc change**

Add a section named **Precedence and migration matrix** that explicitly defines behavior for:

- complete typed-in pair
- incomplete typed-in pair
- backend-stored pair present
- current frontend local-storage key present
- legacy alias key present
- no credentials present

Also add a concrete migration sequence:

1. scan both `vectorgrid_exec_keys_v1` and `vectorbt_exec_keys_v1`
2. attempt backend store write
3. verify backend success
4. only then delete legacy local storage
5. surface explicit migration success or failure
6. if the migration cannot be proven, use the already-documented clear-and-reenter fallback rather than silently guessing

---

### Must-fix 4 — Decide the degraded-storage policy instead of treating Polygon as a drop-in precedent

- **Severity:** high  
- **Type:** operator-truth / safety  
- **Basis:** confirmed contradiction between plan wording and owner file behavior  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:79-86, 118-121`; `polygon.rs:212-223, 318-391`

**Why it matters**

The plan says to reuse the Polygon secure-storage pattern, but Polygon is not “secure storage only.” In the provided owner file:

- `set_api_key()` falls back to a local JSON file when keyring write fails
- `read_api_key()` can also fall back to that file and then to env / `.env`
- legacy plaintext cleanup only happens after a successful keyring round trip

That may be acceptable for Polygon market-data keys, but it is not automatically truthful to present that as the Alpaca broker-credential posture.

**Recommended doc change**

Add a section named **Degraded storage policy** and answer this directly:

- Is Alpaca remember-mode keyring-only, with “remember unavailable on this machine” if secure storage is unavailable?
- Or is degraded fallback allowed, with explicit degraded-security copy and diagnostics?

The doc should not imply strong secure storage while silently inheriting Polygon’s weaker fallback behavior.

---

### Must-fix 5 — Replace “keep the current UI contract stable” with an explicit post-migration UI truth model

- **Severity:** high  
- **Type:** operator-truth / correctness  
- **Basis:** confirmed contradiction with current frontend owner files  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:101, 118-120`; `executionStore.ts:206-285, 348-388, 482-500, 876-889`; `executionService.ts:515-553, 876-887`

**Why it matters**

The bundle says to keep the current UI contract stable while moving ownership behind the scenes. That is not literally possible if raw secret readback is removed, because the current UI contract depends on raw persisted values being hydrated into visible store state:

- startup loads remembered values into `apiKey`, `apiSecret`, `persistedApiKey`, and `persistedApiSecret`
- key-state labels such as `Persisted`, `Will update persisted`, and `Will persist next action` are derived from raw-value comparison
- `clearKeys()` blanks both current and persisted mirrors before async clear finishes

So a backend-owned, presence-only model requires a visible UI truth-model rewrite even if the surface stays familiar.

**Recommended doc change**

Replace the vague rollout note with an explicit **Frontend truth model after migration** section that defines:

- what the form shows on startup when credentials are stored
- whether fields stay blank, masked, or prefilled
- how `Missing`, `Session only`, `Stored`, `Will store`, `Will update`, and `Clear failed` are computed without raw secret hydration
- whether clear is optimistic or confirmation-based
- what gets restored or shown if backend clear fails

---

### Must-fix 6 — Expand verification to cover migration, UI truthfulness, redaction, and every credential-consuming command path

- **Severity:** high  
- **Type:** verification / testing  
- **Basis:** confirmed doc gap  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:42-47, 102-114`; `05_live_execution_productization_2026-04-10.md:55-63, 134-141`; `executionService.ts:685-887`; `executionStore.ts:700-705, 876-889`; `commands_execution.rs:1038-1117, 1402-1485`

**Why it matters**

The current verification plan is too narrow for the real blast radius of this bundle. The change affects:

- startup credential hydration behavior
- visible remember-key state labels
- migration from current and legacy storage keys
- clear-state semantics
- request resolution for all current Alpaca commands
- redaction in `statusLine`, `statusHint`, and `outputPreview`

The umbrella live plan already treats component tests and browser/desktop smoke as part of the live verification surface, but the secure-storage bundle does not pull that surface forward.

**Recommended doc change**

Expand the verification plan to include:

- `ExecutionWorkspacePanel` tests for stored / session / missing / clear-failure states
- migration tests for both current and legacy key aliases
- command-resolution tests for `test_alpaca_connection`, `get_live_execution_snapshot`, `execute_strategy_signal_once`, and `start_live_execution_runner`
- negative redaction tests proving secrets never leak into `statusLine`, `statusHint`, or `outputPreview`
- browser UX and desktop smoke if visible startup or remember-key behavior changes

---

## 3. Consolidated should-fix findings

### Should-fix 1 — Expand the current code-surface owner list

- **Severity:** medium  
- **Type:** maintainability  
- **Basis:** confirmed owner-file omission  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:31-40`; `executionService.ts:1-6`; `REPO_MAP.md:901-903`

**Why it matters**

The bundle’s owner map omits two files that are clearly part of the current behavior being changed:

- `web/src/utils/storage.ts`, which `executionService.ts` actually uses for current local persistence
- `web/src/components/ExecutionWorkspacePanel.vue`, which owns the user-visible remember/clear/status behavior in the mounted execution surface

**Recommended doc change**

Add both files to **Current Code Surface** so the doc’s owner map matches the real change surface.

---

### Should-fix 2 — Clarify whether remember semantics stay deferred or become immediate

- **Severity:** medium  
- **Type:** operator-truth  
- **Basis:** likely issue, grounded in current implementation  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:84-86, 118-120`; `executionService.ts:524-537, 685-818`; `executionStore.ts:268-285`

**Why it matters**

Current behavior is not a simple “toggle saves now” flow. Persistence happens after successful broker-facing actions. In the current service layer that includes:

- account test
- broker snapshot refresh
- run once
- runner start

The current store copy only partially explains this. If backend `set` starts saving on toggle or field entry instead, that changes the UX contract.

**Recommended doc change**

Add a short **Remember semantics** subsection that either:

- preserves deferred persistence and names the exact successful actions that commit storage, or
- intentionally switches to immediate save and states that the UX contract is changing

---

### Should-fix 3 — State whether stored credentials remain one shared slot or become mode-scoped

- **Severity:** medium  
- **Type:** operator-truth  
- **Basis:** open question grounded in current code shape  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:5-23, 77-120`; `executionStore.ts:439-479`; `commands_execution.rs:40-61, 83-100`

**Why it matters**

The current UI and request shapes model one Alpaca key pair plus a separate `mode` switch. The bundle does not say whether backend storage preserves that single-slot model or splits storage by paper/live mode. That is an operator-truth question because mode switches could otherwise appear more isolated than they are.

**Recommended doc change**

Add one sentence making the storage model explicit:

- either “this bundle preserves the current single stored key-pair model”
- or “this bundle introduces mode-scoped storage”

Given the packet, preserving the current single-slot model is the repo-grounded default unless another narrow doc explicitly widens scope.

---

### Should-fix 4 — Add acceptance evidence expectations, not just gate commands

- **Severity:** low  
- **Type:** maintainability / verification  
- **Basis:** optional-but-useful improvement grounded in umbrella plan style  
- **Location:** `11_live_execution_secure_credential_storage_2026-04-11.md:106-114`; `05_live_execution_productization_2026-04-10.md:136-141`

**Why it matters**

The bundle lists commands to rerun, but not what closure evidence reviewers should expect for the behavior change itself.

**Recommended doc change**

Add a compact **Acceptance evidence** subsection with examples such as:

- migrated user with old local key successfully moved to backend storage
- startup with stored credentials but no raw secret hydration
- clear failure with truthful UI state
- redaction-safe error path

---

## 4. Optional improvements worth considering

1. Add a one-screen command matrix: current command, current credential source, post-migration source, and required tests.
2. Add one short copy table for `Missing`, `Stored`, `Session only`, `Clear failed`, and `Remember unavailable` states so frontend and backend copy stay aligned.
3. Add one explicit example of the allowed transient override path if typed session credentials remain supported after migration.
4. Add a small touched-files note that reminds implementers new credential commands will also require Tauri registry wiring in `lib.rs` and frontend invoke/service changes.
5. Add one narrow note that this bundle is storage hardening only and does not change live-runner, broker-reconciliation, or shell-promotion ownership, which are covered by other live bundles.

---

## 5. Suggestions to reject as out of scope or unsupported

1. **General account-management or multi-broker credential abstractions**  
   Reject. The bundle explicitly keeps new broker integrations and general-purpose vault work out of scope, and the umbrella live plan keeps the repo’s broker scope narrow. (`11_live_execution_secure_credential_storage_2026-04-11.md:18-23, 120-121`; `05_live_execution_productization_2026-04-10.md:19-24`)

2. **Cloud sync or cross-device secret storage**  
   Reject. The plan explicitly excludes cloud secret sync. (`11_live_execution_secure_credential_storage_2026-04-11.md:20-22`)

3. **Broad shell redesign or moving the execution surface as part of this bundle**  
   Reject. The current execution surface is still settings-mounted, but shell placement belongs to the umbrella live productization lane and the discrete-surface bundle, not this storage-hardening bundle. (`05_live_execution_productization_2026-04-10.md:42, 99-109`; `REPO_MAP.md:901-903`)

4. **Returning raw secrets to the frontend for “edit existing key” convenience**  
   Reject. That would preserve frontend secret ownership and directly weaken the stated goal of backend-owned storage.

5. **Multi-symbol, options-live, portfolio dashboard, or OMS-style expansion**  
   Reject. None of that is required by the provided files, and the umbrella live doc explicitly keeps live scope narrow. (`05_live_execution_productization_2026-04-10.md:11-24, 36-42`)

6. **Mode-scoped or multi-slot credential families as part of this bundle unless the doc explicitly widens scope**  
   Treat as unsupported for now. The current files only prove a single credential pair plus a mode switch, not a broader account model. (`executionStore.ts:439-479`; `commands_execution.rs:40-61, 83-100`)

---

## 6. Recommended doc edits

A good additive rewrite would make these seven edits:

1. **Add “Credential ownership and command resolution.”**  
   Name the four current Alpaca command consumers and define how each resolves credentials after migration.

2. **Change “get status / presence” into an explicit no-readback contract.**  
   State that backend retrieval never returns raw secrets to the frontend.

3. **Add “Precedence and migration matrix.”**  
   Cover typed complete, typed incomplete, backend stored, current local stored, legacy alias stored, and missing states.

4. **Add “Degraded storage policy.”**  
   Explicitly state whether Alpaca remember-mode is keyring-only or allows a degraded fallback, and what operator copy appears in that degraded state.

5. **Replace “keep the current UI contract stable” with “Frontend truth model after migration.”**  
   Define startup appearance, remember-state labels, clear behavior, and failure states.

6. **Expand “Current Code Surface” and “Current Verification Surface.”**  
   Add `web/src/utils/storage.ts`, `web/src/components/ExecutionWorkspacePanel.vue`, migration alias tests, command-resolution tests, redaction tests, and UX/smoke coverage when visible behavior changes.

7. **Add “Acceptance evidence.”**  
   Require one migrated-user example, one stored-no-readback startup example, one clear-failure example, and one redaction-safe error example.

---

## 7. Recommended implementation order

1. **Lock the truth model in the doc first.**  
   Define the single persisted owner, the no-readback rule, and the exact command-resolution contract for all current Alpaca consumers.

2. **Decide degraded-storage policy before writing migration steps.**  
   The bundle cannot truthfully say how migration behaves until it states whether secure storage is mandatory or whether degraded fallback is allowed.

3. **Write the precedence and migration matrix.**  
   Include typed overrides, incomplete typed input behavior, current key, legacy alias key, backend verification, and deletion conditions.

4. **Define the post-migration frontend truth model.**  
   Specify startup state, remember-state labeling, and clear failure semantics after backend ownership removes raw persisted hydration.

5. **Expand verification and acceptance evidence.**  
   Name the test targets and visible closure evidence before implementation begins.

6. **Then implement surgically.**  
   That matches repo guidance to prefer small, surgical changes and to add tests close to the changed surfaces. (`AGENTS.md:42-46`)

---

## Bottom line

The strongest consolidated conclusion is this:

The bundle is directionally correct, but it is still underspecified in the places that actually make the change real.

The doc is ready to proceed only after it explicitly defines:

- one persisted secret owner
- one command-resolution contract for all current Alpaca consumers
- one no-readback rule
- one precedence and migration matrix
- one degraded-storage policy
- one truthful post-migration UI model
- one verification plan that covers migration, redaction, and all current credential-consuming paths

Without those additions, the most likely failure mode is not a broken feature. It is a feature that appears hardened while still leaving real secret ownership, migration behavior, or UI truth ambiguous.
