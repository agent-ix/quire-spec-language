---
id: SR-7826
title: "QSL-719 spec review — allInstances incomplete closure boundary"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@34ff94663898614f8c6fc969eefe2caef45aa460 initial review; disposition rounds 2-4 through spec=e944de53912ebb41f6d068541520ea7e39180d40 and companion code PR #763=2d952659451988b230b05b7d89796ac112e0b5b4; FR-084 admission/AC-2, FR-090 AC-21, TC-227, spec/tests.md TC-227 row"
review_set: subset
---

## Summary

Base requirements, criteria, trace links, and the test index were reviewed for QSL-719 at the frozen head. The contract follows the planner's FR-090 ownership ruling, but the object-closure-missing public evaluator case is not covered by FR-090-AC-21 or TC-227.

## Verdict

**CONDITIONAL** — one medium coverage gap remains for the object-closure branch.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Public evaluator coverage omits incomplete object closure | FR-090-AC-21, TC-227 |
| FND-002 | low | TC-227's prose scope omits its new FR-090 criterion | TC-227, spec/tests.md |

## Review notes

FR-090 retains exactly two `FamilyResult` and `FamilyOutcome` variants; FR-090 owns the public undefined-reason shape while FR-084 owns the allInstances trigger, consistent with the QSL-719 planner ruling. TC-227 now exercises the CheckedPackage::evaluate boundary for a missing subtype closure, and the index links FR-084-AC-2 plus FR-090-AC-21. Quire full-spec validation exited 0 (1560/1569 grammar-clean; warnings include a relevant `effective` terminology warning and other existing advisories). The computed matrix command did not complete within the review window; the specific frontmatter and index links were inspected directly.

## Findings detail

FND-001: FR-084 requires incomplete output when either object closure or subtype closure is absent, and FR-090 maps that incomplete result to `Undefined(incomplete-closure)`. However, FR-090-AC-21 and the new TC-227 evaluator-boundary assertion only exercise the case where object closure holds and subtype closure is missing. No criterion/test asserts the public S6a mapping when object closure itself is missing.

FND-002: TC-227 now verifies FR-090-AC-21 and its frontmatter plus `spec/tests.md` index both name that AC, but the test's Description still says `Scope: FR-084-AC-2` only. Update that scope statement to include FR-090-AC-21 so the human-readable scope agrees with the declared trace.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | high | The object-closure-missing evaluator case is unreachable under the admission contract | FR-084, FR-084-AC-1, FR-090-AC-21, TC-227 |
| FND-004 | medium | FR-090-AC-21 has no computed test-matrix binder | FR-090-AC-21, TC-227, spec/tests.md |

FR-084 says admission returns unknown-closure with no binding when object closure cannot be established. FR-090-AC-21 and TC-227 step 2 require evaluating that same population through `CheckedPackage::evaluate` as incomplete-closure, while explicitly avoiding admission refusal or unknown-closure. The contract does not explain how an admitted query exists in that condition; align the admission and query preconditions or distinguish the closure concepts.

FND-004: `quire matrix --scope . --format tsv` reports FR-090-AC-21 as `untagged`. TC-227's spec relationship and `spec/tests.md` index entry do not create a code-level trace binder in the computed matrix. Add the accepted source-test trace for the public evaluator case (the existing FR-084-AC-2 row is also untagged in this run).

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9b217e4d0525fcfea0dbf7f12897a81ce6b528ec — FR-090-AC-21 now covers object-closure-not-established and subtype-closure-missing cases. |
| FND-002 | fixed | 9b217e4d0525fcfea0dbf7f12897a81ce6b528ec — TC-227 scope now names FR-084-AC-2 and FR-090-AC-21. |

### Disposition evidence

FND-001 after excerpt: `Given an allInstances<T> query evaluated through CheckedPackage::evaluate when either object closure is not established or object closure is established but effective subtype closure is missing, the result is Ok(e) with e.outcome exactly FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)); cause.undefined_record() has reason incomplete-closure, names closure object or subtype respectively, and names queried type T; for subtype closure it additionally names every missing effective subtype. Neither case exposes a partial collection. FamilyResult and FamilyOutcome retain exactly their two variants.`

FND-002 after excerpt: `Verify that allInstances<T> returns a typed incomplete result naming the missing closure when either object closure or subtype closure for T does not hold, and returns the complete deduplicated set only when both closures hold, in canonical reference-key order independent of input order. Scope: FR-084-AC-2 and FR-090-AC-21.`

### Disposition round 3 — reviewed spec head `b0651f2837a02a9d7ccaf46e00c54211045d69a8`

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | rejected | Object closure failure prevents admission and is not an S6a query under FR-084/FR-090. |
| FND-003 | fixed | b0651f2837a02a9d7ccaf46e00c54211045d69a8 — FR-090-AC-21 defines the subtype-closure public result. |
| FND-004 | still-open | No source test binder was present at that head; matrix run did not complete. |

### Disposition round 4 — spec head `e944de53912ebb41f6d068541520ea7e39180d40`; companion code head `1a231e322ef595e0787da97528214efd82c74345`

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-004 | fixed | 1a231e322ef595e0787da97528214efd82c74345 — the public CheckedPackage::evaluate test carries the exact FR-090-AC-21 trace tag and asserts the incomplete-closure record. |
| FND-005 | still-open | TC-227 requires `closure=subtype` in the undefined record, but the exact public source binder does not assert that field. |

The fixed binder is `qsl-eval/tests/it/model_reference_queries.rs::tc_227_incomplete_subtype_closure_is_undefined_through_checked_evaluate`, tagged `TC-227`, `FR-084-AC-2`, and `FR-090-AC-21`. Its fixture uses a closed population declared for `model.A`, actual members `a1:A` and `a2:A`, and effective subtypes `model.B` and `model.C`; the admission helper requires `AdmissionOutcome::Admitted`. The test evaluates through `CheckedPackage::evaluate` and asserts `Undefined(incomplete-closure)`, queried type A, and both uncovered subtypes. The companion semantics binder supplies the complete-coverage/canonical-order control. No build or test execution was performed. `quire matrix` was interrupted after 60 seconds without output; direct trace tags and source were inspected instead.

## New findings (disposition pass 4)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | TC-227's public source binder does not assert the documented `closure=subtype` field | TC-227, FR-090-AC-21 |

TC-227 Expected Results specifies that `undefined_record()` has `closure` `subtype`. The named public binder asserts `reason`, `queried_type`, and `uncovered_subtypes`, but has no assertion for `record.fields.get("closure")`; the criterion and test case therefore allow this documented field to regress unnoticed. Add the missing assertion or remove that field from TC-227's expected result if it is intentionally non-normative.

### Disposition round 5 — spec head `e944de53912ebb41f6d068541520ea7e39180d40`; companion code head `2d952659451988b230b05b7d89796ac112e0b5b4`

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-005 | fixed | 2d952659451988b230b05b7d89796ac112e0b5b4 — the public binder now asserts the expected closure field. |

The exact source test now contains `assert_eq!(record.fields.get("closure"), Some(&"subtype".to_owned()));` before its existing queried-type and uncovered-subtype assertions. The preceding dispositions were rechecked: the scope statement includes FR-090-AC-21, the admission/query closure distinction remains consistent, and the public binder retains the exact trace tag and admission-valid fixture. No builds or tests were run.

Round 5 recheck across earlier artifacts: SR-7826 FND-001 remains rejected and FND-002 through FND-005 are fixed; SR-7827 FND-001 remains rejected and FND-002/FND-003 are fixed; SR-7831 FND-002/FND-003 are fixed; SR-7829 FND-002 is fixed; SR-7830 FND-001 is fixed; SR-7828 remains clean. Their previously resolved outcomes were revalidated against the unchanged spec head and the current companion code head; no additional findings were identified.
