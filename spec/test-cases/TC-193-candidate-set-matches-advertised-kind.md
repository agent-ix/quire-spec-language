---
id: TC-193
title: "Candidate set matches registered backends advertising the requested kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-193: Candidate set matches registered backends advertising the requested kind

## Description

Verify that the registry's candidate-set computation for an unnamed request
returns exactly the registered backends that advertise the item's capability
kind, and none that do not. Scope: FR-075-AC-1.

Catches an implementation that matches candidates by backend registration
presence alone (any registered backend is a candidate for any item,
ignoring `advertises`), and an implementation that matches by mode instead
of kind (a bounded-only registrant is silently excluded from a bounded item
because the matcher compares extent before it should).

## Test Procedure

1. Build a registry with two backends: Backend A advertising
   `value-validity`, Backend B advertising `operation-contract`.
2. Compute the candidate set for an item requiring `value-validity` with no
   named backend.
3. Compute the candidate set for an item requiring `operation-contract` with
   no named backend.
4. Build a second registry with one backend, Backend C, advertising
   `temporal-satisfaction`, and compute the candidate set for an item
   requiring `value-validity`.

## Expected Results

Step 2 returns a candidate set containing exactly Backend A. Step 3 returns
a candidate set containing exactly Backend B. Step 4 returns an empty
candidate set (Backend C does not advertise `value-validity`).
