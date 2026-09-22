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
**I want** every evaluation of a checked declaration to return either the
kernel evaluation outcome or a typed family refusal that carries a catalog
code
**So that** I can tell a kernel result (completed, undefined, refused,
incomplete) from a family that does not evaluate natively, and from a broken
evaluator invariant, by matching a closed type rather than by reading a
message, and so that no evaluation path ends in a panic.

## Context

ADR-013 O-16 decides three outcome families with one owner each, and places
the S6a evaluation result in a QSL layer-3 `check`-core type,
`FamilyOutcome { Evaluated(kernel::Outcome), Refused(FamilyRefusal) }`, beside
T-4's `InternalFault`. The kernel `Refusal` in `quire-exact` carries kernel
causes only (ADR-013 T-6), so a refusal whose cause is a QSL family concept
cannot ride inside the kernel outcome. Every refusal a caller receives maps to
a catalog code, and F `diagnostic` maps that code to an O-16 category
(ADR-013 O-17).

## Acceptance Examples (Illustrative)

### US-013-EX-1: A kernel outcome arrives unchanged

- **Given** a checked `Value` function whose body divides by a zero argument.
- **When** the caller evaluates it through S6a.
- **Then** the caller receives `FamilyOutcome::Evaluated` holding the kernel
  `Outcome::Undefined(DivisionByZero)`, exactly as the kernel produced it.

### US-013-EX-2: A family that sits out evaluation refuses with a code

- **Given** a `Relation` declaration, which ADR-012 §3 does not evaluate
  natively.
- **When** the caller asks S6a to evaluate it.
- **Then** the caller receives `FamilyOutcome::Refused` with cause
  `FamilyNotNativelyEvaluable`, its catalog code, and category `refusal`.

### US-013-EX-3: A broken invariant is a fault, not a refusal or a panic

- **Given** an evaluation environment whose arguments an earlier call already
  consumed.
- **When** S6a evaluates against it.
- **Then** the caller receives `Err(InternalFault)` naming the stage and the
  invariant, category `internal-failure`.

## Priority and Risk (Informative)

Priority: High. Until the family outcome exists, QSL's own copy of the kernel
`Outcome`/`Refusal` in `src/value/outcome.rs` is the only place its
non-kernel refusal causes can live, so that copy cannot be replaced by
`quire_exact::{Outcome, Refusal}`.

## Traceability (Informative)

- [FR-090](../functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md)
