# Theorem Candidate Screen

Attach the copied files from this bundle and use this prompt:

```text
I want you to act as a theorem-candidate screener for this repository.

Using the attached repo context and broad literature search, identify which mathematically explicit research directions are actually worth theorem-level effort here, and which should be rejected before more time is spent.

Constraints:
- Do not generate a broad idea list.
- Focus on directions that are both repo-grounded and potentially valid as CS.
- For each direction, explicitly classify the critical assumptions as:
  - repo-supported
  - plausible but unverified
  - unrealistic
- Reject directions that depend mainly on unrealistic assumptions.

Candidate direction families to evaluate:
- distortion-aware multi-fidelity screening
- restricted certified elimination / exact pruning
- structure-extraction correctness for a compiler or IR layer
- batch-delayed budgeted optimization theory
- a mathematically meaningful negative-result or impossibility direction

Deliverables:
1. Rank these direction families from strongest to weakest.
2. For each family, give:
   - the exact formal object that would be studied
   - the strongest plausible theorem / guarantee / correctness claim
   - the assumptions required
   - the closest prior work
   - the strongest likely reviewer objection
3. State which directions should be killed immediately.
4. Choose one leading positive-theorem direction and one leading negative-result fallback.
5. State the minimum proof burden for each of those two survivors.
```
