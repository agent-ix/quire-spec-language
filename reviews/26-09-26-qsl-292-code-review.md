---
id: SR-748
title: "PR 494 code review (QSL-292)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; qsl-replay/src/spine/call.rs; src/command/output.rs; src/command/output/types.rs; qsl-foundation/src/diagnostic.rs (kernel_refusal_record); quire-exact/src/outcome.rs (Refusal::code, Refusal::cause); qsl-eval/src/value/expression/evaluate.rs (Evaluation::refusal_record)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: reviews
---
## Summary

Ticket: QSL-292. PR: quire-spec-language#494. Code review with the rust-review lane folded in.

`CallRefusal::Kernel` is truly unreachable, so deleting it is correct:

- `Refusal::code()` and `Refusal::cause()` return `None` only for `CheckedInvariant` (quire-exact/src/outcome.rs:229-273).
- `kernel_refusal_record` returns a record for every other variant. Its match is exhaustive under `deny(wildcard_enum_match_arm)` (qsl-foundation/src/diagnostic.rs:946-1016).
- `Evaluation::refusal_record` sends every `Outcome::Refused` to `kernel_refusal_record` (qsl-eval/src/value/expression/evaluate.rs:84-100).
- `convert_outcome` catches `CheckedInvariant` before the generic `Outcome::Refused(_)` arm (qsl-replay/src/spine/call.rs:550-555). The family arm always passes `Some(fallback)` (call.rs:562-569).

So `record == None && fallback == None` cannot occur. The new branch returns a typed `InternalFault` (`call`/`kernel-refusal-with-no-record`), which renders through the existing `runtime_invariant` path at exit 30. It does not panic. Stage `call` matches the sibling unreachable faults (`spine-run-supplies-admitted-name-and-arity`, `boolean-or-integer-function-completes-that-kind`). FR-100 "Internal failure at S6a" does not list any of these by name, so this one is consistent with that precedent.

Gates run on this head: `cargo test -p qsl-replay spine` (32 passed), `cargo test -p quire-spec-language --lib output` (8 passed), and `cargo clippy -p qsl-replay -p quire-spec-language --all-targets -- -D warnings` (clean).

## Verdict

**Approve with two low findings.** The change is correct and does not panic.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new no-record fault branch has no test. Its sibling unreachable fault in `convert_call_failure` has a direct unit test (tests.rs:681-713). Add one: `convert_refusal(None, None, None, &[])` gives `RunRefusal::Fault` with stage `call` and invariant `kernel-refusal-with-no-record`. | qsl-replay/src/spine/call.rs:513-518; qsl-replay/src/spine/call/tests.rs:681 |
| FND-002 | low | `SpineOutcome::Refused` still types `code` and `cause` as `Option` with `skip_serializing_if`, but every remaining arm sets `Some`. FR-100's refusal table requires both members on every refused outcome. A future arm could silently emit a bare `{"kind":"refused"}`, the shape this PR removes. Make both fields plain `&'static str`. | src/command/output/types.rs:259-262; src/command/output.rs:428-429, 439-440 |

## Dispositions

Round 2, reviewed after the fix commit. Gates run: `cargo test -p qsl-replay` (97 passed), `cargo test -p quire-spec-language --lib output` (8 passed), `cargo test -p quire-spec-language --test it family_outcome_layering` (5 passed), and `cargo clippy -p qsl-replay -p quire-spec-language --all-targets -- -D warnings` (clean).

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | the new test `convert_refusal_with_no_record_and_no_fallback_is_a_typed_fault` asserts stage `call` and invariant `kernel-refusal-with-no-record` (qsl-replay/src/spine/call/tests.rs:931-945, fn at :937) |
| FND-002 | fixed | `SpineOutcome::Refused` now has plain `code: &'static str` and `cause: &'static str`, and the type's doc comment is rewritten to match (src/command/output/types.rs:247-262; src/command/output.rs:428-429, 438-439) |
