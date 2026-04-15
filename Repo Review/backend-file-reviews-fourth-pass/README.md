# Backend Fourth-Pass Bundle Layout

This folder contains flattened, per-file GPT-5.4 Pro review bundles generated from the completed third-pass backend review corpus.

Structure:
- `Tier 1/`: mandatory GPT-5.4 Pro review bundles
- `Tier 2/`: broadened GPT-5.4 Pro review bundles
- `Tier 3/`: lower-yield bundles held unless the scope widens to all files

Each per-file bundle folder contains only files:
- `prompt.txt`
- `bundle_manifest.txt`
- a flattened copy of the target source file
- flattened copies of the first-pass, second-pass, and third-pass review packets
- flattened copies of repo guidance files and a small set of adjacent context files

Authoritative planning document:
- `Repo Review/backend_review_gpt54_pro_evaluation_plan.md`
