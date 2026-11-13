---
id: SR-2026111306075603
title: "spec integrity review of PR2697 FR-570, FR-574 and FR-575 edits"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-570-*.md, spec/functional/FR-574-*.md, spec/functional/FR-575-*.md against spec/decisions/ADR-036-*.md; PR2697 c7587607454cf151473007cd14cd87fdd7b08286"
review_set: subset
---

## Summary

Ticket: QSL-1895 (ADR-036 slice 3). Integrity review of the spec lines the PR adds to FR-570, FR-574 and FR-575, checked against ADR-036 PA-7, PA-9 to PA-12, PA-14, PA-20, PA-21, PA-24 and QSpec FR-466 "Decision facts" and FR-053-AC-1. The coder's design decisions were checked one by one. Decisions (b) path A ordering, (c) trace position, (d) the `unsupported-control` slug, (e) the run-limit defaults, (f) choice and `parallel` events as global events and (g) the reshaped cause types refine ADR-036 and do not contradict it: ADR-036 open item 2 leaves the slugs to the QSL requirements, and PA-14 asks for published defaults. Decision (a) is grounded in QSpec: FR-466 says `visible` holds the owner's own decision-fact expressions, and a `send` binder is never eligible for the owner. So the ADR's "visible = {Buyer}" and "Seller -> Warehouse: decision whose payload the guards test" cannot be written. But the FR rewrite weakens PA-7's "the guards are a function of that message's payload" to "the decision facts read that payload binder", which is unsound when the decision also reads other facts. ADR-036 itself was not amended, so its worked example and FR-570 now disagree.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The FR-570 "observing the outcome" rule makes a role informed when it sent or received any earlier message whose payload binder the choice's decision facts read. ADR-036 PA-7 and QSpec FR-053-AC-1 need the guards to be a function of that message's payload. A choice whose `visible` lists `got.ok` (sent by Warehouse) and `got2.ok` (sent by Buyer), with guards testing only `got2.ok`, makes Warehouse "informed" by the FR text although Warehouse cannot know the outcome. The rule must require every decision fact (or every fact the guards depend on) to be known to the role, not just one. | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:70-77 |
| FND-002 | medium | Decision (a) rewrites the ADR-036 worked example in FR-570-AC-6 ("an earlier message between Warehouse and Seller whose payload binder the visible fact reads"), but ADR-036 PA-7, PA-10 and "Worked example" still say `visible = { Buyer }` and an earlier `Seller -> Warehouse: decision` whose payload the guards test. QSpec FR-466 makes both impossible (visible holds expression nodes; a send binder is never eligible for the owner). The FR is right and the ADR is now wrong. Amend the ADR text in this PR. The AC should also give the direction (Warehouse sends, Seller receives), because only that direction can carry the outcome. | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:115; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:120,128,160-174 |
| FND-003 | medium | The new horizon bullet says a branch pair holding any event beyond `max_depth` is not compared. A counterexample found in another choice is still reported as the PA-11 minimal counterexample, but the skipped pair can hold a shorter divergence within the horizon. FR-570-AC-5 and PA-11 ("minimal and unique") then do not hold under a cut. Either compare a cut pair up to the horizon, or state that minimality is over the compared pairs only, and settle a cut search with a counterexample accordingly. | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:97-100 |
| FND-004 | low | FR-570-AC-4 still lists "`visible`" as its own merge basis beside "an earlier outcome-carrying message". After decision (a) the Behavior says a role is informed only by such a message, so `visible` is no separate basis. Drop it from AC-4 or say what it means. | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:113 |

## Scope and evidence

The diff `origin/main...HEAD` at c75876074, spec files only. Each edited line was compared with ADR-036 and with QSpec FR-466 (quire-specification b690867b) "Decision facts" and the `choice`/`await` layout rows. Decision (b) is consistent with PA-11: the old FR rule would have made the empty path path A and left `trace_position` with nothing to index. FR-574's trace-position edit (decision c) matches PA-24 and FR-574-AC-4. FR-575's new row and the "first construct" order are within ADR-036 open item 2, and an `await` always carries `clock`, `within` and `timeout` in FR-466, so mapping every `await` to `timed-await` is correct. The run-limit defaults (decision e) are published as PA-14 requires and stay out of the key.

- FR-570 Behavior "At a choice" (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): observing the outcome when it sent or received an earlier message whose payload binder the choice's decision facts read. (examined)
- FR-570 Behavior "A failure to merge" (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): path A being the path to an action when the other branch has ended, else the canonically first one. (examined)
- FR-570 Behavior global events, run limits, horizon bullets (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md). (examined)
- FR-570-AC-4 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): A choice branch merges for the role only by the PA-10 rule. (examined)
- FR-570-AC-6 (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): the ADR-036 worked example, restated. (examined)
- FR-574 `trace_position` row (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md). (examined)
- FR-575 `unsupported-control` row and first-construct order (spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md). (examined)
- ADR-036 PA-7, PA-10, PA-11, PA-14, PA-21, PA-24, Worked example (spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md). (context_only)

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | The new nested-choice rule ("The actions of a branch include those inside a choice nested in the branch: every alternative of the nested choice is a possible first action of the branch"; PA-10 "The actions of a branch include those of a choice nested in it") does not say how alternatives pair off. Read as written, and as implemented, all alternatives must be matched like concurrent events, which refutes a projectable protocol (SR-2026111306075601 FND-010). State that each alternative of a nested choice is compared against the other branch on its own, so that the merge holds when every alternative pairs off. | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:108-110; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:128 |
| FND-006 | low | FR-575 and ADR-036 PA-21 say a choice assigned to no role is "not a construct a checked package can hold". QSpec FR-466 lets `choice.owner` be a `protocol` node. It is the QSL checker (`ProtocolAnchorCause::NodeRole`) that refuses one. A package from another producer can hold one, and the engine would then treat every role as `MustMerge`. Say "the QSL checker refuses", or refuse a non-role owner at admission. | spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md:66-68; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:154 |
| FND-007 | low | The ADR edits left broken grammar: PA-10 ends a clause with "... keyed by the message type), (PA-7)." and PA-21 runs a full sentence ("a protocol where ... cannot be written, because ...") into the list of causes. FR-570-AC-6 now bundles the worked example with the nested-choice and partial-read rules. Fix the two ADR sentences and move the nested-choice clause to its own AC. | spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:128,154; spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:128 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 66bed12d8: FR-570 now reads "only when every earlier message whose payload binder the choice's decision facts read is a message the role sent or received ... A decision that also reads a message the role took no part in does not inform the role", and ADR-036 PA-7 and PA-10 match. |
| FND-002 | fixed | 66bed12d8: ADR-036 PA-7, PA-10 and the worked example are amended: the decision reads the payload of `Buyer -> Seller: order`, the repair is `Warehouse -> Seller: report`, and a decision that also reads another message does not inform. FR-570-AC-6 states the direction ("a message that Warehouse sends to Seller and Seller receives"). |
| FND-003 | fixed | 66bed12d8: the horizon bullet now compares layer by layer, stops at the first layer that holds an event beyond the horizon, and reports a divergence at an earlier layer as a counterexample. |
| FND-004 | fixed | 66bed12d8: FR-570-AC-4 reads "owner, an earlier message the role took part in that carries the whole decision, or distinct first-receive message types ..."; `visible` is no longer listed as its own basis. |
