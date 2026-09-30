---
id: SR-803
title: "Dependency review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries"
type: SpecReview
analysis: dependency
scope: "spec/decisions/ADR-017-protocol-refinement-abstraction-boundaries.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/decisions/ADR-012-semantic-family-extension-contracts.md, spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md, spec/spec.md"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: reviews
---
# SR-803: Dependency review of ADR-017 protocol/frame, refinement and abstraction-relation boundaries

## Summary

Reviewed the uncommitted ADR-017 draft on `spec/16-arch43-mapping`, its
amendment pointers in ADR-011, ADR-012, ADR-013
and `spec/spec.md`, against QSpec `origin/main`. Linear
relations for QSL-16, QSL-15, QSL-36, QSL-39, QSL-40, IR-32, IR-33, IR-93,
IR-339, IR-370 and IR-89 were read as data. The review checked each edge the
ADR states or implies. It asked whether the edge is real, whether the graph
is acyclic, and whether enablement is kept apart from feature work.

What holds:

- The graph is acyclic. The ADR asks nothing of IR-339, CG#49, CG#84 or
  IR#136, and no downstream ticket blocks a QSL decision.
- The downstream corrections are correct. QSL-16's "#84" is
  agent-ix/quire-contract-codegen#84 (IR-32) and its "#136" is
  agent-ix/quire-contract-ir#136 (IR-33). IR-370 is Done. IR-339 and IR-93
  are Backlog. QSL-36 blocks IR-32 in Linear, as §8 implies.
- QSL-16 blocks QSL-15 in Linear, which matches the Status.
- The PF-2 claim holds: frame records settle `unsupported` until IR-339 and
  CG#49 land, and no QSL change is needed when they land. That edge is
  correctly marked as not blocking QSL.
- TK-3 and TK-4 are independent of each other and of the other tickets.

What does not hold:

- Three findings are high. #198's "S3 check and in-process export do not
  wait" on Q-1 and Q-2 is false for the spine path, in two separate ways.
  PF-4 changes a wire member that QSpec and CG own, but the ADR files no
  QSpec or CG item for it.
- The Linear edges and the ticket ordering inside the ADR disagree with the
  ADR text. Q-1 to Q-3 and TK-1 to TK-4 have no ticket ids, so none of their
  edges can be recorded.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | §8.1, §5 and the Consequences say #198's "S3 check and in-process export, keyed as AR-2, do not wait" on Q-1. The S3 checker's input is an S2 form (ADR-011 §1), and AR-1 says #198's S2 form waits on Q-1, because there is no surface spelling and the CST `RelationClause` production is refused (`qsl-forms/src/value.rs:1044`). Without Q-1, spine `compile` cannot produce a `CheckedAbstractionRelation`. S3 can then be tested only on hand-built forms, and #198's exit ("one ConfigVersion object binds to a Rust struct") and QSL-15 scenario 8 cannot run. Fix: split #198. The enablement slice is `CheckedAbstractionRelation`, `FrameBindingKey`, `RustPath`, the AR-3 refusals and the export function, tested on constructed forms. The feature slice is the spine exit and ARCH-G4 scenario 8. Record Q-1 → feature slice as a blocking edge and say that scenario 8 waits on QSpec. | ADR-017 §3 AR-1, §5, §8.1, Consequences; ADR-011 §1; `qsl-forms/src/value.rs:1044` |
| FND-002 | high | Q-2 blocks compilation, not only the wire. E4 omits every checked node that has no v2 form, and spine `compile` then refuses the whole unit with `CompileRefusal::Omitted` ("partial output, which E4 never writes", `qsl-replay/src/spine.rs:139-143`; `qsl-package/src/emit.rs:125-200`). `package_id` is the digest of the emitted v2 `identity_preimage` (ADR-013 T-2). So one of two things is true. If the relation is a checked node, every unit that contains one refuses at E4 until Q-2, and the export "after S4" never runs. If it is not a node, it does not enter `package_id`, and the AR-3 revision rule (QSpec FR-353-AC-5) is false before Q-2. §5's "met in process before Q-2" holds under neither reading. Fix: record Q-2 as a prerequisite of #198's feature slice and of FR-353-AC-5. Alternatively, state where the relation is held before Q-2, and drop the `package_id` revision claim until Q-2 lands. | ADR-017 §3 AR-3, AR-4, §5, §8.1; ADR-013 T-2, O-02; `qsl-replay/src/spine.rs:139-143`; `qsl-package/src/emit.rs:125-200` |
| FND-003 | high | PF-4 and TK-2 change a member that QSpec and CG own, but list only QSL work. QSpec FR-323-AC-6 and TC-278 RE-01/RE-02 fix "the selected function's `QualifiedName`" as a replay member. ADR-013 O-26 names FR-323's `selection` as the request's serialized authority. QC-8 lists the same member. CG's FR-024-AC-4 lists exactly the 14 `WitnessPacket` members that `WitnessEnvelope::reconstruct` refuses when absent. C-12 (agent-ix/quire-contract-codegen#50) copies each member. The ADR-013 diff amends only the O-25 list line, not the O-26 Public type or Serialized authority rows. Fix: add a QSpec item (Q-4) for the FR-323 selection member and TC-278, and add a CG edge (FR-024, #50). Make TK-2 depend on both, and amend O-26. Otherwise, confine `ReplaySelection` to the in-process envelope and leave the wire unchanged. | ADR-017 §1 PF-4, §6 TK-2, §7; ADR-013 O-25, O-26, QC-8, C-12; QSpec FR-323-AC-6, TC-278; agent-ix/quire-contract-codegen FR-024-AC-4 |
| FND-004 | medium | The Linear edges contradict the ADR. First, QSL-39 (#192) blocks QSL-36 (#198), but nothing in §2 or §3 makes #198 need anything from #192. Second, §5 says #192 "builds … on #191's classification and reader", yet QSL-40 and QSL-39 are only `related`, with no blocking edge. Third, QSL-20 (ARCH-G3, Backlog) blocks QSL-16, while Context treats ADR-013 O-24 to O-27 as accepted and PF-4 amends O-25 and O-26, which is what G3 gates. Fix: list each edge in §8. Drop QSL-39 → QSL-36 and add QSL-40 → QSL-39. Either record QSL-20 as a prerequisite of accepting this ADR, or ask the owner to remove that edge. Sequence TK-2 against QSL-20 so that G3 does not validate an envelope that TK-2 then changes. | Linear QSL-16, QSL-20, QSL-36, QSL-39, QSL-40 relations; ADR-017 Context, §1 PF-4, §5, §8 |
| FND-005 | medium | §6 lists Q-1 to Q-3 and TK-1 to TK-4 by local id only, and none is filed; a Linear title search found none. So the edges Q-1 → #198, Q-2 → #198, Q-2 → IR-32 and TK-2 → QSpec cannot be recorded, and "check Linear for what is blocked" returns nothing. §8 also omits two existing edges. IR-33 blocks IR-32, so CG#84 waits on IR#136 as well as on Q-2. IR-339 has no edge to IR-93, although PF-2 treats "IR-339 and CG#49" as one joint prerequisite. Fix: file the seven tickets, cite their ids in §6, and link the edges. Add IR-33 → IR-32 to §8, and either add IR-339 → IR-93 or state that the two land independently. | ADR-017 §6, §8; Linear IR-32, IR-33, IR-339, IR-93 |
| FND-006 | medium | TK-1 and TK-2 overlap and have no stated order. PF-3 and PF-4 name a typed `OperationName` ("`QualifiedName` of the object type plus the operation `Identifier`"), but the existing `qsl_replay::spine::clause::OperationName` is three `String`s (`spine/clause.rs:93-100`). The typed shape is `FrameOperation`. `ReplaySelection::Frame(OperationName)` (TK-2) needs the type that TK-1 introduces. Both tickets rewrite `resolve_frame` and `execute/frame.rs`. TK-1 says "its three callers", but there is one production caller, `resolve_frame` (`spine/clause.rs:883-894`), and one test caller (`spine/clause/tests/frame_replay.rs:85`). `execute/frame.rs:348-357` builds the name and does not call the lookup. Fix: add TK-1 → TK-2. Name the typed `OperationName`'s owner and definition in TK-1, and correct the caller count. | ADR-017 §1 PF-3 G-1, PF-4, §6 TK-1, TK-2; `qsl-replay/src/spine/clause.rs:93-100`, `:883-894`; `qsl-replay/src/execute/frame.rs:348-357` |
| FND-007 | low | The front matter does not match what the text depends on. Several dependencies are missing: QSpec FR-323 (PF-4, AR-6), FR-331 (AR-5), AD-006 and FR-012/FR-013 (AR-2 keys), and FR-301 (RF-5 exit code), plus QSL FR-070, FR-071, FR-100, FR-106, FR-109, FR-112, FR-113 and FR-089. FR-110 is listed as `relates_to`, but RF-2's decision rests on it. Fix: add `depends_on` edges for these, and change FR-110 to `depends_on`. | ADR-017 front matter, §1 PF-3, PF-4, §2 RF-2, RF-5, §3 AR-2, AR-5, AR-6 |

## Resolution

Resolved by the author on `spec/16-arch43-mapping`. FND-001 and FND-002:
#198 is split; the enablement slice (crate-private, outside the checked
graph) waits on nothing, and the feature slice, AC-5 and ARCH-G4 scenario 8
wait on Q-1 and Q-2 (§5, §8). FND-003: no wire member changes; the selection
member is Q-4 (QSpec FR-323 with CG FR-024 and codegen#50), and TK-2 is a
consistency check inside `replay_frame`. FND-004: §8 item 4 lists the Linear
edges for the owner. FND-005: IR-33 → IR-32 added; IR-339 and CG#49 land
independently (PF-2). The Q and TK items are proposed for the team lead to
file; this record does not create tracker entries. FND-006: TK-2 no longer
needs TK-1's type; callers corrected. FND-007: front matter extended, FR-110
is `depends_on`, FR-089, FR-103 and FR-001 added.
