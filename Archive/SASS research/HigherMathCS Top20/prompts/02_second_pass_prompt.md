# Second-Pass Prompt

Reuse the same copied attachment set from the first pass.

Use this prompt only after the first-pass answer exists:

```text
Review your prior answer as a critical second pass.

Do not restate the first answer. Pressure-test the mathematics.

Tasks:
1. Identify the assumptions that are carrying the claimed theorem, guarantee, or impossibility result.
2. Reclassify each important assumption as:
   - repo-supported
   - plausible but unverified
   - unrealistic
3. Cut any direction whose mathematical claim depends mainly on unrealistic assumptions.
4. State whether the leading direction still has value if the theorem is weakened to:
   - a local guarantee
   - an instance-dependent guarantee
   - a distortion bound
   - or only a correctness property plus strong empirical evidence
5. Identify the strongest likely reviewer objection that the math is ornamental rather than essential.
6. Compare the leading direction against:
   - one more conservative systems-oriented fallback
   - one more ambitious but riskier theorem-oriented alternative

Deliverables:
1. Keep / revise / reject verdict on the first-pass leading direction
2. Revised leading direction
3. Strongest fallback direction
4. Explicitly rejected directions
5. Minimum remaining proof obligations
6. Minimum remaining benchmark obligations
7. Kill test
```
