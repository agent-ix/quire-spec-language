---
id: SR-465
title: "EARS conformance review of ADR-010 observed architecture baseline"
type: SpecReview
analysis: ears-conformance
scope: "spec/decisions/ADR-010-observed-architecture-baseline.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-010
    type: reviews
---
# SR-465: EARS conformance review of ADR-010

## Summary

Reviewed commit `faa1731` (branch `task/206-observed-architecture`):
`spec/decisions/ADR-010-observed-architecture-baseline.md` and its index row in
`spec/spec.md` (line 385). ADR-010 is a descriptive ADR, not an FR, NFR or StR, so
the EARS scope is empty. The document has no `shall` statement. Its only `must`
(§9.1 L1-D1) sits inside a question, not a requirement. The four Decision items
are indicative governance statements, which is normal for an ADR, and are not EARS
requirements. `quire validate --strict` on the ADR reports 1/1 docs grammar-clean,
0 findings. The current-state wording is sound. No intended design is described as
present: AD-016 and the #205 flow appear only as comparison references, and absent
elements are marked ABSENT with evidence. The body narrates no history (no
"was", "previously", "no longer" or "now"). The one real defect is that Decision 4 and §8 name
different authorities for the work-in-progress merge order, and the §8 table
contradicts its own governing prose. A smaller issue is that one Consequence
describes gate #212 doing something #212 does not state.

Verdict: ACCEPT WITH FINDINGS

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Decision 4 says dispositions "follow the ARCH-01 classification posted on #207", and §8 calls that comment "the authority for these dispositions". §8 prose then overrides its merge order ("#204 → #228 → #200 … the coordinator's order governs"). The §8 table Notes still carry the #207 order: #228 "merges first", #204 "merges second". So the record states two opposite merge orders. Fix: limit Decision 4 and the §8 authority sentence to the disposition (keep/revise/defer/…). Add a separate sentence that makes the #205 coordinator (2026-09-19) the authority for merge order. Change the Notes cells to #204 "merges first" and #228 "merges second". | ADR-010 Decision 4, §8 |
| FND-002 | low | Consequences says "#212's change-scenario gate checks that every §9 item has a decision". That describes an open gate ticket as if it already works, and #212 does not say it. #212 applies seven change scenarios, and its Failure rule reopens the owning Layer 1 ticket when a scenario exposes a missing decision. Fix: cite #212 as written, for example "#212 applies seven change scenarios; a scenario that exposes a missing decision reopens its owning Layer 1 ticket (#212 Failure rule)". | ADR-010 Consequences |
| FND-003 | low | The only modal verb in the record is the `must` in L1-D1 ("Which of #186 … must wait on capability registry #185"). It is a question, but a normative-keyword scan reads it as an obligation. Fix: rephrase as "Whether each of #186, #187, #188, #189, #191, #192 and #198 waits on capability registry #185". | ADR-010 §9.1 L1-D1 |
