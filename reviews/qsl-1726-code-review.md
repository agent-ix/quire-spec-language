---
id: SR-20261105101010
title: "Code review — QSL-1726 trace-subject temporal route"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2391; qsl-replay/src/spine/clause/trace_subject.rs, qsl-replay/src/spine/native_result/temporal.rs, qsl-replay/src/result/wire.rs, qsl-semantics/src/check/mod.rs; FR-330-AC-17, FR-267-AC-15"
review_set: subset
---
# Code review — QSL-1726 trace-subject temporal route

## Summary

Ticket: QSL-1726. Reviewed PR #2391 at `caea1b81d50484f4584e7dbf5adeaf85dc961692` against merged FR-330-AC-17, FR-267-AC-15, and the FR-330 decisive-atom contract. The Rust lane covered API and behavior, test assertions, resource/error paths, and native wire compatibility. No applicable AssuranceProfile was found. Focused check results were supplied by the coder and not rerun; the lead will run the aggregate gate after review.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| Trace-subject admission and aggregate-only restriction | qsl-replay/src/spine/clause/trace_subject.rs:149-171 | examined |
| Aggregate atom and decisive side-channel | qsl-replay/src/spine/clause/trace_subject.rs:206-231, 425-480 | examined |
| Trace-subject tests and fixture clauses | qsl-replay/src/spine/clause/trace_subject.rs:630-846 | examined |
| Native insufficient-data wire variant and writer | qsl-replay/src/spine/native_result/temporal.rs:85-91, 500-516 | examined |
| Native insufficient-data decoder | qsl-replay/src/spine/native_result/temporal.rs:650-675, 930-958 | examined |
| FR-330-AC-17 | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:288 | examined |
| FR-330 decisive atom contract | spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:236-254 | examined |
| FR-267 native insufficient-data member set | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:225 | examined |
| FR-267-AC-15 wire vector | spec/functional/FR-267-write-run-results-as-native-run-result-2.md:515 | examined |

## Verdict

**CONDITIONAL** — three contract defects remain: the decisive side-channel can report a non-deciding aggregate, valid record-subject clauses without aggregates are refused, and the native wire adds an `atom` member absent from FR-267's closed wire contract.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The route remembers the last false aggregate encountered while the engine pre-reads all positions and atoms, then attaches it to any `Violated` result. The evaluator instead selects the first failing activation and descends through its formula to the deciding atom. An `on each` clause can therefore fail at position 0 while a later false aggregate overwrites the side-channel, producing a decisive record for a non-deciding occurrence or position. Track the evaluator's actual descent/decision, or omit `decisive` when it cannot be identified. | qsl-replay/src/spine/clause/trace_subject.rs:210-231, 470-478; qsl-eval/src/temporal/engine.rs:681-729, 786-834; FR-330:236-254 | correct-requirement-no-evidence |
| FND-002 | medium | `has_aggregate == false` refuses every otherwise valid record-subject clause. FR-330-AC-17 only names refusal for a model-subject clause without aggregate terms; the route's accepted record trace subject may have an ordinary `holds` formula and should be evaluated. The `Plain` fixture is a non-model `Counter` record clause, yet the test asserts that it is refused, baking in the extra restriction. Remove this blanket refusal or specify the route restriction and add the criterion. | qsl-replay/src/spine/clause/trace_subject.rs:160-168, 650-667, 826-846; FR-330:79-94, 288 | correct-requirement-no-evidence |
| FND-003 | medium | The writer now emits `atom` in `kind: insufficient-data`, and its closed decoder requires it, but FR-267's wire table defines this row as `position`, `count`, and `min_count` and FR-267-AC-15's vector has no `atom`. Existing native-result consumers following the declared closed schema will reject the new member, and this PR changes the persisted contract without updating its owning artifact. Reconcile FR-267's schema/vector and compatibility expectations before emitting the member. | qsl-replay/src/spine/native_result/temporal.rs:85-91, 500-516, 662-674; spec/functional/FR-267-write-run-results-as-native-run-result-2.md:225, 515 | correct-requirement-no-evidence |

## Rust Review

The code is contained in the changed replay/semantics paths; I found no new unsafe code, panic path from the caller-provided sample length/type checks, or unbounded sample traversal beyond the finite caller-supplied sequence and existing evaluation meter. The focused tests do exercise the new aggregate route and refusals, but the decisive test uses only one false aggregate at the same position as the failing activation, so it cannot detect the side-channel mismatch (FND-001). The new `InsufficientData` atom value is retained through the internal verdict and wire reader/writer, but that round trip does not reconcile the FR-267 closed-object contract (FND-003).

Coder-reported checks: `trace_subject_` tests 3/3, the existing bounded-clause test 1/1, `cargo check -p qsl-replay --lib`, and `cargo clippy -p qsl-replay --lib -- -D warnings` passed. These focused commands were not independently rerun. No workspace aggregate gate was run in review; the lead owns that gate.

## Review Method Limits

The full-repository Test Matrix did not complete: `quoin matrix --repo . --json` ran for about four minutes without output and was stopped at the lead's direction. The separate gap-analysis artifact records the manual AC-to-test check. Optional semantic review was not requested and was skipped.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | medium | `decisive_aggregate` returns a decisive record only when the formula root is an aggregate or `always[0,0]` directly over an aggregate. Other valid formulas, such as `always[0,2]` directly over a false window aggregate, still produce a `Violated` result decided by that aggregate but return no `decisive`, contrary to FR-330's trace-subject route behavior. Follow the evaluator's actual false descent for the supported formula tree, or constrain and specify the route's formula profile. | qsl-replay/src/spine/clause/trace_subject.rs:235-269; spec/functional/FR-330-run-a-temporal-clause-through-the-spine.md:215-220, 222-254 | correct-requirement-no-evidence |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Commit `efba68725` records the first false position per aggregate occurrence and emits a decisive only when it matches the evaluator's failing position. The added `on each always[0,0]` regression asserts the earlier violation position remains decisive despite later false reads. |
| FND-002 | fixed | Commit `efba68725` removes the blanket no-aggregate refusal. The new record-clause test expects `Holds`, and the model fixture test exercises a model-subject/no-aggregate clause and asserts refusal before evaluation. |
| FND-003 | fixed | Commit `efba68725` restores FR-267's wire row to exactly `position`, `count`, and `min_count`; the wire test asserts the emitted row has no `atom` and explicitly refuses an added `atom` member. |
