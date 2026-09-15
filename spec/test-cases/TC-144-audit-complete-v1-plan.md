---
id: TC-144
title: "Audit the complete-V1 Agent-A delivery plan"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-055
    type: verifies
  - target: ix://agent-ix/quire-spec-language/IT-011
    type: references
  - target: ix://agent-ix/quire-specification/TM-009
    type: references
---
# TC-144: Audit the complete-V1 Agent-A delivery plan

## Description

Audit the QSL repository-local adoption of the frozen complete-V1 Agent-A lane,
including its capability allocation, downstream test ownership, dependency
order, existing evidence state and delivery constraints.

## Test Procedure

Compile and run `tests/complete_v1_plan.rs`. The test includes Plan-013, the
accepted QSpec Agent-A projection fixture and each task file at compile time,
parses every `V1-*` allocation row, rejects a duplicate or missing row, compares
every requirement/ticket/test/qualification tuple exactly, derives the only
permitted primary ticket for every capability, and checks the serial predecessor
edge on every downstream task.
Inspect the changed-file set for the non-code constraints that cannot be proved
from the plan text alone.

## Expected Results

The Rust audit observes exactly 83 unique capabilities with one central
requirement, primary ticket, real central TestCase and QSL #123 qualification
owner per row. Ticket totals and the nine-task chain match the accepted central
manifest and all FR-055 acceptance criteria. The frozen revision,
evidence-state counts, Rust-only/local-build
constraints and protected resource path remain explicit; changed files contain
no `resources/native-v1` or hosted-workflow modification.
