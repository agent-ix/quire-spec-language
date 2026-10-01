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

## Dispositions

Round 1, reviewed at c29e7b3397a0cb9bcec144a5ec4db210c170ff10. The coder's make ci log on c29e7b33 reports `exit=0`; I did not re-run it.

Five of the six families now have admitted rows: decimal and decimal_range (`FormsUnit`), float32 (`Float32Add`), model_population (`ModelPopulation`) and systems_interface (`SystemsInterface`). Two more families, record_value and tuple_value, also have admitted rows (`FormsUnit`).

I verified the claim that relation/relationship is not emitted by QSL:
- `model_node` (lowering/model.rs:275) is the only production caller of `record_form`. It is reached from `object_node` (references, clause contexts, populations, anchors), `frame_field` (the owner of a field member) and `frame_objects` (creates and deletes).
- A `Reference<T>` cannot name a relationship. assemble.rs:527 says relationship records name no type environment entry.
- Intake refuses a relationship in `modifies` as unsupported (`resolve_frame` passes `Some(FrameEntryKind::Relationship)`, intake.rs:2425). It admits only object types in `creates` and `deletes`.
- The only call that reaches the relationship arm is the unit test at lowering/model/tests.rs:585.

The `record_form` relationship arm is unreachable from production today. I do not record that as a finding. FR-094-CON-1 requires one arm per record kind, and the arm is the correct FR-322 mapping, not ceremony. It is also outside this PR's diff. If a frame ever admits a relationship, the corpus fails until a row is added, because the kind is not a row family.

The coder's side note on QSL's `Operator` arms `temporal`, `protocol_control`, `state_transition` and `claim` is out of scope for this PR. The enum mirrors the schema's closed application-operator vocabulary. No production lowering builds those arms, and the PR does not touch them. If one were built, `families()` would panic on a form IR does not have, so the corpus already guards it.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | c29e7b33: decimal, decimal_range, float32, model_population and systems_interface have admitted rows. relation/relationship is classified as not emitted, with a verified reason (no production path reaches it) |
