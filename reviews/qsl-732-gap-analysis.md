---
id: SR-7862
title: "QSL-732 PR 794 gap analysis: FR-056-AC-18 composite conformance"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b4a952dd6cb0e9192a18668a254fea7fcdc42463; FR-056-AC-18; qsl-semantics/tests/it/tc235_business_meanings.rs, qsl-semantics/tests/it/tc235_record_event.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-eval/tests/it/composite_values.rs"
review_set: subset
---

## Summary

Plan completion: not assessed

Ticket: QSL-732. This analysis checks FR-056-AC-18 (TC-235 M17 run 1) against
the tests the PR adds or un-ignores. It also spot-checks whether each test
would fail without its production change. No build was run.

Red-first spot check, done by reading the code:

- `tc235_record_subtype_value_conforms_to_parent_typed_parameter`: red
  without `coerce` or the supertype wiring.
- `qsl_732_composite_subtype_is_admitted_at_its_parent_slot_only` and
  `qsl_732_event_subtype_conforms_to_its_parent_event`: red without the
  wiring, because they assert `supertypes() == [money]`.
- `a_call_admits_a_child_composite_at_its_parent_parameter_only`: red
  without the QSV bump.
- `a_dangling_composite_supertype_is_refused_not_dropped`: red without the
  `admit_types_with` change.
- `tc235_record_parent_value_is_refused_at_child_typed_parameter` and
  `a_child_of_a_refused_composite_parent_is_a_consequence`: pass before and
  after. They are regression guards against widening and against
  over-reporting, which is acceptable.

The assertions are typed: they check `CheckCause`, `DeclarationCause`,
catalog code and `InputRefusal`.

## Verdict

CONDITIONAL. The parameter-admission and reverse/unrelated-refusal parts of
FR-056-AC-18 are backed. These parts are not:

- the result position;
- admission at `Order/total`;
- "unequal to the stated Money value".

Every positive body is `{ 0 }` or `literal(1)` and never reads the admitted
value. That is why the evaluation defects in SR-7861 FND-001..003 pass
unseen.

On (c): the un-ignored test is narrowed to Money/TaxedMoney because the full
K2 slice does not check. The narrowing is not this PR hiding a new bug. The
slice keeps `Order.order_id: UUID` and `OrderPlaced`/`ExpressOrderPlaced`
Timestamp/UUID fields. The base lowering refuses `String`, `Uuid` and
`Timestamp` type nodes as `ill_typed`/`type-mismatch` (`lowering.rs`
`ValueType::String | ValueType::Uuid | ValueType::Timestamp => Err(refuse(..TypeMismatch))`).
PR 789 (QSL-723) replaces exactly that arm. Those two refusals are most
likely these lowering refusals, so PR 789 covers them. Confidence is medium:
the reviewer read the code and did not run the check. The test comment
blames the "object, process and persistence-interface members" and names no
owning ticket (FND-002).

## Examined Scope

```yaml
scope:
  - {id: FR-056-AC-18, path: spec/functional/FR-056-admit-domain-package-model-declarations.md, role: examined, excerpt: "TC-235 M17 run 1 retains TaxedMoney's inherited amount_minor, currency and invariant NonNegative, its own tax_minor, and conformance to Money: its stated value is admitted at Order/total and wherever a parameter or result names Money, yet is unequal to the stated Money value because their most-specific types differ. M17 runs 2 and 3 retain the exact wrong-meaning and specialization-cycle refusals, including the cycle list, rather than object-reference or Boolean substitutes."}
  - {id: FR-056-AC-21, path: spec/functional/FR-056-admit-domain-package-model-declarations.md, role: context_only, excerpt: "The type environment gives Account.address the record value type, an optional Address field Option<Address>, and an ordered non-unique [0, 3] field Sequence<Address>[0, 3], never Reference<Address>; an operation parameter or result naming Address has the same value type."}
  - {id: QSpec-FR-208-generalization, path: quire-specification spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: context_only, excerpt: "A value of the subtype is a value of each ancestor wherever a field, parameter or result names the ancestor. Equality compares the most-specific type and every effective field, so a subtype value never equals a value of a proper ancestor."}
bindings:
  - {test_id: tc235_record_subtype_value_conforms_to_parent_typed_parameter, ac_id: FR-056-AC-18, trace: correct}
  - {test_id: tc235_record_parent_value_is_refused_at_child_typed_parameter, ac_id: FR-056-AC-18, trace: correct}
  - {test_id: qsl_732_composite_subtype_is_admitted_at_its_parent_slot_only, ac_id: FR-056-AC-18, trace: correct}
  - {test_id: qsl_732_event_subtype_conforms_to_its_parent_event, ac_id: FR-056-AC-18, trace: correct}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-056-AC-18 is only partly backed. No test admits a `TaxedMoney` value where a result names `Money`, admits it at `Order/total`, reads a field of it through a `Money`-typed slot, or evaluates it against a `Money` value and asserts unequal. The positive bodies (`{ 0 }`, `literal(1)`) never use the admitted value, and the evaluator fixture's child and parent share one identical field. So these tests cannot detect the wrong-slot read, the equality fault or the option-projection fault (SR-7861 FND-001..003). | qsl-semantics/tests/it/tc235_business_meanings.rs:1178-1190; qsl-eval/tests/it/composite_values.rs:1009-1059; qsl-semantics/tests/it/tc235_record_event.rs:1170-1245 |
| FND-002 | low | `money_slice_document` narrows the un-ignored AC-18 test to Money/TaxedMoney. Its comment blames the K2 "object, process and persistence-interface members" and cites no owning ticket. The likely cause is the base lowering's `String`/`Uuid`/`Timestamp` `type-mismatch`, which QSL-723 (PR 789) replaces. Without a pointer, nobody will restore the full slice, and the `Order/total` half of AC-18 stays out of the checked test. | qsl-semantics/tests/it/tc235_business_meanings.rs:1160-1175 |
