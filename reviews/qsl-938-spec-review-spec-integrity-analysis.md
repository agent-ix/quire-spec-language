---
id: SR-13675
title: "integrity review of QSL-938 FR-395 and TC-1089 (PR #1147)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@29f11fe9b396165cbfd7fe9c52f50aec5074cbdc; spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md, spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md, spec/functional/FR-062-implement-checked-family-contract.md, spec/functional/FR-075-compute-candidates-from-registered-backends.md, spec/functional/FR-100-run-a-named-function-through-the-spine.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-395
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1089
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
# SR-13675: integrity review of QSL-938 FR-395 and TC-1089 (PR #1147)

## Summary

Ticket: QSL-938. Completeness, consistency, atomicity and cross-link
integrity of the diff. I checked every `ix://` target (FR-060, FR-062,
FR-075, FR-100, FR-394, ADR-011, FR-395): each resolves. The spec.md
`contains` edge and index row are present, and
`tools/check-index-completeness.sh` exits 0. I checked open PRs (#1146,
#1142, #1141, #1139, #1137, #1136, #1131, #1128, #995, #723, #706, #703,
#700, #689, #686, #683, #681, #679, #674, #673, #667) for FR-395, TC-1089,
FR-062-AC-14 and FR-075-AC-17, and for edits to FR-062, FR-075 and FR-100.
None collides. No status wording.

Examined: every prose cross-reference in FR-395 and TC-1089, TC-1089's
relationships and scope, the Outputs/Terms consistency.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1089 step 6 checks "the `post Q using V on M::A::remove` unit of TC-198 L07 (FR-104)". TC-198 is a registry backend-absence test with no L07, and neither `L07` nor `M::A::remove` appears anywhere else under `spec/` or in any test. FR-062-AC-14's operation-contract clause ("A state clause's or an operation frame's `operation-contract` record has no owner") names no fixture either. Name a real FR-104 fixture (a clause with a frame) in FR-062-AC-14 and TC-1089. | spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md:31-32 |
| FND-002 | medium | FR-062-AC-14 and FR-075-AC-17 both name TC-1089 as their verification. But TC-1089's relationships are only `verifies FR-395`, and its Scope is "FR-395-AC-1 to FR-395-AC-10". Add `verifies` edges to FR-062 and FR-075 and list both ACs in Scope, so the matrix and a reader can trace them. | spec/test-cases/TC-1089-the-replay-facade-associates-the-selected-call-s-requirement-items.md:1-15 |
| FND-003 | medium | `CallRequirements` holds "the selected call as received (the function name and the arguments, in the declared parameter order)". FR-100's `Call.arguments` are "one per parameter, in any order", so "as received" and "in the declared parameter order" disagree when the caller's order differs. One implementer echoes the `Vec` unchanged and another reorders it. Pick one, and say whether the echoed values are the received values or the admitted ones. | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:67-68 |
| FND-004 | low | "Each is an existing `RunRefusal` arm and FR-272 cause". In this repo FR-272 is "Fail on a second definition of a canonical type". The cause catalog other QSL specs cite is QSpec FR-272. Write "QSpec FR-272". | spec/functional/FR-395-associate-the-selected-call-s-requirement-items-through-the-replay-facade.md:95 |

## Verdict

Not mergeable on integrity grounds until FND-001 and FND-002 are fixed.
The verifying TC cites a fixture that does not exist, and two ACs trace to
a TC that does not declare them. Cross-links otherwise resolve, the index
is complete, and no open PR collides with FR-395, TC-1089, FR-062-AC-14 or
FR-075-AC-17.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | still-open | Correction: the L07 fixture is real. It is QSpec TC-198 (quire-specification spec/test-cases/TC-198-closed-model-lookup.md line 65: F1 plus `A/remove`, `post Q using V on M::A::remove`), and qsl-semantics/qsl-eval tests cite it. My 'appears nowhere' searched only this repo and was wrong. What remains: FR-062-AC-14 and TC-1089 steps 6 write bare 'TC-198 L07', which in this repo resolves to the local TC-198 (registry backend absence, no L07). Write 'QSpec TC-198 L07', as other specs write 'QSpec FR-272'. |
| FND-002 | fixed | 29f11fe9b: relationships: verifies FR-395; verifies FR-062; verifies FR-075. Scope: FR-395-AC-1 to FR-395-AC-12, FR-062-AC-14 and FR-075-AC-17. |
| FND-003 | fixed | 29f11fe9b: - **Selected call**: the function name and the argument list the entry received, as FR-100's `Call` carries them. The function is one identifier (FR-100 Inputs). The facade stores the arguments as received. The identity and equality of a call is its function name and its set of (parameter name, value) pairs, so the order in which the arguments arrived (FR-100 lets them arrive in any order) does not change it. |
| FND-004 | fixed | 29f11fe9b: - **Typed refusals.** The facade SHALL refuse with an existing `RunRefusal` arm, FR-100's stage, code and category and QSpec FR-272's cause, and SHALL return no `CallRequirements`, in these cases: - the source, a model, a library or a dependency does not compile, so there is no checked graph: `Compile` (FR-100); - the function name is empty, is not an identifier, has more than one segment, or names no function of the checked package: `MissingDeclaration`, `missing_declaration`; - the named function's declared result is not Boolean, integer or Reference: `UnsupportedResult` (FR-100); - an argum |
