---
id: SR-13628
title: "Gap analysis — QSL-1381/1382/1383 checked successors"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; QSL-1381 FR-043/044, QSL-1382 FR-045, QSL-1383 FR-046-AC-5; full origin/main...HEAD diff at 57497065073dade8d1715a7b698f5a377d8c8b71, including all changed Rust, FR, TC and review artifacts"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-043
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-044
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-046
    type: reviews
---

## Summary

Manually checked the retained ticket acceptance criteria against the checked-package implementation and all added trace tags because the Quoin matrix had not completed at review time. The new checked route is exercised, but the evidence does not establish all retained FR-043/044/045 successors, and TC-127 does not assert the selected checked query identity.

## Verdict

**FAIL** — retained criteria remain untagged or lack discriminating checked-route assertions.

## Coverage

- QSL-1381 / FR-043: checked tests cover representative profiles, formula operators, support, open/incomplete outcomes, deadlines, history, and progress binding. No successor tags cover FR-043-AC-10, AC-12, AC-13, or AC-14; AC-4's normalization test does not establish its full zero-width/nonzero lower-bound, strong-previous, and duality cases. FR-044 tags omit AC-4 and the cases do not establish whole-execution origin identity isolation under AC-8. TC-122/123 descriptions still define only the legacy emitted-artifact route; new checked-route tests are tagged to those existing IDs without adding checked-route successor test-case scope.
- QSL-1382 / FR-045: TC-125 tags cover AC-1, AC-2, AC-3, AC-5, and AC-6, but no test carries AC-4. The dimension-accumulation example is tagged only AC-2/3. The AC-6 test only refuses an out-of-range clause index; it does not assert checked selection identity/preservation or the retained profile/activation alongside classification.
- QSL-1383 / FR-046-AC-5: the new test reaches checked Map and Filter and verifies the typed unsupported cause and collection result type; the result shape produces no value or charge. It does not assert which checked query node was selected, so it cannot detect loss or ambiguity of that identity. The ordinary clause evaluator is not changed by this boundary.
- Reverse-gap review: the changed production path is reachable from checked packages; no stub or permanently generic refusal was found in the added temporal evaluator. Legacy TC-122/123 and TC-125 tests are not treated as evidence for the new checked route by themselves.
- Plan completion: not assessed (no implementation plan supplied).
- Quoin matrix command: quoin 0.29.1, quoin matrix --repo . --json, ran for approximately 3 minutes without stdout or stderr and was interrupted (exit 130). The quire matrix --scope . --format tsv fallback ran 30 seconds, emitted module duplicate/inline-schema warnings to stderr and no matrix rows, then was interrupted (exit 130). Coverage conclusions are manual AC-to-code/test checks. Semantic review was not run; this was an implementation gap pass, and the explicit checked-route and ticket acceptance criteria were compared directly.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | [QSL-1381] The checked successor suite does not map all retained FR-043/044 obligations to named checked-route tests. FR-043-AC-10/12/13/14 and FR-044-AC-4 lack trace tags; FR-043-AC-4's single normalization loop does not test the criterion's boundary/duality cases, and FR-044-AC-8 does not exercise distinct whole-execution origin identities. The new cases are tagged to TC-122/123 whose published procedures remain legacy emitted-artifact tests, with no checked-route successor procedure added. | tests/it/checked_temporal_successors.rs:143 |
| FND-002 | high | [QSL-1382] FR-045-AC-4 has no checked-route trace tag, while the only multiple-dimension assertion is tagged AC-2/3. FR-045-AC-6's checked test proves only that an invalid index is refused; it does not assert retained selection identity/profile/activation or distinguish the supported and unsupported classification evidence required by the ticket. | tests/it/checked_temporal_successors.rs:361 |
| FND-003 | medium | [QSL-1383] TC-127 proves a typed unsupported result for Map and Filter collection types, but neither the test nor result carries/asserts the selected checked node identity. Two same-typed query nodes therefore leave the representation-selection obligation unobservable. | qsl-replay/tests/it/query_proof.rs:59 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Added distinct checked-route successor cases TC-1277/1278 and preserved TC-122/123 as legacy emitted-artifact controls. |
| FND-002 | fixed | Added checked-route AC-4 dimension accumulation and retained identity/profile/activation assertions. |
| FND-003 | fixed | Added clause NodeKey and exact query NodeId to the unsupported result and tested same-typed Map candidates. |
| FND-004 | still-open | TC-1277 maps these ACs, but its executable tests do not exercise the named subcases and the procedure overstates its executable evidence. |
| FND-005 | still-open | The AC-1 checked test uses unequal capture amounts and does not assert a mismatch diagnostic naming both instance identities. |

## Disposition pass 1

The three original gap-analysis findings are fixed in the named fix commit. Two additional checked-route oracle gaps remain and keep the PR non-mergeable.

### Fix evidence

- FND-001: TC-1277/1278 explicitly describe S1-S4 CheckedPackage execution and state that TC-122/123 remain legacy emitted-artifact procedures. Every retained FR-043/044 AC is now trace-tagged to a named checked successor.
- FND-002: TC-1279 and its tests now trace FR-045-AC-4 and assert both unmatched dimensions, checked declaration identity, profile, and activation.
- FND-003: TC-1280 exercises two same-typed Map nodes and asserts both the containing clause NodeKey and selected query NodeId.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | [QSL-1381] TC-1277 maps retained ACs but its executable checked-route tests do not cover required subcases: order refusal for release/since/triggered (AC-5), the specified temporal negation examples (AC-6), an omitted interior history interval (AC-9), a conflicting completeness revision (AC-12), and result identity changes for clock/parameter premises (AC-13). TC-1277's procedure claims the interior-gap case, but no checked test removes an interior position. | tests/it/checked_temporal_successors.rs:257, tests/it/checked_temporal_successors.rs:744, spec/test-cases/TC-1277-evaluate-checked-package-temporal-obligations.md:28 |
| FND-005 | medium | [QSL-1381] The checked FR-044-AC-1 test supplies capture values 1, 9 and 5 and asserts different truths; it does not use equal captured amounts or assert the required mismatch report naming both instance identities, so equal-value identity confusion could pass. | tests/it/checked_temporal_successors.rs:1328 |

### New finding dispositions

Both findings remain open pending checked-route vectors and oracles for the listed obligations.
