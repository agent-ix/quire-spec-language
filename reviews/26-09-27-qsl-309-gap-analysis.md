---
id: SR-771
title: "QSL-309 gap analysis of PR 510 (FR-114 acceptance criteria against TC-513 tests)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@d2c1a5d389c8a94ef820fb66c74ad811586ac7e2; spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md; spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md; spec/test-cases/TC-511-s3-resolves-scoped-anchors-in-nested-scopes.md; spec/test-cases/TC-512-s3-refuses-ambiguous-and-shadowing-names.md (unchanged); spec/test-cases/TC-513-s3-binds-a-protocol-attempt-to-its-operation-frame.md; qsl-forms/tests/it/protocol_clause_forms.rs; qsl-package/src/emit/tests.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/src/check/protocol_clause.rs; qsl-replay/src/spine.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-513
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: references
---
## Summary

Ticket: QSL-309. PR: quire-spec-language#510 at d2c1a5d. There is no plan
bundle; the scope is the ticket's five build steps and FR-114-AC-1 to AC-4
(TC-513).

The seven tests tagged TC-513 are:

- 2 S2 form tests;
- 1 assembly test (unadmitted model);
- 3 protocol_clause tests (missing entry, invariant entry, empty list);
- 1 emission test.

Coverage per AC:

- **AC-1:** checked-attempt identities not asserted; the emission oracle is
  vacuous.
- **AC-2:** check only; emission fails (SR-770 FND-002).
- **AC-3:** 2 of 4 variants.
- **AC-4:** none.

TC-511 regression: `a_missing_anchor_or_member_refuses_naming_the_failing_segment`
and the rest of the 24 protocol_clause tests pass on a fresh run, so a
protocol with a broken anchor, binder or contract still refuses.

Reviewer probes: temporary, reverted. **A:** a `post` clause alone emits
exactly 1 `operation_anchor` and 1 `frame`, the same counts the PR's
emission test asserts with the attempt present. **BASE:** in the PR's own
unit, `CheckedAttempt.contracts[0]` equals `VersionUnchanged`'s identity, so
the implementation is right but untested. **F:** a missing operation on an
attempt nested two sequences deep refuses `UnresolvedOperation` at assembly.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-114-AC-1 is not verified. (a) No test reads `CheckedProtocol::attempts` / `CheckedAttempt`, so the attempt's anchor, frame and contract identities (FR-114 Outputs) are never asserted. (b) `an_attempt_and_its_clause_share_one_frame_node_through_emission` asserts only `operation_anchor == 1` and `frame == 1`. The `post` clause alone already produces exactly those counts (probe A), so the test passes with attempt lowering deleted. (c) The frame's `modifies` being exactly `versionNumber` is not asserted. (d) "exactly one `operation-contract` record for the frame" is not asserted. Fix: assert `attempts[0].anchor`/`.frame` are node ids present in the wire (and equal to the clause's anchor/frame), assert `contracts == [state_clauses[0].identity()]`, and assert the frame body and a single frame requirement record. | qsl-package/src/emit/tests.rs:2873-2908 |
| FND-002 | high | FR-114-AC-2 (only the attempt names `attemptUpdate`: one anchor, one frame, one frame record) has no emission test, and fails when tried (SR-770 FND-002). `an_empty_contracts_list_binds_with_no_refusal` has the same body as `a_protocol_whose_anchors_and_attempt_bindings_both_resolve_checks` (`checks(&recovery_flow("Main::Committed", ""))`), and its `M::Actor` fixture has an empty frame. | qsl-semantics/src/check/protocol_clause.rs:1319-1383 |
| FND-003 | medium | FR-114-AC-3 is partial. The test named `a_contract_entry_anchored_at_a_different_operation_refuses` uses an invariant (`ParentOrder`), so `contracts [ProbePre]` (a `pre` clause of another operation, the only case that compares two operation anchors) is untested. `on Config::ConfigVersion::missing` (attempt `UnresolvedOperation` refusing `missing_declaration`/`missing-name` at `missing`) is untested; the one assembly test covers only an unadmitted model. | qsl-semantics/src/check/protocol_clause.rs:1346-1378; qsl-semantics/src/check/assemble/tests.rs:458-475 |
| FND-004 | medium | FR-114-AC-4 (an attempt on `Config::Sub::attemptUpdate` and a `post` on `Config::ConfigVersion::attemptUpdate` share one anchor and one frame with context `ConfigVersion`) has no test at all. | spec/test-cases/TC-513-s3-binds-a-protocol-attempt-to-its-operation-frame.md |
| FND-005 | medium | The SR-761 FND-001 regression guard lost coverage. `a_protocol_with_unchecked_garbage_content_does_not_compile_silently` became `a_protocol_naming_an_undeclared_model_does_not_compile_silently` and now passes because of the attempt's undeclared model at assembly. No test remains that a protocol with a valid attempt and an ill-typed body or undeclared binder type refuses, and it does not refuse (SR-770 FND-001). | qsl-replay/src/spine.rs:1224-1272 |
| FND-006 | low | S2 capture is tested only for the attempt directly in `run sequence Main`. No S2 test puts an attempt under case, branch or await. Structurally the capture is correct, and probe F showed depth-3 resolution. | qsl-forms/tests/it/protocol_clause_forms.rs:347-383 |
| FND-007 | low | Spec Status text is stale. FR-113 Status still says a resolving protocol "is still refused `unsupported_construct`/`not-yet-implemented`", and FR-114 Status says "Not yet implemented". Update both to what this PR lands, or keep the refusal (SR-770 FND-001). | spec/functional/FR-113-resolve-scoped-anchors-in-nested-control-scopes.md:144-157; spec/functional/FR-114-bind-a-protocol-attempt-to-its-operation-frame.md:100-104 |

## Verdict

Not complete. Only one of the four ACs has a test that could fail on a real
defect in the new code (AC-3, partly). Write the AC-1, AC-2 and AC-4 tests
and the two missing AC-3 variants, restore a garbage-content regression
guard, and update the Status text.
