---
id: Task-013
title: "Validate finite snapshots and recorded invocations"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/Task-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/Task-009
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-006
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-003
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-058
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-059
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-060
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-061
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-062
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-063
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-064
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-065
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-066
    type: verifies
---
# Task-013: Validate finite snapshots and recorded invocations

## Scope

Implement actual model-aware runtime validation over the landed CheckedPackage and immutable input artifacts. Return constructor-private ValidatedContext only after all required checks pass.

## Subtasks

- [x] Write real source-derived model/input tests before implementation; assert successful setup separately from the intended runtime refusal.
- [x] Build bounded exact inventory/model/population indexes and validate every supplied selected value, including skipped fields, under each native type and captured observation.
- [x] Resolve reference closure, required self/State inputs and invocation parameter/result correspondence without using unavailable data as absence.
- [x] Validate complete pre/post identity differences and immutable object/State-root frame permissions with separate storage equality.
- [x] Retain structured runtime paths, original native/model loci, deterministic complete diagnostics and honest partial reports; implement fresh budgets and deterministic cancellation/panic behavior.
- [x] Run generated shape/permutation/retry and isolated budget controls; repair any observed failures before admitting contexts to evaluation.

## Deliverables

Qualified validator, immutable validated context API, stable classified runtime diagnostics and frame/closure evidence. No predicate Boolean is produced here.

## Notes

Task-009 is done in Plan-005 and Task-012 is qualified at c8fa41f by SR-096.
The implementation dependencies are satisfied. Reuse exact native roles and
checked runtime obligations; do not infer a formal model from generated layouts
or create another type authority.

Qualified at 45ed1b4d11eb162ac6bab9d94e2dca6113545037 by SR-097.
All fifteen FR-007 criteria and NFR-006 validation metrics M-6..11 have
executed evidence: 35 validation tests, 166 ordinary tests plus one compile-fail
doctest, three private audits and strict local Rust gates passed. The review
retains real failures, fixes, traceability and catalog limitations. TC-061's
predicate/captured-read portion remains with FR-008/TC-071; validation completion
does not qualify reference execution or the full Plan-006/backend/Quire workflow.
