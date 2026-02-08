# UI/UX Self-Evaluation Loop (Iterative Improvement)

## 1) Purpose

This defines a repeatable way to evaluate each redesign iteration and decide what to improve next.

Use this for every major UI revision (wireframe, prototype, implemented build).

---

## 2) Evaluation Cycle

Run this loop each iteration:

1. Define scope of iteration.
2. Score against rubric (Section 3).
3. Execute key tasks (Section 4).
4. Record failures and friction.
5. Prioritize top 3 fixes.
6. Implement fixes.
7. Re-score and compare.

Stop only when acceptance thresholds are met.

---

## 3) Scoring Rubric (0-5 per category)

Scoring scale:

- 0 = broken / missing
- 1 = severe friction
- 2 = high friction
- 3 = acceptable
- 4 = good
- 5 = excellent

Categories:

1. Task Clarity
2. Navigation Simplicity
3. Visual Hierarchy
4. Cognitive Load
5. Feature Discoverability
6. Workflow Speed
7. Error Recovery
8. Accessibility + Keyboard
9. Feedback Quality (status/progress/toast)
10. Feature Parity (no regressions)

Target:

- Average >= 4.0
- No category < 3.0

---

## 4) Task-Based Evaluation Set

Run these tasks on each iteration:

1. Download and load data from Polygon.
2. Configure and run optimization.
3. Run validation and export report.
4. Save/tag run and rerun from library.
5. Run aggregate analysis with timeframe/tag filters.
6. Find distributed tools and run status check.

Metrics to capture per task:

- completion (yes/no)
- time to completion (seconds)
- number of navigation mistakes
- number of clarification moments

Target:

- completion >= 90%
- median completion time trending down each iteration

---

## 5) Heuristic Checklist

For each screen, check:

- Is there one obvious primary action?
- Is non-essential information hidden by default?
- Are labels concise and specific?
- Is status information visible without scanning?
- Can user recover from common errors in one step?
- Is there a clear next action after success/failure?

Any "no" becomes a backlog item.

---

## 6) Regression Guardrail Checklist

Each iteration must pass:

- existing `UI_REGRESSION_CHECKLIST.md`
- feature preservation matrix from `UI_UX_COMPLETE_REDESIGN_MASTER_PLAN.md`

If any fail:

- block release of that iteration
- log regression root cause
- add automated/manual check to prevent repeat

---

## 7) Iteration Scorecard Template

```text
Iteration: <name/date>
Scope: <what changed>

Rubric Scores (0-5):
- Task Clarity:
- Navigation Simplicity:
- Visual Hierarchy:
- Cognitive Load:
- Feature Discoverability:
- Workflow Speed:
- Error Recovery:
- Accessibility + Keyboard:
- Feedback Quality:
- Feature Parity:

Average:
Lowest category:

Task Results:
1) Polygon -> Load: completion/time/errors
2) Optimize: completion/time/errors
3) Validate + Export: completion/time/errors
4) Run Library flow: completion/time/errors
5) Aggregate analysis: completion/time/errors
6) Pro tools discovery: completion/time/errors

Top Frictions:
1.
2.
3.

Top Fixes for next iteration:
1.
2.
3.
```

---

## 8) Suggested Cadence

- Wireframe phase: evaluate every major wireframe revision.
- Build phase: evaluate after each implementation phase (Phase 1-6).
- Pre-release: full rubric + full regression checklist + one external usability pass.

---

## 9) Practical Note

Self-evaluation is useful, but strongest results come from combining:

- self-scoring
- user task observation
- objective timing/error metrics

Use all three together for iteration decisions.

