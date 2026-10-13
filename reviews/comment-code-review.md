<!-- reviewer repo=agent-ix/quire-spec-language visibility=public quoin=0.28.3 module=spec-artifacts-process@b82e42e893612b4b4ee94127714b4577ee893502 id=SR-8693 method=code-review lang=rust pr=quire-spec-language#803 reviewed=3156a168b92fa8bb1d68c05d5832d2fa87791ef1 model=claude-opus-5-5 run=cdfd8954-161b-4d9b-8692-50fc93b3c534 -->

| FND | Severity | Check | Summary |
| --- | --- | --- | --- |
| FND-001 | high | code-bug | The committed head does not compile |
| FND-002 | medium | test-intent | The ignored test `a_child_record_in_a_snapshot_admits_at_a_parent_typed_field` is a stub tagged `#[trace("TC-1002", "FR-359-AC-9")]` |
| FND-003 | medium | coverage | `a_child_record_argument_reads_back_verified` stays `#[ignore]` with the reason "until quire-contract-ir lands IR-770 and FR-347" |
| FND-004 | medium | test-intent | The refusal oracles assert the cause and nothing else |
| FND-005 | low | other | Every field read during evaluation allocates and scans |
| FND-006 | low | other | `ModelRecords::rename` and `ModelRecords::declare_event` are `pub` but have no caller outside qsl-semantics, where only `assemble.rs` calls them |
| FND-007 | low | code-bug | `close_composite` finds a model record's occurrence root by splitting the display name `alias::Artifact` with `split_once("::")`, which recovers identity from a formatted string |

+++ [reviewer data]

```yaml
clean: false
scope:
- id: FR-359 item 1
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**A child keeps its identity.** The value at a `P` slot is the child value itself, tagged with `S`''s declaration key. Admission SHALL NOT retag it, copy it into a `P` value, drop its extra fields or insert a conversion step. This is the only reading under which "a subtype value never equals a value of a proper ancestor" can hold: a projected copy would equal the stated `P`.'
- id: FR-359 item 2
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**A slot is read by field identity, never by position.** A checked record field read SHALL carry the field''s identity, its declaring record''s declaration key and its name, and SHALL NOT carry a position. At evaluation the field''s slot in the value is found in the value''s own declaration by that identity. When the value''s type redefines the field (FR-082), the slot is the effective field that redefines it, transitively. The read returns the stored value as it is, with no conversion; the value is admitted at the read''s checked result type, which is the type the declaring record gives the field, '
- id: FR-359 item 3
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**Records and events are emitted as model-owned nodes.** The checked IR SHALL carry a domain package''s record value type as a `model`/`record_value_type` node with its `ModelOwner` (FR-094, FR-103, FR-322-AC-63), and its event type as a `model`/`event_type` node with its `ModelOwner` (cross-repo requirement 7), and SHALL NOT emit either as a `composite_type`/`record` node with a source owner. Today QSL''s `close_composite` emits it as the latter and `record_form` maps both kinds to no node, so the checked package names no relation to the selection that declares `supertypes`; the emitter is chan'
- id: FR-359 item 4
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**Equality is decided by identity first.** FR-208 gives an event type no equality rule, so this item applies to record value types only; an event value is read, stored, passed and encoded as the other items state, and `==` over event types stays refused. For two operands whose static type is the record value type `P`, `==` SHALL be false and `!=` SHALL be true when their most-specific declaration keys differ, at any depth of specialization. When the keys are equal the operands are equal exactly when every effective field is equal, each field under this same rule. The result is a Boolean, never'
- id: FR-359 item 5
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**Payloads are admitted by conformance at every depth.** An `Option<P>` payload, a `Sequence<P>`, `Set<P>`, `Bag<P>` or `OrderedSet<P>` element, a tuple position, a union member payload and a record field of type `P` SHALL admit a child. Constructing, projecting (`present`, `value`, field read), storing and re-reading such a value SHALL preserve the child''s identity. A `Set<P>`, `Bag<P>` or `OrderedSet<P>` holding a `P` value and a child with the same shared fields holds two distinct elements, because they are unequal (item 4). For `Set` and `Bag` only, the canonical-key order between two valu'
- id: FR-359 item 6
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**Arguments and results.** A function argument, a model-operation argument (including an `Upcast` of an operation parameter and a dispatch guard or body binding), a result and a `modifies` field grant SHALL admit a child at a `P`-typed parameter or result by the same conformance rule. A `P` value never conforms to `S`, an intermediate ancestor never conforms to a descendant, and an unrelated record never conforms to `P`. A compiled program passing a child to an operation parameter is read back by the IR reader (`emit_and_read_v2`), so it also depends on the reader accepting that call (cross-re'
- id: FR-359 item 7
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**Wire and replay.** Replay SHALL decode every record by the declaration key its own `type` member names (FR-106-AC-14) and SHALL read its fields by name against that declaration''s effective fields, not against the slot''s declared type. It SHALL then admit the decoded value at its slot with the selected-model `TypeEnvironment::admits`, which accepts exactly the values whose most-specific type conforms to the slot''s type. The same holds for an `Option` payload and a collection element. Encoding SHALL write the value''s own most-specific `type` and its own effective fields, and a decode followed '
- id: FR-359 item 8
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**Kernel checks that stop being exact-key.** Every check between a conforming value and its slot SHALL decide conformance with the `TypeEnvironment`, and a refusal there SHALL mean non-conformance. The outcome is that no conforming payload, element or operand reaches a checked-invariant fault. The kernel `ValueType::admits` stays the exact, environment-less predicate and SHALL NOT be called to admit a value into a slot that can hold a child. The mechanisms, one per owner, are the cross-repo requirements below.'
- id: FR-359 item 9
  path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md
  role: examined
  excerpt: '**A model record''s runtime declaration key is its model node key.** The declaration key under which a model record or event type is declared in the type environment, and which identifies a value''s most-specific type for equality and for the item 5 order, SHALL be the model-owned node key of FR-322 "Model-owned members" step 2, a function of the selected document''s identity and the node alone. It SHALL NOT be derived from the selecting unit''s source owner or from the alias a unit gives the model (`model M = ...`); QSL''s `declared_type_handle(owner, "{alias}::{artifact}")` derivation does that t'
- id: qsl-semantics/src/check/check.rs
  path: qsl-semantics/src/check/check.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/assemble.rs
  path: qsl-semantics/src/check/assemble.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/ir.rs
  path: qsl-semantics/src/check/ir.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/facts.rs
  path: qsl-semantics/src/check/facts.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/field_refinement.rs
  path: qsl-semantics/src/check/field_refinement.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/lowering.rs
  path: qsl-semantics/src/check/lowering.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/lowering/model.rs
  path: qsl-semantics/src/check/lowering/model.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/state_model.rs
  path: qsl-semantics/src/check/state_model.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/check/mod.rs
  path: qsl-semantics/src/check/mod.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-semantics/src/model/observation/document.rs
  path: qsl-semantics/src/model/observation/document.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-eval/src/value/expression/evaluate.rs
  path: qsl-eval/src/value/expression/evaluate.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-eval/src/value/expression/evaluate/invariants.rs
  path: qsl-eval/src/value/expression/evaluate/invariants.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-replay/src/execute/argument.rs
  path: qsl-replay/src/execute/argument.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: qsl-replay/src/result/wire/value.rs
  path: qsl-replay/src/result/wire/value.rs
  role: examined
  excerpt: 'changed by PR #803'
- id: Cargo.lock
  path: Cargo.lock
  role: examined
  excerpt: 'changed by PR #803'
- id: spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md
  path: spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md
  role: examined
  excerpt: 'changed by PR #803'
findings:
- fnd: FND-001
  severity: high
  confidence: high
  method: code-review
  check: code-bug
  artifact_id: qsl-eval/src/value/expression/evaluate.rs
  related_ids:
  - FR-359
  path: qsl-eval/src/value/expression/evaluate.rs
  lines: 1364-1369
  excerpt: "                    _ => Value::Boolean(outcome_into_stop(checked.evaluate(\n                        self.scope.types(),\n                        &left,\n                        &right,\n                        self.meter,\n                    ))?),"
  finding: 'The committed head does not compile. evaluate.rs calls `checked.evaluate(self.scope.types(), &left, &right, meter)` and `operand_value(self.scope.types(), operand, ...)`, and the eval tests call `evaluate(&env, ...)`. Cargo.lock pins quire-semantic-value `4ff3c61`, where `CheckedEquality::evaluate(&self, left, right, meter)` and `operand_value(operand, value, units, meter)` take no environment. That API is QSV PR #16, which is still OPEN. Merging #803 as it stands turns its base red. QSV #16 must merge first and the lock bump must land in this PR.'
- fnd: FND-002
  severity: medium
  confidence: high
  method: code-review
  check: test-intent
  artifact_id: FR-359-AC-9
  related_ids:
  - TC-1002
  - FR-106
  path: qsl-semantics/src/model/observation/document.rs
  lines: 2188-2205
  excerpt: 'A replay snapshot whose `Order.total: Money` holds `{"record":{"type":".../TaxedMoney","fields":{...}}}` admits, and so does the same record as the payload of an `Order.discount: Money?` field and as an element of a `Sequence<Money>` field and parameter. A `GiftTaxedMoney` admits at a `Money` slot and at a `TaxedMoney` slot, and a `TaxedMoney` at a `GiftTaxedMoney` slot refuses `wrong-value-kind`. A decode followed by an encode returns the same typed observation and digest, with `type` still naming the value''s own record. A `type` naming an unrelated record, a record that is only an ancestor o'
  finding: 'The ignored test `a_child_record_in_a_snapshot_admits_at_a_parent_typed_field` is a stub tagged `#[trace("TC-1002", "FR-359-AC-9")]`. It parses a JSON literal, asserts the parse succeeded and then unconditionally `panic!`s, and it calls no production code. Its doc comment says it "asserts today''s refusal so it fails once the reader lands". That is false: it fails whatever the reader does. As written it inflates AC-9 coverage and cannot become a passing test without being rewritten.'
- fnd: FND-003
  severity: medium
  confidence: high
  method: code-review
  check: coverage
  artifact_id: FR-359-AC-3
  related_ids:
  - FR-359-AC-8
  - TC-998
  - TC-1001
  path: qsl-package/src/emit/tests/child_composite.rs
  lines: 113-116
  excerpt: 'In the checked package for AC-1, `OrderPlaced`, `ExpressOrderPlaced` and `RushOrderPlaced` are each a `model`/`record_value_type` node with a `ModelOwner` and none is a `composite_type`; a domain event type is a `model`/`event_type` node with a `ModelOwner`, and reads back `Verified` once cross-repo requirement 7 holds. The body of `f` holds one `quire.op.record.project` application whose `field` member is `{declaration: OrderPlaced''s node, name: occurred_at}`; the call to `f` takes its argument by `ref`, with no `convert` or other node between them; no node has a `supertypes` member. The pack'
  finding: '`a_child_record_argument_reads_back_verified` stays `#[ignore]` with the reason "until quire-contract-ir lands IR-770 and FR-347". Both of those are done: quire-contract-ir PR #376 is MERGED (d52d5f1) and IR-770 is Done. The lock pins quire-contract-ir `eb007b2`, which predates #376. The cited blocker has cleared, so this PR should bump the lock and un-ignore the test. Until then the AC-3 and AC-8 read-back halves have no passing test.'
- fnd: FND-004
  severity: medium
  confidence: high
  method: code-review
  check: test-intent
  artifact_id: FR-359-AC-8
  related_ids:
  - FR-359-AC-6
  - TC-999
  - TC-1000
  - TC-1001
  path: qsl-semantics/tests/it/child_composite.rs
  lines: 75-82
  excerpt: 'Model-operation arguments: an operation parameter `m: Money` called with a `TaxedMoney` argument checks, is emitted, reads back `Verified` through `emit_and_read_v2` and runs, including through an `Upcast` of the operation and a dispatch guard or body binding, and its result of declared type `Money` may be a `TaxedMoney`. A `Money` argument at a `TaxedMoney` parameter and an unrelated record at a `Money` parameter refuse `ill_typed`/`type-mismatch`, naming the parameter. A function argument behaves the same way. The read-back half depends on cross-repo requirement 4.'
  finding: The refusal oracles assert the cause and nothing else. `refused_as_mismatch` passes when every refusal is `TypeMismatch`, wherever it is reported. FR-359-AC-6 requires "naming the field", AC-8 requires "naming the parameter" and TC-999 step 5 requires "at the operands", and no assertion checks any refusal's location or path. A `type-mismatch` raised at the wrong site, such as the callee body or another argument, still passes.
- fnd: FND-005
  severity: low
  confidence: medium
  method: code-review
  check: other
  artifact_id: qsl-semantics/src/check/check.rs
  related_ids:
  - FR-359
  path: qsl-semantics/src/check/check.rs
  lines: 373-381
  excerpt: "    fn own_name<'f>(\n        &'f self,\n        declaration: quire_exact::NodeKey,\n        field: &'f RecordField,\n    ) -> &'f str {\n        self.renamed\n            .get(&(declaration, field.declaration(), field.name().to_owned()))\n            .map_or(field.name(), String::as_str)\n    }"
  finding: 'Every field read during evaluation allocates and scans. `ModelRecords::own_name` builds `field.name().to_owned()` to look up a `(NodeKey, NodeKey, String)` BTreeMap key, even when the rename table is empty, and then `record_slot` scans the field names linearly. Before this PR, `Field` used an O(1) index. Every `e.f` in a hot loop now pays a heap allocation. A fix: short-circuit when `renamed` is empty, and use a borrowed-key lookup.'
- fnd: FND-006
  severity: low
  confidence: high
  method: code-review
  check: other
  artifact_id: qsl-semantics/src/check/check.rs
  related_ids: []
  path: qsl-semantics/src/check/check.rs
  lines: 350-366
  excerpt: "    pub fn rename(\n        &mut self,\n        value: quire_exact::NodeKey,\n        read_at: quire_exact::NodeKey,\n        name: impl Into<String>,\n        own: impl Into<String>,\n    ) {\n        self.renamed\n            .insert((value, read_at, name.into()), own.into());\n    }\n\n    /// Record that `declaration` is an event type.\n    pub fn declare_event(&mut self, declaration: quire_exact::NodeKey) {\n        self.events.insert(declaration);\n    }\n\n    /// Every entry of `other`."
  finding: '`ModelRecords::rename` and `ModelRecords::declare_event` are `pub` but have no caller outside qsl-semantics, where only `assemble.rs` calls them. `pub use check::ModelRecords;` was inserted inside the comment block that documents the test-support re-exports, so the comment now misdescribes it.'
- fnd: FND-007
  severity: low
  confidence: medium
  method: code-review
  check: code-bug
  artifact_id: qsl-semantics/src/check/lowering.rs
  related_ids:
  - FR-359
  path: qsl-semantics/src/check/lowering.rs
  lines: 1998-2006
  excerpt: "            let alias = self\n                .scope\n                .types()\n                .composite(declaration)\n                .and_then(|composite| composite.name().split_once(\"::\"))\n                .map(|(alias, _)| alias.to_owned());\n            if let Some(alias) = alias {\n                self.record(key, OccurrenceRole::Generated, type_declaration(&alias));\n            }"
  finding: '`close_composite` finds a model record''s occurrence root by splitting the display name `alias::Artifact` with `split_once("::")`, which recovers identity from a formatted string. A name without `::` silently records no occurrence root, with no refusal. The alias is known at assembly, so it could be carried with the record''s declaration.'
bindings: []
```

+++
