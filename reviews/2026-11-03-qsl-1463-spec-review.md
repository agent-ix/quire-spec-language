---
id: SR-2239006
title: "spec-review of QSL-1463 FR-251 retained-row contract, FR-251-AC-42 and TC-1081 step 26 (PR #2257)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ede08267050c252deecec764fe7beb3c592b5e3b; spec/functional/FR-251-derive-a-tick-based-monitor-plan-for-an-embedded-target.md, spec/test-cases/TC-1081-monitor-builds-from-the-plan-alone-and-refuses-tampered-or-unsupported-plans.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-251
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1081
    type: reviews
---
# SR-2239006: spec-review of QSL-1463 FR-251 retained-row edits (PR #2257)

## Summary

Ticket: QSL-1463. quire 0.38.3: `quire validate` exits 0 on FR-251 and TC-1081 (the only non-duplicate-archetype output is the pre-existing semantic.inline-data-schema advisory). IDs: FR-251-AC-42 and TC-1081 step 26 are unused elsewhere. TC-1081 Scope line and expected-result line 26 agree with AC-42. Sub-analyses: EARS and integrity scopes applied to the new paragraph and AC.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-251-AC-42 is narrower than the paragraph it verifies. The paragraph says every retained witness row is re-keyed to the nodes the writer renames and none is written anew; the AC checks only declaration/field row equality, non-empty source-unit and equality rows each naming a package node, and the IR join. It does not state the re-keying obligation, nor retention of the operation-member, tuple, enum, omission or temporal-binding rows the paragraph lists, so the criterion cannot fail on the part of the contract that changed code (see SR-2239004 FND-001). | FR-251-AC-42 |
| FND-002 | low | The new paragraph is descriptive and carries no SHALL (EARS: "The Emission ... also carries"), and its row list differs from the code and the AC: it omits tuple declaration rows and omissions, which the writer carries, and says "the enum rows" without naming them model-owned. | FR-251 paragraph after the plan-root key text |

## Verdict

Mergeable after FND-001 is restated; FND-002 is wording. No contradiction with QSpec FR-487 (call closure order and keying agree with FR-251-AC-38) or FR-322-AC-309 found.
