---
id: SR-088
title: "Native runtime base specification review"
type: SpecReview
analysis: base
scope: "FR-007/008/018, NFR-006, native-runtime input/evaluation contracts, IT-006, TC-055–077 and TM-004"
review_set: all
evaluated_revision: "045025f843346001a91919b8d0816a519e2df337"
review_date: "2026-09-09"
---

## Summary

The LC03 packet has 42 testable functional criteria mapped to 23 planned cases and 17 explicit resource metrics. The concrete interfaces and stage outcomes are specified before implementation.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Resolved before the evaluated revision: the first drafts had no discrete runtime artifacts or exact work ceilings; FR-018, NFR-006, expanded FR-007/008, IT-006 and TM-004 now own them. | FR-007; FR-008; FR-018; NFR-006; TM-004 |
| FND-002 | low | Resolved: the baseline Node items field contains Version scalars; TC-067/070 now use that actual model and require a separately admitted source-derived model for reference-element variants. | TC-067; TC-070; tests/fixtures/native-rule-model.json |

## Checklist and six coverage rules

Read all scoped requirements, both complete API contracts, the 23 test cases,
IT-006, TM-004 and the existing US-003/StR-001 lineage. The US states operator
value with two Given/When/Then examples; it specifies no UI solution. The FRs
each own one stage with exact inputs, atomic success/failure and measurable
criteria. Requirements use distinct full IDs and the new TC range follows
TC-054. Existing requirement criteria are extended without renumbering.

Every criterion maps to a real planned TC; NFR metrics map to named boundary
suites. Options include all runtime roles, type/wrapper sites and independent
limit dimensions. Boundary cases include empty/zero, exact, one-over or
one-below, Unicode lengths, structural depth and coupled ceilings. Every
documented error code has a case. Pre/post effects and immutable retries cover
state transitions. Identity conflicts, shared nodes, cycles, duplicate sequence
occurrences, skipped values and partial events cover edge cases.

The artifacts define exact source/model/input correspondence, immutable typed
outputs and explicit actor responsibilities. Resource controls address untrusted
input without inventing an authentication/network surface. The public API has
one caller cancellation extension with explicit normal/panic behavior. There
are no selectable semantic fallbacks requiring another option matrix.

## Automated checks

data/spec-validation-full.txt is the successful scoped structural validation;
data/spec-validation.txt reports 189/189 grammar-clean documents and zero grammar
findings. Six existing module/relationship duplicate notices are registry
first-wins messages, not duplicate IDs introduced by this packet. Coverage
reports 113 existing bound Rust test symbols, no status lies and no untracked
symbols; none is offered as execution of these 23 new cases.

## Verdict and provenance

PASS for implementation of this specified LC03 API scope. Agent A applied the
actual installed QUOIN base and all seven analysis skills serially, under the
owner's existing all-review selection. No additional agents or Cargo builds
ran. No applicable required AssuranceProfile was found. The declined optional
semantic gap comparison remains excluded; this specification review still
checks the actual adopted meaning and existing interface boundaries.

All runtime test rows remain planned. Review approval does not qualify native
execution, finish LC02/FS03 acceptance or complete the original backend/Quire
workflow. Implementation changes to this contract reopen specify/review.

## Constructor setup correction — 2026-09-09

Read the TC-055–057 setup correction at dc63f337fcdaee1320d221383eca38599239f62c.
The original shared setup sentence incorrectly required a CheckedPackage for
construction. The corrected cases use the public input constructors and existing
IR identities, as FR-018 already requires. All seven AC mappings, three case
procedures, expected results and six coverage-rule dispositions remain intact.
No ID, relationship, interface or acceptance result changed. Scoped strict
validation records 204/204 grammar-clean documents and no grammar findings in
reviews/data/native-runtime/input-setup-validation.txt. PASS for the correction.
