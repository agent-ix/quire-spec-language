---
id: SR-202611121441562
title: "failure-domain review of PR2688 ADR-036 projection and realizability algorithms"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md PA-7 to PA-20; PR2688 234fdec9bc4e24b543b6e63ec461c6b53a209f42"
review_set: subset
evaluated_revision: "234fdec9bc4e24b543b6e63ec461c6b53a209f42"
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-036
    type: reviews
---

## Summary

Ticket: QSL-1889. Soundness of the projection merge rule, minimal counterexample, composition check and the realizability decision. The realizability refutation is unsound as written, and the composition check names a transition system that does not step local graphs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | PA-18/PA-19(ii) unsound refutation: a choice owner's branch selection is a role strategy decision (FR-175 "Strategy game"), but V1 fixes it ("the owner picks a branch") and refutes on one environment play that defeats that fixed strategy. FR-175 requires a play that defeats every role choice. Explore owner choices as the existential player (from the owner's observation class) or settle `inconclusive` when a defeat depends on an owner selection; the witness must also commit one branch per class | ADR-036:138 |
| FND-002 | high | PA-13 runs the composition check "on the one qsl-eval protocol transition system", but qsl-eval `ProtocolSystem` steps a checked global protocol clause; nothing steps a composition of role-local graphs, and PA-23 forbids a second system. The ADR must name the qsl-eval extension that steps local graphs (and who owns it) or drop the runtime check in V1 | ADR-036:123 |
| FND-003 | medium | PA-10 leaves two cases undefined: "next visible action" when the choice is under or before `parallel ... join all` (a set of minimal events, not one action); and a first divergent receive from one peer with distinct message types, the standard external-choice merge, which PA-10 accepts only via an "earlier" message. As written the classic `Seller -> Buyer: shipped or cancelled` pattern is refuted for any receiver not in `visible`; state it and, if that is FR-174's intended reading, record the cost | ADR-036:120 |
| FND-004 | medium | PA-7 gives one alternative per delivery count, but `at-least-once` ([1,inf)) and `unknown` ([0,inf)) have unbounded counts, so exploration never closes and every such result is `inconclusive`; `at-most-once` loss makes any receive-dependent protocol an environment defeat. State how counts are cut (capacity, max_depth) and that V1 decides usefully only under `exactly-once-premise`/`[1,1]` | ADR-036:112 |
| FND-005 | medium | PA-8 does not distinguish an absent group (c)/(d) premise from a declared-empty one; without that, realizability of a protocol with no shared facts or no external events is always `inconclusive` | ADR-036:113 |
| FND-006 | low | PA-11 calls the counterexample "minimal and unique" without fixing the measure (sum or max of path lengths, then lexicographic) or which choice is reported when several fail to merge | ADR-036:121 |

## Verdict

Not mergeable: FND-001 and FND-002 must be fixed. Clean: PA-9 (one send/receive pair per communication, induced order, FIFO-key receive order, no interleaving enumeration) matches FR-174 "Projection correctness"; PA-15 correctly keeps bound exhaustion `inconclusive` per FR-360; PA-17's game matches FR-175 "Strategy game".

## Scope and evidence

Read qsl-eval/src/simulation/protocol.rs module doc (ProtocolSystem steps a protocol subject's clause) and QSpec FR-053, FR-054, FR-174, FR-175 at origin/main b690867b.

- FR-175 Strategy game (quire-specification FR-175): The environment chooses admitted external events and channel outcomes; each role chooses enabled local actions from only its local history and declared shared facts. Unrealizability is witnessed by two histories that one role cannot distinguish but that require incompatible actions, or by an admitted environment play that defeats every role choice.
- FR-174-AC-5 (quire-specification FR-174): A local branch merge is accepted only under compatible next actions or explicit distinguishing evidence.
- FR-174-AC-1 (quire-specification FR-174): A projectable graph yields local graphs whose composed traces are included in the admitted global traces.
- FR-175-AC-3 (quire-specification FR-175): Missing environment/channel premises or analysis limits return inconclusive rather than realizable.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | PA-17/PA-18 compute winning-action sets under perfect information and then intersect per class. Two gaps: (a) refutation (i) is sound only if neither history can be avoided by an earlier owner choice, otherwise it must be inconclusive; (b) per-class commitments need not compose, so `proved` should settle only after the PA-19 table walk passes inside the engine, not only in test support | ADR-036:144 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | PA-17/PA-18: owner choices are existential moves in an AND/OR search; environment defeat refutes only when the root is lost (every owner choice defeated); cut-dependent or non-disjoint cases are inconclusive; residual gaps tracked as FND-007 |
| FND-002 | fixed | PA-12 drops the composition check; soundness is by construction, proved is Uncertified, AC-1 shown by the TC-205 corpus; rejected alternative recorded |
| FND-003 | fixed | PA-10 defines first(r, branch) as a set for parallel regions and accepts a divergent first receive from one peer with distinct message types |
| FND-004 | fixed | PA-7 states V1 is useful only under exactly-once-premise or [1,1], cuts open ranges at literal capacity and max_depth, and makes a cut-dependent result inconclusive |
| FND-005 | fixed | PA-8 separates absent (None, inconclusive) from declared-empty (Some of empty set, decides) |
| FND-006 | fixed | PA-11 fixes the measure (sum of path event counts) and the lexicographic tie-break over (choice, path_a, path_b), which also picks among failing choices |
