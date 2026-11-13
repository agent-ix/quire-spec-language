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
  - target: ix://agent-ix/quire-spec-language/FR-496
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
associates it with the operation's `DeclarationKey` and sets `has_body`,
[FR-302](./FR-302-link-dispatch-during-unit-compile.md) checks every body,
[FR-094](./FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
builds a body clause function for every operation with `has_body`, and
[FR-494](./FR-494-emit-producer-dispatch-decisions-into-checked-packages.md)
carries a body identity inside a dispatch candidate. None of them makes a
checked package say, for every operation, whether it has an authored body and
which checked body it is. A backend consumer (CG, through quire-driver, for a
Kani proof subject; QSL-1899) has no producer-stated body to bind to the
operation and would have to rebuild one. FR-302, FR-094 and FR-103 are
widened to every body in the same change as this requirement.

This requirement owns the QSL producer side: that `check` builds and checks the
body of every operation with `has_body`, and that emission writes one record per
operation carrying that body or its explicit absence. It follows the
[FR-496](./FR-496-emit-model-operation-effects-into-checked-packages.md)
precedent: the record is a top-level checked-package member, the
`operation_bodies` member, whose wire shape, closed row schema and reader joins
QSpec FR-322 owns. QSpec FR-322-AC-288 closes `checked_term_data`, so the
member is not a `checked_term_data` array. This requirement names the content
QSL emits; it fixes no wire spelling beyond the member name, and the member
cannot be emitted until QSpec FR-322 admits it (Dependencies). The
quire-driver typed read and CG's subject emission are owned in their own
repositories.

## Inputs

- Each operation member declared by a `model` node of form `object_type`,
  `process` or `state_machine` in the selected documents, with its
  `DeclarationKey` and `has_body` (FR-103).
- Each retained unit `operation-body` (FR-102) resolved to such an operation.

## Outputs

- A checked package that carries `operation_bodies` with one row per operation
  member.
- The existing typed refusal when a body fails its check.

## Behavior

- An **operation** in this requirement is an operation member as FR-496 keys
  it: the declaring node, the selected document and the original member name
  the declaring node itself declares. A subtype that only inherits an operation
  has no row; a subtype that redefines it and declares its own member has its
  own row, keyed by its own declaring node.
- The emitter SHALL emit one row `{document, declaring, operation, body}` for
  every operation FR-496 emits an `operation_effects` row for, with the same
  `document`, `declaring` and `operation` values, so the two members join on
  one key. It SHALL sort the rows by `(document, declaring, operation)` as
  FR-496 does, independent of source declaration order, and an operation
  reached through two model aliases of one document has one row.
- While an operation has `has_body` true, `check` SHALL build the FR-094 body
  clause function for it and check the body as FR-302 does (parameter count and
  result type), whether or not any dispatched call reaches the operation, and
  the emitter SHALL set the row's `body` to a reference to that function node.
  A body that fails the check refuses the compile as FR-302 states, with no
  package. Because every body is checked, a unit whose uncalled body contains
  a dispatched call refuses `ill_typed`/`operator-ineligible` at that body
  (FR-302), where it compiled before this requirement; the body check has no
  other new refusal.
- While an operation declares no result type, an `operation-body` resolved to
  it SHALL refuse the compile `ill_typed`/`type-mismatch`, located at the body,
  with no package, as for a body whose type differs from the declared result
  (FR-302): FR-094 builds a body function only against a declared result, so
  such an operation has no body function and no row can reference one. The
  emitter SHALL NOT withhold `operation_bodies`, emit a partial member, or
  write `no_body` for it, since `no_body` states that the operation has no
  authored body.
- While an operation has `has_body` false, the emitter SHALL set the row's
  `body` to the typed absence `no_body`. It SHALL NOT omit the row, write a
  default or empty body, or substitute the body of another operation.
- When an operation is a candidate of an FR-494 dispatch table, the emitter SHALL give the row's body reference the same checked identity as that candidate's operation-body identity.
- The emitter SHALL source every field from checked producer data. It SHALL
  NOT rebuild a body from source text, names, declaration order, graph
  traversal or an operation catalog. Every body of an admitted operation is
  built and checked, or the compile refuses, so the emitter always has the body
  or its absence for every operation and SHALL NOT emit a partial member.
- The emitter SHALL write each row so that the row and the graph node it names
  give a reader the operation identity and the body expression with its source
  provenance, with no other input. The reader's typed read belongs to
  quire-driver.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-576-AC-1 | A unit with `A.size` having an `operation-body` and a dispatched call that reaches it emits an `operation_bodies` row for `A.size` whose `body` references the FR-094 body function node; reading that node yields the authored expression with its source provenance. | Test |
| FR-576-AC-2 | Operation `A.empty` has a body and no call in the unit reaches it: the package still holds its body function node and a row referencing it. The same source with a wrong-typed `A.empty` body, and again with a body whose parameter count differs from `A.empty`'s signature, each refuses `ill_typed`/`type-mismatch` at that body and emits no package. The existing traced test for FR-302-AC-8 (`unit_compile_checks_a_bodys_arity_and_type_against_its_operation`) calls the operation from a `pre` clause, so a test for the uncalled wrong-type and uncalled wrong-arity cases is missing and the implementer writes it. The body function node's id is equal in the package where a dispatched call reaches `A.empty` and the package where none does. | Test |
| FR-576-AC-3 | An operation without an `operation-body` has exactly one row whose `body` is `no_body`, and the package holds no body function node, default value or empty body for it. | Test |
| FR-576-AC-4 | Types `A` and `B` each declare `size`, `A` with a body and `B` without: rows `(doc, A, size)` and `(doc, B, size)` are separate, the first with a body reference and the second `no_body`; permuting declaration order emits an equal member. | Test |
| FR-576-AC-5 | Subtype `S` of `A` inherits `A.size` and declares nothing: the member has no row `(doc, S, size)`. Subtype `R` of `A` redefines `size` with its own body: it has its own row `(doc, R, size)` naming `R`'s body function node, distinct from `A`'s. | Test |
| FR-576-AC-6 | A document selected through two model aliases in one unit emits one row per operation of that document, not one per alias. | Test |
| FR-576-AC-7 | For an operation that is a candidate in an FR-494 dispatch table, the row's body reference equals that candidate's operation-body identity; for each operation the member's `(document, declaring, operation)` set equals the `operation_effects` row set of the same package. | Test |
| FR-576-AC-8 | A unit whose uncalled `A.empty` body contains a dispatched call refuses `ill_typed`/`operator-ineligible` located at that body and emits no package; the same unit with the call removed compiles. | Test |
| FR-576-AC-9 | Each row, together with the graph node it names, yields the operation identity and the body expression with source provenance by lookup alone: for the AC-1 package, resolving the row's `body` in the package graph returns the authored body for the row's operation, and for a `no_body` row no node lookup is needed. | Test |
| FR-576-AC-10 | Operation `A.touch` declares no result type and has an `operation-body`, with or without a dispatched call reaching it: the compile refuses `ill_typed`/`type-mismatch` located at that body and emits no package, so `operation_bodies` is never withheld or partial for it; the same unit with the body removed emits a `no_body` row for `A.touch`, and with a declared result and a matching body emits a row referencing the body function. | Test (TC-1011) |

## Dependencies

- **Upstream:** [FR-094](./FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
  builds the body clause function, and this requirement extends the operations it
  is built for to every operation with `has_body`; [FR-302](./FR-302-link-dispatch-during-unit-compile.md)
  states the body check this requirement requires for every body;
  [FR-103](./FR-103-admit-model-operations-and-frames-on-the-spine.md) sets
  `has_body`; [FR-494](./FR-494-emit-producer-dispatch-decisions-into-checked-packages.md)
  states the candidate body identity the row equals;
  [FR-496](./FR-496-emit-model-operation-effects-into-checked-packages.md) is
  the carrier precedent and the key. QSpec FR-322 must admit the
  `operation_bodies` member before this requirement is implemented (STD-660
  ticket for the carrier, blocking QSL-1899).
- **Downstream:** the quire-driver typed read of the member (quire-driver
  repository) and CG FR-015-AC-151 subject emission consume it; neither
  reconstructs a body.
