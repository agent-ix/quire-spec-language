---
id: FR-576
title: "Emit the operation body into checked packages for backend consumers"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-012
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-102
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-494
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-576: Emit the operation body into checked packages for backend consumers

## Description

QSpec FR-151 states that "a body is supplied by an `operation-body`
declaration". [FR-102](./FR-102-build-state-clause-forms.md) retains that
declaration's expression tree, [FR-103](./FR-103-admit-model-operations-and-frames-on-the-spine.md)
associates it with the operation's `DeclarationKey` and sets `has_body`, and
[FR-302](./FR-302-link-dispatch-during-unit-compile.md) checks it when dispatch
linking first reads it. [FR-094](./FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
builds a body clause function for a dispatch-eligible operation, and
[FR-494](./FR-494-emit-producer-dispatch-decisions-into-checked-packages.md)
carries a body identity inside a dispatch candidate. No requirement states that
a checked package exposes, for an operation occurrence, whether it has an
authored body and which checked body it is. A backend consumer (CG, through
quire-driver, for a Kani proof subject; QSL-1899) therefore has no
producer-stated body to bind to the operation and would have to rebuild one.

This requirement owns the QSL producer side: the emitted checked-package record
that exposes each operation's authored body, or its explicit absence. It owns no
wire shape (QSpec FR-322 does) and no consumer read (the quire-driver typed
read and CG's subject emission do).

## Inputs

- Each admitted model operation with its `DeclarationKey` and `has_body`
  (FR-103), and its retained, checked unit `operation-body` (FR-102, FR-302).
- The body clause function node `check` built for the operation (FR-094).

## Outputs

- For every operation occurrence of a checked package, one record in
  `checked_term_data` that is either a body reference or a typed absence.
- The existing typed refusal when emission cannot produce a complete record.

## Behavior

- For every admitted operation occurrence whose `has_body` is true, the emitter
  SHALL write one body record carrying the operation's checked identity and the
  identity of the checked body function node (FR-094 clause kind `"body"`),
  so the body expression is read from the checked package's own graph.
- For every admitted operation occurrence whose `has_body` is false, the
  emitter SHALL write one record whose body member is the typed absence
  `no_body`. It SHALL NOT omit the record, write a default or empty body, or
  substitute the body of another operation.
- A body record SHALL be keyed by the operation occurrence, so two operations
  with equal member spellings on different owners, or one operation reached
  through two model aliases, keep separate records.
- The emitter SHALL source every field from the checked producer data. It SHALL
  NOT rebuild a body from source text, source names, declaration order, graph
  traversal or an operation catalog, and SHALL preserve an unavailable body as
  unavailable rather than fabricate it. A body record SHALL NOT change any
  semantic graph node or node identity.
- The checked package SHALL let quire-driver read the record through its typed
  checked-package read with no CG-side or driver-side reconstruction of the
  body or of its association to the operation.
- A body that fails its FR-302 check refuses the compile before emission; the
  emitter SHALL NOT write a record for it.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-576-AC-1 | Compiling a unit with a model operation `A.size` and one `operation-body` for it emits one body record for the `A.size` occurrence naming the checked body function node (clause kind `"body"`); reading that node from the emitted package yields the authored expression with its source provenance. | Test |
| FR-576-AC-2 | In the same package, an operation `A.empty` with no `operation-body` emits exactly one record whose body member is the typed absence `no_body`; no body function node, default value or empty body is emitted for it. | Test |
| FR-576-AC-3 | Types `A` and `B` each declare `size` with equal spelling, `A` with a body and `B` without: the emitted package holds two separately keyed records, `A.size` with a body reference and `B.size` with `no_body`; permuting declaration order changes neither record. | Test |
| FR-576-AC-4 | Emitting the package from AC-1 with the body records removed leaves every semantic graph node and node identity equal to the package emitted with them. | Test |
| FR-576-AC-5 | A package emitted from the AC-1 and AC-2 source is read through the quire-driver typed checked-package read and the read returns, per operation, the body reference or `no_body` with no step that parses source or walks the graph to find a body. | Test |
| FR-576-AC-6 | Emitting the AC-1 source twice yields byte-identical body records, and re-emitting after renaming a clause function's lookup label leaves the record identities equal. | Test |
| FR-576-AC-7 | A unit whose `operation-body` has the wrong result type refuses `ill_typed`/`type-mismatch` at the body and emits no package and no body record. | Test |

## Dependencies

- **Upstream:** [FR-094](./FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
  builds the body clause function; [FR-103](./FR-103-admit-model-operations-and-frames-on-the-spine.md)
  sets `has_body`; [FR-302](./FR-302-link-dispatch-during-unit-compile.md)
  checks the body; QSpec FR-322 owns the `checked_term_data` wire carrier and
  must admit the body record before this requirement is implemented.
- **Downstream:** the quire-driver typed read of the body record (quire-driver
  repository) and CG FR-015-AC-151 subject emission consume it. Neither
  reconstructs a body.
