---
id: SR-2164003
title: "spec-ears-analysis of FR-522 to FR-527 (quire-spec-language PR 2164)"
type: SpecReview
analysis: "ears-conformance"
scope: "agent-ix/quire-spec-language; spec/functional/FR-522..FR-527, FR-255, FR-327, FR-330, FR-499, FR-516, spec/test-cases/TC-1317..TC-1322, TC-837, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-523
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-524
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-525
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-526
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-527
    type: references
---

# spec-ears-analysis of FR-522 to FR-527 (quire-spec-language PR 2164)

## Summary

Tickets: QSL-1502, QSL-1503, QSL-1504. EARS pass over the Behavior statements of FR-522 to FR-527 at head 6e7d7d34fd6a030a7dcfa8eae0f4cd3013a5585f. Statements are mostly well-formed (trigger or state, one actor, a SHALL), with the exceptions below: statements with two or more SHALL clauses (FR-523 7 of 16, FR-524 12 of 22, FR-522 3 of 14, FR-526 1 of 11, FR-527 2 of 14) and process language (author inferences and recommendations written into the requirement body).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [QSL-1502] Seven of the sixteen Behavior statements carry two or more SHALL clauses (charge and refused-charge; use ceiling and preserve zero; stop declaration and carry no result; stop instance and name it; and others). One requirement statement states one obligation. | spec/functional/FR-523-count-temporal-evidence-work-on-thirteen-named-counters-and-keep-a-stopped-instance-apart-from-a-stopped-declaration.md:90 | wrong-requirement |
| FND-002 | medium | [QSL-1503] Twelve of the twenty-two Behavior statements carry two or more SHALL clauses (for example 'create the instance, and SHALL NOT create a second'; 'retain a capture ... and one instance's captures SHALL NOT satisfy another'); FR-526 has one more. Split each into one obligation per statement. | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md:111 | wrong-requirement |
| FND-003 | low | [QSL-1502] Three of the fourteen Behavior statements carry two or more SHALL clauses (member comparison and refusal dimension; order and first-difference report; no reuse and no rewrite). | spec/functional/FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md:121 | wrong-requirement |
| FND-004 | low | [QSL-1504] Two statements carry two obligations each: retain selection and constrain its construction; read formula() and fix connective direction. | spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md:110 | wrong-requirement |
| FND-005 | medium | [QSL-1503] Normative text carries the author's own caveat ('This reading ... is an inference that FR-524-AC-2 and AC-3 make testable'). A requirement states what the system does; the open question belongs on the ticket for the owner. FR-526 ('Both are inferences from the legacy value-taking API'), FR-527 ('The function's crate and names are an inference') and FR-525 ('The recommendation is the requirement author's inference, not a ruling'; 'The default of formula_depth ... is also an owner choice') do the same. | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md:61 | wrong-requirement |
| FND-006 | medium | [QSL-1503] Same defect as the FR-524 caveat: the description of why prev is a value and why input_identity exists is stated as an inference in the requirement body. State the behavior and the rationale; carry the question for the owner on the ticket. | spec/functional/FR-526-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md:54 | wrong-requirement |
| FND-007 | low | [QSL-1504] The requirement body says the function's crate and names are an inference; state the location and names as the requirement and keep the question on the ticket. | spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md:55 | wrong-requirement |
| FND-008 | low | [QSL-1503] The Open rows section embeds a Recommendation column and an owner-choice note on formula_depth; recommendations are not requirement content and belong on the tickets that need the ruling. | spec/functional/FR-525-map-each-legacy-temporal-evaluator-criterion-to-its-spine-successor-or-name-the-open-row.md:131 | wrong-requirement |

## Verdict

8 findings: 0 high, 4 medium, 4 low. Tickets: QSL-1502 2, QSL-1503 4, QSL-1504 2.
