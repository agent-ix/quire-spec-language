---
id: SR-2225001
title: "Code review of PR #2225: compensation body binders lowered by checked slot names, code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@fc9952562; git diff origin/main...fc9952562 (PR #2225, branch peter/qsl-1555-sum-in-recover): qsl-semantics/src/check/protocol_obligation.rs; qsl-semantics/src/check/lowering/protocol.rs; qsl-package/src/protocol_selection.rs; qsl-package/src/emit/tests/protocol_compensation.rs; tests/it/checked_handoff.rs; examples/checked-handoff/unit-event-position.native; ticket QSL-1555"
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1555. PR: quire-spec-language#2225, head fc9952562. Method: code-review with the rust-review lane.
The checker now keeps the typer's slot names next to every compensation body, protocol capture and `each`
guard, and lowering uses them behind a prefix check (`checked_slot_names`) against the binders lowering itself
places. A duplicated `protocol_input_entries` in the activation-capture scope is removed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| type_named_body / Checker::expression | qsl-semantics/src/check/protocol_obligation.rs:1188-1376 | examined |
| CheckedCapture/CheckedRetry/CheckedRecovery/CheckedCompensation/OnEach slot names | qsl-semantics/src/check/protocol_obligation.rs:86-210 | examined |
| checked_slot_names | qsl-semantics/src/check/lowering/protocol.rs:2471-2486 | examined |
| capture_initializer | qsl-semantics/src/check/lowering/protocol.rs:1730-1751 | examined |
| each-guard, protocol captures, compensation guard/retry/recover lowering | qsl-semantics/src/check/lowering/protocol.rs:1166-1232, 1521-1700 | examined |
| until / finish / terminal `when` lowering | qsl-semantics/src/check/lowering/protocol.rs:1341-1360, 2060-2080, 2120-2140 | context_only |
| new test | qsl-package/src/emit/tests/protocol_compensation.rs:239-329 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The same root cause remains in the protocol slot bodies: `until`, `finish` and terminal `when` are still typed through `type_body` (names dropped) and lowered with slot names rebuilt from `slots.binders`. A query/`let`/aggregate binder there takes a slot past the readable binders, so it is either unnamed (`EmptyBindingName`) or, for `finish`/terminal `when` whose list is all slot binders, silently named after an unrelated protocol binder. Fix: return slot names from `protocol_slots` typing (via `type_named_body`) and lower these three sites through `checked_slot_names`, with one test. | qsl-semantics/src/check/lowering/protocol.rs:1343-1358, 2071-2077, 2132-2138; qsl-semantics/src/check/protocol_slots.rs:459 |
| FND-002 | low | Only the recovery predicate (and the activation-capture read shift) is tested. The new paths for a binder inside a trigger guard, retry relation, registration/activation capture initializer, protocol capture and `each` guard are untested; a wrong stage order at any of them would surface only as an `UncheckedProtocolFact` fault nobody exercises. Fix: extend the new test's unit with a `sum` in the trigger guard and the retry relation (cheap: same harness). | qsl-package/src/emit/tests/protocol_compensation.rs:255-329 |

## Verdict

Conditional. The fix is correct for what it targets: the prefix check turns any checker/lowering scope
disagreement into a fault instead of a wrong name, the duplicate `over` entry removal is confirmed by the test
reading `saved` at level 2, and the reordered `?`s in the `build` closure are equivalent (all return `None`).
`type_body` stays a thin wrapper, so other callers are unchanged. FND-001 is the same defect class in the
protocol slot bodies; fix it in this PR or file it against QSL-1555's parent before merge. FND-002 is optional.

Evidence: `cargo test -p qsl-package --lib protocol_compensation` at fc9952562: 4 passed, 0 failed,
including the new test. FND-001 is from reading the lowering code; it was not reproduced.
