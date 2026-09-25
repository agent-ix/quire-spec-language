---
id: FR-072
title: "Implement the typed replay result and record carrier"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: traces_to
---
# FR-072: Implement the typed replay result and record carrier

## Description

QSL SHALL implement a typed replay result, defined in the layer-6 `replay`
module and part of its public API (ADR-013 O-27), as one per-item result
whose `arm` member is a sum of two distinct, independently-typed arm
results — a `Witness`-arm result and an `Input`-arm result — each carrying
its own settlement, the O-16 category, the evaluated value, the nested FR-351
separating-witness record when the settlement basis is decisive, the
resolved nested regions (ADR-013 O-12), the replay charges, and the
executor's toolchain pin.

Parity is agreement between the proved and replayed verdicts, each verdict
taken from the QSpec outcome-to-verdict map fixed per O-16 category
(ADR-013 QC-8). A disagreement SHALL settle `inconclusive` with a typed
cause, never repaired: the result type exposes no public constructor,
setter, or conversion that can turn a disagreement into an agreement
result. Agreement on an `Input`-sourced packet SHALL settle
`reproduced-without-witness`, distinct from `reproduced-with-evaluated-witness`
on the `Witness` arm and never counted as backend evidence.

This requirement builds the result type and its round trip only; it
performs no comparison against a live replay execution (#243 produces the
executed native outcome this type carries) and defines no new witness or
replay type for any consumer, including #217's function-application
exemplar.

## Inputs

- A native execution outcome (kernel `Outcome`, ADR-013 O-16) for the
  replayed item.
- The originating `ReplaySource` arm (`Witness` or `Input`) and, for the
  `Witness` arm, the FR-351 separating-witness record decoded from it.
- Replay accounting charges and the executor's toolchain pin.

## Outputs

- A typed replay result carrying the O-16 category, arm-specific settlement,
  nested FR-351 witness record where decisive, resolved regions, charges and
  toolchain pin.

## Behavior

- The result's `arm` member SHALL be a sum of a `Witness`-arm result and an
  `Input`-arm result; each arm SHALL carry its own `settlement`, and no
  shared, arm-agnostic Boolean or flag SHALL represent agreement across both
  arms.
- Agreement on the `Witness` arm SHALL settle `reproduced-with-evaluated-witness`.
- Agreement on the `Input` arm SHALL settle `reproduced-without-witness`,
  never reported or convertible into backend evidence. AD-016's
  Replay-ownership table gives the CG parity comparator's sealed
  backend-evidence-verdict type — constructible only from an agreeing
  `Witness`-arm result, with no public field and no `Deserialize`,
  `Default`, `From` or `TryFrom` — as the type that actually enforces this
  in CG; that sealed type belongs to CG, not this requirement. This
  requirement's own obligation is narrower and is what CG's comparator
  depends on: the `Witness`-arm and `Input`-arm result types SHALL be
  distinct types with no `From`, `TryFrom`, `Into` or blanket conversion
  between them in either direction, so that no call site typed to accept a
  `Witness`-arm result can be satisfied by an `Input`-arm result.
- A disagreement between the proved and replayed verdicts (each taken from
  the fixed O-16-category-to-verdict map) SHALL settle `inconclusive` with a
  typed cause, with no public API path on the result type that constructs
  an agreement result from disagreeing verdicts.
- An arm result's evaluated value, and a `Witness`-arm result's FR-351
  record with it, SHALL be absent when the replay completed no value (a
  `refused`, `incomplete` or `undefined` S6a outcome, or a family result).
  A result with no value SHALL settle `inconclusive` with a `NoValue`
  cause whatever the two verdicts are; a result with a value settles
  `inconclusive` with a `Verdicts` cause when the verdicts differ. The
  evaluated value is a Boolean, a stand-in for the kernel `Value`, which
  has no structural equality. **Amended by QSL-5** (FR-098): the value and
  record used to be required, so a replay that completed no value could
  not be represented.
- A construct → serialize → read round trip of a decisive `Witness`-arm
  result SHALL preserve the nested FR-351 record's deciding element, index,
  value path and trace position exactly.
- A comparison of two results for agreement SHALL read only the typed
  FR-351 fields, never a transcript's rendered text or a diagnostic message
  string.
- #217's function-application exemplar SHALL construct and compare a
  replay result using the result type together with FR-070's witness
  envelope and FR-071's request type, with no new witness or replay type
  defined in #217's own scope.
- The reader SHALL refuse a result whose encoded size exceeds the
  configured reader bound, and SHALL NOT return a truncated or
  partially-populated result in that case.

## Constraints

| ID | Constraint | Type | Validation |
|----|------------|------|------------|
| FR-072-CON-1 | The `native-run-result/2` wire this result type's serializer targets is QSpec's own contract (FR-352); this requirement's Rust type accepts and refuses versions exactly as FR-352-AC-5 requires, and adds no separate version vector of its own. `native-run-result/2`'s wire serializer itself is #186's (ADR-013 §5). | Design | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-072-AC-1 | Given a `Witness`-arm agreement and an `Input`-arm agreement, each settles its own distinct value (`reproduced-with-evaluated-witness` and `reproduced-without-witness` respectively); the `Witness`-arm and `Input`-arm result types are distinct with no `From`, `TryFrom`, `Into` or blanket conversion between them, so no `Input`-arm result can satisfy a call site typed for a `Witness`-arm result (the only input CG's sealed backend-evidence-verdict type, AD-016, admits). | Test (TC-189) |
| FR-072-AC-2 | Given a proved verdict and a replayed verdict for the same item that differ under the fixed O-16-category-to-verdict map, the result settles `inconclusive` with a typed cause, and no public constructor, setter or `From`/`TryFrom` conversion on the result type can produce an agreement result from those disagreeing verdicts. | Test (TC-190) |
| FR-072-AC-3 | A positive `Witness`-arm result's construct → serialize → read round trip preserves the nested FR-351 record's deciding element, index, value path and trace position exactly, and a subsequent equality/agreement comparison between two results reads only those typed fields, never a rendered transcript or message string. | Test (TC-191) |
| FR-072-AC-4 | #217's function-application exemplar constructs and compares a replay result using only this type together with FR-070's witness envelope and FR-071's request type, with no new witness or replay type defined in #217's repository scope. | Test (TC-192) |
| FR-072-AC-5 | A result whose encoded size exceeds the configured reader bound refuses, and no truncated or partially-populated result is returned. | Test (TC-191) |

## Dependencies

- **Upstream**: [FR-070](FR-070-implement-typed-counterexample-witness-envelope.md),
  [FR-071](FR-071-implement-typed-replay-request.md); ADR-013 O-27, O-16
  (category-to-verdict map); QSpec
  [FR-351](https://github.com/agent-ix/quire-specification/blob/main/spec/objects/protocol/FR-351-separating-witness-record.md)
  (separating-witness record) and
  [FR-352](https://github.com/agent-ix/quire-specification/blob/main/spec/functional/foundation/FR-352-mint-native-run-result-witness-wire.md)
  (`native-run-result/2` wire; both QSpec status **Draft** as of this
  writing — fully specified with acceptance criteria and cited here as the
  normative record shape this result type carries and the version-refusal
  rule FR-072-CON-1 delegates to). QSpec AD-016's Replay-ownership table
  (`spec/assurance/AD-016-semantic-family-extension-path.md`) assigns the
  "Replay result" row to QSL (#231, this requirement) and the separate
  "Backend-evidence verdict" row to the CG parity comparator; this
  requirement builds only the former and defines no backend-evidence
  verdict type itself.
- **Shared types**: #213 (ARCH-20) owns the O-16 category-to-verdict map's
  kernel `Outcome` type and the ADR-013 O-12 resolved-region type this
  result's nested fields reuse; this requirement adds no parallel type for
  either.
- **Downstream**: [#217](https://github.com/agent-ix/quire-spec-language/issues/217)'s
  function-application exemplar; [#186](https://github.com/agent-ix/quire-spec-language/issues/186)'s
  `native-run-result/2` serializer and state-specific payload; the CG
  parity comparator, which builds the sealed backend-evidence-verdict type
  (AD-016) from this requirement's `Witness`-arm result.
