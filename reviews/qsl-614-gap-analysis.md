---
id: SR-1210
title: "QSL-614 gap analysis of PR #595 (FR-069, FR-070, FR-071, FR-098, FR-121)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@31fa710fe1399a1203bbe03d0b09e204049300bc; PR #595 diff against origin/main; spec/functional/FR-071-implement-typed-replay-request.md; spec/functional/FR-098-execute-a-replay-request.md; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
---
## Summary

Ticket: QSL-614. PR: quire-spec-language#595.

Deleted tests, checked against the spec on origin/main:
- `tc_178`'s version and vocabulary cases traced FR-069-AC-2. That AC is gone
  from FR-069. TC-178 now traces only AC-4, and the oversized case is kept.
- `tc_188_refuses_unknown_version_or_profile_before_recompilation` traced
  FR-071-AC-4, which is gone. Its profile case survives as
  `refuses_an_unknown_semantic_profile_before_the_byte_provision`, now with no
  trace (FND-001).
- `tc_445` in request.rs and in witness.rs traced FR-071-AC-8 and FR-070-AC-8.
  Both ACs and TC-445 are gone from spec/.
- `tc_444_an_unknown_version_refuses` traced FR-098-AC-4, which still lists
  "unknown version" (FND-002).
- So every deleted test backed only deleted ACs, except
  `tc_444_an_unknown_version_refuses`, whose AC text was not updated.

Trace:
- FR-121-AC-14: `call_site_refusal_codes_are_the_replay_refusal_codes`
  (execute/tests.rs) and `call_site_refusal_codes_for_operations_clauses_and_intake`
  (spine/clause/tests/call_site.rs) cover all seven variants AC-14 names,
  each against `replay`'s own code. These bindings are correct.
- FR-071-AC-1 (TC-185): the round trip carries `obligation_identity`.
- FR-069-AC-4 (TC-178) and FR-070's TC-181 bound test still exercise the
  measured bound.
- The FR-069, FR-070 and FR-121 Status text matches the code. FR-071's Status
  matches, but its Outputs do not (FND-003).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The unknown-semantic-profile refusal is kept by team-leader ruling: QSL cannot evaluate under semantics it does not know. Its test, `refuses_an_unknown_semantic_profile_before_the_byte_provision`, lost its trace when FR-071-AC-4 was deleted, and no FR-071 AC owns the behaviour. The test's own doc says "no acceptance criterion traces it". Add an FR-071 AC: a profile selection outside the closed known set refuses `unknown_profile`/`unsupported-selection` at decode, before the byte provision is read, retaining the selection and its role. Then trace the test to that AC and a TC. | qsl-replay/src/request.rs:996-1027; spec/functional/FR-071-implement-typed-replay-request.md:117-123 |
| FND-002 | high | FR-098-AC-4 lists "unknown version" among the refusals that replay must produce. TC-444 step 5 still says "Replay with an unknown contract version" (procedure, :45) and expects `UnknownContractVersion` first (expected results, :83). No such refusal exists after this PR, and its test was deleted, so AC-4 as written cannot be met. Delete "unknown version" from FR-098-AC-4 (and from FR-098 Behavior if it is listed there), and delete the version case from TC-444 step 5's procedure and expected results. | spec/functional/FR-098-execute-a-replay-request.md:151; spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md:45; spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md:83 |
| FND-003 | medium | The FR-071 Outputs still promise "a structured refusal when the input is malformed, unversioned, or names an ...". The request no longer has a version member, so no refusal for "unversioned" input exists. Delete "unversioned". | spec/functional/FR-071-implement-typed-replay-request.md:51 |

## Verdict

Changes requested: FND-002 is high, FND-001 and FND-003 are medium. All three
are spec drift, or a missing trace, for this PR's fix round.

## Dispositions

Round 1, reviewed at 321b740821583f148b1e18a7a90a81202bec298d.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 321b7408: new FR-071 Behavior bullet and FR-071-AC-10 (an unknown semantic profile refuses `unknown_profile`/`unsupported-selection` at decode, keeping the selection and its role, before the byte provision is read). TC-186 gains step 8 and its expected result. The test is traced `#[trace("TC-186", "FR-071-AC-10")]`, and it asserts the selection, the role, the code and the message, with a mismatching byte-provision entry alongside. |
| FND-002 | fixed | 321b7408: "unknown version" is deleted from FR-098-AC-4, and the unknown contract version is deleted from TC-444 step 5's procedure and expected results. FR-098 Behavior had no such entry. |
| FND-003 | fixed | 321b7408: FR-071 Outputs now read "a structured refusal when the input is malformed or names an out-of-domain digest or profile identifier." |
