# Fourth-Pass Review: crates/app_core/src/rocb.rs

- Review date: `2026-04-16`
- Source file: `crates/app_core/src/rocb.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass bucket is still directionally right, but the third-pass write-up drifted too far toward abstract “policy surface” caution and not far enough into the live mechanics of the file. `rocb.rs` is not waiting on a wide redesign before it can be improved. It still contains a handful of concrete, local contract weaknesses that are narrower than a redesign, stronger than “just add docs,” and worth calling out explicitly in a fourth pass.

This pass therefore re-opened the live source to challenge two parts of the current packet. First, it checked whether the file really had only deferred cleanup left. Second, it checked whether the packet’s broad caution against tightening fail-open behavior was too sweeping. The answer on both counts is yes: the bucket should stand, but there are still safe-now hardening targets that the third-pass packet underweighted.

## Fourth-Pass Research Log

This pass re-read the live `rocb.rs` source first, then compared it against the second- and third-pass packets to isolate claims that were still grounded by the code versus claims that had become too generalized. It also used `crates/app_core/src/lib.rs`, `AGENTS.md`, and `REPO_MAP.md` only to confirm that `RocbStrategyParams` is still a public re-export and that ROCB remains a validation-sensitive, route-sensitive strategy-local surface rather than a throwaway helper.

Targeted standards review was useful for the parts of the file that are most likely to mislead a reviewer. The Rust API Guidelines dependability guidance (`C-VALIDATE`) remains the right frame for the public helper surface: invalid inputs should be rejected or at least checked when practical, rather than silently normalized everywhere. The flexibility guidance around caller-controlled placement and intermediate results supports keeping the caller-owned `*_into` buffer API, but it also supports adding a checked sibling instead of forcing all diagnostics through a silent `() -> ()` path. Serde’s field-default behavior still confirms that `RocbStrategyParams` defaults are a compatibility-sensitive persistence surface, not a casual cleanup target. `OnceLock` still looks appropriate for one-time descriptor binding and should not be “simplified” away. The most important fresh standards check was the standard-library `Vec::reserve` contract: `reserve(additional)` is relative to `len`, not current capacity. That matters here because the current buffer-reuse path calculates `additional` from `capacity`, which means the file’s intended no-extra-allocation posture is not actually implemented correctly.

## What Fourth Pass Sharpened

### Bucket assessment, and what the third-pass packet got right

The current third-pass bucket should **stand**: this is still a `Small surgical refactor`, not a redesign candidate. The third-pass packet was right to keep compatibility discipline high. It was also right that this file sits close enough to optimization and validation semantics that broad “cleanup symmetry” work would be risky. The packet was further correct not to push for removing the `OnceLock` descriptor cache, not to touch the fast/slow-to-channel/ATR default mapping casually, and not to treat CPU/GPU route questions as cosmetic.

What changed in this fourth pass is not the bucket, but the reason for it. The file is not small-surgical merely because it is policy-sensitive. It is small-surgical because there are several precise, high-value fixes that can be made without redesigning ROCB itself.

### What the third-pass packet missed or underweighted

The strongest miss is in `build_rocb_desired_positions_into`. The third-pass packet treated the caller-owned buffer path as something to preserve, which is correct, but it did not notice that the reserve logic is wrong. The function clears the vector, then does `desired.reserve(n - desired.capacity())` when `capacity < n`. That is not how `Vec::reserve` works. `reserve` guarantees capacity for `len + additional`, and `len` is `0` immediately after `clear()`. So a buffer with capacity `100` and target length `120` will call `reserve(20)`, and that can legally be a no-op because capacity `100` already satisfies `len + additional == 20`. In other words, the current code often fails to reserve enough space precisely in the buffer-reuse cases it is trying to optimize. This is a real, local performance bug, and it is safer to fix than most of the documentation work emphasized so far.

The malformed-shape early-return path is also worse than the prior packet made it sound. The earlier review correctly noticed that `build_rocb_desired_positions_into` returns silently when `breakout.upper.len()`, `breakout.lower.len()`, or `risk.atr.len()` do not match `close.len()`. But the sharper issue is not just “quiet fallback.” Because the function has already resized the destination to `start_t.min(n)` zeros before that check, the early return leaves `desired` **truncated**, not merely flat. That means the failure mode is not “all-zero positions for this series.” It is “vector length no longer matches the price series.” On a validation-sensitive strategy surface, that is a materially sharper contract hazard than the third-pass packet described.

The dynamic-indicator boundary is also more than generic stringly-typed debt. `resolve_output_id` does case-insensitive matching first, then permissive substring matching, and finally falls back to `descriptor.outputs[0]`. That fallback is survivable for single-output indicators like ATR. It is much more dangerous for Donchian, where ROCB needs semantically distinct upper and lower outputs. If the dynamic-indicator catalog changes output names or output order in a future `vector-ta` update, ROCB can silently bind the wrong series and still initialize successfully. The third-pass packet was right that this boundary is runtime-bound, but it understated the concrete failure mode: this is not just about readability or compile-time guarantees, it is about silent strategy-semantics drift.

The warmup helper also deserves a more exact description than it has received so far. `first_joint_finite_idx_expected_or_scan` is not truly “scan for the first jointly finite index” after the fast path misses. Its fallback takes `max(first_finite(lhs), first_finite(rhs))`. That works only if both series become finite once and then stay finite. For the current Donchian-style use, that assumption is probably true in practice, which is why this is not a redesign trigger. But the helper name and the earlier review language both oversell the robustness of the current implementation. If either side can have intermittent non-finite values after its first finite element, the helper may return an index that is not actually jointly finite.

One more underweighted point is the opportunistic GPU path. `compute_indicator_f64` will try CUDA for non-CPU backends when the descriptor says GPU is preferred, but if the CUDA compute call fails it silently falls back to CPU. That is probably acceptable product behavior today, and I would **not** change it in a narrow patch. But the third-pass packet should have called it out as an explicit route-truthfulness concern. Without that note, it is too easy for later reviewers to assume that a non-CPU ROCB request implies GPU execution for indicator acquisition, which the live code does not guarantee.

### Claims in the third-pass packet that are too weak or too strong

The most overstated claim in the current packet is the broad admonition not to tighten fail-open behavior without wider coordination. That guidance is directionally useful for route policy, but it is too blunt for the live code that is actually here. Some fail-open behavior in this file is plausibly intentional product policy, such as best-effort CPU fallback after a CUDA indicator miss. Other fail-open behavior does **not** read like product policy. Falling back to `descriptor.outputs[0]` for a required multi-output Donchian binding, or returning a truncated desired-position vector on malformed inputs, looks much more like accidental local weakness than a user-facing compatibility choice.

The packet is also too weak on the buffer-reuse path. It described the function as allocation-light and worth preserving, which is fair, but it did not challenge whether the current reserve logic actually achieves the intended allocation behavior. It does not. That omission matters because it lets the existing code look more “finished” than it is.

The packet is further too weak on the first-valid helper semantics. It praised the expected-or-scan behavior as resilience, but in the joint case the implementation is only resilient under a monotonic-warmup assumption. That assumption may be fine; it still needs to be named as an assumption.

Finally, the packet treated the stringly dynamic-indicator boundary primarily as a documentation problem. That is incomplete. For ROCB, the risk is not just that a future contributor will find the code hard to read. The risk is that a future indicator-catalog change will bind the wrong output without forcing ROCB to fail closed.

### Safe-now hardening

The highest-value safe-now change is to fix the reserve arithmetic in `build_rocb_desired_positions_into`. That is a narrow, behavior-preserving patch with immediate value: it makes the buffer-reuse API actually honor its intended “pre-grow once, then push” behavior instead of sometimes relying on a later reallocation. This is the cleanest improvement in the file and should come before any documentation-only pass.

The next safe-now change is to harden the malformed-shape path without turning the function into a behavioral redesign. The safer narrow option is to add a checked internal sibling such as a `try_build_rocb_desired_positions_into(...) -> Result<(), String>` and have current internal callers use it where diagnostics matter. If the public `*_into` function must remain infallible, it should at least stop emitting a truncated vector on malformed input. The current silent truncation is the most dangerous part of the existing contract. A debug assertion is also justified here because the function’s callers appear to rely on same-length series.

The next hardening target is `resolve_output_id`. For ROCB’s Donchian binding, falling back to the first declared output is too permissive. The narrower safer move is **not** to rip out all heuristics. It is to keep the existing exact/synonym matching for now, but fail closed when no acceptable upper/lower match is found on a multi-output descriptor. Single-output fallback can stay for ATR-like cases. That preserves flexibility where the descriptor truly offers only one semantic choice, while removing the worst silent-drift case.

`first_joint_finite_idx_expected_or_scan` should also be tightened, or at minimum named more honestly. The low-risk fix is to keep the fast path exactly as written, then perform a real forward scan if the expected index misses instead of just taking `max(first_finite(lhs), first_finite(rhs))`. If even that is considered too behavior-sensitive, the helper should at least be documented as assuming monotonic warmup tails, and a unit test should lock that assumption in. As written, the helper name implies more than the code actually guarantees.

Test coverage deserves a sharper target than the earlier packet suggested. The next tests should not just be more entry/exit smoke tests. The missing tests that matter are the ones that lock down the fragile edges: malformed-length handling, `long_only = false`, `allow_flip = true` versus `false`, and the joint-finite fallback behavior under staggered non-finite prefixes. If output-resolution hardening is implemented, add a catalog-level unit test or descriptor fixture test so that future `vector-ta` output renames fail loudly instead of drifting silently.

One more safe-now note is diagnostic only: the file should document that the CUDA indicator path is best-effort and may fall back to CPU. I would not change that behavior in this pass. But I would make it explicit, because the current code path matters for route-truthfulness and performance expectations.

### Larger redesign still deferred

A typed or generated strategy-local binding layer for indicator IDs, param keys, and output IDs is still a larger redesign and should remain deferred. The fourth pass strengthens the case that ROCB’s descriptor binding is brittle, but the right long-term fix would touch broader dynamic-indicator patterns, not just this file.

The API split between `RocbBreakoutSignals` / `RocbRiskSignals` carrying `first_valid`, while `build_rocb_desired_positions_into` separately trusts an external `start_t`, is also a real design smell. It spreads one invariant across multiple public values and leaves the builder unable to validate its own start semantics. Even so, this is not the next patch. The next patch should harden the current API, not replace it.

If product/report surfaces ever need to state whether ROCB actually used GPU or CPU for indicator acquisition, that should be solved above this file. Right now `compute_indicator_f64` can fall back silently, which means “requested backend” and “actual indicator compute route” are not the same thing. Surfacing that distinction would be useful, but it is an orchestration/reporting redesign, not a small local cleanup.

Likewise, splitting `rocb.rs` into separate CPU-semantics and CUDA-sweep modules remains optional and deferred. The current file is still coherent enough that a split would cost more than it saves unless ROCB churn increases.

## Implementation Priority

The action bucket should remain `Small surgical refactor`, but the order of work should be more concrete than the third-pass packet implied.

1. Fix the `Vec::reserve` misuse in the caller-owned desired-position builder.
2. Harden the malformed-shape path so it no longer silently returns a truncated vector, ideally through a checked sibling used by internal callers.
3. Tighten `resolve_output_id` for required multi-output bindings so Donchian upper/lower selection fails closed on descriptor mismatch.
4. Correct or explicitly constrain the joint-first-valid helper, and add tests that lock down the intended warmup assumption.
5. Add the missing short-side / flip / malformed-input tests.
6. Leave route-accounting changes, typed descriptor redesign, and module splitting deferred.

That is still a small-surgical plan. It just targets the actual sharp edges in the file instead of treating the remaining work as mostly narrative cleanup.

## Verification If Changed

At minimum, keep the existing file-local ROCB tests and expand them around the real contract edges this pass identified. The first command to keep is still:

- `cargo test -p ta_app_core rocb -- --nocapture`

If the desired-position builder changes, add explicit tests for malformed input lengths and for the guarantee that the produced vector length matches `close.len()` on successful paths and on whichever failure semantics are chosen. If the trading loop is touched at all, add the missing short-entry and flip-behavior tests before accepting the patch.

Because ROCB is a composite-strategy-local surface, any behavior-affecting change should also be checked through representative ROCB paths in composite optimization and composite validation. If descriptor binding or CUDA device-sweep helpers change, re-run the ROCB CUDA coverage in a CUDA-enabled build as well, because the output-id / param-key strings here are effectively part of the strategy’s CPU/GPU bridge contract.

If future work chooses to surface actual CPU-versus-GPU indicator execution, that needs route/report verification outside this file too. That is not required for the small hardening steps above.

## Regression And Feature-Preservation Constraints

`RocbStrategyParams` is a public, serde-backed surface and should still be treated as a persistence contract. This file does not directly own a Tauri IPC command, but the serialized config is still part of the repo’s public request/artifact ecosystem. Do not rename fields or change defaults without treating the change as compatibility-sensitive.

Do not change the current fast/slow-to-channel/ATR mapping defaults, long/short entry rules, ATR-stop semantics, channel-exit semantics, or `max_trade_bars` counting rules unless CPU optimization and validation parity are both checked together. Those are strategy semantics, not style choices.

Do not “fix” the dynamic-indicator bridge by broadly renaming IDs or changing the `*_start` / `*_end` / `*_step` period-sweep parameter construction in this file alone. Those strings are effectively part of the dynamic-indicator / CUDA-dispatch ABI. Any change there needs a broader contract audit.

Keep the caller-owned buffer API and the `OnceLock` descriptor cache. The fourth pass found bugs around how those pieces are used, not evidence that the abstractions themselves should be removed.

Do not convert the current best-effort CUDA indicator path into a hard failure in this file alone. That would change actual runtime behavior and route expectations in a way that is much broader than the safe-now hardening recommended here.

Finally, any malformed-input hardening should preserve the current no-panic posture in release builds unless callers are upgraded in the same change. A checked sibling plus debug assertions is a safer move than an abrupt public-API break.

## Fourth-Pass Conclusion

The third-pass bucket should **stand**: `crates/app_core/src/rocb.rs` is still a `Small surgical refactor`. But the current third-pass packet is too abstract and too deferential to existing fail-open behavior. It got the compatibility posture right, yet it missed several sharper local issues: a real `Vec::reserve` misuse in the hot buffer-reuse path, a truncated-vector failure mode on malformed signal lengths, an overly permissive multi-output descriptor fallback that can silently bind the wrong Donchian series, and a “joint scan” helper that is less robust than its name suggests.

That changes the recommended emphasis of the packet. The next work here should not be led by more generic documentation cleanup. It should be led by narrow contract hardening and targeted tests, while leaving typed descriptor redesign, route-accounting changes, and larger module decomposition deferred. In other words: same bucket, but a materially sharper packet.
