---
id: SR-2164002
title: "spec-integrity-analysis of FR-522 to FR-527 (quire-spec-language PR 2164)"
type: SpecReview
analysis: "integrity"
scope: "agent-ix/quire-spec-language; spec/functional/FR-522..FR-527, FR-255, FR-327, FR-330, FR-499, FR-516, spec/test-cases/TC-1317..TC-1322, TC-837, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-499
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

# spec-integrity-analysis of FR-522 to FR-527 (quire-spec-language PR 2164)

## Summary

Tickets: QSL-1502, QSL-1503, QSL-1504. Integrity pass (atomicity and consistency) over FR-522 to FR-527 and TC-1317 to TC-1322 at head 6e7d7d34fd6a030a7dcfa8eae0f4cd3013a5585f. All TC rows map one to one to their FR's ACs and scopes (TC-1317 AC-1..15, TC-1318 AC-1..12, TC-1319 AC-1..16, TC-1320 AC-1..5, TC-1321 AC-1..10, TC-1322 AC-1..15) and TC-837 scope was extended to AC-8 and AC-9 with steps 5 and 6 whose expected values agree with the ACs. Findings are the compound criteria and the FR-525 gate criteria.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | [QSL-1502] One criterion asserts twelve counters each at exact usage and one below (24 claims); a failure does not say which counter. FR-516-AC-7 does the same over thirteen. Split per counter, or state the AC as a per-counter row set. | spec/functional/FR-523-count-temporal-evidence-work-on-thirteen-named-counters-and-keep-a-stopped-instance-apart-from-a-stopped-declaration.md:147 | wrong-requirement |
| FND-002 | low | [QSL-1502] AC-13 varies twelve members in one criterion; AC-6 (two precedence claims), AC-8 (empty charge log and a precedence claim) and AC-15 (distinct identities and in-process/read-back equality) also hold two claims each. | spec/functional/FR-522-bind-a-temporal-trace-to-the-checked-clause-s-profile-and-clock-and-report-the-authenticated-clock.md:172 | wrong-requirement |
| FND-003 | low | [QSL-1503] AC-9 asserts four fault inputs (WrongType, Stale, delivery anchor, capture anchor) with two naming rules, AC-8 four, AC-5 three origin claims and AC-15 five dispositions in one criterion each. | spec/functional/FR-524-create-one-temporal-instance-per-trigger-or-origin-with-immutable-captures-and-name-every-capture-fault.md:190 | wrong-requirement |
| FND-004 | low | [QSL-1504] AC-10 combines a compile-fail claim with a behavioral claim about surrounding; AC-2 combines two Unsupported claims with the four past-operator claims and AC-3 repeats the finite-window claim. | spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md:135 | wrong-requirement |
| FND-005 | low | [QSL-1503] The bullet and the next one ('a predecessor that is the result the request would produce (equal input_identity)') state the same refusal; one of them is redundant. | spec/functional/FR-526-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md:84 | wrong-requirement |
| FND-006 | medium | [QSL-1503] AC-4 and AC-5 are conditions on the head of a future change that deletes src/temporal, restate FR-499's gate, carry Verification 'Test' while TC-1320 says no test carries the tag because the check is not a Rust test, and cannot pass or fail in this repository now. State them as FR-499 gate rows with Inspection, or make them gate conditions of FR-499 only. | spec/functional/FR-525-map-each-legacy-temporal-evaluator-criterion-to-its-spine-successor-or-name-the-open-row.md:157 | wrong-requirement |

## Verdict

6 findings: 0 high, 2 medium, 4 low. Tickets: QSL-1502 2, QSL-1503 3, QSL-1504 1.
