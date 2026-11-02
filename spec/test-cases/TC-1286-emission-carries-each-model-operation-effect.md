---
id: TC-1286
title: "Emission carries each model operation's effect"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-496
    type: verifies
---
# TC-1286: Emission carries each model operation's effect

## Description

Verify that QSL emits the optional `operation_effects` member with one row per
operation of every object type, process and state machine in the selected
documents, each row equal to the operation's admitted frame lift, keyed and
sorted by `(document, declaring, operation)`, with provenance, and that an
unavailable effect omits the member. Scope: FR-496-AC-1 to FR-496-AC-5. The
reader's refusals are QSpec TC-977's and are not repeated here.

## Test Procedure

1. Compile the packages of QSpec `proposals/checked-package-v2/operation-effect-vectors.json`
   cases OEC-01 to OEC-07 by reference, emit the checked package, and compare
   `operation_effects` with the case's rows member by member.
2. Compile a package with a `process`, a `state_machine`, a `persistence_interface`,
   a relationship and a subtype that only inherits an operation, and list the
   rows.
3. Permute source declaration order, member order and frame entry order of the
   OEC-06 and OEC-07 packages and re-emit.
4. Compile a package whose producer cannot supply one operation's effect and
   emit it; compare the semantic graph nodes with the package that carries the
   member.

Tag the test `#[trace("FR-496-AC-1", "FR-496-AC-2", "FR-496-AC-3", "FR-496-AC-4", "FR-496-AC-5", "TC-1286")]`.
Vectors are referenced by QSpec case ID, never copied.

## Expected Results

- Step 1: every case emits exactly the rows it lists; no effect kind is dropped
  and the all-empty effect is a row, not an absent row; the same member name
  under two objects and documents is distinct rows.
- Step 2: the process and state machine each emit rows; the persistence
  interface, the relationship and the inheriting subtype emit none.
- Step 3: the emitted member is equal across permutations, sorted by key with
  canonical entry order.
- Step 4: the member is omitted, no other member differs, and semantic graph
  nodes and identities are equal with and without the member.
