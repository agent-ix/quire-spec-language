---
id: SR-2026111405325203
title: "spec review of PR2702 FR-573 serving and replay, ADR-036 handoff"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md, spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md, spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md, spec/test-cases/TC-1413-protocol-analysis-results-settle-with-fr-360-labels-and-uncertified.md, spec/test-cases/TC-155-keep-admission-backend-independent.md; PR2702 5d32b7e0f4cc3d33ce7d17af10881b37d8a62c14"
review_set: subset
---

## Summary

Ticket: QSL-1897. Spec review of the spec hunks in the diff, covering consistency, integrity and soundness. The new FR-573 "Serving" and "FR-360 Scope" sections, the FR-574 Outputs change, the ADR-036 open-item notes and the TL-387 handoff section, and the TC-155 step 9 are consistent with the code and with QSpec FR-241 and FR-360. The FR-360 Scope wording is stated, but it is carried by QSL-1898 and is not assumed. The PA-24 carriage deferral is a lead decision and is not flagged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-573 Behavior sets the indistinguishable-histories replay check as "equal observations of the role, two different actions, each enabled at its history's end". That check does not establish the stated divergence (PA-18 (i): disjoint not-lost action sets), so the requirement mandates an arm that accepts non-refutations. Example: one history used twice, with two actions both enabled at its end. The bullet should require each cross action to be disabled or losing at the other end within the bounds. The code follows the text; see code-review FND-001. | spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md:62-75 |
| FND-002 | medium | The PR revised FR-573 Behavior: a realizability refutation now replays over the `qsl-eval` arena, "which a replay source carries no member for". But FR-573-AC-2 still requires each realizability refutation to replay "through `ReplaySource::ProtocolAnalysis`". TC-1413 step 2's expected result ("after a passing `ReplaySource::ProtocolAnalysis` walk") and ADR-036 PA-20 ("The V1 replay arm is `ReplaySource::ProtocolAnalysis`") say the same, for both kinds. As written, AC-2 cannot pass for realizability. Reword AC-2, TC-1413 step 2 and PA-20 to name the realizability arena arm. | spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md:125; spec/test-cases/TC-1413-protocol-analysis-results-settle-with-fr-360-labels-and-uncertified.md:41-42; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:153 |
| FND-003 | low | The second replay bullet ("re-walks the stated paths with the structure's successor relation and recomputes the actions' shapes, the role's basis at the choice and the diverging layer") describes the projection arm only, but does not say so. It now follows a bullet that gives realizability a different arm, so it reads as if it covers both. Prefix it with "For a projection". | spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md:76-81 |

## Scope and evidence

- FR-573 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): settlement map, replay arm, Serving, FR-360 Scope. (examined)
- FR-573-AC-2 (spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md): refuted only after replay through ReplaySource::ProtocolAnalysis. (examined)
- FR-574 (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): Outputs, carriage on the analyze result, open on QSL-1898. (examined)
- ADR-036 (spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md): PA-20, open items 1 and 3, TL-387 handoff. (examined)
- TC-1413 (spec/test-cases/TC-1413-protocol-analysis-results-settle-with-fr-360-labels-and-uncertified.md): description and step 2. (examined)
- TC-155 (spec/test-cases/TC-155-keep-admission-backend-independent.md): step 9 and its expected result. (examined)
- QSpec FR-241 (quire-specification spec/objects/protocol/FR-241-assessment-execution-disposition.md): execution `unsupported` member. (context_only)
- QSpec FR-360 (quire-specification spec/objects/temporal/FR-360-infinite-trace-result-disposition.md): label-axis combinations, FR-360-AC-4. (context_only)

## Verdict

Fix FND-001, together with the code change, and FND-002 before merge. FND-003 is a one-word clarification. The rest of the spec change is accurate, including the TL-387 handoff note, apart from the absent-artifact case raised in code-review FND-005.
