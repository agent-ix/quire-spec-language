<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-8694 method=gap-analysis lang=rust pr=quire-spec-language#803 reviewed=3156a168b92fa8bb1d68c05d5832d2fa87791ef1 model=claude-opus-5-5 run=cdfd8954-161b-4d9b-8692-50fc93b3c534 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | coverage | The snapshot half of FR-359-AC-9 is not implemented and has no external blocker |
| FND-002 | medium | coverage | The model-operation path of FR-359-AC-8 (TC-1001 steps 1-3) is tested only by a unit test of `Form::Argument` over a hand-built `TypeEnvironment` |
| FND-003 | medium | coverage | The event half of FR-359-AC-3 has no test over a checked or emitted package |
| FND-004 | medium | coverage | No test asserts that "the comparison of a child with a different most-specific type performs no text-leaf comparison" (FR-359-AC-5, TC-999 step 3) |
| FND-005 | low | coverage | The `reaches_event` refusal for an operand that holds an event inside a record field, an `Option` or a collection has no test, and neither has `!=` over events |
| FND-006 | low | test-intent | Value-identity oracles compare `format!("{:?}")` strings instead of typed values |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-359-AC-1
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: 'With `record OrderPlaced { order_id: UUID; occurred_at: Timestamp }`, a child `ExpressOrderPlaced` adding `promised_by: Timestamp`, whose effective fields are ordered `[promised_by, occurred_at, order_id]`, a grandchild `RushOrderPlaced` of `ExpressOrderPlaced` adding `rush_by: Timestamp`, and `f(e: OrderPlaced): Timestamp { e.occurred_at }`, passing an `ExpressOrderPlaced` or a `RushOrderPlaced` evaluates to its `occurred_at` and `e.order_id` to its `order_id`, never `promised_by` or `rush_by`. The same read through a record field, an `Option` payload and a `Sequence` element holding the chil'
- id: FR-359-AC-2
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: When a child's type redefines an inherited field with a narrower record type, a parent-typed read of that field yields the child's redefining slot. When it redefines an `Integer` field as `Int[0, 9]`, a parent-typed read yields the stored integer unchanged, admitted at `Integer`. A read of an inherited field the child does not redefine yields the inherited slot.
- id: FR-359-AC-3
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: 'In the checked package for AC-1, `OrderPlaced`, `ExpressOrderPlaced` and `RushOrderPlaced` are each a `model`/`record_value_type` node with a `ModelOwner` and none is a `composite_type`; a domain event type is a `model`/`event_type` node with a `ModelOwner`, and reads back `Verified` once cross-repo requirement 7 holds. The body of `f` holds one `quire.op.record.project` application whose `field` member is `{declaration: OrderPlaced''s node, name: occurred_at}`; the call to `f` takes its argument by `ref`, with no `convert` or other node between them; no node has a `supertypes` member. The pack'
- id: FR-359-AC-4
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: 'For `a == b` and `a != b` with `a: Money` bound to a `TaxedMoney` value and `b: Money` bound to a `Money` value with the same `amount_minor` and `currency`, `==` is `false` and `!=` is `true`, in either operand order. With a grandchild `GiftTaxedMoney` of `TaxedMoney`, `GiftTaxedMoney` against `Money`, against `TaxedMoney` and against another `GiftTaxedMoney` of equal fields give `false`, `false` and `true`. No checked-invariant refusal is reported. Two `TaxedMoney` values with equal effective fields are equal, and unequal when only `tax_minor` differs.'
- id: FR-359-AC-5
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: 'Equality reaches a child inside another value by the same rule: two `Pair { first: Money; second: Money }` whose `first` fields hold a `TaxedMoney` and a `Money` of equal shared fields are unequal, and equal when both hold equal `TaxedMoney` values; the same holds for two `Option<Money>`, two `Sequence<Money>`, two `Set<Money>`, a tuple position and a union payload. An `==` over two event-typed operands stays refused. The comparison of a child with a different most-specific type performs no text-leaf comparison.'
- id: FR-359-AC-6
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: 'A record `Order { total: Money?; }` built with `total` holding a `TaxedMoney` checks, evaluates and is read back through `total`, `present(total)` and `value(total)` as the same `TaxedMoney` value, with its `tax_minor` and most-specific type intact. No `field-option-payload-rejected` fault is reported. A `Money` value at a `TaxedMoney?` field and a value of an unrelated record at a `Money?` field refuse `ill_typed`/`type-mismatch`.'
- id: FR-359-AC-7
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '`Sequence<Money>`, `Set<Money>`, `Bag<Money>` and `OrderedSet<Money>` literals holding a `TaxedMoney` element admit it and keep its identity through `contains`, iteration and `length`. A `Set<Money>` or `Bag<Money>` holding a `Money` and a `TaxedMoney` of equal shared fields has two elements, ordered ascending by declaration key bytes in either source order, and constructing it reports neither `ValueKindMismatch` nor `CanonicalKeyUnavailable`. An `OrderedSet<Money>` of the same two keeps first-occurrence order and a `Sequence<Money>` keeps supply order, in either source order. A `Sequence<Taxe'
- id: FR-359-AC-8
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: 'Model-operation arguments: an operation parameter `m: Money` called with a `TaxedMoney` argument checks, is emitted, reads back `Verified` through `emit_and_read_v2` and runs, including through an `Upcast` of the operation and a dispatch guard or body binding, and its result of declared type `Money` may be a `TaxedMoney`. A `Money` argument at a `TaxedMoney` parameter and an unrelated record at a `Money` parameter refuse `ill_typed`/`type-mismatch`, naming the parameter. A function argument behaves the same way. The read-back half depends on cross-repo requirement 4.'
- id: FR-359-AC-9
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: 'A replay snapshot whose `Order.total: Money` holds `{"record":{"type":".../TaxedMoney","fields":{...}}}` admits, and so does the same record as the payload of an `Order.discount: Money?` field and as an element of a `Sequence<Money>` field and parameter. A `GiftTaxedMoney` admits at a `Money` slot and at a `TaxedMoney` slot, and a `TaxedMoney` at a `GiftTaxedMoney` slot refuses `wrong-value-kind`. A decode followed by an encode returns the same typed observation and digest, with `type` still naming the value''s own record. A `type` naming an unrelated record, a record that is only an ancestor o'
- id: FR-359-AC-10
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: For a `TaxedMoney` at a `Money` slot, no QSL check, evaluation or replay path reports `EqualityOperandSourceNotAdmitted`, `ValueKindMismatch`, `CanonicalKeyUnavailable` or `field-option-payload-rejected`. A hand-built checked package whose `Option<Money>` payload is an unrelated record still reports `field-option-payload-rejected`, and one whose field read names a field absent from the value's declaration reports `field-slot-unresolved`.
- id: FR-359-AC-11
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: Two units that select the same model under different aliases (`model M = ...` and `model N = ...`) assign every model record the same declaration key, equal to its model node key. A `Set<Money>` holding a `Money` and a `TaxedMoney` orders its elements identically in both, and a `TaxedMoney` built in one unit equals the same value built in the other.
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-359-AC-9
  related_ids:
  - FR-106
  - TC-1002
  path: qsl-semantics/src/model/observation/document.rs
  lines: '2200'
  excerpt: 'A replay snapshot whose `Order.total: Money` holds `{"record":{"type":".../TaxedMoney","fields":{...}}}` admits, and so does the same record as the payload of an `Order.discount: Money?` field and as an element of a `Sequence<Money>` field and parameter. A `GiftTaxedMoney` admits at a `Money` slot and at a `TaxedMoney` slot, and a `TaxedMoney` at a `GiftTaxedMoney` slot refuses `wrong-value-kind`. A decode followed by an encode returns the same typed observation and digest, with `type` still naming the value''s own record. A `type` naming an unrelated record, a record that is only an ancestor o'
  finding: 'The snapshot half of FR-359-AC-9 is not implemented and has no external blocker. The observation reader has no FR-106 `record` value form: `read_raw_value` reads no `record` tag, and `admit_form` has no record arm. So no test covers a snapshot admitting `Order.total: Money` holding a `TaxedMoney`, the `Order.discount: Money?` payload, the `Sequence<Money>` field, `missing-member` for a `TaxedMoney` missing `tax_minor` (with a `Money` admitting without it), or the first-path-in-field-order `invalid_runtime_input`/`wrong-value-kind`. This is in-repo work inside QSL-737''s scope, and the ticket''s own text says it "enumerates ALL of it, no later split". The only binder is the stub in SR-8693 FND-002.'
- fnd: FND-002
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-359-AC-8
  related_ids:
  - TC-1001
  path: qsl-semantics/src/check/state_model.rs
  lines: 453-533
  excerpt: 'Model-operation arguments: an operation parameter `m: Money` called with a `TaxedMoney` argument checks, is emitted, reads back `Verified` through `emit_and_read_v2` and runs, including through an `Upcast` of the operation and a dispatch guard or body binding, and its result of declared type `Money` may be a `TaxedMoney`. A `Money` argument at a `TaxedMoney` parameter and an unrelated record at a `Money` parameter refuse `ill_typed`/`type-mismatch`, naming the parameter. A function argument behaves the same way. The read-back half depends on cross-repo requirement 4.'
  finding: 'The model-operation path of FR-359-AC-8 (TC-1001 steps 1-3) is tested only by a unit test of `Form::Argument` over a hand-built `TypeEnvironment`. The shared fixture declares `Ledger.pay(m: Money): Money`, but no test calls it. Nothing checks, emits, runs or reads back an operation call with a `TaxedMoney` argument, an `Upcast` of the operation, a dispatch guard or body binding, or an operation result of declared type `Money` holding a `TaxedMoney`. Only the function-argument half is covered end to end.'
- fnd: FND-003
  severity: medium
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-359-AC-3
  related_ids:
  - TC-998
  path: qsl-semantics/tests/it/child_composite.rs
  lines: 233-286
  excerpt: 'In the checked package for AC-1, `OrderPlaced`, `ExpressOrderPlaced` and `RushOrderPlaced` are each a `model`/`record_value_type` node with a `ModelOwner` and none is a `composite_type`; a domain event type is a `model`/`event_type` node with a `ModelOwner`, and reads back `Verified` once cross-repo requirement 7 holds. The body of `f` holds one `quire.op.record.project` application whose `field` member is `{declaration: OrderPlaced''s node, name: occurred_at}`; the call to `f` takes its argument by `ref`, with no `convert` or other node between them; no node has a `supertypes` member. The pack'
  finding: The event half of FR-359-AC-3 has no test over a checked or emitted package. `records_and_events_are_model_owned_nodes_without_supertypes` asserts only `record_value_type` nodes, despite its name. The `event_type` form is checked only by the `record_form`/`record_node_key` unit test. TC-998 step 5 requires the package to be emitted with an `OrderPlaced` domain event type beside it, and `document_with_event()` exists, so a node-level assertion is possible today.
- fnd: FND-004
  severity: medium
  confidence: medium
  method: gap-analysis
  check: coverage
  artifact_id: FR-359-AC-5
  related_ids:
  - TC-999
  path: qsl-semantics/tests/support/child_composite.rs
  lines: '159'
  excerpt: 'Equality reaches a child inside another value by the same rule: two `Pair { first: Money; second: Money }` whose `first` fields hold a `TaxedMoney` and a `Money` of equal shared fields are unequal, and equal when both hold equal `TaxedMoney` values; the same holds for two `Option<Money>`, two `Sequence<Money>`, two `Set<Money>`, a tuple position and a union payload. An `==` over two event-typed operands stays refused. The comparison of a child with a different most-specific type performs no text-leaf comparison.'
  finding: 'No test asserts that "the comparison of a child with a different most-specific type performs no text-leaf comparison" (FR-359-AC-5, TC-999 step 3). The fixture''s `Money.currency` is native `Integer`, not TC-999''s `Text[3, 3; nfc]`, so no test fails if the identity check is moved after the leaf comparison. If a text field in a model record is blocked by PR #789, an ignored test should cite that blocker. Otherwise the fixture should use a text field.'
- fnd: FND-005
  severity: low
  confidence: high
  method: gap-analysis
  check: coverage
  artifact_id: FR-359-AC-5
  related_ids:
  - TC-999
  - FR-208
  path: qsl-semantics/src/check/check.rs
  lines: 657-686
  excerpt: 'Equality reaches a child inside another value by the same rule: two `Pair { first: Money; second: Money }` whose `first` fields hold a `TaxedMoney` and a `Money` of equal shared fields are unequal, and equal when both hold equal `TaxedMoney` values; the same holds for two `Option<Money>`, two `Sequence<Money>`, two `Set<Money>`, a tuple position and a union payload. An `==` over two event-typed operands stays refused. The comparison of a child with a different most-specific type performs no text-leaf comparison.'
  finding: The `reaches_event` refusal for an operand that holds an event inside a record field, an `Option` or a collection has no test, and neither has `!=` over events. Only `a = b` over two direct `Shipped` operands is asserted.
- fnd: FND-006
  severity: low
  confidence: medium
  method: gap-analysis
  check: test-intent
  artifact_id: FR-359-AC-11
  related_ids:
  - FR-359-AC-1
  - FR-359-AC-6
  - FR-359-AC-9
  path: qsl-eval/tests/it/child_composite.rs
  lines: 782-786
  excerpt: Two units that select the same model under different aliases (`model M = ...` and `model N = ...`) assign every model record the same declaration key, equal to its model node key. A `Set<Money>` holding a `Money` and a `TaxedMoney` orders its elements identically in both, and a `TaxedMoney` built in one unit equals the same value built in the other.
  finding: Value-identity oracles compare `format!("{:?}")` strings instead of typed values. This covers `shown(a) == shown(b)` in the eval tests, the AC-11 "a `TaxedMoney` built in one unit equals the same value built in the other" check (two independently built values compared by Debug text, not by evaluating `==`), and the wire round-trip test. A Debug format that omits a field would let a wrong value pass.
bindings:
- test_id: a_field_read_at_a_parent_typed_parameter_reads_the_childs_field_by_identity
  ac_id: FR-359-AC-1
  trace: correct
- test_id: a_child_is_read_through_a_record_field_an_option_and_a_sequence_element
  ac_id: FR-359-AC-1
  trace: correct
- test_id: a_child_is_accepted_at_a_parent_typed_function_argument
  ac_id: FR-359-AC-1
  trace: correct
- test_id: a_parent_typed_read_of_a_child_field_is_refused
  ac_id: FR-359-AC-1
  trace: correct
- test_id: a_redefining_child_is_read_through_its_redefining_slot
  ac_id: FR-359-AC-2
  trace: correct
- test_id: records_are_model_owned_nodes_and_the_call_takes_its_argument_by_ref
  ac_id: FR-359-AC-3
  trace: correct
- test_id: a_child_record_argument_reads_back_verified
  ac_id: FR-359-AC-3
  trace: correct
- test_id: a_record_and_an_event_type_are_model_nodes_keyed_by_their_owner
  ac_id: FR-359-AC-3
  trace: correct
- test_id: records_and_events_are_model_owned_nodes_without_supertypes
  ac_id: FR-359-AC-3
  trace: correct
- test_id: a_field_read_names_the_parent_node_and_the_call_adds_no_node
  ac_id: FR-359-AC-3
  trace: correct
- test_id: equality_over_a_child_at_a_parent_typed_slot_is_a_boolean
  ac_id: FR-359-AC-4
  trace: correct
- test_id: equality_over_a_child_and_a_parent_static_type_stays_ill_typed
  ac_id: FR-359-AC-4
  trace: correct
- test_id: equality_reaches_a_child_inside_another_value_by_the_same_rule
  ac_id: FR-359-AC-5
  trace: correct
- test_id: equality_over_event_operands_stays_refused
  ac_id: FR-359-AC-5
  trace: correct
- test_id: an_optional_field_holding_a_child_reads_back_the_same_child
  ac_id: FR-359-AC-6
  trace: correct
- test_id: an_optional_field_admits_a_child_and_refuses_a_parent_or_a_stranger
  ac_id: FR-359-AC-6
  trace: correct
- test_id: collections_of_the_parent_hold_children_and_order_by_declaration_key
  ac_id: FR-359-AC-7
  trace: correct
- test_id: a_collection_or_option_of_the_child_is_refused_at_a_parent_slot
  ac_id: FR-359-AC-7
  trace: correct
- test_id: a_collection_literal_of_the_parent_admits_a_child_element
  ac_id: FR-359-AC-7
  trace: correct
- test_id: a_dispatch_argument_admits_a_conforming_record_only
  ac_id: FR-359-AC-8
  trace: correct
- test_id: a_parent_or_an_unrelated_record_is_refused_at_a_child_typed_argument
  ac_id: FR-359-AC-8
  trace: correct
- test_id: a_child_record_is_admitted_at_an_ancestor_typed_parameter
  ac_id: FR-359-AC-9
  trace: correct
- test_id: a_record_that_does_not_conform_refuses_wrong_value_kind
  ac_id: FR-359-AC-9
  trace: correct
- test_id: a_child_record_is_admitted_as_a_payload_and_as_an_element
  ac_id: FR-359-AC-9
  trace: correct
- test_id: a_child_record_at_an_ancestor_slot_round_trips
  ac_id: FR-359-AC-9
  trace: correct
- test_id: a_child_record_in_a_snapshot_admits_at_a_parent_typed_field
  ac_id: FR-359-AC-9
  trace: correct
- test_id: a_conforming_option_payload_projects_and_an_unrelated_one_is_rejected
  ac_id: FR-359-AC-10
  trace: correct
- test_id: two_aliases_of_one_model_agree_on_keys_order_and_equality
  ac_id: FR-359-AC-11
  trace: correct
- test_id: a_model_record_key_is_its_model_node_key_whatever_the_alias
  ac_id: FR-359-AC-11
  trace: correct
```

+++
