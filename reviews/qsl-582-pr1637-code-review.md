---
id: SR-1637001
title: "code-review and rust-review of PR #1637 (QSL-582, FR-245 zone certificate checker)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1637 git diff origin/qsl-581d-timed-replay...382cafe32: qsl-replay/src/certificate/zone_certificate.rs, qsl-replay/src/certificate/{zone.rs,zone/tests.rs}, qsl-replay/src/certificate.rs, qsl-replay/src/lib.rs, qsl-replay/src/model_check/limits.rs, qsl-analyze/src/zone_check/{certificate.rs,settle.rs,search.rs}, qsl-analyze/src/zone_check.rs, qsl-analyze/tests/it/{zone_certificate.rs,zone_search.rs,zone_replay.rs,timed_world.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: reviews
---
# SR-1637001: code-review and rust-review of PR #1637 (QSL-582, FR-245 zone certificate checker)

## Summary

Ticket: QSL-582. Head reviewed: 382cafe32 (`qsl-582-cert-checker`), scoped to the diff against `origin/qsl-581d-timed-replay` (#1634, reviewed separately). The review was led by soundness: can `check_zone_certificate` return `Accepted` for a certificate that an honest EN-6 run could not emit?

It can, in five ways. I reproduced each one with temporary probe tests in the review worktree. Each probe built a certificate from a real EN-6 proof, either a sibling model's proof transplanted the way FR-245-AC-5's test does it, or a proof with one member edited. EN-6's honest run on the real model gives `Violated` or `Undecided` in every case, and the checker still returns `Accepted`:

| Probe | Honest EN-6 on the real model | Checker on the forged certificate |
| --- | --- | --- |
| `flip` posts `phase = 1 or phase = 2` under one transition identity; the certificate lists only the phase-1 edge (FND-001) | `Violated` | `Accepted` |
| deadlock-freedom; `go` is guarded by `x <= 1`; the node zone is capped at `x <= 1` (FND-002) | `Undecided(DeadlockAfterDelay)`; the honest-zone certificate is `BadState` | `Accepted` |
| phase 0 has `x <= 1`; `go` needs `x >= 2`; `admitted` goes through `go` to quiescent phase 1 (FND-003) | `Undecided(NoAdmittedBehaviour)` | `Accepted` |
| `Lock` with a second clock `w` that only an unreachable invariant names; the plain certificate is transplanted onto the `ping`-resets-`x` variant (FND-004) | `Violated` | `Accepted` |
| `bad` is guarded by `x < 1/2 ms` and another clock bound is 3; the certificate has scale 1 (FND-005) | `Violated` | `Accepted` |

These tamperings were rejected with the right rule and locus:
- the scale doubled, set to 3, or set to 0: `Malformed` at `bounds` or `scale`;
- `initial` emptied: `InitialMissing` at the initial `ModelState`;
- `admitted` given an empty cycle, routed through Lock's locked `ping` cycle, or ended at the locked node as `Quiescent`: `WitnessFails` at its last node;
- the PR's own six tamperings.

What I traced and found correct:
- **Types moved, no copy left.** `ZoneCertificate`, `CertNode`, `CertEdge`, `DivergenceWitness`, `DivergentTarget` and `LuBounds` are each defined once, in qsl-replay. qsl-analyze imports them, and `ZoneCertificateCheck` is deleted.
- **The obligation is generic.** `ZoneCertificate<O>` takes any obligation type. qsl-replay's `Cargo.toml` names neither qsl-analyze nor qsl-route, so there is no dependency cycle.
- **The aLU test** is Herbreteau's: it treats an absent bound as `-∞`, it is Kani-checked by QSL-579, and the seeded tests of `down`, `minus` and `from_matrix` are sound.
- **Order of checks.** Identity, then budgets, then members, then initial coverage, then nodes in node order, then `admitted`. `reach` only recomputes and rejects nothing. This matches FR-245's order.
- **Wiring.** `settle_zone_check` maps `Accepted`, `Rejected` and `Stopped` with no catch-all arm. EN-6's certificate tests (`assert_edges_cover`) now run the core checker.

Gates, run with locked-build at jobs 4 and `--all-features`:
- qsl-analyze `zone_`: 51/51.
- qsl-replay lib `certificate`, `limits` and `proof_result`: 80/80.
- `clippy -p qsl-replay -p qsl-analyze --all-targets -D warnings`: exit 0.

Kani was not re-run, because this diff adds no harnesses. The probe source is kept in the review scratchpad (`probe_1637.rs`).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `node()` treats a successor as covered when any valid edge from the node has the same transition identity and guard part. It does not check that this successor's post-state digest matches that edge's target, or that the target's zone covers this successor's zone. An operation with a nondeterministic post has several `GuardedStep`s under one identity. A certificate that lists only the phase-1 edge is therefore accepted, and the phase-2 successor is never explored. Probe: `flip` posts `phase = 1 or phase = 2` and the claim is `always holds(not (c.phase = 2))`. EN-6 gives `Violated`; the checker gives `Accepted`. `step_of` also picks the first successor with a matching identity and part. Fix: in `node()`, require for each successor an edge with the same transition and part whose target `state_key` equals `successor.digest` and whose zone aLU-covers `successor.zone`. Select `step_of` by digest too. Add the probe as a regression test. This is the same bug class as SR-1634001 FND-002. | qsl-replay/src/certificate/zone_certificate.rs:654-672,851-865 |
| FND-002 | high | `deadlocked()` judges deadlock on the node's zone, but the node is reached under aLU coverage with the certificate's LU bounds. LU simulation does not preserve deadlocks: a valuation that enables nothing can be simulated by one that enables a step. Probe: `go` is guarded by `x <= 1` (so `L_x` is absent and `U_x = 1`). The initial node's zone is capped at `x <= 1`, and it aLU-covers the real `{x >= 0}`. Every valuation of the capped zone enables `go`, so the deadlock-freedom item is `Accepted`. The real model deadlocks at `x > 1`, and EN-6 gives `Undecided(DeadlockAfterDelay)`. Fix: for the deadlock-freedom item, compute and require `L = U = max(L, U)` per clock (an M-simulation, which preserves enabledness), or judge deadlock on the recomputed successor and initial zones. | qsl-replay/src/certificate/zone_certificate.rs:714-767,1023-1084 |
| FND-003 | high | `successors_of` takes the node's zone as given. It never intersects that zone with the source state's own invariants, and never checks that the zone holds only reachable valuations. A node zone that is too large enables edges no real behaviour takes, and `admitted()` follows certificate edges over those zones, so TD-6's vacuity witness can be forged. Probe: phase 0 has `x <= 1` and `go` needs `x >= 2`, so every behaviour is time-locked. A certificate whose node 0 has the zone `{x >= 0}` has a valid `go` edge to quiescent phase 1, and `admitted` = [`go`] is `Accepted`. EN-6 gives `Undecided(NoAdmittedBehaviour)`, so FR-235 would settle `Proved` where it should settle `inconclusive`. Fix: constrain each node's zone by its state's invariants before computing successors, and reject `Malformed` at `nodes` when the zone leaves them. Check `admitted` by recomputing the exact zones forward from the initial zone along its path, not over node zones. | qsl-replay/src/certificate/zone_certificate.rs:619-650,914-1004 |
| FND-004 | high | The Zeno test in `zeno()` ("some clock that no edge resets is bounded above at every member") reads the certificate's node zones. Those zones only aLU-cover the real valuations, so a clock with no lower-bound constant (`L` absent) can be bounded in every node zone while it grows without bound in the model. Probe: `Lock` plus a clock `w` that only an unreachable invariant names. The plain certificate is transplanted onto the `ping`-resets-`x` variant, as FR-245-AC-5's test does. `w == x <= 1` in every node, so the `ping` SCC is judged Zeno and the `locked` node passes, giving `Accepted`. The model diverges through `ping`, and EN-6 gives `Violated`. This answers the brief's question: the "sufficient conditions" rule does accept a time-divergent cycle as Zeno. Fix: bound the clock by the member states' own invariants (recomputed from the model, which constrain every real valuation), not by node zones. | qsl-replay/src/certificate/zone_certificate.rs:871-910,772-834 |
| FND-005 | high | `computed_bounds` checks only each clock's largest constant for integrality under the certificate's scale. `Checker::scaled` then maps any other non-integral constant to 0 with `unwrap_or_else(Integer::zero)`. A certificate whose scale is too small has its guards and invariants silently rewritten: `x < 1/2` becomes `x < 0`, which is empty. Probe: `bad` is guarded by `x < 1/2 ms`, and an unreachable invariant gives `x <= 3 ms`. The certificate has scale 1 and `U = 3`. The `bad` step is never enabled, so the result is `Accepted`. EN-6 gives `Violated`. Fix: check every guard, invariant and reset constant (not only the maxima) and reject `Malformed` at `scale`. Make `scaled` return `Option` all the way down, with no zero fallback. | qsl-replay/src/certificate/zone_certificate.rs:482-486,1007-1013,1070-1083 |
| FND-006 | medium | The identity check compares the certificate's obligation only with the caller's `item: &O`. Nothing ties `O` to `request.item` (`ModelCheckItem`) or to the recompiled package. A caller can pass a deadlock-freedom obligation with a clause item, or the reverse, and get `Accepted` for the wrong item. FR-245 also refuses when the certificate differs "from the recompiled item's identity", and the generic `O` cannot do that. Fix: derive the checked item from the obligation (a small trait on `O` that yields the `ModelCheckItem` kind and subject) or check that they agree, until FR-098's request lands with 580d. | qsl-replay/src/certificate/zone_certificate.rs:285-324 |
| FND-007 | medium | The work is quadratic or worse, while the budgets only bound input size. `node()` rescans every edge for every reached node, even though `reach()` already built the `leaving` map. `diverges_after()` rescans every edge for each BFS step, once per locked node. At the default 2^24 edges this is about 2^48 edge visits. Cancellation is polled per node, but no budget stops it. Fix: build the `from → edges` index once and reuse it in `node()`, `diverges_after()` and `admitted()`. Compute SCCs once for the whole valid-edge graph. | qsl-replay/src/certificate/zone_certificate.rs:655-662,772-783 |
| FND-008 | low | `diverges_after` calls `components(&graph, &within, &Cancel::new())`, so the caller's `Cancel` is not honoured inside the SCC computation. Pass `self.cancel` and map its stop to `Stopped(Cancelled)`. | qsl-replay/src/certificate/zone_certificate.rs:813-814 |
| FND-009 | low | `qsl-replay/src/lib.rs` still has `#[allow(dead_code, reason = "the zone certificate checker is its first caller and has not landed yet")]` on `mod certificate`, and a comment saying zone arithmetic stays crate-private. The checker has landed and `Zone` is now re-exported, so both are false. The allow now hides dead code across every certificate module. Delete the allow and the comment. | qsl-replay/src/lib.rs:52-59 |
| FND-010 | low | `ZoneCertificateRefusal::Fault(&'static str)` carries the discriminant as a string literal (`"zone-shape"`, `"graph"`, `"components"`, `"a-reached-node"`, `"a-valid-edge-has-a-step"`). Callers and tests can match only on `Fault(_)`. Make it a typed enum, or one variant per invariant. | qsl-replay/src/certificate/zone_certificate.rs:222-224,246-250,684-690,862-864 |
| FND-011 | low | `certificate_bytes` is a size estimate with fixed costs (40 per node, 48 per edge, 8 per witness edge) and is not the bytes the check reads. FR-245's `max_certificate_bytes` names "certificate bytes". State in the doc that this is the canonical-encoding size bound, or measure the FR-418 wire encoding once that exists. | qsl-replay/src/certificate/zone_certificate.rs:1086-1119 |

## Verdict

**Not mergeable.** FND-001 to FND-005 each let the qualified core accept a zone certificate for a claim that is false, or vacuously true, in the model, and FR-235 would then settle it `Proved{ZoneCertified, Certified}`. That defeats the checker's purpose (ADR-026 CF-3 and CF-6). Each fix needs its probe as a regression test in `zone_certificate.rs`. FND-002 to FND-004 share a root cause: badness, divergence and vacuity are judged over node zones, which only aLU-cover the real valuations. Judge them over the recomputed exact zones, or over the model's own invariants, instead. FND-006 and FND-007 are medium and belong in the same round. FND-008 to FND-011 are low. The type move, the generic obligation, the dependency layering, the check order and the settlement wiring are all correct.

## New findings (disposition pass 1)

Round 1 was reviewed at head 982b0235a, with `git diff origin/qsl-581d-timed-replay...982b0235a`. I re-ran all five original probes; the regression tests that carry them reject each one. I then ran these new probes:
- A diagonal vacuity forge: the node zone is inside its invariant, but its valuations cannot be reached. Rejected `WitnessFails`.
- A clause certificate checked as the deadlock item, and the reverse. Both refuse `stale_dependency`.
- Seven TT-2 tampers:
  - automaton numbers permuted: accepted, which is legitimate;
  - two numbers collapsed into one: `SuccessorMissing`;
  - every number set to 0: `SuccessorMissing`;
  - the horizon piece dropped: `SuccessorMissing`;
  - the horizon `L`/`U` zeroed: `Malformed` at `bounds`;
  - the 2 ms certificate moved onto the 5/2 ms and 3 ms worlds, which EN-6 refutes: `InitialMissing`, `SuccessorMissing` or `Malformed` at `scale`;
  - every node marked `locked`: `BadState`.
- Cycle targets, which turned up FND-012.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | high | `repeats` starts the greatest fixpoint from the exact zone the `admitted` path reaches. It then keeps only valuations from which the cycle returns *into that set*. A cycle that leaves any clock unreset moves the clock differences away from their first-visit values, so it can never return, and the fixpoint empties. As a result an honest EN-6 certificate is rejected. Probe: `loop` is guarded `y >= 1`, resets `y`, and phase 0 has `y <= 2`, while `x` runs free. EN-6 gives `Holds` with `admitted` = `Cycle([0])` for both the clause and the deadlock-freedom item, and the checker rejects both `WitnessFails` at node 0. Round 0 accepted these certificates. This is not unsound (it only rejects), but a true proof would settle `inconclusive`, `CertificateRejected`. Fix: compute the fixpoint from the start state's whole invariant zone, not from the path's zone, and accept when it meets the path's exact zone. Add the probe as a regression test. | qsl-replay/src/certificate/zone_certificate.rs:1426-1463 |
| FND-013 | low | `CYCLE_ROUNDS = 64` is arbitrary. The cap can only reject, never accept: `repeats` returns `true` only when a round reaches a fixpoint, so its soundness holds. But a cap reached is reported as `WitnessFails`, which says the certificate is wrong. FR-245 says that reaching a limit returns `Stopped` naming the limit, its value and its setting. Either make it a `CertificateCheckLimits` budget that gives `Stopped(LimitReached)`, or derive the bound from the model's constants and the cycle's minimum duration. | qsl-replay/src/certificate/zone_certificate.rs:335-337,1426-1463 |
| FND-014 | low | `ZoneCertificateRefusal::UnknownClause` is not a refusal FR-245 names. Its code (`stale_dependency`) fits FR-245's "stale package" refusal. But FR-245 has an identity mismatch refused "naming both identities", and this refusal names neither identity. Map a clause missing from the recompiled package to `ObligationMismatch` (the certificate's obligation against the request's clause), or record the extra refusal as a spec gap on STD-493. | qsl-replay/src/certificate/zone_certificate.rs:367-377 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `reach` matches transition, part, horizon, post digest, coverage and the bound automaton state. `node` requires every successor to be covered. `a_successor_is_matched_by_its_post_state` rejects the flip probe with `SuccessorMissing` at node 0. |
| FND-002 | fixed | For the deadlock item, coverage uses `L = U = max(L, U)` (aM), in both the checker and EN-6, and the node zone is held to its invariants in `deadlocked`. `a_deadlock_is_not_hidden_by_coverage` rejects the capped zone with `InitialMissing`. |
| FND-003 | fixed | `successors_of` holds the node zone to its state's invariants, and `admitted` is recomputed forward in exact zones. The probe and a new diagonal probe are both rejected `WitnessFails`. The new cycle check has a completeness bug (FND-012). |
| FND-004 | fixed | `zeno` reads the clocks bounded by every member state's own invariants, or all members urgent. `a_zeno_set_is_bounded_by_its_own_invariants` gives `BadState` at the locked node. |
| FND-005 | fixed | `computed_bounds` checks every constant, the window's included, and `scaled` gives `Malformed` at `scale` with no zero fallback. `every_constant_is_exact_at_the_scale` passes. |
| FND-006 | fixed | `item_of` gives a `CertificateItem` that is compared with the recompiled clause occurrence or kind. Probes of a clause certificate checked as the deadlock item, and the reverse, refuse `stale_dependency`. The `UnknownClause` refusal is FND-014. |
| FND-007 | fixed | `leaving` is indexed once, `node` reads `covers`, and `divergent` computes the SCCs once and propagates back. |
| FND-008 | fixed | `components(&graph, &within, self.cancel)`, with the cancellation mapped to `Stopped(Cancelled)`. |
| FND-009 | fixed | The stale `#[allow(dead_code)]` and its comment are gone from `lib.rs`. |
| FND-010 | fixed | `Fault(CheckerFault)` is a typed enum. |
| FND-011 | fixed | The `certificate_bytes` doc now says it bounds the in-memory reading because FR-418's encoding does not exist yet (STD-493). |
