---
id: SR-911
title: "QSL-340 spec review of PR 542 (NFR-007, FR-097, TC-440 status, spec.md and tests.md rows)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@a305f56ffc793e2470708c45af4c77fba094f3a1; spec/non-functional/NFR-007-bound-native-packages.md; spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md; spec/test-cases/TC-440-qsl-extent-agrees-with-ir-requires-bound.md; spec/spec.md; spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-440
    type: reviews
---
## Summary

Ticket: QSL-340. PR: quire-spec-language#542 at a305f56f. The spec edits
are status and scope text only; no requirement statement or AC changed.

What was checked:

- No ticket id is added in any edited text. IR-279, IR-283 and IR-284
  citations are removed from status text and code comments, which is right:
  the statuses now state what holds.
- NFR-007's exception now describes IR's fixed maximum by reference to
  `CheckedPackageReadLimits::MAXIMUM_DEPTH`, without copying the number, and
  states the incomplete outcome. It matches the code and IR at ead72675.
- The spec.md FR-097 row, the tests.md TC-440 row and TC-440's Status agree
  with each other and with the test names in the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-097-AC-6's "IR's first unbounded node is a form QSL names as a domain" has no defined meaning for a recursive type. QSL names the record root as `Recursive`; IR names whichever recursion-group member has the lowest node id (here the `kids` sequence). The edited statuses (FR-097 Status, TC-440 Status, tests.md row) now say `RangedTree` agrees, which holds only under a reading the AC does not state. Two implementers would read the clause differently. | spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:74; spec/functional/FR-097-classify-claim-extent-and-write-bounded-requests.md:98-101 |
| FND-002 | low | "charged by IR at most at IR's fixed maximum" reads awkwardly; "IR charges the v2 reader's nesting ceiling up to its fixed maximum" says the same plainly. | spec/non-functional/NFR-007-bound-native-packages.md:31-33 |

## Verdict

The status edits are accurate for the code except for the `RangedTree`
agreement claim, which rests on an AC clause that does not cover recursive
types (FND-001, tied to SR-910 FND-001). One low wording nit.
