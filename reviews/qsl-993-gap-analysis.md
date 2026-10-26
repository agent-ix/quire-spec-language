---
id: SR-1587004
title: "gap analysis of QSL-993 attempt records and RecoveryView (QSL PR 1591)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@7b31c224045d9d3dba6594eed4ee2f6510e55e83; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md, spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md, qsl-analyze/tests/it/protocol_compensation.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# SR-1587004: gap analysis of QSL-993 (QSL PR 1591)

## Summary

Ticket: QSL-993. Plan completion: not assessed. `quoin matrix --json` timed out at 600 s on
this repo, so the AC-to-test check is manual, made by reading the trace tags and the test
bodies.

The `#[trace]` bindings found:

- `a_recovery_predicate_over_the_recovery_view_settles_unsupported_before_exploring` is tagged
  FR-207-AC-5 (TC-652). It is correct.
- `a_cattempt_takes_one_step_per_attempt_record_the_retry_relation_admits` is tagged
  FR-207-AC-6 (TC-652). It is correct.
- `cattempt_and_the_commit_step_cover_the_attempt_record` is tagged FR-216-AC-7 (TC-661). It is
  correct.
- `the_attempt_record_is_keyed_while_a_retry_can_read_it` is tagged FR-205-AC-8 (TC-650). It is
  partial; see FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The tag inflates coverage. `the_attempt_record_is_keyed_while_a_retry_can_read_it` carries `#[trace("TC-650", "FR-205-AC-8")]`, but its doc says it covers only the attempt-record variant. No tagged test asserts AC-8's first three sentences: under `repeat` and `parallel Triggers`, a compensation thread's entry names `r1` or `r2` crosswise; the key holds `Committed` under its instance after its step; and the entry is absent once the instance finishes. The untagged `a_compensation_thread_keys_its_registration` covers only part of the first. The repo's convention for partial coverage is an untagged `Part of FR-x-AC-k ... Not covered:` doc. So the matrix would report AC-8 as covered when it is not. Either assert the remaining clauses under the tag, or drop the tag and use the `Part of` form. | qsl-analyze/tests/it/protocol_compensation.rs:1309-1321, spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:131 |

## Verdict

FR-207-AC-5, FR-207-AC-6 and FR-216-AC-7 are fully asserted by their tests. FR-205-AC-8's
attempt-record clause is fully asserted, but the AC as a whole is not. No code was found that
has no owning requirement: every new path traces to PS-8, ST-11, ST-12, TS-2 or CP-4. The
exception is the attempt-type `UnenumerableRecord` refusal, recorded as SR-1587003 FND-001.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | Two new tests are tagged FR-207-AC-6 for behaviour the AC does not state. `an_optional_field_adds_a_record_and_an_unbounded_one_requires_a_bound` (qsl-analyze) and `an_attempt_type_of_finite_fields_enumerates_every_record` (qsl-eval unit) carry `#[trace("TC-652", "FR-207-AC-6")]`. AC-6 states only a `RefundAttempt { ok: Boolean }` template under `not e.ok` and `l.ok`. Optional fields, enums, nested records and `requires-bound` for an unbounded attempt field are ADR-027 ST-11/TS-5 behaviour, and no AC states them. AC-6 is fully covered by its own test, so the matrix total does not change, but these bindings are wrong. Drop the tags, or add an AC for the attempt-type domain and the attempt-type root and tag the tests to it. | qsl-analyze/tests/it/protocol_compensation.rs:1492-1499, qsl-eval/src/simulation/model/domain.rs:511-513 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The AC-8 integration test is now untagged, with a correct "Part of" doc. But the new unit test `a_record_value_is_written_in_fr_120_s_record_form` (qsl-eval domain.rs:538) is tagged `#[trace("TC-652", "FR-205-AC-8")]`, and it asserts none of AC-8: it tests FR-120's record encoding. The matrix still reports FR-205-AC-8 as covered while its first three sentences are asserted by no tagged test. Drop that tag. |
| FND-001 | fixed | ee414a85a removes `#[trace("TC-652","FR-205-AC-8")]` from `a_record_value_is_written_in_fr_120_s_record_form`. No test now claims FR-205-AC-8 falsely. The remaining coverage gap is FND-003. |
| FND-002 | fixed | Both FR-207-AC-6 tags are removed. The tests now cite ADR-027 ST-11/TS-5 in prose only. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | FR-205-AC-8 now has no tagged test. One of the four ACs this ticket names is therefore uncovered in the matrix. Its first scenario is asserted nowhere: `Refund` with `attempts 1` and `commit Committed`, two registrations from a `repeat` of maximum 2 that differ in capture `id`, and a `parallel Triggers` of two branches. That scenario has two outcomes, which reach different keys: thread `(0, Refund, 0)` names `r1` after one trigger order and `r2` after the other. `a_compensation_thread_keys_its_registration` (crosswise, untagged) and the new commit-entry assertions cover parts of it. Write the scenario's test and tag it FR-205-AC-8. | spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md:131, qsl-analyze/tests/it/protocol_compensation.rs:1319-1331 |
