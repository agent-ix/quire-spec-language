---
id: SR-1700001
title: "code review (with rust-review) of PR #1700 (QSL-1005c) against FR-430-AC-8, FR-244-AC-5, FR-245-AC-8, TC-1170 steps 6-8"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1700 git diff origin/qsl-1005b-timed-refinement...2fd0cedfb; qsl-analyze/src/zone_check/{refine,settle,certificate}.rs, qsl-replay/src/certificate/zone_certificate.rs; spec text of QSL #1697 (STD-516)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
---
# SR-1700001: code review of PR #1700 (QSL-1005c)

## Summary

Ticket: QSL-1005. Head reviewed: 2fd0cedfb, base qsl-1005b-timed-refinement (ec07fbeab). Method: code-review with the rust-review lane, read against the FR-244/FR-245/FR-430 text of QSL #1697. The checker was probed with forged certificates and two mutants (temporary, reverted).

**The checker is sound against every forgery tried.** All were rejected:

| Forgery | Result |
| --- | --- |
| A passing edge's target moved into the locked region, with `locked` set and `taken` set to `None` | `PostDiffers` at edge 0 |
| `taken` set to `None` on a passing edge | `PostDiffers` |
| A failing edge into the entry with `taken` set to `Some` | `StepFails` |
| The initial node given another valid position | `InitialMismatch` |
| The initial node given the reserved index | `InitialMismatch` |
| An edge dropped | `SuccessorMissing` |
| The AC-6 certificate checked against the same model without the time invariant | Exact zones give `SuccessorMissing`. With every zone widened to the universe, the entry is divergent and gives `BadState` at the locked node. |

Reading the code confirms:
- **Reached nodes:** a node at a positioned index can only be reached by an initial or a passing edge, which records its position before the node is queued. A node at the reserved index can be reached only by a failing edge into a `locked` node, or from the locked region.
- **Locked region:** an edge leaving the region for a positioned node is `PostDiffers`, so an unjudged region cannot reach a judged one.
- **Stops:** the meter, the run budgets and faults give `Stopped`/fault, never `Rejected`. A mutant that drops the meter classification fails `a_refinement_certificate_check_out_of_meter_stops`, so the stop is at `check_step`.
- **Settlement:** `settle_timed_safety` gives `Proved{ZoneCertified, Certified}` only on `Accepted`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | **The StepFails locus omits the failing verdict.** FR-245's rule table says `StepFails` is located at "`ZoneEdge` of that edge, with the failing verdict as FR-149's locus carries it". FR-149's `CertificateLocus::SimulationStep` carries `verdict: Option<Box<FailedStep>>`, but `CertificateLocus::ZoneEdge` has only `edge`, and `refine_edge` discards the verdict. The spec is explicit, so build it here: add the same optional `verdict` to `ZoneEdge`, fill it for `StepFails`, and assert it in the AC-6 mark-removed test. | qsl-replay/src/certificate.rs:136; qsl-replay/src/certificate/zone_certificate.rs:1488-1499 |
| FND-002 | low | **An engine Holds that no certificate can pass.** With several initial states, where one fails `check_initial` while time-locked and another diverges, the engine skips the failing state (`locked_failure`, `continue`) and returns `Holds`. The checker's `initial_position` rejects every initial state whose `check_initial` fails, as FR-245 says ("a failing `check_initial` is rejected the same way"). That certificate always ends `CertificateRejected`, although FR-430 "A failing initial state" calls the failure vacuous. The spec does not say what such a mixed case settles. This is a spec question for STD; no fixture reaches it today. | qsl-analyze/src/zone_check/refine.rs:380-397; qsl-replay/src/certificate/zone_certificate.rs:1418-1457 |
| FND-003 | low | **`max_certificate_bytes` does not count the refinement part.** `certificate_bytes` counts the scale, the bounds, the nodes, the edges and the witnesses, but not `refinement.positions` or `taken`, so the size budget checked before any work does not cover the new member. Add both to the count. | qsl-replay/src/certificate/zone_certificate.rs:2548-2584 |
| FND-004 | low | **`check_step` runs after the DBM match, not before.** FR-245 says `check_step` runs on every edge out of a positioned node "before the DBM checks of that node". `refine_edge` runs only once an edge has matched a successor under aLU. A forged edge that both fails `check_step` and fails to cover therefore reports `SuccessorMissing` where the spec gives `StepFails`. Verdicts are unaffected; only the reported rule differs. | qsl-replay/src/certificate/zone_certificate.rs:1625-1643 |

## Verdict

The checker is sound, and `proved` is gated on `Accepted`. **Not mergeable until FND-001 is fixed**, together with SR-1700002 FND-001. FND-002 to FND-004 may be deferred with a ticket.
