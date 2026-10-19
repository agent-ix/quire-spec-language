---
id: SR-13270
title: "Spec review of PR #1036: FR-126 liveness ranking, model-only pass and unbuilt refusals"
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

Ticket: STD-203. PR quire-spec-language#1036 at head 226cfb393. Base spec review. Every new and restated expected value was checked against the engine. Measured with a probe test in a scratch copy of PR #1024's head 04ac7043d (/private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1036-probe/repo/qsl-analyze/tests/it/probe_1036.rs; output /private/tmp/claude-501/-Users-peter-dev/be6a486a-dbff-4734-b40c-5d7654187d9c/scratchpad/sr-1036-probe/probe.log): the symmetric no-granularity run binds `a` with loop `(0,0) -upd(b)-> (0,1) -upd(b)-> (0,2) -upd(b)-> (0,0)`, 30 product states, matching the restated AC-1 and §6 rows and the merged #1030 example. AC-10: with `a` 2, `b` 0 the engine (which has no cross-binding ranking yet) returns binding `a` with stem `(2,0) -upd(a)-> (0,0)` and loop `upd(b)` x3, entry 1, length 4; the role-swapped run (`a` 0, `b` 2) gives the empty-stem length-3 loop through version 2, so `b`'s counterexample is the AC's `(2,0) -upd(a)-> (0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`. AC-11 (with `mod`, see the `/` finding): both 0 gives `a`, prefix `(0,0) -upd(a)-> (1,0) -upd(a)-> (2,0)`, position 2, DivisionByZero; `a` 0 / `b` 1 gives `b`, `(0,1) -upd(b)-> (0,2)`, position 1. AC-12 (with `mod`): `Violated` bound to `a`, UndefinedEvaluation at position 2, prefix `upd(a)` x2, `product_states` 0; `max_states` 1 gives `Stopped{ResourceExhausted, {MaxStates, 1}}`. AC-13: `fair strong each` gives `Liveness(StrongFairness)`; the `each` claim gives `ComponentCertificateNotBuilt` (existing test, 30 product states). The ADR's invariants `ParentOrder`/`NoCycle` are vacuous with `parent` absent and `versionNumber` is `Int[0, 1000]`, so initial versions 1 and 2 are admitted (the probe's worlds admit). No FR-126 AC id was renumbered (AC-1..AC-9 kept, AC-10..AC-13 appended); AC-10/11 -> TC-521 and AC-12/13 -> TC-1083, and both TCs `verifies` FR-126 and list them in Scope. `quire validate` on the four changed files exits 0 (quire 0.36.2; DuplicateArchetype warnings from a doubly-registered spec-artifacts-process module).

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
| FND-001 | high | The new certificate bullet says a TP-4 item whose second phase finds no fair accepting cycle SHALL return ComponentCertificateNotBuilt, 'never Holds and never BoundReached'. That contradicts FR-126's max_depth rule (lines 263-268: a TP-4 item at depth k with no passing candidate returns BoundReached{depth: k}) and FR-126-AC-5 (max_depth 2 returns BoundReached{depth: 2}); the engine at 04ac7043d returns BoundReached there (example_units horizon test). BoundReached is a V-5 bound, not a proof, and needs no certificate. Fix: condition the refusal on a first phase that completed with an empty frontier (the Holds case) and drop 'never BoundReached'. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:343-350 |
| FND-002 | medium | The new fixtures spell the undefined atom `6 / (2 - c.versionNumber)` (AC-11, AC-12, TC-521 step 8, TC-1083 step 1). S3 refuses it: measured at 04ac7043d, every `/` spelling over versionNumber is `IllTyped(AmbiguousLiteral)`, while `6 mod (2 - c.versionNumber)` checks; the existing engine tests already substitute `mod` for AC-9 for this reason (model_check.rs:425-427). With `mod` the measured outcomes equal the ACs' expected values (cause DivisionByZero). Fix: write `mod` in the new rows. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:417 |
| FND-003 | medium | AC-11 is the only unequal-depth undefined-ranking row, but `always holds(...)` is TP-1, checked by the single-product safety engine, which already ranks by state depth; the new rule it is meant to verify is scoped TP-4 ('Bindings and ranking (TP-4)'). No AC exercises the TP-4 per-binding products with undefined findings at different depths. Measured: AC-12's claim with `a` 0, `b` 1 returns `a` at position 2 at 04ac7043d, while the new rule requires `b` at position 1 (b's product reaches (0,2) at depth 1). Fix: add that TP-4 row (or retarget AC-11 to a TP-4 claim) so the rule can fail. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:417 |
| FND-004 | low | The guarded `on each ... when` row cannot be reached through a package: at 04ac7043d S3/S4 refuses every `on each` clause `Temporal(NotEmitted)` (FR-337, temporal_clause.rs is_emitted), measured on the AC-1 claim with `on each (e: Config::ConfigVersion) when (true)`. AC-13 and TC-1083 step 3 should say the checked clause is given to the engine directly, or name the refusal a package gets. Step 4's 'a TP-4 claim with no fair accepting cycle' is also unnamed; AC-1's `each` claim (30 product states) is the fixture. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:419 |
| FND-005 | low | STD-203's planner default says model-only (dead) states stay out of counts and limits; the PR charges them to max_states/max_transitions (out of counts only). The engine at 04ac7043d also charges them (measured: max_states 1 stops), though its module doc says they 'count toward no limit'. The choice is sound (an uncharged pass is unbounded) but departs from the ticket's default; record the ruling on STD-203 so the engine doc follows. | spec/functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md:327-332 |

## Verdict

Findings: FND-001 (high), FND-002 (medium), FND-003 (medium), FND-004 (low), FND-005 (low). The restated symmetric tie (binds `a`, loop `upd(b)`), the AC-10 asymmetric lasso, the AC-11 and AC-12 undefined prefixes and the AC-12 `max_states` 1 stop all reproduce on the engine (with `mod` for `/`); AC ids are kept and every new AC is linked to a TC that verifies FR-126. Not mergeable until the high and medium findings are resolved.
