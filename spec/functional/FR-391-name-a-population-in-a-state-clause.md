---
id: FR-391
title: "Name a domain-package population in a state clause or a holds atom"
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
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-391: Name a domain-package population in a state clause or a holds atom

## Description

`allInstances<T>(p)` and `lookup<T>(p, r)` read an explicit population
binding `p` (QSpec FR-153). A function names one through a
`Population<T>[N]` parameter. A **population position** is the body of a
unit-level state clause (an invariant, a precondition, a postcondition or a
step's clause, FR-102) or a `holds` atom of a temporal clause. The invariant
of a record value type or an event type (FR-366) is not a population
position: it keeps FR-366's refusal of a population read, since such a value
is evaluated with no object universe. In a population
position this requirement lets a clause name a population the domain package
declares by the qualified name `M::P`, where `M` is a model alias of the unit
and `P` is a population of `M`'s domain package. The name is the same
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
population only where a population is read: as the population operand of
`allInstances` or `lookup`, and as the domain of a quantifier or of a
population form (`count`, `sum`, `exists`, `all`, `only`), in a population
position. It is no operand of any other operator, no `let` initializer and no
result. A function body is not a population position: a function names a
population only through a `Population<T>[N]` parameter, so no population is
ambient in a function.

Inside a refinement declaration FR-139 governs population names and forms;
in a population position FR-391 does, with the typing and evaluation of the
forms that FR-139 states.

A `holds` atom evaluates at a trace state exactly as a state clause evaluates
at that state: it reads the state's current snapshot, evaluates by the clause
evaluator, and its requirement record and footprint gain the population as a
state clause's do (Requirements). An atom whose population is not marked
`complete` is `Incomplete` for a domain read, as for an operand.

## Use case

A unit declares `pre Room using v on M::Counter::bump { forall(o in
allInstances<M::Counter>(M::counters) : o.v < 2) }`. The precondition then
ranges over every `Counter` the `counters` population holds, which is the
read that FR-155 derives `AnyMembership{counters}` from. The same population
read as a domain, `forall(o in M::counters : o.v < 2)`, means the same.

## Inputs

- A population position holding `allInstances<T>(e)` or `lookup<T>(e, r)`
  whose population operand `e` is a `Name` with two segments, or a quantifier
  or population form whose domain is such a `Name`.
- The unit's model aliases and the populations of each alias's admitted
  domain package (FR-084; QSpec FR-153).

## Outputs

- The name checked as a population of type `Population<U>[N]`, where `U`
  is the population's canonical member type (FR-104, "Requirements") and
  `N` is the maximum FR-380 authors for it, with no bracket when the
  population has none; as a domain it reads as `Set<Reference<U>>[0,N]`.
- The population as a domain of the clause's requirement record, keyed as
  FR-104 keys a population domain.
- Or a located refusal with a catalog code.

## Behavior

### Resolution

- The assembler SHALL resolve a two-segment `Name` in a population position
  as a population when its first segment is a model alias of the unit and its
  second names a population of that alias's domain package.
- If the first segment is a model alias and the second names no population
  of its domain package (a type such as `M::Counter` included), then the
  assembler SHALL refuse `missing_declaration`/`missing-name` at the second
  segment.
- If the first segment names both a model alias and an enum of the unit,
  then the assembler SHALL refuse `ambiguous_declaration`/`ambiguous-name`
  at the name, naming both.
- The assembler SHALL refuse a two-segment `M::P` name in a function body
  or in the type-level invariant of an object type, a state machine or a
  process (FR-366) `missing_declaration`/`missing-name` at the second
  segment, since only a population position resolves one. In a declaration header or an `on`
  target, `M::X` names the type or operation of the alias's domain package.

### Typing

- The checker SHALL type the resolved population as `Population<U>[N]` and
  admit it as the population operand of `allInstances` or `lookup`, and as
  the domain of a quantifier or of `count`, `sum`, `exists`, `all` or `only`,
  where it reads as the set of the population's objects, typed
  `Set<Reference<U>>[0,N]` (unbounded when `P` has no authored maximum).
  Anywhere else in a population position it SHALL refuse
  `ill_typed`/`operator-ineligible` at the name: a population is no value.
- In place of FR-139's universe size `n`, the bound of a `count` over a named
  population is `[0, N]` and of a `sum` with summand bounded by `[lo, hi]` is
  `[min(0, N*lo), max(0, N*hi)]`, for the authored maximum `N` (FR-380); with
  no authored maximum the result is unbounded above (and below, for a sum).
- The checker SHALL refuse `seq` in a clause outside a refinement
  declaration `unsupported_construct`/`expression-form` at its span: it ranges
  over integers, has no population domain and is not a population form here.
- The checker SHALL check `T` of `allInstances<T>` and `lookup<T>` as it does
  for a parameter operand (FR-084); the queried type need not be `U`, only
  covered by a member type of the population at evaluation, where an
  uncovered `T` is FR-084's typed incomplete result.
- The result type of `allInstances<T>(M::P)` is `Set<Reference<T>>[0,N]`
  when `P` has an authored maximum `N` (FR-380) and the unbounded
  `Set<Reference<T>>` otherwise, as FR-084 states for a parameter.
- Every read through the name takes the clause's observation (FR-104,
  "Observations of reads"): `current` in an invariant and in a `holds` atom,
  `pre` in a precondition, `post` in a postcondition and `pre` inside
  `pre(e)`.

### Lowering

- A population as a domain lowers and evaluates as `allInstances<U>(M::P)`
  for the canonical member type `U`: it takes FR-084's coverage and incomplete
  rules, and a population that is not `complete` is `Incomplete` for the
  domain read.
- The operand lowers to a population-reference operand term that carries the
  population's declaration key (package, then node, each as UTF-8 bytes), in
  the slot where FR-093's `AllInstances` and `Lookup` rows take `ref(p)`. A
  named domain population has no binder to reference and the
  `Population<U>[N]` type node is anonymous, keyed by `(U, N)` alone (FR-094),
  so the type node cannot identify the population. `allInstances<Counter>(M::counters)`
  and `allInstances<Counter>(M::archive)` therefore check to different terms.
  Package emission (S4) SHALL emit the term as QSpec FR-322's population
  reference, the `relation`/`population` node keyed by the declaration key
  alone, as operand 0 of the `quire.op.model.all_instances` or
  `quire.op.model.lookup` application, with the authored maximum `N` entering
  through the application's `result_type` and not through the population
  reference. A population read as a domain emits the population reference, the
  `all_instances` application over it and the quantifier or population form
  over that application, as FR-322 "Quantifier domain" states.

### Evaluation

- The `ProtocolClause` evaluator SHALL evaluate `allInstances`, `lookup` and
  a population domain in a state clause (FR-107) and in a `holds` atom, reading the population named by the operand
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
  (FR-104, "Requirements"; for a `holds` atom, of its temporal clause's record), keyed as FR-104 keys it. A clause that names its
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
| FR-391-AC-1 | Over the `Counter` unit (model alias `M`, `Counter` with `v: Int[0, 2]`, operation `bump()`, population `counters`), `pre Room using v on M::Counter::bump { forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }` checks as a `Precondition` with the population operand typed `Population<Counter>`, the `allInstances` result `Set<Reference<Counter>>` and every read at `pre`; the invariant `invariant Cap using v on M::Counter at current { forall(o in allInstances<M::Counter>(M::counters) : o.v < 4) }` checks with reads at `current`. | Test (TC-1076) |
| FR-391-AC-2 | `M::nothing` as the operand refuses `missing_declaration`/`missing-name` at `nothing`; an alias `M` that is also an enum name refuses `ambiguous_declaration`/`ambiguous-name` at the name. | Test (TC-1076) |
| FR-391-AC-3 | `M::counters = M::counters`, `let q = M::counters in allInstances<M::Counter>(q)`, `size(M::counters)` and a clause body that is `M::counters` each refuse `ill_typed`/`operator-ineligible` at the name, because a population is no value; `M::Counter = M::Counter` refuses `missing_declaration`/`missing-name` at `Counter`, since `Counter` is a type. | Test (TC-1076) |
| FR-391-AC-4 | With `bound B on M::counters [3]` the operand is `Population<Counter>[3]` and `allInstances<M::Counter>(M::counters)` is `Set<Reference<Counter>>[0,3]`; the record's `counters` domain is `Population(Some(3))`. With a second population `archive` of `Counter`, a clause on `M::Counter` refuses `ambiguous_declaration`/`ambiguous-name` at its `on` (FR-104-AC-5) whichever population its body names. With a type `Widget` that only a population `widgets` covers, a clause on `M::Widget` whose body names `M::counters` records `widgets` (the context population) and `counters` as its two domains. | Test (TC-1076) |
| FR-391-AC-5 | Evaluated at S6a over a snapshot whose `counters` holds `a`, `b` and `c`, the precondition of AC-1 decides over exactly those three, in canonical reference-key order; over a snapshot in which `counters` is not marked `complete`, it returns `Incomplete` with `incomplete_population`/`incomplete-scope`, as for a parameter operand; `allInstances<M::Other>(M::counters)` with `Other` uncovered by the population is FR-084's typed incomplete result. | Test (TC-1076) |
| FR-391-AC-6 | With populations `counters` and `archive` of `Counter` and an operation `grow` on `Widget`, a type covered only by `widgets`, the preconditions `pre A using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::counters) : true) }` and `pre B using v on M::Widget::grow { forall(o in allInstances<M::Counter>(M::archive) : true) }` check to two different terms, each carrying its own population's declaration key, and of the populations `A` reads `AnyMembership{counters}` and not `AnyMembership{archive}`, and `B` the reverse (each footprint also holds the receiver's own `Membership` per FR-155); the precondition of FR-155-AC-2 over `counters` holds `AnyField{v}` and `AnyMembership{counters}`. The emitted package holds one population reference per declared population read, so `A` and `B` reference two nodes and a second read of `counters` references the first's node; the package is admitted by the full reader (QSpec FR-322-AC-83). | Test (TC-1076) |
| FR-391-AC-7 | On the `Counter` unit with `bound B on M::counters [3]` and a `Watch` type with population `watchers`: `post P using v on M::Counter::bump { forall(o in M::counters : o.v < 2) }` checks as a `Postcondition` reading `post`; `pre Q using v on M::Counter::bump { exists(o in M::counters : o.v = 2) }` checks as a `Precondition` reading `pre`; `invariant Cap using v on M::Counter at current { count(o in M::counters where o.v = 0) < 4 }` checks with the `count` result typed `Int[0,3]`, and without the bound the result is unbounded above; `temporal C using t0 over (w: M::Watch) clock "model-steps" on origin { eventually holds(forall(x in M::counters : x.v < 2)) }` checks with `counters` a domain of the temporal clause's record. Each is evaluated at S6a over a snapshot whose `counters` is `complete` to the same result as its `allInstances<M::Counter>(M::counters)` spelling, and over a `counters` that is not `complete` it returns `Incomplete` with `incomplete_population`/`incomplete-scope`. | Test (TC-1076) |
| FR-391-AC-8 | `M::counters` in a function body and in the type-level invariant of an object type refuses `missing_declaration`/`missing-name` at `counters`, and `seq(i in 0 .. 2: i)` in an invariant refuses `unsupported_construct`/`expression-form` at its span. | Test (TC-1076) |

## Dependencies

- FR-106 (required populations) and FR-107 (clause evaluation), which this requirement amends; FR-084 (population binding and query results), FR-091 (`Name` forms), FR-093 and
  FR-094 (lowering and type nodes), FR-104 (state clause checking and the
  requirement record), FR-380 (the authored bound).
- QSpec FR-153 and `shared-grammar.md` (`qualified-name` primary,
  `allInstances<T>(p)`).
