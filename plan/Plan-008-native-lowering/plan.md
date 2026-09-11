---
id: Plan-008
title: "Native Boolean lowering and backend parity"
type: Plan
status: in_progress
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: references
  - target: ix://agent-ix/quire-spec-language/IT-008
    type: references
---
# Plan-008: Native Boolean lowering and backend parity

## Scope

LC04's first executable projection: existing native declarations and exact source
bindings feed the existing strict IR binder and Boolean backend. Full numeric,
object and compiled ConfigVersion acceptance remains LC04 work.

## Delivery

1. [Task-019](tasks/Task-019-lower-native-booleans.md): tests and bounded lowering.
2. [Task-020](tasks/Task-020-qualify-boolean-backend.md): actual generated parity,
   local gates, QUOIN spec/code/Rust/gap reviews at PR readiness, then delivery.
3. [Task-032](tasks/Task-032-integer-ir.md): bounded integer IR lowering and
   standalone export, independently of C's backend implementation.
4. [Task-033](tasks/Task-033-state-scalar-ir.md): concrete context-field projection
   and validated primitive inputs at their original observations.

Owner direction for the first proof of concept permits engineering delivery with
generated activation qualification deferred. Complete the scoped PR review and
land the working Boolean lowering; continue LC05 engineering independently.
FR-009-AC-5, IT-008-SC-04 and Task-020 remain open until the actual generated
activation counts pass. C owns the coverage-reader update; it is an assurance
dependency, not a prerequisite for further compiler implementation.

## Coordination

A owns compiler changes on `agent-a/lc04-native-projections`; C owns downstream
producer changes. Consume the existing backend's exact wire boundary in tests.
One Cargo phase at a time, nice 10, one job/test thread, locked/offline cached
builds. No additional agents or hosted CI. Optional semantic review is declined.

## Test Plan

[TM-006](../../spec/native-lowering/tests.md) maps the criteria. Unsupported
clauses must reach lowering after valid native checking. Tests cover identity,
complete populations, observation mapping, limits and real generated execution.

## Deferred follow-ups

A retains the two low findings carried forward by
[SR-115](../../reviews/26-09-09-lc04-hard-limit-remediation-rereview.md):

- SR-114 FND-002: qualify hosted-CI timing/caching before enabling cloud runs;
  retain manual dispatch and the ten-minute cost cap while testing stays local.
- SR-114 FND-003: automate dependency-inventory verification before distribution;
  the current 140-entry inventory was verified, but has no automated gate.
