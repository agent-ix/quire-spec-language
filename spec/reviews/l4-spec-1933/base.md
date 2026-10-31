---
id: SR-1933001
title: "Base requirements review for QSL-498 retirement and QSL-1255 replacement tests"
type: SpecReview
analysis: base
scope: "FR-493, TC-1276, TC-137, TC-136, NFR-009-AC-3"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-493
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1276
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-137
    type: reviews
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: reviews
---

## Summary

FR-493 states the end state of the remaining native-path deletion by absence and names, for the runtime-input and package slice, which requirements and test cases go and which spine requirement replaces each. TC-137 now gives each of FR-255's ten counters a fixture, a charge point and an independent oracle, and NFR-009-AC-3 is a test with two controls instead of an Analysis row with no symbol.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-493 enumerates the retired requirements only for the package and runtime-input paths. The native front end (FR-001, FR-016, FR-017, FR-025, FR-035, FR-036, FR-040, FR-041, TC-101, TC-102, TC-120) and the compiled-protocol read side (FR-042 and TC-121) are retired by absence in the ACs but their per-requirement disposition is not listed. FR-036-AC-5 is cited as live by FR-049-AC-10, so those requirements need a per-criterion split by their owners, not a wholesale delete. Remaining work: QSL-837 and QSL-838 dispositions. | FR-493, FR-049-AC-10 |
| FND-002 | medium | FR-008 and its TC-067 to TC-077 describe the native state evaluator that QSL-834 already deleted. They stay in the tree until the code change that removes the runtime paths deletes them, and IT-002, US-003, StR-001 and NFR-003 still cite FR-007 and FR-008. The code change must re-point or remove those citations. | FR-493 retired table |
| FND-003 | low | TC-137 names `quire-exact` charge points for the text, decimal and unit counters. Whether each is reached by a checked-term entry point was not verified in this review; the test case requires the admitted-charge log to prove it and records a counter no fixture reaches as inapplicable, with that evidence. | TC-137, FR-049-AC-7 |
| FND-004 | low | TC-137 control C2 uses an injected denial as the stand-in for an allocator refusal, because no checked-term call can be made to fail an allocation deterministically. It proves the typed record for a non-limit denial, not a real allocation failure. | NFR-009-AC-3 |
