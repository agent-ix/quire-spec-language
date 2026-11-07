---
id: SR-202611070458156
title: "Code review of QSL-1798: FR-283 monitor trace shapes, sources, emission authentication"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f472f37ba4d21296f9cc442a8529f4700c96ca3f; git diff origin/main...HEAD (PR #2493): qsl-analyze/src/monitor.rs, qsl-replay/src/spine/**, src/command/output.rs and the FR-283 tests"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-283
    type: reviews
---
# Code review of QSL-1798

## Summary

Ticket: QSL-1798, PR #2493, head f472f37ba. The Rust lane is folded in. Checked
FR-283-AC-8, AC-11, AC-12, AC-13 and AC-15 against the spec text and the named
tests. `cargo test -p qsl-analyze monitor` (91) and `cargo test -p qsl-replay`
(1114) pass; clippy `-D warnings` and fmt are clean.

## Verdict

Changes requested. The implementation matches the spec on the main paths. The
old `trace: &[u8]` field is fully replaced with no shim. Defects are in how
caller-input defects are classified and in test evidence.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Caller-input defects in emission authentication surface as StageFailure::Fault (internal failure, exit 30): an unselected clock-bearing declaration with no selection, a selection set that does not fit the package, conflicting domain package documents, a missing dependency closure and a strict read that does not admit all map to InternalFault. FR-283 defines only the selected-clause missing-selection refusal; the rest are request defects, not invariant failures. | qsl-analyze/src/monitor.rs:318-342; qsl-replay/src/spine/clause/package_authentication.rs:1112-1136 |
| FND-002 | medium | a_cancelled_handle_stops_the_evaluation is tagged FR-283-AC-13, whose text names no cancel behaviour (tag inflation), and its monitor() half passes on the pre-checks (cancelled() before admit_positions and trace_run), so it does not prove the poll inside evaluation through monitor. The Samples route has no cancel test. | qsl-analyze/tests/it/monitor_inputs.rs:561-615 |
| FND-003 | medium | Authentication tests cover one forged field (clock_name) on one Timestamped trace. No test alters the profile, the clock parameters or the ordinal, none drives a Progressed trace through monitor, and none shows a bad or conflicting domain package is refused by the strict read. | qsl-analyze/tests/it/monitor_inputs.rs:661-790 |
| FND-004 | low | atom_name falls back to the formula node index (node@N) for a non-Holds, non-Aggregate atom, which FR-283 forbids (never by a formula node index). | qsl-replay/src/spine/clause/temporal.rs:1618-1621 |
| FND-005 | low | charge_document runs before the top-level object check, so a limit defect can precede wrong-value-kind/invalid-value where the spec's defect order lists the reader refusal and the kind check first. authenticate_emission uses SpineLimits::default(), not request limits. | qsl-analyze/src/monitor.rs:785-788 |
