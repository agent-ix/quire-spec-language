---
type: master-requirements
name: quire-spec-language
org: agent-ix
component_type: rust-lib
implementation_language: rust
standards_alignment:
  - iso-iec-ieee-29148
  - ieee-828
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-002
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-004
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-011
    type: contains
  - target: ix://agent-ix/quire-spec-language/US-001
    type: contains
  - target: ix://agent-ix/quire-spec-language/US-002
    type: contains
  - target: ix://agent-ix/quire-spec-language/US-003
    type: contains
  - target: ix://agent-ix/quire-spec-language/US-004
    type: contains
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-001
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-002
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-003
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-004
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-001
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-002
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-003
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-012
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-004
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-005
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-014
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-017
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-006
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-006
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-019
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-020
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-021
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-022
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-023
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-024
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-025
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-007
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-007
    type: contains
---
# Master Requirements Specification

## Native finite-state compiler and executable core

## 1. Purpose

This root indexes the discrete requirements for Agent A's assigned native finite-state work. It corrects the earlier proposal-only authoring by using the requested /specify catalog contract. All artifacts remain draft with conditional /spec-review findings; retrospective documentation does not retroactively approve implementation.

## 2. Scope

### 2.1 In Scope

LC01 native source/parse/format/diagnostics; LC02 model linking and typing; LC03 reference evaluation; LC04 qualified existing-IR lowering; LC05 existing-extractor integration; Rust-owned fixture verification under FR-012/NFR-005.

### 2.2 Out of Scope

Temporal/protocol/message semantics, B's portable method/plan/result schemas, C's existing-repository integration work, a duplicate archetype model or Contract IR type authority, a second executable binder, public publication and standard-wide license selection are outside this repository's implementation ownership. A owns native semantics and concrete source-to-formal projections needed by its proof cases. These boundaries do not remove the full supported-state workflow.

## 3. System Overview

### 3.1 System Description

The language defines an explicit finite-state profile. The native pipeline preserves source identity through parsing, model linking, runtime validation, reference evaluation and qualified backend projection. Archetype schemas, explicit formal declarations and runtime populations are distinct. Contract IR owns formal types and its executable binder; the modeling language owns the semantics and qualified projection of concepts its clauses require.

### 3.2 Intended Users

Specification authors, model producers, verification operators and toolchain integrators.

## 4. Requirements Architecture

Each stakeholder requirement, story, functional requirement, quality constraint and integration specification is a separate catalog-typed file. Proposal documents remain detailed design inputs; this index does not replace those requirements.

## 5. Requirement Classes

StR states the operational need. US records user intent. FR defines observable behavior. NFR records measurable quality constraints. IT specifies a real external process/API/file boundary. TC and matrix coverage are assessed by the review workflow.

## 6. Requirement Identification

IDs are stable within this repository. Cross-repository identities use ix://agent-ix/quire-spec-language/ID. Acceptance criteria use the parent ID plus -AC-N; integration steps use -SC-NN.

## 7. Requirement Quality Policy

Requirements use atomic observable outcomes. Unknown support, absent evidence and unimplemented prerequisites remain explicit. Validation of Markdown structure cannot establish semantic correctness or implementation completion.

## 8. State and Execution Model

### 8.1 State Semantics

Runtime inputs are immutable typed finite populations with explicit closure assumptions. Object identity differs from record equality. Optional absence, invalid input and unavailable observation retain distinct meanings.

### 8.2 Transition Semantics

Operation observations belong to one invocation with an authored frame and exact object delta. This first profile does not define a protocol state machine or temporal clock semantics.

### 8.3 Determinism Guarantees

Guard and collection evaluation order, source activation and selected canonical domains remain explicit. Resource exhaustion yields incompleteness, not a logical violation.

## 9. Events and Signals

### 9.1 Event Model

The pipeline may report observed implication activation. It introduces no domain event bus or observation protocol.

### 9.2 Event Guarantees

Only events actually observed during evaluation are reported. Missing backend probes remain unavailable coverage.

## 10. Error and Failure Model

### 10.1 Error Classification

Invalid syntax/input and unsupported capabilities refuse their stage. Missing observations, cancellation and exhausted resources remain incomplete.

### 10.2 Failure Handling Guarantees

No failed prerequisite yields a completed Boolean. No unsupported obligation is silently erased.

## 11. Traceability

Frontmatter relates StR, US, FR/NFR and IT artifacts. Acceptance-criterion-to-TC coverage and implementation trace tags require the review workflow; existing Rust tests do not automatically create formal TC identities.

## 12. Verification Strategy

Run Quire over this exact repository scope. Use real existing producers/consumers for integration evidence. The end-state acceptance remains a supported healthy, violating and refused/incomplete native state workflow. Existing syntax tests and authored semantic examples are partial evidence only.

## 13. Change Management

The user requested /specify and /spec-review for all work. Existing implementation is being specified retrospectively and remains subject to findings. Further dependent implementation waits for the required review and accepted shared contracts. Historical fixture and profile bytes remain immutable under their selected bindings.

## 14. Lifecycle Status

Draft requirements. The admitted implementation target and new-code AGPL choice were approved in the task conversation. On 2026-09-08 the owner also adopted specification PR8 at e897f810a7356d4ce8fd19026221ebda7b65596f for internal implementation. Accepted IR ADR-0054 at 690bde7f2dc58662cf9ff0595c2c0e3b17107c6f subsequently supersedes the Filament model-authority assumption and closes IR #54 without a new reader. LC02 proceeds against existing FR-013/019/023 APIs. IT-005/TM-003 remain planned; native implementation, concrete semantic projection qualification, independent consumer adoption and broader FS evidence remain work rather than external #54 prerequisites.

The PR10 model/checker scope is now qualified under Plan-005 and SR-083–087.
TM-003 contains its 35 qualified cases; the earlier planned status above is
historical. The LC03 packet adds FR-018, expands FR-007/008, and defines NFR-006,
IT-006 and TM-004. Its 23 runtime cases are planned. Its input/evaluation API
specification does not close LC02/FS03 acceptance or replace IT-002.

## 15. Governance Notes

Use private owning issues and isolated worktrees. A owns the native producer and these requirements; B owns portable verification contracts; C owns coordinated existing-repository consumers. Do not spawn additional agents under the current assignment. Preserve original dependency/template grants. Standard-wide publication terms remain unresolved.

## 16. References

The full task is tmp/formalization-agent-a-language-core.md in the workspace. The current authoring pack selected agent-ix from the Git remote. Authoring used Quoin 0.23.1, Quire 0.31.0 and the installed spec-artifacts-iso catalog shapes.

## Requirements

| Artifact | Type | Status |
| --- | --- | --- |
| [FR-001](functional/FR-001-read-exact-source.md) | FR | Draft |
| [FR-002](functional/FR-002-parse-native-units.md) | FR | Draft |
| [FR-003](functional/FR-003-format-native-source.md) | FR | Draft |
| [FR-004](functional/FR-004-verify-source-maps.md) | FR | Draft |
| [FR-005](functional/FR-005-link-shared-model.md) | FR | Draft |
| [FR-006](functional/FR-006-check-defined-expressions.md) | FR | Qualified native static judgments |
| [FR-007](functional/FR-007-validate-runtime-inputs.md) | FR | Draft |
| [FR-008](functional/FR-008-evaluate-state-reference.md) | FR | Draft |
| [FR-009](functional/FR-009-lower-qualified-projections.md) | FR | Draft |
| [FR-010](functional/FR-010-report-native-outcomes.md) | FR | Draft |
| [FR-011](functional/FR-011-integrate-opaque-extraction.md) | FR | Draft |
| [US-001](usecase/US-001-author-native-source.md) | US | Draft |
| [US-002](usecase/US-002-link-exact-models.md) | US | Draft |
| [US-003](usecase/US-003-evaluate-bounded-state.md) | US | Draft |
| [US-004](usecase/US-004-reuse-existing-toolchain.md) | US | Draft |
| [StR-001](stakeholder/StR-001-native-assessment-trust.md) | StR | Draft |
| [NFR-001](non-functional/NFR-001-bound-syntax-work.md) | NFR | Draft |
| [NFR-002](non-functional/NFR-002-reproduce-native-builds.md) | NFR | Draft |
| [NFR-003](non-functional/NFR-003-preserve-uncertain-outcomes.md) | NFR | Draft |
| [NFR-004](non-functional/NFR-004-preserve-implementation-rights.md) | NFR | Draft |
| [IT-001](integration/IT-001-real-model-producer.md) | IT | Draft |
| [IT-002](integration/IT-002-native-state-workflow.md) | IT | Draft |
| [IT-003](integration/IT-003-opaque-extraction-adapter.md) | IT | Draft |

| [FR-012](functional/FR-012-audit-fixtures-in-rust.md) | FR | Draft Rust verification remediation |
| [NFR-005](non-functional/NFR-005-rust-verification-paths.md) | NFR | Draft campaign language constraint |
| [IT-004](integration/IT-004-rust-fixture-audits.md) | IT | Draft real Rust audits |
| [IT-005](integration/IT-005-qualify-native-model-consumption.md) | IT | Planned qualified model consumption |
| [FR-013](functional/FR-013-link-formal-environments.md) | FR | Implemented formal linking |
| [FR-014](functional/FR-014-bind-native-formal-source.md) | FR | Implemented native/formal source bridge |
| [FR-015](functional/FR-015-project-native-model-semantics.md) | FR | Planned native semantic roles |
| [FR-016](functional/FR-016-check-native-clauses.md) | FR | Qualified type/definedness checker |
| [FR-017](functional/FR-017-separate-qualification-stages.md) | FR | Qualified construction-stage repairs; SR-083 |
| [FR-018](functional/FR-018-construct-native-runtime-inputs.md) | FR | Planned LC03 byte-bound input construction |
| [NFR-006](non-functional/NFR-006-bound-native-runtime.md) | NFR | Planned runtime work/content limits |
| [IT-006](integration/IT-006-native-reference-workflow.md) | IT | Planned native reference API qualification |
