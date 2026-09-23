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
---
# FR-089: Carry population identity across the kernel boundary as an opaque PopulationId

## Description

The `quire-exact` kernel `Value` type SHALL carry a `Population` variant
whose payload is an opaque `PopulationId` digest, never a `PopulationBinding`
or any other `model::population` state. QSL `model` SHALL mint a
`PopulationId` at binding-admission time and SHALL record the
`PopulationId` → `PopulationBinding` correspondence it mints. The QSL
evaluator SHALL resolve a `Value::Population(PopulationId)` to its
`PopulationBinding` by lookup in that recorded correspondence, and SHALL
refuse a `PopulationId` absent from it, never by decoding the identity into
a binding or by carrying the binding inside the kernel value.

This requirement is ADR-013 O-13's Population row (QC-21), authored under
QSL-172 to close an AD-016 kernel-row gap: `ValueType::Population(u64)`
admits no kernel `Value` variant today, even though `Value::Population` is a
live runtime value with a real producer
([`model::population::admit_binding`](../../src/model/population.rs),
[`admit_invocation`]) and a real consumer (the evaluator,
[`value::expression::evaluate`](../../src/value/expression/evaluate.rs)).
Deleting `Value::Population` would remove that evaluation path; moving
`PopulationBinding` into the kernel would violate ADR-011 §6.1's K-leaf rule.
This requirement follows the O-14 `VariantId` precedent instead: the kernel
carries an opaque identity, and `model` keeps the binding.

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

### PopulationId is opaque and content-addressed over the admission

`model` SHALL compute a binding's `PopulationId` as a 32-byte digest over the
binding's own preimage: the domain package identity it was admitted against,
the `population_key` declaring it, and a closed three-state admission-role
discriminator applied to every admission, not only the ones
`admit_invocation` attaches: `Direct` for a binding `admit_binding` mints
standalone, and `Pre`/`Post` for the two bindings `admit_invocation`
attaches to one invocation frame. `admit_invocation` SHALL tag its own two
constituent bindings `Pre` and `Post`; a directly-called `admit_binding`
SHALL mint `Direct`. Two admissions inside one evaluation that share every
one of these facts SHALL share a `PopulationId`; two admissions that differ
in any of them SHALL NOT collide -- including a standalone `Direct`
admission and an invocation's `Post` binding that share the same domain
package and `population_key`, which would otherwise carry an identical
preimage under a two-state (`pre`/`post`-only) discriminator.
`PopulationId` SHALL have exactly one public constructor, taking this
preimage digest, defined in `quire-exact` and callable only from QSL
`model` -- mirroring `EffectiveId`'s O-05 constructor discipline (ADR-011
T-12's API-surface check enforces both).

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
(`src/value/expression/evaluate.rs:921,934`), it SHALL instead match
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
`src/value/expression/mod.rs`), before the kernel `ValueType::admits` runs. The
kernel `admits` refuses every `(Population, Population)` pair (FR-089-AC-6).

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
| FR-089-AC-1 | Given two `Direct` admissions of the same `PopulationDocument` against the same `population_key` and domain package identity, both admissions mint the same `PopulationId`; given a second admission that differs in `population_key`, domain package identity, or admission-role (`Direct`, `Pre` or `Post`), the minted `PopulationId` differs from the first; in particular, a standalone `Direct` admission and an `admit_invocation`-attached `Post` binding that share the same domain package and `population_key` mint distinct identities. | Test (TC-291, TC-296) |
| FR-089-AC-2 | The kernel `Value::Population` variant's payload type is `PopulationId`; no kernel source file imports `PopulationBinding` or any other `model::population` type to define, construct or match this variant. | Inspection (TC-292): crate DAG, `quire-exact/Cargo.toml` has no workspace dependency (ADR-011 §6.1) |
| FR-089-AC-3 | Given a `Value::Population(population_id)` whose `population_id` was minted for an admitted binding earlier in the same evaluation, evaluating an expression that consumes it (the `evaluate.rs:921`/`:934` sites) resolves the same `PopulationBinding` that admission produced, by lookup in the recorded correspondence, never by a payload the `Value` itself carries. | Test (TC-293) |
| FR-089-AC-4 | Given a `Value::Population(population_id)` whose `population_id` names no binding recorded in the current evaluation's correspondence, evaluation produces a typed refusal naming the unresolved identity, not a panic and not `Undefined`. | Test (TC-294) |
| FR-089-AC-5 | Given a `ValueType::Population(maximum)` and a `Value::Population(population_id)`, the QSL layer resolves `population_id` to its `PopulationBinding` through the recorded correspondence and admits the value when that binding's declared maximum equals `maximum`, and refuses it when the declared maximum differs. | Test (TC-295) |
| FR-089-AC-6 | Given any `ValueType::Population(maximum)` and any `Value::Population(population_id)`, kernel `ValueType::admits` returns false; given two `Value::Population` operands, kernel `equality::plan_pairs` returns `Err(Refusal::CheckedInvariant)` and kernel `key::compare_keys` returns `None`. | Test (TC-297) |

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

## Status

Specified under QSL-172, which found the gap, and ADR-013 O-13's Population
row, which decided it. QSL-131 Slice B (kernel half) implements
`PopulationId` and `Value::Population(PopulationId)` in `quire-exact`
(FR-089-AC-2; TC-292), with kernel `admits`, `plan_pairs` and `compare_keys`
refusing a population pair (FR-089-AC-6; TC-297). QSL-131 Slice B's other
half (`model` minting and the evaluator's resolution step) implements
FR-089-AC-1, FR-089-AC-3, FR-089-AC-4 and FR-089-AC-5 (TC-291, TC-293,
TC-294, TC-295, TC-296; all `✅ Passed locally`): `model::population::
mint_population_id` mints a `PopulationId` at admission time
(`admit_binding`/`admit_invocation`), `ObjectEnvironment` records the
`PopulationId` -> `PopulationBinding` correspondence
(`with_population`/`resolve_population`), and `CheckedPackage::call`/
`evaluate`'s own argument-admission `validate` (`src/value/expression/
mod.rs`) resolves and compares the declared maximum for every
`Population<T>[N]` parameter, refusing an unresolved identity or a
mismatched maximum whether or not the checked body consumes it; the
evaluator's own `Machine::resolve_population` (`src/value/expression/
evaluate.rs`) performs the identical resolution at its `allInstances`/
`lookup` consumption sites, kept as defence in depth.

FR-089's own admission preimage (domain package selection, `population_key`,
admission role) does not distinguish two bindings that differ only in
document content or declared maximum, admitted under the same
package/key/role within one evaluation -- an open spec question (Linear
QSL-131) this Slice records rather than resolves.
`ObjectEnvironment::with_population` is the interim guard: it refuses to
record a second, unequal binding under an id already bound, rather than
silently letting the later admission overwrite the earlier one.
