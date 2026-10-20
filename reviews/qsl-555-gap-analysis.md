---
id: SR-13606
title: "Gap analysis of QSL-555 E14c3 PR #1123 against FR-174 (AC-1..AC-4) and TC-599"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2f4479baf; PR #1123 (git diff origin/main...2f4479baf): qsl-eval/src/hyper/{copy_swap,product,mod}.rs, qsl-eval/src/automaton/{formula,translate,mod}.rs, qsl-analyze/src/model_check/hyper.rs, qsl-analyze/src/model_check/hyper/{lift,reductions}.rs, qsl-replay/src/model_check/{hyper_outcome,hyper_replay}.rs, qsl-replay/src/{lib,proof_result}.rs, qsl-semantics/src/check/{hyper_clause,mod}.rs, qsl-analyze/tests/it/{hyper_swap,hyper,hyper_world,main}.rs, qsl-semantics/tests/it/hyper_clauses.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-174
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-599
    type: reviews
---
# SR-13606: Gap analysis of QSL-555 E14c3 PR #1123 against FR-174 (AC-1..AC-4) and TC-599

## Summary

Ticket: QSL-555 (E14c3). Tools: `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2), exit 0. Plan completion: not assessed.

Matrix at 2f4479baf:

| AC | Status | Bound by | Honest? |
| --- | --- | --- | --- |
| FR-174-AC-1 | tagged | qsl-semantics hyper_clauses.rs:675 (on main) | yes |
| FR-174-AC-2 | tagged | hyper_swap.rs:57, :96, :126, :153, :199 | no: AC-2 is not delivered (FND-002); only :57 asserts part of it; :96, :126, :153, :199 over-tagged (FND-004, FND-011 to FND-013) |
| FR-174-AC-3 | tagged | hyper_swap.rs:263, :296, :326, :418, :459, :523, :572 | yes; two lifting paths untested (FND-005) |
| FR-174-AC-4 | tagged | hyper_swap.rs:602, :682 | :602 yes; :682 over-tagged (FND-008) |

The new semantics test `a_copy_group_carries_the_exchange_of_the_body_s_atoms` carries TC-599 alone, which is right: it checks the generators, which no AC names. The changed TC-601 tests keep `#[trace("TC-601")]`. Each now asserts the unreduced count through `World::explore` beside the quotient's. The FR-176 bindings are unchanged (AC-3 and AC-4 tagged; AC-1 and AC-2 untagged, as on main).

Code with no owning requirement: none. `lift.rs`, `reductions.rs`, `copy_swap.rs`, the normal-form arena and `CopyGenerator` all serve FR-174's behaviour bullets. Stubs: none. The V-8 `Unsupported` arm is a modelled answer, not a placeholder, but it stands in for an unbuilt SHALL (FND-003).

The oracle and mutation results are in SR-13605.

Spec follow-ups (route to /specify):

- ADR-023 CS-3 and PH-5, and FR-174's detection rule: the claim that the quotient keeps fairness for any granularity is false for a per-slot fairness check (gap FND-001).
- FR-174-AC-2 and TC-599 step 2: 9 and 14 should be 7 and 10 (gap FND-002).
- FR-176 / FR-174: which canonical lasso a refutation under copy-swap reports (gap FND-009).
- ADR-023 CS-3: the swap that exchanges acceptance sets (gap FND-010).

Declared out of scope, and confirmed not built: the `Proved{Reduced, Certified}` settlement (FR-163, FR-182), diagonal key symmetry and state constraints for hyper items (V-8 stopgap), and copy-swap for HP-6.

## Verdict

**Not mergeable for coverage until FND-001 and FND-002 are addressed.**

- **FND-001 (high).** The spec's own soundness claim for copy-swap under fairness (CS-3, PH-5) is false for the quotient EN-1 builds. Route to /specify, and restrict the code meanwhile (SR-13605 FND-001).
- **FND-002 (medium).** FR-174-AC-2 is bound by five tests but not delivered: the counts need a spec fix (7 and 10, not 9 and 14), and the `Proved{Reduced}` settlement is not built. Move the tags to TC-599 only.
- **FND-003 and FND-005 (medium).** A 'yes' row settles V-8 instead of being admitted, with no follow-up ticket. Two lifting paths are untested (L2 and U2 survive).
- **FND-004 and FND-006 to FND-014 (low).** Four over-tagged AC-2 tests and one AC-4 test; test gaps for past and interval operators and two preservation branches; the HP-6 follow-up; which canonical lasso a refutation under the swap reports; and the unstated acceptance-set fallback.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The spec admits copy-swap for copies with equal fairness sets ('any fairness granularity', PH-5). ADR-023 CS-3 argues that 'the fairness filter is invariant for `whole` and `each` constraints alike', so the quotient keeps fair cycles. For the quotient EN-1 builds, that is false: a fairness obligation read on the stored state's slots is not one copy's obligation once edges permute the copies. Code review SR-13605 FND-001 has a false proof, a second false proof and a spurious refutation, each with weak fairness on `reset` on both copies. Emerson and Sistla's result for fairness needs an annotated quotient, which CS-2 and CS-4 do not describe. No TC-599 step and no FR-174 AC exercises copy-swap with a non-empty fairness set. Route to /specify. Either restrict copy-swap to classes whose fairness sets are empty (the ADR's own §8.1 worked example: 'fairness sets are empty, so S3 admits copy-swap'), or specify the annotated quotient and its CX-5 loop. Then add an AC that the reduced and unreduced verdicts agree under fairness. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:Copy-swap detection |
| FND-002 | medium | The matrix reports FR-174-AC-2 as `tagged`, by five tests. But AC-2 is not delivered as written, so the coverage is inflated against the PR body's own 'AC-2, partly'. (a) Its counts, 9 against 14 for the leaky vault, cannot be reproduced: the deterministic shared automaton gives 10 unreduced and 7 reduced, which the tests assert. (b) The secure verdict `proved` with basis `Reduced` naming `CopySwap{[[a, b]]}` is not built: the engine returns `HoldsReduced`, with no certificate and no settlement (FR-163, FR-182). Only `the_swap_halves_the_pair_product_and_keeps_both_verdicts` asserts part of AC-2 (secure 6 against 8, both verdicts). Fix: route the count fix (7 and 10) for AC-2 and TC-599 step 2 to /specify. Until then, and until the settlement lands, tag the five tests `#[trace("TC-599")]` only and take `FR-174-AC-2` out of their doc-comment openings, as #1092's fix round did for FR-146. Or split AC-2 so that its engine half can be tagged alone. | qsl-analyze/tests/it/hyper_swap.rs:57, 96, 126, 153, 199 |
| FND-003 | medium | FR-174 says EN-1 'SHALL admit' a selected reduction wherever ADR-023 §5 reads 'yes', with diagonal key symmetry (PH-2) and the run's group 'the product of the copy group and any admitted diagonal key symmetry' (Outputs). The engine admits none of them. A 'yes' row settles V-8 `Unsupported`, which the PR calls a judgement call. No AC covers the admitted case, so the matrix shows no gap. But an FR SHALL is unbuilt, and no ticket names it: neither the PR body nor the module doc points at a follow-up. Fix: file a follow-up ticket for diagonal key symmetry and state constraints on hyper items (PH-2, PH-4, PH-6 symmetry), and cite it in `reductions.rs`'s module doc. Or record the V-8 stopgap in FR-174 itself. | qsl-analyze/src/model_check/hyper/reductions.rs:1-11, 65-92 |
| FND-004 | low | `a_swap_that_permutes_the_automaton_s_states_halves_the_product` is tagged FR-174-AC-2, but it asserts 8 to 5 states on the toggling vault for an `eventually` conjunction, which AC-2 does not state (AC-2 is ADR-023 §8.1's leaky and secure vaults). It is good TC-599 evidence for FR-174's canonicalisation bullet. Tag it `#[trace("TC-599")]` only, and take the AC id out of its doc-comment opening. | qsl-analyze/tests/it/hyper_swap.rs:114-144 |
| FND-005 | medium | Two lifting paths are untested: mutants L2 and U2 survive. (a) L2: the joint step of a lifted path is not reordered after a permuted state. No test's counterexample takes a step with unequal component transitions after a swapped state: the paths are one step from the root, or loops where both copies take the same step. (b) U2 (hyper.rs:1615-1616): an undefined `match` (`MatchUndefined`) at a swapped state, reported with its step and posts in stored order. The oracle shows the code is right on random cases (every reduced counterexample replays). But the PR's tests would not notice a regression. Add a counterexample at depth 2 or more through a swapped state with `step(0)` against `step(1)`, checking each trace's inputs, and an undefined-`match` fixture (FR-176-AC-4's division conjunct) whose first undefined joint step is from a swapped stored state. | qsl-analyze/src/model_check/hyper/lift.rs:100 |
| FND-006 | low | The automaton rename is untested for past and interval operators. Mutants N5 (past-operator `memory` not renamed), N6 (interval `counters` not renamed) and S3 (the check that the temporal operators stay in the arena removed) survive. No hyper_swap.rs body has `once`, `since` or an interval. The oracle ran such bodies (`once`, `since`, `eventually[0,2]`, `eventually[0,3]`) and agreed, so this is a test gap, not a defect. Add one symmetric body with a past operator and one with an interval, each asserting reduced and unreduced counts and the verdict. | qsl-eval/src/automaton/translate.rs:544-567 |
| FND-007 | low | Two branches of `preserves` are untested. Mutant F4 (the `whole`-fairness condition of PH-2 ignored, so an `each` constraint gives `Unsupported` instead of PH-3's `ReductionNotPreserving`) survives. So does F6 (key symmetry for HP-3 refused as not preserving, against PH-6's 'yes'). Add a key-symmetry selection on `NonInterference` with a `fair each` constraint (expect `NotPreserving`), and one on `Opaque` (expect `Unsupported`). | qsl-analyze/src/model_check/hyper/reductions.rs:40-63 |
| FND-008 | low | `a_reduction_whose_row_reads_yes_is_not_refused_as_non_preserving` is tagged FR-174-AC-4. AC-4 states only the three refusals. This test asserts the V-8 `Unsupported` stopgap, which contradicts FR-174's 'SHALL admit' (FND-003), and the no-selection `HoldsReduced` path. Tag it `#[trace("TC-599")]` only. | qsl-analyze/tests/it/hyper_swap.rs:676-716 |
| FND-009 | low | The spec does not say which canonical counterexample a refutation under copy-swap reports. FR-176 asks for 'FR-126's canonical stem and loop over the product'. The engine reports the quotient's canonical lasso, lifted and repeated, which differs from the unreduced product's: in 24 of the oracle's 1,229 agreeing cases, `trace_position` differs (1 against 0, 4 or 5 against 2), and the lasso is longer. Both are real and replay, and each is deterministic (FR-176-AC-4's byte-equal rerun holds). But two conforming engines, or one engine before and after a spec change to the copy group, report different counterexamples for one item. State in FR-174 or CS-4 that the reported lasso is the lifted canonical lasso of the quotient. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:Counterexamples and verdicts |
| FND-010 | low | CS-3 says `σ` 'maps the automaton's acceptance sets … onto themselves' as a property of every swappable clause. For the normal-form automaton EN-1 builds, a swap of two copies exchanges the per-eventuality acceptance sets of bodies such as `always p@a and always p@b`. The engine then stores the product whole and names no reduction, while S3 still reports the copy group. That is sound, and it is detected correctly (mutants S1 and H2 are killed, and the oracle agrees). But the spec does not describe it: the item silently runs unreduced. Say in CS-3 (or FR-174) that a swap that does not fix each acceptance set is not applied, and whether the run reports that it was withheld. | spec/decisions/ADR-023-hyperproperties-over-every-behaviour.md:CS-3 |
| FND-011 | low | `an_asymmetric_clause_is_stored_unreduced` is tagged FR-174-AC-2, but it asserts that a body that fails CS-1 gets an empty copy group and an unreduced run of 8 states, which AC-2 does not state. Tag it `#[trace("TC-599")]` only, and take the AC id out of its doc-comment opening. | qsl-analyze/tests/it/hyper_swap.rs:90-112 |
| FND-012 | low | `a_swap_that_exchanges_acceptance_sets_is_not_applied` is tagged FR-174-AC-2, but it asserts the CS-3 fallback: a swap that exchanges acceptance sets is withheld, and the product is stored whole. AC-2 does not state that. Tag it `#[trace("TC-599")]` only, and take the AC id out of its doc-comment opening. | qsl-analyze/tests/it/hyper_swap.rs:146-172 |
| FND-013 | low | `three_copies_store_one_state_for_each_multiset_of_secrets` is tagged FR-174-AC-2, but it asserts the symmetric group of three copies (8 against 16 states, and a three-copy refutation that replays), which AC-2 does not state. Tag it `#[trace("TC-599")]` only, and take the AC id out of its doc-comment opening. | qsl-analyze/tests/it/hyper_swap.rs:192-230 |
| FND-014 | low | FR-174 applies copy-swap to HP-6 as well as HP-2 ('on every HP-2 and HP-6 clause'). The projected product (QSL-554, #1121) is not on main, so this PR cannot cover it. But nothing tracks it: the PR body and the module docs are silent. When #1121 lands, its product needs the `copy_swap` input and the canonical hook, and a monitor-state rename in place of the automaton exchange. File a follow-up ticket, ordered after QSL-554. | spec/functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md:Description |
