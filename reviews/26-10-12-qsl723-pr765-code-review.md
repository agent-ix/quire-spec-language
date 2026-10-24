---
id: SR-7852
title: "QSL-723 PR 765 runtime vector review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@58a47ac35e160b3772401c928efc56ae1719565a; qsl-eval runtime vectors and evaluator setup; foundation witness; replay codec; observation admission; TC-235 semantics test delta"
review_set: subset
---

## Summary

Reviewed exact clean remote head against integration base 8d293ebf3b90878eda2703e717bd519273ea9b70. Code/Rust review and external QSpec TC-235/FR-208/FR-207 alignment performed; no builds run, per dispatch. Production changes span observation admission, evaluator clause setup, path subjects and replay serialization, beyond five added test selectors. Existing semantics regression assertions are also removed. Shared K2 is referenced through the existing common support module rather than copied.

## Verdict

FAIL. Several vectors observe manually constructed expectations rather than required runtime behavior; production snapshot/path defects and deleted regression coverage also remain. Findings must be fixed in PR 765. The open record/event and native-String dependencies preclude treating this rebased head as green.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | M13 never traverses observed_order or an invocation parameter through production invariant evaluation. It manually creates six Money values, evaluates a standalone non_negative function six times, attaches authored string paths, and sorts those strings with a hardcoded rank. Removing all reachable-value discovery, invariant dispatch, and runtime path generation would leave this test green. Exercise the actual reachable-value invariant boundary and assert its six evaluations and four emitted typed violation paths in canonical order. | qsl-eval/tests/it/tc235_business_meaning_runtime.rs:706; qsl-eval/tests/it/tc235_business_meaning_runtime.rs:750 |
| FND-002 | high | M22 constructs receiver_path and parameter_path with json! inside the test and compares their literal strings. No runtime or replay codec produces either path, so wrong binding roots, missing field steps, broken serialization or ordering all pass. Obtain paths from actual receiver and parameter reads, serialize through the production codec, and compare those outputs; retain the process-object receiver negative control required by M22. | qsl-eval/tests/it/tc235_business_meaning_runtime.rs:1162 |
| FND-003 | high | M14 models OrderPlaced as an ObjectType with a Boolean occurred_at, supplies a fabricated unresolved object reference, and expects CallFailure::Input. TC-235 M14 requires a bound and type-checked event invariant with Timestamp occurrence and no result when no event value is reached. An input refusal is a different outcome. Use the admitted event meaning and invariant selection, and assert the empty result set with no invented refusal or event evaluation. | qsl-eval/tests/it/tc235_business_meaning_runtime.rs:783; qsl-eval/tests/it/tc235_business_meaning_runtime.rs:832 |
| FND-004 | high | State-machine postcondition pre(self.field) selects the pre environment but retains the post receiver reference. New context references use observation.digest as universe identity, so pre and post references differ; ClauseSetup binds observations.self_object (post) and the evaluator never uses receiver_pre when switching to pre. Attribute lookup therefore cannot resolve the post reference in the pre closure. Add an admitted postcondition reading both current and pre context fields, and bind the proper context at each anchor. | qsl-semantics/src/model/observation/document.rs:1498; qsl-eval/src/value/expression/mod.rs:481; qsl-eval/src/value/expression/evaluate.rs:1425 |
| FND-005 | high | Actual member provenance remains Provenance::member(...object...), which always creates ValuePathSubject::Object. The new receiver_path accessor is a separate root and does not re-root member-read provenance; locatable state-machine receiver fields can therefore escape with an artificial object identity rather than Invocation/Receiver plus field steps. Integrate receiver binding identity into production trail generation and test emitted paths; a side-channel root assertion does not establish correct read locations. | qsl-eval/src/value/expression/evaluate.rs:2268; qsl-eval/src/value/expression/s6a/separation.rs:156; qsl-eval/src/value/expression/mod.rs:509 |
| FND-006 | high | The rebase deletes M01 process effective-type/object-universe and persistence no-universe controls, removes M07 NormalizeRecord charge assertions and typed source-locus helpers, and changes duplicate-step conflicting-binding coverage to a generic malformed-declaration expectation. This weakens or contradicts previously landed process contracts without any runtime-vector justification. Restore these assertions while extracting the shared K2 fixture. | qsl-semantics/tests/it/tc235_business_meanings.rs:161; qsl-semantics/tests/it/tc235_business_meanings.rs:334; qsl-semantics/tests/it/tc235_business_meanings.rs:407 |
| FND-007 | high | The rebased head still refuses RecordValueType/EventType in model_object_types and native String in model_value_type. lifecycle_package assembles full K2, and this PR removes the M02/M13 semantics ignores; those paths cannot succeed until PR 777 and PR 781 land. PR body nevertheless claims all five runtime selectors pass and does not declare the dependency state. Rebase/qualify against the actual dependencies, or explicitly preserve unmet status without claiming this head green. | qsl-semantics/src/check/assemble.rs:680; qsl-semantics/src/check/assemble.rs:811; qsl-eval/tests/it/tc235_business_meaning_runtime.rs:201 |
| FND-008 | medium | PR-body line links are stale after the fixture expansion: M02 is linked around 269 while its test begins at 539; M13/M14/M15/M22 links likewise point to helpers or other cases. The table also calls authored string/JSON assertions exact runtime evidence. Update links and coverage descriptions to the reviewed implementation, identifying component-only checks and real runtime acceptance separately. | PR 765 body; qsl-eval/tests/it/tc235_business_meaning_runtime.rs:539 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| QSpec FR-208-AC-4 / TC-235 M02 | examined | Record equality depends on the most-specific type and every field; matching values are true and a changed currency is false. |
| QSpec FR-208-AC-10 / TC-235 M13 | examined | Evaluate invariants once for every reachable record value, including collection elements, optionals and invocation parameters; four false results have exact runtime value paths. |
| QSpec TC-235 M14 | examined | Occurred is bound and type-checked; no reached event value means no result for Occurred. |
| QSpec FR-208 context anchors / TC-235 M15 | examined | Invariants bind pre and post contexts; postcondition pre(e) reads the pre context; absent context and duplicate contexts have distinct typed refusals. |
| QSpec FR-207-AC-8 / TC-235 M22 | examined | Receiver and parameter reads have distinct invocation-binding roots, field steps and canonical ordering; process receivers retain object identity. |
| Existing TC-235 M01/M07 | examined | Effective process and persistence identities/universes; typed process refusals and source record accounting. |
| PR 777 / PR 781 | context_only | Record/event assembly and native String adaptation remain open at review. |

## Coverage and limits

M02 does exercise real record equality with currency changed, but its authored K2 fixture is not consumed and the Order value is a record component rather than the model-bound object. It does not establish the complete M02 model/runtime path or a change in every individual field. M15 exercises real model normalization/admission and clause evaluation when its dependencies are present; missing/duplicate context tests assert useful typed fields. Its invariant helper constructs admitted invariant observations manually, so it does not qualify automatic invocation invariant selection or the exact result set excluding guards. No result-wire round-trip/invalid-input tests for the new Invocation variants were added in this diff; M22 must use that boundary to qualify its serialization claim.

No spec/plan files change, so separate requirement-authoring/EARS review is not applicable. No unsafe or CI changes identified. Diff whitespace check passes. Source and test claims were inspected at the exact remote head; no execution evidence was accepted as a final gate. No builds, tracker posts, source modifications or merge performed.
