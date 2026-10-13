---
id: FR-360
title: "Type a field, parameter or result by a variant type through intake, check, evaluation and replay"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-002
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-317
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-359
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-141
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-208
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-360: Type a field, parameter or result by a variant type through intake, check, evaluation and replay

## Description

When a field member, an operation parameter or an operation result of an
admitted domain package has a `typeRef` that names a declaration bound to
`quire.meaning.model.variant-type/v1`, QSL SHALL admit the member, type it by the variant type's enum type, and
check, evaluate, compare, replay and encode its values as the target design
below states.

FR-056 "Variant types" admits the variant type itself: one `enum` export, one
`variant` export per case and, per "Effective identity of a variant type", one
effective identity for the type and one for each case. QSpec FR-208 defines the
variant type as a closed set of named cases with no payload, value, ordinal or
supertype. This requirement states what a member, an expression and a replay
document do with it. A **variant value** is one case of one variant type. A
**variant slot** is a field, parameter, result, `Option` payload, collection
element, tuple position or record field whose declared type is a variant type.

## Target design

1. **Admission.** A `typeRef` naming a variant type SHALL resolve to that
   variant type's declaration key, and the member SHALL be admitted. This holds
   for a field of every declaration meaning that carries fields (object type,
   record value type, event type, state machine, process), for an operation
   parameter and for an operation result. The member's own refusals are
   unchanged: an event's occurrence field must still resolve to `Timestamp`, so
   a variant-typed occurrence field refuses as FR-056-AC-17 states for a
   non-Timestamp binding. A `typeRef` naming a variant type whose declaration
   FR-056 refuses raises no refusal of its own. This requirement replaces
   FR-056's former placeholder refusals of a variant `typeRef`
   (`invalid_model_binding`/`malformed-declaration`, paths `Field.typeRef`,
   `Operation.params[i].typeRef` and `Operation.returns.typeRef`) and of a
   clause naming a variant type (`unsupported_construct`/`declaration-form`);
   all three are removed for variant types and for no other meaning.
2. **Export shape.** A variant-typed field SHALL export one `field`, keyed and
   shaped as FR-056 keys every other `field`. It SHALL NOT export a
   `reference`, which only an object-typed field has. An enclosing `record`
   export lists the field among its effective fields. The `enum` and `variant`
   exports of the variant type are byte-identical whether or not any member
   names it. The assembler SHALL map the member to the type environment's
   `M::<artifact id>` enum declaration of FR-056 "Effective identity of a
   variant type", the declaration keyed by the type's effective identity with
   each case keyed by its own effective identity.
3. **Presence and multiplicity** follow FR-056-AC-10 and QSpec FR-322's
   "Model-owned members" step 4 unchanged: multiplicity `[1, 1]` gives
   `M::Tier` outright, an `optional` presence wraps it in `Option`, and any
   other multiplicity gives the `Sequence`, `Set`, `Bag` or `OrderedSet` its
   flags name. Neither a `Reference<...>` wrapper nor object identity applies.
4. **Naming a variant type and its cases in source.** In a type position,
   `Alias::Tier` names the variant type artifact `Tier` of the package the
   `model Alias` declaration selects, as `Alias::Widget` names an object type. A
   case is written `Alias::Tier::Gold`, where `Gold` is the exact UTF-8 case
   name, matched byte for byte (`gold` is another case). A case literal has the
   static type `Alias::Tier` and the value of that case. No other literal has a
   variant type: a Text literal, an Integer literal or a bare `Gold` at a
   variant slot is not a variant value. A name that resolves to no variant type
   of the alias, or a case the type does not declare, refuses
   `ill_typed`/`type-mismatch` at the name, naming the variant type and, for a
   case, the case, as FR-317 refuses an undeclared union member.
5. **Expression forms that produce a variant value.** A variant value is
   produced by a case literal, a read of a variant-typed field, a variant-typed
   parameter, `let` binding or function result, a variant-typed result of an
   operation, the payload of a present `Option` (`value`), an element of a
   collection, and a conditional whose branches both have the same variant
   type. There is no conversion into or out of a variant type: no Text, Integer
   or ordinal conversion of a case, and no construction of a case from a name
   computed at runtime. A `case` expression over a variant type is refused as
   one over a native enum is (ADR-012 §16: `case` is admitted over a declared
   union only).
6. **Check typing is nominal and invariant.** A value of variant type `V`
   conforms to a slot whose declared type is exactly `V` and to no other. A
   variant type has no supertype, so FR-359's ancestor rule never applies to it
   and no other type conforms to `V`. Two variant types that share a case name
   are different types: `Alias::Level::Gold` at an `Alias::Tier` slot refuses
   `ill_typed`/`type-mismatch`, as do a value of another record, object,
   native enum or scalar, and an `Option<V>` where `V` is expected. The
   refusal is at the expression, names the expected and actual types, and
   follows FR-359's rule that a collection or `Option` of a variant type is
   not covariant.
7. **Equality is by case.** For two operands whose static type is `V`, `==` is
   true exactly when both are the same case of `V` and `!=` is its negation,
   decided by the case's effective identity (FR-056 "Effective identity of a
   variant type"), never by name text, position or rank. The result is a
   Boolean; it is never a checked-invariant fault. A comparison of two
   different static types is `ill_typed` at check, as FR-141-AC-2 states for
   enumerations, so it never reaches evaluation. Equality reaches a variant
   value inside another value by the same rule: a record field (under
   FR-359 item 4), an `Option` payload, a collection element and a tuple
   position. A variant type is **unordered**: `<`, `<=`, `>` and `>=` refuse at
   check with the cause the checker gives an ordering over a native unordered
   enum (FR-141-AC-5), and no case has a rank visible to source. The canonical
   key order a `Set`, `Bag` or `OrderedSet` needs between two cases is the
   ascending order of the cases' effective identities, the order FR-056 gives
   the type environment declaration. It is internal and is not an ordering
   operator.
8. **Signature positions.** An operation parameter or result typed by a variant
   type is admitted under item 1, is checked under items 4 to 7 at every call,
   dispatch guard, `requires`, `ensures` and body binding, and is replayed under
   item 9. FR-056's signature refusal path (`Operation.params[i].typeRef`,
   `Operation.returns.typeRef`) stays for the meanings FR-056 lists and is
   removed for a variant type.
9. **Replay observation form.** A variant value in a snapshot, invocation
   parameter or result is exactly `{"variant": {"type": "<declaration
   identity>", "case": "<case name>"}}`, closed: the outer object has exactly
   the `variant` tag and its payload has exactly `type` and `case`. `type` is
   the IR node identity of the variant type (`ix://<package identity>/<artifact
   id>`) and `case` is the exact declared case name. Admission SHALL resolve
   `type` to the slot's declared variant type and the case to one of its
   declared cases, deciding case equality by byte equality of the name. It
   SHALL refuse at the nested value path with `invalid_runtime_input`: a
   payload missing `type` or `case` as `missing-member`; an extra member as
   `unknown-member`; a value tag other than `variant`, a `type` that names any
   declaration other than the slot's variant type (another variant type, a
   record, an object, no declaration) and a payload that is not an object as
   `wrong-value-kind`; and a `case` that is not a declared case of the type,
   including a letter-case variant of one, as `invalid-value`. A variant value
   inside an `{"present": ...}` or `{"sequence": [...]}` is admitted by the
   same rule at its element, and the closed value grammar of FR-106 otherwise
   gains only this form. Evaluation, result sizing and equality use the
   admitted case's effective identity.
10. **Encoding.** Encoding a variant value SHALL write the form of item 9 from
    the value's type and case, and a decode followed by an encode SHALL yield
    the same typed observation and digest; member order of the JSON does not
    affect either. A counterexample witness value text encodes a variant value
    as FR-070's `enum` shape, with `name` the variant type's checked node id
    and `member` the case name.
11. **Identity across units.** The variant type's declaration key and each
    case key SHALL be functions of the selected document's identity and the
    node alone (FR-056's effective identity, FR-359 item 9), not of the alias a
    unit gives the model. Two units that select the same package under
    different aliases SHALL agree on every key, and a value built in one equals
    the same case built in the other.

## Cross-repo requirements

Each owner adds its own specification. QSL does not wait on them to state its
own behaviour above.

1. **QSpec FR-322:** a `model`/`variant_type` semantic form with a
   `ModelOwner` and a step 2 row naming `quire.meaning.model.variant-type/v1`,
   with family `enum` for equality and no ordering, so the checked package
   carries the variant type as a model-owned node. Reusing a unit-owned `enum`
   node would key it by the selecting unit's alias.
2. **QSpec FR-208:** the "Field types" bullet, the "Variant types" section and
   FR-208-AC-15 name a variant `typeRef` as `malformed-declaration`, and the
   "Variant types" meaning row says a clause naming one is
   `unsupported_construct`/`declaration-form` (FR-150). Both are replaced by
   this requirement's admission; the case, payload, shape, identity and
   duplicate-case refusals stay.
3. **QSpec shared grammar:** the three-segment qualified name `Alias::Tier::Gold`
   of item 4, resolved by the same qualified-reference production as
   FR-317's `q::m`.

## Inputs

- An admitted domain package with variant types (FR-056 "Variant types") and a
  member whose `typeRef` names one.
- A `1-draft` unit that selects the package and an expression, call or clause
  with a variant-typed operand.
- A replay request whose snapshot, parameter or result holds a variant value.

## Outputs

- A package whose variant-typed members are admitted, with `field` exports and
  no `reference`, and an assembled type environment whose members have the
  variant type's enum type.
- A checked expression, an evaluated Boolean or variant value, or a typed
  refusal at the expression or the nested replay value.

## Behavior

- Every refusal of this requirement is raised at the node it names and is a
  typed code and cause, never a checked-invariant fault.
- Admission raises no refusal for a variant `typeRef`. The refusals that remain
  for the variant type itself are FR-056's: payload, no case, no span,
  mis-minted identity, repeated case and non-enumeration shape.
- A variant slot never holds `none` unless its type is `Option<V>`, and never
  holds a case of another variant type.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-360-AC-1 | An admitted package with variant type `Tier` (cases `Gold`, `Standard`), object `Account` with field `tier: Tier`, and record value type `Settings` with field `tier: Tier` is admitted with no refusal. Each field has one `field` export keyed as any other field and no `reference` export; `Settings`' `record` export lists `tier`; `Tier`'s `enum` and `variant` exports equal those of the same package with no field naming it. A field `Event.tier` of an event type whose occurrence field is `Tier` refuses `invalid_model_binding`/`malformed-declaration` at `TypeDefinition.occurrenceField`, while a non-occurrence `Event.tier` is admitted. | Test (TC-1006) |
| FR-360-AC-2 | The unit assembled over that package gives `Account.tier` the type `Config::Tier`, an `optional` field `Option<Config::Tier>`, and an ordered non-unique `[0, 3]` field `Sequence<Config::Tier>[0, 3]`, never `Reference<Config::Tier>`; an operation parameter or result naming `Tier` has the same type. The declaration key of `Config::Tier` and of each case equals the effective identity of the view, and a unit that selects the same package as `Cfg` assigns the same keys. | Test (TC-1006) |
| FR-360-AC-3 | A variant `typeRef` whose variant type has no case refuses once, `invalid_model_binding`/`malformed-declaration` at the variant type, path `TypeDefinition.variants`, and the field naming it raises no refusal of its own. A field whose `typeRef` names an event type, state machine, process, persistence interface or namespace keeps its refusal `invalid_model_binding`/`malformed-declaration` at the field, path `Field.typeRef`. | Test (TC-1006) |
| FR-360-AC-4 | With `Config::Tier::Gold` in a function `f(t: Config::Tier): Config::Tier { t }`, a field read `a.tier`, a `let` binding, an `Option` `value`, a collection element and `if c then Config::Tier::Gold else a.tier` each check as type `Config::Tier`; `Config::Tier::Gold` at an `Account.tier` slot, passed to `f` and in an `Option<Config::Tier>` literal checks. A clause may name `Config::Tier` in a type position and `Config::Tier::Gold` as a literal with no `unsupported_construct` refusal. | Test (TC-1007) |
| FR-360-AC-5 | Each of the following refuses `ill_typed`/`type-mismatch` at the expression, naming the expected and actual types: `Config::Level::Gold` at a `Config::Tier` slot; the Text literal `"Gold"` and the Integer literal `0` at a `Config::Tier` slot; a bare `Gold`; a `Config::Tier` at a `Config::Level` parameter; a `Sequence<Config::Tier>` at a `Sequence<Config::Level>` and at `Option<Config::Tier>`; `Config::Tier::Silver` (undeclared case) and `Config::Tier::gold` (letter-case variant); `Config::Nope::Gold`; and `Config::Tier::Gold < Config::Tier::Standard`, `<=`, `>` and `>=` with the cause the checker gives an ordering over a native unordered enum. A `case` expression over a `Config::Tier` is refused as over an enum. | Test (TC-1007) |
| FR-360-AC-6 | Over `Config::Tier`, `==` is true for `Gold == Gold` and false for `Gold == Standard`, `!=` the negation, in either operand order, with no checked-invariant refusal. `Config::Level::Gold == Config::Tier::Gold` refuses `ill_typed` at check and never evaluates. A case read from a field equals the same case written as a literal. | Test (TC-1008) |
| FR-360-AC-7 | Equality reaches a variant value inside other values by the same rule: two `Settings` records equal exactly when `tier` is the same case (and their other fields are equal), two `Option<Config::Tier>` equal exactly when both are `none` or both hold the same case, and two `Sequence<Config::Tier>`, `Set<Config::Tier>` and a tuple position likewise. A `Set<Config::Tier>` holding `Gold` twice has one element, and holding `Gold` and `Standard` has two in the ascending order of the cases' effective identities, whichever order the source writes them. | Test (TC-1008) |
| FR-360-AC-8 | An operation parameter of type `Tier` is admitted at intake, its call with `Config::Tier::Gold` checks, runs and reads back, and a call with a `Config::Level` value refuses `ill_typed`/`type-mismatch` naming the parameter; an operation result of type `Tier` is admitted and its value compares by case. A parameter or result naming an interface-featured object type keeps `invalid_model_binding`/`malformed-declaration` at the operation node, path `Operation.params[i].typeRef` or `Operation.returns.typeRef`. | Test (TC-1009) |
| FR-360-AC-9 | A replay snapshot whose `Account.tier` holds `{"variant":{"type":"ix://<package identity>/Tier","case":"Gold"}}` admits, and so does the same value as the payload of `{"present": ...}` and as an element of `{"sequence": [...]}` at a `Sequence<Config::Tier>` slot, and as an operation parameter and result. The admitted value equals the literal `Config::Tier::Gold`. | Test (TC-1010) |
| FR-360-AC-10 | Each of these refuses `invalid_runtime_input` at the nested value path: a payload missing `case` (`missing-member`) or `type` (`missing-member`); an extra payload member (`unknown-member`); a `type` naming `Level`, a record, an object or no declaration, a `{"string":"Gold"}` or `{"integer":"0"}` at the slot, and a payload that is not an object (`wrong-value-kind`); `"case":"Silver"` and `"case":"gold"` (`invalid-value`). | Test (TC-1010) |
| FR-360-AC-11 | A decode followed by an encode returns the same typed observation and digest with the object members in either order; the encoded value names its own variant type and case. A counterexample witness holding a `Config::Tier` parameter encodes it as the enum shape with the type's checked node id and the case name and decodes to the same value. Two units that select the package as `Config` and as `Cfg` assign equal keys, and a case built in one equals the same case built in the other. | Test (TC-1010) |

## Dependencies

- FR-056 admits the variant type, its exports, its effective identity and the
  type environment declaration; this requirement replaces its placeholder
  refusals for fields, signatures and clauses. QSpec FR-208 defines the variant
  type and FR-141 the enumeration equality and unordered refusals this
  requirement reuses. FR-359 states the record-slot rules it parallels and
  keeps its alias-independence rule (item 11). FR-317 states qualified-name
  resolution. FR-106 states the replay value grammar it extends by one form.

## References

- Owning ticket: QSL-738.
