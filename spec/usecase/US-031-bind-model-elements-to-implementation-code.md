---
id: US-031
title: "Bind model elements to the implementation code a proof backend checks"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-305
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: exercises
  - target: ix://agent-ix/quire-spec-language/US-012
    type: references
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-031: Bind model elements to the implementation code a proof backend checks

## Story

**As a** model author whose operation contracts are proved against a Rust
implementation by Kani or Verus
**I want** to declare, in the specification, which Rust type, field,
collection and function implement each model object, population and
operation, and have the compiler check those bindings and carry them in the
checked package
**So that** every generated contract reads the implementation through one
authored mapping, a claim over an element I have not bound is refused by name
before any backend runs, and a change to a binding can never silently
reinterpret a contract generated earlier.

## Context

US-012 binds the model graph itself. This story starts from that bound graph
and relates it to code. The mapping is authored, never inferred: QSL checks
that each binding's model key exists and that its Rust names are well formed,
and leaves resolving those names against the code to the generator that emits
it. A specification may bind part of its model; only the items that reference
an unbound element are held back, and the rest go on to generation.

## Acceptance Examples (Illustrative)

### US-031-EX-1: Bind ConfigVersion to its implementation

- **Given** the ConfigVersion model and a relation binding the ConfigVersion
  object type to a Rust struct, its population to a collection, and
  `attemptUpdate` to a Rust function
- **When** the unit compiles
- **Then** the checked package carries the three bindings, and recompiling
  after changing one field binding gives a different `package_id`

### US-031-EX-2: An unbound element is refused by name

- **Given** a relation that binds the object type but not its population
- **When** the driver requests bindings for a claim over the population and a
  claim over the object's fields
- **Then** the first is refused, naming the population and its domain
  package, and the second goes on with its binding

### US-031-EX-3: A conflicting binding is refused

- **Given** two bindings for one object type
- **When** the unit compiles
- **Then** the compiler refuses, naming both bindings and the key

## Priority and Risk (Informative)

Priority: High. Proofs about implementation code are only as sound as the
mapping from the model to that code. A guessed or partially emitted mapping
would let a backend prove a contract about the wrong field.

## Dependencies (Contextual)

Upstream: the model graph of US-012 and QSpec FR-353's abstraction relation.
Downstream: CG's Kani and Verus generators, which read the relation from the
checked package.

## Traceability (Informative)

- [FR-304](../functional/FR-304-check-an-authored-abstraction-relation.md)
- [FR-305](../functional/FR-305-relate-a-frame-binding-to-its-operation-s-frame.md)
- [FR-306](../functional/FR-306-carry-the-abstraction-relation-in-the-checked-package.md)
- [FR-307](../functional/FR-307-export-the-bindings-each-item-references.md)
