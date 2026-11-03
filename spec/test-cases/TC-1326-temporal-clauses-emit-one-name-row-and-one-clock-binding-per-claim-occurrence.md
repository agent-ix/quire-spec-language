---
id: TC-1326
title: "Temporal clauses emit one name row and one clock binding per claim occurrence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-322
    type: verifies
---
# TC-1326: Temporal clauses emit one name row and one clock binding per claim occurrence

## Description

Verify that the emitter writes `temporal_clause_names` and `temporal_bindings` per `claim` occurrence of a shared clause node, sorted, and refuses an inconsistent clock selection.

Scope: FR-337-AC-16 to FR-337-AC-22.

## Test Procedure

1. Compile two structurally equal `eventually[0,1] holds(true)` clauses named `Alpha` and `Beta`, under infinite-trace and then under event-position false-extension with two distinct clock configurations, and under `quire.temporal.timestamped-event.finite-window/v1` with timestamped-event configurations.
2. Rename `Alpha` and recompile; compile a unit with no clause.
3. Compile the two clauses of step 1 with their declarations in reversed source order and compare both arrays with step 1's.
4. Compile with a clock selection removed, with an unused clock selection, and with a selection of a kind the profile does not determine.

Tag the tests `#[trace("TC-1326", "FR-337-AC-16", "FR-337-AC-17", "FR-337-AC-18", "FR-337-AC-19", "FR-337-AC-20", "FR-337-AC-21", "FR-337-AC-22", "FR-322-AC-343", "FR-322-AC-344", "FR-322-AC-361")]`.

## Expected Results

- Step 1: one node; two name rows at occurrences 0 and 1 with their names; under the clocked profile one binding per occurrence with its own configuration; under infinite-trace no `temporal_bindings` member; arrays sorted by (NodeId, occurrence).
- Step 2: only the renamed row changes, no NodeId or `package_id`; the clause-free unit omits `temporal_clause_names`.
- Step 3: both arrays are equal to step 1's, sorted by (NodeId, occurrence).
- Step 4: each refuses the compile and produces no package.
