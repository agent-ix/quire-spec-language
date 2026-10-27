---
id: SR-1647002
title: "gap-analysis of PR #1647 (QSL-583c) against FR-244-AC-2/3, FR-245-AC-1/2/7 and FR-241-AC-4"
type: SpecReview
analysis: gap-analysis
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1647 git diff origin/qsl-583b-zone-liveness...09259275c, against FR-244-AC-2, FR-244-AC-3, FR-245-AC-1, FR-245-AC-2, FR-245-AC-7, FR-241-AC-4 and the author's deferrals"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-244
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-245
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-241
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: reviews
---
# SR-1647002: gap-analysis of PR #1647 (QSL-583c) against FR-244-AC-2/3, FR-245-AC-1/2/7 and FR-241-AC-4

## Summary

Plan completion: not assessed. The ACs in scope are met:

- **FR-244-AC-2:** numbering, a reason per component, and `Weak(serve)` on the idle loop's component.
- **FR-244-AC-3:** a divergence part with a quiescent target for every node.
- **FR-245-AC-1:** each certificate is Accepted.
- **FR-245-AC-2/7:** BackwardEdge at the edge, wrong reason as WitnessFails at the component, missing components as Malformed before recomputation.

I treated the author's deferrals as claims and checked each:

- **FR-241-AC-4 "unreachable from a model": refuted.** Model (dense; clocks x, y):
  - `a`: phase 0 to 1, guard `x >= 1 ms`, resets `x`;
  - `b`: phase 1 to 0, guard `y > 1 ms`, resets `y`;
  - `time invariant when phase = 0 { x <= 1 ms }` and `time invariant when phase = 1 { x <= 1 ms }`;
  - claim `eventually holds(c.retried)`, where nothing sets `retried`.

  `a` fires every 1 time unit. `b`'s offset after `a` must strictly increase, so runs such as offsets `1 - 2^-n` diverge forever and refute the claim. No run is periodic: one loop iteration forces `y = 1` at `b`, against `y > 1`. EN-6 returns `Undecided { cause: LassoNotConcretized }` with 4 symbolic states.

  The STD-493 8eafddf9 argument fails at its premise. Büchi preservation by aM gives an infinite accepting run, not a one-period lasso. These convergence phenomena are known (Puri 2000). The end-to-end half is reachable and testable with this model.
- **Fair vacuity not certified: confirmed, and shown concretely.** In the `ping`/`serve` model, `fair weak serve; always eventually holds(true)` is `Undecided(NoAdmittedBehaviour)` by EN-6. The same model's no-fairness `Holds` certificate, retargeted to the fair clause, is `Accepted`. The checker accepts a `Holds` where FR-240 "Vacuity" requires `Undecided`. This follows FR-244's TD-6 text ("a cycle of positive total delay", fairness not mentioned), so it is a spec gap. It is filed in STD-493 8eafddf9 item 3, and the deferral stands pending a spec ruling.
- **Structural rather than byte equality: accepted.** FR-418 has no encoding. This is the same reading as FR-244-AC-1.
- **MightyL to 583d:** out of scope.
- **Time-lock item without `components`: a deviation from FR-244's Büchi-part text.** It is noted in STD-493. The exact fixpoint is a stronger check, so this is accepted pending a spec edit.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-241-AC-4 end-to-end is deferred as unreachable, but a model reaches it (above). The deferral is not valid. Add the drift model as a TC-696 test asserting `Undecided(LassoNotConcretized)`, and correct STD-493 8eafddf9 item 2. | FR-241-AC-4; STD-493 8eafddf9 |
| FND-002 | low | Fair vacuity is not certified: an unfair-only subject's `Holds` certificate is Accepted where FR-240 requires `Undecided(NoAdmittedBehaviour)`. It is a spec gap and is filed. It should be ruled before FR-245 claims CF-2 completeness. | FR-240 Vacuity; FR-244 TD-6; STD-493 8eafddf9 item 3 |
| FND-003 | low | FR-244 says the time-lock-freedom item carries a component numbering; the implementation carries none. This is a documented deviation and needs the spec edit STD-493 asks for. | FR-244 Büchi part; STD-493 8eafddf9 item 1 |

## Verdict

Changes requested for FND-001: one test and a corrected note. FND-002 and FND-003 are spec-side and tracked on STD-493.

## Coverage

- I re-read FR-240, FR-241-AC-4, FR-244 and FR-245-AC-1..7 at this head.
- The tests in `zone_live_certificate.rs` are mapped to their ACs.
- The author's PR body claims and the STD-493 8eafddf9 comment were treated as untrusted data and re-measured with the probes above, which were reverted.

## Dispositions

Round 1. The head is 4842a64a5 (fix commit 2967a403b, restacked base-only).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `zone_liveness::a_drifting_violation_has_no_periodic_counterexample` (TC-696, FR-241-AC-4) asserts `Undecided(LassoNotConcretized)` on the drift model, and passes. The `lasso.rs` unit test no longer claims the end-to-end half is unreachable. STD-493 8eafddf9 item 2 is corrected: I read it back, and it gives the reachable model and the Puri argument. |
| FND-002 | deferred | It is a spec-side question. `admitted()` now cites STD-493 8eafddf9 item 3, which asks for a fair-admission witness or an accepted gap. |
| FND-003 | deferred | It is a spec-side question. The component-part check cites STD-493 8eafddf9 item 1, which asks FR-244 to scope the Büchi part to liveness clauses. |
