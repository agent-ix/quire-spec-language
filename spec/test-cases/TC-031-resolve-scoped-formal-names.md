---
id: TC-031
title: "Resolve lexical and formal declaration occurrences"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: verifies
---

## Description

Resolve lexical and formal declaration occurrences. Integration/property controls, priority P1. Traces: FR-013-AC-3.
Planned before implementation; use the public link API and actual IR constructors.

## Test Procedure

Construct valid acyclic nested records, an optional record, a bounded collection, an enum and explicit values through IR constructors. Link native field chains, enum literals, let-bound records and quantifier variables. Exercise shadowing, an initializer outside its own new scope, sibling scope isolation, missing values/fields/variants and an ambiguous conditional receiver shape.

## Expected Results

Occurrences identify their exact formal owner or native binding-name span. Shadowing selects the nearest binding without leaking. Every missing or unresolvable field/name refuses atomically; successful linkage makes no typing/definedness judgment.

