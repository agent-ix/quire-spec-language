---
id: FR-359
title: "Carry a child record value at an ancestor-typed slot through check, evaluation and replay"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-208
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
  - target: ix://agent-ix/quire-semantic-value/FR-113
    type: depends_on
---
# FR-359: Carry a child record value at an ancestor-typed slot through check, evaluation and replay

## Description

QSpec FR-208 says a value of a record subtype is a value of each ancestor
wherever a field, parameter or result names the ancestor, and that equality
compares the most-specific type and every effective field. FR-056-AC-18 binds
that to TC-235 M17. QSV FR-113 makes `TypeEnvironment::admits` accept a child
composite at an ancestor-typed slot. This requirement states what the rest of
QSL does with that value: how check types it, how the evaluator reads, compares
and stores it, and how replay decodes and encodes it. An event type is a record
value for this purpose (FR-208 gives it the same inheritance); an object is
never one, and a `Reference<T>` keeps its own conformance rule (FR-100,
FR-322 "Reference conformance").

A **slot** is a field, parameter, result, `Option` payload or collection
element whose declared type names a record value type `P`. A **child** is a
value whose most-specific record type `S` is `P` or has `P` among its
ancestors.

## Target design

1. **A child keeps its identity.** The value at a `P` slot is the child value
   itself, tagged with `S`'s declaration key. Admission SHALL NOT retag it,
   copy it into a `P` value, drop its extra fields or insert a conversion
   step. This is the only reading under which "a subtype value never equals a
   value of a proper ancestor" can hold: a projected copy would equal the
   stated `P`.
2. **A slot is read by field identity, never by position.** A checked `Field`
   read of a record SHALL carry the field's identity, its declaring record's
   declaration key and its name, and SHALL NOT carry a position. At
   evaluation the field's slot in the value is found in the value's own
   declaration by that identity. When the value's type redefines the field
   (FR-082), the slot is the effective field that redefines it, transitively;
   its type conforms to the redefined field's type, so the result keeps the
   type check gave it. A child's effective fields are ordered by their own
   keys and are not a positional prefix of the ancestor's, so a position
   computed from `P` names a different field in `S`. A tuple has no
   supertypes in source, so a tuple position stays a position.
3. **The checked IR carries no conversion and no supertypes.** The lowered
   read stays `quire.op.record.project` with `field{declaration: P's node,
   name}` (FR-093), and the lowered call or record keeps its operand `ref`
   unchanged. A model-owned record is a `model`/`record_value_type` node
   whose `ModelOwner` joins to a digest-pinned selection (FR-322 "Model-owned
   members" step 2); that selected document declares the record's
   `supertypes` and FR-322-AC-30 already resolves an inherited field on the
   subtype's node. The checked package therefore already names the relation
   through content it pins. A `supertypes` member on the node would copy the
   selected document into the package and change node identity for no
   behaviour; a conversion node would contradict item 1. The emitter SHALL
   NOT add either, and SHALL emit a model-owned record as FR-103 and
   FR-322-AC-63 state.
4. **Equality is decided by identity first.** For two operands whose static
   type is the record type `P`, `==` is false and `!=` is true when their
   most-specific declaration keys differ. When the keys are equal, the
   operands are equal exactly when every effective field is equal, each field
   under this same rule, so a child held at a parent-typed field of two
   otherwise equal parents compares the same way. The result is a Boolean,
   never a checked-invariant fault: the evaluator SHALL NOT report
   `EqualityOperandSourceNotAdmitted` for a conforming operand. The
   identity check precedes the field and text-leaf comparison, so a leaf list
   derived from the static type (FR-093) is never applied to a value of a
   different most-specific type.
5. **Payloads are admitted by conformance at every depth.** An `Option<P>`
   payload, a `Sequence<P>`, `Set<P>`, `Bag<P>` or `OrderedSet<P>` element and
   a record field of type `P` all admit a child. Constructing, projecting
   (`present`, `value`, field read), storing and re-reading such a value
   preserves the child's identity. A `Set<P>`, `Bag<P>` or `OrderedSet<P>`
   holding a `P` value and a child with the same shared fields holds two
   distinct elements, because they are unequal (item 4); a collection's
   canonical key and order begin from the most-specific declaration key. A
   type is not covariant: an expression of type `Sequence<S>` or `Option<S>`
   is not accepted at a `Sequence<P>` or `Option<P>` slot, because a
   collection's type identity includes its element type (FR-334). A child is
   admitted into the slot as a value, as `sequence[child]` at `Sequence<P>`
   does.
6. **Arguments and results.** A function argument, a model-operation argument
   (including an `Upcast` of an operation parameter and a dispatch guard or
   body binding), a result and a `modifies` field grant admit a child at a
   `P`-typed parameter or result by the same conformance rule. A `P` value
   never conforms to `S`, and an unrelated record never conforms to `P`.
7. **Wire and replay.** Replay decodes every record by the declaration key its
   own `type` member names (FR-106-AC-14) and reads its fields by name against
   that declaration's effective fields, not against the slot's declared type.
   It then admits the decoded value at its slot with the selected-model
   `TypeEnvironment::admits`, which accepts exactly the values whose
   most-specific type conforms to the slot's type. The same holds for an
   `Option` payload and a collection element. Encoding writes the value's own
   most-specific `type` and its own effective fields. A decode followed by an
   encode yields the same typed observation and digest.
8. **Kernel checks that stop being exact-key.** Every check that stands between
   a conforming value and its slot SHALL decide conformance with the
   `TypeEnvironment`, and a refusal there SHALL mean non-conformance:
   - QSV `TypeEnvironment::admits` already does for a composite slot
     (QSV FR-113); it SHALL do so at every nested depth.
   - The construction check of `OptionValue::present` and of every collection
     constructor (quire-exact) SHALL admit a conforming payload or element when
     given the environment. Kernel `ValueType::admits` stays the exact,
     environment-less predicate and SHALL NOT be called to admit a value into
     a slot that can hold a child.
   - QSV `operand_value` SHALL check a composite operand against its source
     type by conformance. It already checks a `Reference<T>` operand by kind
     only, because the checker has proved the static types related; a
     composite operand SHALL be treated the same way, with the identity rule
     of item 4 deciding the result.
   Changes to quire-exact and QSV ride with the QSL slice that implements this
   requirement; the owning specifications are QSV FR-113 and the quire-exact
   kernel requirements for composite and `Option` admission. Their
   amendments are separate spec changes in those repositories.
9. **Static typing is unchanged elsewhere.** Equality still requires both
   operands to have the same static type. A child is not accepted where a
   different unrelated type is expected.

## Inputs

- The effective view and ancestor graph FR-081 and FR-082 produce.
- A checked expression whose operand, argument or element has a record type
  that conforms to the slot's declared record type.
- A replay request whose snapshot, parameter or result holds a record.

## Outputs

- A checked package whose nodes are those of a program with no subtyping, and
  whose field reads name their field.
- An evaluated value that retains its most-specific type, or a Boolean for an
  equality, or a typed refusal at the slot that does not conform.

## Behavior

- The refusal for a value whose most-specific type does not conform to the
  slot is `ill_typed`/`type-mismatch` at check, `wrong-value-kind` at replay
  admission (FR-106-AC-15) and `WrongValueKind` at call admission (FR-100).
  Neither the evaluator nor the kernel reports these as a checked-invariant
  fault.
- The checked-invariant causes `field-option-payload-rejected` and
  `field-slot-unresolved` (FR-090) remain, for a payload that does not conform
  and a value whose declaration has no such field. A conforming value reaches
  neither.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-359-AC-1 | With `record OrderPlaced { order_id: UUID; occurred_at: Timestamp }`, a child `ExpressOrderPlaced` adding `promised_by: Timestamp`, whose effective fields are ordered `[promised_by, occurred_at, order_id]`, and `f(e: OrderPlaced): Timestamp { e.occurred_at }`, passing an `ExpressOrderPlaced` evaluates to its `occurred_at` and `e.order_id` to its `order_id`, never `promised_by`. The same read through a record field, an `Option` payload and a `Sequence` element holding the child yields the same fields. The parent-typed read of `promised_by` refuses `ill_typed`/`operator-ineligible` at check, naming the field. | Test (TC-998) |
| FR-359-AC-2 | When a child's type redefines an inherited field with a narrower type, a parent-typed read of that field yields the child's redefining slot, a read of an inherited field the child does not redefine yields the inherited slot, and the result has the type check gave it. | Test (TC-998) |
| FR-359-AC-3 | The checked package for AC-1 holds, for the parent-typed read, one `quire.op.record.project` application whose `field` member is `{declaration: OrderPlaced's node, name: occurred_at}`, no `convert` or other node between the argument and the call, and no `supertypes` member on any node. It reads back `Verified`. The package for the same program with `ExpressOrderPlaced` removed has the same read and call nodes with the same node ids. | Test (TC-998) |
| FR-359-AC-4 | For `a == b` and `a != b` with `a: Money` bound to a `TaxedMoney` value and `b: Money` bound to a `Money` value with the same `amount_minor` and `currency`, `==` is `false` and `!=` is `true`, in either operand order. No `CheckedInvariant` refusal is reported. Two `TaxedMoney` values at `Money` slots with equal effective fields are equal, and are unequal when only `tax_minor` differs. | Test (TC-999) |
| FR-359-AC-5 | With `record Pair { first: Money; second: Money }`, two `Pair` values whose `first` fields hold a `TaxedMoney` and a `Money` of equal shared fields are unequal; two whose `first` fields hold equal `TaxedMoney` values are equal. The comparison of a child with a different most-specific type performs no text-leaf comparison. | Test (TC-999) |
| FR-359-AC-6 | A record `Order { total: Money?; }` built with `total` holding a `TaxedMoney` checks, evaluates and is read back through `total`, `present(total)` and `value(total)` as the same `TaxedMoney` value, with its `tax_minor` and most-specific type intact. No `field-option-payload-rejected` fault is reported. A `Money` value at a `TaxedMoney?` field and a value of an unrelated record at a `Money?` field refuse `ill_typed`/`type-mismatch`. | Test (TC-1000) |
| FR-359-AC-7 | `Sequence<Money>`, `Set<Money>`, `Bag<Money>` and `OrderedSet<Money>` literals holding a `TaxedMoney` element admit it and keep its identity through `contains`, iteration and `length`. A `Set<Money>` holding a `Money` and a `TaxedMoney` of equal shared fields has two elements. Reordering the source elements leaves the canonical key unchanged. A `Sequence<TaxedMoney>` expression at a `Sequence<Money>` parameter, and an `Option<TaxedMoney>` expression at an `Option<Money>` parameter, refuse `ill_typed`/`type-mismatch`. | Test (TC-1000) |
| FR-359-AC-8 | Model-operation arguments: an operation parameter `m: Money` called with a `TaxedMoney` argument checks and runs, including through an `Upcast` of the operation and a dispatch guard or body binding, and its result of declared type `Money` may be a `TaxedMoney`. A `Money` argument at a `TaxedMoney` parameter and an unrelated record at a `Money` parameter refuse `ill_typed`/`type-mismatch`, naming the parameter. A function argument behaves the same way. | Test (TC-1001) |
| FR-359-AC-9 | A replay snapshot whose `Order.total: Money` holds `{"record":{"type":".../TaxedMoney","fields":{...}}}` admits, and so does the same record as the payload of an `Order.discount: Money?` field and as an element of a `Sequence<Money>` field and parameter. A decode followed by an encode returns the same typed observation and digest, with `type` still naming `TaxedMoney`. A `type` naming an unrelated record, a record that is only an ancestor of the slot's type, or an object refuses `invalid_runtime_input`/`wrong-value-kind` at the first path reached in field order (FR-106-AC-15). A `TaxedMoney` payload missing `tax_minor` refuses `missing-member`, though a `Money` payload without it admits. | Test (TC-1002) |
| FR-359-AC-10 | For a `TaxedMoney` at a `Money` slot, no QSL check, evaluation or replay path reports `EqualityOperandSourceNotAdmitted` or `field-option-payload-rejected`. A hand-built checked package whose `Option<Money>` payload is an unrelated record still reports `field-option-payload-rejected`, and one whose field read names a field absent from the value's declaration reports `field-slot-unresolved`. | Test (TC-1002) |

## Dependencies

- QSpec FR-208 states the substitution and equality rule; FR-151 the
  conformance rule; FR-322 the project, structural equality and owner join.
  FR-208 names the field, parameter and result slots. Its extension to
  `Option` payloads and collection elements in item 5 is this requirement's
  statement, and a QSpec amendment of FR-208 is requested.
- QSV FR-113 states `TypeEnvironment::admits` and exact composite equality
  identity; the amendments in item 8 are requested there and in quire-exact.
- FR-056-AC-18, FR-093, FR-100, FR-103 and FR-106 keep their text; this
  requirement states how the child value they admit is typed and carried.
