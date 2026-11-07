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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | a_handle_cancelled_during_a_samples_evaluation_stops_it proves the in-evaluation poll with a wall-clock race: it sleeps uncancelled/10 on a second thread and asserts cancelled < uncancelled/2. Under a loaded gate (tests run in parallel) a descheduled canceller lets the run finish, so it fails spuriously. quire_exact::Cancel::observing(fn) calls the observer at every meter charge; cancel from the observer at the Nth charge and assert Cancelled and charges() == N, which is deterministic and is what TC-757 ("within one charge") states. | qsl-analyze/tests/it/monitor_inputs.rs (a_handle_cancelled_during_a_samples_evaluation_stops_it) |
| FND-007 | low | The domain-package refusal test accepts any stage in {conflict, closure, strict-read}; the Conflict and Closure branches of authenticate_emission are not individually exercised, so reverting either to a Fault is not caught. | qsl-analyze/tests/it/monitor_inputs.rs (domain_packages_the_strict_read_refuses_are_request_refusals) |

## Dispositions

Round 1, reviewed at 86456bba3355d65ec747d3d14d16e8a6d3d54a11. Touched tests pass (qsl-analyze it monitor 98, qsl-replay 1114), clippy -D warnings and fmt clean.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Each defect is now ClauseRefused (missing-selection or invalid-value with member temporal_selections or packages); tests defective_clock_selections_are_request_refusals and domain_packages_the_strict_read_refuses_are_request_refusals. FR-283 should still name these causes (spec gap, see comment). |
| FND-002 | still-open | Tag inflation fixed (before-call test tagged FR-276-AC-1, mid-run test TC-757) and a Samples-route mid-run test exists, but it is timing based; see FND-006. |
| FND-003 | fixed | Profile, parameters, unit forgeries and a Progressed trace through monitor are tested before admission. The ordinal is unreachable: TraceBinding has no ordinal and the trace document has none. |
| FND-004 | fixed | atom_name now returns an internal failure instead of node@N; only_a_holds_or_aggregate_atom_is_named. |
| FND-005 | fixed | Kind check precedes charge_document; the_kind_refusal_precedes_the_document_limits. Default SpineLimits: accepted-no-change, documented on authenticate_emission, the spec gives the request no limit for it. |
| FND-006 | still-open | Replace the sleep race with Cancel::observing cancelling at a fixed charge. |
| FND-007 | accepted-no-change | Same mapping class for all three stages; low risk, may be assert-by-stage later. |
