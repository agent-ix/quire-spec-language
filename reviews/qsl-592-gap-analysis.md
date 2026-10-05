---
id: SR-1314
title: "Gap analysis of quire-spec-language PR #638: FR-286 outcome document trace"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@5dd6bacc001141ba412f385c0562ae8ee5e6f64d; PR #638 diff against c8f0c2818; FR-286 Behavior and AC-1 to AC-4; TC-770; trace tags in qsl-replay/src/outcome.rs and qsl-replay/tests/it/outcome_facade.rs; spec/spec.md and spec/tests.md status rows"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-286
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-770
    type: reviews
---
## Summary

Ticket: QSL-592 (LC4). PR: quire-spec-language#638.

Trace map:
- FR-286-AC-1 → `check_success_document_names_its_package_identity`
  (TC-770). It runs the real `check` over `tests/fixtures/spine-compile.native`.
- FR-286-AC-2 → `check_refusal_document_carries_the_refusals_diagnostic`. It runs
  the real `check` over the `inv` source.
- FR-286-AC-3 → `analyze_items_keep_request_order_and_bytes_are_stable` and
  the facade test. Both build their items from hand-built `TerminalRecord`s.
- FR-286-AC-4 → `undefined_label_appears_only_on_a_non_proof_outcome`. The
  analyze and monitor halves use `OutcomeItem::undefined_evaluation`; the execute
  half uses a hand-built `CallOutcome::Undefined`.

All four trace tags name TC-770 and the right AC. No code in the diff is
untraced. The code has no owning requirement only where noted in FND-002.

## Verdict

Partial: 3 medium findings and 1 low. AC-1 and AC-2 are backed by real outcomes.
The analyze and monitor halves of AC-3 and AC-4 are backed only by builders,
because `qsl-analyze` has no analyze or monitor outcome type yet. That is the
right call now, but the spec status does not say so. FR-286 Behavior 1 ("each
lifecycle operation's outcome type") is met for `check` and FR-100's `run`
outcome only.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The analyze and monitor halves of AC-3 and AC-4 are verified only through public builders. AC-3 names "an `analyze` outcome" (TC-770 step 3: TC-762 step 3's outcome) and AC-4 names FR-281-AC-7's and FR-283-AC-5's outcomes. No `AnalyzeOutcome` or `MonitorOutcome` exists (qsl-analyze holds only `zone_check`). Building from builders is an honest partial, but only the PR body says so. spec/spec.md still calls FR-286 "not yet implemented", and spec/tests.md lists TC-770 as "Planned" with all four ACs. The trace tags read as full coverage. Fix: set the FR-286 and TC-770 status to partial: AC-1 and AC-2 pass, plus the item serialization of AC-3 and AC-4 over builders. Name the layer-A tickets for FR-281 and FR-283 that will re-run TC-770 steps 3 and 4 over real outcomes. | qsl-replay/src/outcome.rs:736-818; spec/spec.md:1275; spec/tests.md:984 |
| FND-002 | medium | FR-286 Behavior 1 and the ticket's "every library outcome" are met only for `check` (`from_check`) and FR-100's `CallOutcome` (`from_call`). There is no constructor for `package`, the operation that owns `package_id`, or for `replay`, `parse` or `select`. The qualified binary's verbs (check, compile, prove, replay) need at least `package` and `replay`. Without them, a driver calls `OutcomeDocument::new` and maps stage and category itself, which is the second serializer the ticket rules out. Fix: add `from_package` (this also resolves SR-1313 FND-002) and `from_replay` over `Staged<ReplayResult>`, each with a TC-770 test. | qsl-replay/src/outcome.rs:475-625 |
| FND-003 | medium | The execute document drops everything but the category. A completed call's value, an undefined call's reason (`sum-out-of-domain`) and an incomplete call's limit are all lost. The AC-4 execute test passes only because AC-4 asks for nothing else. The cause is a spec gap (SR-1315 FND-001), so a code fix waits on the spec. Recorded here because no test can show the execute document is enough to replace FR-100's output. | qsl-replay/src/outcome.rs:603-619, 811-817 |
| FND-004 | low | `from_call` takes FR-100's `CallOutcome`, the `run` mapping, not the `execute` operation's typed result `Result<Evaluation<Value>, CallFailure>` (ADR-029 LC-2). `CallFailure::Cancelled` and an S6a fault have no route into a document. Fix: once `execute` is public, add a constructor over its result, with cancelled as category incomplete. | qsl-replay/src/outcome.rs:599-619 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 17619203d (FR-286 `## Status` says Partial and names QSL-596 for FR-281 and QSL-597 for FR-283; spec/spec.md and spec/tests.md mark TC-770 partial; the TC-770 procedure says builders until those tickets land) |
| FND-002 | fixed | 17619203d (`from_package` and `from_replay` added, with TC-770 tests `check_and_package_documents_over_the_fixture` and `replay_documents_follow_the_arm_and_the_refusal`) |
| FND-003 | fixed | 9e42b2c56 (the `result` member carries a completed value, an undefined reason and an exhausted limit, built in 4cf691c7f, f4043f474 and 9e42b2c56; FR-286-AC-5 is tested over real `run` outcomes for `seven`, and for `seven` with `work_units` 0) |
| FND-004 | deferred | `execute` is still `pub(crate)` (qsl-replay/src/spine/lifecycle.rs:589), so there is no public `Result<Evaluation, CallFailure>` for a driver to hand the serializer. The constructor belongs with the change that makes `execute` public; FR-100's `CallOutcome` is the public route today. |
