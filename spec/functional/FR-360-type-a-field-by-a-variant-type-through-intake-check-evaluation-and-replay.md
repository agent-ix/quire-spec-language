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
`quire.meaning.model.variant-type/v1`, QSL SHALL admit the member.

When a member is admitted under that rule, the assembler SHALL type it by the
variant type's enum type.

When a unit or a replay document holds a value of a variant type, QSL SHALL
check, evaluate, compare, replay and encode it as the target design below
states.

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
   variant type's declaration key, and intake SHALL admit the member. This holds
   for a field of every declaration meaning that carries fields (object type,
   record value type, event type, state machine, process), for an operation
   parameter and for an operation result. The member's own refusals are
   unchanged: an event's occurrence field must still resolve to `Timestamp`, so
   a variant-typed occurrence field refuses as FR-056-AC-17 states for a
   non-Timestamp binding. A `typeRef` naming a variant type whose declaration
   FR-056 refuses SHALL raise no refusal of its own. This requirement replaces
   FR-056's former placeholder refusals of a variant `typeRef`
   (`invalid_model_binding`/`malformed-declaration`, paths `Field.typeRef`,
   `Operation.params[i].typeRef` and `Operation.returns.typeRef`) and of a
   clause naming a variant type (`unsupported_construct`/`declaration-form`);
   all three are removed for variant types and for no other meaning.
2. **Export shape.** A variant-typed field SHALL export one `field`, keyed and
   shaped as FR-056 keys every other `field`. It SHALL NOT export a
   `reference`, which only an object-typed field has. An enclosing `record`
   export SHALL list the field among its effective fields. The `enum` and
   `variant` exports of the variant type SHALL be byte-identical whether or not
   any member names it. The assembler SHALL map the member to the type
   environment's `M::<artifact id>` enum declaration of FR-056 "Effective
   identity of a variant type".
3. **Presence and multiplicity** SHALL follow FR-056-AC-10 and QSpec FR-322's
   "Model-owned members" step 4 unchanged: multiplicity `[1, 1]` gives
   `M::Tier` outright, an `optional` presence wraps it in `Option`, and any
   other multiplicity gives the `Sequence`, `Set`, `Bag` or `OrderedSet` its
   flags name. Neither a `Reference<...>` wrapper nor object identity applies.
4. **Naming a variant type and its cases in source.** In a type position,
   `Alias::Tier` names the variant type artifact `Tier` of the package the
   `model Alias` declaration selects, as `Alias::Widget` names an object type. A
   case is written `Alias::Tier::Gold`, the form the native linker already
   resolves for a native model enum member, where `Gold` is the case name matched
   byte for byte (`gold` is another case). The checker SHALL give a case literal
   the static type `Alias::Tier` and the value of that case. No other literal has
   a variant type: a Text literal, an Integer literal or a bare `Gold` at a
   variant slot is not a variant value. Four refusals apply, each at the name:
   - a variant type name that resolves to no variant type of the alias (in a
     type position or as the qualifier of a case) SHALL refuse
     `missing_declaration`/`missing-name`, as FR-056-AC-9 refuses
     `Reference<M::Nope>` and FR-317 refuses a qualified name with no candidate;
   - a case the resolved variant type does not declare SHALL refuse
     `ill_typed`/`type-mismatch`, naming the type and the case, as FR-317-AC-2
     refuses an undeclared union member;
   - a clause context or an operation-body target that names a variant type
     names no object type and SHALL refuse `missing_declaration`/`missing-name`
     at the name;
   - the name of a case that is not a source identifier SHALL be unwritable:
     FR-056 admits any case name of at least one character (`Gold Plus`, `2x`),
     the variant type and its other cases stay admitted and writable, and the
     language has no quoting syntax, so a spelling of such a name does not parse
     as a case literal and is refused by the parser as any malformed name. A
     value of such a case still arises from a field read, a call result and
     replay.
5. **Expression forms that produce a variant value.** A variant value SHALL be
   produced only by a case literal, a read of a variant-typed field, a
   variant-typed parameter, `let` binding or function result, a variant-typed
   result of an operation, the payload of a present `Option` (`value`), an
   element of a collection, and a conditional whose branches both have the same
   variant type. The checker SHALL admit no conversion into or out of a variant
   type: no Text, Integer or ordinal conversion of a case, and no construction of
   a case from a name computed at runtime. Where the language has a Text literal, a
   `none` or a `case` expression, the checker SHALL refuse a Text literal or a
   `none` at a variant slot `ill_typed`/`type-mismatch` at the expression, and a
   `case` expression whose scrutinee has a variant type `ill_typed`/`type-mismatch`
   at the scrutinee (ADR-012 §16.2 admits `case` over a declared union only, and
   §16.5 gives a scrutinee that is not a union this refusal). QSL's expression
   forms hold neither a Text literal nor `none` today. A Text literal or `none` in an
   expression is read by S1 and refused by S2 as `unsupported_construct`, cause
   `UnrepresentedConstruct` of the `Primary` production, at the literal. Since FR-313, `case`
   opens an expression whose arm list is `{ ... }`: S1 refuses the control form
   `case t when ...` in expression position as `invalid_syntax`, cause
   `UnexpectedToken`, at the `when` token, and a well-formed `case` expression
   reaches the checker, which refuses it `unknown_required_feature`/`unsupported-feature`
   until FR-318 checks `case` expressions. The scrutinee `ill_typed`/`type-mismatch`
   refusal above belongs to FR-318. Neither the Text literal nor `none` reaches
   the checker.
6. **Check typing is nominal and invariant.** A value of variant type `V` SHALL
   conform to a slot whose declared type is exactly `V` and to no other. A
   variant type has no supertype, so FR-359's ancestor rule never applies to it
   and no other type conforms to `V`. Two variant types that share a case name
   are different types: `Alias::Level::Gold` at an `Alias::Tier` slot, a value of
   another record, object, native enum or scalar, and an `Option<V>` where `V` is
   expected SHALL each refuse `ill_typed`/`type-mismatch` at the expression,
   naming the expected and actual types. A collection or `Option` of a variant
   type is not covariant (FR-359 item 5). A `none` at a slot whose type is `V`
   rather than `Option<V>` SHALL refuse the same way.
7. **Equality is by case.** For two operands whose static type is `V`, `=`
   SHALL be true exactly when both are the same case of `V` and `!=` SHALL be its
   negation, decided by the case's identity (item 11), never by name text,
   position or rank. The result SHALL be a Boolean and never a checked-invariant
   fault. A comparison of two different variant static types SHALL refuse
   `ill_typed`/`type-mismatch` at check, as QSpec FR-141-AC-2 states for
   enumerations of different declarations, so it never reaches evaluation.
   Equality SHALL reach a variant value inside another value by the same rule: a
   record field (under FR-359 item 4), an `Option` payload, a collection element
   and a tuple position. A variant type is **unordered**: `<`, `<=`, `>` and
   `>=` over it SHALL refuse `ill_typed`/`operator-ineligible` at check (QSpec
   FR-141-AC-5; the checker's unordered-enum ordering refusal), and no case has a
   rank visible to source. The canonical key order of the cases in a `Set` or `Bag`
   SHALL be the ascending UTF-8 byte order of the case names,
   which is QSpec FR-144's key for an unordered enumeration and the order FR-056
   gives the `enum` export's `members`. A collection holds one declaration's
   cases, so FR-359 item 5's declaration-key order between declarations never
   applies. An `OrderedSet` keeps first-occurrence order and a `Sequence` keeps supply
   order (FR-359 item 5); neither reads the key. The key order is internal and
   is not an ordering operator.
8. **Signature positions.** An operation parameter or result typed by a variant
   type SHALL be admitted under item 1, checked under items 4 to 7 at every call,
   dispatch guard, `requires`, `ensures` and body binding, and replayed under
   item 9. FR-056's signature refusal path (`Operation.params[i].typeRef`,
   `Operation.returns.typeRef`) SHALL stay for the meanings FR-056 lists and be
   removed for a variant type.
9. **Replay observation form.** A variant value in a snapshot, invocation
   parameter or result is exactly `{"variant": {"type": "<declaration
   identity>", "case": "<case name>"}}`, closed: the outer object has exactly
   the `variant` tag and its payload has exactly `type` and `case`. `type` is
   the IR node identity of the variant type (`ix://<package identity>/<artifact
   id>`) and `case` is the exact declared case name. Admission SHALL resolve
   `type` to the slot's declared variant type and the case to one of its
   declared cases, deciding case equality by byte equality of the name.
   Admission SHALL check in this order and refuse the first defect found, at the
   nested value path, with `invalid_runtime_input`: (a) a value tag other than
   `variant`, or a payload that is not an object, `wrong-value-kind`; (b) a
   missing `type` or `case`, `missing-member`, reporting `type` first; (c) an
   extra payload member, `unknown-member`; (d) a `type` that names any
   declaration other than the slot's variant type (another variant type, a
   record, an object, no declaration), `wrong-value-kind`; (e) a `case` that is
   not a declared case of the type, including a letter-case variant of one,
   `invalid-value`. An `{"absent": {}}` at a required variant field is
   `wrong-value-kind` under FR-106 check 6 step 5. A variant value inside an
   `{"present": ...}` or `{"sequence": [...]}` SHALL be admitted by the same rule
   at its element, and the closed value grammar of FR-106 otherwise gains only
   this form. Evaluation, result sizing and equality SHALL use the admitted
   case's identity.
10. **Encoding.** Encoding a variant value SHALL write the form of item 9 from
    the value's type and case, and a decode followed by an encode SHALL yield
    the same typed observation and digest; member order of the JSON affects
    neither. A counterexample witness value text SHALL encode a variant value as
    FR-070's `enum` shape, with `name` the variant type's checked node id and
    `member` the case name.
11. **Identity and its join.** A variant value carries two keys and the
    compiler SHALL join them one to one. In the type environment, during check,
    evaluation and replay, the variant type is keyed by its FR-081 effective
    identity and each case by its own effective identity (FR-056 "Effective
    identity of a variant type"); equality (item 7) and replay admission
    (item 9) use these. In the checked IR and the kernel, the variant type is
    the model-owned node of QSpec FR-322 "Model-owned members" step 2 and each
    case an enum member keyed over that node and the case name (QSpec FR-141);
    emission (item 12) and the kernel enum declaration use these. Both keys are
    functions of the selected document's identity, the type node and the case
    name alone, never of the alias a unit gives the model (FR-359 item 9). The
    assembler SHALL declare, for each variant type, the effective identity and
    the checked node key together in one type-environment declaration, so a case
    has one of each and neither is derived from the other. Two units that select
    the same package under different aliases SHALL agree on every key of both
    kinds, and a value built in one equals the same case built in the other.
12. **Lowering and emission.** The emitter SHALL emit the variant type as a
    `model`/`variant_type` node with a `ModelOwner` (cross-repo requirement 1),
    not as a unit-owned `scalar_type`/`enum` node. A case literal SHALL lower to
    the `value`/`enum_value` node whose `semantic_type` is that node and whose
    case is the literal's case. A read of a variant-typed field SHALL lower to
    the field projection FR-093 gives every field, typed by that node. `=` and
    `!=` SHALL lower to the enumeration equality application FR-093 gives an enum
    comparison. A call passing a variant value takes it by `ref` with no
    conversion node, and the package reads back `Verified`. The node and each
    case node are built where the unit first names the type, and the type's
    cases are every case the model declares, any non-empty name kept exactly as
    written (`Gold Plus`, `2x`, `Café`): a case name is never normalized and
    names that differ in any byte are distinct cases. The runtime enumeration of
    the type holds the same cases.

## Cross-repo requirements

Each owner adds its own specification. QSL does not wait on them to state its
own behaviour above. The rows below are in `agent-ix/quire-specification`; this
requirement cites them by id.

1. **Model-owned variant type node and member identity:** QSpec FR-322-AC-71
   and QSpec FR-141-AC-7 and FR-141-AC-8.
2. **Admission of a variant `typeRef` and of a clause naming the type:** QSpec
   FR-208-AC-15 and QSpec FR-208-AC-16.
3. **Naming and typing of a variant type and its cases in source:** QSpec
   FR-208-AC-17.

The cross-repo test cases are QSpec TC-939 to TC-942.

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
| FR-360-AC-1 | An admitted package with variant type `Tier` (cases `Gold`, `Standard`), object `Account` with field `tier: Tier`, record value type `Settings` with field `tier: Tier`, a state machine with a context field `tier: Tier` and a process with a field `tier: Tier` is admitted with no refusal. Each field has one `field` export keyed as any other field and no `reference` export; `Settings`' `record` export lists `tier`; `Tier`'s `enum` and `variant` exports equal those of the same package with no field naming it. A field `Event.tier` of an event type whose occurrence field is `Tier` refuses `invalid_model_binding`/`malformed-declaration` at the event type node, path `TypeDefinition.occurrenceField`, while a non-occurrence `Event.tier` is admitted. | Test (TC-1006) |
| FR-360-AC-2 | The unit assembled over that package gives `Account.tier` the type `Config::Tier`, an `optional` field `Option<Config::Tier>`, and an ordered non-unique `[0, 3]` field `Sequence<Config::Tier>[0, 3]`, never `Reference<Config::Tier>`; an operation parameter or result naming `Tier` has the same type. The type environment holds, for `Config::Tier` and each case, the effective identity of the view and the checked node key together, and a unit that selects the same package as `Cfg` assigns the same keys of both kinds. | Test (TC-1006) |
| FR-360-AC-3 | A variant `typeRef` whose variant type has no case refuses once, `invalid_model_binding`/`malformed-declaration` at the variant type, path `TypeDefinition.variants`, and the field naming it raises no refusal of its own. | Test (TC-1006) |
| FR-360-AC-4 | With `Config::Tier::Gold` in a function `f(t: Config::Tier): Config::Tier { t }`, a field read `a.tier`, a `let` binding, an `Option` `value`, a collection element and `if c then Config::Tier::Gold else a.tier` each check as type `Config::Tier`; `Config::Tier::Gold` at an `Account.tier` slot, passed to `f` and in an `Option<Config::Tier>` literal checks. A clause may name `Config::Tier` in a type position and `Config::Tier::Gold` as a literal with no `unsupported_construct` refusal. A variant type with a case named `Gold Plus` is admitted; `Config::Tier::Gold` beside it still checks, and the spelling `Config::Tier::Gold Plus` does not parse as a case literal. | Test (TC-1007) |
| FR-360-AC-5 | Each of the following refuses at the expression naming the expected and actual types. `ill_typed`/`type-mismatch`: `Config::Level::Gold` at a `Config::Tier` slot; the Integer literal `0` at a `Config::Tier` slot; a bare `Gold`; a `Config::Tier` at a `Config::Level` parameter; a `Sequence<Config::Tier>` at a `Sequence<Config::Level>` and at `Option<Config::Tier>`; `Config::Tier::Silver` (undeclared case) and `Config::Tier::gold` (letter-case variant). The Text literal `"Gold"` and `none` are refused by S2 as `unsupported_construct` (cause `UnrepresentedConstruct`, production `Primary`) at the literal, and, since FR-313, `case t when { true } true` by S1 as `invalid_syntax` (cause `UnexpectedToken`) at the `when` token (item 5); a well-formed `case` expression over a variant is refused by the checker as `unknown_required_feature`/`unsupported-feature` until FR-318 (item 5). `missing_declaration`/`missing-name` at the name: `Config::Nope::Gold` and `Config::Nope` in a type position. `ill_typed`/`operator-ineligible`: `Config::Tier::Gold < Config::Tier::Standard`, `<=`, `>` and `>=`. | Test (TC-1007) |
| FR-360-AC-6 | Over `Config::Tier`, `=` is true for `Gold = Gold` and false for `Gold = Standard`, `!=` the negation, in either operand order, with no checked-invariant refusal. `Config::Level::Gold = Config::Tier::Gold` refuses `ill_typed`/`type-mismatch` at check and never evaluates. A case read from a field equals the same case written as a literal. | Test (TC-1008) |
| FR-360-AC-7 | Equality reaches a variant value inside other values by the same rule: two `Settings` records equal exactly when `tier` is the same case (and their other fields are equal), two `Option<Config::Tier>` equal exactly when both are `none` or both hold the same case, and two `Sequence<Config::Tier>`, `Set<Config::Tier>` and a tuple position likewise. A `Set<Config::Tier>` built from `Gold, Gold, Standard` has two elements and one built from `Standard, Gold` has the same two, each in the order `Gold`, `Standard` (ascending UTF-8 bytes of the case names); a `Set<Config::Tier>` over a `Tier` with cases `Standard`, `Gold`, `Bronze` orders `Bronze`, `Gold`, `Standard`, and the reverse order fails. | Test (TC-1008) |
| FR-360-AC-8 | An operation parameter of type `Tier` is admitted at intake, its call with `Config::Tier::Gold` checks and runs, and a call with a `Config::Level` value refuses `ill_typed`/`type-mismatch` naming the parameter; an operation result of type `Tier` is admitted and its value compares by case. The checked package emits `Config::Tier` as a `model`/`variant_type` node, the case literal as an `enum_value` of it and the argument by `ref` with no conversion node, and reads back `Verified`, also for a type whose cases include `Gold Plus`, `2x` and `Café`. A parameter or result naming an interface-featured object type keeps `invalid_model_binding`/`malformed-declaration` at the operation node, path `Operation.params[i].typeRef` or `Operation.returns.typeRef`. | Test (TC-1009) |
| FR-360-AC-9 | A replay snapshot whose `Account.tier` holds `{"variant":{"type":"ix://<package identity>/Tier","case":"Gold"}}` admits, and so does the same value as the payload of `{"present": ...}` and as an element of `{"sequence": [...]}` at a `Sequence<Config::Tier>` slot, and as an operation parameter and result. The admitted value equals the literal `Config::Tier::Gold`. | Test (TC-1010) |
| FR-360-AC-10 | Each of these refuses `invalid_runtime_input` at the nested value path: a payload missing `case` (`missing-member`) or `type` (`missing-member`); an extra payload member (`unknown-member`); a `type` naming `Level`, a record, an object or no declaration, a `{"string":"Gold"}` or `{"integer":"0"}` at the slot, a payload that is not an object, and `{"absent":{}}` at a required variant field (`wrong-value-kind`); `"case":"Silver"` and `"case":"gold"` (`invalid-value`). The order of item 9 decides a value with several defects: `{"variant":{"type":"<Level>"}}` is `missing-member` (the missing `case`), not `wrong-value-kind`; `{"variant":{"type":"<Tier>","case":"Silver","x":1}}` is `unknown-member`, not `invalid-value`; `{"variant":{"type":"<Level>","case":"Silver"}}` is `wrong-value-kind`. | Test (TC-1010) |
| FR-360-AC-11 | A decode followed by an encode returns the same typed observation and digest with the object members in either order; the encoded value names its own variant type and case. A counterexample witness holding a `Config::Tier` parameter encodes it as the enum shape with the type's checked node id and the case name and decodes to the same value. Two units that select the package as `Config` and as `Cfg` assign equal keys of both kinds, and a case built in one equals the same case built in the other. | Test (TC-1010) |

## Dependencies

- FR-056 admits the variant type, its exports, its effective identity and the
  type environment declaration; this requirement replaces its placeholder
  refusals for fields, signatures and clauses. QSpec FR-208 defines the variant
  type, and QSpec FR-141 and FR-144 the enumeration equality, unordered
  refusals and unordered key this requirement reuses. FR-359 states the
  record-slot rules it parallels and keeps its alias-independence rule
  (item 11). FR-317 states qualified-name resolution. FR-106 states the replay
  value grammar it extends by one form. FR-093 states the lowering item 12 uses.

## References

- Owning ticket: QSL-738.
