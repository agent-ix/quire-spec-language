---
id: US-011
title: "Register a backend's capabilities without editing the lowering catalog"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-076
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-077
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-078
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-079
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-080
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-011: Register a backend's capabilities without editing the lowering catalog

## Story

**As a** backend integrator adding a new proof or analysis backend to QSL
(Kani today; Verus, SMT or an implication backend later)
**I want** my backend to register the capability kinds and modes it
advertises through one open contract, instead of a change to a fixed,
hand-maintained enum of lowering targets
**So that** adding my backend costs a registration, not a change to QSL's
core type, and a claim my backend does not cover settles as an honest
`unsupported` result rather than a build break, a refusal, or a request
that waits indefinitely for a backend that may never register.

## Context

`src/lowering/target.rs` currently defines `ProjectionTarget` as a fixed
three-variant catalog (`BooleanOracleV1`, `IntegerIrV1`, `StateScalarIrV1`)
built with the `targets!` macro. Every new backend needs a QSL code change to
add a variant, and there is no registration boundary a backend implements
once. Separately, the composed linker still reads backend support directly
when it records a capability request, and QSL's `value::ieee` and
`value::division` modules still carry `negotiate_*` functions left over from
before capability negotiation moved to `quire-contract-codegen`'s single
`negotiate_*` point (ADR-012 §7, §10 OBS-003, OBS-004). Kani is the first
and, for this story, only backend that actually registers; the registration
contract is what this story asks to exist, not multiple backends.

## Acceptance Examples (Illustrative)

### US-011-EX-1: A registered backend's kind produces a candidate

- **Given** Kani is registered advertising `value-validity`.
- **When** an item requiring `value-validity` is routed with no named
  backend.
- **Then** the candidate set contains Kani and no other registrant.

### US-011-EX-2: No registrant advertises the needed kind

- **Given** no registered backend advertises `temporal-satisfaction`.
- **When** an item requiring `temporal-satisfaction` is routed.
- **Then** the item settles `unsupported` with a warning naming
  `temporal-satisfaction`, never a refusal and never a request left open
  waiting for a future registration.

### US-011-EX-3: Registration order does not change the outcome

- **Given** two backends are registered, once in one order and once in the
  reverse order, with nothing else different.
- **When** the same items are routed against each resulting registry.
- **Then** every item's candidate set is the same list in the same order in
  both cases.

## Constraints (Contextual)

The registry consumes the canonical `Capability` value type
([quire-spec-language#213](https://github.com/agent-ix/quire-spec-language/issues/213))
and defines no second capability vocabulary or competing enum. Existing Kani
lowering corpora must keep producing the same output across the swap from
the fixed catalog to the registry.

## Dependencies (Contextual)

Upstream: the capability-kind vocabulary and backend-absence policy
(quire-specification FR-290, quire-specification#116) and the QSL admission
side of the same contract (FR-057). Downstream: `quire-contract-codegen`'s
`negotiate_*` taking the candidate set
([quire-contract-codegen#86](https://github.com/agent-ix/quire-contract-codegen/issues/86)),
because only `negotiate_*` settles a disposition; this story's registry
supplies candidates, not dispositions.

## Priority and Risk (Informative)

Priority: High. This registry is an architectural prerequisite for every
backend beyond Kani (Verus, SMT, an implication backend) and for lane E's
model/relation and temporal/protocol lowering, which route through it. The
risk of leaving the fixed catalog in place is that every future backend
requires a QSL core change and there is no honest, uniform way to report
"no backend covers this claim" short of a refusal or an indefinite hold.

## Traceability (Informative)

- [FR-075](../functional/FR-075-compute-candidates-from-registered-backends.md)
- [FR-076](../functional/FR-076-settle-backend-absence-as-unsupported.md)
- [FR-077](../functional/FR-077-remove-composed-linker-backend-negotiation.md)
- [FR-078](../functional/FR-078-remove-qsl-negotiate-copies.md)
- [FR-079](../functional/FR-079-preserve-kani-lowering-corpora.md)
- [FR-080](../functional/FR-080-registry-evidence-and-gates.md)
