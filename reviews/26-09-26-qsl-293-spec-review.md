---
id: SR-744
title: "Spec review of TC-426 step 3 status flip"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@09e9d18585d4aed678ce681ef824a24b19a24b27; spec/test-cases/TC-426-a-check-location-resolves-to-its-unit-region.md; spec/tests.md; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/functional/FR-095-occurrence-keyed-source-map-and-locus.md; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/TC-426
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
---
## Summary

Ticket: QSL-293 (blocks QSL-160). PR: quire-spec-language#493 at 09e9d185.
The spec edits are the TC-426 Status section and the TC-426 row in
`spec/tests.md`. No FR-100 to FR-111, TC-450 to TC-469 or `.github/workflows`
file is touched.

Sound: flipping the TC-426 row to Passed is honest. Steps 1, 2 and 4 are
backed by the QSL-239 tests. Step 3 is backed by the new test, which checks all
four locations under `d` shifted by `k`, from the declarations and from the
checked package. A single optional embedding per package matches ADR-013 C-21
(ADR-013:970, :324). There, `SourceMap` maps one embedded body to one document
and keeps the document's `RawSourceRef`, and the ADR models no nested or
multiple embedding for one unit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec does not define the split-span disposition, and the code contradicts it. FR-096 says a location "SHALL resolve to no region in exactly three cases" (FR-096:95-107). Each of those cases is a tree not read from the unit. The PR adds a fourth case: an embedded span that the map splits into more than one document region (a layout deletion) resolves to `None`. TC-426's Status now states that as behaviour. AC-1 and C-21 say only that the span is "mapped to the document region". Neither says what happens when the mapping gives several regions. The owner must decide: (a) resolve to the enclosing document region (first start to last end; one contiguous region under `d`), (b) add it to FR-096 as a fourth no-region case, or (c) make it a refusal. The rule then goes into FR-096 and an AC. Today the case is unreachable: `qsl-source` builds single-segment maps (qsl-source/src/lib.rs:319-333), and production never sets `embedding`. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:89-107; spec/test-cases/TC-426-a-check-location-resolves-to-its-unit-region.md:51; qsl-semantics/src/check/region.rs:57-59 |
| FND-002 | medium | FR-096's Status is now stale. Its "Not built" list still contains "The embedded-document mapping of AC-1 (TC-426 step 3)" (FR-096:486). That contradicts the TC-426 flip in this PR. Update it, and keep the production-wiring gap (SR-743 FND-001) if it is deferred. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:486 |
| FND-003 | low | The tests.md S-5b coverage prose lists the TCs that pass locally (TC-427, TC-429, TC-378, TC-428, TC-500). It does not list TC-426, which is now Passed. | spec/tests.md:269-274 |

## Verdict

Approve with changes. Fix FND-002 and FND-003 in this PR. FND-001 needs an
owner ruling on the disposition before its test (SR-743 FND-002) can be
written.
