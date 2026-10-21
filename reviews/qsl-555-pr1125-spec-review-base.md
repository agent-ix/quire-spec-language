---
id: SR-13611
title: 'Spec review of PR #1125: hyperproperty spec gaps from the HP-2, HP-3, HP-6 and copy-swap reviews, base checklist'
type: SpecReview
analysis: base
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

Ticket: QSL-553, QSL-554, QSL-555. PR quire-spec-language#1125, head 14ab4727d (branch qsl-spec-hyper-followups), base main 0799f31d8. Diff `git diff origin/main...HEAD`: 13 spec files, +286/-63, spec-only, so code-review and gap-analysis do not apply. This base file covers the checklist, the claims checked against the code, traceability, and the QSpec follow-up list. The EARS, integrity and failure-domain lenses are SR-13613, SR-13612 and SR-13614.

Gates on head: `make check-spec-validation` exit 0, with no diagnostic on any touched file. `tools/check-index-completeness.sh` exit 0.

Verified against the code on origin/main (HP-3, HP-6) and on origin/qsl-555-copy-swap 63b4fd17c (copy-swap). I did not trust the PR text.

- FR-184, count = value+1: witness.rs `admit` stops with `Halt::WitnessSet(held+1)` once that count passes the value. The HP-1 engine for max_relation_tuples is not built, so AC-3's 64 cannot be tested yet.
- FR-177 positions and tie order: in hyper.rs `first_phase`, an undefined letter is at the state's depth, an undefined or refused `μ_U`/`μ_E` at depth+1, and a contract conjunction at the state's depth. `decide_witness` ranks undefined 0, refused 1 and empty X 2, by (position, rank), and a later instance wins only when strictly better. Undefined at position 0 breaks the loop; another position-0 finding caps later instances at horizon 1 and skips exhaustion.
- NoPartnerPath: `first_partner` refuses on an empty X for an undefined `μ_U` or universal contract. It ranks 0 and carries no catalog code. Existential contract findings (`MemberContract`) give UndecidedSuccessor. Paths and `state_keys` follow the counts the FR states.
- Lasso: `exhaustion` takes the first reaching state, the BFS connection and the CX-5 loop, and rotates the entry while the step and the universal digests are equal.
- FR-177-AC-1/7/8/9/10/11/12: each matches an existing test in tests/it/hyper_witness.rs (7 vs 6 states included).
- FR-178: `decide_projected` never calls the vacuity check. Rejecting, undefined-letter and undefined-`μ_U` states are product states ranked by BFS. A refused `μ_U` is offered at depth+1, rank 1. An Undefined path is unmarked, holds its skipped steps, and its position counts visible moves (`position_of`). AC-2/4/8/9: the tests use MID_OPERATION (0,0,0)/(1,0,1), IDLE, and (1,0,1).
- FR-183-AC-7: `replay_hyper_trace_tuple` refuses every counterexample of an align-skip clause with `HyperProjected`, which is `unsupported_construct`/`not-yet-implemented`.
- Copy-swap (#1123): `unfair_classes` drops any class with fairness. The translation drops every exchange unless all of them fix the initial state and each acceptance set. Witness and Projected products never canonicalise. `reductions::refusal` gives "no" as NotPreserving, then "yes" as Unsupported, in the order symmetry, partial order, constraint. Counts 10/7, 8/6, 4 (fairness), 8/5 (toggling), 64/48 (two classes), 18 (HP-6) match hyper_swap.rs and hyper.rs. FR-176-AC-1 at 10 matches hyper.rs:133.

Code-lane notes (not spec defects): these new or changed ACs have tests that carry no AC tag and need one: FR-177-AC-7 to AC-12, FR-178-AC-2 and AC-4 (idle halves), FR-178-AC-8, FR-178-AC-9 (its test is tagged FR-178-AC-5), FR-183-AC-7 (its test is in TC-603), FR-176-AC-1, and FR-174-AC-2 and AC-5 to AC-8 on #1123. Missing tests are FND-009 and FND-010. FR-184-AC-3 waits for the HP-1 engine.

The QSpec follow-up items FR-398, FR-400 x2 and FR-401 are accurate. FR-399's count already has a home in QSpec FR-461 ('the count the denied charge would have reached'), so that item only needs a reference. The list is incomplete: see FND-003 and FND-004. The added text has no changelog, status or owner wording.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-174 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-rows | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-classes | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-accept | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-hp6 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-lasso | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-AC-2 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-AC-3 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-AC-5 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-AC-6 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-AC-7 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-174-AC-8 | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md | examined |
| FR-176-AC-1 | spec/functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md | examined |
| FR-177-contract | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-paths | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-names | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-rank | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-instances | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-lasso | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-AC-7 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-AC-8 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-AC-9 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-AC-10 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-AC-11 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-177-AC-12 | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md | examined |
| FR-178-outputs | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-undef | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-rank | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-vacuity | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-AC-2 | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-AC-4 | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-AC-7 | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-AC-8 | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-178-AC-9 | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md | examined |
| FR-183-align | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-183-AC-7 | spec/functional/FR-183-replay-a-hyper-counterexample.md | examined |
| FR-184-outputs | spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md | examined |
| FR-184-AC-3 | spec/functional/FR-184-bound-hyper-runs-with-caller-budgets.md | examined |
| ADR-023 HM-7 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 PA-3 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 PA-6 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 PH-4 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 PH-5 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 What EN-1 admits | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 8.1 product | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 8.1 leaky | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 8.1 copy-swap | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 13 example | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 CS-1 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 CS-2 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 CS-3 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| ADR-023 CS-4 | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md | examined |
| TC-599 | spec/test-cases/TC-599-hyper-reductions-and-copy-swap.md | examined |
| TC-601 | spec/test-cases/TC-601-universal-hyperproperty-by-self-composition.md | examined |
| TC-602 | spec/test-cases/TC-602-forall-exists-by-witness-sets.md | examined |
| TC-603 | spec/test-cases/TC-603-projection-aligned-hyperproperty.md | examined |
| TC-608 | spec/test-cases/TC-608-replay-a-hyper-counterexample.md | examined |
| TC-609 | spec/test-cases/TC-609-hyper-run-budgets.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rule for total lengths is wrong. FR-178 Outputs says the lengths 'are equal when no skipped step precedes the rejecting state, and differ when one does', and ADR-023 PA-6 says the same. The lengths differ only when the traces take different numbers of skipped steps. Two runs that each take one `mix` before their visible `step` have equal lengths (2 and 2), although a skipped step precedes the rejecting state. FR-178-AC-9 is this case for an `Undefined` counterexample: the code test an_undefined_match_after_skip_moves_is_at_the_projected_position gives each path `[mix, step(0)]`. The same holds for a `Projected` counterexample over two busy starts with different secrets. Fix: 'equal when every trace takes the same number of skipped steps, and otherwise unequal'. | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:54-57; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:493; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:152 |
| FND-002 | medium | The FR-174 reconciliation drops a capability. It does not only settle a gap. On main FR-174 said EN-1 'SHALL admit' a reduction where the §5 row reads 'yes', and an admitted key symmetry 'SHALL act diagonally'. The PR replaces that with V-8 `unsupported-requested-capability` and states as permanent fact that EN-1 'does not construct the diagonal action of a key symmetry ... and does not apply a state constraint to a hyper product'. Keeping 'the row preserves' apart from 'EN-1 builds it' is coherent, and it matches the #1123 code (reductions.rs `refusal`). But it removes diagonal symmetry and state constraints for hyper items from the requirements altogether. That is an owner-level scope cut, and the PR body does not list it as a decision, as it does for NoPartnerPath. Either record it as a decision (with a follow-up if the capability is still wanted), or state the V-8 rule as the settlement of a capability not yet in scope. Leftover references to the dropped capability are SR-13612 FND-001 and FND-002. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:25-31; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:68-76; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:246-257; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:242; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:185-187; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:88-89 |
| FND-003 | medium | The QSpec follow-up list in the PR body leaves out the vacuity change. HM-7 and FR-178 'Vacuity' now say an HP-6 clause is never vacuous. QSpec FR-395 (main 4826c36, 'Vacuity') still says any clause 'with a non-empty `μ_U` and no tuple of fair universal behaviours on which `μ_U` holds at every joint step is vacuous', with no HP-6 exception, and FR-395-AC-6 tests it. FR-178-AC-7 (`match false` returns Holds) contradicts QSpec as it stands. Add FR-395 to the follow-up list. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:164; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:126-131; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:150 |
| FND-004 | low | More QSpec text repeats what this PR corrects, and the follow-up list omits it. QSpec TC-347 RD-01 still says 'leaky stores 9 classes' (now 7). TC-347 RD-08 expects 'an admitted symmetry over one subject' to give 'the same verdict as unreduced' (now V-8). TC-344 PA-03 expects 'unequal total lengths' (now fixture-dependent). FR-403's 'The copy group commutes with diagonal key symmetry, and the run's group is their product' and FR-403-AC-4's 'found under diagonal symmetry' contradict CS-2's 'EN-1 reduces by the copy group alone'. Add them to the FR-398 and FR-403 items. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:366-368; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:340-342; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:537; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:145; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:160 |
| FND-005 | low | The second sentence of AC-12 ('With another instance whose empty X is at a later position, the refusal at the earlier position still ends the item') does not check the tie rule the FR states ('It ranks as the undefined evaluation it is'). The existing code test an_unstated_undefined_evaluation_ranks_before_an_empty_witness_set_at_its_position puts the other instance's empty X at the same position, 1, which is the case that exercises the rank. Restate the sentence for an empty X at the same position, so the criterion matches the test. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:204; spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:114-120 |
| FND-006 | low | 'instance `w` is explored through the expansion of its initial states, which needs 7 product states' reads as 7 states for w. The 7 are both instances together. max_states is shared across instances (Totals), and the test a_violation_at_position_zero_bounds_the_later_instances says '7 states in all'. Say '7 product states across both instances'. | spec/functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md:203 |
| FND-007 | low | AC-6 does not name the initial states its counts depend on. '5 states against the unreduced 8' holds for the toggling vault from `(0, 0)` and `(0, 1)` (test a_swap_that_permutes_the_automaton_s_states_halves_the_product), but the AC names only 'a vault whose `step` toggles `l`'. Name the starts, as AC-5 does. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:159 |
| FND-008 | low | AC-5 says 'Reduced, the same item would be proved'. No request can force the reduction, so no test can check this clause. It is rationale, not a criterion. Move it to the FR body. The AC also names `NonInterference`, but the fixture that gives '4 product states' (copies_alternating_their_outputs_under_fairness_are_stored_whole) uses `match { a.step.op = b.step.op }`, not the §1 clause's equal-input match. Name the clause the AC means. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:158 |
| FND-009 | low | No test exists for FR-178-AC-7 (`match` conjunct `false` returns `Holds{Exhaustive}` and never `VacuousMatch`) on main or in #1123. The code satisfies it, because `decide_projected` never calls the vacuity check, but nothing pins it. Hand it to the code lane: add a test tagged TC-603/FR-178-AC-7. | spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:150; spec/functional/FR-178-check-a-projection-aligned-hyperproperty.md:126-131 |
| FND-010 | low | The new half of AC-3, 'Running the request twice gives byte-equal counterexamples', has no test in #1123's hyper_swap.rs. The reproducibility the new FR-174 lasso bullet promises is therefore unchecked under copy-swap. Hand it to the code lane. The rule for several selected reductions in the 'Where the table reads yes' bullet ('the first whose row reads no, in the order key symmetry, partial order, state constraint') has no AC and no test either. reduce.rs `selected` follows that order. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:156; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:137-142 |
| FND-011 | low | The edited §13 example still ends 'This example is reasoned by hand; it has no state count'. FR-174-AC-8 now states its count, '18 states of its projected product', and the #1123 test a_projected_clause_is_stored_unreduced asserts 18. Drop the clause or give the count. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:505-506; spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:161 |
| FND-012 | low | The new product paragraph says the negated body's automaton is 'deterministic as the one automaton every engine shares makes it (ADR-018 SM-6)'. SM-6 makes the safety monitor deterministic, but it gives a liveness negation a generalized Büchi automaton and says nothing about determinism. The 10-state count is right (the test asserts it). The automaton comes out deterministic because of the tableau translation, not because of SM-6. Cite the right source, or describe the two states without the attribution. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:330-333 |
| FND-013 | low | The reason given for HP-2-only copy-swap, 'the projected product's states pair skipped moves of one component with a monitor state, which the swap does not act on', does not describe PA-4. A projected product state is a tuple of component states with a monitor state, not 'skipped moves'. The swap could act on that tuple the way it acts on HP-2's. The real reason is that EN-1 builds no exchange for the bad-prefix monitor. State that, or state copy-swap for HP-6 as out of scope. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:114-118; spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:536 |

## Verdict

Not mergeable as it stands. FND-001 is a false general claim in FR-178 Outputs and ADR-023 PA-6. FND-003 leaves a QSpec contradiction off the follow-up list. FND-002 is a scope decision the PR does not record as one. All three are text edits. The lows are wording, traceability and fixture gaps that can ride along. The positions, tie order, counts and refusals the PR states otherwise match the code on main and on #1123. The judgment calls: the FR-174 reconciliation is coherent, but it is a capability cut (FND-002). The 14->10 change is correct for this repo, and it also needs the QSpec TC-347 follow-up (FND-004).
