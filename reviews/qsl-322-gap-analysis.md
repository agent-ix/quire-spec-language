---
id: SR-814
title: "QSL-322 gap analysis of PR 536 (ticket items, ADR-013 O-25/OQ-H, FR-070, FR-098 vs tests)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b95b671a01f151632be03adb078b83d7280d51ae; qsl-replay/src/witness.rs; qsl-replay/src/execute.rs; qsl-replay/src/execute/tests.rs; qsl-replay/src/identity.rs; qsl-replay/src/lib.rs; spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md (O-25, OQ-H); spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md (unchanged); spec/functional/FR-098-execute-a-replay-request.md (unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-070
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: reviews
---
## Summary

Ticket: QSL-322. PR: quire-spec-language#536 at b95b671a.

Ticket items, measured at head:

1. Typed decode. Done. `decode(&[WitnessBinding]) -> Result<Vec<WitnessValue>,
   DecodeRefusal>` matches O-25's `decode(&[WitnessBinding])` and the
   typed-values conversion row. The ADR was amended to state the concrete
   shape (`WitnessBinding` = parameter node id + `WitnessValueType`).
2. Wider Input. Done. `CanonicalAssignment.value: WitnessValue`. O-25 now
   says the Input arm carries the same `WitnessValue`. FR-098 was not
   updated to match (FND-001).
3. Stale docs. witness.rs "mirrors IR's own Witness" is removed. The
   identity.rs and lib.rs "provisional" claim for `ObligationIdentity` is
   removed. The replacement text over-claims (SR-813 FND-001). frame.rs
   "IR's `WitnessBinding`" is not fixed (SR-813 FND-002).
4. Ownership (FND-004 of SR-781). Resolved. The three types are QSL-owned
   in qsl-replay, and qsl-replay has no IR dependency.

Coder's open points:

- (1) Integer(0|1) for a Boolean Input parameter now refuses. This is a
  real gap: FR-098 Behavior and AC-2 still state the old rule (FND-001).
- (2) O-25 requires decode refusals to carry the packet's obligation
  identity. Nothing carries it (FND-002).
- (3) `WitnessBinding { parameter: WireNodeId, value_type }` matches O-25
  ("each binding names its parameter node id") and O-09 (per-argument
  domain). The declared domain is checked at S6a admission, not in the
  binding, which is consistent with "width or type mismatch refuses". No
  finding.
- (4) The frame.rs leftover is SR-813 FND-002. FR-070's "IR Witness" and
  "IR CounterexamplePacket" wording is FND-003 here.
- (5) The Unbound change breaks no caller or corpus (SR-813 check 5). No
  finding.

Test <-> AC bindings checked: `tc_180` -> FR-070-AC-1 (correct, now a real
value oracle). `tc_444_decode_reads_typed_values_by_parameter_node_id` ->
FR-098-AC-2 (correct). `tc_444_decode_refuses_each_join_failure_with_its_own_variant`
-> FR-098-AC-4 (correct: Behavior lists "a witness that does not decode").
`tc_444_a_witness_decodes_by_parameter_node_id` -> FR-098-AC-2 (correct).
`tc_444_a_type_or_domain_mismatch_refuses_as_wrong_value_kind` -> FR-098-AC-4
(correct). `tc_444_a_boolean_parameter_replays` -> FR-098-AC-2 (correct).

## Verdict

The code delivers ticket items 1, 2 and 4. Three spec gaps stay open.
FR-098's normative text now contradicts the code (FND-001, high, a
spec-only fix). O-25's obligation-identity provenance is not carried
anywhere (FND-002). FR-070 still describes the witness as IR's (FND-003).
Not mergeable until FND-001 is fixed. FND-002 needs a code change or an
explicit O-25 amendment in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-098 Behavior still says "Each canonical integer assignment SHALL become a value of its parameter's declared type: an integer for an integer type, and `0` or `1` (`false`, `true`) for `Boolean`". AC-2 says "a Boolean parameter takes `0` and `1`". At head, an `Input` assignment of `WitnessValue::Integer(0)` or `Integer(1)` for a Boolean parameter refuses `WrongValueKind` (execute.rs `argument`), and `tc_444_a_type_or_domain_mismatch_refuses_as_wrong_value_kind` asserts that `flag` with `Integer(1)` refuses. The spec says it must replay. Fix (spec only): Behavior says each `Input` assignment's typed `WitnessValue` must match the declared type (`Boolean` for `Boolean`, `Integer` for an integer type), a witness transcript entry reads `0`/`1` as a Boolean, and any other pairing refuses `WrongValueKind`. AC-2 says a Boolean parameter takes a typed Boolean assignment or a `0`/`1` witness entry. | spec/functional/FR-098-execute-a-replay-request.md:111-117; spec/functional/FR-098-execute-a-replay-request.md:144; qsl-replay/src/execute.rs:703-724 |
| FND-002 | medium | ADR-013 O-25 says "`decode` refusals carry the packet's obligation identity as provenance, not placeholder strings." `DecodeRefusal` names only the parameter node id (or the raw entry text, for `Unbound`). `ReplayRefusal::Witness(DecodeRefusal)` adds nothing. `replay(wire)` consumes the request, so the refusal a caller receives names no obligation, although the request holds `originating_counterexample_identity: ObligationIdentity`. No layer carries the identity. Scenario: CG replays a batch of counterexamples and gets `Witness(Missing(p))` with no way to tell which obligation failed. Fix: carry it (`ReplayRefusal::Witness { obligation: request.originating_counterexample_identity(), refusal }`, with a test), or amend the O-25 sentence in this PR to what the code does. | spec/decisions/ADR-013-canonical-type-package-conversion-ownership.md:893-894; qsl-replay/src/execute.rs:149; qsl-replay/src/execute.rs:666 |
| FND-003 | medium | FR-070 still describes the witness as IR's. Description: "stores the IR `Witness`'s admitted transcript" and "it does not implement `Witness::parse`'s admission rule itself (IR PR #139 and quire-contract-ir#144 own admission ...)". Inputs: "An IR `CounterexamplePacket{source: ReplaySource}`" and "An already-admitted IR `Witness`". Dependencies: "IR `Witness` (`src/kani/witness.rs`, IR PR #139 ...)". Under OQ-H, which this PR restates, QSL owns `Witness` and its admission (`qsl-replay` implements `Witness::parse`), IR defines no packet, and CG builds the envelope. The FR's "does not implement parse" statement is false against the code. Fix: rewrite these sentences to the QSL-owned `Witness`, with admission by `Witness::parse` in qsl-replay, and a CG-built envelope. | spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:21; spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:38-41; spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:45-49; spec/functional/FR-070-implement-typed-counterexample-witness-envelope.md:125-126 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b417e178: the FR-098 Behavior bullet now says each argument arrives as a typed `WitnessValue`. `Input` carries `Boolean` for `Boolean` and `Integer` for an integer type, a transcript entry reads `0`/`1` as a Boolean, and an integer for a Boolean parameter refuses `WrongValueKind`. AC-2 says the same. This matches execute.rs `argument` and the tests. |
| FND-002 | fixed | b417e178: `ReplayRefusal::Witness { obligation: ObligationIdentity, refusal: DecodeRefusal }`, filled from `request.originating_counterexample_identity()` in `arguments`. `tc_444_a_witness_decodes_by_parameter_node_id` sets the identity to `[7; 32]` and asserts both the field and its presence in `Display`. The FR-098 refusal list names it. |
| FND-003 | fixed | b417e178: FR-070 Description, Inputs and Dependencies now name QSL's `Witness`, with admission by `Witness::parse` in qsl-replay, a CG-built envelope, and OQ-H. The new Inputs term `WitnessPacket` is the real exported qsl-replay type (witness.rs:745, the wire shape `WitnessEnvelope::reconstruct` reads). It matches ADR-013 O-25's "a packet missing any member is refused at reconstruction". |
