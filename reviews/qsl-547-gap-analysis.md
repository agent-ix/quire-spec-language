---
id: SR-13596
title: 'Gap analysis of PR #1113: FR-143, FR-144 liveness half and FR-145 lasso coverage'
type: SpecReview
scope: 'agent-ix/quire-spec-language@38b65f748; git diff origin/main...38b65f748 (PR #1113, branch qsl-547-refine-liveness); specs FR-143, FR-144, FR-145, TC-548/549/550/556; quire matrix (quire 0.36.2) on head; ticket QSL-547'
review_set: subset
date: '2026-10-20'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-143
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-144
  type: reviews
- target: ix://agent-ix/quire-spec-language/FR-145
  type: reviews
---

## Summary

Ticket: QSL-547. PR: quire-spec-language#1113, head 38b65f748. Method: gap-analysis, planless (Plan completion: not assessed). Suites are green (see SR-13595). `quire matrix --format tsv` on head lists FR-143-AC-1..4, FR-144-AC-2/3/4/6/7 and FR-145-AC-1..6 as tagged. FR-143-AC-5, FR-144-AC-1 and FR-144-AC-5 are untagged. These are declared remaining work, as are FR-145-AC-4's identity and package refusals, and are not findings.

Mutation probes were run against `cargo test -p qsl-analyze --test it refinement_`, each reverted after; survivors were re-run against the reviewer's random oracle (250 cases).
- Killed by the PR suite: M01 (taking edges kept), M02 (all vertices within), M03 (no F_C filter), M04 (always Divergence), M05 (no hidden gate), M11 (entry = last vertex), M13 (product drift unchecked), M14 (no F_C replay check), M20 (liveness Certified), M21 (record always safety), M24 (cause string), M28 (stutter takes).
- Killed only by the reviewer oracle: M07 (classes reversed), M17 (divergent = any stutter), M18 (terminal stutter not pushed), M25 (each read as whole).
- Survived both: M06 (stem initial index 0), M08 (abstract undecided ignored), M09 (no max_transitions in phase 2), M10 (no cancel poll per class), M12 (no close_loop in refinement replay), M15 (replay skips the enabled check), M16 (replay skips the taken check), M19 (replay accepts any class), M23 (liveness record even when safety is not proved), M26 (liveness Stopped settles failed).
- Not run: M22 (anchor ambiguous) and M27 (does not compile).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-143-AC-1 | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md | examined |
| FR-143-AC-2 | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md | examined |
| FR-143-AC-3 | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md | examined |
| FR-143-AC-4 | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md | examined |
| FR-143 Behavior | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md | examined |
| FR-144-AC-2 | spec/functional/FR-144-request-and-settle-a-refinement-item.md | examined |
| FR-144-AC-3 | spec/functional/FR-144-request-and-settle-a-refinement-item.md | examined |
| FR-144-AC-7 | spec/functional/FR-144-request-and-settle-a-refinement-item.md | examined |
| FR-144 Settlement | spec/functional/FR-144-request-and-settle-a-refinement-item.md | examined |
| FR-145-AC-2 | spec/functional/FR-145-replay-a-refinement-counterexample.md | examined |
| FR-145-AC-3 | spec/functional/FR-145-replay-a-refinement-counterexample.md | examined |
| FR-145-AC-4 | spec/functional/FR-145-replay-a-refinement-counterexample.md | examined |
| FR-145 Lasso | spec/functional/FR-145-replay-a-refinement-counterexample.md | examined |
| FR-137-AC-4 | spec/functional/FR-137-check-a-refinement-s-fairness-rows.md | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-145 "Lasso": the replay checks that decide agreement are untested. M15 (drop "enabled at every loop state"), M16 (drop "taken by no loop step") and M19 (accept a class of any constraint) each survive the PR suite, so a lasso whose loop takes the recorded `F_A` constraint, or leaves it disabled, would reproduce `refuted`. M12 (drop the concrete `close_loop` call in the refinement arm) also survives. `same_product_state` compares only history values and candidates, so a concretely open loop over states with the same mapping (CAS begin steps) would reproduce. No test feeds a lasso that fails any of these checks. | qsl-replay/src/model_check/refinement_replay.rs:107, 373-377; qsl-analyze/tests/it/refinement_replay.rs:493-595 |
| FND-002 | medium | FR-143 Behavior "The phase SHALL respect FR-126's limits and FR-142's `cancel` handle and return `Stopped`" has no test. Nor does the undecided-abstract-contract path, or the settlement of a Stopped or Undecided liveness half. M08, M09, M10 and M26 survive. | qsl-analyze/src/model_check/refine.rs:607-660, 709-711; qsl-replay/src/model_check/refinement_settle.rs:538-545 |
| FND-003 | medium | FR-143's second phase defines only the weak reading of `F_A` ("enabled at every mapped state"). FR-137-AC-4 admits `ensure fair strong`, and ADR-020 roadmap step 4 says strong fairness in `F_A` comes "with no change here". The spec gives no rule for a strong `F_A` class, and the code checks it as weak, which is unsound (SR-13595 FND-001). FR-143 (and FR-145 "Lasso") need a ruling: a strong class is violated by a fair SCC where it is enabled somewhere and never taken, or strong `F_A` settles `unsupported`. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md (Second phase) |
| FND-004 | low | Several ordering and shape rules are pinned only by the reviewer's oracle, not by the PR suite. These are F_A source and identity order (M07), `each` against `whole` granularity (M25), Divergence needing every loop step stuttering (M17) and the terminal-stutter extra step in replay (M18). The tree-path stem's initial index (M06, single-initial fixtures only) is pinned by nothing. Neither is item-level selection when a hidden-field refinement's safety half is not proved (M23). | qsl-analyze/src/model_check/refine.rs:709, 811; qsl-replay/src/model_check/refinement_replay.rs:238-268, 378; qsl-replay/src/refinement/liveness.rs:31-37; qsl-replay/src/model_check/refinement_settle.rs:500-503 |
| FND-005 | low | Spec follow-up: FR-143 Outputs and AC-1/2/3 name `Checked(Holds{Exhaustive})` and `Unsupported(UnsupportedCause)`. The code returns `Checked(HoldsReduced{reductions: []})` and `UnavailabilityCause::HiddenFieldLiveness`, and overloads `HoldsReduced`'s "no reductions" meaning. The tests assert the code's form. FR-143 should name the representation it accepts. | spec/functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md (Outputs); qsl-replay/src/model_check/outcome.rs:96-103 |

## Verdict

Not mergeable while SR-13595 FND-001 stands, and the two medium coverage gaps should close in the same PR. Coverage of the delivered ACs is honest. FR-143-AC-1..4, FR-144-AC-2/3(hidden-field part)/7 and FR-145-AC-2/3/4(drift and determinism parts) each have a tagged test that asserts the AC's stated outcome, with the right TC tags (TC-548, TC-549, TC-550, TC-556). No tagged test that cannot fail was found. Untagged ACs FR-143-AC-5, FR-144-AC-1 and FR-144-AC-5 are declared remaining work.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | Head 5884e0302 does not contain main's d2e3c128b (#1117), so the branch's spec copy has no FR-143-AC-6, FR-143-AC-7 or AX-6. The new tests carry only `TC-548`, and no test is tagged FR-143-AC-6 or FR-143-AC-7. Two parts of AC-6/AC-7 have no test: AC-6's "strong variant whose safety half is violated" through the engine (the settle test uses a hand-built outcome with the hidden-field cause), and AC-7 (no fault-injection test). The branch merges main cleanly (merge-tree). | qsl-analyze/tests/it/refinement_liveness.rs:287-322, 530-543 |
| FND-007 | low | Cancellation inside the second phase's fairness analysis is untested. Mutant M10' (pass a fresh `Cancel` to `analyze`) survives. `a_cancellation_during_the_second_phase_stops_it` only needs some phase-2 charge point to stop, and the `abstract_enabled` polls give it one. Removing the per-class poll (the original M10) is equivalent, because `analyze` charges the same handle before each decomposition. | qsl-analyze/src/model_check/refine.rs:724-734; qsl-analyze/tests/it/refinement_liveness.rs:460-502 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | M12 (a_loop_open_in_the_concrete_model_is_refused), M15, M16, M19 (a_loop_reproduces_only_when_the_recorded_constraint_is_owed_and_never_taken) are each killed by the PR suite. |
| FND-002 | fixed | M08, M09 and M26 are killed, as is M27' (wrong Undecided cause). The per-class poll is removed: the original M10 is equivalent, since `analyze` polls the same handle. In-analyze cancel is untested: FND-007. |
| FND-003 | fixed | Spec ruling merged on main (#1117): ADR-020 AX-6 and FR-143's strong bullet settle strong `F_A` as `unsupported`, and the code matches. The branch still has to merge main: FND-006. |
| FND-004 | fixed | M06, M07, M17, M18, M23 and M25 are each killed by a new PR-suite test. |
| FND-005 | fixed | FR-143 on main (#1117) names `Checked(HoldsReduced{reductions: []})` and the typed `UnavailabilityCause`. |
