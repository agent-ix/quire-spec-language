---
id: SR-2026111305294500000001
title: "Code review of QSL-1886 protocol captures and on-each templates in the stepper"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR 2695 head e7b13c3125a89d3045ce10dc5a0f0dc9a94ac080; qsl-eval/src/simulation/protocol.rs, qsl-eval/src/simulation/protocol/{program,compensation,roles,footprint}.rs, qsl-analyze/tests/it/protocol_roles.rs"
review_set: subset
---

## Summary

Ticket: QSL-1886. Code review with the Rust lane (rust-review) of PR 2695. The
change evaluates protocol captures once when an instance starts, freezes them
into the state key, and fills the activation binder and protocol captures into
every compensation-template frame. Each frame layout was checked against the
S3 binding order in `qsl-semantics/src/check/protocol_obligation.rs` (`Site`
construction per stage) and the `C0` order in `protocol_slots.rs`; all match.
The `trigger_read` change is sufficient to stop the wrong merge. It
over-approximates: it is broader than it needs to be, so it can create extra
states, but it never merges states that should stay apart.

## Verdict

**CONDITIONAL**: no correctness defect was found. `cargo fmt --check` fails on
the new test code (FND-001), and three lower findings remain.

Checked and clean: each stage's frame order (protocol capture `[over, A,
earlier P]`; registration capture `[forward, over, A, P, earlier R]`; trigger
guard `[trigger, P, R, over]`; activation capture `[trigger, over, P, R,
earlier Act]`; retry `[earlier, later, P, R, Act, over]`; recovery `[recovery,
P, R, Act, over]`; `until` `[over, A, P, role binder]`) agrees with the
checker. With the `NodelessBinder` refusal removed, every nodeless binder slot
(the `over` input, the activation binder, the protocol captures) has a filler.
An undecided protocol capture follows the guard path: `Started::Undecided`
pushes an undetermined finding under `on each`, and gives
`UndecidedInitialGuard` at an `on origin` initial state. It is never a silent
false. `trigger_read`: when there are templates, the trigger record stays in
the key. Protocol captures go into `observations`, and registration captures
go into the registration's control entry. Together these stop the wrong merge
that FR-207-AC-7 exposes: triggers 2 and 3 give equal `big` but different
`seen`. The `attempt_records` signature change is behaviour-preserving: it
looks up the same registration by `compensating.registration`. There are no
`/// Trace:` doc lines. Both tagged tests fail without the change, because
`Program::new` refused `Unsupported::Capture` before it.

Note for the downstream consumer: the PR removes the public `Unsupported`
variants `Capture`, `ActivationOnEachTemplate` and `NodelessBinder`.
quire-protocol `tests/tc_055_map_unsupported_and_requires_bound.rs:67-68` and
`src/captures.rs:11` still name them, so that repo must adapt when it takes
this qsl-eval. That is the expected unblock for TL-11 and TL-6, not a defect
in this PR.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `cargo fmt --check` fails on the new test code: two rustfmt diffs (an `assert!` with message, a `format!` call). The repository's fmt gate will fail on this head | qsl-analyze/tests/it/protocol_roles.rs:1907, qsl-analyze/tests/it/protocol_roles.rs:1928 |
| FND-002 | low | `trigger_read` keeps the trigger record in every state key whenever the protocol has any template, even when no registration capture reads the activation binder. That multiplies reachable states by the trigger domain with no semantic need. It is sound, but a `reads_local`-style check of the registration-capture bodies would be exact | qsl-eval/src/simulation/protocol/program.rs:832 |
| FND-003 | low | `compensation` already looks up `registration` and then calls `attempt_records`, which repeats the same lookup with the same fault string. One rule is now stated twice; passing the looked-up registration (or deriving both from `instance` once) removes the duplicate | qsl-eval/src/simulation/protocol/compensation.rs:412, qsl-eval/src/simulation/protocol/compensation.rs:503 |
| FND-004 | low | FR-207-AC-7 test: it never asserts the AC's literal `seen` values. Trigger 3's `seen` true follows only from `recovered`, and `assert_ne!` stands in for trigger 2's `seen` false. The `registered`-until-`finish` half is checked at one state only | qsl-analyze/tests/it/protocol_roles.rs:1904, qsl-analyze/tests/it/protocol_roles.rs:1916 |

## Gates

- `cargo fmt --check -p qsl-eval -p qsl-analyze`: FAIL (FND-001).
- `cargo test -p qsl-analyze --test it -- protocol_roles::` (flock, `-j2`): PASS, 29 passed and 0 failed, including the three new tests.
- `make ci` and full clippy were not run, per the dispatch brief.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | e305ec25d: `cargo fmt --check -p qsl-eval -p qsl-analyze` exits 0; both diffs are formatted |
| FND-002 | fixed | e305ec25d: `trigger_read` now asks whether any template's registration-capture body `reads_local(2)` (the activation binder's frame position after forward and `over`), instead of `!templates.is_empty()` |
| FND-003 | fixed | e305ec25d: `compensation()` passes an `AttemptOf { instance, compensating, registration }`; `attempt_records` no longer repeats the lookup |
| FND-004 | fixed | e305ec25d: the test asserts `seen` equals `{boolean true}` for trigger 3 and `{boolean false}` for trigger 2, and walks triggers 0 and 1 through `finish`, checking `registered`, `seen` false and no `cattempt` at every state |
