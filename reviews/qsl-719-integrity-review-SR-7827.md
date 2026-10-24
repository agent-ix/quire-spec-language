---
id: SR-7827
title: "QSL-719 integrity review — closure outcome contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@34ff94663898614f8c6fc969eefe2caef45aa460 initial review; disposition rounds 2-4 through spec=e944de53912ebb41f6d068541520ea7e39180d40 and companion code PR #763=2d952659451988b230b05b7d89796ac112e0b5b4; FR-084 admission/AC-2, FR-090 AC-21, TC-227, spec/tests.md TC-227 row"
review_set: subset
---

## Summary

Integrity review checked obligation atomicity, observable outcomes, criteria, and criterion-to-test trace. The two-owner contract is consistent, but the object-closure-missing public-boundary branch lacks an acceptance criterion and test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Object-closure incomplete mapping lacks public-boundary acceptance evidence | FR-084-AC-2, FR-090-AC-21, TC-227 |

## Review notes

FR-084 owns the feature trigger; FR-090 owns the S6a result representation. Their statements do not conflict: the model checker produces typed incompleteness, and the public family result maps it to the existing Undefined arm. The acceptance criterion and TC-227 need a second case where object closure is absent, because the criterion currently specifies only missing subtype closure.

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | Query-level object incompleteness conflicts with admission's no-binding rule | FR-084, FR-090-AC-21, TC-227 |
| FND-003 | medium | The new public-boundary criterion is untagged in the computed matrix | FR-090-AC-21, TC-227 |

FR-084 admission says it yields unknown-closure and no binding when object closure cannot be established. The new FR-090 criterion and TC-227 step 2 require a public evaluation query over that state and an incomplete-closure family outcome. Without a specified admitted state that can satisfy both statements, the criterion is unreachable; clarify the distinction or reconcile the two requirements.

FND-003: the exact current `quire matrix --scope . --format tsv` output classifies FR-090-AC-21 as `untagged`. The TC artifact relationship and index row do not establish the test-function trace required by the computed matrix.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 9b217e4d0525fcfea0dbf7f12897a81ce6b528ec — FR-090-AC-21 and TC-227 now add the object-closure-missing case. |

### Disposition evidence

FND-001 after excerpt: `Given an allInstances<T> query evaluated through CheckedPackage::evaluate when either object closure is not established or object closure is established but effective subtype closure is missing, the result is Ok(e) with e.outcome exactly FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)); cause.undefined_record() has reason incomplete-closure, names closure object or subtype respectively, and names queried type T; for subtype closure it additionally names every missing effective subtype. Neither case exposes a partial collection. FamilyResult and FamilyOutcome retain exactly their two variants.`

### Disposition round 3 — reviewed spec head `b0651f2837a02a9d7ccaf46e00c54211045d69a8`

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | rejected | Object-closure failure yields no binding and is outside the S6a query contract. |
| FND-002 | fixed | b0651f2837a02a9d7ccaf46e00c54211045d69a8 — FR-084 and FR-090 now distinguish admission graph closure from per-T subtype coverage. |
| FND-003 | still-open | No source-test trace tag for FR-090-AC-21 was present at that head. |

### Disposition round 4 — spec head `e944de53912ebb41f6d068541520ea7e39180d40`; companion code head `1a231e322ef595e0787da97528214efd82c74345`

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | 1a231e322ef595e0787da97528214efd82c74345 — public evaluator source test is tagged to FR-090-AC-21 and asserts its typed incomplete result. |

The code-candidate binder and admission-valid fixture are described in round 4 of SR-7826. This disposition verifies the criterion-to-test evidence exists on the exact companion PR head; it does not claim that the companion code PR is merged or that its tests were executed.

### Round 5 recheck

Rechecked prior integrity dispositions at spec head `e944de53912ebb41f6d068541520ea7e39180d40` and code head `2d952659451988b230b05b7d89796ac112e0b5b4`. FND-001 remains rejected because object-closure failure yields no binding and is not an S6a query. FND-002 remains fixed by the explicit separation of admission-time graph closure and per-`T` subtype coverage. FND-003 remains fixed: the public evaluator test carries the FR-090-AC-21 trace tag, and now asserts `closure=subtype` as required by TC-227. No new integrity finding; no build/test run.
