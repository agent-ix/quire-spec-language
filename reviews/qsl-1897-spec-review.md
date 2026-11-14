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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-573-AC-5 and TC-1413's step 5 expected result still say a stopped run settles "with no artifact". FR-573 Serving now makes the stop the result's own `failed` outcome, carried with its key on the `ProtocolAnalysisArtifact`, and the table row's "No artifact" means no witness or counterexample (PA-14). Reword both to "no witness or counterexample". The ADR-036 TL-387 handoff note should also say that `protocol_analysis` is absent for a refusal or an engine failure. | spec/functional/FR-573-settle-protocol-analysis-results-with-the-fr-360-labels.md:138; spec/test-cases/TC-1413-protocol-analysis-results-settle-with-fr-360-labels-and-uncertified.md:52-53; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:224-240 |


## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | FR-571 Behavior, ADR-036 PA-18 and FR-573 now state the sole-mover rule: a pair at an end where another role or the environment can move is non-conclusive. No acceptance criterion states it, and TC-1411 has no step for it. So the engine test that backs it is tagged FR-571-AC-2 (gap-analysis FND-003). Add an FR-571 AC, for example "a pair at an end where another role or the environment can move settles `inconclusive` `unforced-refutation`, never `refuted`", and a TC-1411 step. | spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:115-123; spec/test-cases/TC-1411 |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | FR-571 Behavior ("a role never waits while other roles move"), FR-571-AC-7 ("A role that could only wait for an action not yet enabled is refuted when another state of its class needs a different action"), ADR-036 PA-16 ("may be made to move at any state ... it never waits") and PA-19 extend QSpec FR-175's strategy game. FR-175's environment "chooses admitted external events and channel outcomes", and "each role chooses enabled local actions". Neither thread scheduling nor forcing a role to act is the environment's there. So AC-7 requires refuting protocols that FR-175 calls realizable, such as the AC-7 test's own fixture (see code-review FND-012). QSL cannot redefine the QSpec game in its own requirements. Restate the waiting semantics so that a role whose committed action is not enabled waits, and refute only where waiting cannot help. Otherwise, raise the extension with QSpec (owner decision). | spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:120-128, 143; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:144, 147 |
| FND-007 | low | ADR-036 PA-18 and FR-571-AC-6 still say that in `bounded-static` "no history is avoidable" and "PA-18's two inconclusive cases do not arise in V1". Yet the same PA-18 sentence and FR-571 Behavior now make a history avoidable by a role's choice among its own enabled actions, and settle it `inconclusive`. Reword AC-6 and PA-18 to separate an owner's branch choice, which cannot arise, from a role's table choice among its own steps, which can and is `unforced`. | spec/functional/FR-571-decide-whether-local-strategies-realize-a-global-protocol.md:144; spec/decisions/ADR-036-protocol-local-projection-and-realizability-engine.md:146 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 16c26295e: FR-573 Behavior now requires the not-lost sets to be disjoint (PA-18 (i)): every action the role can take at both ends needs a carried continuation from one end, starting with that action and ending in a violation. ADR-036 PA-20 says the same. The "enabled at its own end" premise is still open as code-review FND-007. |
| FND-002 | fixed | FR-573-AC-2 now reads "through its arm: `ReplaySource::ProtocolAnalysis` for a projection, the arena walk for realizability". TC-1413 step 2 and ADR-036 PA-20 name both arms the same way, and PA-20 says the realizability arm has no replay source. |
| FND-003 | fixed | The second bullet now begins "For a projection, the replay arm re-walks the stated paths with the structure's successor relation". |
| FND-004 | fixed | 255b10aa2: the FR-573 table row and FR-573-AC-5 read "no witness or counterexample", as does TC-1413's step 5 expected result. The ADR-036 handoff note adds that `protocol_analysis` is absent for a refusal before analysis or an engine failure, and that a run stopped by a limit or the meter carries its result and key. |
| FND-005 | fixed | e6bdb2b33: FR-571-AC-7 and TC-1411 step 7 state the rule, and ADR-036 PA-16, PA-18, PA-19 and PA-20 and FR-573 match the code. What the rule says is disputed in FND-006. |
