# Counterexample Attack

Reuse the same copied attachment set and use this prompt on the leading survivor:

```text
Attack the leading theorem candidate from the prior answer.

Do not defend it. Try to break it.

Tasks:
1. Construct the strongest plausible counterexamples, edge cases, or repo-grounded failure scenarios.
2. Identify where the assumed structure fails:
   - non-smooth objective regions
   - ranking inversions across fidelities
   - lack of admissible bounds
   - strategy-family heterogeneity
   - route-dependent behavior that breaks the claim
3. Separate:
   - fatal counterexamples
   - repairable scope restrictions
   - harmless edge cases
4. State whether the candidate can be saved only by narrowing scope to:
   - DoubleMa only
   - a restricted family with explicit assumptions
   - a correctness-only compiler/IR result
   - or a negative-result framing

Deliverables:
1. Fatal counterexamples
2. Repairable restrictions
3. The narrowest version of the claim that still looks publishable
4. The point at which the claim becomes too narrow to matter
5. A keep / narrow / abandon verdict
```
