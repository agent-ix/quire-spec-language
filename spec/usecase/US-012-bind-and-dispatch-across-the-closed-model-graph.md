---
id: US-012
title: "Bind and dispatch across the closed model graph"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-082
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-084
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-085
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-086
    type: exercises
  - target: ix://agent-ix/quire-spec-language/US-002
    type: references
  - target: "ix://agent-ix/quire-spec-language/StR-001"
    type: traces_to
---
# US-012: Bind and dispatch across the closed model graph

## Story

**As a** model author who declares object types with generalization,
subsetting, redefinition, closed populations, relationships and systems
structures
**I want** the compiler to bind those declarations into one immutable model
graph and resolve conformance, redefinition, closed lookup and dispatch from
it deterministically
**So that** every check, proof and generated artifact agrees on exactly which
declaration and which redefinition governs a call or a query, with no hidden
registry, display-text tiebreak or partially-populated result standing in for
a closure or ambiguity failure.

## Context

US-002 covers linking a clause to its selected domain model at the name-
resolution boundary. This story picks up after that boundary: once a domain
package's declarations are admitted ([FR-056](../functional/FR-056-admit-domain-package-model-declarations.md)),
the checker still has to bind them into a graph that answers, for one closed
package, "what is this declaration's original and effective identity,"
"does this subtype conform to that supertype," "which redefinition of this
operation applies to this receiver," "is this population's membership closed
enough to answer allInstances," "what does this relationship's other end
resolve to," and "is this endpoint a Port on a Part, and is this connection
well-formed." None of these questions has an open-world answer: each one
either resolves to one value over the closed declaration set the domain
package admits, or it refuses or reports incompleteness. A model author who
declares two same-titled but distinct object types, or an ambiguous
redefinition, needs the checker to tell them so rather than silently picking
one by source order or returning half an answer.

## Acceptance Examples (Illustrative)

### US-012-EX-1: Deterministic redefinition and dispatch

- **Given** an operation redefined along two independent subtype branches
  that both apply to a call's runtime receiver, with one branch a proper
  descendant of the other
- **When** the checker links the dispatch table for that call
- **Then** it selects the descendant branch's redefinition, and repeating the
  same build from the same declarations always selects the same one

### US-012-EX-2: Ambiguity refuses, it does not guess

- **Given** two redefinitions of the same operation that apply to the same
  receiver subtype and neither is a descendant of the other
- **When** the checker links the dispatch table
- **Then** it reports the ambiguity for that subtype and produces no dispatch
  table entry standing in for either candidate

### US-012-EX-3: Closed population lookup

- **Given** a population declared with a closed extent whose subtype coverage
  is complete for the requested type
- **When** the checker evaluates `allInstances<T>`
- **Then** it returns the complete, deduplicated set; if either closure were
  not established, it would report incompleteness rather than a partial set

## Priority and Risk (Informative)

Priority: High. This is the binding step every later proof, dispatch call and
generated-code correspondence claim depends on. A silent display-text
tiebreak, an ambient registry that lets one process run see another's
declarations, or a partial dispatch table on ambiguity would let two runs of
the same package disagree about which declaration governs, undermining every
downstream proof that assumes deterministic replay.

## Dependencies (Contextual)

Upstream: domain-package admission
([FR-056](../functional/FR-056-admit-domain-package-model-declarations.md)),
the shared model-graph vocabulary quire-specification owns (AD-006,
FR-150–153, interfaces I03–I05). Downstream: state and graph evaluation
([FR-047](../functional/FR-047-evaluate-finite-object-reference-graphs.md))
consumes the bound graph's identities and closed populations as its own
independently supplied runtime inputs.

## Traceability (Informative)

- [FR-081](../functional/FR-081-preserve-model-correspondence-and-declaration-identity.md)
- [FR-082](../functional/FR-082-resolve-conformance-subsetting-and-redefinition.md)
- [FR-083](../functional/FR-083-resolve-unique-most-specific-dispatch.md)
- [FR-084](../functional/FR-084-admit-closed-populations-and-resolve-lookup.md)
- [FR-085](../functional/FR-085-resolve-relationship-end-references.md)
- [FR-086](../functional/FR-086-bind-systems-model-references.md)
