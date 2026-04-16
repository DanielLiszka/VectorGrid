# Fourth-Pass Review: `src-tauri/src/commands_reports_shared.rs`

- Review date: `2026-04-16`
- Source file: `src-tauri/src/commands_reports_shared.rs`
- First-pass packet: `review__first_pass.md`
- Second-pass packet: `review__second_pass.md`
- Third-pass packet: `review__third_pass.md`
- Third-pass action bucket: `Small surgical refactor`
- Fourth-pass action bucket: `Small surgical refactor`
- Packet status: `Reviewed`

## Why This File Got A Fourth Pass

The third-pass packet moved in a useful direction by treating `commands_reports_shared.rs` as more than report-path glue, but it still centered the wrong risk. The file does sit behind public desktop commands, yet the live source still looks like a bounded support module with user-visible path, persistence, and shell-launch contracts, not a module that now obviously demands helper-level allowlisting or a broader security redesign.

The fourth pass was therefore worth doing for a narrower reason: to challenge whether the third-pass framing was too broad, and to see whether there were smaller, more concrete edge cases in the live source that matter more than another round of `exists()` commentary.

My conclusion is that the action bucket should stand. The safe-now work is still surgical. But the next packet should stop acting as though helper-local path scoping is the most likely next step. The sharper issues are exact-path preservation, supported-platform shell behavior, and the fact that report-path collision probing is still only advisory rather than atomic.

## Fourth-Pass Research Log

1. Live source read against the current third-pass packet:
   - `source__src-tauri__src__commands_reports_shared.rs`
   - key lines:
     - `quantile`: lines `6-15`
     - `existing_path`: lines `18-29`
     - Linux reveal behavior: lines `71-84`
     - Windows open behavior: lines `96-104`
     - report-path suggestion: lines `167-188`

2. Public command reachability and re-export context:
   - `context__01__src-tauri__src__lib.rs`: `open_path` / `reveal_path` are in the Tauri invoke surface at lines `126-133`
   - `context__02__src-tauri__src__commands.rs`: the shared helpers are re-exported or wrapped at lines `132-139`

3. Cross-file persistence comparison:
   - `context__04__src-tauri__src__run_artifacts.rs` carries the same timestamp-suffix probing pattern at lines `501-525`, which matters when evaluating whether a file-local collision fix would really stay local

4. Official Rust docs consulted:
   - `std::path::Path::exists` / `try_exists`
   - `std::fs` TOCTOU guidance
   - `OpenOptions::create_new` / `File::create_new`
   - `std::process::Command`
   - `std::path::Path::parent`
   - `f64::clamp`
   - Rust Reference float-to-int cast rules

5. Platform docs consulted:
   - Microsoft Learn `cmd`
   - Microsoft Learn `start`
   - POSIX filename/pathname rationale

These sources were enough to validate the two main questions the third pass left open:
- which risks are real on currently handled platforms
- which fixes can actually be called “safe now” from this file alone

## What Fourth Pass Sharpened

### What the third-pass packet got right

The current third-pass packet got three important things right.

First, the bucket should stay small. This file is still support code with user-visible effects, not a hot-path runtime subsystem.

Second, it was right not to oversell `spawn()` as proof that the OS definitely opened the target. This helper can only report “launch attempted”.

Third, it was right that `Path::exists()` is too lossy for path diagnostics. That concern remains valid for both launch preflight and report-path probing.

### What it still missed or underweighted

#### 1. `existing_path()` currently rewrites valid paths

The most concrete file-local bug is not “unsupported platform returns `Ok(())`”. It is that `existing_path()` trims the incoming string before building the `PathBuf` (`18-29`).

That is convenient for blank-input rejection, but it also mutates the actual path bytes. On POSIX platforms, filenames/pathnames can contain any bytes other than slash and NUL, so leading and trailing spaces are legal. The current helper can therefore reject or mis-target valid Linux/macOS paths simply because it normalizes them before checking existence.

That is a real correctness issue on supported platforms. It is also a much narrower and safer fix than helper-level allowlisting. The smallest justified change is to use `trim()` only to detect an all-whitespace input, while constructing the `PathBuf` from the original string.

#### 2. Linux reveal has a relative-path edge case the third pass did not call out

The Linux reveal branch (`71-84`) tries to fall back from a file path to its parent directory. The intent is reasonable, but the current implementation uses:

- `path.parent().unwrap_or_else(|| Path::new("."))`

Rust documents that `Path::parent()` returns `Some("")` for a relative path with one component. That means a relative existing file such as `report.xlsx` does not hit the `unwrap_or_else` fallback at all; it produces an empty path, not `"."`.

So for Linux, `reveal_path_blocking("report.xlsx")` can end up calling `xdg-open ""` instead of opening the current directory. The third-pass packet did not mention this supported-platform reveal bug, and it is more actionable than the unsupported-target fallback.

This is still surgical to fix: treat an empty parent path the same way as `None`.

#### 3. `try_exists()` is only a diagnostic improvement for collision probing, not a real race fix

The third-pass packet still made `try_exists()` sound like the main path-forward for report-path slot selection. That is too optimistic.

`suggest_report_path_in_dir()` (`167-188`) is a pure existence-probe helper. It returns a candidate path, but it does not reserve that slot. So even if every `exists()` call becomes `try_exists()`, the function is still non-atomic: another save can create the same file between suggestion and write.

Rust’s own filesystem guidance is explicit here: when possible, avoid TOCTOU by using atomic create-new operations. That means the real collision-safety fix is not inside this helper alone. It has to live at the writer site, or the helper API has to change so it actually creates or reserves the file.

This matters because the third-pass packet makes the probing API look like the core issue, when the deeper persistence risk is advisory allocation itself.

#### 4. The Windows `cmd /C start` path is a real supported-platform shell edge

`open_path_blocking()` on Windows (`96-104`) routes launches through:

- `cmd /C start "" <path>`

That is understandable because `start` is how the code gets file-association behavior for non-executable files. But this also means the path is being interpreted by `cmd`’s command language before `start` runs.

The third-pass packet talked about best-effort `spawn()` semantics, but it still missed the more specific Windows issue: because this route goes through `cmd`, shell parsing and environment-variable expansion rules apply. This is not a broad “injection” claim against `Command`; Rust is still passing structured arguments to `cmd.exe`. But it is still a genuine shell-compatibility edge for literal filenames containing command-meaningful sequences such as `%VAR%`.

That is a higher-value supported-platform concern than the unsupported-target `Ok(())` tail.

#### 5. The quantile edge case is narrower than a semantics rewrite, but still worth pinning

The earlier packets were right that `quantile()` is semantics-sensitive. The fourth pass adds one narrower point: the helper has an unpinned non-finite-input behavior.

The code clamps `q` and then casts the rounded floating index to `usize` (`13-15`). Rust documents that `f64::clamp` preserves `NaN`, and Rust float-to-int casts convert `NaN` to `0`. So `quantile(sorted, f64::NAN)` silently returns the first element.

I would not treat that as justification to redesign the quantile method. But if the packet is already asking for more quantile tests and clearer contract notes, this edge belongs in scope. The safe move is to document or assert the finite-`q` expectation, not to silently change user-visible meta statistics.

### What the third-pass packet overstated

#### 1. Helper-level path scoping is too strong a recommendation from the current evidence

The third-pass packet framed this file as though the next architectural choice is “either keep arbitrary existing-path launch or add allowlists”.

That is too strong.

The local context shows that `open_path` / `reveal_path` are part of the public command surface, but it also shows that this helper sits in a generic cross-workflow utility lane, not a narrowly report-scoped path API. The earlier packet also tied these helpers to report, history, meta, ML-signal, data, and options-expression flows. In other words, the current product contract appears to be “open an existing desktop path that the workflow already knows about”, not “only open app-local report files”.

Introducing helper-level root allowlists here would be a product behavior change, not conservative hardening. If path scoping is ever needed, it belongs at specific public commands or caller workflows, not as a surprise narrowing of this shared helper.

#### 2. Unsupported-platform explicit errors are reasonable cleanup, but not the most important issue

Replacing the final `Ok(())` with an explicit unsupported-platform error is not wrong. But the third-pass packet still made it feel more central than it is.

The current code already has handled branches for Windows, macOS, and Linux. The bigger review value is in supported-platform behavior:
- path trimming
- Linux relative reveal fallback
- Windows `cmd/start` shell semantics
- non-atomic collision allocation

Those deserve priority first.

#### 3. `try_exists()` was treated as a bigger win than it really is

For launch preflight, `try_exists()` is good hygiene. For filename-slot selection, it is only partial hardening. The persistence-sensitive part is atomic reservation, not more honest probing.

The third-pass packet should have been clearer about that distinction.

## Implementation Priority

### Safe-now hardening

1. Preserve exact path bytes in `existing_path()`.
   - Keep whitespace-only rejection, but build the `PathBuf` from the original string rather than the trimmed one.
   - Add a Unix-only regression test for a valid filename with leading or trailing spaces.

2. Fix the Linux relative-file reveal fallback.
   - Treat `parent == ""` the same as no parent and fall back to `"."`.
   - This is a real bug in the current branch, not just documentation debt.

3. Upgrade open/reveal preflight from `exists()` to `try_exists()`.
   - This is still worthwhile for diagnostics, especially for missing vs inaccessible paths.
   - But the packet should describe it honestly as a launch-preflight improvement, not a collision-safety fix.

4. Add tighter tests around helper contracts that are already stable.
   - `quantile()` finite-input boundaries
   - `quantile()` non-finite `q` behavior or explicit finite-input assertion
   - `sanitize_filename_component()` truncation and replacement behavior
   - blank-path rejection without changing exact-path bytes

5. Document the real current shell contract.
   - launch success means process-spawn success only
   - Linux reveal opens the containing directory, not a highlighted file
   - filename-slot suggestion is best-effort and non-reserving

### Larger redesign still deferred

1. Atomic filename reservation for report writes.
   - If collision safety matters under concurrent saves, move path allocation to the write site with `create_new`-style reservation, or redesign the helper to reserve/create the file itself.
   - Do not describe a `try_exists()` swap as solving this.

2. Windows launcher replacement, if pursued.
   - The current `cmd /C start` path has shell semantics, but it also preserves file-association behavior.
   - Any replacement needs real desktop verification on Windows before landing.

3. Quantile-method changes.
   - Do not casually replace the rounded-index selector with interpolation or spreadsheet-style percentile behavior.
   - Earlier review history already noted that another local quantile copy exists in the workbook path, so changing only this helper risks output drift across surfaces.

4. Product-level path scoping.
   - If the repo wants tighter path policy, add it at the command or workflow layer where allowed roots are actually known.
   - Do not silently narrow this generic helper and hope the rest of the app still works.

## Verification If Changed

1. Rust unit scope:
   - `cargo test -p ta_desktop_demo_app commands_reports_shared -- --nocapture`

2. Adjacent report wrapper coverage:
   - `cargo test -p ta_desktop_demo_app commands_reports_api -- --nocapture`
   - `cargo test -p ta_desktop_demo_app commands_reports -- --nocapture`

3. Cross-file persistence checks if collision behavior changes:
   - `cargo test -p ta_desktop_demo_app run_artifacts -- --nocapture`

4. Any quantile or meta-summary behavior change:
   - `cargo test -p ta_desktop_demo_app commands_meta -- --nocapture`

5. Frontend contract coverage if open/reveal behavior changes:
   - the existing `openPath` fallback tests and representative service tests called out in the prior packets should still run, because frontend behavior depends on “open first, reveal on error”.

6. Manual desktop verification:
   - open an existing report file
   - reveal an existing report file
   - open the app-local reports directory
   - if Windows launch code changes, manually verify a path whose filename contains `%...%`
   - if Linux reveal fallback changes, manually verify a relative one-component file path

## Regression And Feature-Preservation Constraints

- Preserve the current generic helper role unless product-level command scopes are intentionally changed.
- Do not silently root-allowlist this helper; that would be a behavior change across multiple workflows.
- Do not claim that `spawn()` proves OS-open success.
- Do not present `try_exists()` as a complete fix for collision safety.
- Keep filename base format, suffix numbering, and report discoverability stable unless downstream tests and callers are updated together.
- Any Windows launcher change must preserve default file-association behavior for non-executable files and normal directory opening.
- Any quantile behavior change must be coordinated across all report/meta surfaces, not just this helper.
- No performance, CUDA, or GPU ABI action is warranted here; the real risks are persistence correctness and desktop-shell compatibility.

## Fourth-Pass Conclusion

The current third-pass bucket should stand: `Small surgical refactor` is still the right call.

What changed in the fourth pass is the emphasis. The third-pass packet was right to keep the work bounded and right to keep best-effort launch semantics explicit. But it still over-centered helper-level trust-boundary language and underweighted narrower, more provable issues in the live code.

The sharper review is:

- safe-now:
  - stop trimming exact path bytes
  - fix Linux relative-file reveal fallback
  - use `try_exists()` for launch diagnostics
  - pin and document current helper semantics with better tests

- still deferred:
  - atomic path reservation at the write site
  - Windows launcher replacement away from `cmd /C start`
  - quantile-method changes
  - any product-level path allowlisting

That keeps the recommendations conservative, evidence-backed, and aligned with the real regression surface of this file.
