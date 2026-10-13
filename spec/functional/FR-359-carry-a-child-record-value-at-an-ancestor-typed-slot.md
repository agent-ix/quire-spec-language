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
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-334
    type: depends_on
---
# FR-359: Carry a child record value at an ancestor-typed slot through check, evaluation and replay

## Description

When a record value whose most-specific type conforms to the record type `P` of
a slot reaches that slot at check, evaluation or replay, QSL SHALL keep the
value as it is and SHALL read, compare, store, pass and encode it as the target
design below states.

QSpec FR-208 says a value of a record subtype is a value of each ancestor
wherever a field, parameter or result names the ancestor, and that equality
compares the most-specific type and every effective field. FR-056-AC-18 binds
that to TC-235 M17. QSV FR-113 makes `TypeEnvironment::admits` accept a child
composite at an ancestor-typed slot. This requirement states what the rest of
QSL, and the kernel changes it needs, do with that value. An event type is a
record value for this purpose (FR-208 gives it the same inheritance); an object
is never one, and a `Reference<T>` keeps its own conformance rule (FR-100,
FR-322 "Reference conformance").

A **slot** is a field, parameter, result, `Option` payload, collection element,
tuple position or union member payload whose declared type names a record value
type `P`. A **child** is a
value whose most-specific record type `S` is `P` or has `P` among its
ancestors, at any depth.

## Target design

1. **A child keeps its identity.** The value at a `P` slot is the child value
   itself, tagged with `S`'s declaration key. Admission SHALL NOT retag it,
   copy it into a `P` value, drop its extra fields or insert a conversion
   step. This is the only reading under which "a subtype value never equals a
   value of a proper ancestor" can hold: a projected copy would equal the
   stated `P`.
2. **A slot is read by field identity, never by position.** A checked record
   field read SHALL carry the field's identity, its declaring record's
   declaration key and its name, and SHALL NOT carry a position. At
   evaluation the field's slot in the value is found in the value's own
   declaration by that identity. When the value's type redefines the field
   (FR-082), the slot is the effective field that redefines it, transitively.
   The read returns the stored value as it is, with no conversion; the value
   is admitted at the read's checked result type, which is the type the
   declaring record gives the field, because the redefining field's type
   conforms to it (FR-151). That holds for a narrower record type and for a
   narrower scalar such as `Int[0, 9]` for `Integer`. A child's effective
   fields are ordered by their own keys and are not a positional prefix of the
   ancestor's, so a position computed from `P` names a different field in `S`.
   A tuple has no supertypes in source, so a tuple position stays a position.
3. **Records and events are emitted as model-owned nodes.** The checked IR
   SHALL carry a domain package's record value type as a
   `model`/`record_value_type` node with its `ModelOwner` (FR-094, FR-103,
   FR-322-AC-63), and its event type as a `model`/`event_type` node with its
   `ModelOwner` (cross-repo requirement 7), and SHALL NOT emit either as a
   `composite_type`/`record` node with a source owner. Today QSL's `close_composite` emits it as the latter and
   `record_form` maps both kinds to no node, so the checked package names no
   relation to the selection that declares `supertypes`; the emitter is
   changed by the slice that implements this requirement. With that node the
   relation is carried by content the package pins: the `ModelOwner` joins to a
   digest-pinned selection (FR-322 "Model-owned members" step 2), that
   document declares the record's `supertypes`, and FR-322-AC-30 resolves an
   inherited field on the subtype's node. The lowered read stays
   `quire.op.record.project` with `field{declaration: P's node, name}` (FR-093)
   and the lowered call or record keeps its operand `ref`. The emitter SHALL
   NOT add a `supertypes` member, which would copy the selected document into
   the package, or a conversion node, which would contradict item 1.
4. **Equality is decided by identity first.** FR-208 gives an event type no
   equality rule, so this item applies to record value types only; an event
   value is read, stored, passed and encoded as the other items state, and
   `==` over event types stays refused. For two operands whose static type is
   the record value type `P`, `==` SHALL be false and `!=` SHALL be true when
   their most-specific declaration keys differ, at any depth of specialization.
   When the keys are equal the operands are equal exactly when every effective
   field is equal, each field under this same rule. The result is a Boolean,
   never a checked-invariant fault: the evaluator SHALL NOT report
   `EqualityOperandSourceNotAdmitted` or `ValueKindMismatch` for a conforming
   operand. The identity check precedes the field and text-leaf comparison, so
   a leaf list derived from the static type (FR-093) is never applied to a
   value of a different most-specific type. The same rule applies wherever
   equality reaches a record inside another value: a record field, an `Option`
   payload, a collection element, a tuple position and a union payload.
   Equality still requires both operands to have the same static type, and a
   `Child == Parent` over different static types stays `ill_typed`.
5. **Payloads are admitted by conformance at every depth.** An `Option<P>`
   payload, a `Sequence<P>`, `Set<P>`, `Bag<P>` or `OrderedSet<P>` element, a
   tuple position, a union member payload and a record field of type `P` SHALL
   admit a child.
   Constructing, projecting (`present`, `value`, field read), storing and
   re-reading such a value SHALL preserve the child's identity. A `Set<P>`,
   `Bag<P>` or `OrderedSet<P>` holding a `P` value and a child with the same
   shared fields holds two distinct elements, because they are unequal
   (item 4). The canonical-key order between two values of **distinct
   declarations** is the ascending byte order of their 32-byte declaration
   keys, compared before any slot; values of one declaration continue by slots
   as FR-144 orders a record (lexicographic over fields in declaration order).
   Two values then have equal keys exactly when they have the same declaration
   and equal fields, which is when they are equal under item 4, as FR-144
   requires of equal keys and FR-149 of equality. The order is total and
   unrelated to ancestry, which no collection order needs. It is a function of
   content alone only because of item 9. A type is not covariant: an expression of type `Sequence<S>` or
   `Option<S>` SHALL be refused at a `Sequence<P>` or `Option<P>` slot, because
   a collection's type identity includes its element type (FR-334). A child is
   admitted into the slot as a value, as `sequence[child]` at `Sequence<P>`
   does.
6. **Arguments and results.** A function argument, a model-operation argument
   (including an `Upcast` of an operation parameter and a dispatch guard or
   body binding), a result and a `modifies` field grant SHALL admit a child at
   a `P`-typed parameter or result by the same conformance rule. A `P` value
   never conforms to `S`, an intermediate ancestor never conforms to a
   descendant, and an unrelated record never conforms to `P`. A compiled
   program passing a child to an operation parameter is read back by the IR
   reader (`emit_and_read_v2`), so it also depends on the reader accepting that
   call (cross-repo requirement 4).
7. **Wire and replay.** Replay SHALL decode every record by the declaration key
   its own `type` member names (FR-106-AC-14) and SHALL read its fields by name
   against that declaration's effective fields, not against the slot's declared
   type. It SHALL then admit the decoded value at its slot with the
   selected-model `TypeEnvironment::admits`, which accepts exactly the values
   whose most-specific type conforms to the slot's type. The same holds for an
   `Option` payload and a collection element. Encoding SHALL write the value's
   own most-specific `type` and its own effective fields, and a decode followed
   by an encode SHALL yield the same typed observation and digest.
8. **Kernel checks that stop being exact-key.** Every check between a
   conforming value and its slot SHALL decide conformance with the
   `TypeEnvironment`, and a refusal there SHALL mean non-conformance. The
   outcome is that no conforming payload, element or operand reaches a
   checked-invariant fault. The kernel `ValueType::admits` stays the exact,
   environment-less predicate and SHALL NOT be called to admit a value into a
   slot that can hold a child. The mechanisms, one per owner, are the
   cross-repo requirements below.
9. **A model record's runtime declaration key is its model node key.** The
   declaration key under which a model record or event type is declared in the
   type environment, and which identifies a value's most-specific type for
   equality and for the item 5 order, SHALL be the model-owned node key of
   FR-322 "Model-owned members" step 2, a function of the selected
   document's identity and the node alone. It SHALL NOT be derived from the
   selecting unit's source owner or from the alias a unit gives the model
   (`model M = ...`); QSL's `declared_type_handle(owner, "{alias}::{artifact}")`
   derivation does that today, so a `Set<Money>` would order differently under
   two aliases and a value would not equal the same value built by a unit
   using another alias. Two units selecting the same model under different
   aliases SHALL therefore agree on every declaration key.

## Cross-repo requirements

QSL-737 enumerates all of these. Each owner adds its own specification.

1. **QSV** (FR-113 amended): `TypeEnvironment::admits` admits a child at every
   nested depth, and `operand_value` checks a composite operand by conformance,
   as it already checks a `Reference<T>` operand by kind only.
2. **quire-exact, equality:** kernel equality of two composite values with
   different declarations SHALL be `false` (and `!=` true), not the
   `ValueKindMismatch` checked invariant that the catch-all arm of
   `equality.rs` returns today. A composite against a non-composite stays that
   fault.
3. **quire-exact, canonical key and collections:** `compare_keys` (`key.rs`)
   SHALL order two composites of distinct declarations by the byte order of
   their declaration keys (item 5), so that set, bag and ordered-set
   construction no longer refuses `CanonicalKeyUnavailable` for them. Collection
   construction SHALL gain an environment-taking constructor that admits
   conforming elements, orders them by the canonical key and charges them. No
   new `Option` constructor is needed: `OptionValue::from_admitted` exists for
   a caller that has admitted its payload, and QSL SHALL use it after
   `TypeEnvironment::admits`; `OptionValue::present` stays the exact kernel
   constructor.
4. **quire-contract-ir:** the IR reader's operation-call rule
   (`check_model_member`) SHALL accept an argument whose record type conforms to
   the parameter's declared record type, as QSpec FR-322 will state (5).
5. **QSpec:** FR-208 SHALL name `Option` payload, collection element, tuple
   position and union payload slots. FR-144 (canonical keys) SHALL define the
   record key order across distinct declarations as item 5 does, keeping its
   rule that keys are equal exactly when the values are FR-149-equal. FR-149
   (composite `=`) and FR-143 (record equality, FR-143-AC-5) SHALL state that
   two records of different declarations are unequal, not a refusal. FR-322
   SHALL state record-operand conformance for `quire.op.function.call` and
   operation-call arguments, `quire.op.record.project` and
   `quire.op.structural.eq`.
6. **QSL emitter:** items 3 and 9.
7. **QSpec, event owner join:** FR-322 "Model-owned members" step 2 admits a
   `model`/`record_value_type` node only for an FR-208 record value type and
   gives an event type no form, so an event node's owner join refuses
   `missing_declaration`/`missing-selection`. FR-322 SHALL add a
   `model`/`event_type` semantic form (family `record`, field projection
   admitted, equality refused as FR-208 gives events none), a step 2 row naming
   `quire.meaning.model.event-type/v1`, and the owner-presence and semantic-form
   entries of the V2 schema. Reusing `record_value_type` for events was
   considered: it is the smaller edit, but it labels an event with a record
   value type's equality and value semantics that FR-208 withholds.

## Inputs

- The effective view and ancestor graph FR-081 and FR-082 produce.
- A checked expression whose operand, argument or element has a record type
  that conforms to the slot's declared record type.
- A replay request whose snapshot, parameter or result holds a record.

## Outputs

- A checked package whose record and event nodes are model-owned
  `record_value_type` nodes and whose field reads name their field.
- An evaluated value that retains its most-specific type, or a Boolean for an
  equality, or a typed refusal at the slot that does not conform.

## Behavior

- The refusal for a value whose most-specific type does not conform to the
  slot is `ill_typed`/`type-mismatch` at check, `wrong-value-kind` at replay
  admission (FR-106-AC-15) and `WrongValueKind` at call admission (FR-100).
  Neither the evaluator nor the kernel reports these as a checked-invariant
  fault.
- A source read of a field the operand's declared record type does not declare
  keeps today's `ill_typed`/`type-mismatch` from the QSL checker. The
  `ill_typed`/`operator-ineligible` of FR-322-AC-65 is the IR reader's cause for
  a hand-built package and is not a QSL source-check cause. This requirement
  changes neither.
- The checked-invariant causes `field-option-payload-rejected` and
  `field-slot-unresolved` (FR-090) remain, for a payload that does not conform
  and a value whose declaration has no such field. A conforming value reaches
  neither.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-359-AC-1 | With `record OrderPlaced { order_id: UUID; occurred_at: Timestamp }`, a child `ExpressOrderPlaced` adding `promised_by: Timestamp`, whose effective fields are ordered `[promised_by, occurred_at, order_id]`, a grandchild `RushOrderPlaced` of `ExpressOrderPlaced` adding `rush_by: Timestamp`, and `f(e: OrderPlaced): Timestamp { e.occurred_at }`, passing an `ExpressOrderPlaced` or a `RushOrderPlaced` evaluates to its `occurred_at` and `e.order_id` to its `order_id`, never `promised_by` or `rush_by`. The same read through a record field, an `Option` payload and a `Sequence` element holding the child yields the same fields. A parent-typed read of `promised_by` refuses `ill_typed`/`type-mismatch` at check, naming the field, the cause the checker gives an undeclared record field today. | Test (TC-998) |
| FR-359-AC-2 | When a child's type redefines an inherited field with a narrower record type, a parent-typed read of that field yields the child's redefining slot. When it redefines an `Integer` field as `Int[0, 9]`, a parent-typed read yields the stored integer unchanged, admitted at `Integer`. A read of an inherited field the child does not redefine yields the inherited slot. | Test (TC-998) |
| FR-359-AC-3 | In the checked package for AC-1, `OrderPlaced`, `ExpressOrderPlaced` and `RushOrderPlaced` are each a `model`/`record_value_type` node with a `ModelOwner` and none is a `composite_type`; a domain event type is a `model`/`event_type` node with a `ModelOwner`, and reads back `Verified` once cross-repo requirement 7 holds. The body of `f` holds one `quire.op.record.project` application whose `field` member is `{declaration: OrderPlaced's node, name: occurred_at}`; the call to `f` takes its argument by `ref`, with no `convert` or other node between them; no node has a `supertypes` member. The package reads back `Verified`. | Test (TC-998) |
| FR-359-AC-4 | For `a == b` and `a != b` with `a: Money` bound to a `TaxedMoney` value and `b: Money` bound to a `Money` value with the same `amount_minor` and `currency`, `==` is `false` and `!=` is `true`, in either operand order. With a grandchild `GiftTaxedMoney` of `TaxedMoney`, `GiftTaxedMoney` against `Money`, against `TaxedMoney` and against another `GiftTaxedMoney` of equal fields give `false`, `false` and `true`. No checked-invariant refusal is reported. Two `TaxedMoney` values with equal effective fields are equal, and unequal when only `tax_minor` differs. | Test (TC-999) |
| FR-359-AC-5 | Equality reaches a child inside another value by the same rule: two `Pair { first: Money; second: Money }` whose `first` fields hold a `TaxedMoney` and a `Money` of equal shared fields are unequal, and equal when both hold equal `TaxedMoney` values; the same holds for two `Option<Money>`, two `Sequence<Money>`, two `Set<Money>`, a tuple position and a union payload. An `==` over two event-typed operands stays refused. The comparison of a child with a different most-specific type performs no text-leaf comparison. | Test (TC-999) |
| FR-359-AC-6 | A record `Order { total: Money?; }` built with `total` holding a `TaxedMoney` checks, evaluates and is read back through `total`, `present(total)` and `value(total)` as the same `TaxedMoney` value, with its `tax_minor` and most-specific type intact. No `field-option-payload-rejected` fault is reported. A `Money` value at a `TaxedMoney?` field and a value of an unrelated record at a `Money?` field refuse `ill_typed`/`type-mismatch`. | Test (TC-1000) |
| FR-359-AC-7 | `Sequence<Money>`, `Set<Money>`, `Bag<Money>` and `OrderedSet<Money>` literals holding a `TaxedMoney` element admit it and keep its identity through `contains`, iteration and `length`. A `Set<Money>` holding a `Money` and a `TaxedMoney` of equal shared fields has two elements, whose order is ascending by declaration key bytes, in either source order, and constructing it does not report `CanonicalKeyUnavailable`. A `Sequence<TaxedMoney>` expression at a `Sequence<Money>` parameter, and an `Option<TaxedMoney>` expression at an `Option<Money>` parameter, refuse `ill_typed`/`type-mismatch`. | Test (TC-1000) |
| FR-359-AC-8 | Model-operation arguments: an operation parameter `m: Money` called with a `TaxedMoney` argument checks, is emitted, reads back `Verified` through `emit_and_read_v2` and runs, including through an `Upcast` of the operation and a dispatch guard or body binding, and its result of declared type `Money` may be a `TaxedMoney`. A `Money` argument at a `TaxedMoney` parameter and an unrelated record at a `Money` parameter refuse `ill_typed`/`type-mismatch`, naming the parameter. A function argument behaves the same way. The read-back half depends on cross-repo requirement 4. | Test (TC-1001) |
| FR-359-AC-9 | A replay snapshot whose `Order.total: Money` holds `{"record":{"type":".../TaxedMoney","fields":{...}}}` admits, and so does the same record as the payload of an `Order.discount: Money?` field and as an element of a `Sequence<Money>` field and parameter. A `GiftTaxedMoney` admits at a `Money` slot and at a `TaxedMoney` slot, and a `TaxedMoney` at a `GiftTaxedMoney` slot refuses `wrong-value-kind`. A decode followed by an encode returns the same typed observation and digest, with `type` still naming the value's own record. A `type` naming an unrelated record, a record that is only an ancestor of the slot's type, or an object refuses `invalid_runtime_input`/`wrong-value-kind` at the first path reached in field order (FR-106-AC-15). A `TaxedMoney` payload missing `tax_minor` refuses `missing-member`, though a `Money` payload without it admits. | Test (TC-1002) |
| FR-359-AC-10 | For a `TaxedMoney` at a `Money` slot, no QSL check, evaluation or replay path reports `EqualityOperandSourceNotAdmitted`, `ValueKindMismatch`, `CanonicalKeyUnavailable` or `field-option-payload-rejected`. A hand-built checked package whose `Option<Money>` payload is an unrelated record still reports `field-option-payload-rejected`, and one whose field read names a field absent from the value's declaration reports `field-slot-unresolved`. | Test (TC-1002) |
| FR-359-AC-11 | Two units that select the same model under different aliases (`model M = ...` and `model N = ...`) assign every model record the same declaration key, equal to its model node key. A `Set<Money>` holding a `Money` and a `TaxedMoney` orders its elements identically in both, and a `TaxedMoney` built in one unit equals the same value built in the other. | Test (TC-1000) |

## Dependencies

- QSpec FR-208 states the substitution and equality rule; FR-151 the
  conformance rule; FR-322 the project, structural equality and owner join.
  FR-208 names the field, parameter and result slots; its extension to
  `Option` payloads, collection elements and tuple positions in item 5 is this
  requirement's statement, and the QSpec amendment is cross-repo requirement 5.
- QSV FR-113 states `TypeEnvironment::admits` and exact composite equality
  identity; its amendment is cross-repo requirement 1.
- FR-056-AC-18, FR-100, FR-103 and FR-106 keep their text. FR-093 changes: its
  `Field` row names the field identity (item 2).
