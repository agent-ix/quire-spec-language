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
  - target: ix://agent-ix/quire-specification/FR-323
    type: depends_on
---
# FR-116: Replay a frame counterexample through the replay facade

## Description

A frame counterexample is an invocation that writes, creates, deletes or
retypes an object, or adds or removes a relationship link, outside its
operation's frame. QSL SHALL replay one through the layer-6
`replay` facade (FR-098) and the `ProtocolClause` S6a `evaluate` arm
(FR-115), keeping the clause, frame, anchor and occurrence identity and the
witness's values through replay, and SHALL settle an FR-072 replay result
(ADR-012 §12.2 Witness and replay row, ADR-013 O-25 to O-27).

The frame counterexample is the `ProtocolClause` family's own payload on the
FR-070 envelope: `FrameCounterexample`, which implements `FamilyPayload`
(FR-070-AC-5). Today `qsl-replay` has only `NoPayload` and a test-only
payload.

## Inputs

- FR-098's request, whose replay target is `frame_operation` (QSpec
  FR-323): `object`, the `QualifiedName` of the object type, process or
  state machine that declares the operation (for an inherited operation,
  the supertype that declares it, as FR-121's `OperationSite` names it),
  and `operation`, the operation's `Identifier`. It is the one carrier of
  the operation selection: the payload names no operation. A frame request
  carries no `arguments` and no occurrence key.
- A `WitnessEnvelope<FrameCounterexample>` (FR-070) whose payload holds:
  - `anchor` and `frame`: the checked node identities of the operation's
    `state`/`operation_anchor` and `state`/`frame` nodes (FR-105), and the
    frame's `generated` occurrence key (FR-104);
  - `invocation`: the `DocumentRef` (FR-106) of the invocation, with its pre
    and post snapshots in the request's byte provision by `sha256-jcs`
    digest;
  - `change`: the change the counterexample claims: an object reference and
    a field name for a write, an object reference and its type for a
    creation or deletion, or an object reference and its pre and post types
    for a retyping, or a relationship, an added or removed difference and
    the source and target object references for a link (the five change
    kinds of FR-106's check 11, which FR-115 runs).
- The envelope's members for a frame packet: its `clause_node` is the
  frame node, its `occurrence_key` is the frame's `generated` occurrence,
  and its `obligation_identity` is the O-09 obligation identity of the
  frame's `operation-contract` record (ADR-017 PF-4). Its replay target is
  the packet's copy of the request's (FR-070).
- A check-time FR-115 frame witness decodes to the `change` it claims:
  the witness's population and object key, and its field or types (for a
  link, its relationship, difference and two end references), without the
  frame permission.
- FR-098's package reference, byte provision and limits.

## Outputs

- An FR-072 replay result on the envelope's arm, or a typed `ReplayRefusal`
  with no partial result. A result retains the source identity and digest,
  the `package_id`, the request's `frame_operation`, the payload's anchor,
  frame and occurrence identities, the identity and digest of the invocation and both snapshots,
  the witness's `change`, and the change the replay found.

## Behavior

- The executor SHALL decode the request by FR-098's rules first, refusing
  by FR-098's decode refusals.
- The executor SHALL apply the three envelope checks below after the request decodes by FR-071 (QSpec FR-323 decode rule 1, then the byte-provision rule) and before it recompiles, in the order listed, refusing with the first that applies:
  1. If the request's replay target is not `frame_operation`, then the
     executor SHALL refuse `invalid_runtime_input`/`unknown-member` at that
     member's `/replay/<member>` pointer, naming the target, the entry
     `replay_frame` and the member it reads (`frame_operation`). The code
     and pointer tell it apart from QSpec FR-323's refusal of a request
     carrying two targets (`invalid-value`).
  2. If the envelope's replay target differs from the request's, then the
     executor SHALL refuse `stale_dependency`/`content-mismatch`, naming
     both targets.
  3. If the envelope's `clause_node` differs from the payload's `frame`, or
     its `occurrence_key` from the payload's `occurrence`, then the
     executor SHALL refuse `stale_dependency`/`content-mismatch` (QSpec
     FR-272-AC-14 and native-diagnostics, quire-specification#174 and
     #176), naming the envelope's and the payload's identity.
- The executor SHALL recompile and check the package by FR-098's rules, in
  FR-098's order, before it resolves the request's `frame_operation`.
- If the recompiled `package_id` differs from the request's or the
  envelope's, then the executor SHALL refuse by FR-098's stale `package_id`
  rule.
- The executor SHALL resolve `frame_operation` by QSpec FR-342's operation
  resolution (QSpec FR-323). If `object` names no declaration, the object
  has no operation of that name, or the operation has no frame node, then
  the executor SHALL refuse `missing_declaration`/`missing-name`; if
  `object` names a node that cannot declare operations, or an object that
  only inherits the operation, then `invalid_model_binding`/
  `malformed-declaration`; and if two supertypes expose the name, then
  `ambiguous_declaration`/`ambiguous-name`; each naming the target. If the recompiled anchor or frame
  node identity differs from the payload's, or the frame's occurrence key
  differs, then the executor SHALL refuse `stale_dependency`/
  `content-mismatch` before any admission, naming the payload's and the
  recompiled identity. No identity is recovered from a display name.
- The executor SHALL admit the invocation by FR-115's admission, reading the
  three documents only from the byte provision.
- If admission fails, then the executor SHALL refuse with a `ReplayRefusal`
  holding FR-106's record.
- The executor SHALL run FR-115's frame check once and compare verdicts as
  FR-072 does. The counterexample refuted the frame, so the proved verdict is
  `violation`.
  - If the check reports a violation (FR-115), then the executor SHALL
    settle `reproduced-with-evaluated-witness`. The result carries the
    replay's frame witness beside the payload's `change`; when the first
    change the check finds is not the one the payload names, both stay in
    the result as found.
  - If the check finds nothing, then the executor SHALL settle
    `inconclusive` with cause `Verdicts` (`violation` proved, `success`
    replayed), never repaired (FR-072).
  - If the check refuses with `population_delta_mismatch`, then the
    executor SHALL settle `inconclusive` with cause `NoValue`: a refusal completes no
    value (FR-072).
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
| FR-116-AC-2 | The same envelope carrying the changed-version invocation (inside the frame) settles `inconclusive`, `Verdicts`, holding `violation` and `success`; carrying forbidden-parent-change with `change` (`child`, `versionNumber`) settles `reproduced-with-evaluated-witness` and holds both the payload's change and the replay's (`child`, `parent`); carrying an invocation whose `created` disagrees settles `inconclusive`, `NoValue`. | Test (TC-515) |
| FR-116-AC-3 | Stale identity: an envelope whose frame node identity is taken from a package whose `attemptUpdate` frame also modifies `parent` refuses `stale_dependency`/`content-mismatch` naming both frame identities, with no admission; a source edit that changes the `package_id` refuses by FR-098's stale `package_id` rule; a `frame_operation` whose `operation` is `missing` refuses `missing_declaration`/`missing-name` naming the target. | Test (TC-515) |
| FR-116-AC-4 | An envelope whose pre snapshot is absent from the byte provision refuses with FR-106's `unavailable_observation` record, and one whose invocation bytes differ from their digest refuses with `stale_dependency`/`content-mismatch`; neither settles a result. | Test (TC-515) |
| FR-116-AC-5 | Replaying one envelope twice gives equal results. `FrameCounterexample` implements `FamilyPayload`, and the envelope carries it as its generic parameter with no string-keyed field. | Test (TC-515) |
| FR-116-AC-6 | Envelope consistency: the step-1 envelope with its `clause_node` replaced by another package's frame node refuses `stale_dependency`/`content-mismatch` naming the envelope's clause node and the payload's frame node; with its `occurrence_key` at another ordinal it refuses the same way naming both occurrences. Each refuses before the recompile: the request's source does not compile, and the consistent envelope over the same request refuses at the recompile. | Test (TC-515) |
| FR-116-AC-7 | A hand-built envelope for `Orders::Pump::rehome` carrying TC-1037's invocation I under a frame that modifies no relationship and `change` the link `p1`-`t1` of `Pump-housing-Tank` `removed` settles `reproduced-with-evaluated-witness`, holding the payload's change and the replay's `Link` change; with `change` the link `p1`-`t2` `added` it settles the same and holds both changes as found; under a frame that modifies `Pump-housing-Tank` it settles `inconclusive`, `Verdicts`. | Test (TC-1037) |
| FR-116-AC-8 | Replay target, over FR-121-AC-18's domain package (`Sub` a subtype of `ConfigVersion` that inherits `attemptUpdate`), with the step-1 envelope rebuilt over that package and the request's and the envelope's target equal unless a case says otherwise: with the target `{Config::ConfigVersion, attemptUpdate}`, which FR-121's `OperationSite` returns for the `Config::Sub::attemptUpdate` selection, it settles `reproduced-with-evaluated-witness`; with `{Config::Sub, attemptUpdate}` it refuses `invalid_model_binding`/`malformed-declaration` naming the target. With the request's target `function` `sameIdentity` and the envelope's `{Config::ConfigVersion, attemptUpdate}` it refuses `invalid_runtime_input`/`unknown-member` at `/replay/function` naming `replay_frame` and `frame_operation` (check 1 precedes check 2). With the request's `{Config::ConfigVersion, attemptUpdate}` and the envelope's `{Config::ConfigVersion, probe}` it refuses `stale_dependency`/`content-mismatch` naming both targets, and does the same when the envelope's `clause_node` is also another package's frame node (check 2 precedes check 3). Each of these two refusals is unchanged when the request's source does not compile, and each becomes the byte-provision refusal (`missing_import`/`missing-selection`) when the source's byte-provision entry is removed. `FrameCounterexample` has no member naming the operation. | Test (TC-515) |

## Dependencies

- QSpec FR-323 (the `frame_operation` replay target, its decode refusals and
  its resolution) and FR-121 (the `OperationSite` whose `frame_operation`
  CG copies, CG FR-024).
- FR-070 (envelope, its replay target and `FamilyPayload`), FR-072 (replay result), FR-098
  (replay facade), FR-104 and FR-105 (frame identity and occurrence),
  FR-106 and FR-115 (admission and the frame check).
- ADR-013 O-25 to O-27; AD-016 (`WitnessBinding`).
- agent-ix/quire-contract-ir#109 and agent-ix/quire-contract-codegen#49 for
  the end-to-end path from a Kani frame counterexample.

The decode
from IR's witness waits on agent-ix/quire-contract-ir#109 and
agent-ix/quire-contract-codegen#49.
