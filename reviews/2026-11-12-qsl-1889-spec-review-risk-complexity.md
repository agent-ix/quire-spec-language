---
id: SR-202611121441564
title: "risk and complexity review of PR2688 ADR-036 scope and slicing"
type: SpecReview
analysis: risk-complexity
scope: "agent-ix/quire-spec-language; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md PA-2, PA-22, section 12; PR2688 234fdec9bc4e24b543b6e63ec461c6b53a209f42"
review_set: subset
evaluated_revision: "234fdec9bc4e24b543b6e63ec461c6b53a209f42"
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: reviews
---

## Summary

Ticket: QSL-1889. Judged against the owner's ask for the minimal engine that unblocks TL-387. The core (projection, fixed-strategy realizability, settlement, serving) is the right size; the independent checkers, slice 1's breadth and the stubbed slice 3 are not needed for V1.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | PA-2's independent checkers and slices 6 and 8 are not required: FR-174/175 ask for no checker, and ADR-011 S6c lets a native engine with no core checker settle `proved` `uncertified`. Defer them to a follow-up and fold the determinism test into slices 7 and 9 | ADR-036:102 |
| FND-002 | medium | Slice 1 asks for "requirements for PA-1 to PA-25, each with an AC and a TC row", which turns placement and process decisions (PA-1, PA-2, PA-4, PA-23, PA-24) into invented requirements; write QSL FRs only for the behaviour quire-protocol consumes (projection, realizability, key, settlement map, unsupported causes) | ADR-036:201 |
| FND-003 | medium | Slice 3 returns `proved` "with the composition check stubbed to the closed case", a stubbed success path contrary to PA-12; keep slices 3 to 5 unserved until slice 9 or merge 3, 4 and 5 into one PR | ADR-036:203 |
| FND-004 | low | Slice 8's "identical result across thread counts" assumes a parallel engine nothing else specifies; slice 7 depends on local graphs (slice 4), not on the composition check (slice 5) | ADR-036:208 |

## Verdict

Mergeable on these points once the slicing is trimmed. Minimum useful subset: 1 (trimmed), 2, 3+4+5 merged, 7, 9; 6 and 8 deferred. Order: 1, 2, 3-5, then 7 (needs local graphs), then 9. V1's unsupported list is justified except the stated reason for `repeat` (see the integrity review FND-004); excluding loops is still acceptable for a minimal V1 since projecting recursion is extra scope, but the cost (no retry protocols in quire-protocol corpora) should be said.

## Scope and evidence

- Section 12 slicing table (ADR-036 lines 199-212).
- ADR-011 S6c (spec/decisions/ADR-011): a proof settles only after its certificate checker in layer 6 accepts it, or settles proved labelled uncertified when the engine has no core checker.
- TL-387 (Linear): AC1 dispatch and witness carrying tested against the real QSL backend; AC2 typed unsupported path stays for refused requests.
