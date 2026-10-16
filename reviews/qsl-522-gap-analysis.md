---
id: SR-11734
title: "Gap analysis of quire-spec-language PR #903: TC-1043 intake rows to tests"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@7df1426cacf99f467739f866c063359847185bbd; qsl-semantics/tests/it/model_receiver_frames.rs, qsl-semantics/src/model/intake.rs (tests module)"
review_set: subset
---
# Gap analysis of quire-spec-language PR #903

## Summary

Ticket: QSL-522 (partial delivery: intake only). Planless gap analysis, scoped
to the PR diff: each TC-1043 step 3 row (I1 to I17) and FR-154-AC-8 mapped to
the test that asserts it, and each new trace tag checked against the AC and
TC it names. Plan completion: not assessed. The rows the author lists as not
done (W1 to W6, X1 to X9, steps 2/2b, FR-105-AC-7, FR-154-AC-6/7/9/10, D1 to
D5, F1 to F3) are out of scope and are not findings; the PR body does not
claim them.

Examined:
- FR-154-AC-8 (examined)
- TC-1043 step 3 rows I1 to I17 and their Expected Results (examined)
- FR-154-AC-4 and TC-573 "Scope: FR-154-AC-3 to FR-154-AC-4" (examined)
- `model_receiver_frames.rs`: nine tests (examined)
- `intake.rs` `a_malformed_frame_member_refuses_at_the_member` (I10 to I12)
  (examined)

Row map: I1 `a_receiver_entry_naming_no_declaration_refuses_missing_name`;
I2, I3 `a_receiver_entry_of_another_type_or_meaning_refuses_malformed_declaration`;
I4, I5 `a_deleted_parameter_that_is_no_reference_parameter_refuses_missing_name`;
I6 `a_reference_parameter_and_the_receiver_admit_as_deletes`; I7
`a_receiver_entry_may_name_an_inherited_field`; I8
`an_unresolved_entry_is_reported_before_a_scope_conflict`; I9
`both_scopes_of_one_field_refuse_conflicting_binding`; I10 to I12 the
`intake.rs` unit test; I13, I14, I17
`a_process_operation_refuses_receiver_scoped_members_at_the_member`; I15, I16
`a_process_operation_admits_empty_receiver_members_and_deleted_parameters`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | I13 and I17 cannot fail if the receiver rule is removed. The test asserts code `invalid_model_binding`, cause string `malformed-declaration`, one refusal, and `detail.contains("frame.modifies_receiver")`. Without the receiver rule, `Flow/x` resolves in `resolve_receiver_modifies` with `receiver_type: None` and refuses `FrameEntryMalformed` at the entry: same code, same cause string, one refusal, and its detail `$.types[3].operations[0].frame.modifies_receiver[0]: ...` contains the same substring. So the test does not check what TC-1043 states for I13/I17: refused at the member, before any entry is read (`Flow/x` is not read). Only I14 depends on the rule. Fix: assert the cause is `IntakeMalformedDeclaration` (not `FrameEntryMalformed`) and that its member path ends at `frame.modifies_receiver` with no index. | qsl-semantics/tests/it/model_receiver_frames.rs:385-410 |
| FND-002 | high | Trace overclaim on FR-154-AC-4. `both_scopes_of_one_field_refuse_conflicting_binding` is tagged `#[trace("TC-1043", "FR-154-AC-8", "FR-154-AC-4")]`. FR-154-AC-4 is verified by TC-573 (its Verification column; TC-573 "Scope: FR-154-AC-3 to FR-154-AC-4", tag `("TC-573", "FR-154-AC-n")`), not TC-1043, and the AC's first half (the S4 frame node carries scope `receiver` vs `every-object`, contents differ) is the emission work this PR says is not done. The tag makes the computed matrix count FR-154-AC-4 as tested. Fix: drop `"FR-154-AC-4"` from the tag (I9 is fully FR-154-AC-8); bind AC-4 under TC-573 when emission lands. | qsl-semantics/tests/it/model_receiver_frames.rs:347-350 |

## Verdict

FAIL, two blocking findings, both test-only fixes. The production intake is
correct against FR-154 (see SR-11733). Every row I1 to I17 has a test; I1 to
I12, I14, I15 and I16 assert the stated code, cause and locus. I8 asserts the
single `missing-name` refusal, so the conflict really is checked last. I9
asserts the later-entry locus and that both entries are named.

Recorded, not blocking:
- low: I15/I16 assert admission only, not the resulting effect (for I16,
  `deletes_arguments == ["a"]` on `go`).
- low: I1 to I5 assert the entry string but not the entry's member path.
