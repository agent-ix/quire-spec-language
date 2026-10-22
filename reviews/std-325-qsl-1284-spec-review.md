---
id: SR-16194
title: "Spec review of quire-spec-language PR #1284: STD-325, FR-430-AC-3 both-selected reduction vector"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@753668e6; PR #1284 (git diff origin/main...753668e6): spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md, spec/test-cases/TC-1170-timed-refinement-reductions-and-strong-fairness.md; read for context: QSpec PR #385 head eddbc43a (FR-382-AC-7, TC-963 TR-06)"
review_set: subset
evaluated_revision: "753668e6"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1170
    type: reviews
---
# SR-16194: Spec review of quire-spec-language PR #1284: STD-325, FR-430-AC-3 both-selected reduction vector

## Summary

Ticket: STD-325 (QSpec #385 SR-16185 FND-001). Reviewed `git diff origin/main...HEAD` at 753668e6. FR-430-AC-3 gains the item selecting both partial-order reduction and a state constraint, settling V-8 `unsupported-requested-capability` naming partial-order reduction only, which is the FR-430 Behavior rule ('the first such reduction in the order partial-order, constraint') and matches QSpec FR-382-AC-7 and TC-963 TR-06 at eddbc43a word for word in substance. TC-1170 step 3 adds the request and its expected result. The vector can fail on a refusal naming the constraint, naming both, or running the item. No new ids; no other open QSL PR touches FR-430 or TC-1170. `QSPEC_DIR=<QSpec main 6f24ddd4> make check-spec-validation` exits 0 with no diagnostic on a touched file. The head is one commit behind QSL main 84c61f0ab and merges with it cleanly, and with #1283.

Units examined:

- `FR-430-AC-3` (examined): spec/functional/FR-430-check-refinement-reductions-and-strong-fairness-over-timed-subjects.md
- `TC-1170 step 3` (examined): spec/test-cases/TC-1170-timed-refinement-reductions-and-strong-fairness.md
- `QSpec FR-382-AC-7` (context_only): agent-ix/quire-specification PR #385

## Verdict

**MERGEABLE.** No findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
