---
id: SR-17012
title: "Gap analysis of quire-spec-language PR #1443: QSL-676 frame occurrence region (FR-105-AC-9)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@f6219be1e28138f73bbaa7de2189a867577e67c4; PR #1443 (QSL-676) diff origin/main...HEAD; criterion FR-105-AC-9"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1443

## Summary

Ticket: QSL-676. This is a planless gap analysis. `quoin matrix --repo .
--json` timed out after 300 s on this tree. The binding was therefore read
with `quire trace --scope . --id FR-105-AC-9 --exclude-path target`, which
runs over the same traceability model.

FR-105-AC-9, clause by clause, against
`frame_occurrence_region_is_the_first_naming_clauses_operation_reference`:

| AC-9 clause | Status in the test |
| --- | --- |
| ConfigVersion package: the frame `generated` occurrence for `attemptUpdate` carries exactly one region, the byte span of the qualified reference in `VersionUnchanged` | asserted: one region, exact start offset and text |
| An earlier `post` clause naming `attemptUpdate` moves the region there, with (node id, role, ordinal) unchanged | asserted |
| Fixture R: the occurrences for `all` and `same` carry each operation's reference in the first naming clause | asserted (shared frame node, ordinals 0 and 1, `DoneAll` and `DoneSame` references) |
| A regionless `generated` frame occurrence refuses `UnlocatedOccurrence` (FR-062) | asserted, against a fully placed control that emits |

The test asserts every clause of the AC. Three reviewer mutants of the
production rule are killed (see SR-17011).

Reverse gap: the new `state_operation_site` and `state_operation_location`
each have a production caller. No stubs were found.

Uncovered by design: the "or protocol attempt" half of the FR-105 Outputs
and FR-104 Requirements sentence has no AC, and attempts still use their own
location. QSL-1055 tracks it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-105-AC-9 has no claim in the traceability model. The test is tagged with the path-qualified `#[ix_trace_rs::trace("TC-462", "FR-105-AC-9")]`, and `quire trace --id FR-105-AC-9` reports `Claims (0)`, listing the test only as an `evidence_near_miss` citation. The same form at qsl-eval/tests/it/fairness_kernels.rs:784 is also a near miss, while the plain `#[trace(...)]` form (for example FR-105-AC-1 at qsl-replay/src/spine/clause/tests.rs:3798) is a `rust-trace-attribute` claim. The matrix therefore still reads FR-105-AC-9 as untested. Fix: use `#[trace("TC-462", "FR-105-AC-9")]`. `trace` is already in scope through `use super::*` from tests.rs:20. | qsl-replay/src/spine/clause/tests/frame_occurrence_region.rs:90 |
