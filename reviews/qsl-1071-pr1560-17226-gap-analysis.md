---
id: SR-17226
title: "gap analysis of QSL-1071 hyperproperty node emission, FR-488-AC-1 to AC-23 and TC-1266 (quire-spec-language PR 1560)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@843fe72ebabf115a94cd969e1e3db9f06e0ea7d5; spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md; spec/test-cases/TC-1266-a-hyper-clause-emits-a-hyperproperty-claim-node.md; spec/functional/FR-315-encode-the-smt-lib-transition-relation.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-488
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1266
    type: reviews
---

# SR-17226: gap analysis of QSL-1071 hyperproperty node emission (quire-spec-language PR 1560)

## Summary

Ticket: QSL-1071. Plan completion: not assessed. Reviewed head 843fe72ebabf115a94cd969e1e3db9f06e0ea7d5; PR 1560 was merged at e1e5edddb (head 35de233f3, a merge of main only) while this ran, so every action below is a follow-up against main. AC to test trace, from the `#[trace("TC-1266", ...)]` tags (nine tests in `qsl-package/src/emit/tests/hyper_clause.rs` plus `slot_names_are_identifier_safe_and_injective` in `lowering/hyper.rs`):

- AC-1, AC-2, AC-3: `a_hyper_clause_emits_the_hyperproperty_node_with_its_seven_bindings`. Covered (names and order, domain, subjects, parameters, quantifier Groups, empty align_skip; the I2 admission is in `admit`).
- AC-4, AC-5: `the_body_wrapper_reads_slot_zero_and_slots_are_named_identifier_safe`. Partly: identity, clock, empty captures and fairness, slot name and level are asserted; the `temporal_profile` law, the activation `origin`, the formula (`always` with a `null` interval over a `holds`), the holds operand, and the slot `expression`/`type` occurrence sites are not (FND-002).
- AC-6: tagged, not run (FND-004 of SR-17225; FND-001 here). No two-alias clause exists.
- AC-7, AC-8, AC-9: no assertion on the match terms beyond `len() == 1` and the presence of the names `op_sa`, `op_sb`. Not covered (FND-001, FND-003).
- AC-10: `an_align_skip_operation_shares_the_state_clause_anchor`. Covered for one operation; two operations in written order are not (low).
- AC-11, AC-12: `equal_clauses_share_a_node_and_differing_ones_do_not`. Partly: no-match and renamed-variable differ, two equal clauses share a node with ordinals 0 and 1. Alias change, `exists`, reversed order, the fairness constraint on `a` and on `b`, an `align skip` list and a changed body are not asserted, nor is reformatting (FND-006). All were probed correct.
- AC-13: `fairness_constraints_follow_their_quantifier_as_resolved`. Partly: kind, variable, granularity, name, position; not `declaration`, not `weak whole` equal to unmarked, not the `systems_interface` refusal (FND-006).
- AC-14, AC-15: `the_occurrence_round_trips_and_the_wrapper_is_no_clause_root`. Partly: the round trip, the wrapper triple, a wrong ordinal, and `clause_roots` empty are asserted; an unknown NodeId, a non-hyper NodeId, role `invariant`, and FR-338's resolution of the wrapper triple are not (FND-006).
- AC-16, AC-17: not covered (FND-004). Probed: `Linked`-shaped `let r = c.parent in ... deref(value(r)).versionNumber ... @ a` emits; the same with `(c.parent @ a)` inside `@ b` is refused at `deref(value(r)).versionNumber`.
- AC-18, AC-19, AC-20: the one test tagged AC-18/19 asserts AC-20's bounded-fairness refusal only. Timed, the alias-without-parameter refusal, two-refusal precedence, the bounded and infinite-trace intervals and `until` are not asserted (FND-005). Probed: timed is refused at the clause; `always [0, 3]`, `until` and `eventually [2, *]` emit and are admitted.
- AC-21: `the_emitted_nodes_count_against_the_node_ceiling`, tight limit refuses only, and `with_nodes(8)` is two below the boundary 10 (FND-007; SR-17225 FND-002).
- AC-22: `every_emitted_hyper_shape_is_admitted_by_the_i2_read` admits four shapes (plain, fairness on both variables, `align skip`, receiver match). Covered; no argument-read or `let` shape is among them.
- AC-23: `slot_names_are_identifier_safe_and_injective` (unit, `esc` and the injectivity grid) and the admission of the emitted names. Covered; the `arg_` and `rcv_` names are only exercised through admission (FND-001).

No stub, `todo!`, `unimplemented!` or tautological assertion was found; each existing assertion compares against a computed value. Two tags are wrong (SR-17225 FND-003, FND-004) and are coverage inflation.

Code with no owning requirement: `assembled_occurrence` and `MatchPart` re-export in `qsl-eval` are owned by FR-488 "Outputs" and FR-487; `conjunct_trees` and `ExpressionSpans::subtree` exist for the FR-487 conjunct sites; none is unowned.

FR-315-AC-12/13: `HyperProductIdentity` does not exist in the repository; this PR satisfies only the `occurrence()` input those ACs name (round trip proven). The PR cannot close the FR-315 half.

Judgement of the coder's admitted gaps (which must be fixed, which are acceptable):

- Argument-read slots and interleaved or constant conjuncts (AC-7 to AC-9): must fix. This is the wrong-output surface of the requirement: the level rule makes the same read a different node at a different position, and no test pins it. The production behaviour was probed correct (op_sb at level 0 and 1 as two nodes; order `U0, E0, U1, E1`), so the fix is a test.
- `Linked` accepted and refused atoms (AC-16, AC-17): must fix the accepted vector and the refused vector at the read of `l`; the call-result and two-component vectors should follow. All are cheap on the ConfigVersion fixture (`parent` is an optional reference).
- systems_interface, timed, two-refusal precedence (AC-13, AC-18): should fix; precedence is a stated requirement and the refusal function is the part QSL-1094 will edit, so the order needs a regression test before it is edited.
- Until clauses and intervals (AC-19): should fix; assert the interval objects, not only admission.
- AC-21 tight-limit-only: should fix to the exact-boundary pattern.
- AC-18/19 through bounded fairness only: must re-tag (it is AC-20), and add the AC-18 and AC-19 tests.
- `occurrence()` returning the zero key for an unassembled clause: acceptable.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-488 | spec/functional/FR-488-emit-a-checked-hyper-clause-as-a-v2-hyperproperty-node.md | examined |
| TC-1266 | spec/test-cases/TC-1266-a-hyper-clause-emits-a-hyperproperty-claim-node.md | examined |
| FR-315-AC-12 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md | context_only |
| FR-315-AC-13 | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md | context_only |

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-488-AC-7 and AC-8 (match terms, the argument-read slot names `arg_s..`, the `String` and `Option` slot types, `op_sb` at level 0 in one conjunct and level 1 in another) have no test. The only match assertion is `members.len() == 1` plus the presence of `op_sa` and `op_sb`. This is the level/key collision surface. Probe: correct (`op_sb` is two nodes, levels 0 and 1). Add a test over `b.step.op = stutter and a.step.op = b.step.op` and an argument read over a model with an operation parameter (the semantics vault has `step(i)`). | qsl-package/src/emit/tests/hyper_clause.rs:172-232 | correct-requirement-no-evidence |
| FND-002 | medium | FR-488-AC-4 and AC-5 are asserted partly: the wrapper's `temporal_profile` law against the lock row, the activation `kind` = `origin` alone, the formula (`quire.op.temporal.always` with a `null` interval over `holds`), the holds operand (equality of `l` of slot level 0 and level 1), and the `hyper 0 slot 0` and `hyper 0 slot 1` `expression` and `type` occurrences are not. These are the wrapper's content, the part a consumer outside QSL reads. | qsl-package/src/emit/tests/hyper_clause.rs:238-275 | correct-requirement-no-evidence |
| FND-003 | medium | FR-488-AC-9 (interleaved universal, existential, universal conjuncts in `conjuncts()` order, with `c1` at `hyper 0 match exists 0`) and the constant-conjunct half of AC-8 have no test. Probe: `conjuncts()` is `U0, E0, U1, E1` and the constant `true` emits as a reference to a `literal` node (see SR-17225 FND-008 for the AC wording). | qsl-semantics/src/check/hyper_clause.rs:59-77 | correct-requirement-no-evidence |
| FND-004 | medium | FR-488-AC-16 and AC-17 have no test. The `let`-rooted read inside `@ a` (emits), the same inside `@ b` over `(v.next @ a)` (refused at the read of `l`), a read whose object operand is a call result, and a read rooted in two components are the whole of the `unencodable_read` logic, the second most intricate function of the PR. Probe: the first two behave as the spec states. | qsl-semantics/src/check/hyper_clause/emission.rs:390-445 | correct-requirement-no-evidence |
| FND-005 | medium | FR-488-AC-18, AC-19 and AC-20 are covered by one test tagged for the wrong ACs (SR-17225 FND-003). Timed refusal at the clause, alias-without-parameter refusal, "two refusals report the first" precedence (QSL-1094 lifts refusals 1, 4, 5 and 2 one at a time, so the order is the thing to protect), the `{lower: "0", upper: "3"}` and `{lower: "2", upper: null}` intervals, and `until`'s `null` interval are not asserted. Probe: all emit or refuse as stated. | qsl-package/src/emit/tests/hyper_clause.rs:472-491 | correct-requirement-no-evidence |
| FND-006 | medium | FR-488-AC-11 (6 of 8 variants unasserted), AC-12 (reformatting), AC-13 (`weak whole` equal to unmarked bytes, `declaration` reference, `systems_interface` refusal), AC-14 (unknown NodeId, non-hyper NodeId, role `invariant`) and AC-15 (FR-338's resolution of the wrapper triple) are asserted partly. Probe: every NodeId inequality and equality of AC-11, AC-12 and AC-13 holds; the negative lookups of AC-14 follow from `resolve_hyper_root` by reading. | qsl-package/src/emit/tests/hyper_clause.rs:315-470 | correct-requirement-no-evidence |
| FND-007 | low | FR-488-AC-21: tight limit only, magic limit 8, `is_err()` (SR-17225 FND-002). | qsl-package/src/emit/tests/hyper_clause.rs:493-503 | correct-requirement-no-evidence |
| FND-008 | low | FR-315-AC-12 and AC-13 name `HyperProductIdentity`, which this repository does not contain; the PR provides the `occurrence()` input only. The FR-315 ticket owns the rest; the PR description should not claim the ACs satisfied. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1142 | wrong-requirement |

## Verdict

PASS with findings on the production code, FAIL on test evidence for a merged PR: every group of ACs has at least one real test or a probe showing the behaviour correct, none is a stub, but AC-6, AC-7 to AC-9, AC-16, AC-17 and the precedence half of AC-18 have no test and two tests carry the wrong ACs. Fix FND-001, FND-002, FND-003, FND-004 and the re-tagging in a follow-up PR; FND-005 and FND-006 in the same PR; FND-007 with SR-17225 FND-002; FND-008 is a ticket note.
