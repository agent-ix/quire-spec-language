---
id: SR-13612
title: 'Spec review of PR #1125: hyperproperty spec gaps, integrity'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@14ab4727d; git diff origin/main...14ab4727d (PR #1125, branch qsl-spec-hyper-followups, base main 0799f31d8): ADR-023, FR-174, FR-176, FR-177, FR-178, FR-183, FR-184, TC-599, TC-601, TC-602, TC-603, TC-608, TC-609; read against origin/main code and origin/qsl-555-copy-swap 63b4fd17c (#1123) and QSpec main 4826c36; tickets QSL-553, QSL-554, QSL-555'
review_set: subset
date: '2026-10-21'
relationships:
- target: ix://agent-ix/quire-spec-language/ADR-023
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-174
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-176
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-177
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-178
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-183
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-184
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-599
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-601
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-602
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-603
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-608
  type: reviews
- target: ix://agent-ix/quire-spec-language/TC-609
  type: reviews
---

## Summary

Ticket: QSL-553, QSL-554, QSL-555. PR quire-spec-language#1125, head 14ab4727d. Integrity lens: whether FR-174, FR-176, FR-177, FR-178, FR-183, FR-184 and ADR-023 still agree with each other after the edits. That includes context lines the PR left unchanged but whose meaning the new text changes.

Consistent: the FR-174 rows, ADR-023 §5 'What EN-1 admits', PH-4 and PH-5 and CS-1 to CS-4 agree with each other and with #1123. FR-176-AC-1 and TC-601 (10), §8.1 (10/7, 8/6) and FR-174-AC-2 and TC-599 agree. FR-178-AC-2, AC-4 and AC-9, PA-6's example and TC-603 agree on the fixtures. FR-183-AC-7 and TC-608 step 5 agree. FR-184-AC-3 and TC-609 agree on 64. The TC scope lines cover the new ACs.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-174 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-rows | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-hp6 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-177-holds | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-178-holds | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-177-rank | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-paths | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| ADR-023 HV-8 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| FR-178-rank | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-184-count | spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md | examined |
| FR-184-outputs | spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md | examined |
| ADR-023 CS-2 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 What EN-1 admits | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-177 'Violation' still says 'No such state SHALL return `Holds{Exhaustive}`, or `Holds{Reduced{…}}` under FR-174's diagonal symmetry, `X` renamed member by member and re-sorted'. After this PR, FR-174 and ADR-023 'What EN-1 admits' say EN-1 builds no diagonal action for a hyper product, and a key-symmetry selection on HP-3 settles V-8. The `Holds{Reduced}` branch can no longer occur, and the two FRs contradict each other. Remove it, or tie it to the capability FND-002 of SR-13611 asks the owner about. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:185-187; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:68-76; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:246-257 |
| FND-002 | medium | FR-178 'Projected product' still says 'Exploration that completes with no rejecting state SHALL return `Holds{Exhaustive}`, or `Holds{Reduced{…}}` under FR-174'. FR-174 now says an HP-6 clause 'SHALL be stored whole as its projected product and name no `CopySwap`', and that key symmetry and state constraints on HP-6 settle V-8. No HP-6 run can return `Holds{Reduced}`. Remove the branch. | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:88-89; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:114-118; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:68-76 |
| FND-003 | medium | ADR-023 HV-8 was not amended, and it now contradicts FR-177. HV-8 says that for HP-3 'the first product state, in canonical breadth-first order, at which a letter is undefined or a violation is reached' is reported. FR-177 ranks by joint position and then by kind (undefined, then refused or contract, then empty X), so at one depth an undefined evaluation beats an empty-X violation found earlier. HV-8 also says every path ends 'at the state the undefined evaluation reads', while FR-177 ends the existential paths one step short for an undefined `μ_U`. HV-8 says an undefined `μ` 'settles refuted', while FR-177 refuses with NoPartnerPath when X is empty. FR-177's Dependencies cite HV-8 for these rules. Amend HV-8 for HP-3, or move those rules out of it. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:294; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:137-145; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:105-113; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:114-120 |
| FND-004 | medium | 'A stopped record SHALL carry the count reached, one past the budget's value' has no scope, so it reads as covering every budget the FR lists. The code gives `count: None` for `max_automaton_states`, the evaluation meter and `max_states` (hyper.rs halt_ending), and the test the_safety_automaton_stops_at_max_automaton_states asserts `count == None`. Scope the bullet to `max_witness_set` and `max_relation_tuples`, as Outputs does. | spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md:74-76; spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md:58-59 |
| FND-005 | medium | FR-178 never says how a contract conjunction finding ranks or settles. The new Ranking section lists three findings that end the first phase (rejecting state, undefined letter, undefined `μ_U` move). But the shared `first_phase` also takes an HP-6 component's contract findings: an undefined contract conjunction is a rank-0 finding at its state's depth that returns an `Undefined` counterexample naming that component, and a refused one gives `UndecidedSuccessor`. Neither is a product state. Add the contract conjunction to Ranking and to 'Undefined and undetermined evaluation', as FR-177 now does for HP-3. | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:113-119; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:103-106 |
| FND-006 | low | The tie list ('an undefined evaluation SHALL come before a refused or incomplete `μ`, or a contract conjunction that leaves a member's steps unknown, which SHALL come before an empty `X`') leaves out a refused or incomplete universal contract conjunction. The FR's own contract bullet says that conjunction gives `UndecidedSuccessor`, and the code ranks it with the refused findings (rank 1, at its state's depth). Add it to the middle rank. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:137-145; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:98-104 |

## Verdict

Not mergeable as it stands. Two FRs still promise a `Holds{Reduced}` that FR-174 now rules out (FND-001, FND-002). ADR-023 HV-8 contradicts FR-177's new ranking and refusal (FND-003). The new FR-184 count bullet is broader than the code (FND-004). FR-178 does not cover contract findings (FND-005). All are text fixes in this PR.

## New findings (disposition pass 1)

Head f4b02702b (PR #1125 round 1).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | The new no-count bullet ('A stopped record for `max_states`, `max_automaton_states` or the evaluation meter SHALL carry no count') leaves out `max_transitions`, which FR-184 also scopes for hyper items. The code carries no count for it either (hyper.rs explore: Limit::Transitions gives `Ending::Stopped(..., None)`). Add `max_transitions`, or say 'every other limit'. | spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md:77 |
| FND-008 | low | 'SHALL return `Violated` with an `Undefined` counterexample naming that component alone, at the state's depth' can be read as putting the counterexample's position at the state's depth in moves. Two bullets earlier FR-178 says an `Undefined` counterexample's position is the projected position, the number of visible steps, and the code uses that for a contract finding too (undefined_counterexample: `stem = position_of(path)`). Depth is only its rank (first_phase offers `state.depth`). Say 'ranked at the state's depth, at the projected position of that state'. | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:110 |

## Dispositions

Round 1, head f4b02702b, checked against origin/main 0799f31d8 and origin/qsl-555-copy-swap 63b4fd17c.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in f4b02702b: No such state SHALL return `Holds{Exhaustive}`; FR-174 names no reduction for an HP-3 item. |
| FND-002 | fixed | Fixed in f4b02702b: Exploration that completes with no rejecting state SHALL return `Holds{Exhaustive}`; FR-174 names no reduction for an HP-6 item. |
| FND-003 | fixed | Fixed in f4b02702b: **HP-3.** For HP-3 the engine ranks its findings by joint position and then by kind, not by discovery order alone: at the least position, an undefined evaluation comes before a refused or incomplete `μ` or contract conjunction, which comes before an empty `X` with a fair universal cycle; FR-177 stat |
| FND-004 | fixed | Fixed in f4b02702b: A stopped record for `max_witness_set` or `max_relation_tuples` SHALL carry the count reached, one past the budget's value, since the run stops at the first member or tuple that would pass it and counts that one. - A stopped record for `max_states`, `max_automaton_states` or the evaluation meter SHA |
| FND-005 | fixed | Fixed in f4b02702b: If a component's contract conjunction evaluates `Undefined` at a product state, then the engine SHALL return `Violated` with an `Undefined` counterexample naming that component alone, at the state's depth. - If a component's contract conjunction is refused or incomplete at a product state, and no re |
| FND-006 | fixed | Fixed in f4b02702b: At one position an undefined evaluation SHALL come before a refused or incomplete `μ` or universal contract conjunction, or a contract conjunction that leaves a member's steps unknown, which SHALL come before an empty `X`, since a refused `μ_E` may have dropped members from the `X` |
