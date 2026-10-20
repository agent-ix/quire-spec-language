---
id: SR-12229
title: "Code review of quire-spec-language PR #1107: FR-391 population names in state clauses and holds atoms"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@50d047195253cf4ddcbca445656d15364d7986d8; git diff origin/main...HEAD: qsl-semantics/src/check/{check.rs, check/typing.rs, claims.rs, footprint.rs, ir.rs, lowering.rs, lowering/model.rs, mod.rs, node_key/mod.rs, references.rs, refusal.rs, state_clause.rs, temporal_clause.rs, temporal_clause/condition.rs}, qsl-semantics/src/model/{normalize.rs, observation.rs, observation/document.rs, population.rs}, qsl-eval/src/value/expression/evaluate.rs, qsl-package/src/emit.rs, qsl-replay/src/spine/clause.rs, and the changed tests"
review_set: subset
---
# Code review of quire-spec-language PR #1107

## Summary

Ticket: QSL-541 (FR-391 follow-up). One review pass over the PR diff:
`code-review` with its `rust-review` lane folded in. Soundness was checked
first, against FR-391, FR-139 as narrowed by FR-391, FR-106 "required
population" and FR-155.

Examined and sound:

- **Function bodies stay refused.** A function body types with no
  `NamedScope`, so `M::counters` there refuses `missing_declaration`/
  `missing-name`. Type-level invariants also take no `NamedScope`.
- **`seq` outside a refinement still refuses** `unsupported_construct`/
  `expression-form`. Only the other population forms are now admitted, and
  only in a population position.
- **A population is no value.** In a population position, a population name
  outside an operand or domain slot refuses `ill_typed`/`operator-ineligible`.
  That covers `=`, `let`, `size` and a bare body. `map`, `filter` and
  `flatMap` reach the same arm, so a population never leaks into a context
  the spec excludes.
- **Ambiguity** is decided by segments (`is_ambiguous`), not by a string
  probe, and comes before resolution.
- **Typing.** The operand is `Population(N)`. A domain is lowered as
  `allInstances<U>(M::P)` with `U` canonical. `count` is `[0,N]` and `sum`
  is `[min(0,N*lo), max(0,N*hi)]`, or unbounded when there is no maximum.
- **Footprint.** `AnyMembership{P}` is derived for the named population
  only. A parameter operand keeps the related-populations rule.
- **Requirements.** A named population becomes a domain of the state-clause
  record, and its authored bound is linked by key rather than by
  `walked_bound` on the object type, so two populations of one type do not
  collide. Unbounded named populations in `holds` atoms join the temporal
  record.
- **Emission refusal cannot be bypassed.** `emit_package_inner` scans every
  semantic-graph node's body leaves before candidates are built, and a body
  is a flat term. A temporal-only unit was confirmed with a scratch probe: it
  refuses `PopulationReferenceNotEmitted`. `BodyNames` skips the leaf only
  after that refusal has already run, so nothing is dropped silently.
- **The `ModelRefusal::absent_key` refactor** moves the old `AbsenceMode::Refused`
  arm of `population::lookup` unchanged: same code, same cause fields, same
  detail text, and the same non-UTF-8 hex rendering. The named path calls the
  same constructor, so both lookup paths produce identical refusals.
- **The Incomplete admission rule for state clauses.**
  `require_named_populations` runs after the other admission checks. It
  returns `incomplete_population`/`incomplete-scope` when the snapshot of the
  read's own observation does not list the population as `complete`.
- **Meter.** Named `allInstances` charges one `population.visit` per listed
  member, plus `collection.bound` on the result. The coverage check charges
  nothing. Named `lookup` charges `lookup.key` and then the result-retain
  occurrences, as `population::lookup` does.

Two soundness holes were reproduced in the new evaluator path (FND-001 and
FND-002). Both were reproduced at the reviewed sha with scratch tests in
`qsl-eval/tests/it/named_population.rs`, reverted afterwards.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A population listed `complete` with no objects is an internal fault, not an empty domain. `finish_populations` builds `population_members` from `values.holders`, which has one entry per object. A population with no objects therefore gets no key, unlike `initial.rs`, which inserts every population. `named_all_instances` and `named_lookup` then return `invariant("named-population-unobserved")`. Reproduced: `pre Grow ... { forall(o in allInstances<M::Counter>(M::counters) : o.v < 2) }` over a pre snapshot whose `counters` is `complete` and empty admits, then `evaluate_clause` returns `Err(Fault(InternalFault { stage: "S6a", invariant: "named-population-unobserved" }))`. FR-084 and FR-391 require a vacuous `true`. Fix: seed `members` from `keys_by_population` or `completeness`, so every listed population has an entry. Add the empty-complete case as a test. | qsl-semantics/src/model/observation/document.rs:2803-2812, qsl-eval/src/value/expression/evaluate.rs:2408-2410, qsl-eval/src/value/expression/evaluate.rs:2442-2444 |
| FND-002 | high | A `holds` atom decides over a population the snapshot does not list as `complete`. The Incomplete rule exists only as `require_named_populations` in state-clause admission. Temporal positions are admitted by `admit_current_snapshot` and evaluated through `evaluate_holds`, and neither checks the named population. `named_all_instances` itself reads `population_members` with no completeness check. Reproduced: `temporal C ... { eventually holds(forall(x in M::counters : x.v < 2)) }` over a current snapshot whose `counters` is `complete: false` with one object returns `Evaluated(Completed(Boolean(true)))`. FR-391 Description and AC-7 require `Incomplete` with `incomplete_population`/`incomplete-scope`. Package emission refuses the term, so replay, which recompiles and emits, does not reach this today. It is reachable through the public in-process `evaluate_holds`/`evaluate_observed`, and it becomes live as soon as QSL-869 lands the encoding. Fix: keep completeness on `ObjectEnvironment` (it already keeps `population_members`) and return `Incomplete` in `named_all_instances`/`named_lookup` when the named population is not complete. That covers both the state-clause and temporal paths in one place. | qsl-eval/src/value/expression/evaluate.rs:2378-2425, qsl-eval/src/temporal/observed.rs:117-134, qsl-semantics/src/model/observation.rs:1960-1981 |
| FND-003 | medium | Named `lookup` skips FR-084's coverage check and `lookup`'s conformance check. `named_all_instances` returns the typed incomplete result when no member type covers `T`, but `named_lookup` has no such check. So `lookup<T>(M::P, r)`, where `T` is covered by no member type of `P`, returns a decided `none`/`Undefined`/refusal instead of FR-391 Typing's "uncovered `T` is FR-084's typed incomplete result". It also never checks that a present reference conforms to `T`, as `population::lookup` does. No test exercises named `lookup` at all, at S3 or S6a. | qsl-eval/src/value/expression/evaluate.rs:2428-2488 |
| FND-004 | medium | The FR-106 required-population closure does not extend from a named population. FR-106 "required population" includes every population the clause names by `M::P` and, repeatedly, every population a reference field of an object of a required population names. `require_named_populations` checks only the named row's `complete` flag, after the closure was computed from `self` and the parameters. A precondition on `Widget` that reads `o.peer.v` for `o` in `M::counters` therefore admits when the population `peer` points into is incomplete. Admission's dangling tolerance then lets a reference to a missing object reach evaluation. (Confidence medium: the outcome at evaluation depends on how a tolerated dangling reference is read.) | qsl-semantics/src/model/observation.rs:1951-1981 |
| FND-005 | medium | Missing-name refusals are located at the whole `M::P` name, not at the second segment. FR-391 Resolution and AC-2, AC-3 and AC-8 say `at nothing`, `at Counter` and `at counters`. The new refusals use the name's location, and the tests assert the whole span (`"M::nothing"`, `"M::Counter"`, `"M::counters"`), so the tests encode the deviation. Either locate at the segment or amend the spec. | qsl-semantics/src/check/check/typing.rs:941, qsl-semantics/src/check/check/typing.rs:2722-2734, qsl-semantics/tests/it/named_population.rs:179, qsl-semantics/tests/it/named_population.rs:215, qsl-semantics/tests/it/named_population.rs:400 |
| FND-006 | low | `population_form` with no scope, or with a name that does not resolve, now refuses `missing_declaration`/`missing-name` for any domain name. That includes a declared local or a `Population<T>` parameter, as in `count(o in xs where ...)`. The old cause, `expression-form`, was accurate. The new one says a declared name is missing. | qsl-semantics/src/check/check/typing.rs:2706-2735 |
| FND-007 | low | Style notes. `NamedScope::of` is rebuilt for every state clause and cloned for every `holds` atom. `is_ambiguous` allocates a `Vec` per name. The footprint arm matches `NamedPopulation` twice (a `matches!` guard, then an `if let`). The ambiguity refusal puts "naming both" into prose (`"M (model alias and enum)"`) with one locus. | qsl-semantics/src/check/check.rs:1563-1612, qsl-semantics/src/check/footprint.rs:563-579, qsl-semantics/src/check/check/typing.rs:115-124 |

## Verdict

Not mergeable: two majors.

- FND-001 is a reproduced internal fault on an ordinary input: a complete,
  empty population.
- FND-002 is a reproduced decided verdict over an incomplete population in a
  `holds` atom. That is the exact mutant TC-1076 names ("an incomplete
  population decided as an empty set").

Both are in evaluator code this PR adds. The emission refusal shields replay
today, but not the in-process evaluator, and not once QSL-869 lands. The S3
side is sound on every point the brief named: function bodies, `seq`, no
population as a value, ambiguity, typing and bounds, footprint, the
requirement record, the emit refusal and the `absent_key` refactor.
FND-003 to FND-005 are mediums; FND-006 and FND-007 are lows.

Rust lane: no new `unwrap`, `expect` or indexing on library paths (the
`segments[..]` slice pattern cannot panic). `u64::try_from(..).unwrap_or(u64::MAX)`
is used on counts. Errors are typed, using `EmitRefusal::PopulationReferenceNotEmitted`
with code and cause from the catalog, and the `invariant(..)` string
convention. No new `allow`, no `unsafe`, and no CI or workflow change. The
match arms over `NodeKind` and `LeafTerm` are exhaustive.

Narrow test run at the reviewed sha:
`cargo test -p qsl-eval --test it named_population` passed its 3 tests. The 2
scratch probes printed the FND-001 fault and the FND-002 `Boolean(true)`. A
temporal-only unit refused at emission.
