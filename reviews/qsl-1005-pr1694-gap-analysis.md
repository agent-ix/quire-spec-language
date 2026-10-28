---
id: SR-1694002
title: "gap analysis of PR #1694 (QSL-1005b) against FR-430-AC-1, AC-2's TV-1 part, FR-142/145/149/241, ADR-026 TV-1, TC-1170"
type: SpecReview
analysis: gap-analysis
date: 2026-10-27
scope: "agent-ix/quire-spec-language; PR #1694 git diff origin/qsl-1005a2-reduction-negotiation...5c7b8371d; FR-430-AC-1, FR-430-AC-2 (TV-1 part); FR-142, FR-145, FR-149, FR-241; ADR-026 TV-1; TC-1170"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-430
    type: reviews
---
# SR-1694002: gap analysis of PR #1694 (QSL-1005b)

## Summary

Ticket: QSL-1005. Head reviewed: 5c7b8371d. Method: each AC checked against its tests, probes with EN-6 and S3, and the gates run. Plan completion: not assessed.

| Requirement | Measured |
| --- | --- |
| FR-430-AC-1 | Met, except routing, which is deferred to QSL-544 / IR-1014 as the PR states. `Holds` with 2 symbolic states and 2 edges; the clock read and the timed abstract model are refused as required. |
| FR-430-AC-2, TV-1 part | **Partial.** The trap variant `Holds` and the plain variant `Violated` with `send`@1, `enter`@1, `undo`@0. Not asserted: `RefinementFailure` at position 3, "replaying through FR-241", and the trap model's time-lock-freedom item refuted (FND-002). The TV-1 rule as built refutes too much (SR-1694001 FND-001). |
| FR-145 / FR-241 replay | **Not built for a timed counterexample** (FND-001). |
| FR-149 / ADR-026 RU-6 certificate | `Holds` carries none. This is a spec gap on STD-493 (c5171104), judged below. |

**Judging the missing certificate.** No core checker reads a zone product's simulation relation (FR-149 reads explicit successors, which ignore clock guards), so EN-6 has nothing to emit. This is acceptable while the record is not routed to EN-6. But ADR-026 RU-6 / §8 ("a zone certificate with every proof") means a timed refinement `Holds` cannot settle `proved` until STD-493 is decided. That must be settled before QSL-544 routes the record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **No replay for a timed refinement counterexample.** `replay_refinement_trace` (FR-145) re-executes the prefix and runs `check_step`. It checks no delay, no clock guard and no time-divergent closure (ADR-026 CT-3), and nothing in the PR replays EN-6's counterexample. FR-430-AC-2 requires "replaying through FR-241", and SM-7 admits no refutation that does not replay. Add the CT-3 arm to the refinement replay (delays, guards, invariants, and the closure from the last state), and replay the AC-2 counterexample in the test. Or name the ticket that carries it. | qsl-replay/src/model_check/refinement_replay.rs:96-140; qsl-analyze/tests/it/timed_refinement.rs (AC-2 test) |
| FND-002 | low | **FR-430-AC-2's other clauses are not asserted.** The plain variant's failure position is not checked (the AC says `RefinementFailure` at position 3), and the AC says the trap variant's "time-lock-freedom item of that model is refuted", which no test runs. Add both assertions. | qsl-analyze/tests/it/timed_refinement.rs:185-238 |

## Verdict

**Not mergeable together with SR-1694001.**

## Dispositions, round 1 (head ec07fbeab)


Round 1. Fix 5c7b8371d..ec07fbeab.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `replay_timed_refinement_trace`: the prefix is re-run over time by FR-237's rules, `check_initial` / `check_step` run along it, and a fresh closure is explored from the failing step's post point (`Divergent` reproduces, `Locked` is inconclusive). A forged early delay is refused (PR test). Details below. |
| FND-002 | fixed | `a_failing_undo_refutes_only_where_time_can_diverge` asserts the AC-2 positions, the trap model's time-lock-freedom refutation, and `Inconclusive` on the trapped model. |

**FND-001: trying to forge a Reproduced replay.**
- EN-6's counterexample from my open dead-trap variant (send@1, undo@0) replays as `Reproduced{StutterChanged{2}}` on the open model.
- Forged onto the trapped model, where the half holds, it replays as `Inconclusive(proved: Violation, replayed: Success)`.
- A forgery cannot reproduce on a model that holds. Reproduction needs a prefix that re-executes with admissible delays, guards and invariants, and ends at a failing step whose post point has a time-divergent continuation. That is an admitted violating behaviour.
- The two failures without a position (`Divergence`, `AbstractUnfair`) are lasso-only, and the timed replay refuses lassos.

**The certificate moves to 1005c.** The PR body says so: "Holds is uncertified in this PR ... That certificate and its checker are built in QSL-1005c. Until then `TimedSafetyOutcome::Holds` carries no certificate, so nothing settles `proved` from it."

**Verdict: mergeable.** No findings remain open on SR-1694001 or SR-1694002.

Gates at ec07fbeab (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=qspec-e3d48d76):
- fmt: exit 0.
- qsl-analyze `timed_refinement` / `zone_`: 113/113.
- qsl-semantics `refinement`: 85/85.
- qsl-replay `refinement`: 43/43; `model_check`: 6/6.
- root `refinement` / `family_outcome_layering`: 10/10.
- clippy with `-D warnings`: exit 0.

+++ [reviewer data]

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: 'Ok(match explore(&timed, point, exploration) { Ok(Closure::Divergent) => settlement, Ok(Closure::Locked) => RefinementTraceSettlement::Inconclusive(disagreement(Category::Success)),'
- fnd: FND-002
  outcome: fixed
  after_excerpt: 'fn a_failing_undo_refutes_only_where_time_can_diverge()'
```

+++
