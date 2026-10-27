---
id: SR-1637002
title: "gap analysis of PR #1637 (QSL-582, FR-245 zone certificate checker)"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1637 git diff origin/qsl-581d-timed-replay...382cafe32; FR-245 AC-1 to AC-7, FR-244 (types moved), FR-235-AC-2, QSpec FR-331 zone loci; TC-700, TC-1197, TC-690; qsl-analyze/tests/it/{zone_certificate.rs,zone_search.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-700
    type: reviews
---
# SR-1637002: gap analysis of PR #1637 (QSL-582, FR-245 zone certificate checker)

## Summary

Ticket: QSL-582. Head reviewed: 382cafe32. Method: a manual check of each AC against its tests over the diff. I re-measured the PR body's claims in the code, in the test run and in Linear (QSL-582, QSL-583 and QSL-580 comments). Plan completion: not assessed. The soundness defects are in SR-1637001. This file covers AC coverage and deferrals.

| AC | PR claim | Measured |
| --- | --- | --- |
| FR-245-AC-1 | partial | Partial, as claimed. FR-244-AC-1's certificate and the deadlock-freedom certificate are `Accepted` (`the_search_certificates_are_accepted`). The AC-2 and AC-3 certificates need FR-240's components and divergence parts, which EN-6 does not emit yet. That deferral is sound, and a QSL-583 comment of 2026-10-26 records it. |
| FR-245-AC-2 / AC-7 | partial | Partial, as claimed. The node removal, the shrunk zone, the lowered LU bound, the cut `admitted`, the misplaced initial node and the bad node (over Lock) are each rejected at the right rule and locus. The backward edge, the component reason and `components` absent are deferred to QSL-583 (recorded). The loci serialize in FR-331's forms. The tampering set missed five unsound acceptances (SR-1637001 FND-001 to FND-005). |
| FR-245-AC-3 | partial | Partial. The identity mismatch refuses `stale_dependency`, naming both, and `max_certificate_edges` 1 and `max_certificate_bytes` 1 stop at that limit and value. The source-edit half is deferred to "QSL-580 (580d)", but QSL-580 has no comment that records FR-245-AC-3 (FND-002). |
| FR-245-AC-4 | covered by QSL-579 | Unchanged by this diff. The Kani harnesses for close, constrain, reset, up, includes, aLU and extrapolate are in `zone/proofs.rs`. They were not re-run because the diff adds none. |
| FR-245-AC-5 | covered | Covered. Lock is accepted, the unmarked node is `BadState`, and the variant's `locked` node is `BadState`. The same transplant plus one unconstrained clock is accepted (SR-1637001 FND-004). |
| FR-245-AC-6 | holds | Holds. qsl-replay's `Cargo.toml` names neither qsl-analyze nor qsl-route. |
| FR-235-AC-2 | partial | Partial, as claimed. `NoLateReply` settles `Proved{ZoneCertified, Certified}` through the real checker, and with one node removed it settles `CertificateRejected{SuccessorMissing, ZoneNode}`, so the CertificateRejected arm is tested. The `Settles` (TT-3) half is deferred to QSL-583 (recorded). |

Judgement on each deferral:
- **(a) TT-2 window claims refused as `UnsupportedItem`: buildable now.** `qsl-analyze/src/zone_check/window.rs` (727 lines) imports only `qsl_eval`, `qsl_foundation`, `qsl_semantics` and `quire_exact`, and qsl-eval's `Cargo.toml` already depends on all of them. Moving it into qsl-eval needs no new edge and does not need MightyL: QSL-583 owns only the nested-window case, which #1631 already refuses. EN-6 emits TT-2 certificates today (FR-242-AC-2 proofs in `zone_search.rs`), and none of them can reach `Certified`. This is a gap in this PR (FND-001).
- **(b) CF-2 `components` / `divergence`, liveness and time-lock-freedom certificates, and `Settles`: sound.** EN-6 produces none of them yet. QSL-583 records them in a comment.
- **(c) The FR-098 source-edit refusal waits on 580d: sound in substance.** Recompiling a timed unit needs S4 timed emission, which QSL-580 still lacks. But the deferral is not recorded on QSL-580 (FND-002), and until then the identity binding is only as good as the caller (SR-1637001 FND-006).
- **(d) No FR-331 schema validation of the written causes: not an AC.** FR-245-AC-7 asks for FR-331's rule spelling and locus forms. The rule spellings are asserted in `qsl-replay/src/outcome.rs`, the zone loci in `zone_loci_are_written_in_their_fr331_forms`, and no other certificate checker in the repo has a schema harness. No finding.

"Every EN-6 certificate in the suite is accepted" is overstated (FND-003). In the suite, only the certificates passed to `assert_edges_cover` or `zone_certificate.rs` reach the checker. These EN-6 proofs never do:
- the tick-source proofs in `a_tick_source_admits_by_a_tick` (a probe I ran gave `Accepted` for both);
- the `tt`-profile `NoLateReply` proofs;
- the `lock(false)` proofs in the locked-node and limit tests;
- the TT-2 proofs, which would be refused.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TT-2 window-claim certificates are refused as `UnsupportedItem`, though the move that would let the checker read them can be built now. `window.rs` depends only on crates qsl-eval already depends on, and MightyL is needed only for nested windows. FR-245's Behavior asks the checker to check every zone proof, and its bounds rule names "the claim automaton, built by qsl-eval's property-automaton translation". EN-6 already emits TT-2 proofs (FR-242-AC-2), and they cannot settle `Certified`. Fix in this PR: move the window automaton into qsl-eval, use it from EN-6 and the checker, and accept the automaton state, the horizon clock and stutter edges in `members()`. Or have the lead record an explicit owner decision on QSL-583. | qsl-replay/src/certificate/zone_certificate.rs:40-44,302-324,453-468; qsl-analyze/src/zone_check/window.rs |
| FND-002 | low | The deferral of FR-245-AC-3's source-edit half ("after QSL-580 (580d)", in the PR body) is not recorded on QSL-580. None of its comments names FR-245-AC-3 or 580d. Add a scope note to QSL-580, so the AC does not drop out when 580d lands. | Linear QSL-580; qsl-analyze/tests/it/zone_certificate.rs:201-206 |
| FND-003 | low | The PR body says every EN-6 certificate in the suite is accepted. The tick-source certificates (`a_tick_source_admits_by_a_tick`), the `tt`-profile proofs and the `lock(false)` proofs in `zone_search.rs` never reach the checker. So the checker's tick-source branches (`Terminal` targets, tick cycles, the tick Zeno rule) have no acceptance assertion, although a probe accepts them. Add `assert_edges_cover` to those tests. | qsl-analyze/tests/it/zone_search.rs:950-978,1240-1270,500-515 |
| FND-004 | medium | The tests for FR-245-AC-2 and AC-5 back the listed tamperings but none of the soundness boundaries the checker relies on. There is no test for a nondeterministic post under one transition identity, for deadlock under aLU coverage, for a node zone outside its invariant, for an unreset clock with no lower bound in a Zeno set, or for a fractional non-maximal constant. Each of these accepts a false certificate today (SR-1637001 FND-001 to FND-005). Add the review's probes as regression tests tagged `TC-700` / `FR-245-AC-2`. | qsl-analyze/tests/it/zone_certificate.rs:118-199,268-343 |

## Verdict

**Not mergeable** while SR-1637001 FND-001 to FND-005 stand. On coverage alone, the PR backs the parts of FR-245 it claims, and deferrals (b), (c) and (d) are sound. Deferral (a) is not forced: the TT-2 automaton can move into qsl-eval now, so this PR should either do it or record an explicit owner decision on QSL-583 (FND-001). FND-004 is the test half of the soundness fixes. FND-002 and FND-003 are bookkeeping and test-tightening.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | FR-245-AC-1's tests have no certificate whose `admitted` is a cycle that leaves a clock unreset. The suite's cycle witnesses reset every clock. So the new cycle-repetition check rejects honest EN-6 certificates and no test notices (SR-1637001 FND-012). Add the drifting-clock model (`loop`: `y >= 1`, resets `y`, `y <= 2` in phase 0, `x` free) as an acceptance test for the clause and the deadlock-freedom item. | qsl-analyze/tests/it/zone_certificate.rs; qsl-analyze/tests/it/zone_search.rs |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `WindowClaim` moved into `qsl_eval::automaton::timed_window`, and `qsl-analyze/src/zone_check/window.rs` is gone. The checker rebuilds the window automaton and binds certificate automaton numbers to its own states one to one. The TT-2 punctual proof is accepted (`a_punctual_window_is_decided_by_a_horizon_clock`). Binding probes: a permutation is accepted; collapsed numbers, all-zero numbers, a dropped horizon, zeroed horizon `L`/`U` and transplants onto refuted worlds are all rejected. |
| FND-002 | fixed | QSL-580's comment of 2026-10-26 21:10 records that FR-245-AC-3's source-edit half is pending 580d. |
| FND-003 | fixed | `assert_edges_cover` now runs on the tick-source, `tt`-profile, `lock(false)`, deadlock and TT-2 proofs (16 sites). |
| FND-004 | fixed | `a_successor_is_matched_by_its_post_state`, `a_deadlock_is_not_hidden_by_coverage`, `an_admitted_path_is_one_some_behaviour_takes`, `a_zeno_set_is_bounded_by_its_own_invariants` and `every_constant_is_exact_at_the_scale` carry the five probes. All pass. |

### Round 2

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | fixed | `a_cycle_that_leaves_a_clock_unreset_is_accepted` accepts the drifting-clock model's honest certificates for the clause and the deadlock-freedom item, and stops with `CertificateMaxEdges` when the budget is the listed edges alone. `an_identity_mismatch_names_both_identities` covers the identity refusal. |
