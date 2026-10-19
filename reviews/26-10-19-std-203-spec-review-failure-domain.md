---
id: SR-13272
title: "Failure-domain analysis of PR #1036: cross-binding ranking and the model-only pass"
type: SpecReview
scope: "agent-ix/quire-spec-language@226cfb393; git diff origin/main...HEAD: spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md, spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md, spec/test-cases/TC-521-explicit-state-model-check.md, spec/test-cases/TC-1083-liveness-model-only-pass-and-unsupported-refusals.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-521
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1083
    type: reviews
---

## Summary

Ticket: STD-203. PR quire-spec-language#1036 at head 226cfb393. Failure-domain pass over the cross-binding ranking, the model-only pass and limits. Measured with a probe test in a scratch copy of PR #1024's head 04ac7043d (/private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1036-probe/repo/qsl-analyze/tests/it/probe_1036.rs; output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1036-probe/probe.log): the symmetric no-granularity run binds `a` with loop `(0,0) -upd(b)-> (0,1) -upd(b)-> (0,2) -upd(b)-> (0,0)`, 30 product states, matching the restated AC-1 and §6 rows and the merged #1030 example. AC-10: with `a` 2, `b` 0 the engine (which has no cross-binding ranking yet) returns binding `a` with stem `(2,0) -upd(a)-> (0,0)` and loop `upd(b)` x3, entry 1, length 4; the role-swapped run (`a` 0, `b` 2) gives the empty-stem length-3 loop through version 2, so `b`'s counterexample is the AC's `(2,0) -upd(a)-> (0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`. AC-11 (with `mod`, see the `/` finding): both 0 gives `a`, prefix `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`, position 2, DivisionByZero; `a` 0 / `b` 1 gives `b`, `(0,1) -upd(b)-> (0,2)`, position 1. AC-12 (with `mod`): `Violated` bound to `a`, UndefinedEvaluation at position 2, prefix `upd(a)` x2, `product_states` 0; `max_states` 1 gives `Stopped{ResourceExhausted, {MaxStates, 1}}`. AC-13: `fair strong each` gives `Liveness(StrongFairness)`; the `each` claim gives `ComponentCertificateNotBuilt` (existing test, 30 product states). The ADR's invariants `ParentOrder`/`NoCycle` are vacuous with `parent` absent and `versionNumber` is `Int[0, 1000]`, so initial versions 1 and 2 are admitted (the probe's worlds admit). No FR-126 AC id was renumbered (AC-1..AC-9 kept, AC-10..AC-13 appended); AC-10/11 -> TC-521 and AC-12/13 -> TC-1083, and both TCs `verifies` FR-126 and list them in Scope. `quire validate` on the four changed files exits 0 (quire 0.36.2; DuplicateArchetype warnings from a doubly-registered spec-artifacts-process module).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-126-AC-1 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126-AC-2 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126-AC-5 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | context_only |
| FR-126-AC-10 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126-AC-11 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126-AC-12 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126-AC-13 | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Bindings and ranking | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Bindings and ranking (same order) | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Bindings and ranking (statistics) | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Model-only pass | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Model-only pass (budgets) | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Unsupported liveness clauses (refusals) | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Unsupported liveness clauses (certificate) | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| FR-126 Counterexamples CX-5 walk | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md | examined |
| ADR-018 UE-4 | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| ADR-018 §6 ranking paragraph | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md | examined |
| TC-521 step 2 expected | spec/test-cases/TC-521-explicit-state-model-check.md | examined |
| TC-521 step 8 | spec/test-cases/TC-521-explicit-state-model-check.md | examined |
| TC-1083 | spec/test-cases/TC-1083-liveness-model-only-pass-and-unsupported-refusals.md | examined |
| TC-1083 step 1 | spec/test-cases/TC-1083-liveness-model-only-pass-and-unsupported-refusals.md | examined |
| TC-1083 step 3-4 | spec/test-cases/TC-1083-liveness-model-only-pass-and-unsupported-refusals.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | 'Shortest across bindings' needs every binding explored, but nothing says what a run returns when one binding has a counterexample and a later binding's exploration stops on max_states, max_transitions, cancellation or max_depth before it is known not to hold a shorter one. Returning the found counterexample breaks 'shortest'; returning Stopped hides a real violation. The engine at 04ac7043d returns the current binding's hit on a stop. State the rule (for example: a found counterexample is returned, and it is the shortest among the bindings explored). | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:303-307 |
| FND-002 | medium | 'A binding's violation ranks against another binding's undefined evaluation by the same rule' conflicts with the item-level gate in FR-126 ('second phase only when the first phase ended with no violation') and UE-4 ('A TP-4 item's second phase runs only when the first phase found no undefined evaluation'). If binding b is undefined at depth 5 and binding a has a length-3 lasso, the gate never builds a's lasso, while the ranking rule makes it win. Say whether the gate is per binding and whether a second-phase lasso competes with another binding's first-phase finding. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:308-311 |
| FND-003 | medium | UE-4 now says 'the shortest prefix first and canonical transition order after that; across over bindings the shortest first, then the earlier binding', which leaves open whether binding order or canonical transition order decides a tie at equal depth. The safety engines rank (depth, canonical state order, binding): check.rs invariant_letter reads every binding at a state before moving on, and PR #1029's TP-2 path does the same. FR-126's TP-4 rule ranks (depth, binding). The two orders give different answers when a later binding is undefined at an earlier state of the same depth. Fix the order per engine path, or state one order for all. | spec/decisions/ADR-018-temporal-properties-over-every-behaviour.md:217 |
| FND-004 | low | Two tie rules meet with no order between them: the pass's finding loses a tie at equal depth to 'the product's', and across bindings the earlier binding wins. When binding a's model-only pass and binding b's product both find an undefined letter at the same depth, the rules disagree. Say the product-wins rule applies within one binding. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:318-326 |

## Verdict

Findings: FND-001 (medium), FND-002 (medium), FND-003 (medium), FND-004 (low). The restated symmetric tie (binds `a`, loop `upd(b)`), the AC-10 asymmetric lasso, the AC-11 and AC-12 undefined prefixes and the AC-12 `max_states` 1 stop all reproduce on the engine (with `mod` for `/`); AC ids are kept and every new AC is linked to a TC that verifies FR-126. Not mergeable until the high and medium findings are resolved.
