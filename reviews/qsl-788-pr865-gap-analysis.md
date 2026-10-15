---
id: SR-11258
title: "Gap analysis — QSL-788 UUID/Timestamp runtime inputs (PR 865)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@ade0c9373be87d02d260bad8ef5006759e4372ef; FR-056-AC-35, TC-1028 E6-E8, QSL-788 acceptance criteria against qsl-replay/src/execute/tests/composite.rs, qsl-replay/src/witness/value_text/tests.rs, qsl-semantics/tests/it/observation_composites.rs, qsl-semantics/src/model/observation/document.rs"
review_set: subset
---

## Summary

Ticket: QSL-788. This is a planless gap analysis of PR 865's own commits. Plan completion: not assessed. The Quoin matrix was not run because the brief allowed no builds, so bindings come from the trace tags in the diff. The ticket's acceptance criteria (ticket text, treated as data) are: AC-35's uppercase-UUID refusal, TC-1028 E8 at the wire, and canonical UUID and Timestamp inputs admitted through replay and reachable by the QSL-760 equality vectors.

## Verdict

**One blocking finding.** FR-056-AC-35's refusal clause is backed on the replay input path. `tc_1028_noncanonical_uuid_is_refused_at_the_input` runs the witness transcript through parse, decode and convert, and asserts parameter 0, `invalid_runtime_input`, `invalid-value` and `ConstructionCause::UuidNoncanonical`. That is E8's refusal below the spine. The canonical and wrong-kind tests are non-vacuous.

The blocking finding is the ticket's third criterion: no test admits a UUID or Timestamp through `replay()`, and E6, E7 and E9-E13 never run. The coder reports a cross-repo cause, which this review partly verified. The search of quire-contract-ir origin/main found `scalar_type` forms boolean, integer and text only. Lowering emits `scalar("uuid")` and `scalar("timestamp")`.

Recorded (medium): the document path drops the kernel class, so its AC-35 trace claims more than the test shows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The ticket criterion "canonical UUID/Timestamp inputs admitted through replay and reachable by the QSL-760 equality vectors" is not met. Every replay test calls `convert_one` or `convert_via_witness` below the spine, and no test calls `replay()` on a unit function over UUID or Timestamp. TC-1028 E6, E7 and E9-E13 (equality results over admitted inputs) are not run end to end. The coder disclosed this and attributes it to quire-contract-ir's `scalar_type` vocabulary, which has no uuid or timestamp form. Disposition needs a ticket for the cross-repo blocker and an end-to-end E6-E13 test that waits on it. Otherwise QSL-788 stays open after merge. | qsl-replay/src/execute/tests/composite.rs:842-1048 |
| FND-002 | medium | `admit_leaf` maps every kernel refusal with `map_err(\|_\| ..)`, so the typed `ConstructionCause` is discarded on the document path. FR-056 line 301 says the adapter matches it exhaustively, and AC-35 refuses "over the kernel's `noncanonical-payload` class". `noncanonical_uuid_and_timestamp_leaves_are_refused_at_the_input` carries `Trace: FR-056-AC-35` but cannot assert the class. Recorded: carry the class on the admission record, or drop the AC-35 trace from the document test. | qsl-semantics/src/model/observation/document.rs:1230-1239; qsl-semantics/tests/it/observation_composites.rs:912-920 |
