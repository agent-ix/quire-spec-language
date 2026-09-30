---
id: SR-722
title: "Round-two code and spec review of the string-edge fix round"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; src/protocol_artifact/mod.rs; src/temporal.rs; src/protocol_artifact/native_temporal/request.rs; src/protocol_artifact/validate.rs; src/protocol_artifact/intake.rs; src/protocol_artifact/v2/intake.rs; src/protocol_artifact/v3/intake.rs; src/protocol_artifact/native/metadata.rs; src/protocol_artifact/native/mod.rs; src/protocol_artifact/native/temporal_v2.rs; src/protocol_artifact/handoff/writer.rs; src/state/input.rs; src/state/evaluation.rs; src/mapped.rs; src/linking.rs; qsl-semantics/src/check/family.rs; qsl-semantics/src/check/claims.rs; qsl-semantics/src/check/lowering.rs; xtask/src/string_edge.rs; spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md; spec/test-cases/TC-162-string-edge-scan-coverage.md; spec/decisions/ADR-012-semantic-family-extension-contracts.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: reviews
---
## Summary

Ticket: QSL-145 (cross-reference QSL-268). PR: quire-spec-language#485.
This review covers the fix round diff: four code
commits plus the review-file commit. SR-709, SR-710 and SR-711 carry the
dispositions of the first-round findings.

The reviewer measured these in a separate detached worktree:

- `cargo xtask string-edge` exits 0.
- `cargo test -p xtask --lib string_edge`: 17 passed.
- `cargo test --test it` filtered to protocol_v2, state_evaluation,
  layering, temporal and protocol_artifact: 132 passed.
- `qsl-semantics` lib, claims tests: 12 passed.
- `qsl-semantics` lib, occurrence/identity/checking tests: 58 passed.
- Root lib, state/temporal/protocol/linking/mapped tests: 18 passed.
- `quire validate` on the edited specs and the reviews: exit 0.

Mutations, each reverted afterwards:

- `&&`/`||` widening off: fails the real-site test at `AdapterArtifact::try_from`.
- `strip_prefix` widening off: fails the real-site test at `clock_binding_name`.
- Arm-value widening off: fails the combinator fixture.
- A plain `mod plain_tests;` file with a string compare, and new compares in
  `features` and `mapped::compile`: all three are reported (exit non-zero).

New `#[string_edge]` marks in the fix round, each judged:

- `OccurrenceRole::of`: a conversion to a closed enum. Genuine.
- `NativeLanguage::of`: a conversion to a closed type. Genuine.
- `FamilyFeature::from_wire`, `RequiredFeature::from_wire`: conversions to
  closed enums. Genuine.
- `CanonicalizationDomain::from_str`: a conversion to a closed enum. Genuine.
- `AdapterArtifact::try_from`: a validating constructor. Genuine.
- `protocol_artifact::clock_binding_name`: a single reader, called once per
  admission. Genuine.
- `is_workflow_apply`: a marked boolean predicate. See FND-002.

Behaviour preserved:

- `Graph::profile` over `RegisteredDefinition`: same identity sets per
  `Family` arm. The native path passes metadata's per-definition
  `registered`.
- `FamilyFeature`/`RequiredFeature`: variant order equals the sorted wire
  order, so the list/set equality is unchanged.
- `OccurrenceRole`: variants are alphabetical, matching the old `Role`
  string order, with the same kernel spellings.
- `ByteDigest::from_hex` enforces the old 64-lowercase-hex rule.
- The request-derivation clock index agrees with the package, because the
  subject document is re-derived and compared on read.
- Exception: one refusal changed (FND-001).

## Verdict

**Approve with low findings.** All first-round findings are fixed except
SR-710 FND-002. That one is deferred to QSL-287, which exists, carries the
scope, and is reflected in FR-064 Status. The three findings below are low
and non-blocking.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `ClockNames::of` quietly maps a Clock-kind binding without the `clock:` prefix to `None`, instead of refusing at admission with a typed cause. `validate` checks only the binding kind (validate.rs `binding`), so such a package is admitted. `temporal::run` then refuses it with `Refusal::Reference`, where 9686fc5f gave `Refusal::Binding { dimension: Clock }`. This is a refusal-category change, and no test pins it. Refuse the package at admission, or keep the old cause. | src/protocol_artifact/mod.rs:62-77; src/temporal.rs:389-398 |
| FND-002 | low | `is_workflow_apply` is a marked boolean predicate over two model identity strings in the handoff writer. It is not a conversion to a closed type, and the emitter is not on ADR-012 §9's edge list. It is narrow, so it cannot silence other compares, but it is still a string decision after the edge. | src/protocol_artifact/handoff/writer.rs:855-860 |
| FND-003 | low | Two deferred items have no owning ticket in the spec. FR-064 Status points to branch `task/145-268-string-edge-consts` but not to QSL-287, the ticket that owns the unbuilt string-value clause. ADR-012 §9's clock row says "a typed clock-role variant in the wire form is not built" and names no owner. Name QSL-287 in FR-064 Status and in the ADR-012 deferral row, and give the clock-role remainder a ticket. | spec/functional/FR-064-restrict-string-dispatch-to-marked-edges.md:132-142; spec/decisions/ADR-012-semantic-family-extension-contracts.md:858 |
