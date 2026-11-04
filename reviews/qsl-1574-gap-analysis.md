---
id: SR-1574002
title: "gap-analysis of QSL-1574 enum declaration comparison and ten-category catalog restatement (quire-spec-language PR 2231)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-285-*.md; spec/test-cases/TC-769-*.md; spec/decisions/ADR-013-*.md; quire-specification FR-141, TC-186; quire-semantic-value FR-131"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-285
    type: references
---

# Gap analysis of QSL-1574 enum declaration comparison (quire-spec-language PR 2231)

## Summary

Ticket: QSL-1574. Head a46867d0eaf45ac3c36bcbbb0df761486423cd2a. Planless; criteria read from the spec in the repo, QSpec at cceef5918 and QSV at ab81a09c. Plan completion: not assessed.
Planner ruling (d) is honoured: FR-322 identity is unchanged, nothing is hashed, and the declaration's members and order are compared by quire-semantic-value FR-131. QSpec FR-141-AC-2 and TC-186 T09 (owner, case or order change refuses ill_typed) are asserted by t07b for case-set and order differences with value assertions, and the meter stays empty (FR-141-AC-5). FR-285-AC-1 (ten rows in TC-769 step 1) is asserted with the values 0,10,10,20,21,22,0,11,22,30 and trace exit equal to exit for every category; no assertion was weakened. FR-285-AC-11 and TC-769 step 10 have no test.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-285-AC-11 (and TC-769 step 10) require that with a catalog holding Indeterminate and Pending the label and exit of a finite-window Pending and Indeterminate outcome are the catalog's and QSL holds no local mapping. The catalog now holds them, but no test is tagged for AC-11 and qsl-replay still holds the local mapping (code review SR-1574001 FND-002). | spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:153 | correct-requirement-no-evidence |
| FND-002 | medium | FR-285's "Interim mapping until Contract IR IR-1281" section still says the catalog Category holds eight values and has neither Indeterminate nor Pending, and the Behavior bullet and AC-12 describe the interim mapping; the dependency and TC-769 step 10 are marked gated on IR-1281. With the ten-value catalog adopted, the text is false and the gating no longer applies. Spec lane work, not a code change. | spec/functional/FR-285-map-every-outcome-category-to-one-exit-code.md:78-90 | missing-requirement |
| FND-003 | low | t07b names a variant "different ordering flag" but it also reorders the members (ordered false with DONE, READY against ordered true with READY, DONE), so a QSV that ignored the ordered flag would still refuse it on the member list. QSV FR-131-AC-4 (a difference only in the ordered flag yields DistinctEnumDeclarations) is therefore not exercised by a QSL test. | tests/it/text_enum_identity.rs:499-511 | implementation-bug-despite-evidence |

## Verdict

Enum-comparison coverage (QSpec FR-141-AC-2, TC-186 T09 for case set and order) is sound and non-vacuous. Gaps: FR-285-AC-11 untested and the FR-285 interim text stale (FND-001, FND-002), and the ordered-flag-only case is unproven (FND-003).
