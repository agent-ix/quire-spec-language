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
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-391: Name a domain-package population as the operand of allInstances and lookup in a state clause

## Description

`allInstances<T>(p)` and `lookup<T>(p, r)` read an explicit population
binding `p` (QSpec FR-153). A function names one through a
`Population<T>[N]` parameter, but a state clause has no such operand: the
clause checker and the clause evaluator admit neither form today. This
requirement lets a state clause name a population the domain package
declares by the qualified name `M::P`, where `M` is a model alias of the
unit and `P` is a population of `M`'s domain package. The name is the same
`M::member` spelling that `bound` (FR-380) uses for a population.

"QSpec unchanged" holds for syntax only: the complete-V1 `primary` already
derives `qualified-name` without a call, and FR-091 builds it as a `Name`,
so S1 and S2 need no new production. The meaning is QSL's own. QSpec
FR-153 and `shared-grammar.md` state that no population binding is ambient,
and QSpec FR-383-AC-13 still writes `forall o in counters`. Here the binding
of `M::P` comes from the admitted observation (FR-106), not from the
environment. QSpec needs a follow-up that states this name-to-binding rule and
the spelling of a population in a state clause, so the two specifications agree.

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

### Lowering

- The operand lowers to a population-reference operand term that carries the
  population's declaration key (package, then node, each as UTF-8 bytes), in
  the slot where FR-093's `AllInstances` and `Lookup` rows take `ref(p)`. A
  named domain population has no binder to reference and the
  `Population<U>[N]` type node is anonymous, keyed by `(U, N)` alone (FR-094),
  so the type node cannot identify the population. `allInstances<Counter>(M::counters)`
  and `allInstances<Counter>(M::archive)` therefore check to different terms.
  The IR encoding of the population-reference term (an FR-322 extension) is
  tracked as a follow-up ticket; this requirement fixes only that the checked
  term carries the declaration key. Until that encoding exists, package
  emission (S4) SHALL refuse a unit whose checked clause contains the term
  with `unsupported_construct`/`not-yet-implemented`, as for any other
  form it does not yet emit.

### Evaluation

- The `ProtocolClause` evaluator SHALL evaluate `allInstances` and `lookup`
  in a state clause (FR-107), reading the population named by the operand
  term's declaration key from the snapshot of the clause's observation: the
  `pre` snapshot for `pre` and the `post` snapshot for `post` (a `current`
  invariant reads the current snapshot), and under `pre(e)` the `pre`
  snapshot. The snapshot's population row whose `population` is that key's
  identity is the binding. FR-106 requires it (FR-106, "required
  population"), so a population that is not marked `complete` returns
  `Incomplete` with `incomplete_population`/`incomplete-scope` before
  evaluation, and an admitted one is a closed binding in FR-084's sense.
- The result, the coverage check of `T` and the `population.visit` charges
  per member are FR-084's and QSpec's `value-accounting.md`, unchanged: the
  coverage check charges nothing, each member visited charges once.

### Requirements

- The named population SHALL be a domain of the clause's requirement record
  (FR-104, "Requirements"), keyed as FR-104 keys it. A clause that names its
  context population adds no second domain. A clause that names a different
  population adds that one, so a bound on it (FR-380) makes it a bounded
  domain.
- Naming a population does not select the context population of FR-104; the
  ambiguity rule there still applies to the clause's `on` type.
- The footprint (FR-155) of a clause that names `P` SHALL hold
  `AnyMembership{P}` for that read: the named population, and no other
  population of the same member type. With populations `counters` and
  `archive` of `Counter`, a clause naming `M::counters` reads
  `AnyMembership{counters}` only and a clause naming `M::archive` reads
  `AnyMembership{archive}` only.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-391-AC-1 | Over the `Counter` unit (model alias `M`, `Counter` with `v: Int[0, 2]`, operation `bump()`, population `counters`), `pre Room using v on M::Counter::bump { forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }` checks as a `Precondition` with the population operand typed `Population<Counter>`, the `allInstances` result `Set<Reference<Counter>>` and every read at `pre`; the invariant `invariant Cap using v on M::Counter at current { count<M::Counter>(o in allInstances<M::Counter>(M::counters) : true) < 4 }` checks with reads at `current`. | Test (TC-1076) |
| FR-391-AC-2 | `M::nothing` as the operand refuses `missing_declaration`/`missing-name` at `nothing`; an alias `M` that is also an enum name refuses `ambiguous_declaration`/`ambiguous-name` at the name. | Test (TC-1076) |
| FR-391-AC-3 | `M::counters = M::counters`, `let q = M::counters in allInstances<M::Counter>(q)`, `size(M::counters)` and a clause body that is `M::counters` each refuse `ill_typed`/`operator-ineligible` at the name; in a function body `allInstances<M::Counter>(M::counters)` names no population and refuses `missing_declaration`/`missing-name`, as does a type name `M::Counter` used as a value, because only populations resolve as a two-segment name there. | Test (TC-1076) |
| FR-391-AC-4 | With `bound B on M::counters [3]` the operand is `Population<Counter>[3]` and `allInstances<M::Counter>(M::counters)` is `Set<Reference<Counter>>[0,3]`; the record's `counters` domain is `Population(Some(3))`. With a second population `archive` of `Counter`, a clause on `M::Counter` refuses `ambiguous_declaration`/`ambiguous-name` at its `on` (FR-104-AC-5) whichever population its body names. With a type `Widget` that only a population `widgets` covers, a clause on `M::Widget` whose body names `M::counters` records `widgets` (the context population) and `counters` as its two domains. | Test (TC-1076) |
| FR-391-AC-5 | Evaluated at S6a over a snapshot whose `counters` holds `a`, `b` and `c`, the precondition of AC-1 decides over exactly those three, in canonical reference-key order; over a snapshot in which `counters` is not marked `complete`, it returns `Incomplete` with `incomplete_population`/`incomplete-scope`, as for a parameter operand; `allInstances<M::Other>(M::counters)` with `Other` uncovered by the population is FR-084's typed incomplete result. | Test (TC-1076) |
| FR-391-AC-6 | With populations `counters` and `archive` of `Counter` and an operation `grow` on `Widget`, a type covered only by `widgets`, the preconditions `pre A using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::counters) : true) }` and `pre B using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::archive) : true) }` check to two different terms, each carrying its own population's declaration key, and of the populations `A` reads `AnyMembership{counters}` and not `AnyMembership{archive}`, and `B` the reverse (each footprint also holds the receiver's own `Membership` per FR-155); the precondition of FR-155-AC-2 over `counters` holds `AnyField{v}` and `AnyMembership{counters}`. A package with such a clause refuses at emission with `unsupported_construct`/`not-yet-implemented` until the IR encoding exists. | Test (TC-1076) |

## Dependencies

- FR-106 (required populations) and FR-107 (clause evaluation), which this requirement amends; FR-084 (population binding and query results), FR-091 (`Name` forms), FR-093 and
  FR-094 (lowering and type nodes), FR-104 (state clause checking and the
  requirement record), FR-380 (the authored bound).
- QSpec FR-153 and `shared-grammar.md` (`qualified-name` primary,
  `allInstances<T>(p)`).
