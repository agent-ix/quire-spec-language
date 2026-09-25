---
id: FR-070
title: "Implement the typed counterexample/witness envelope"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: traces_to
---
# FR-070: Implement the typed counterexample/witness envelope

## Description

QSL SHALL implement a typed counterexample/witness envelope, defined in the
layer-6 `replay` module and part of its public API (ADR-011 FB-05; ADR-013
O-25), that stores the IR `Witness`'s admitted transcript as its one stored
fact and derives every other witness fact (`harness_symbol`,
`check`/`check_text`, `concrete_values`, `decode`) from that transcript on
every call, never as a second, independently-stored copy (ADR-013 QC-13; the
AD-016 Replay-ownership row's five `Witness` fields collapse to this one
stored field plus four derived ones).

The envelope carries, for deterministic replay, every ADR-013 O-25 member:
the obligation identity and clause occurrence key; the selected function's
`QualifiedName`; the `package_id`, contract version and `RawSourceRef`
digests of the package the harness was generated from; the semantic profile
selections; the `run_limits` and declared domains; the `backend` member
(O-19); the trace position where the family has one; and the `ReplaySource`
(`Witness` or `Input`). A packet missing any of these members SHALL be
refused at reconstruction; none is optional or defaulted.

This requirement builds the common envelope only. It does not add any
family-specific payload (that is #186's), and it does not implement
`Witness::parse`'s admission rule itself (IR PR #139 and
agent-ix/quire-contract-ir#144 own admission; this envelope's constructor
requires an already-admitted `Witness`).

## Inputs

- An IR `CounterexamplePacket{source: ReplaySource}` and its O-25 members
  (obligation identity, occurrence key, package reference, profile
  selections, bounds, backend, trace position).
- An already-admitted IR `Witness` (admitted through `Witness::parse`) for
  the `ReplaySource::Witness` arm, or canonical assignments for the
  `ReplaySource::Input` arm.

## Outputs

- A typed counterexample/witness envelope carrying the O-25 members above,
  or a structured refusal when any required member is absent or a
  transcript fails admission.

## Behavior

- The envelope SHALL store the admitted transcript as its only witness-shape
  field; `harness_symbol()`, `check()`, `check_text()`, `concrete_values()`
  and `decode(&[WitnessBinding])` SHALL each recompute their answer from the
  stored transcript on every call, so the envelope cannot disagree with its
  own backend evidence.
- Envelope construction SHALL refuse a transcript that is not the exact,
  single, trimmed assertion block `Witness::parse` selects (a cover
  transcript, an untrimmed transcript, or a transcript with zero or more
  than one assertion block), and SHALL NOT produce a partially-populated
  envelope in that case.
- A construct → serialize → read round trip SHALL preserve the stored
  transcript byte-for-byte and every O-25 member exactly: obligation
  identity, occurrence key, `package_id` and every `RawSourceRef` digest,
  semantic profile selections, `run_limits` and declared domains, `backend`
  member, trace position, and the `ReplaySource` variant (with its
  `Witness`/`Input` payload).
- Envelope reconstruction SHALL refuse when any one O-25 member is absent,
  and SHALL NOT substitute a default value for the missing member.
- The common envelope type SHALL expose an extension point through which a
  family (starting with #186's state `forall`) attaches its own typed
  payload as a distinct, family-owned type, without adding a variant,
  field, or special case to the common envelope's own type or constructors.
  The common envelope type SHALL define that extension point as a trait a
  family-owned payload type implements, or as a generic parameter the
  common envelope carries, and SHALL NOT define it as a `String`-keyed or
  otherwise untyped map that a family populates by convention.
- This envelope carries no `contract_version` of its own: version ownership
  is delegated to the FR-331 envelope (FR-069) within which a
  `counterexamples` entry travels, and to the closed FR-201 digest-domain
  set every `RawSourceRef` and `package_id` digest this envelope stores must
  belong to. Envelope construction SHALL refuse a digest whose domain falls
  outside that closed set.
- Envelope construction SHALL refuse when the envelope's encoded size
  exceeds the configured reader bound, and SHALL NOT return a truncated or
  partially-populated envelope in that case.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-070-CON-1 | Decoding a witness or its derived facts SHALL rely only on `Witness`'s own typed accessors, never on parsing or interpreting the transcript's rendered/display text. | Design | Test (TC-182) |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-070-AC-1 | The envelope stores the transcript once; `harness_symbol`, `concrete_values` and `decode` each recompute their result from that one stored transcript rather than from a separately-stored value, so no code path can leave the envelope's derived facts disagreeing with its transcript. | Test (TC-180) |
| FR-070-AC-2 | A cover transcript, an untrimmed transcript, and a transcript with zero or two assertion blocks each refuse at envelope construction; none produces a partially-built envelope. | Test (TC-181) |
| FR-070-AC-3 | A positive envelope's construct → serialize → read round trip preserves the transcript byte-for-byte and every O-25 member (obligation identity, occurrence key, package reference and digests, profile selections, bounds and domains, backend, trace position, `ReplaySource` variant) exactly, with no member re-derived, reordered or dropped. | Test (TC-182) |
| FR-070-AC-4 | An O-25 packet missing any one required member (for example, no trace position on a family that carries one, or no `backend` member) refuses at reconstruction; no envelope is built with a defaulted or absent value in that member's place. | Test (TC-183) |
| FR-070-AC-5 | #186 can add a state-`forall`-specific witness payload as a typed consumer of the envelope's extension point, in #186's own change, with no edit to this envelope's type, constructors, or round-trip contract, and the extension point itself is typed (a trait or generic parameter), never a `String`-keyed untyped map. | Test (TC-184) |
| FR-070-AC-6 | This envelope defines no `contract_version` member of its own; a `RawSourceRef` or `package_id` digest it stores whose domain falls outside the closed FR-201 digest-domain set refuses at construction. | Test (TC-181) |
| FR-070-AC-7 | An envelope whose encoded size exceeds the configured reader bound refuses at construction, with no truncated or partially-populated envelope returned. | Test (TC-181) |

## Dependencies

- **Upstream**: [US-010](../usecase/US-010-carry-a-proof-witness-or-replay-outcome-without-a-shadow-type.md);
  ADR-013 O-25 (witness/counterexample carrier), QC-13 (transcript-only
  storage), QC-6/QC-8 (FR-331 `counterexamples` shape); IR `Witness`
  (`src/kani/witness.rs`, IR PR #139, merged at `954c2f2`) and its admission
  rule (agent-ix/quire-contract-ir#144); QSpec
  [FR-331](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/interfaces/FR-331-backend-provider-envelope.md)
  AC-9 (`counterexamples` entry shape) and
  [FR-351](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/protocol/FR-351-separating-witness-record.md)
  (separating-witness record; QSpec status **Draft** as of this writing —
  fully specified with acceptance criteria, cited here as the normative
  shape this envelope's `Input`-arm and #217/#186 consumers read).
- **Shared types**: [#213](https://github.com/agent-ix/quire-spec-language/issues/213)
  (ARCH-20) owns the O-04 `NodeKey`/occurrence-key shapes the obligation
  identity and occurrence key members reuse, the O-18 digest record the
  `RawSourceRef` and `package_id` digests reuse, and the O-11 `QualifiedName`
  the selected function's identity reuses; this requirement defines no
  parallel identity or digest type.
- **Downstream**: [FR-071](FR-071-implement-typed-replay-request.md) carries
  this envelope's members into the replay request;
  [FR-072](FR-072-implement-typed-replay-result.md)'s per-item result embeds
  the FR-351 record this envelope's `Witness` arm decodes.
