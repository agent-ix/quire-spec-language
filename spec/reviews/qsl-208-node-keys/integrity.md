---
id: SR-524
title: "Integrity review of the QSL-208 node-key and expression-lowering requirements"
type: SpecReview
analysis: integrity
scope: "Commit d96591be: FR-092 and FR-093 against QSpec FR-322 and proposals/checked-package-v2 at e72756f, check::ir::NodeKind, and the FR-065, FR-091, ADR-011 and ADR-013 edits"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
  - target: ix://agent-ix/quire-specification/FR-322
    type: references
---

## Summary

What matches the ground truth:

- **Operator classes and semantic forms.** FR-093's operator-class to
  `semantic_form` list matches `expression_form_for_class` in QSpec
  `tests/checked_package_v2.rs:1066-1083`.
- **Application rows.** Every row's `operation.identity`, `operator`, mode
  kind, member kind and law roles match `operation-catalog.json`. That covers
  `decimal.*` with `rounding`, `text.*` with `text_profile`, `ieee.*` with
  `ieee_profile`, `model.lookup` with `absence`, and the `type_argument`
  members of `count`, `sum.integer`, `fold`, `reduce`, `size` and the
  conversions.
- **NodeKind coverage.** Every `check::ir::NodeKind` variant (39) has a row or
  a prose rule. `Local` is covered by prose.
- **Type-node spellings.** The bounded-domain binding names match the
  positive operation fixture's nodes: `integer_range`, `rational_range`,
  `decimal_range` and `text_bounds`.
- **Call shape.** The fixture's `function.call` puts the function reference
  first, and E2 does the same.
- **Removed wording.** The "owner member on the `quire.application-node/v1`
  preimage" wording is gone from FR-065, FR-091 and ADR-013 O-04 and QC-18.
  O-04, QC-18, QC-24, TK-08, the ADR-011 To-QSpec list and the module map
  agree on the split: structural-node keys for every node whose body holds no
  application, and undeclared expression nodes for applications.
- **Status claims.** FR-093's Status claims are accurate:
  `LiteralValue::Integer(i64)` at `application_key.rs:200` and
  `ProjectionNotYetImplemented` in `emit.rs`.

Four statements contradict QSpec or the checker:

- the parameter node's `declaration`;
- `ConvertScalar` over quantities;
- the Float type nodes the IEEE rows need;
- CON-2 against the emitter's own `package_id`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-093 gives a parameter node a `declaration` occurrence at its binder site. FR-092 gives the same node `declaration: null`. FR-322 says `declaration` "is carried exactly by … a `value` node other than an `enum_value`, that has a `declaration` source occurrence". A parameter is a `value` node, so an FR-322 reader requires the member. A parameter node is also shared by every function that binds the same name, type and level (P1 is bound by `both`, `nb` and `h`), so it cannot carry any single `qualified_name`. Fix: give the binder site a role other than `declaration` (`anchor` or `generated`, the FR-322 roles). Otherwise, add to QC-24 a request that `value`/`parameter` nodes are exempt from the `declaration` rule. | FR-093:71-72; FR-092:162; QSpec FR-322 `declaration` (FR-322.md:160-166 at e72756f) |
| FND-002 | high | FR-093 maps every `ConvertScalar` to `quire.op.numeric.convert`. `admits_equality_conversion` admits `Quantity` to `Quantity` when the units convert, and it admits a same-type source, which includes `Boolean` and `Text`. `numeric.convert`'s operand and member family is `exact_numeric`. A quantity unit conversion is `quire.op.quantity.convert`, which requires a `rounding` mode. The lowered node would therefore be refused as `operator-ineligible`. This also covers every quantity equality whose operands have different compatible units. Fix: add a `ConvertScalar` row per source family: `quantity.convert` with `mode rounding` for a quantity. For an identical type, state that no node is built, or that the checker never emits `ConvertScalar`. | FR-093:118; `qsl-semantics/src/value/declaration.rs:1306-1360`; `check/check.rs:2076-2090`; catalog `quire.op.quantity.convert` |
| FND-003 | medium | FR-093's `Ieee`, `IeeeToRational` and IEEE `result_type` rows need a type node for `ValueType::Float(IeeeWidth)`. FR-092's type table has no Float row. The fixture's forms are `scalar_type`/`float32`, `float64` and `bounded_domain`/`float_rounding` with a `rounding` binding. FR-092-CON-2 requires the type-node match to be exhaustive with no `_` arm, so the implementer has nothing to write in that arm. Fix: add the `float32`/`float64` scalar rows and the `float_rounding` bounded row with the fixture's spelling. Alternatively, state that the Float arm refuses with a named code while FR-091-AC-19 refuses floating types. | FR-092:118-131, :402; FR-093:114, :120; `quire-exact/src/value.rs:187`; QSpec `fixtures/positive-operation-identities.json` (float nodes) |
| FND-004 | medium | FR-093-CON-2 says the `package` crate "calls no preimage or key function". The v2 emitter, which is in `qsl-package` (ADR-011 module map), must compute `package_id` over the `identity_preimage`, and the I2 reader must recompute it (ADR-013 T-2). Read literally, CON-2 forbids both. Fix: narrow CON-2 to "no node-key preimage", meaning FR-092's structural-node and FR-322's application-node builders, and no `NodeKey` constructor. | FR-093:169; ADR-011:830; ADR-013 T-2 (:906) |
| FND-005 | medium | FR-093's lead rule says `laws`, `mode`, `member` and `leaves` are "exactly those the catalog entry names". The catalog names a member kind and a leaf source (`operand:0`, `inner:0`, `result_inner`), not values. FR-322 derives `leaves` as one `{path, laws, mode}` per text leaf, and for `result_inner` only when the result is a `set`, `bag` or `ordered_set`. So the rows' "leaves `result_inner`" does not say what to build: `map` to a `sequence` has `leaves: []`. It also does not say that each text leaf needs a lock-supplied `text_profile` law, which is refused when missing, as in AC-6. Fix: define leaves as FR-322 does, and extend the missing-law refusal to leaf laws. | FR-093:94-97, :121, :126-127, :132, :136; FR-322.md:108-114 |
| FND-006 | medium | The `Attribute` row lowers `deref(r).f` to `quire.op.model.deref` followed by `quire.op.record.project`, with `member field{declaration: the object type's node}`. `record.project`'s operand family is `record` and its constraint is `member_of`. FR-322 gives a model entity type the family `reference`, and the object type is a `model` node. No fixture or QC-24 entry covers a projection over a `deref` result, so the row may be refused as `operator-ineligible`. Fix: confirm the operand family of a `deref` result against FR-322 and add a QC-24 item if needed. | FR-093:137; FR-322.md:232-241; catalog `quire.op.record.project`, `quire.op.model.deref` |
| FND-007 | low | FR-092 quotes ADR-013 O-04 as "QSL implements a proposed one, specified with the type-node key work". This commit removes that sentence from O-04. Fix: cite the new O-04 sentence ("Every other node whose body holds no application … is keyed by QSL's proposed `quire.structural-node/v1` preimage"). | FR-092:32-33; ADR-013:193 |
| FND-008 | low | ADR-011 OQ-7 and ADR-013 OQ-G still give the reopen condition as "QSpec rejects the QC-18 owner-member request". QC-18 now asks for the structural-node arm, and the application-node owner member is withdrawn. Fix: "QSpec rejects the QC-18 structural-node arm". FR-091-OQ-3's Reason ("extending it to `composite_type` and `function` nodes") should also name the structural-node preimage. | ADR-011:1318-1325; ADR-013:1162; FR-091:554 |
| FND-009 | low | FR-092's `owner` is always a `SourceOwner`. ADR-013 O-04 also has `DefinitionOwner` declared nodes, and ADR-013 says O-04 "covers the record, tuple and function nodes QSL compiles from source". Fix: give `owner` as the QSpec `Owner` union and state which declarations reach it, or add a sentence that only source-declared nodes are keyed by this preimage. | FR-092:46, :80; ADR-013:193 |
| FND-010 | low | The JSON-number rule lists `recursion.size`, `recursion.ordinal` and `operation.member.position`. It omits `group_reference.ordinal`, which is also an integer in the schema. Fix: add it to the list. | FR-092:94-95; QSpec `node-identity-preimage.schema.json` `PreimageTerm` group_reference |

## Resolution

All findings are fixed in the PR. FND-001: a binder site is an `anchor`
occurrence of its parameter node, which carries no `declaration`; QC-24
records it. FND-002: `ConvertScalar` has one row per source family
(`numeric.convert`, `quantity.convert` with its rounding mode), and a
same-type conversion builds no node. FND-003: `Float32[mode]` and
`Float64[mode]` rows follow the fixture's `float_rounding` spelling.
FND-004: FR-093-CON-2 names node-identity preimages and the `NodeKey`
constructor and excludes `package_id`. FND-005: leaves follow FR-322's rule,
and a leaf's missing law refuses too. FND-006: the `Attribute` row is recorded
as a QSL proposal in QC-24 and in FR-093's Dependencies. FND-007 to FND-010:
applied as proposed; `owner` is the QSpec `Owner`.
