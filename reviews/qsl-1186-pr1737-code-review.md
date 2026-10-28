---
id: SR-1737001
title: "code review (with rust-review) of PR #1737 (QSL-1186b): choice and repeat to V2"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1737 git diff origin/qsl-1186a-parallel...09b0d42e3 (10 files); qsl-semantics check/protocol_slots.rs, check/lowering/protocol.rs, check/lowering/protocol/controls.rs, check/mod.rs, check/refusal.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1737001: code review of PR #1737 (QSL-1186b)

## Summary

Ticket: QSL-1186. Head reviewed: 09b0d42e3, a single commit on qsl-1186a-parallel (7cd3f46f6), 10 files.

- **post_test fix (soundness).** The fix is right, and the new reading is FR-218's. FR-218 says "Where the guard reads no binder declared in that repeat's body, the repeat SHALL retain pre-test semantics". The old `body_binders` was `Out(body)`, which includes every entry binder, so any guard that read an entry binder was classed as post-test. The new `Out(body) \ In(repeat)` is exactly the body-declared set.
  - Models that flip are repeats whose guard reads only entry binders, for example `tests/fixtures/composed-choreography.native` `Retry` (`view.pending`) and `protocol_obligation/tests/recorded.rs:434` `Again` (`attempted.ready`).
  - Before the fix, qsl-eval's settle ran their body once unconditionally (post-test). Now the guard is evaluated on entry.
  - Before the fix, `max 0` on such a repeat refused `RepeatMaxConflict`. Now it is admitted.
  - Probes confirm all of this:
    - An entry guard gives post_test=false.
    - A body guard gives true.
    - A mixed guard gives true.
    - Entry guard with `max 0` is admitted. In qsl-package it emits, and IR reads it back Verified.
    - Body guard with `max 0` refuses RepeatMaxConflict.
  - No existing test changes outcome.
- **Choice.**
  - Cases are sorted by label bytes, and there is no case node.
  - A guard that reads its own case's binder, or a sibling case's binder, refuses MissingName.
  - Duplicate labels refuse Ambiguous.
  - A capture as a bare `visible` entry refuses VisibleEntryNotAnExpression, and `not flag` emits.
- **Repeat.**
  - `max 0` pre-test emits.
  - `max` without `exhausted` is a parse error.
  - A duplicate `visible` entry emits one set member and reads back.
  - `repeat_steps_through_repeat` matches FR-464 "Progress".
- **Defect (FND-001).** A repeat whose `visible` entry, `invariant` or `variant` reads a body-declared binder, while its guard does not, passes S3 with post_test=false. Its lowering over `In(repeat)` then misses the binder in `Binders::find`. It refuses `NodePreimage(EmptyBindingName)`, a key-fault cause, where it should name the STD-493 construct. All three probes reproduce this.

Gates at 09b0d42e3 (locked-build, jobs 4, `--all-features`, debug info off, QSPEC_DIR=~/dev/quire-specification):
- qsl-semantics: lib 805 + it 1328 + doc 30, all passing.
- qsl-package lib: 381 passed, with 2 pre-existing failures (conformance_c14 and i2, QSpec #468 MalformedWire).
- qsl-eval: 783 + 138 + 13, all passing.
- qsl-analyze `repeat`: 8/8.
- Root `native_choice`/`native_payment`/`native_protocol_emission`/`composed`: 284 passed, with 1 pre-existing failure (`domain_operation_parameters_and_result_bind_and_type_check`).
- `cargo clippy -p qsl-semantics -p qsl-package --all-targets -D warnings` and fmt: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A repeat `visible`/invariant/variant that reads a body binder (guard does not) passes S3, then lowering refuses the mis-caused `NodePreimage(EmptyBindingName)` instead of a typed STD-493 not-written construct | FR-218, FR-466 Binder scope |

## Verdict

Changes requested: FND-001.

## Dispositions

Round 1, head 8be8961f2: one commit restacked onto the rebuilt 1186a (98b750b7b). I reviewed the whole `git diff 98b750b7b 8be8961f2`, including the adaptation to main's #1714 (QSL-1131), as new code.

**The #1714 adaptation:**
- `body_roots()` now also returns every `choice` `visible` entry and case guard, and every `repeat` `visible` entry, guard, invariant and variant.
- So `dependencies()` lists the functions those slots call, keyed before the protocol. `a_decision_that_calls_a_function_reads_back` checks that a guard names the call's node and that IR admits the package.
- So `ProtocolReferenceInBody` now also covers decision slots. `decisions_without_a_node_form_are_not_written` checks a `while { protocol Other }` guard.
- The `ControlReadsDeclaration` decision path is gone, as the dependency now carries the call. This is correct.
- Each decision slot gets its own `ProtocolSite::Decision(path, span)`, registered in `register_protocol_sites`. Refusals and occurrences inside a decision therefore locate at that slot's span.

**Other changes I checked:**
- `scoped_body` lowers decisions over the binders at the repeat's or choice's entry, as FR-466 "Binder scope" requires.
- `visible_entries` keeps only reference leaves; a literal or bare binder was refused earlier, as `VisibleEntryNotAnExpression`.
- `repeat_steps_through_repeat` follows FR-464 "Progress". A sequence steps if any child steps, a choice only if every case steps, and a check or a repeat never steps.

One consequence outside this PR: with #1714, a protocol body may call a function that is keyed before the protocol. The predicate-join deferral that #1727 recorded (QSL-1186 comment 5949da6c: "function nodes are written after the protocols") may no longer hold. The QSL-1186 planner should re-check it.

Gates (locked-build, jobs 4, debug info off, `--all-features`, QSPEC_DIR 48373530):
- qsl-semantics: lib 810, it 1334, doc 30.
- qsl-package lib: 397 pass, 2 ignored, 0 fail. c14/i2 now pass.
- qsl-eval: lib 805, it 140.
- qsl-analyze `repeat`: 9/9.
- Workspace clippy `-D warnings`: exit 0.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `CheckedRepeat.decisions_read_body` is set when a `visible` entry, the invariant or the variant reads a binder in `Out(body) \ In(repeat)`. Such a repeat that is not post-test refuses `unsupported_construct`/`not-yet-implemented` naming `RepeatDecisionReadsBody` (STD-493), before lowering, in place of `NodePreimage(EmptyBindingName)`. `decisions_without_a_node_form_are_not_written` covers the three cases: `visible (got.ok)` with guard `true`, `invariant { got.ok }`, and `variant { if got.ok then 1 else 0 }`. A post-test repeat still refuses only `PostTestRepeat`, so the guard alone decides FR-218's post-test semantics. |
