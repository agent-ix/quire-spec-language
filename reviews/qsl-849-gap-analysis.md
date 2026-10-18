---
id: SR-12128
title: "QSL-849 PR #956 gap analysis: FR-218 and FR-308 channel, join and member criteria"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a5115665a91042bc33b64b0571ca9a21df0664b8; FR-218-AC-10, FR-218-AC-13, FR-218-AC-21..FR-218-AC-24, FR-308-AC-1..FR-308-AC-5, FR-308-AC-13, TC-887, TC-1056, TC-1065; qsl-semantics/src/check/protocol_controls.rs, qsl-semantics/src/check/protocol_clause/coverage.rs, qsl-forms/src/protocol_control.rs, qsl-forms/tests/it/protocol_control_forms.rs"
review_set: subset
---

## Summary

Ticket: QSL-849. PR quire-spec-language#956. This is a planless gap analysis scoped to the diff, so plan completion was not assessed. The criteria were read from `spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md` and `spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md`. Coverage was measured with `quire matrix --format tsv` (quire 0.36.2) on the reviewed head. Every criterion the diff binds was then checked against its test, by asking what source change would make that test fail.

The matrix binds a criterion to a test when the criterion id appears in the test's doc comment beside a TC-only `#[trace]`. For example, `FR-218-AC-22 (TC-1065 rows 4 and 5)` binds AC-22, and `FR-218 "Channels" (TC-1056 rows 1 to 4)` binds nothing. Three of the doc comments this PR adds name a criterion that the test only partly verifies. The matrix then reports that criterion as `tagged`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `quire matrix` reports FR-218-AC-24 as tagged, through the doc comment of `a_role_on_a_record_and_a_channel_of_an_object_refuse_at_the_type_name`, but the code does not have the behaviour. The AC says a role on a process and a role on a state machine check. `content` still refuses any role whose model is neither an object type nor a named type (alias, composite, enum or object) with `MissingRoleType`, so those roles refuse `missing_declaration`/`missing-name`. No test writes a role on a process or a state machine, and none writes a channel that carries an event type or a variant type. | qsl-semantics/src/check/protocol_controls.rs:787-794, qsl-semantics/src/check/protocol_clause/coverage.rs:111-120, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:392 |
| FND-002 | medium | `quire matrix` reports FR-218-AC-22 as tagged, through the doc comment of `equal_case_or_branch_labels_refuse_and_a_single_one_fails_at_the_parser`. The AC's "two cases and two branches check" half is tested for `parallel` only. A two-case `choice` with distinct labels still refuses `unsupported_construct`/`not-yet-implemented`, because `choice` and `case` are not covered kinds, so that half is reported covered while it fails. | qsl-semantics/src/check/protocol_controls.rs:661-706, qsl-semantics/src/check/protocol_clause/coverage.rs:31-33, spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:390 |
| FND-003 | medium | `quire matrix` reports FR-308-AC-1 as tagged, through the doc comment "FR-308-AC-1, the channel part" on `a_channel_builds_every_premise_as_written`. That test is the only binding. The AC's `Replicated` role (population, max, `Until` lifetime) and `OnEach` activation parts have no form and no test. FR-308 Outputs states that they are the target of later slices. | qsl-forms/tests/it/protocol_control_forms.rs:93-97, spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md:589 |

## Verdict

FIX, for three coverage-inflation gaps. Either narrow the three doc comments so that they stop naming a criterion they do not fully verify, or implement and test the missing halves (role on a process or state machine, event-type and variant-type payloads, a two-case `choice` that checks). No code defect was found in the behaviour the tests do cover.

Criteria examined with no finding (oracle strength confirmed):

- FR-218-AC-13 and FR-218-AC-21 (`every_premise_is_recorded_as_written_and_none_is_derived_from_another`): the five overflow values are asserted as an ordered list, and the four single-premise variants are compared field by field and by `assert_ne!` after normalising name and span. A derived selector, or a delivery that leaked into overflow, would fail the test.
- FR-218-AC-23 (`an_unsound_join_refuses_at_the_list_or_the_quorum_count`): each fault, its code and its located text (`[east]`, `0`, `3`, `[]`) are asserted. `quorum(3) [east, west, east]` would fail if repeats were counted.
- FR-218-AC-10, without its relationship clause (untagged, TC-1056 only): the `to` end, both ends with `from` first, the unresolved `carries` and the duplicate channel are each asserted with code, cause, retained members and span. The test does not claim AC-10, which is correct, because the "a channel and a relationship of one name check" half needs relationships, which are still unimplemented.
- FR-218-AC-2's join clause: `a_join_naming_no_branch_refuses_at_the_label` asserts `missing-name` at `middle` with segments and scope. It is untagged, which is correct, because the rest of AC-2 is out of scope.
- FR-308-AC-2, AC-3, AC-4, AC-5 and AC-13: the form values and the span slices are asserted, including labels that exclude `outstanding cancel`, the repeat shapes the parser refuses, the parser failure offsets, `RecoveringCst` for four broken members, and rebuild equality.
- FR-211 and FR-212's second-member refusal (TC-656) is asserted with both loci.
