---
id: FR-116
title: "Replay a frame counterexample through the replay facade"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: depends_on
---
# FR-116: Replay a frame counterexample through the replay facade

## Description

A frame counterexample is an invocation that writes, creates or deletes
outside its operation's frame. QSL SHALL replay one through the layer-6
`replay` facade (FR-098) and the `ProtocolClause` S6a `evaluate` arm
(FR-115), keeping the clause, frame, anchor and occurrence identity and the
witness's values through replay, and SHALL settle an FR-072 replay result
(ADR-012 §12.2 Witness and replay row, ADR-013 O-25 to O-27).

The frame counterexample is the `ProtocolClause` family's own payload on the
FR-070 envelope: `FrameCounterexample`, which implements `FamilyPayload`
(FR-070-AC-5). Today `qsl-replay` has only `NoPayload` and a test-only
payload.

## Inputs

- A `WitnessEnvelope<FrameCounterexample>` (FR-070) whose payload holds:
  - `operation`: the declaring object type's `QualifiedName` and the
    operation name;
  - `anchor` and `frame`: the checked node identities of the operation's
    `state`/`operation_anchor` and `state`/`frame` nodes (FR-105), and the
    frame's `generated` occurrence key (FR-104);
  - `invocation`: the `DocumentRef` (FR-106) of the invocation, with its pre
    and post snapshots in the request's byte provision by `sha256-jcs`
    digest;
  - `change`: the change the counterexample claims: an object reference and
    a field name for a write, or an object reference and its type for a
    creation or deletion.
- FR-098's package reference, byte provision and limits.

## Outputs

- An FR-072 replay result on the envelope's arm, or a typed `ReplayRefusal`
  with no partial result. A result retains the source identity and digest,
  the `package_id`, the payload's operation, anchor, frame and occurrence
  identities, the identity and digest of the invocation and both snapshots,
  the witness's `change`, and the change the replay found.

## Behavior

- The executor SHALL recompile and check the package by FR-098's rules, in
  FR-098's order, before it reads the payload.
- If the recompiled `package_id` differs from the request's, then the
  executor SHALL refuse by FR-098's stale `package_id` rule.
- The executor SHALL resolve `operation` in the recompiled package. If it
  names no operation with a frame node, then the executor SHALL refuse
  `missing_declaration`/`missing-name`. If the recompiled anchor or frame
  node identity differs from the payload's, or the frame's occurrence key
  differs, then the executor SHALL refuse `stale_dependency`/
  `revision-mismatch` before any admission, naming the payload's and the
  recompiled identity. No identity is recovered from a display name.
- The executor SHALL admit the invocation by FR-115's admission, reading the
  three documents only from the byte provision.
- If admission fails, then the executor SHALL refuse with a `ReplayRefusal`
  holding FR-106's record.
- The executor SHALL run FR-115's frame check once.
  - If it reports `frame_violation`/`unauthorized-change` at the object and
    member (field, or created or deleted type) the payload's `change` names,
    then the executor SHALL settle `reproduced-with-evaluated-witness`.
  - If the frame check passes, or reports a violation at another object or
    member, then the executor SHALL settle `inconclusive` with cause
    `Verdicts`, holding both, never repaired (FR-072).
  - If it reports `population_delta_mismatch`, then the executor SHALL settle
    `inconclusive` with cause `NoValue`: the invocation completes no frame
    verdict.
- Replay SHALL read no path, environment variable, clock or search location,
  and SHALL give the same result for the same request.
- CG builds the frame witness bindings over IR's `WitnessBinding` (AD-016)
  once agent-ix/quire-contract-ir#109 and agent-ix/quire-contract-codegen#49
  land. Until then this requirement's input is the in-process envelope, and
  the decode from IR's witness is the part that waits.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-116-AC-1 | A hand-built envelope for `Config::ConfigVersion::attemptUpdate` carrying the forbidden-parent-change invocation and `change` (`child`, `parent`) settles `reproduced-with-evaluated-witness`. The result holds the source digest, `package_id`, the anchor, frame and occurrence identities equal to the payload's, the three document identities and digests, and both changes. | Test (TC-515) |
| FR-116-AC-2 | The same envelope carrying the changed-version invocation (inside the frame) settles `inconclusive`, `Verdicts`; carrying forbidden-parent-change with `change` (`child`, `versionNumber`) settles `inconclusive`, `Verdicts`, holding both changes; carrying an invocation whose `created` disagrees settles `inconclusive`, `NoValue`. | Test (TC-515) |
| FR-116-AC-3 | Stale identity: an envelope whose frame node identity is taken from a package whose `attemptUpdate` frame also modifies `parent` refuses `stale_dependency`/`revision-mismatch` naming both frame identities, with no admission; a source edit that changes the `package_id` refuses by FR-098's stale `package_id` rule; an `operation` naming `missing` refuses `missing_declaration`/`missing-name`. | Test (TC-515) |
| FR-116-AC-4 | An envelope whose pre snapshot is absent from the byte provision refuses with FR-106's `unavailable_observation` record, and one whose invocation bytes differ from their digest refuses with `stale_dependency`/`byte-digest-mismatch`; neither settles a result. | Test (TC-515) |
| FR-116-AC-5 | Replaying one envelope twice gives equal results. `FrameCounterexample` implements `FamilyPayload`, and the envelope carries it as its generic parameter with no string-keyed field. | Test (TC-515) |

## Dependencies

- FR-070 (envelope and `FamilyPayload`), FR-072 (replay result), FR-098
  (replay facade), FR-104 and FR-105 (frame identity and occurrence),
  FR-106 and FR-115 (admission and the frame check).
- ADR-013 O-25 to O-27; AD-016 (`WitnessBinding`).
- agent-ix/quire-contract-ir#109 and agent-ix/quire-contract-codegen#49 for
  the end-to-end path from a Kani frame counterexample.

## Status

Specified under QSL-296 (QSL-21a). Not yet implemented; QSL-301 (QSL-21f)
implements it. The IR witness decode waits on
agent-ix/quire-contract-ir#109 and agent-ix/quire-contract-codegen#49.
