---
id: SR-1252
title: "Code review of quire-spec-language PR #612: a precondition selected by Invocation reads only the pre side (PC1)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@5402fab068aaf55dc893edf0d9bbd329f30410fd; PR #612 diff against origin/main (merge base c54e3595): qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-eval/src/value/expression/mod.rs, qsl-replay/src/execute/state_clause.rs, qsl-replay/src/spine/clause.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/pre_call_invocation.rs, qsl-semantics/tests/it/state_clauses.rs, spec/functional/FR-106-admit-snapshots-and-invocations.md, spec/test-cases/TC-465-admission-refuses-each-input-defect.md, and the 36 further files the code commit 2bbcd902 changes (FND-001)"
review_set: subset
---
# Code review of quire-spec-language PR #612

## Summary

Ticket: QSL-505 (slice PC1). Reviewed head 5402fab0 (code head 2bbcd902,
last commit spec-only). Rust lane (rust-review) folded into this file.

The PR has two parts. One is the QSL-505 change, in nine files. The other is
an undeclared revert of #610 (QSL-590), in 36 more files (FND-001).

**The branch reverts #610.** `git diff origin/main...HEAD` lists 47 files, but
the PR describes nine. The code commit's parent is c54e3595, the #610 merge.
Its tree is main before #610 (9246fa38) plus the QSL-505 edits: 38 of the 41
files #610 changed are byte-identical to 9246fa38 at 2bbcd902, and the other
three (`qsl-eval/src/value/expression/mod.rs`,
`qsl-replay/src/execute/state_clause.rs`,
`qsl-replay/src/spine/clause/tests.rs`) differ from 9246fa38 only by QSL-505
hunks. `git diff 9246fa38 5402fab0 --stat` gives exactly the nine intended
files. The gate log (`pc1-make-ci.log`) shows exit 0 and runs zero `tc_769`
or `tc_786` tests, because they no longer exist on this head.

**The QSL-505 change itself is correct.** Checked against the brief:

- Admission. `admit_observations` sends an `Invocation`-selected precondition
  to `admit_pre_call_invocation`, which reads the invocation as
  `DocumentKind::InvocationCall` and then its `pre` snapshot, and nothing
  else. It hands both to `admit_pre_call`, the same function a `PreCall`
  selection uses. That function runs check 3 on the pre snapshot, check 4 on
  the invocation's model and then the snapshot's, check 5, checks 6 to 10 on
  pre, and no check 11. It returns `post: None`, `result: None` and an empty
  delta. `admit_operation` now serves only postconditions and passes
  `post_self = true`. `admit_frame_invocation` still passes `false`.
- Reader split. `read_invocation_body` calls `read_invocation_call` and then
  reads `post`, `result`, `created` and `deleted`. There is one read path for
  the call half, and no compatibility reader. `InvocationCall` requires the
  eight call members and allows all twelve, so a post-side member is neither
  required nor unknown (FR-106 check 1.5 and 1.6).
- S6a evaluator. `ClauseSetup` reads `observations.pre` for
  `Observation::Pre`, where it used to read `post.or(pre)`. `own` is now
  `current.identity`, which is the pre identity for a precondition, so the
  separation trail is unchanged.
- Documents-read reporting. `admitted_documents`
  (qsl-replay/src/spine/clause.rs:495) flattens the admitted
  current, pre and post observations. For this path it gives
  `[invocation, pre]` with no code change. The replay result
  (`StateClauseReplayResult::documents`) takes the report's list. Replay's
  `expected_form` sends a precondition counterexample through `PreCall`, so
  replay never re-reads a post side for a precondition.
- No other consumer of `AdmittedObservations` reads `post` for a
  precondition. The root crate's `src/runtime/validation` (FR-007) still
  resolves both snapshots for any `Invocation` selection, but that is a
  separate requirement and input model, and it is outside this diff. FR-007's
  own text ("A recorded operation supplies both pre and post for frame/delta
  validation, including when selecting a precondition") matches its code.
- Moved check-11 tests. The four `attempt_update_refuses_*` tests now select
  `AttemptUpdatePost`. In each one `root` (self) is present in post, and the
  frame fixture's result is valid for `attemptUpdate` (the postcondition
  positive test admits over the same fixture). So check 9 and check 10 pass,
  and check 11 is still the first failing check, with the same code, cause
  and object as before.
- Ruling applied. TC-465 row 38 selects the postcondition `ProbeHolds`, and
  FR-106-AC-3 names "a result value on an invocation selected for a
  postcondition on `probe`". No part of the PR's spec or code says a
  precondition selected by `Invocation` reads `result`.

**Test oracle.** `a_precondition_over_an_invocation_reads_only_the_pre_side`
fails under each wrong implementation. If the evaluator read post,
`present(self.parent)` would be `false`, not the `PreCall` `true`. If
admission ran check 11, the run would refuse `frame_violation`, and
`boolean_disposition` panics on any non-Boolean disposition. If admission
read the post snapshot, `provenance.documents` would not equal
`[invocation, pre]`. The missing-post and malformed-post repeats would refuse
`unavailable_observation` or `byte-digest-mismatch` under the old path.
Reverting only the evaluator hunk to `post.or(pre)` passes every test, but
that hunk is unreachable once admission returns `post: None` for every
precondition, so this is not a gap.

Rust lane: no new `unwrap`, `expect`, panic or `unsafe` on a production
path. There is no wildcard arm. The `StateClauseKind::Invariant` arm under
`Invocation` returns an internal fault after check 2 has already refused that
pair, and that is the right shape for an exhaustive match. No new limit,
depth cap, pin, digest, compatibility layer or ceremony. Gates were not
re-run for this review (reading only, per the brief).

## Verdict

Changes requested. FND-001 (high) blocks the merge: merging lands an unseen
revert of QSL-590. The QSL-505 change itself is sound. It leaves two low
findings.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The code commit 2bbcd902 reverts #610 (QSL-590, merged as c54e3595, this PR's merge base). Its tree is 9246fa38 plus the QSL-505 edits. Merging deletes `Code::category`, `Category::exit_code`, `Category::trace_exit_code`, `Category::most_severe` and `CallFailure::category`, re-adds `Code::exit_code`, `ProofCategory` and `ClauseRunReport::exit_code`, deletes the TC-769 (FR-285-AC-1 to AC-4) and TC-786 (FR-100-AC-10, AC-11) tests, reverts the FR-100, FR-109, FR-285, TC-452, spec.md and tests.md edits, and deletes reviews/qsl-590-code-review.md and reviews/qsl-590-gap-analysis.md (SR-1250, SR-1251). The PR body does not mention any of this. make ci passes because the reverted tree is self-consistent, and GitHub reports the PR MERGEABLE. Fix: rebuild the branch from current origin/main (652ae3d5) with only the QSL-505 change, for example `git diff 9246fa38 5402fab0 \| git apply --3way` on a fresh branch from main. Then check that `git diff --stat origin/main...HEAD` lists only the nine intended files, and rerun make ci. | qsl-foundation/src/diagnostic.rs; qsl-eval/src/value/expression/mod.rs:92; qsl-replay/src/spine/clause.rs:517; qsl-replay/src/proof_result.rs; src/main.rs; tests/it/spine_run.rs; reviews/qsl-590-code-review.md; reviews/qsl-590-gap-analysis.md |
| FND-002 | low | The invocation's call half is passed twice. `admit_pre_call_invocation` extracts `call` with `as_invocation_call()`, then passes both the whole `ReadDocument` and `call.self_object`/`call.parameters` in `PreCallInput`. `admit_pre_call` extracts the call again, with its own fault. `ReadDocument::as_invocation_call` also has a `Body::Invocation` arm that no caller uses. Fix: give `PreCallInput` `call: Option<&InvocationCall>` and the invocation's read usage, instead of the `ReadDocument`, and drop the unused `Body::Invocation` arm. | qsl-semantics/src/model/observation.rs:1146-1151; qsl-semantics/src/model/observation.rs:1170-1182; qsl-semantics/src/model/observation.rs:1201-1207; qsl-semantics/src/model/observation/document.rs:166-174 |
| FND-003 | low | Two doc comments are stale. `admitted_documents` still says it returns "the invocation and then its pre and post snapshots". For a precondition it returns the invocation and pre only, and the PR updated the matching comment in state_clause.rs but not this one. `PreCallContext` is documented as "What a `PreCall` admission reads", but it now serves the `Invocation` path too. Fix: reword both comments. | qsl-replay/src/spine/clause.rs:492-494; qsl-semantics/src/model/observation.rs:1117 |
