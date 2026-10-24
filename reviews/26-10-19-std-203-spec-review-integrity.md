---
id: SR-13271
title: "Integrity analysis of PR #1036: FR-126, TC-521, TC-1083, ADR-018"
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

Ticket: STD-203. PR quire-spec-language#1036 at head 226cfb393. Integrity pass over ids, AC<->TC links, scope lines and cross-file consistency. Measured with a probe test in a scratch copy of PR #1024's head 04ac7043d (/private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1036-probe/repo/qsl-analyze/tests/it/probe_1036.rs; output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1036-probe/probe.log): the symmetric no-granularity run binds `a` with loop `(0,0) -upd(b)-> (0,1) -upd(b)-> (0,2) -upd(b)-> (0,0)`, 30 product states, matching the restated AC-1 and §6 rows and the merged #1030 example. AC-10: with `a` 2, `b` 0 the engine (which has no cross-binding ranking yet) returns binding `a` with stem `(2,0) -upd(a)-> (0,0)` and loop `upd(b)` x3, entry 1, length 4; the role-swapped run (`a` 0, `b` 2) gives the empty-stem length-3 loop through version 2, so `b`'s counterexample is the AC's `(2,0) -upd(a)-> (0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`. AC-11 (with `mod`, see the `/` finding): both 0 gives `a`, prefix `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`, position 2, DivisionByZero; `a` 0 / `b` 1 gives `b`, `(0,1) -upd(b)-> (0,2)`, position 1. AC-12 (with `mod`): `Violated` bound to `a`, UndefinedEvaluation at position 2, prefix `upd(a)` x2, `product_states` 0; `max_states` 1 gives `Stopped{ResourceExhausted, {MaxStates, 1}}`. AC-13: `fair strong each` gives `Liveness(StrongFairness)`; the `each` claim gives `ComponentCertificateNotBuilt` (existing test, 30 product states). The ADR's invariants `ParentOrder`/`NoCycle` are vacuous with `parent` absent and `versionNumber` is `Int[0, 1000]`, so initial versions 1 and 2 are admitted (the probe's worlds admit). No FR-126 AC id was renumbered (AC-1..AC-9 kept, AC-10..AC-13 appended); AC-10/11 -> TC-521 and AC-12/13 -> TC-1083, and both TCs `verifies` FR-126 and list them in Scope. `quire validate` on the four changed files exits 0 (quire 0.36.2; DuplicateArchetype warnings from a doubly-registered spec-artifacts-process module).

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
| FND-001 | medium | TC-1083 collides with open PR #1038 (std-206-spec), which adds spec/test-cases/TC-1083-the-refinement-mapping-lifts-references-and-reports-what-it-cannot-map.md. Whichever merges second gives main a duplicate artifact id. Renumber one before merge (TC-1084 is taken by #1039). | spec/test-cases/TC-1083-liveness-model-only-pass-and-unsupported-refusals.md:1-3 |
| FND-002 | medium | TC-521 step 2 still expects 'a five-step prefix on which `vb` is never 2', but AC-2 now binds the TP-2 counterexample to `a`; the bound instance's version is `va`, and on a canonical `a`-bound prefix `vb` may reach 2. Restate as 'bound to `a`, on which `va` is never 2'. | spec/test-cases/TC-521-explicit-state-model-check.md:60-61 |

## Verdict

Findings: FND-001 (medium), FND-002 (medium). The restated symmetric tie (binds `a`, loop `upd(b)`), the AC-10 asymmetric lasso, the AC-11 and AC-12 undefined prefixes and the AC-12 `max_states` 1 stop all reproduce on the engine (with `mod` for `/`); AC ids are kept and every new AC is linked to a TC that verifies FR-126. Not mergeable until the high and medium findings are resolved.

## Dispositions

Round 1, against head a403f83ce (fix commit a403f83ce).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | id: TC-1085 (spec/test-cases/TC-1085-liveness-model-only-pass-and-unsupported-refusals.md); no other open PR or main adds TC-1085. |
| FND-002 | fixed | - Step 2: `Violated` with a five-step prefix bound to `a`, on which `va` is never 2;   `Holds{Exhaustive}`; `Holds{Exhaustive}`; `Violated` with the prefix   `0 -inc-> 1`. |
