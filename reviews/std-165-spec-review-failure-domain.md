---
id: SR-12885
title: "failure-domain review of STD-165 state clause names a population (PR #982)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/functional/FR-391-name-a-population-in-a-state-clause.md, spec/test-cases/TC-1076-a-state-clause-names-a-population.md, spec/functional/FR-155-derive-and-enforce-read-and-write-footprints.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-391
    type: reviews
---
# SR-12885: failure-domain review of STD-165 (PR #982)

## Summary

Ticket: STD-165. Checked FR-391's design against the code on main and on
PR #967 (footprints): unstated failure modes, identity confusion, and
evaluation and lowering paths. The design choice itself holds up. A static
`M::P` name keeps footprints state-free, reuses FR-380's spelling, and makes
the population a requirement-record domain, so the runtime supplies the
binding. The rejected alternatives (an implicit context population, clause
population parameters, a `population` form, a first-class value) are
reasonably rejected. Two paths the ACs depend on are not specified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-391 states no S6a evaluation rule for the named operand, yet AC-5 and TC-1076 row 7 assert evaluation results. FR-107, the state-clause evaluator, never names `allInstances` or `lookup`. Main refuses both in the ProtocolClause family (`ProtocolClauseUnsupported::AllInstances`/`Lookup`), because no `PopulationBinding` is ever recorded for a clause evaluation. Nothing says how the named `P` becomes a binding from the snapshot's population (FR-106), how pre and post anchors select it, or what it charges. Failure: AC-5 is unsatisfiable as specified, and FR-155 POR-2 read enforcement over such a clause refuses rather than evaluating. Add an evaluation rule (here or in FR-107) and depend on FR-106/FR-107. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:98-112,122; spec/functional/FR-107-evaluate-state-clauses-at-s6a.md:56-109; qsl-eval/src/value/expression/causes.rs:118-146; qsl-eval/src/value/expression/evaluate.rs:1990-1995 |
| FND-002 | high | The lowering rule ("as FR-093 lowers the population operand ..., over the `Population<U>[N]` type node") gives the operand no population identity. FR-093's AllInstances row takes `[ref(p)]`, a reference to a binder, and a named domain population has no binder. The `Population<U>[N]` node is anonymous and content-addressed by `(U, N)` (FR-094; `lowering/model.rs:942-975`), so `allInstances<Counter>(M::counters)` and `allInstances<Counter>(M::archive)` over two unbounded populations of `Counter` lower to identical terms. An IR reader or replay cannot tell which population a clause reads. Specify the operand term, for example a reference carrying the population's `DeclarationKey`, and add an AC with two populations of one type. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:104-105; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:200; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:190-200 |
| FND-003 | medium | Nothing pins the footprint of `allInstances<T>(M::P)` to the named `P`. PR #967's `node_reads` AllInstances arm reads `AnyMembership` of every population related to the result's referenced type (`populations_relating(target)`), not of the operand. With `counters` and `archive` both covering `Counter`, a clause naming `counters` also reads `AnyMembership{archive}`. That is sound but needlessly dependent, and FR-155 asks for FR-383's exact sets. FR-391-AC-6 and FR-155-AC-2 use a one-population fixture, so they cannot catch it. State that the read is `AnyMembership{P}` for the named `P` and add a two-population footprint case. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:123; qsl-semantics/src/model/footprint.rs:523-536 (PR #967) |
| FND-004 | low | Resolution rule 2 refuses every two-segment model-alias name in a clause body that is not a population with `missing_declaration`/`missing-name`. That includes `M::Counter`, a type, used as a value, which QSpec shared-grammar classes as a "bare type/context name refusal" (line 906). The code is misleading when the name does exist. Limit rule 2 to the operand position, or refuse a type name as a bare type name. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:75-77 |

## Verdict

Not mergeable: FND-001 and FND-002 leave AC-5 and the lowering
unimplementable as written. Examined and clean: typing as `Population<U>[N]`
only as the operand, with `ill_typed`/`operator-ineligible` elsewhere
(consistent with FR-070 and FR-098, where a population is never a witness or
replay value). Also clean: the observation rule (`current`/`pre`/`post`/
`pre(e)`, matching QSpec FR-153's anchors), the domain keying and "no second
domain" for the context population, the interaction with FR-380 bounds, and
FR-104's ambiguity rule. On FR-084, leaving an uncovered `T` as the typed
incomplete result with no static refusal is acceptable: it matches the
parameter case and QSpec FR-153, although a static refusal would be
decidable here.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The fix leaves "the FR-322 encoding of this operand term is a follow-up to IR". It names no ticket and states no S4 behaviour in the meantime. A coder implementing S4 either invents an encoding or falls back to the anonymous `Population<U>[N]` node, which is the identity defect FND-002 fixed. TC-1076 row 8 expects "two different lowered terms", which cannot be compared in IR until the encoding exists, while AC-6 says "checked terms". Name the IR ticket. State the interim: S4 refuses a clause naming a population as `unsupported_construct`, or the comparison stays at the checked term. Make row 8 and AC-6 agree on which level is compared. | spec/functional/FR-391-name-a-population-in-a-state-clause.md:116-124,167; spec/test-cases/TC-1076-a-state-clause-names-a-population.md:30 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 5604034c5: a new FR-391 "Evaluation" section and an FR-107 bullet specify how a named population is evaluated. The binding is the row of the snapshot for the read's observation. FR-106 makes the named population required, so an incomplete one gives `Incomplete`, `incomplete_population`/`incomplete-scope`. Charges are FR-084's, one `population.visit` per member. |
| FND-002 | fixed | 5604034c5: the operand lowers to a population-reference term that carries the declaration key, so the two populations of `Counter` lower differently (AC-6, TC-1076 row 8). The FR-322 encoding left as a follow-up is FND-005. |
| FND-003 | fixed | 5604034c5: "Requirements" states the footprint holds `AnyMembership{P}` for the named `P` and no other population of the same member type; AC-6 and row 8 test it. PR #967's `footprint.rs` must follow, and the code lead routes that. |
| FND-004 | accepted-no-change | Kept as `missing_declaration`/`missing-name`, and TC-1076 row 4 pins it. QSpec catalogues no distinct code for a bare type name, so a type name used as a value is a missing value name. The finding is low and the choice is reasonable. |
