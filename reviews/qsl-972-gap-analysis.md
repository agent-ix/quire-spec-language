---
id: SR-12326
title: "Gap analysis of quire-spec-language PR #1215: frame_operation on OperationSite (QSL-972)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a59c5848f; FR-121 operation-selection bullets, FR-121-AC-23, FR-121-AC-24, TC-516; QSpec FR-323 frame_operation"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1215

## Summary

Plan completion: not assessed.

Sources: Linear QSL-972, FR-121 (this branch), TC-516, and QSpec main
FR-323 (`/replay/frame_operation`: `object` the `QualifiedName` of the type
that declares the operation, `operation` its `Identifier`). CG FR-024's
`frame_operation` text is not in the local CG checkout, so the CG side
was not read.

| Criterion | Test | Status |
| --- | --- | --- |
| FR-121-AC-23 (inherited op names the declaring `ConfigVersion`; anchor, frame, occurrence equal) | `call_site_names_the_declaring_object_of_an_inherited_operation` | covered; fails under the selected-type mutation |
| FR-121-AC-24 (redefined op names the redefining `Sub`; frames differ) | `call_site_names_the_redefining_type_for_a_redefined_operation` | covered; fails under the base-type mutation |
| FR-121 "no OperationSite without the pair" | the field is not optional; the one constructor sets it | covered by construction |
| FR-121 Fault for an unresolvable declaring object | none | not testable from valid input: supertypes are intra-package and come from the same table |

The ticket asks for "typed refusals if unresolvable". The code uses
`CallSiteRefusal::Fault`, and FR-121 now says so. That is correct: the case
can only happen if the checker's own tables disagree, and the FR-121
refusal list keeps `Fault` for broken invariants. A typed caller-facing
refusal would be dead code.

The existing `call_site_locates_an_operations_anchor_frame_and_clauses`
test also asserts `frame_operation` for the plain declared case.

## Verdict

No major gaps. Mergeable.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The ticket and QSpec FR-323 say the declaring object may be a process or a state machine. `call_site` can select only object types (`model_objects` holds `ObjectType` records only), so a process or state-machine operation returns `UnknownOperation`. That limit was there before this PR, and no clause can name such an operation today, so CG gets a refusal, not a wrong pair. It is a reach gap to note for CG IR-958 and the driver IR-672 if they expect process frames. | qsl-semantics/src/check/lowering/model.rs:549; qsl-replay/src/call_site.rs:575 |
| FND-002 | low | Same as the code review's FND-002: no multi-level-inheritance case. | qsl-replay/src/spine/clause/tests/call_site.rs |
