---
id: FR-391
title: "Name a domain-package population as the operand of allInstances and lookup in a state clause"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-391: Name a domain-package population as the operand of allInstances and lookup in a state clause

## Description

`allInstances<T>(p)` and `lookup<T>(p, r)` read an explicit population
binding `p` (QSpec FR-153). A function names one through a
`Population<T>[N]` parameter, but a state clause has no parameters of that
kind, so no clause could quantify over the objects of a model. This
requirement lets a state clause name a population the domain package
declares by the qualified name `M::P`, where `M` is a model alias of the
unit and `P` is a population of `M`'s domain package. The name is the same
`M::member` spelling that `bound` (FR-380) uses for a population, and the
QSpec complete-V1 `primary` already derives it (`qualified-name` without a
call), so S1 and S2 need no new production (FR-091 builds it as a `Name`)
and QSpec's grammar is unchanged.

A population is a binding, not a value. The name therefore denotes the
population only as the population operand of `allInstances` or `lookup`;
it is no operand of any other operator, no `let` initializer and no result.

## Use case

A unit declares `pre Room using v on M::Counter::bump { forall(o in
allInstances<M::Counter>(M::counters) : o.v < 2) }`. The precondition then
ranges over every `Counter` the `counters` population holds, which is the
read that FR-155 derives `AnyMembership{counters}` from.

## Inputs

- A state clause body (FR-102) holding `allInstances<T>(e)` or
  `lookup<T>(e, r)` whose population operand `e` is a `Name` with two
  segments.
- The unit's model aliases and the populations of each alias's admitted
  domain package (FR-084; QSpec FR-153).

## Outputs

- The operand checked as a population of type `Population<U>[N]`, where `U`
  is the population's canonical member type (FR-104, "Requirements") and
  `N` is the maximum FR-380 authors for it, with no bracket when the
  population has none.
- The population as a domain of the clause's requirement record, keyed as
  FR-104 keys a population domain.
- Or a located refusal with a catalog code.

## Behavior

### Resolution

- The assembler SHALL resolve a two-segment `Name` inside a state clause
  body as a population when its first segment is a model alias of the unit
  and its second names a population of that alias's domain package.
- If the first segment is a model alias and the second names no population
  of its domain package, then the assembler SHALL refuse
  `missing_declaration`/`missing-name` at the second segment.
- If the first segment names both a model alias and an enum of the unit,
  then the assembler SHALL refuse `ambiguous_declaration`/`ambiguous-name`
  at the name, naming both.
- Outside a state clause body a `M::P` name resolves as it did before this
  requirement: it names no population, and the population forms of a
  function remain its `Population<T>[N]` parameters.

### Typing

- The checker SHALL type the resolved population as `Population<U>[N]` and
  admit it only as the population operand of `allInstances` or `lookup`.
  Anywhere else it SHALL refuse `ill_typed`/`operator-ineligible` at the
  name: a population is no value.
- The checker SHALL check `T` of `allInstances<T>` and `lookup<T>` as it does
  for a parameter operand (FR-084); the queried type need not be `U`, only
  covered by a member type of the population at evaluation, where an
  uncovered `T` is FR-084's typed incomplete result.
- The result type of `allInstances<T>(M::P)` is `Set<Reference<T>>[0,N]`
  when `P` has an authored maximum `N` (FR-380) and the unbounded
  `Set<Reference<T>>` otherwise, as FR-084 states for a parameter.
- Every read through the operand takes the clause's observation (FR-104,
  "Observations of reads"): `current` in an invariant, `pre` in a
  precondition, `post` in a postcondition and `pre` inside `pre(e)`.

### Lowering and requirements

- The operand lowers as FR-093 lowers the population operand of `AllInstances`
  or `Lookup`, over the `Population<U>[N]` type node FR-094 keys.
- The named population SHALL be a domain of the clause's requirement record
  (FR-104, "Requirements"), keyed as FR-104 keys it. A clause that names its
  context population adds no second domain. A clause that names a different
  population adds that one, so a bound on it (FR-380) makes it a bounded
  domain.
- Naming a population does not select the context population of FR-104; the
  ambiguity rule there still applies to the clause's `on` type.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-391-AC-1 | Over the `Counter` unit (model alias `M`, `Counter` with `v: Int[0, 2]`, operation `bump()`, population `counters`), `pre Room using v on M::Counter::bump { forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }` checks as a `Precondition` with the population operand typed `Population<Counter>`, the `allInstances` result `Set<Reference<Counter>>` and every read at `pre`; the invariant `invariant Cap using v on M::Counter at current { count<M::Counter>(o in allInstances<M::Counter>(M::counters) : true) < 4 }` checks with reads at `current`. | Test (TC-1076) |
| FR-391-AC-2 | `M::nothing` as the operand refuses `missing_declaration`/`missing-name` at `nothing`; an alias `M` that is also an enum name refuses `ambiguous_declaration`/`ambiguous-name` at the name. | Test (TC-1076) |
| FR-391-AC-3 | `M::counters = M::counters`, `let q = M::counters in allInstances<M::Counter>(q)`, `size(M::counters)` and a clause body that is `M::counters` each refuse `ill_typed`/`operator-ineligible` at the name; in a function body `allInstances<M::Counter>(M::counters)` names no population and refuses as an unresolved name did before. | Test (TC-1076) |
| FR-391-AC-4 | With `bound B on M::counters [3]` the operand is `Population<Counter>[3]` and `allInstances<M::Counter>(M::counters)` is `Set<Reference<Counter>>[0,3]`; the record's `counters` domain is `Population(Some(3))`. With a second population `archive` of `Counter`, a clause on `M::Counter` still refuses `ambiguous_declaration`/`ambiguous-name` at its `on` (FR-104-AC-5) whichever population its body names; where only `archive` covers the context type, a body naming `M::counters` records both the context population `archive` and `counters` as domains. | Test (TC-1076) |
| FR-391-AC-5 | Evaluated at S6a over a snapshot whose `counters` holds `a`, `b` and `c`, the precondition of AC-1 decides over exactly those three, in canonical reference-key order; over a snapshot in which `counters` is not marked `complete`, it returns `Incomplete` with `incomplete_population`/`incomplete-scope`, as for a parameter operand; `allInstances<M::Other>(M::counters)` with `Other` uncovered by the population is FR-084's typed incomplete result. | Test (TC-1076) |
| FR-391-AC-6 | FR-155-AC-2's precondition, quantifying over `counters` through this form, yields the read footprint `AnyField{v}` and `AnyMembership{counters}`. | Test (TC-574) |

## Dependencies

- FR-084 (population binding and query results), FR-091 (`Name` forms), FR-093 and
  FR-094 (lowering and type nodes), FR-104 (state clause checking and the
  requirement record), FR-380 (the authored bound).
- QSpec FR-153 and `shared-grammar.md` (`qualified-name` primary,
  `allInstances<T>(p)`).
