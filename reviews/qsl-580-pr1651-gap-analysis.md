---
id: SR-1651002
title: "Gap analysis of quire-spec-language PR #1651: a clock of a referenced object (580i)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@33831d742bad3cfdaeb45005f880ecd68d149470; PR #1651 against FR-230-AC-10 (peer_clock part) and TC-685; FR-230-AC-24 and FR-231-AC-4 deferrals; QSL FR-120 closure rule (context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Gap analysis of quire-spec-language PR #1651

## Summary

Ticket: QSL-580, slice 580i. Plan completion: not assessed.

FR-230-AC-10 is now covered except for its expected checked form:
- `pick` and `g` were already covered in 580e.
- `peer_clock(self) <= 2 ms` is covered at S3 (`via = [peer]`) and at qsl-eval over `{c1, c2}` with crossed peers, including a delay, a reset and the guard of each receiver.
- `own_clock(self.peer)` is covered at S3.

The deferrals are tracked:
- **FR-231-AC-4's clock atom:** STD-493 item 20. `holds(c.x <= 3 ms)` still refuses.
- **AC-24 (`Reference<Clock>`):** IR-1036. A non-field Clock position is `CLOCK_OUTSIDE_FIELD` in the FCD IR.
- **Record collections:** 580j.

## Verdict

**APPROVED with low findings.**

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The checked form is a spec conflict. AC-10 says `peer_clock(self) <= 2 ms` "checks to the parts `c1.x <= 2` under `self.peer = c1` and `c2.x <= 2` under `self.peer = c2`", that is, one part per object, each with a data condition. S3 has no universe, so QSL instead checks to one part with `via = [peer]` and makes it concrete per state in `guarded_steps`. The semantics are the same, and the S3 test asserts the `via` form rather than AC-10's two parts. FR-230 should say that the universe-level parts are an evaluation-time reading, or define the checked form (`via`). File this on STD-493 next to item 9 (timed node bodies), which also has to carry `via` before S4 can emit such a guard. | FR-230-AC-10; qsl-semantics/tests/it/clock_places.rs (a_clock_through_a_reference_field_is_the_referenced_objects) |
| FND-002 | low | Edge coverage. No test checks that `o.peer.x` through an optional or multi-valued reference is refused as not a place (the walk stops there). No test runs `own_clock(self.peer)` at qsl-eval. No test uses `peer_clock` in a negated guard position (`not (peer_clock(self) <= 2 ms)`), where `via` has to carry into the negated DNF. | qsl-semantics/src/check/lowering/model.rs:1158-1200; qsl-semantics/src/check/timed_model/reading.rs:1250-1272 |
