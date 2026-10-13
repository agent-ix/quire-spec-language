---
id: SR-7861
title: "QSL-732 PR 794 code review: composite ancestor conformance"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@b4a952dd6cb0e9192a18668a254fea7fcdc42463; Cargo.lock, qsl-semantics/src/check/assemble.rs, qsl-semantics/src/check/check.rs, qsl-semantics/src/check/assemble/tests.rs, qsl-semantics/tests/it/tc235_business_meanings.rs, qsl-semantics/tests/it/tc235_record_event.rs, qsl-eval/tests/it/composite_values.rs"
review_set: subset
---

## Summary

Ticket: QSL-732. PR 794 (base `impl/business-model-meanings`) is the QSL
consumer half of composite supertype conformance. It bumps
`quire-semantic-value` to 4ff3c617 (QSV PR 14, the current QSV `main`), adds
`CompositeGeneralizationCycle` arms, passes record and event `supertypes[]`
into `CompositeDeclaration::with_supertypes`, widens `Typer::coerce` to admit
`(Composite(sub), Composite(sup))` when `composite_conforms` holds, and
narrows the `admit_types_with` consequence rule. This review covers code
review plus the Rust lane. No build was run (reviewer-only brief).

Governing text: QSpec FR-208 "Generalization", record value type: "A value of
the subtype is a value of each ancestor wherever a field, parameter or result
names the ancestor. Equality compares the most-specific type and every
effective field, so a subtype value never equals a value of a proper
ancestor." QSL FR-056-AC-18 restates this for TC-235 M17 run 1. QSV FR-113
item 4 widens only `TypeEnvironment::admits`; the kernel's
`ValueType::admits` stays exact-key.

## Verdict

FAIL. The assembly wiring, the cycle arms and the `admit_types_with` change
are correct. The `admit_types_with` change is tested red-first: before the
PR a dangling key was a silent consequence, so
`a_dangling_composite_supertype_is_refused_not_dropped` fails without it. The
consequence path stays covered by `a_child_of_a_refused_composite_parent_is_a_consequence`.
Source-unit composites are refused before admission, so the narrowing adds
no spurious refusal there.

The `coerce` rule is shallow. It does not recurse into `Option`,
`Collection` or `Reference`, so it adds no covariance for wrappers and no
widening for references. Collection literal elements reach it through
`Expected::Checked`, and evaluation forms those with the unchecked `form`,
so that site is consistent. Replay decodes each composite by its own wire
`declaration` key and admits field slots with `TypeEnvironment::admits`, so
a child keeps its identity at a parent field slot. No spec requirement asks
for a replay test of that case.

The defect is what `coerce` exposes. Check now accepts a child value at
parent-typed parameters, fields and results. Evaluation still assumes that a
value at a `Composite(P)` slot is exactly `P`, in three places:

- positional field reads (FND-001);
- equality operands (FND-002);
- optional-field projection (FND-003).

Each one either gives a wrong value or raises a checked-invariant fault on a
program that checked. The PR's tests cannot see this: their bodies are
`{ 0 }` or `literal(1)`, and the eval fixture's child and parent have one
identical field.

## Examined Scope

```yaml
scope:
  - {id: assemble.rs-model_object_types, path: qsl-semantics/src/check/assemble.rs, role: examined, excerpt: "let declared_supertypes: &[DeclarationKey] = match record { RecordValueType(record) => &record.supertypes, EventType(event) => &event.supertypes, _ => &[] }; ... meaning_handles.get(supertype) ... None => errors.push(broken(&supertype.node)) ... .with_supertypes(composite_supertypes)"}
  - {id: assemble.rs-cause-arms, path: qsl-semantics/src/check/assemble.rs, role: examined, excerpt: "| DeclarationCause::CompositeGeneralizationCycle { .. } => Code::IllTyped / \"type-mismatch\""}
  - {id: assemble.rs-admit_types_with, path: qsl-semantics/src/check/assemble.rs, role: examined, excerpt: "let consequence = set_aside && matches!(&invalid.cause, DeclarationCause::UnknownDeclaration(missing) if set_aside_keys.contains(missing));"}
  - {id: check.rs-coerce, path: qsl-semantics/src/check/check.rs, role: examined, excerpt: "(ValueType::Composite(sub), ValueType::Composite(sup)) if self.scope.types.composite_conforms(*sub, *sup) => { Ok(node) }"}
  - {id: Cargo.lock-qsv, path: Cargo.lock, role: examined, excerpt: "source = \"git+https://github.com/agent-ix/quire-semantic-value?branch=main#4ff3c617556ad6be1cdc424da31ce9018ed6864b\""}
  - {id: FR-056-AC-18, path: spec/functional/FR-056-admit-domain-package-model-declarations.md, role: context_only, excerpt: "TC-235 M17 run 1 retains TaxedMoney's inherited amount_minor, currency and invariant NonNegative, its own tax_minor, and conformance to Money: its stated value is admitted at Order/total and wherever a parameter or result names Money, yet is unequal to the stated Money value because their most-specific types differ."}
  - {id: QSpec-FR-208-generalization, path: quire-specification spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: context_only, excerpt: "A value of the subtype is a value of each ancestor wherever a field, parameter or result names the ancestor. Equality compares the most-specific type and every effective field, so a subtype value never equals a value of a proper ancestor."}
  - {id: QSV-FR-113, path: quire-semantic-value spec/functional/FR-113-composite-value-conformance.md, role: context_only, excerpt: "TypeEnvironment::admits(ValueType::Composite(sup), value) SHALL accept a composite value declared as sub when sub conforms to sup ... equality conversion remains based on exact composite type identity."}
  - {id: qsl-replay-wire-value, path: qsl-replay/src/result/wire/value.rs, role: examined, excerpt: "FieldValue::Present(value) => types.admits(field.value_type(), value)"}
  - {id: qsl-eval-call-admission, path: qsl-eval/src/value/expression/mod.rs, role: examined, excerpt: "} else if !types.admits(value_type, argument) { return Err(InputRefusal::WrongValueKind { parameter }); }"}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Field reads through a parent-typed operand use the parent's positional index (`check.rs` `field()`, `NodeKind::Field { index }`), and evaluation reads `composite.slots()[index]` from the child value. A child's effective fields are sorted by field `DeclarationKey` (`assemble.rs:862`), so the parent's fields are not a positional prefix of the child's. In the PR's own K2 slice, `OrderPlaced` is `[occurred_at, order_id]` but `ExpressOrderPlaced` is `[ExpressOrderPlaced/promised_by, OrderPlaced/occurred_at, OrderPlaced/order_id]`. So `e.occurred_at` on an `ExpressOrderPlaced` passed at an `M::OrderPlaced` parameter silently returns `promised_by`, and `e.order_id` reads a Timestamp slot as a UUID. `coerce` makes this reachable. | qsl-semantics/src/check/assemble.rs:862; qsl-semantics/src/check/check.rs:918; qsl-semantics/src/check/check.rs:1925-1962; qsl-eval/src/value/expression/evaluate.rs:1376-1388 |
| FND-002 | high | Equality of a child value held at a parent-typed slot faults instead of evaluating unequal, which contradicts FR-056-AC-18 and QSpec FR-208. For `a == b` with both operands statically `M::Money`, `CheckedEquality` schedules `Plan`. At evaluation, QSV `operand_value` checks the exact kernel `source.admits(value)`. With a `TaxedMoney` value bound to `a`, that fails with the checked invariant `EqualityOperandSourceNotAdmitted`. The `Reference` arm of `operand_value` already handles this case for references; composites have no equivalent. | qsl-semantics/src/check/check.rs:918; quire-semantic-value src/declaration.rs:4127-4146 |
| FND-003 | high | Reading an optional parent-typed field whose slot holds a child value faults. Record construction admits the child through `TypeEnvironment::admits`, and `coerce` accepts the initializer. `Machine::project` then rebuilds `Option<Money>` through `OptionValue::present`, whose exact kernel `admits` refuses a `TaxedMoney` payload with the checked invariant `field-option-payload-rejected`. FR-056-AC-21 names optional record fields as `Option<Address>`. | qsl-semantics/src/check/check.rs:918; qsl-eval/src/value/expression/evaluate.rs:1993-1996 |
| FND-004 | medium | Model-operation (dispatch) arguments still refuse a child composite at a parent-typed parameter. They go through `Expected::Argument` and `Frame::Upcast` to `Form::Argument`, which accepts only `actual == required` or conforming references. Function arguments now admit the child through `coerce`. FR-056-AC-18 and QSpec FR-208 say "wherever a parameter ... names the ancestor", so an operation parameter naming `Money` refuses `TaxedMoney` with `ill_typed`. | qsl-semantics/src/check/state_model.rs:221-224; qsl-semantics/src/check/check/typing.rs:1393-1397 |
| FND-005 | medium | The checked IR records no composite supertype. `close_composite` emits a `CompositeType` node with only its name and members, and `coerce` leaves no conversion node, so the emitted graph has a call passing a `TaxedMoney`-typed argument to a `Money`-typed parameter with nothing in the IR to justify it. A downstream consumer that re-validates types from the checked package cannot recover the relation, and two packages that differ only in record supertypes key the same composite nodes. Confidence is low because the reviewer did not trace how IR consumers type a call site. | qsl-semantics/src/check/lowering.rs:2055-2093; qsl-semantics/src/check/check.rs:918 |
| FND-006 | low | The new evaluator test `a_call_admits_a_child_composite_at_its_parent_parameter_only` has no `#[trace(..)]`, although every neighbouring test in the module carries one, so it binds to no criterion (FR-056-AC-18 is the natural one). | qsl-eval/tests/it/composite_values.rs:1009-1012 |
