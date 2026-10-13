---
id: TC-1005
title: "A normalization malformed declaration retains the artifact and position of the refused node's source origin"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: verifies
---
# TC-1005: A normalization malformed declaration retains the artifact and position of the refused node's source origin

## Description

Verify that each normalization `malformed-declaration` carries the source artifact and position of the node it names. Scope: FR-056-AC-33.

## Test Procedure

1. Read the TC-235 K2 package with every node carrying a source origin at its own line.
2. For each of the following mutations, normalize and compare the single refusal: a record value type with identity fields; an event type with identity fields; an event whose occurrence field resolves to a non-Timestamp scalar; an operation declared by an event type; a state machine with no transitions or no operations; a state machine transition that emits a non-event; a process with no identity fields and no process supertype, and one whose supertype is a state machine; a relationship whose source end, and one whose target end, is not an object type.
3. Declare a population whose second member names a record value type, an event type, a scalar, a persistence interface or a namespace, and one whose member names nothing.
4. Repeat one mutation of step 2 with every node carrying a `generated` origin.
5. Normalize a package whose model selection has an empty version, and one with a declaration key whose package is empty, and read a document that is not JSON.

## Expected Results

- Each refusal of step 2 is `invalid_model_binding`/`malformed-declaration` at the refused node (the operation, not the event type, for an operation an event type declares) with the typed member path, the source artifact and the source line and column of that node.
- Step 3 refuses the wrong-meaning member `invalid_model_binding`/`malformed-declaration` at the population node, path `PopulationDefinition.members[1]`, with the population's artifact and position, and the empty member `missing_declaration`.
- Step 4 leaves the artifact and span absent.
- Step 5 refuses each with `invalid_model_binding`/`malformed-declaration` and no artifact or span: the empty selection and the empty key have no node, and the package document is refused at node `$`.
- Assertions compare typed code, cause, node, member path, artifact and position, never detail text.
