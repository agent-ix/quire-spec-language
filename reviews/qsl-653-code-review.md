---
id: SR-2442
title: "Code review of quire-spec-language PR #663 (QSL-653): OutcomeDocument::from_run and spine::run's caller Cancel"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@2f0b659461cfc40a8d31e3efa09d32032e4d42e8; PR #663 diff against origin/main bfeb258c (merge base b8227ba7); qsl-replay/src/outcome.rs, qsl-replay/src/spine/call.rs, qsl-replay/src/spine/call/tests.rs, src/command.rs, src/command/output.rs, tests/it/spine_run.rs; read for context: qsl-replay/src/spine/lifecycle.rs execute, qsl-replay/src/spine/lifecycle/tests.rs Gate harness, qsl-foundation/src/diagnostic.rs Code, quire-exact Cancel; checked against quire-specification main fbd6ea0 FR-271, FR-300, FR-301, TC-221"
review_set: subset
---
# Code review of quire-spec-language PR #663

## Summary

Ticket: QSL-653. Rust lane (`rust-review`) folded in. The PR passes the caller's `&Cancel` through `spine::run`, adds `RunRefusal::Cancelled(CancelCause)`, and adds `OutcomeDocument::from_run`, which covers completed, undefined, call-refused, compile-refused, S6a-refused, faulted and cancelled runs. The cancelled outcome is incomplete with exit 22. An unsupported driver engine is written with `unsupported_construct` (exit 21).

Examined:
- qsl-replay/src/outcome.rs `from_run`, `cancelled`, `faulted`, and the tests `from_run_documents_every_arm` and `a_driver_side_unsupported_engine_document_exits_21` (examined)
- qsl-replay/src/spine/call.rs `RunRefusal::Cancelled`, `stage`, `code`, `category`, `run`, `convert_call_failure` (examined)
- qsl-replay/src/spine/lifecycle.rs `execute`: cancel checked on entry, a `Meter::with_cancel` on every S6a charge, and `tripped()` checked after the call (context_only)
- src/command.rs `run_complete`, src/command/output.rs `error` (examined)
- the test call-site updates in spine/call/tests.rs and tests/it/spine_run.rs (examined)

## Verdict

**CONDITIONAL**. There are medium and low findings and no high ones.

`from_run` matches every `RunRefusal` variant by name, with no wildcard. Its `Ok` arm delegates to `from_call`, which matches all four `CallOutcome` variants exhaustively, so a new variant breaks the build rather than being silently absorbed. The categories and exit codes agree with QSpec FR-301 and TC-221 CL-02: incomplete 22, unsupported 21, refusal 20, internal failure 30.

Cancellation is checked at real points, not only on entry. Each front-end stage receives the handle and checks it at its own charges. S6a runs under `Meter::with_cancel`. `lifecycle::execute` checks `tripped()` after the call. The behavioural test, however, only covers a handle that was already cancelled (FND-001).

The unsupported-engine code reuses `unsupported_construct`, but QSpec's catalog defines `unimplemented_capability` for this case (FND-002).

Merge note: the branch is one commit behind main (bfeb258c), and `spec/tests.md` conflicts at the TC-770 row because QSL-651 rewrote the status column. To resolve it, take main's row and add FR-286-AC-6.

Gates at 2f0b6594 are recorded in the reviewer's Linear comment.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The mid-run cancel path is untested. The only cancelled case cancels the handle before `run` starts, so it stops at S1's first check. The S6a mapping at call.rs:389, `CallFailure::Cancelled` to `RunRefusal::Cancelled`, has no test. If that arm were deleted, `convert_call_failure` would turn a mid-call cancel into `RunRefusal::Fault("cancelled-without-a-shared-handle")`: internal failure, exit 30 instead of 22. Every test would still pass. FR-100 now claims "a cancel at any stage or during the call", and the `Cancel::observing` Gate harness in spine/lifecycle/tests.rs can trip the handle deterministically at a chosen S6a charge. `CancelCause::Deadline` is also never exercised through `from_run`. | qsl-replay/src/spine/call.rs:389; qsl-replay/src/outcome.rs:1543-1555 |
| FND-002 | medium | The driver's unsupported engine (AOT or JIT requested, interpreter only) is written with `unsupported_construct`. QSpec FR-271 defines that code as "a recognized form prohibited by the exact selected profile", which is a source form. The catalog has `unimplemented_capability` ("valid complete-V1 meaning is not implemented by the selected producer; distinct from prohibited source and unsupported backend projection") for exactly this case. QSL's `Code` enum lacks that variant. The ticket said to add a code when the catalog has none for this case. The category and exit (unsupported, 21) are correct; the code a consumer reads is the wrong catalog entry. | qsl-replay/src/outcome.rs:1563-1576; spec/functional/FR-286-serialize-every-outcome-as-one-json-outcome-document.md:125-128 |
| FND-003 | low | `a_driver_side_unsupported_engine_document_exits_21` builds a document with `Category::Unsupported` and asserts that it reads back as unsupported with exit 21. It exercises no code this PR adds, and its two real assertions, `exit_code(Unsupported) == 21` and `UnsupportedConstruct.category()`, are already TC-769's. It documents the driver's usage; it does not test it. | qsl-replay/src/outcome.rs:1563-1576 |
| FND-004 | low | `RunRefusal::stage()` returns `"call"` for `Cancelled`, even when the cancel stopped S1 to S4. Its doc says it returns "the stage FR-100's refusal table names". A CLI or driver envelope that reads `stage()` would report a parse-time cancel as a call-stage stop. `from_run` itself writes `last_stage` null, so the two surfaces disagree. | qsl-replay/src/spine/call.rs:283-293 |
