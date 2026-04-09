GitHub Actions workflows are intentionally deactivated in this repository.

The archived definitions in this folder are retained for reference and possible future reactivation:

- `ci.yml.disabled`
- `desktop-smoke.yml.disabled`

GitHub only loads workflows from `.github/workflows/`, so moving them here disables them without deleting their contents.

If these workflows are reactivated later, they should mirror the current local-first quality contract rather than redefining it. The canonical static-analysis entrypoint is:

- `powershell -ExecutionPolicy Bypass -File scripts/run_repo_quality_gate.ps1`
