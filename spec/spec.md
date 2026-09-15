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
  - target: ix://agent-ix/quire-spec-language/FR-034
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-035
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-036
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-040
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-041
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-009
    type: contains
  - target: ix://agent-ix/quire-spec-language/IT-010
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-038
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-042
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-047
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-048
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-049
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-050
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-051
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-009
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-010
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: contains
  - target: ix://agent-ix/quire-spec-language/NFR-008
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-033
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-032
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-029
    type: contains
  - target: ix://agent-ix/quire-spec-language/FR-028
    type: contains
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

## Native compiler, composed admission and executable profiles

## 1. Purpose

This root indexes the native compiler requirements, including the historical
finite-state implementation and composed-language parsing, static checking,
bounded state evaluation, temporal evaluation, choreography preservation and
versioned producer artifacts under compiler #35–#40. Discrete requirements
distinguish implemented, planned and externally gated behavior from recorded
qualification.

## 2. Scope

### 2.1 In Scope

LC01 native source/parse/format/diagnostics; LC02 model linking and typing; LC03 reference evaluation; LC04 qualified existing-IR lowering; LC05 existing-extractor integration; Rust-owned fixture verification under FR-012/NFR-005.

L2 adds edition-selected syntax and exact package linking across state, temporal
and choreography declarations through
[FR-035](functional/FR-035-parse-composed-native-units.md) and
[FR-036](functional/FR-036-link-composed-native-packages.md). L3/L4 add reusable
predicates, ordered queries and finite graph evaluation through FR-046/047/049
and NFR-009. L5 retains the merged bounded temporal evaluator under FR-043–045
and NFR-008. L6 preserves choreography for downstream assessment under FR-048.
FR-042 and FR-050 own the strict `/1` and `/2` compiled-artifact producer/
consumer boundaries. Each stage consumes shared model and observation contracts
without taking over their producer authority.

### 2.2 Out of Scope

The standard and E own temporal/protocol/message meaning; B owns PT02 protocol
conformance and results; D owns model/configuration and the version-locked
ecosystem integration; F owns concrete observation admission, replay and
completeness; C owns engineering assurance. This compiler does not execute
business operations or compensation, create an observation store, issue model
authority, project portable results, or claim the externally owned composed
campaign complete. Duplicate model/type authorities, a second editable formal
language, public publication and standard-wide license selection remain outside
this scope.

## 3. System Overview

### 3.1 System Description

The implementation preserves the historical finite-state profile and adds an
explicit composed path from source identity through edition recognition,
dependency binding, exact value/control graphs and versioned immutable artifacts.
Public evaluators consume only admitted artifacts plus caller-supplied typed
state/trace inputs and return closed completed/incomplete/refused/exhausted
outcomes under versioned work accounting. Archetype schemas, formal
declarations, producer models and runtime populations remain distinct. Contract
IR owns formal types and proof machinery; the native language standard owns the
meaning of clauses being compiled.

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

Operation observations belong to one invocation with an authored frame and exact
object delta. The historical finite-state profile does not define a protocol
state machine or temporal clock semantics. Composed state evaluation executes
only admitted pure value graphs over explicit immutable inputs. L5 evaluates
temporal requirements without owning observation transport, and L6 preserves
protocol/recovery structure without executing it. The shared standard and
downstream owners retain family meaning and assessment authority.

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

Run Quire over this exact repository scope and execute the serial Rust gates in
both feature configurations. Use independently authored expected values and real
existing producers/consumers at integration boundaries. Historical finite-state
acceptance covers healthy, violating and refused/incomplete workflows. TC-113–
121 cover composed parsing, linkage, checking and the `/1` producer; TC-122–125
cover L5; TC-126–137 cover the consolidated L3/L4/L6 and state-input contracts;
TC-138 covers the strict `/2` temporal selection extension. IT-009 remains the
narrow static D producer boundary. TC-135 proves only A's byte-exact compiler-to-
B intake contribution; D's research #39/#49 owns the later version-locked
aggregate campaign with accepted B/F revisions. A passed earlier stage cannot
qualify an unexecuted later one.

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

Use private owning issues and isolated worktrees. Under the approved initial specification cycle, A owns the compiler and shared language foundation; B owns protocol/results, D model/configuration, E temporal meaning and F observations/consumers. C continues engineering-assurance independently. Preserve original dependency/template grants. Standard-wide publication terms remain unresolved.

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
| [FR-035](functional/FR-035-parse-composed-native-units.md) | FR | Implemented composed syntax; compiler #35 |
| [FR-036](functional/FR-036-link-composed-native-packages.md) | FR | Binding and shared value types implemented; complete family/runtime-role admission open under #35 |
| [FR-040](functional/FR-040-check-composed-values.md) | FR | Shared types and supported guarded proofs implemented; query/runtime acceptance open under #35/#36 |
| [FR-041](functional/FR-041-admit-rational-native-model-profile.md) | FR | Explicit rational model admission implemented; delivered producer reviews pass |
| [IT-009](integration/IT-009-composed-package-boundary.md) | IT | Locally qualified direct producer/composed compiler boundary at accepted FCD merge `4042882`; 8/8 producer-correspondence tests pass |
| [IT-010](integration/IT-010-config-version-numeric-backends.md) | IT | Locally tested ConfigVersion numeric backend parity |
| [FR-038](functional/FR-038-encode-exact-protocol-numbers.md) | FR | Exact numeric wire component; compiler #40 |
| [FR-042](functional/FR-042-publish-compiled-protocol-artifacts.md) | FR | Typed wire reader implemented; complete family admission and real source-to-consumer emission remain open under #40 |
| [FR-043](functional/FR-043-evaluate-bounded-native-temporal.md) | FR | Planned bounded temporal evaluation; compiler #38 |
| [FR-044](functional/FR-044-activate-temporal-obligations.md) | FR | Planned temporal activation and immutable captures; compiler #38 |
| [FR-045](functional/FR-045-classify-temporal-mapping-support.md) | FR | Native-to-TL mapping support classification, including the `/2` authenticated selection; bridge emission blocked on quire-contract-ir #63/#64; compiler #38 |
| [NFR-008](non-functional/NFR-008-bound-temporal-evaluation.md) | NFR | Planned temporal work, instance and retention limits; compiler #38 |
| [FR-046](functional/FR-046-execute-predicates-and-ordered-queries.md) | FR | Retrospective L3 draft under #66; review, matrix binding and full query runtime acceptance remain pending |
| [FR-047](functional/FR-047-evaluate-finite-object-reference-graphs.md) | FR | Retrospective L4 finite graph contract under #66; implementation/evidence reconciliation pending |
| [FR-048](functional/FR-048-preserve-native-choreography-semantics.md) | FR | Retrospective L6 choreography preservation and ecosystem handoff contract under #66 |
| [FR-049](functional/FR-049-admit-composed-evaluation-inputs.md) | FR | Implemented exact admitted `/1` and `/2` composed-evaluation input and typed-outcome boundary; `/2` compensation expressions completed under #101 |
| [FR-050](functional/FR-050-publish-authenticated-temporal-artifacts.md) | FR | Reviewed-scope draft of compiler #40's strict compiled-protocol v2 temporal selection extension |
| [FR-051](functional/FR-051-publish-checked-native-handoffs.md) | FR | Implemented strict checked predicate and temporal subject owner handoffs for #90 |
| [FR-052](functional/FR-052-publish-native-temporal-evaluation-owner.md) | FR | Implemented canonical native temporal request/result owner boundary for Contract IR FR-026; review/merge pending |
| [FR-053](functional/FR-053-preserve-opaque-semantic-trigger-identity.md) | FR | Versioned opaque semantic-trigger identity bridge for Protocol FR-300 |
| [NFR-009](non-functional/NFR-009-bound-composed-evaluation.md) | NFR | Implemented charge-before-work bounds for shared `/1` and `/2` composed evaluation |
