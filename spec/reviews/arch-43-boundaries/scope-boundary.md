---
id: SR-806
title: "Scope and boundary review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: scope-boundary
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/functional/FR-057-admit-shared-capability-kinds.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-806: Scope and boundary review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted ADR-017 draft on `spec/16-arch43-mapping` (QSL
`main` at `99e9b6c7`) and its pointer amendments. The review asked four
questions:

- Does the record stay inside Layer 4, coordination and conformance
  architecture, without implementing features?
- Is each responsibility with its owner: QSL, QSpec, IR or CG?
- Does it reach into QSpec-owned grammar or wire?
- Does it invent gates or ceremony?

The review read QSpec at `origin/main` `4634f5f` (FR-353, AD-003, FR-290
context, V1-TOOL-011/012, V1-BACK-021/022) and ADR-011 FB-05 and T-12.
Linear ticket text was not relied on.

What holds:

- The record implements nothing. It deletes nothing and maps each lane
  deletion to its owning PR (PF-8).
- Surface spelling and the v2 node are left to QSpec (Q-1, Q-2). QSL claims
  no grammar or wire shape of its own.
- Rust path resolution is correctly left out of QSL (AR-2).
- The refinement gates stay QSL test gates, as QSpec#116 records them
  (V1-TOOL-011/012 → QSL #191/#192). The recorded reference outcome is
  test expected data, not a pin or manifest.
- TK-3 removes dead code instead of adding bookkeeping.
- No new gate or ledger is invented.

What does not hold:

- Two findings are high. The AR-4 export is placed where CG may not call
  it, and it adds the second carrier AR-4 says does not exist. QSL's own
  FR-057 still requires the refinement gates to request
  `operation-contract`, and §7 does not amend it.
- The rest are CG duties assigned without an owner, a QSpec-owned reading
  of V1-TOOL-012 decided in QSL, and a miscounted ticket scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The AR-4 implementation export crosses a boundary no one owns. It is "a layer-4 `package` function", but FB-05 and T-12 let CG depend only on the layer-6 `replay` facade. So CG cannot call it, and AR-4 names no other caller. AR-6 then lists "the export (AR-4) keyed by occurrence key" as a versioned CG input beside the v2 relation node. That is a second carrier of the resolved bindings, which contradicts AR-4's "No second carrier exists" and the Alternatives' rejection of a side file. Fix: choose one path and allocate it. (a) The export is a QSL-driver gate only, and its caller (for example `command` or a named layer-6 entry) is named. CG receives only the request's item set and reads bindings from the v2 node, and AR-6 drops the export column. Or (b) the export is a wire artifact, with a QSpec Q-row for its contract and an FB-05 amendment. | ADR-017 AR-4, AR-6, Alternatives ("side file"); ADR-011 FB-05, T-12, `qsl-replay` row (line 891) |
| FND-002 | high | RF-1 rules that the #191 and #192 gates emit no claim, and asks QSpec to change FR-290 (Q-3). QSL's own FR-057 is left saying the opposite. Its claim-form row reads "Refinement between two operation contracts or state models (the relation-family refinement gates, #191 and #192) → `operation-contract`, one claim per clause implication" (FR-057 line 178). FR-057-AC-10 requires "each claim form in this requirement's claim-form table [to request] exactly its listed kind" (line 362). Its Dependencies say AC-10 "also needs … the refinement gates (#191, #192)" (lines 383-385). §7 lists no FR-057 amendment, so on merge FR-057-AC-10 cannot be met. Fix: amend FR-057 in this change. Drop the gate references from the row, state that the row names the claim a future proof route requests (no V1 owner), and remove #191/#192 from the AC-10 dependency. Add the amendment to §7. | ADR-017 RF-1, §7, Q-3; FR-057 claim-form table, FR-057-AC-10, Dependencies |
| FND-003 | medium | §5 says CG#84, IR#136, IR-339 and CG#49 "receive §AR-6 and §PF-2; this record asks nothing else of them". The record still assigns CG three duties, and none has an owning ticket. (1) CG resolves each `RustPath` and refuses one that names no item (AR-2), with no catalog code named. (2) CG keeps its own unbound guard (AR-4). (3) CG Kani (FR-196) consumes the relation (AR-6 row 2), which QSpec FR-353-AC-2 requires. CG#49 is frame harnesses and CG#84 is Verus, so the Kani consumer has no owner. Fix: add these to §6 as CG asks with their tickets (or new ones), naming the catalog code for an unresolved `RustPath`. Otherwise reword §5 to list what each downstream ticket must do. | ADR-017 AR-2, AR-4, AR-6, §5, §6; QSpec FR-353-AC-2, FR-196 |
| FND-004 | medium | RF-2 and RF-3 decide what V1-TOOL-012's "profile layering" means: AD-003 "requiring" edges, with the parent being the required definition. That meaning is QSpec's, and AD-003 edges run between definitions that QSL mostly cannot select (FR-110: `root` only; SR-802 FND-001). This QSL-side reading decides #192's whole scope. No Q-row asks QSpec to confirm it, or to say whether edges QSL cannot compile under are in V1 scope. Fix: add a Q-4 asking QSpec to state which layering V1-TOOL-012 covers and whether non-`root` edges are required. Until QSpec answers, scope #192 to the edge that the `root` compile actually exercises. | ADR-017 RF-2, RF-3, §5 (#192), §6; QSpec V1-TOOL-012, AD-003 "Accepted profile hierarchy"; FR-110 |
| FND-005 | low | TK-1 scopes "`CheckedGraph::operation_frame` and its three callers", but G-1 names two call sites, and the code has one production caller. `resolve_frame` (`qsl-replay/src/spine/clause.rs:883-894`) is used by `spine/clause.rs:867` and `execute/frame.rs:187`, and one test calls it at `spine/clause/tests/frame_replay.rs:85`. A miscounted ticket scope invites a partial fix, such as `execute/frame.rs::operation_name` (`:348-357`) keeping its string split. Fix: list the sites exactly. They are `resolve_frame`, its two callers, `execute/frame.rs::operation_name`, and the test. | ADR-017 G-1, TK-1; `qsl-replay/src/spine/clause.rs:867,883-894`; `qsl-replay/src/execute/frame.rs:187,348-357` |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001: the export's
caller is the orchestrating driver, which puts only bound items in the QSpec
FR-331 request; CG reads bindings from the v2 node, so there is no second
carrier (AR-4, AR-6, §5 driver ask). FND-002: FR-057's row, FR-057-AC-10 and
its Dependencies are amended, with TC-153 and the model-linking matrix row,
and listed in §7. FND-003: §5 has a downstream-asks table for CG#84, CG#49,
IR-339 and the driver. FND-004: Q-5 asks QSpec which layering V1-TOOL-012
covers; #192 waits on it. FND-005: TK-1 lists every site exactly.
