---
id: FR-089
title: "Carry population identity across the kernel boundary as an opaque PopulationId"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-exact/FR-089
    type: depends_on
---
# FR-089: Carry population identity across the kernel boundary as an opaque PopulationId

## Description

The `quire-exact` kernel `Value` type SHALL carry a `Population` variant
whose payload is an opaque `PopulationId` key, never a `PopulationBinding`
or any other `model::population` state. QSL `model` SHALL mint a
`PopulationId` at binding-admission time and SHALL record the
`PopulationId` → `PopulationBinding` correspondence it mints. The QSL
evaluator SHALL resolve a `Value::Population(PopulationId)` to its
`PopulationBinding` by lookup in that recorded correspondence, and SHALL
refuse a `PopulationId` absent from it, never by decoding the identity into
a binding or by carrying the binding inside the kernel value.

This requirement is ADR-013 O-13's Population row (QC-21), authored to
close an AD-016 kernel-row gap: `ValueType::Population(u64)`
admits no kernel `Value` variant today, even though `Value::Population` is a
live runtime value with a real producer
([`model::population::admit_binding`](../../src/model/population.rs),
[`admit_invocation`]) and a real consumer (the evaluator,
[`value::expression::evaluate`](../../qsl-eval/src/value/expression/evaluate.rs)).
Deleting `Value::Population` would remove that evaluation path; moving
`PopulationBinding` into the kernel would violate ADR-011 §6.1's K-leaf rule.
This requirement follows the O-14 `VariantId` precedent instead: the kernel
carries an opaque identity, and `model` keeps the binding.

A `PopulationId` is the structural node key of the population declaration
that `population_key` names, so two bindings that differ only in members,
declared maximum, admission role or pre binding share an id, and
`ObjectEnvironment::with_population` refuses the second, unequal one with
`PopulationConflict`.

## Inputs

- An admitted `PopulationDocument` and the domain package, view,
  `population_key`, closure and declared maximum that
  [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md)'s
  `admit_binding`/`admit_invocation` already take.
- For `admit_invocation`: the pre binding it may attach as `pre_anchor`.
- A `Value::Population(PopulationId)` value the evaluator's task stack holds.

## Outputs

- A minted `PopulationId` for every successfully admitted `PopulationBinding`,
  and the recorded `PopulationId` → `PopulationBinding` correspondence.
- A `Value::Population(PopulationId)` produced wherever the evaluator today
  produces `Value::Population(Arc<PopulationBinding>)`.
- The resolved `PopulationBinding` for a `PopulationId` the evaluator
  consumes, or a typed refusal when the identity is not recorded.

## Behavior

### PopulationId is the population declaration's node key

`model` SHALL compute a binding's `PopulationId` as
`PopulationId::new(population_node_key(population_key))`: the structural
node key `{package, path: [relation, population, <population_key>],
ordinal: 0}` of the population declaration `population_key` names, with
`package` the domain package declaring it. The binding's members, declared
maximum, admission role (`Direct`, `Pre` or `Post`) and pre binding SHALL
NOT enter it, and no byte string is encoded for it. Two admissions against
one `population_key` therefore share a `PopulationId`, and admissions
against different `population_key`s do not.

A binding's field values are not part of its identity: a
`PopulationBinding` carries membership only, and field values belong to the
`ObjectEnvironment` entries its references key.
`ObjectEnvironment::with_population` keeps its typed `PopulationConflict`
refusal for a second binding, unequal to the first, recorded under an id
already recorded; an equal re-admission is accepted. `PopulationId`'s
constructor takes the node key, is defined in `quire-exact` and is called
only from QSL `model`, mirroring `EffectiveId`'s O-05 constructor
discipline.

### Kernel `Value::Population` carries the identity only

Kernel `Value` SHALL gain a `Population(PopulationId)` variant. Neither this
variant nor its construction or destructuring SHALL require the kernel crate
to depend on any `model` type: it carries `PopulationId` alone, never a
`PopulationBinding`, a domain package, or any other `model::population`
state.

### The evaluator resolves identity through the recorded correspondence

`model` SHALL record the `PopulationId` → `PopulationBinding` correspondence
for every binding it admits. Wherever the evaluator today matches
`Value::Population(binding)` and reads `binding` directly
(`qsl-eval/src/value/expression/evaluate.rs:921,934`), it SHALL instead match
`Value::Population(population_id)` and resolve the `PopulationBinding` by
lookup of `population_id` in that recorded correspondence. A `population_id`
absent from the correspondence SHALL produce a typed evaluator refusal,
never a panic, an `Undefined` outcome, or a silently substituted default
binding.

### The type pairing is a QSL-layer check

The QSL layer SHALL treat `ValueType::Population(maximum)` as admitting
exactly the `Value::Population(population_id)` values whose resolved
binding's own declared maximum equals `maximum`: QSL `model`/the evaluator
resolves `population_id` to its `PopulationBinding` through the recorded
correspondence, then compares that binding's declared maximum with `maximum`.
QSL performs this pairing in argument admission (`value::expression::validate`,
`qsl-eval/src/value/expression/mod.rs`), before the kernel `ValueType::admits` runs. The
kernel `admits` refuses every `(Population, Population)` pair (FR-089-AC-6 of
`agent-ix/quire-exact`).

The kernel is a leaf under ADR-011 §6.1's K-leaf rule: the
`PopulationId` → `PopulationBinding` correspondence and every binding's
declared maximum live in `model`. Kernel `ValueType::admits` SHALL return `false` for every
`(ValueType::Population(_), Value::Population(_))` pair. The kernel's
`equality::plan_pairs` SHALL refuse a pair of `Value::Population` operands
with `Refusal::CheckedInvariant`, and `key::compare_keys` SHALL yield no key
(`None`) for a pair of `Value::Population` values: a population is neither an
equality operand nor a key participant in the kernel. QSL has no equality or
key of its own: it calls `quire_exact::member_equal` and
`quire_exact::compare_keys`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-089-AC-1 | Given two `Direct` admissions of the same `PopulationDocument` against the same `population_key`, both admissions mint the same `PopulationId`; given a second admission against a different `population_key`, the minted `PopulationId` differs from the first. | Test (TC-291, TC-296) |
| FR-089-AC-3 | Given a `Value::Population(population_id)` whose `population_id` was minted for an admitted binding earlier in the same evaluation, evaluating an expression that consumes it (the `evaluate.rs:921`/`:934` sites) resolves the same `PopulationBinding` that admission produced, by lookup in the recorded correspondence, never by a payload the `Value` itself carries. | Test (TC-293) |
| FR-089-AC-4 | Given a `Value::Population(population_id)` whose `population_id` names no binding recorded in the current evaluation's correspondence, evaluation produces a typed refusal naming the unresolved identity, not a panic and not `Undefined`. | Test (TC-294) |
| FR-089-AC-5 | Given a `ValueType::Population(maximum)` and a `Value::Population(population_id)`, the QSL layer resolves `population_id` to its `PopulationBinding` through the recorded correspondence and admits the value when that binding's declared maximum equals `maximum`, and refuses it when the declared maximum differs. | Test (TC-295) |
| FR-089-AC-7 | The `PopulationId` of an admission against `population_key` `k` of domain package `p` is the node key `p:[relation,population,k]:0` of the population declaration. Two `Direct` admissions that differ only in one member, two that differ only in the declared maximum (3 and 7), and two `Post` bindings with equal post members whose pre bindings differ each mint equal `PopulationId`s: members, maximum and admission role do not enter it. | Test (TC-291) |

## Dependencies

- **Upstream:** [FR-084](FR-084-admit-closed-populations-and-resolve-lookup.md)
  owns `admit_binding`/`admit_invocation` and the `PopulationBinding` these
  criteria mint identities for and resolve; this requirement adds no new
  admission behavior, only the identity that crosses the K boundary.
  [ADR-013](../decisions/ADR-013-canonical-type-package-conversion-ownership.md)
  O-13's Population row (QC-21) states the target design this requirement
  carries testable criteria for.
- **Downstream:** none within this ticket's scope.
- quire-specification's AD-016 kernel row is amended to match (QC-21), by an
  issue filed against agent-ix/quire-specification, not by this repository.
