# VectorGrid Broader CS Prompt Bundles

Generated: 2026-04-08

Source pack: `SASS research/vectorgrid_broader_cs_research_question_pack_2026-04-05.md`

These folders package the broader CS research question pack into self-contained upload bundles. Each question folder contains:

- prompt files for the first pass, second pass, and final consolidation pass
- a `files/` directory mirroring the original repo-relative paths for every shared baseline file and question-specific attachment

## Question Folders

1. `Question1` - `Question 1: What Should The Actual Research Target Be If “Beat Brute Force” Is Wrong?` (12 copied files)
2. `Question2` - `Question 2: Is Multi-Fidelity Optimization The Best Broad Direction?` (14 copied files)
3. `Question3` - `Question 3: Would Optimizer Portfolios Or Online Algorithm Selection Be Stronger Than One Universal Optimizer?` (14 copied files)
4. `Question4` - `Question 4: Can A Batch-Synchronous, Strategy-Agnostic Optimizer Still Be A Real Contribution?` (13 copied files)
5. `Question5` - `Question 5: Is There A Real Exact-Pruning Or Certified-Elimination Angle?` (14 copied files)
6. `Question6` - `Question 6: Could A Search-Space Compiler Or Structure Extractor Be A Better Systems Direction?` (15 copied files)
7. `Question7` - `Question 7: Could A Benchmark Or Platform Direction Be Better Than A New Algorithm?` (14 copied files)
8. `Question8` - `Question 8: Which Venue Family Best Fits The Strongest Remaining Direction?` (12 copied files)

## Prompt Flow

1. Start with `prompts/01_first_pass_prompt.md` inside the question folder.
2. Reuse the same attachment set with `prompts/02_second_pass_prompt.md`.
3. Reuse the same attachment set again with `prompts/03_final_consolidation_prompt.md`.

## Regeneration

Rebuild these bundles with:

`node scripts/generate_vectorgrid_broader_cs_prompt_bundles.mjs`
