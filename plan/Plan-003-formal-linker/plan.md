---
id: Plan-003
title: "Implement the reviewed native formal linker"
type: Plan
status: active
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-013
    type: references
---
# Plan-003: Native formal linker

## Scope and readiness

Implement the exact formal-environment linking API reviewed at 8dc6b48
(SR-046–053 over c78792a). Owner assignment keeps all work serial in A's
isolated compiler worktree. This is the next executable LC02 stage, not a
replacement for the full native state workflow.

## Dependencies and critical path

The requirement DAG is in SR-049; task frontmatter owns execution edges.
Task-005 implements the resolver and real public-API controls. Task-006 reviews
and lands the qualified result. The implementation is the larger serial step;
no parallel agent or external reader prerequisite exists.

## Test plan and gates

TM-003 is the single test inventory. This plan executes TC-020–024 and
TC-030–034; TC-025–029 remain FR-006's subsequent static-judgment work.

Gate after Task-005: exact owner/source identities, missing/stale/ambiguous/atomic
controls, lexical scopes, unsupported mappings and resource boundaries pass
through the real public API. Existing parser and selected private Rust audit
tests also pass after the IR/serde/Rust 1.98.1 dependency changes.

Gate after Task-006: actual Rust/code review, scoped Quire validation/coverage,
gap report and exact private PR evidence. No hosted dispatch. Remaining typing,
reference/population semantics, evaluation, backend and extraction work stays
open under the full assignment.
