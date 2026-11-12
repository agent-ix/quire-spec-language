---
id: SR-2026111216593802
title: "gap analysis of PR2692 protocol analysis key, premises and witness record amendment"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-572, FR-574, FR-519-AC-21..23 and the tests PR2692 tags to them; PR2692 e2a6140cf9aac9c7c6a8d8f9ab8a5d0dc9fd5fdd"
review_set: subset
---

## Summary

Ticket: QSL-1894 (ADR-036 slice 2). Plan completion: not assessed. The gap analysis is scoped to the criteria the PR tags and to the ADR-036 slice 2 row. FR-572-AC-1 to AC-4 and FR-519-AC-21 and AC-23 are backed. Two tags claim more than their tests show (FR-572-AC-6 and FR-574-AC-2). One FR-519-AC-22 branch is untested. `from_view` is untested (see SR-2026111216593801 FND-004).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The FR-572-AC-6 tag on `run_limits_and_the_order_of_premises_are_outside_the_key` is vacuous. `other` is built with `..result.clone()`, so `result.key == other.key` holds by construction. No request or assessed-subject root exists to vary, and "carries no artifact" is asserted only as `!matches!(Proved)`. The premise-order half is FR-572 Behavior (ordering), not AC-6. Drop the AC-6 tag until slice 3 has a run, or test what the type can show. | qsl-replay/src/protocol_analysis/tests.rs:207-230 |
| FND-002 | medium | The FR-574-AC-2 tag claims more than the test shows. AC-2 requires the strict reader to refuse a record missing `deciding_element`, `value_path` or `trace_position`. The test only inserts `index` or `quantifier` on the read side, asserts `is_err()` without the cause, and exercises the cause codec rather than the `native-run-result/2` document that FR-574 names. Keep FR-574-AC-6 and either narrow the tag or complete AC-2. | qsl-replay/src/spine/clause/tests/witness.rs:2471-2552 |
| FND-003 | medium | FR-519-AC-22 requires `queue_channel` to refuse `MalformedMember` when `channel` is no reference, and `queue_overflow` to refuse a selector on `block`. The test covers neither (only the missing `channel` and a selector on `reject`), and TC-1314 row 22 omits both. | qsl-package/src/emit/tests/protocol_view.rs:2023-2120 |
| FND-004 | low | FR-572-AC-4's reader half is tested for realizability only by removing `role_knowledge`, `environment` and `analyzer`. It is not tested for `definition_key`, `strategy_profile`, `channels`, `choices`, `bounds`, `max_history` or `max_depth`. FR-572-AC-1 changes the choice id but never the visible set, and no FR-054 channel member other than the channel id. | qsl-replay/src/protocol_analysis/tests.rs:72-205 |

## Scope and evidence

Planless audit of the PR diff. Tests are linked by `#[trace]` tags. The focused run of the qsl-replay `protocol_analysis` tests and the new qsl-package `protocol_view` and witness tests is recorded in the reviewer report. The FR-572-AC-5 (determinism) and FR-573, FR-570, FR-571 and FR-575 rows are slice 3 to 5 work and were correctly left untagged. The FR-519-AC-23 test is a source scan for `value_path` in `protocol_view.rs`. It is weak but honest.

- FR-572-AC-1 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): Changing the definition key, the role, the profile, a channel, choice or visibility premise, or a bound yields a different projection key. (examined)
- FR-572-AC-2 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): Changing the definition key, the strategy profile, the role knowledge or visibility, a channel or environment premise, a bound or the analyzer identity or version yields a different realizability key. (examined)
- FR-572-AC-3 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): A realizability key with the role-knowledge or environment premise None is present and unequal to one with that premise Some of the empty set. (examined)
- FR-572-AC-4 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): For each key member in turn, building a key without it, and reading a stored result with it removed, are each refused. (examined)
- FR-572-AC-6 (spec/functional/FR-572-key-protocol-analysis-results-structurally.md): A run stopped by max_states, max_transitions or the evaluation meter leaves the key unchanged; a request with the root changed alone yields an equal key. (examined)
- FR-574-AC-2 (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): The record has an absent deciding_quantifier and an absent index; a strict reader refuses a record with either present or with a missing deciding_element, value_path or trace_position. (examined)
- FR-574-AC-6 (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md): A state-clause record with its quantifier and index round-trips unchanged. (examined)
- FR-519-AC-21 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): channel_ordering and fifo_key. (examined)
- FR-519-AC-22 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): queue_overflow and queue_channel. (examined)
- FR-519-AC-23 (spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md): no value-path accessor. (examined)
