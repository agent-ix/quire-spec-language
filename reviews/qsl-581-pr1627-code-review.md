---
id: SR-1627001
title: "code-review and rust-review of PR #1627 (QSL-581b, FR-244 zone certificate)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1627 git diff origin/main...32411b40c: qsl-analyze/src/zone_check/{certificate.rs,search.rs}, qsl-analyze/src/zone_check.rs, qsl-analyze/tests/it/zone_search.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: reviews
---
# SR-1627001: code-review and rust-review of PR #1627 (QSL-581b, FR-244 zone certificate)

## Summary

Ticket: QSL-581. Head reviewed: 32411b40c. I traced the certificate builder (`Run::certificate`), the `admitted` witness search (`admitted`, `cycle_through`, `cycle_delays`), the obligation plumbing (`ZoneCheckRequest::obligation`, `ZoneCheckOutcome::proving`) and the deletion of `SymbolicGraph`. I also compared the witness rule with FR-239's own vacuity test (`divergence.rs`, Tripakis `z` reset inside an SCC), FR-240's "cycle with positive total delay", and FR-245's acceptance rules.

The search's vacuity verdict is still sound. The `admitted` witness is not. It is computed by a second, weaker rule instead of being taken from the vacuity test. A cycle counts as "positive total delay" when one trip around it, from some valuation of the head's zone, lets `z > 0`. That admits Zeno cycles. Only the shortest cycle through each SCC's lowest node is tried. It also ignores tick sources. I reproduced three defects with probe tests in my own detached worktree. The probes were reverted and never pushed.

1. **Zeno witness.** Initial phase 0 has `x <= 1 ms`, `x` is never reset, and `ping` changes nothing. `go` leads to phase 1, where `tock` (`y >= 1`, resets `y`) diverges. Result: `admitted = {start: 0, path: [], target: Cycle([ping])}`, a loop whose total time is bounded by 1.
2. **False `Undecided`.** Phase 0 is urgent (`x <= 0 ms`) and has a zero-delay `ping` loop. `go`/`back` (`x >= 1`, resets `x`) loops through phase 1 and diverges. Result: `Undecided { cause: NoDivergenceWitness }` with 2 states and 3 edges, though the claim holds and the vacuity test admitted a behaviour.
3. **Tick source.** Phase 0 has only `work` and `go`, and `tick` is enabled in phase 1 only. Result: `admitted = Quiescent(0)`. Node 0 never ticks, so it is not divergent under FR-231 "Digital time".

The PR's 13 `zone_search` tests pass at the head, run under locked-build with `--all-features`. No other crate consumes `check_timed_model`, `SymbolicGraph` or `ZoneCheckOutcome`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `cycle_delays` accepts a Zeno cycle as the TD-6 divergent target. It checks that one trip around the cycle, from some valuation of the head zone, gives `z > 0`. FR-240 and ADR-026 EZ-6 mean a time-divergent cycle (the non-Zeno construction), and CT-2 states the loop-closure conditions. Reproduced: a phase with `x <= 1 ms`, `x` never reset and a no-op `ping` gets `admitted = Cycle([ping])`, though a real divergent `go`/`tock` loop exists. FR-244-AC-4's own `Lock` `(B, x <= 1)` `ping` loop passes this test too. So a checker using the same rule would reject AC-4's certificate under FR-245-AC-5 (`BadState`: a locked node reaches a positive-delay cycle), and a checker that differs would reject this witness. The certificate certifies a time-locked behaviour as the admitted one. Fix: take the witness from the vacuity test's own accepting SCC (a `z >= 1` reset inside an SCC of the `z`-augmented graph), or run that test on the certificate's component, not one traversal. | qsl-analyze/src/zone_check/certificate.rs:216-337 |
| FND-002 | high | `ZoneUndecided::NoDivergenceWitness` is not a spec gap. The witness search tries only the shortest cycle through each SCC's lowest-numbered node (`cycle_through(head, ..)`), and carries exact zones from that node, so it misses divergent cycles that the vacuity test found. Reproduced: an urgent phase 0 with a zero-delay `ping` self-loop, plus a divergent `go`/`back` loop through phase 1, returns `Undecided(NoDivergenceWitness)` for a claim that holds. FR-239 and TD-6 say a run whose initial state reaches a time-divergent SCC proves, and that SCC is the witness. The cause exists only because the witness rule is weaker than the vacuity rule, and STD-493 does not record it (see SR-1627002). Fix it with FND-001: build `admitted` from the same divergence evidence the vacuity check accepted, and delete the cause. | qsl-analyze/src/zone_check/search.rs:134-138,598-604; qsl-analyze/src/zone_check/certificate.rs:286-302,340-372 |
| FND-003 | high | Under a tick source, `admitted` uses dense-time rules. `is_quiescent` (no invariants and not urgent) is taken as divergent. FR-231 "Digital time" and `divergence.rs` count divergence as infinitely many ticks, or a terminal state's stutter. `cycle_delays` can never pass, because `elapses` is false under a tick source and `z` stays 0. Reproduced: a tick subject whose phase 0 has only `work` and `go` (with `tick` enabled only in phase 1) gets `admitted = Quiescent(0)`, a node from which staying never ticks. And a tick subject in which every reachable state has an invariant can only reach `NoDivergenceWitness`. This is the same class as SR-1599001 FND-001. Fix: under a tick source, the target is a terminal node, or an SCC with a tick edge. | qsl-analyze/src/zone_check/certificate.rs:258-268,306-337 |
| FND-004 | low | `check_timed_model` fills the certificate with a placeholder `ItemObligation { subject: (), kind: ObligationKind::DeadlockFreedom }` for every item, TT-1 included. It then re-wraps the outcome through two `proving` conversions. The request's `obligation` is never checked against `request.item`, so an `Invariant` item can be certified as `DeadlockFreedom`. Simpler and safer: pass the obligation into `Run`, or build the certificate after `search` returns, and assert that the kind matches the item. | qsl-analyze/src/zone_check/search.rs:326-353,589-596; qsl-analyze/src/zone_check/certificate.rs:116-131 |
| FND-005 | low | `CertComponent` is exported but never built. It has only `nodes` and lacks the per-component reason that FR-244 "Büchi part" requires. It is a speculative placeholder for QSL-583. Delete it and type `components` when FR-240 lands, or give it the reason now. | qsl-analyze/src/zone_check/certificate.rs:89-94; qsl-analyze/src/zone_check.rs:17-20 |
| FND-006 | low | `CertEdge` is one edge per convex guard part (`part`). FR-244 and CF-1 say one edge per transition identity enabled from a node. Per-part edges are needed when a guard is non-convex, because one target cannot cover a union of zones. That is a real spec gap, but it is neither stated in the code nor filed. Raise it on STD-493. | qsl-analyze/src/zone_check/certificate.rs:75-87 |
| FND-007 | low | `cycle_through` spells `std::collections::BTreeMap` and `btree_map::Entry` inline, though the file already imports from `std::collections`. Import them at the top. | qsl-analyze/src/zone_check/certificate.rs:346,366 |

## Verdict

**Not mergeable.** FND-001 to FND-003 are one root cause: the `admitted` witness is computed by a rule that differs from, and is weaker than, the vacuity test that admitted the run. As a result the certificate can name a Zeno cycle (FND-001) or a non-ticking node (FND-003) as the admitted behaviour, or report `Undecided` for a valid proof (FND-002). Derive the witness from the vacuity test's divergence evidence, dense and tick alike. Then FND-002's cause goes away. Fix FND-004 to FND-007 in the same round.

Correct: the node and edge projection from the retained graph, discovery order and stable edge order (deterministic: equal runs give equal certificates), state-key digests, `lu_bounds` (one global pair per place is a valid instance of FR-244's per-state bounds), Quiescent witnesses through covering edges (coverage keeps the discrete state, so reaching a quiescent discrete state is real), the multi-root `initial`, and the deletion of `SymbolicGraph` with no remaining consumer and no shim.

Rust lane: no `unwrap` or panic on production paths; out-of-range lookups return `Halt::Refusal(Fault)`. `admitted` polls the cancel handle. `cycle_delays` and `cycle_through` are bounded by the graph and run no recursion. `ZoneCertificate<S>` goes through `Box` in the outcome, which keeps the enum small.

## Coverage

Methods: code-review with the rust-review lane. Gap analysis is SR-1627002. The probe tests were run in the reviewer's own worktree under locked-build, then discarded.
