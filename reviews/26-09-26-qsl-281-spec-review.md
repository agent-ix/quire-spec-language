---
id: SR-682
title: "Spec review of kernel ForeignReference record rows"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2fc99d9d8457bc5e838a47842f33a5e5ba053c8f; spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/test-cases/TC-452-spine-run-entry-lives-in-qsl-replay.md; spec/test-cases/TC-428-a-refusal-record-carries-code-category-locus-and-fields.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-452
    type: reviews
---
## Summary

Ticket: QSL-281. PR: quire-spec-language#479 at 2fc99d9d. Base checklist over
the three changed spec files, plus TC-428, whose test the PR extends.

Clean: the FR-096 diff touches only its Status section (lines 346-348). The
key table, Behavior and ACs are unchanged, as the core lane requires. FR-100's
counts are right: two records, then the "remaining ten" no-record causes (8 +
2 IEEE). TC-452's "other eleven" kernel refusals are right (13 variants less
`CardinalityOutOfBound` and `ForeignReference`, `CheckedInvariant` included).
FR-100's new row, TC-452 step 4's oracle and the code agree on code
`foreign_reference`, cause `foreign-universe`, keys `required`/`supplied` and
lowercase hex.

The gap: FR-096's normative key table has no kernel `ForeignReference` row.

## Verdict

**Changes requested.** One medium finding (it needs the QSL core lane, which
owns FR-096); the rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-096's key table has no row for kernel `Refusal::ForeignReference`. Its only kernel row is `CardinalityOutOfBound`, and FR-096 says "A cause with no row here has no fields to give ... no record is built from it". The PR builds a kernel record by borrowing the `ModelQueryRefusal` row, which defines `required` as "the binding's" universe, a notion an equality or membership comparison lacks. Which operand is `required` is decided only in a code doc comment (quire-exact/src/outcome.rs:145-150). Failure scenario: an independent implementation (or TC-428 step 3, "fields hold exactly the keys the table lists") follows FR-096 and builds no kernel record, contradicting FR-100 and TC-452. Or it picks the other operand order, and nothing normative says either is wrong. Fix (QSL core lane): add a kernel `Refusal::ForeignReference` row to FR-096's key table that defines `required`/`supplied` for equality and membership (see SR-680 FND-001 on membership order). | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:182-190, 192-195 |
| FND-002 | low | FR-096's new Status sentence says the record is now built, but it sits in the "Not backed:" list as a continuation of the `CheckedInvariant` bullet. An implemented fact therefore reads as a not-backed item. It belongs in its own "Implemented under QSL-281" group. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:334-348 |
| FND-003 | low | TC-428's Status is stale. The PR adds a `#[trace("TC-428", "FR-096-AC-8")]` test that backs step 4 for `ForeignReference`, but TC-428 still names only `CardinalityOutOfBound` as backed for step 4. | spec/test-cases/TC-428-a-refusal-record-carries-code-category-locus-and-fields.md:43-51; qsl-eval/tests/it/model_reference_queries.rs:3595-3633 |
| FND-004 | low | FR-100's new row says `required` and `supplied` are "each the operand's universe" but not which operand fills which key. The FR-100 text cannot tell a swapped record from a correct one. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:156-160 |
| FND-005 | low | Round 1, new at 6c213eb2. The new FR-096 kernel row says `supplied` is "the value tested against it, as lowercase hex". The key is that value's universe, not the value; FR-100 says "the probed candidate's" universe. Failure scenario: a reader renders the tested reference or value itself, not its universe id. Fix: "the universe of the value tested against it". | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:191 |

## Dispositions

Round 1, re-checked at 6c213eb2. The FR-096 diff against main is the new
key-table row plus the Status group only (the old "Not backed" sentence is
removed). The row meets the owner's conditions: the operand rule (required =
universe in force: equality's left, or membership's collection or kept member),
lowercase hex like the family row, and agreement with FR-100, apart from the
FND-005 wording.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 6c213eb2: kernel `Refusal::ForeignReference` row added to FR-096's key table, approved by the FR-096 owner (qsl-lead2) as relayed by the team leader |
| FND-002 | fixed | 7f4bd626: the Status sentence moved into its own "Implemented under QSL-281" group |
| FND-003 | fixed | 7f4bd626: TC-428 Status names `ForeignReference` as backed for step 4 |
| FND-004 | fixed | 7f4bd626: FR-100 states required = universe in force and supplied = the tested one, for both raise sites |
