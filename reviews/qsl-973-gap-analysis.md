---
id: SR-12329
title: "Gap analysis of quire-spec-language PR #1216 against FR-070/071/098/116/122/323/357/358 (QSL-973)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1af39d30e; spec/functional/FR-070, FR-071, FR-098, FR-116, FR-122, FR-357, FR-358; spec/test-cases/TC-183, TC-185, TC-444, TC-515, TC-517; QSpec FR-323 at quire-specification main 429c59dd; qsl-replay/src"
review_set: subset
---
# Gap analysis of quire-spec-language PR #1216

## Summary

Ticket: QSL-973. Plan completion: not assessed (planless). This analysis
checks every replay-target criterion against its tagged test and the code.

Each of these units was examined, has a tagged test, and is clean:

- FR-070-AC-18 (TC-183): `tc_183_replay_target_round_trips_and_refuses_none_and_two`.
- FR-071-AC-14 (TC-185): `tc_185_each_replay_target_round_trips` and `tc_185_replay_target_refusals_precede_the_byte_provision_rule`.
- FR-098-AC-21 (TC-444): `tc_444_a_non_function_target_refuses_before_the_recompile`.
- FR-116-AC-3 (TC-515): `a_missing_operation_refuses_missing_name`.
- FR-116-AC-8 (TC-515): `an_inherited_operation_is_selected_by_its_declaring_object` and `a_frame_replay_refuses_other_targets_before_the_recompile`.
- FR-122-AC-1 and FR-122-AC-3 (TC-517): the result holds the request's `state_clause`, and `Absent` and `sameIdentity` each refuse `missing-name`.
- FR-122-AC-6 (TC-517): `the_payload_has_no_member_naming_the_clause`.
- FR-122-AC-8 (TC-517): `a_replay_target_the_entry_does_not_read_refuses_before_the_recompile`.

Mutations of the FR-116 and FR-122 check order fail these tests (SR-12328).
QSpec FR-323-AC-19, AC-20 and AC-21 are verified QSpec-side by TC-278. This
review checked the QSL code against their statements directly.

## Verdict

There are three gaps. Two are Behavior statements with no AC and no test, and
for one of them the code is also wrong (SR-12328 FND-001). The third is
pair-coverage depth.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two FR-116 Behavior refusals have no FR-116 AC and no test: an `object` that cannot declare operations (`malformed-declaration`), and a name two supertypes expose (`ambiguous-name`). FR-116-AC-8 covers only the inherited-operation branch. The first branch is wrong in code (SR-12328 FND-001), and the second works in a probe. Each needs an AC clause in FR-116-AC-8 (or a new AC) and a TC-515 test. | spec/functional/FR-116-replay-a-frame-counterexample.md:106-112 |
| FND-002 | medium | FR-357 Behavior ("A request whose replay target is not `function` SHALL refuse as FR-098 does") and the matching FR-358 statement have no AC and no test. A mutation that drops the refusal survives the whole suite (SR-12328 FND-002). | spec/functional/FR-357-replay-scalar-parity-claims.md:74; spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md:87 |
| FND-003 | low | TC-185 tests the target pairs `function`+`frame_operation` and `function`+`state_clause`. It does not test `frame_operation`+`state_clause`, all three members, or a frame request carrying both `arguments` and an occurrence key. These are FR-323 decode-rule-1 cases. A probe shows each is correct (`/replay/state_clause` for the first two, and `/replay/arguments` wins for the third). | qsl-replay/src/request.rs:655-729 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: FR-116-AC-9 and TC-515 step 7, with tests | b02604c70 |
| FND-002 | fixed: FR-357-AC-20 (TC-904 step 19) and FR-358-AC-9 (TC-907 step 13), with tests | b02604c70 |
| FND-003 | fixed: TC-185 test covers frame_operation+state_clause, all three members, and arguments plus occurrence key together | b02604c70 |
