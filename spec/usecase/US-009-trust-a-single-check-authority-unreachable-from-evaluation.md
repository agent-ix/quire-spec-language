---
id: US-009
title: "Trust a single check authority, unreachable from evaluation"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: exercises
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-009: Trust a single check authority, unreachable from evaluation

## Story

**As a** QSL contributor implementing or reviewing a semantic family's
checking or evaluation code
**I want** the checking half of `value::expression` and its evaluation half
in two modules that cannot reach into each other's private state, with the
checked-output types' constructors held only by the checking side
**So that** I never have to ask whether evaluation code could run a
definedness or typing decision on its own, or whether a value passed to
`CheckedPackage::call` could have skipped checking by some other
construction path, because the type system, not a convention, forecloses
both questions.

## Context

Before this ticket, `value::expression` holds checking and evaluation
together in one module: `check.rs`, `facts.rs`, `ir.rs` and `termination.rs`
implement typing, definedness and termination; `evaluate.rs` implements the
kernel evaluator; and `CheckedPackage`, the type that gates one from the
other, is defined in `mod.rs` with both a checking `impl` block and an
evaluation `impl` block reaching its private fields directly, because both
sides live in the same module tree. Nothing in the type system distinguishes
"this code checks" from "this code evaluates a value checking already
admitted" — the distinction exists only in which method a caller reaches for
and in the ADR's own prose. ADR-011 §7.3 M-5 requires the two to become
different modules at different layers (layer-3 `check`, S3, and layer-5
`value::expression`, S6a), with the checked-output types' constructors
private to the checking side, so that evaluation can only ever receive an
already-checked value through the same narrow accessor surface every other
stage boundary in this ADR already uses (§4).

## Acceptance Examples (Illustrative)

### US-009-EX-1: Evaluation cannot construct a checked value from scratch

- **Given** a value of type `CheckedPackage` I want to evaluate.
- **When** I try to build one directly, by naming its fields, from code
  outside `check`.
- **Then** the build fails to compile: the type's constructor is private to
  `check`, and the only way to obtain a `CheckedPackage` is `check`'s own
  checking entry point.

### US-009-EX-2: A checking-only change cannot silently touch evaluation, or vice versa

- **Given** a change to a definedness rule inside `check`.
- **When** I review the diff.
- **Then** the diff touches only files under `check`; no file under
  `value::expression`'s evaluation side changes as a side effect of a
  checking-only rule change, because the two no longer share private state
  in one module.

### US-009-EX-3: `CheckedPackage::call` stays the one evaluation entry, not a second type

- **Given** the split has landed.
- **When** I call `value::expression::CheckedPackage::call` the way existing
  callers already do.
- **Then** it still resolves to one type, defined once in `check` and
  reached through `value::expression`'s `pub use`, exactly as ADR-011 §4
  already establishes for the analogous S4 case — not a second,
  independently-defined `CheckedPackage` living in `value::expression`.

## Priority and Risk (Informative)

Priority: High. ADR-011 §4's typestate guarantee — "no public type is shared
between an unchecked and a checked object" — is only as strong as its
weakest boundary. Leaving `CheckedPackage`'s checking and evaluation halves
in one module with shared private-field access is exactly the kind of
boundary a later change can quietly widen: a convenience method added to
"just read a field directly since it's all one module anyway" that would be
a compile error across a real module boundary is invisible risk while both
halves still share one module.

## Traceability (Informative)

- [FR-068](../functional/FR-068-split-expression-checking-into-check-stage.md)
