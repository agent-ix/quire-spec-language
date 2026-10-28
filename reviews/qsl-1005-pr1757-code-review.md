---
id: SR-1757001
title: "PR #1757 (QSL-1005c) code review: FR-244 refinement zone certificate and FR-245 refinement checker"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; git diff 4177e8021...b8392b9f4 (qsl-replay certificate/zone_certificate.rs, certificate/simulation.rs, certificate.rs; qsl-analyze zone_check/refine.rs, settle.rs, certificate.rs, search.rs, analyze/items.rs)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
---

# PR #1757 code review (Rust)

## Summary

Ticket: QSL-1005 (1005c). Head b8392b9f4; the base is #1755 at 4177e8021.

### Proved needs Accepted

`settle_timed_refinement`'s `Holds` and `settle_zone_check`'s `Holds` both go through the shared `certified` helper. It settles `Proved{ZoneCertified, Certified}` only on `Ok(Accepted)`:
- `Rejected` is `Inconclusive(CertificateRejected)`.
- `Stopped` is `Incomplete`.
- An identity, obligation or refinement mismatch is `Inconclusive(stale_dependency)`.
- Every other refusal, including `Package`, is `Failed`.

`timed_refinement_item` passes `check_zone_certificate` over its own `ZoneCertificateRequest`. In refinement form the checker re-opens both subjects with `open_systems` from the request, and runs `check_initial` and `check_step` with the request's own refinement. Nothing the certificate lists is trusted except positions, which it compares by digest.

### The checker in refinement form

- **Initial states.**
  - Each concrete initial state with a non-empty settled zone is paired, in index order, with `initial[i]`. That node must cover the state (`InitialMissing`).
  - It must also either pass `check_initial` with exactly the listed position while not `locked`, or fail it as a `locked` entry with the reserved index (`InitialMismatch`).
- **Edges out of a positioned node.** Every edge that covers a recomputed successor runs `check_step` from the position that node's index holds.
  - A passing step needs a non-`locked` target with the yielded position and the edge's `taken` (`PostDiffers`).
  - A failing step (`Fails`, `Undefined`, or a non-meter `Undetermined`) needs a `locked` entry with no `taken`. A non-entry target gives `StepFails`; a `taken` gives `PostDiffers`.
- **The locked region.**
  - **Edges.** Edges out of a reserved node must stay reserved with no `taken`.
  - **Entry.** The region can only be entered through a `locked` entry, and divergence propagates backwards over covered edges. So every reserved node lies in the forward closure of some `locked` node, and that node is rejected `BadState` if anything in the closure idles or has a non-Zeno component.
- **Evaluation meter.** An `Undetermined` caused only by the evaluation meter halts with `Stopped(LimitReached{EvaluationMeter})`, never `Accepted` or `Rejected`.
- **Unchecked nodes.** A node whose index has no known position is reached only through an edge whose rejection is already recorded. `verify` reports every recorded rejection before it can accept.

### Forgeries tried (probe_1757.diff), each rejected

- **Dropped edges.** Removing edge 0 or edge 1 of TReq gives `SuccessorMissing`.
- **Mispositioned nodes.**
  - Moving the initial node to any other index gives `InitialMismatch`.
  - Moving the successor to any other index gives `PostDiffers`.
- **Passing steps into the locked region.** A passing edge redirected to a reserved node, unmarked or `locked`, with no `taken`, gives `PostDiffers`.
- **Positioned initial marked as an entry.** It gives `InitialMismatch`.
- **Wrong `taken`.** Setting `taken` to `None` on each edge gives `PostDiffers`.
- **`dead_undo`'s certificate on `live_undo`.** `live_undo` has the same states with no `Dead` invariant, so its half is `Violated`.
  - Unchanged, the certificate gives `SuccessorMissing`.
  - With its locked node widened to the universe DBM, it gives `BadState`.
- **The entry's mark removed.** It gives `StepFails`.

A universe DBM on a TReq node is `Accepted`. That over-approximation is sound: the refinement holds there.

### Spec-gap readings (STD-493 26f4d8e0)

- **`{edge}` without a verdict.** `StepFails` at `{edge}` without the failing verdict drops a diagnostic only. The rejection, rule and edge are exact, so I accept it until FR-331 is amended.
- **The refinement named by name, not `NodeKey`.** The refinement node is still bound: the item identity's requirement occurrence is keyed by the refinement's V2 node. The checker also uses the request's recompiled refinement, so this has no soundness effect.

### Gates at b8392b9f4

`--all-features`, debug info off; recorded in the Verdict.

## Verdict

**Clean.** Nothing I tried made the checker accept a refinement-form certificate for a refinement that does not hold.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
