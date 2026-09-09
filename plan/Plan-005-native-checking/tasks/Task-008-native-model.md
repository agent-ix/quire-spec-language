---
id: Task-008
title: "Implement and qualify source-bound native models"
type: Task
status: done
track: A
priority: P1
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-015
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-005
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-040
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-041
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-042
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-043
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-044
    type: verifies
  - target: ix://agent-ix/quire-spec-language/TC-045
    type: verifies
---
# Task-008: Implement and qualify source-bound native models

## Scope

Implement reviewed model admission and link_native by sharing the existing
linker. Qualify the source-derived Rust model producer before checker use.

## Subtasks

- [x] Write initial public API model/source tests and record missing-API failure.
- [x] Implement immutable model roles, admission and bounded artifact identity.
- [x] Complete model mutations/boundaries and write native link qualification tests.
- [x] Extend shared resolution for explicit native references and operations.
- [x] Execute TC-040–045 and preserve original linker compatibility.

## Deliverables

Rust model module, domain-specific Rust fixture producer, independent source
fixture, traced model/link tests and actual focused evidence.

## Notes

No runtime object IDs are enumerated in model metadata. Every unused declaration
and role is admitted and source-corresponded. No partial model/package escapes.

Admission and the repaired source-derived producer are implemented at d1168dd.
Five initial model tests pass through the real API. Native linkage is implemented
at 667bf07 and TC-044 is qualified by SR-084. Eight new link tests cover exact
selection, inventory conflicts, reference/operation/enum/parameter correspondence,
legacy compatibility and native-link limits. The default suite passed 69 tests;
the three selected private audits and required local gates passed.

Completed at 0cd679c with SR-085. Seventeen additional public-API tests complete
TC-040–043/045's role/carrier/operation mutations, model locus classes, artifact
mutation/permutation families and model-limit controls. Every adverse IR input
passes its actual constructors before native admission is judged. The complete
suite passed 86 tests and the three selected private audits passed. Required
local gates passed; exact logs are under reviews/data/native-checking/ with the
model-qualification prefix. TC-044 retains its SR-084 linkage evidence and ran
again in the full suite. Task-009 is now the next bounded work. Task-011 remains
complete; the full model/checker plan and runtime workflow remain unfinished.
