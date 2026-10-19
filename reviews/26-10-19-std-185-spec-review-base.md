---
id: SR-12902
title: "Base review of PR #999: FR-337 clause node identity and FR-338 refused automaton state key"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@546412bb9; FR-337 Outputs, Behavior, AC-5; FR-338 canonical translation root paragraph, Behavior refused-key rule, AC-8; TC-1079; TC-1080"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1079
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1080
    type: reviews
---

## Summary

Ticket: STD-185. PR quire-spec-language#999 (head 546412bb9, diff
`origin/main...HEAD`, base 9d76f7d4c). The PR closes the two spec gaps PR
#997 (QSL-529) records: FR-337 now states the clause identity triple and
`over` node S4 emits and the I2 reader returns, and FR-338 states the
outcome for a refused `AutomatonStateKey`. New ids: FR-337-AC-5,
FR-338-AC-8, TC-1079, TC-1080. This base review covers id format and
uniqueness, AC to TC coverage, link integrity and the checklist.

`quire validate --scope .` over the four changed files exits 0 (quire 0.36.2,
engine 0.50.2). TC-1078 is the highest TC on main; TC-1079 and TC-1080 are
sequential and appear in no open PR (#998 adds no TC id; #997 and #672 touch
only FR-338-AC-1 to AC-7) and in neither merged #990 (TC-1076, TC-1077) nor
#991 (TC-1077, TC-1078). FR-337-AC-5 and FR-338-AC-8 are the next free AC
numbers and collide with nothing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-338-AC-8 says "each admission negative (a foreign context, a non-canonical encoding, an unsorted set)". It can be read as every negative FR-338 lists, or only the three in the parenthesis. FR-338's list includes bounded and window negatives (`lower>upper`, an offset outside the checked interval, an omitted `on each` debt) that a TP-1 safety key cannot carry, and TC-1080 tests only the three. The AC should say "each of these three admission negatives". | spec/functional/FR-338-check-an-en-1-closure-certificate.md:539-543,599; spec/test-cases/TC-1080-a-refused-automaton-state-key-rejects-the-closure-certificate.md:21-22 |
| FND-002 | low | The new AC rows put a TC id in the Verification cell (`Test (TC-1079)`, `Test (TC-1080)`). The checklist says a Verification cell names a method only. The rest of FR-337 and FR-338 follow the same repo convention, so this is a convention point, not a local slip. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:115; spec/functional/FR-338-check-an-en-1-closure-certificate.md:599 |

## Verdict

The id allocation is clean, and every new AC has exactly one TC that scopes
it (FR-337-AC-5 to TC-1079, FR-338-AC-8 to TC-1080). Each TC's steps and
expected results match its AC one for one. The `NodeId` in the clause root is
FR-338's existing content-addressed node identity, which binds the proof to
the exact clause it proved, so it is the allowed canonical identity digest,
not a hash/pin antipattern hit. Only two low findings come from the base
checklist. Whether the PR is mergeable depends on the mediums in the
integrity and failure-domain reviews (SR-12903, SR-12904).

Implementability note (not a defect of this diff): TC-1080 uses FR-338-AC-1's
certificate over ADR-018 §6's `ConfigVersion` subject. PR #997 reports that
no model-check fixture for that subject exists yet, so TC-1080 inherits
AC-1's fixture gap. TC-1079 compiles the same unit through S4, which TC-524
already does.
