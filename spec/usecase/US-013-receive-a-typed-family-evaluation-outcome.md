---
id: US-013
title: "Receive a typed family evaluation outcome, never a panic or a conflated refusal"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-090
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-013: Receive a typed family evaluation outcome, never a panic or a conflated refusal

## Story

**As a** caller of QSL reference evaluation (the layer-6 `replay` facade, the
CLI, or a test harness evaluating a checked package)
**I want** every evaluation of a checked declaration to return the kernel
evaluation outcome or a family-owned evaluation result that carries its own
code or undefined reason, together with the evaluation's location and loss
records
**So that** I can tell a kernel result (completed, undefined, refused,
incomplete) from a family that ran and refused or found no value, and from a
broken evaluator invariant, by matching a closed type rather than by reading
a message, and so that no evaluation path ends in a panic.

## Context

ADR-013 O-16 decides three outcome families with one owner each, and places
the S6a evaluation outcome in a QSL layer-3 `check`-core type,
`FamilyOutcome { Evaluated(kernel::Outcome), FamilyEvaluated(FamilyResult) }`,
beside T-4's `InternalFault`. S6a returns it inside an `Evaluation` that also
carries the evaluation's location and loss records. A family that does not
evaluate natively (`Relation`) never enters S6a. The kernel
`Refusal` in `quire-exact` carries kernel causes only (ADR-013 T-6), so a
refusal or undefined result whose cause is a QSL family concept rides in
`FamilyEvaluated`, not inside the kernel outcome. Every refusal a caller receives maps to
a catalog code, and F `diagnostic` maps that code to an O-16 category
(ADR-013 O-17).

## Acceptance Examples (Illustrative)

### US-013-EX-1: A kernel outcome arrives unchanged

- **Given** a checked `Value` function that converts a binary64 argument to a
  bounded rational.
- **When** the caller evaluates it through S6a with a NaN argument.
- **Then** the caller receives `FamilyOutcome::Evaluated` holding the kernel
  `Outcome::Undefined(IeeeNotFinite)`, exactly as the kernel produced it.

### US-013-EX-2: A family that does not evaluate natively has no S6a entry

- **Given** a `Relation` declaration, which ADR-012 §3 does not evaluate
  natively.
- **When** the caller writes code that hands it to S6a.
- **Then** that code does not compile: S6a's input type has no `Relation`
  variant, so there is no runtime refusal to handle.

### US-013-EX-3: A bad argument is refused before evaluation

- **Given** a checked `Value` function with a population parameter.
- **When** the caller calls it with a population identity this evaluation
  never recorded.
- **Then** `CheckedPackage::call` returns `CallFailure::Input` and S6a never
  runs.

### US-013-EX-4: A broken invariant is a fault, not a refusal or a panic

- **Given** a checked package and a declaration identity that names no
  function in it, handed to S6a directly (for example by a test harness or
  a replay facade that resolved the wrong package).
- **When** S6a evaluates that identity.
- **Then** the caller receives `Err(InternalFault)` naming the stage and the
  invariant, category `internal-failure`.

### US-013-EX-5: A family refusal found during evaluation keeps its own code

- **Given** a postcondition that reads `pre(allInstances<A>(p))` over a
  population admitted with no pre anchor.
- **When** the caller evaluates it through S6a.
- **Then** the caller receives `FamilyOutcome::FamilyEvaluated` holding a
  family refusal with catalog code `wrong_snapshot`/`wrong-anchor`, category
  `refusal`, distinct from a kernel refusal.

## Priority and Risk (Informative)

Priority: High. Until the family outcome exists, QSL's own copy of the kernel
`Outcome`/`Refusal` in `src/value/outcome.rs` is the only place its
non-kernel refusal causes can live, so that copy cannot be replaced by
`quire_exact::{Outcome, Refusal}`.

## Traceability (Informative)

- [FR-090](../functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md)
