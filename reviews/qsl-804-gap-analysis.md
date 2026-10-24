---
id: SR-11745
title: "Gap analysis of quire-spec-language PR #905: FR-365 criteria to tests"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@90b36116b595fc690050926cbadfce42dd839a30; FR-365-AC-1..12, TC-1046 C01-C16, TC-1047 rows 1-7 and 4b, TC-1048 rows 1-23; qsl-semantics/tests/it/relationship_reachability.rs, qsl-package/src/emit/tests/reachability.rs, qsl-eval/tests/it/relationship_reachability.rs, qsl-forms/tests/it/value_forms.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #905

## Summary

Ticket: QSL-804. Planless. I mapped each FR-365 acceptance criterion to the
tagged tests in the PR diff by reading them; no build was run. Plan
completion: not assessed.

| AC | Tests (vector) |
| --- | --- |
| FR-365-AC-1 | C01-C05; TC-1048 row 8 |
| FR-365-AC-2 | C06, C07 |
| FR-365-AC-3 | C08-C13, C16; C14 in qsl-forms |
| FR-365-AC-4 | C15; `the_context_population_is_not_added_twice` |
| FR-365-AC-5 | TC-1047 rows 1-3 |
| FR-365-AC-6 | TC-1047 rows 4, 4b, 5-7 |
| FR-365-AC-7 | TC-1048 rows 1-8, 19 |
| FR-365-AC-8 | TC-1048 rows 9, 10, 23 |
| FR-365-AC-9 | TC-1048 rows 1, 2, 6, 9, 11-14 |
| FR-365-AC-10 | TC-1048 rows 7, 16 |
| FR-365-AC-11 | TC-1048 rows 20-22 |
| FR-365-AC-12 | TC-1048 rows 15, 17, 18 |

Every criterion has at least one test that asserts real values from the TC
rows. Production code in the diff has no FR-365 behaviour without a test.

## Verdict

No coverage gap blocks. The two defects in test intent are code-review
SR-11744 FND-002 (C06/C07 assert the form locus, not the third argument) and
FND-003 (C15 does not assert the family or prefix). They are recorded there and
not repeated here.

Recorded (medium): TC-1048 row 21 runs only one of its two neighbour orders,
so it does not show that the undecidable-first order decides.

Row 23's Warshall oracle is independent: a matrix closure with no search, and
the inverse role uses its transpose.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
