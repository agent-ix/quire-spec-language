---
id: SR-821
title: "QSL-331 code review (with rust-review lane) of PR 538"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5aee4879c307df1e94dddbe85af5c474b176889d; qsl-replay/src/execute.rs; qsl-replay/src/execute/frame.rs; qsl-replay/src/spine/clause/tests/frame_replay.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
---
## Summary

Ticket: QSL-331. PR: quire-spec-language#538 at 5aee4879, base c7fd631d.
origin/main is now bad4944c (#537). `git merge-tree` shows spec/tests.md
changed on both sides in different hunks (main edits the TC-463/TC-469
rows and a later paragraph, this PR the TC-515 row), so it merges without
conflict. No other file overlaps. Methods: code-review with the
rust-review lane folded in.

Checks run:

1. Ordering. `check_envelope` runs at frame.rs:197, after
   `ReplayRequest::decode` and before `recompile`. It compares
   `clause_node` with `payload.frame` first, then `occurrence_key` with
   `payload.occurrence`, and returns `EnvelopeFrame`/`EnvelopeOccurrence`
   with `{envelope, payload}` through `ReplayRefusal::FrameIdentity`, whose
   Display starts `stale_dependency/revision-mismatch`.
2. Test oracle. `uncompilable_request` pairs the ConfigVersion
   `package_id` with source bytes that do not parse. The clause-node test
   first replays a consistent envelope over the same request and asserts
   `ReplayRefusal::Recompile`, which proves the request decodes and the
   recompile fails. So a `FrameIdentity` refusal on the stale envelope can
   only come from a check that runs before the recompile.
3. Mutations (run by this reviewer, reverted, tree clean afterwards).
   M1: `check_envelope` moved after `recompile`: both new tests FAIL
   (exit 101). M2: the occurrence branch disabled: the occurrence test
   FAILS (exit 101). Both mutants are killed.
4. rust-review lane. No unwrap, panic or unsafe in production code. The
   `Box` matches the existing `FrameIdentity(Box<_>)` variant. The
   `OccurrenceKey` clone happens only on the error path. The docs on
   `replay_frame`, `ReplayRefusal::FrameIdentity` and the module header
   match the code. `cargo fmt --check` passes. The tests carry
   `#[trace("TC-515", "FR-116-AC-6")]`.

## Verdict

The change is correct, small and in the right place. The ordering is
proved by a real oracle, not asserted. One low finding: the occurrence
test checks the typed fields but not the rendered message.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The occurrence test asserts the typed `EnvelopeOccurrence{envelope, payload}` and the code, but not that the rendered refusal names both occurrences, as the clause-node test does for frame nodes (lines 694-697). If the `#[error]` string on `EnvelopeOccurrence` (frame.rs:61) dropped `{payload:?}`, the test would still pass, although FR-116-AC-6 requires the refusal to name both. Fix: assert that the message contains both `{:?}` renderings. | qsl-replay/src/spine/clause/tests/frame_replay.rs:733; qsl-replay/src/execute/frame.rs:61 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | bfe0fcb1 |
