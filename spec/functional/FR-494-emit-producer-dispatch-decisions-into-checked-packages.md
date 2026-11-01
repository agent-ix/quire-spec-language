---
id: FR-494
title: "Emit producer dispatch decisions into checked packages"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-083
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-150
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-322
    type: depends_on
---
# FR-494: Emit producer dispatch decisions into checked packages

## Description

When S3 links a dispatched call's operation family, the QSL package emitter
currently has the checked package graph but does not retain the linked
`DispatchTable` decision in the package's producer-captured data. A later
typed reader therefore cannot expose the exact producer decision to a
consumer. [FR-083](./FR-083-resolve-unique-most-specific-dispatch.md) owns
candidate selection; [FR-302](./FR-302-link-dispatch-during-unit-compile.md)
owns linking tables for calls; this requirement owns carrying those completed
decisions through QSL emission.

The emitter SHALL write each linked dispatch decision to the
`checked_term_data.dispatch_tables` carrier defined by QSpec
[FR-322](ix://agent-ix/quire-specification/FR-322). It SHALL copy the
application and occurrence identity, selected-operation witness, receiver
context, candidate order, each candidate's FR-150 selection key, operation
body identity and effective-precondition body identity from the same checked
producer decision. The carrier is additional checked-package data; it does
not change semantic graph nodes or their identities.

## Inputs

- The successfully linked dispatch tables and selected-operation identities
  produced by S3 for a checked package (FR-302).
- The emitted application nodes and their checked-package occurrences.
- The exact body and effective-precondition function identities associated
  with each linked candidate (FR-094).

## Outputs

- A checked package whose `checked_term_data.dispatch_tables` contains the
  producer decisions required by QSpec FR-322, or the existing typed
  compilation refusal when emission cannot produce a complete decision.

## Behavior

- For every dispatched application occurrence with a linked table, the
  emitter SHALL emit one row joined to that exact local application node and
  occurrence. It SHALL preserve the selected operation and receiver context
  resolved by S3.
- Within each row, the emitter SHALL preserve the candidate sequence held by
  the linked `DispatchTable`. Candidate order is semantic; emission SHALL NOT
  sort candidates by name, selection key, declaration order or graph order.
- Each candidate SHALL carry the exact FR-150 `EffectiveId` used to select it,
  its checked operation-body identity, and its effective-precondition body
  identity, or `null` when that effective precondition has no body. For an
  inherited effective precondition, this is the effective function produced
  by checking, not a reconstruction from the candidate's authored clause.
- Across rows, the emitter SHALL use QSpec FR-322's canonical ordering by
  application structural identity and occurrence role and ordinal. The
  emitter SHALL keep rows for different occurrences separately keyed, even
  when they call operations with equal spellings, and retain each row's own
  operation, context and candidate data.
- The emitter SHALL source all row fields from checked identities and the
  linked `DispatchTable`/`DispatchCandidate` decisions. It SHALL NOT rebuild,
  infer or substitute a decision from source names, source or declaration
  order, display names, checked graph traversal, operation catalogs, or node
  paths. The QSL emitter SHALL preserve a decision that is unavailable from
  the checked producer data as unavailable and SHALL NOT fabricate it.
- QSL emission SHALL neither perform the IR reader's typed join nor claim
  reader admission. QSpec FR-322 owns wire shape and package validation; the
  consumer exposes a decision only through IR-1214's typed read.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-494-AC-1 | Compiling a unit with a dispatched call whose linked table has at least two candidates emits one `checked_term_data.dispatch_tables` row for the call's exact application occurrence. The row preserves the selected-operation witness and receiver context; its candidate sequence equals the linked `DispatchTable` sequence element for element, including each candidate's FR-150 selection key, operation-body identity, and effective-precondition body identity (including a null effective precondition). The emitted row uses the application and occurrence identities in the checked package and source map. | Test (TC-1282) |
| FR-494-AC-2 | Given two dispatched application occurrences that call operations with the same member spelling but have distinct application identities or receiver contexts, emission produces two independently keyed rows in QSpec FR-322 canonical row order. Each row retains only its own selected-operation witness, receiver context, candidate selection keys, body identities and effective-precondition identities. Permuting source declaration order does not pair either row by spelling or alter either row's checked identities. | Test (TC-1283) |
| FR-494-AC-3 | When candidate names and graph traversal order deliberately disagree with the linked `DispatchTable` sequence and identities, the emitted row still equals the captured producer decision in candidate order, selection keys, body identities and effective-precondition identities. | Test (TC-1282) |
| FR-494-AC-4 | Adding dispatch rows leaves the semantic graph nodes and node identities equal to those emitted for the same checked package without the producer-captured dispatch carrier; the carrier is the only added representation of these decisions. | Test (TC-1282) |

## Dependencies

- **Upstream:** [FR-083](./FR-083-resolve-unique-most-specific-dispatch.md)
  selects candidates; [FR-302](./FR-302-link-dispatch-during-unit-compile.md)
  associates linked tables with dispatched calls; [FR-094](./FR-094-key-model-owned-reference-population-and-quantity-nodes.md)
  defines the model-owned clause-function identities. QSpec FR-150 defines
  candidate selection identity, and QSpec FR-322 defines the
  `checked_term_data` wire carrier and its joins.
- **Downstream:** IR-1214 exposes retained decisions only through its bounded
  typed dispatch read. Consumers SHALL use that read and SHALL NOT reconstruct
  a table from names or the semantic graph.
