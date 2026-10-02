---
id: SR-055
title: "Gap analysis of native linker qualification"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-003-formal-linker, spec/model-linking/tests.md, src/linking.rs and tests/linking.rs"
review_set: subset
evaluated_revision: "d44e97424ecfe42343edb869e9203275aca26db1"
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-003
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-003
    type: references
---

## Summary

The scoped linker is implemented and its ten cases have real tagged passing
tests. Plan-003's private review/handoff remains in progress at this revision,
and TM-003 also contains five unimplemented FR-006 static-typing cases.

## Verdict

FAIL for full Plan-003/TM-003 completion at the evaluated revision. This does
not invalidate the qualified FR-005/013 implementation milestone in SR-054.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Critical Task-006 is still in_progress: retained review and exact private PR delivery are not complete at this revision. | plan/Plan-003-formal-linker/tasks/Task-006-linker-qualification.md |
| FND-002 | high | TC-025–029 and FR-006's five criteria have no native checker backing. The full matrix cannot be marked complete. | spec/model-linking/tests.md; spec/functional/FR-006-check-defined-expressions.md |
| FND-003 | medium | Catalog/status extraction diagnostics constrain the global coverage rollup; no evidence is claimed for unmatched or uncatalogued methods. | data/native-linking/formalization-a-link-coverage.json |

## Coverage

Used the installed QUOIN gap-analysis skill, including target, plan, matrix,
reverse-gap and artifact steps. Reconciliation is actual quire coverage --scope .
--json with CLI 0.31.0 and engine 0.46.0, under the active spec-artifacts-process
traceability model. No grep fallback or zero-denominator pass is used.

Target: plan/Plan-003-formal-linker; spec root: spec/; matrix: TM-003 at
spec/model-linking/tests.md; identity: ix://agent-ix/quire-spec-language.
Source and tests are src/ and tests/. Tasks done: 1/2. FR-005: 5/5 backed;
FR-013: 6/6; TM-003: 10/15; FR-006: 0/5. Global rows: 78/114. The report
has zero status lies and zero untracked symbols. These are traceability counts,
not a percentage of the full requested workflow completed.

All ten new symbols are bound. Their real runtime results are retained in the
companion Rust review. Eighteen engine diagnostics include existing unmatched
status columns/sections/archetypes, uncatalogued historical NFR verification
methods and broadly classified properties. Six existing registry duplicates
also remain. No matched FR-005/013 row is inferred from those missing sections.
The known functional Coverage Status/Status mismatch is disclosed separately
from the explicit TC statuses and actual test runs.

Reverse discovery: seven new behavior groups inventoried, all owned by FR-005/
013's reviewed API. Zero untraced behavior groups, zero disguised source stubs
and zero test stubs in the changed implementation. Explicit Shape::Unknown and
unsupported reference/operation results represent the selected phase boundary;
they do not impersonate a complete type checker or state evaluator.

Optional intent/test/code semantic comparison was skipped at the owner's
direction. Actual code-review/Rust idiom checks were performed separately.

## Remaining work

Deliver the exact reviewed linker PR and finish Task-006's private handoff.
Then implement FR-006's concrete checking contract using existing IR judgment
capabilities and explicit source correspondence. Native reference/population
semantics, healthy/violating/refused evaluation, qualified backend lowering and
Quire integration remain required by the full assignment. Neither B adoption
of the separately merged specification PR8 nor closed IR #54 is a reason to
stop this available native work. No broader LC issue or goal is closed here.

## Handoff disposition

FND-001 is resolved for the reviewable-PR deliverable after 3924dbb was pushed,
compiler PR8 was made ready, and exact LC02/CO01 handoffs were posted. Both
Plan-003 tasks are now done for that bounded delivery. The original evaluated
revision and 1/2 rollup above remain historical evidence. FND-002/003 remain;
the full TM-003 verdict is still FAIL, with five FR-006 cases unimplemented.
