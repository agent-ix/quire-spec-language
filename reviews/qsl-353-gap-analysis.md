---
id: SR-939
title: "QSL-353 gap analysis of PR 553, FR-093-AC-19 and TC-416 step 11"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6d5e539395f42434e641fb9854eee4b94140931a; qsl-package/src/emit/tests/admission_corpus.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/lowering/model.rs, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-353. Manual check from AC to test to code, plus the ticket's done criteria.

- FR-093-AC-19 is traced by `#[trace("FR-093-AC-19", "TC-416")]` on
  `every_emitted_node_family_is_admitted_at_its_package_id`. TC-416 step 11 and its
  expected result describe what the test does. The tests.md row lists AC-19.
- The test reuses the existing fixtures: helpers are referenced, and the ConfigVersion
  unit is shared by `#[path]`, not copied. There are two new fixture units, for
  set/bag/ordered_set plus quantify/predicate, and for the STD-129 equality.
- Expected-refused rows are limited to the two the ticket allows (STD-129 and the
  undeclared unit). Any other refusal or omission fails the test.
- Done criterion "one row per node family QSL can emit": not met (FND-001).
- Done criterion "fails when a new family is emitted without a row": met only for
  families the listed fixtures emit (SR-938 FND-001).

I measured which families the lowering can write but no row covers. I grepped the
literal `semantic_form` strings in `qsl-semantics/src/check/lowering*` and compared
them with the 45 rows. Six kinds have no row:
- `scalar_type/decimal` and `bounded_domain/decimal_range` (lowering.rs:1680-1689,
  `ValueType::Decimal`)
- `scalar_type/float32` (lowering.rs:1701-1703, `IeeeWidth::Binary32`)
- `bounded_domain/model_population` (lowering/model.rs:396)
- `model/systems_interface` (lowering/model.rs:237)
- `relation/relationship` (lowering/model.rs:239)

The coder lists the same six as uncovered.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Six node families that QSL's lowering writes have no row and no fixture: decimal, decimal_range, float32, model_population, systems_interface and relation/relationship. QSL-353's done criterion is one row per node family QSL can emit, and any refusal other than the two known gaps is a bug to file. For these six, whether IR admits them is not measured at all, so the corpus cannot surface the bugs it exists to find. Add a fixture and a row for each, and file any refusal it shows. | qsl-semantics/src/check/lowering.rs:1680-1703; qsl-semantics/src/check/lowering/model.rs:237-239,396; qsl-package/src/emit/tests/admission_corpus.rs:197 |

## Verdict

FR-093-AC-19 as written is backed by the test, and the trace and tests.md row are
correct. The ticket's done criteria are not met: six emittable families are uncovered.
Not mergeable until FND-001 is fixed.
