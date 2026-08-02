# Local patches

This subtree is based on the published `embed-resource 3.0.6` crate and
upstream git revision `f1c7e7acf94ddbd19e6c3b683ba6138c4c9f03b4`, as recorded
in `.cargo_vcs_info.json`. The upstream MIT license is preserved in `LICENSE`.

VectorGrid carries one narrow Windows/MSVC patch in
`src/windows_msvc.rs`: when `EMBED_RESOURCE_RC_PATH` is present, resource
compilation requires it to identify an absolute, ordinary `rc.exe` file and
rejects a Windows reparse point on either the file or any ancestor before
passing that exact path to `Command::new`. Invalid overrides fail closed. When
the variable is absent, the upstream Windows SDK discovery and `PATH` fallback
behavior is unchanged. The fork is a workspace member so its unit tests run in
the normal workspace test lane.

The opt-in override lets the CPU reliability-soak launcher pin and attest the
exact Windows resource compiler used by `tauri-winres`, whose public toolkit
path setters are no-ops in the selected dependency version. The same launcher
may supply a fresh, lowercase hexadecimal nonce through
`EMBED_RESOURCE_RC_ATTESTATION_NONCE`. A nonce is accepted only alongside the
explicit compiler override. After `rc.exe` exits successfully, the fork emits a
`VECTORGRID_EMBED_RESOURCE_RC_ATTESTATION` Cargo `rustc-env` value containing
that nonce and the exact compiler path. Failed compiler invocations never emit
the success marker.
