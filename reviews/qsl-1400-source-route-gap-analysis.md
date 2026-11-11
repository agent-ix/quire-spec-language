---
id: SR-2026111015000000000012
title: "gap-analysis of PR 2678 source route against FR-569-AC-9/AC-10/AC-11, FR-105-AC-18 and FR-383-AC-67"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; PR 2678 tests: qsl-eval/tests/it/source_objects.rs, qsl-eval/tests/it/protocol_slots/acceptance/source_node.rs, qsl-semantics/tests/it/source_objects.rs, qsl-package/src/emit/tests/source_objects.rs"
review_set: subset
---

## Summary

Ticket: QSL-1400. `quoin matrix` does not read this repo's `#[trace]` tags (established on PR 2676), so a manual AC-to-test check replaced it. A mutation ran in a scratch clone with its own target: the `ix://` source-spelling sentinel in `declaration_name` was replaced by returning the name unchanged, and all 13 source-route tests still pass.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1407 S3 is not evaluated. Step 0 and FR-569-AC-11 require S1 to S3 on the source route now. The E-S test evaluates the retry and recovery rows (S1, S2) and the two guards, but not `Refund.recover` with TC-1072's supply that makes `target` `Unavailable`. The `Unavailable` path over a source `Node` is unexercised. | qsl-eval/tests/it/protocol_slots/acceptance/source_node.rs:83 |
| FND-002 | medium | FR-106's rule that an `ix://` name resolves only among model declarations is untested for a name that spells a source declaration. The mutation that drops the `unresolved:` sentinel survives. A snapshot typing an object `ix://source/<pkg>/Order` would then be admitted as the source `Order`. | qsl-semantics/src/model/observation.rs:1698 |
| FND-003 | low | AC-9 S-M "`I` reads only the first" is unasserted. The test only admits the mixed snapshot, and no clause evaluation shows that the source population excludes the model's objects. The probe shows the behaviour is correct (a `forall` over `orders` ignores a model object with total 8). | qsl-eval/tests/it/source_objects.rs:937 |
| FND-004 | low | No test admits or evaluates a snapshot of a source object with a model-typed member. The member-type test only checks compilation. The probe shows it works: `t: M::Total` admits 3, and 99 is refused `invalid-value`. | qsl-semantics/tests/it/source_objects.rs:1103 |

## Scope and evidence

- FR-569-AC-9: covered by three tests, apart from FND-002 and FND-003.
- FR-569-AC-10: V1 to V6 are covered with raw outcome and charge-log equality; V6's witness neutralisation is sound (see SR-2026111015000000000011).
- FR-569-AC-11 (E-S part): `fixture_e_s_checks_and_refuses_at_object_node` checks the span, the code and the cause. The S1 to S3 part is FND-001.
- FR-105-AC-18: `clauses_on_a_source_operation_share_the_declaring_type` plus V2 and V3. Adequate.
- FR-383-AC-67 (now part): E-S checks, its slots match E on value, log and metered units, and S4 refuses. Adequate apart from S3.
- Coder-listed gaps: source record, enum, Decimal, Rational and Quantity members stay refused. This is acceptable per the lead's slice ruling, and they are not findings. The missing red runs for the member-type and E-S tests are not a defect in themselves; FND-002's surviving mutation is the concrete weakness.

## Verdict

The vectors that matter most (V1 to V6, E-S against E) are strong. The `Unavailable` vector and the source-spelled `ix://` rule are the two medium gaps.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Round 1: `an_unavailable_target_halts_the_slot_over_a_source_node_as_it_does_over_a_model` evaluates `Refund.recovery.predicate` with `target` supplied `Unavailable`. It requires `ProtocolSlotEvaluation::Unavailable` over E, the read set [recovered, target] with `activated` unread, and E-S equal to E on read set, charge log and metered units. |
| FND-002 | fixed | Round 1: `an_ix_name_spelling_a_source_declaration_is_refused` refuses `ix://source/<pkg>/Order` as a type and `ix://source/<pkg>/orders` as a population `wrong-role-mapping`, over both S and S-M. Traced, not re-run: under the round-0 mutation (no sentinel) the name resolves to the source record and is admitted, so this assertion fails. |
| FND-003 | fixed | Round 1: `a_source_clause_reads_only_the_source_objects_in_a_mixed_snapshot` checks that `Small` over the source `orders` holds beside a model object of total 8, and that the control `SmallModel` over `M::orders` fails. |
| FND-004 | fixed | Round 1: `a_source_member_of_a_model_type_is_admitted_by_that_types_bound` admits and evaluates `t = 3` and refuses `t = 99` `invalid_runtime_input`/`invalid-value`. |
