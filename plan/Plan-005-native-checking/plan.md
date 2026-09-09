---
id: Plan-005
title: "Native model and guarded clause checking"
type: Plan
status: active
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-006
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
---
# Plan-005: Native model and guarded clause checking

## Requirements Summary

Implement the reviewed native-state-model/1 admission and link profile, then
native contextual typing and guarded definedness through the existing IR prover.
Preserve all five FR-006 reference/operation judgments and exact source/model/
authored-clause identity. Requirements are reviewed at ceccabb; all eight actual
QUOIN review artifacts and validation are committed at 3cdeb59.

This is a static-checking milestone in the full Agent A assignment. Runtime
population validation, reference truth/cost, qualified backend and Quire
integration remain required downstream work.

## Dependency Graph

Existing FR-013/014 and the pinned IR API enable model admission; admitted native
models enable clause checking. Qualification/handoff follows both. The graph in
SR-069 is acyclic; the task frontmatter owns its execution edges. NFR-005 governs
all production and qualification. No external reader/#54 gate exists.

## Test Plan

TM-003 is authoritative for criteria and execution status. Model cases qualify
the independent source-derived Rust producer, native roles, complete artifact
identity, source loci, old/new linkage and lowered/hard limits. Checker cases
exercise actual reference/operation rules, numeric proof, inference, observations,
authored bindings, lexical evaluation order, proof expansion and runtime input
requirements. TC-053 independently enumerates Boolean assignments for a bounded
guard family; no native or IR evaluator supplies its oracle.

Write meaningful public API tests first and record missing-API failure before
implementation. Setup must succeed before testing a refusal. Tests use imported
bare single-line trace attributes. Planned statuses advance only after actual
runs. Preserve unsupported/incomplete outcomes and exact failure loci.

## Execution and Quality Gates

The critical path is model tests/admission/linkage, checker tests/implementation,
then complete qualification and review. One Agent A track executes serially with
no additional agents. Each stage must satisfy its task deliverables before the
next consumes it; implementation-discovered contract changes reopen specify and
the affected reviews.

Use existing worktree caches, explicit target directories, nice 10, one Cargo
job and one test thread. Inspect competing builds and memory pressure before
heavy work; await each command's actual exit before starting another phase.
Hosted CI stays workflow_dispatch-only and is not dispatched.

Required final checks are the README build/test/style/audit commands, actual
agent-skills/code-review and rust-review, and non-semantic gap reconciliation.
Record commands, revisions, limitations and evidence. A reviewable ready PR is
the handoff deliverable; broader goal completion is not inferred from its merge.

## Task Mapping

See task frontmatter for ownership, dependencies and exact test traces. This
bundle has one model task, one checker task and one qualification/handoff task.

