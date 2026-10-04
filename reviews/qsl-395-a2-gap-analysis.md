---
id: SR-1282
title: "Gap analysis of quire-spec-language A2: a library carries no version"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@e3c96833abd7e79ca31e84acedd8948253ad39e3; A2's own commits only, git diff ad18594412cb4cdd849104a7643491b231cd79ee..e3c96833a: acceptance criteria FR-027-AC-10, FR-071-AC-9, FR-087-AC-3, FR-087-AC-12, FR-087-AC-14, FR-099-AC-1/2/3/7, FR-098-AC-6/7, TC-186, TC-253, TC-282, TC-444, TC-446 against the tests that trace them"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-446
    type: reviews
---
# Gap analysis of quire-spec-language A2

## Summary

Ticket: QSL-395 (A2, L2). No PR yet (stacked on HL1, then LC1 #624). This is
a manual criteria-to-tests check over the criteria A2 edits or whose tests A2
edits.

Criteria checked, each against its traced tests at e3c96833a:

- FR-027-AC-10 (TC-446 step 7): `tests/it/compile_command.rs`
  `malformed_libraries_refuse_as_invalid_request` covers the `0-draft`
  library, the empty identity and the new `version` member case, each
  asserting exit 20, empty stdout and `invalid-request`.
  `a_complete_v1_request_supplies_its_libraries_to_the_spine` covers the
  exit-0 half. Backed.
- FR-071-AC-9 (TC-186): `request::tests::tc_186_dependencies_round_trip_and_refuse_at_decode`
  covers round trip, missing byte provision, empty identity and a wrong
  `package_id` domain. The empty-version case is gone with the field. Backed.
- FR-087-AC-3 (TC-253): conditions 1 to 3, the three substitute digests and
  the doubly-failing input are each backed in `binding_tests.rs` and
  `checked_v2/tests.rs`. One step label is wrong (SR-1281 FND-001), but the
  `#[trace]` tags are correct.
- FR-087-AC-12 (TC-282): `library_resolution.rs` classifies every
  `resolve_libraries` variant. `duplicate_package_id_is_the_named_exception_outside_all_four`
  now varies `imports`, matching TC-282's amended table. Backed.
- FR-087-AC-14 (TC-253 step 10): `e4_refuses_a_conflicting_diamond` keeps
  the two-`package_id` case and the non-emitting dependency case;
  `a_diamond_selecting_one_package_unifies` the admitted side. Backed.
- FR-099-AC-1 to AC-3 (TC-446 steps 1 to 3):
  `spine::dependency_tests` backs each; the empty-version step is deleted
  from both TC-446 and the test.
- FR-098-AC-6/7 (TC-444): `tc_444_dependency_entries_refuse_by_the_d4_rules`
  drops only the `version` argument. Rules 5 to 7 are unchanged.
- FR-099-AC-7 (TC-446 step 8): not implemented at this head.
  `DependencyLimits` (`qsl-replay/src/spine.rs:788`) has only `depth`; no
  `dependency.libraries`, `dependency.import_edges` or
  `dependency.source_bytes` limit exists, and no test traces FR-099-AC-7.
  TC-446's Status line "Step 8 (FR-099-AC-7) is pending" is therefore true
  and must stay. FR-099's Status already scopes its "passes" claim to AC-1
  to AC-6.

No deleted test leaves a criterion unbacked: every deleted test or test step
asserted a refusal of the deleted `version` field.

## Verdict

Clean. Every criterion A2 touches is backed by a test that would fail on
the wrong behaviour, and FR-099-AC-7 is correctly recorded as pending.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
