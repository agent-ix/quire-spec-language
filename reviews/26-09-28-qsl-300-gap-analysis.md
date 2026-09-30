---
id: SR-775
title: "QSL-300 gap analysis of PR 513 (FR-115 against TC-514)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-115-run-an-operation-frame-over-an-invocation.md; spec/test-cases/TC-514-run-an-operation-frame-over-an-invocation.md; spec/functional/FR-106-admit-snapshots-and-invocations.md (checks 1-11, read); spec/functional/FR-116-replay-a-frame-counterexample.md (read); spec/decisions/ADR-012-semantic-family-extension-contracts.md §12.2 (read); spec/tests.md; qsl-replay/src/spine/clause/tests/frame.rs; qsl-replay/src/spine/clause.rs; qsl-semantics/src/model/observation.rs; qsl-semantics/src/model/observation/frame.rs; qsl-eval/src/value/expression/s6a/protocol_clause.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-115
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-514
    type: reviews
---
## Summary

Ticket: QSL-300. PR: quire-spec-language#513. This review maps
each FR-115 acceptance criterion and Behavior bullet to TC-514's tests
(`qsl-replay/src/spine/clause/tests/frame.rs`) and to the code.

| AC | Test(s) | Does the test tell success from failure? |
| --- | --- | --- |
| FR-115-AC-1 | `a_frame_respecting_invocation_succeeds_with_its_provenance` | Yes. It checks stage, category, truth and exit 0. `provenance.frame` is compared with the key of the one `semantic_form == "frame"` node in the emitted wire package, matched back to the checked graph by `WireNodeId`. That identity is found independently of the code under test. The three `DocumentRef`s, digests included, are compared exactly. |
| FR-115-AC-2 | `a_change_outside_the_frame_is_a_violation_with_its_witness`, `a_creation_outside_the_frame_is_a_violation_naming_it`, `a_disagreeing_declared_delta_refuses_at_evaluate` | Yes. The witness test matches `FrameViolation` exactly. It checks code, cause, all three document refs and the population, `FieldWrite` with (`child`, `parent`), and `modifies` equal to exactly `[…/versionNumber]`. A wrong object, field, permission or document fails it. The creation test checks `Created` with `c2`, its type and an empty `creates`. The delta test checks `Evaluate(Refused)` with `population_delta_mismatch`/`delta-disagreement`, category Refusal and exit 20. |
| FR-115-AC-3 | `a_stale_package_refuses_before_evaluation`, `stale_or_mismatched_documents_refuse_at_admit` | Yes. It checks `StalePackage` with both ids. Four admit cases check the exact (code, cause) pairs and stage `Admit`, so none reaches evaluate. The byte-edit case would fail if the replace edited nothing, because then the digest would match and admission would pass. |
| FR-115-AC-4 | `an_unknown_or_unnamed_operation_refuses_at_select`, `an_absent_pre_snapshot_is_incomplete_not_a_violation` | Yes. `probe` is an operation in the step-3 domain document that the unit's clauses (`ParentOrder`, `NoCycle`, `VersionUnchanged`) do not name. So the "unnamed" case really is "exists but has no frame node", not a second missing name. The absent-pre case checks `Incomplete` with `unavailable_observation`. |
| FR-115-AC-5 | `a_frame_run_is_deterministic` | Yes. It compares the full `Debug` of both reports, usage included, and checks that admission usage is non-zero. |

Behavior bullets with no AC of their own:

- "Check 4 SHALL compare … for the model alias `M`": covered by the AC-3
  model-digest case.
- "The frame SHALL come only from the compiled package": a structural
  property. `ClauseRunSelection::Frame` has no frame or permission field.
- "the created or deleted object and its type" in the violation witness: the
  creation is tested. `FrameChange::Deleted` and `FrameChange::Retyped` are
  built by `verdict` but never reached by a test (see FND-001).

FR-116 / TC-515 scope: the PR body says TC-515 "waits on
quire-contract-ir#109 and quire-contract-codegen#49". FR-116 is assigned to
QSL-301 (QSL-21f, Backlog), so leaving it out of QSL-300 is correct. The
reason given is not accurate, though. FR-116-AC-1 drives a hand-built
envelope, and FR-116 Behavior says only "the decode from IR's witness is the
part that waits". Once this PR lands, QSL-301's QSL side is not blocked by
anything outside the repo (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-115 Behavior says the violation witness names "the created or deleted object and its type". TC-514 covers a field write and a creation, but no test reaches `FrameChange::Deleted`, so the deletion witness mapping in `verdict` is never exercised. Add a TC-514 case where post drops a non-self object (for example `c2`) and assert a `Deleted` witness with that object, its type and an empty `deletes`. | qsl-replay/src/spine/clause/tests/frame.rs; qsl-semantics/src/model/observation/frame.rs:919-923 |
| FND-002 | low | The PR body's out-of-scope note gives the wrong reason for leaving FR-116/TC-515 out. It is out of scope because QSL-301 owns it, not because of IR#109/CG#49. TC-515 uses a hand-built envelope, and only the IR witness decode waits. No repo change is needed. The lead should know that QSL-301 can start once #513 merges. | PR #513 body, "Out of scope" |

## Verdict

PASS with two low findings. Every FR-115 AC has its own test that tells
success from failure. The provenance frame identity is checked against the
real emitted `frame` node, and the violation test would fail on a wrong
witness, not only a missing one.

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | `a_deletion_outside_the_frame_is_a_violation_naming_it` asserts a `Deleted` witness naming `c2`, its type and an empty `deletes`, exit 10 |
| FND-002 | accepted-no-change | The note is in the PR body only, so nothing in the repo is wrong. FR-116 stays with QSL-301. QSL-301's QSL side is unblocked once #513 merges, and only the IR witness decode waits on IR#109/CG#49. |
