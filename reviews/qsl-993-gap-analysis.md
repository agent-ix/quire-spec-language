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
