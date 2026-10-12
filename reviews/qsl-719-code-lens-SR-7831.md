---
id: SR-7831
title: "QSL-719 code-review lens — spec-only contract alignment"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@34ff94663898614f8c6fc969eefe2caef45aa460 initial review; disposition rounds 2-4 at spec=e944de53912ebb41f6d068541520ea7e39180d40 and companion code PR #763=2d952659451988b230b05b7d89796ac112e0b5b4; changed spec files and TC-227 trace"
review_set: subset
---

## Summary

The diff changes specifications only; no production code or executable test implementation is modified. Code-review's duplication, integrity, spec-code-faithfulness, and code-test-alignment lenses found no independent code defect in this spec-only change; the uncovered public object-closure case is captured in the spec coverage findings.

## Verdict

**PASS** — no code-review-specific defect in the changed files; this does not override the conditional spec-review findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No code-review-specific findings (placeholder) | - |

## Review notes

Reviewed exact diff paths: `spec/functional/FR-084-admit-closed-populations-and-resolve-lookup.md`, `spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md`, `spec/test-cases/TC-227-allinstances-requires-both-closures.md`, and `spec/tests.md`. No code or tests were added in this PR, so language-specific implementation review and build/test execution are inapplicable. No new second source of the result contract was identified: FR-084 defines the trigger, FR-090 defines the S6a mapping, and TC-227 exercises the boundary.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | Object-closure query criterion conflicts with admission's no-binding outcome | FR-084, FR-090-AC-21, TC-227 |
| FND-003 | medium | No source test is bound to the new public-boundary acceptance criterion | FR-090-AC-21, TC-227 |

The updated contract requires S6a evaluation of a population whose object closure is not established. FR-084 admission instead returns unknown-closure and no binding in that condition. Since CheckedPackage evaluation requires the admitted binding, the new scenario has no reachable setup under the stated contract.

FND-003: Quire's computed matrix reports the newly added FR-090-AC-21 as `untagged`. TC-227's spec relationship and test-index row are present, but neither is a source test trace tag, so there is no accepted code-test binding for this new criterion.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-002 | fixed | b0651f2837a02a9d7ccaf46e00c54211045d69a8 — the spec now limits the public query result to an admitted population with missing subtype coverage. |
| FND-003 | still-open | At b0651f2837a02a9d7ccaf46e00c54211045d69a8, source binder had not yet been verified. |

### Disposition round 4 — spec head `e944de53912ebb41f6d068541520ea7e39180d40`; companion code head `1a231e322ef595e0787da97528214efd82c74345`

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | 1a231e322ef595e0787da97528214efd82c74345 — exact source binder and valid admission fixture verified. |

`qsl-eval/tests/it/model_reference_queries.rs::tc_227_incomplete_subtype_closure_is_undefined_through_checked_evaluate` has `#[trace("TC-227", "FR-084-AC-2", "FR-090-AC-21")]` and asserts the public `CheckedPackage::evaluate` result fields. Its `subtype_incomplete_scenario` calls `admitted_binding`, whose helper accepts only `AdmissionOutcome::Admitted`; the closed population declares A and contains only A members, while B and C remain effective subtypes uncovered by the declaration. The companion `qsl-semantics/tests/it/model_population.rs::all_instances_requires_declared_subtype_closure` separately asserts admission success, typed incomplete closure, and complete-set order independence. No code build/test was run. The computed `quire matrix`/trace CLI exceeded 60 seconds and was interrupted; the source-level trace tag and matching test were verified directly.

### Disposition round 5 recheck

At companion head `2d952659451988b230b05b7d89796ac112e0b5b4`, the public test now also asserts `record.fields.get("closure") == Some("subtype")` (exact Rust assertion at lines 907-908), in addition to the previously checked reason, queried type, and full uncovered-subtype list. FND-002 remains fixed by the spec's reachable admitted-query contract; FND-003 remains fixed by the exact source trace binding. No additional code-review finding; no build/test run.
