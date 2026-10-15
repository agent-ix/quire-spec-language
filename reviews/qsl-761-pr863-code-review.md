---
id: SR-11253
title: "Code review — QSL-761 clause receiver path at declared object identity (PR 863)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4a0703e794b64329ba64e9a6c1468d23f059be41; qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/expression/s6a/separation.rs, qsl-eval/tests/it/tc235_business_meaning_runtime.rs, qsl-semantics/tests/it/tc235_business_meanings.rs"
review_set: subset
---

## Summary

Ticket: QSL-761. Code review with the rust-review lane over the four-file diff of PR 863. `ClauseSetup::of` now always builds the clause receiver path at the clause's own observation: rooted at the invocation receiver binding when `receiver_invocation` is set (state-machine contexts only), otherwise at the declared object identity `self_object`, with no steps. `ClauseEvaluation::receiver_path()` returns `&RuntimeValuePath` instead of `Option`.

## Verdict

**PASS.** Consumer sweep: `receiver_path()` has no production consumer in this repo or any sibling checkout under ~/dev; its only callers are the TC-235 M15 and M16 tests. The engine's own provenance path (`trail.receiver_invocation`, evaluate.rs) is untouched, so violation reports, replay result projections and golden files are unaffected; no fixture or expected output changed. The invocation-rooted path for a state-machine receiver (M15) is unchanged: `receiver_invocation` is still set only when `is_state_machine_context` holds. The object-rooted path matches TC-235 M16 run 1 (root `(s1, object (universe of Fulfilment, Fulfilment, r2))`, no steps) and FR-207-AC-8 (the receiver binding is never used for a receiver with a declared object identity). No new unwrap/expect/panic in non-test code. Trace tags are unchanged, so nothing overclaims. The new M16 assertion fails if the path is absent, invocation-rooted, rooted at another run, or at another observation; the M07 assertion compares against an independently parsed literal and fails on a wrong or missing member. The PR body states the blocked items (M21 run 2, M16 run 3, M22, UUID form) honestly.

Low notes, not defects of this PR: the M16 test now asserts part of FR-207-AC-8 but does not carry its tag (under-claim); the PR body says no ticket exists for the blocked items although QSL-761 itself enumerates them, and does not mention ticket items 6-10.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
