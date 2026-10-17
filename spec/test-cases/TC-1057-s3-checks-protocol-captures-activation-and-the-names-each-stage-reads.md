---
id: TC-1057
title: "S3 checks a protocol's activation and captures and the names each stage may read"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: verifies
---
# TC-1057: S3 checks a protocol's activation and captures and the names each stage may read

## Description

Verify that S3 checks `activation`, the protocol's captures and a compensation's
registration and activation captures through the clause checker over the names
each stage may read, and refuses a read of a name outside its stage, a capture
that reads itself or a capture written later, and a capture of the wrong type or
without a total initializer. Scope: FR-381-AC-1 to FR-381-AC-4.

Catches a registration capture that reads a recovery binder, an activation
guard that reads the forward record, a retry relation that reads the trigger, a
capture graph with a self, later or other-template read that checks, a
dependency cycle among captures that is reported twice or not at all, and an
activation guard that reads model state.

Fixture **R**: a domain package `M` with `Total = Int[0, 1000]` and `Tally =
Int[0, 3]`; object types `Node{total: Total, tally: Tally, n: Int[0, 9]}`,
`Plain{ready: Boolean}` and `Tick{ready: Boolean, count: Int[0, 9]}`; the
operation `Node::step`; model alias `M`, profile aliases `P` and `T`. The base
protocol `Refunds`:

```
protocol Refunds using P over (view: M::Node)
    on each (opened: M::Tick) when (opened.ready) {
  capture order: M::Total = view.total;
  role Service on M::Node;
  compensate Refund for Main::Applied as (forward: M::Node)
      by Service on M::Node::step using T clock "refund-clock" {
    capture target: M::Total = forward.total;
    activate first (trigger: M::Plain) when { not trigger.ready and target >= 0 } {
      capture activated: Boolean = trigger.ready;
    }
    within [0,30]; attempts 3 of M::Node;
    retry (earlier: M::Node, later: M::Node) {
      earlier.tally < 3 and later.tally = earlier.tally + 1
      and target >= 0 and not activated
    };
    commit Main::Committed;
    recover (recovered: M::Node) { recovered.n = 1 and target >= 0 and not activated };
  }
  run sequence Main {
    attempt Tried by Service on M::Node::step contracts []
        as (attempted: M::Plain) { attempted.ready };
    effect Applied of Main::Tried as (applied: M::Node) { order >= 0 };
    commit Committed by Service as (committed: M::Plain) { committed.ready };
  }
  finish Closed as (closed: M::Node) { true };
}
```

## Test Procedure

Check one unit per row, the base protocol with the stated change, and assert the
checked clause or the refusal's code, cause, span text and retained members.
Each refusal in rows 2 to 5 and 7 to 12 and 19 is `missing_declaration`/`missing-name` at the
named read, retaining `name`, `stage`, `owner` and `read`.

| # | Change | Expected |
| --- | --- | --- |
| 1 | none | checks; the clause holds activation `OnEach{opened, M::Tick}`, the protocol capture `order`, the compensation `Refund` with registration capture `target` and activation capture `activated`, each with its stage and owner |
| 2 | `capture target: M::Total = recovered.total;` | at `recovered`: `stage` `registration-capture`, `owner` `Refund`, `read` `stage` |
| 3 | the guard `when { target >= 0 and forward.total = target }` | at `forward`: `stage` `trigger-guard`, `read` `stage` |
| 4 | the retry relation reads `trigger.ready` | at `trigger`: `stage` `retry`, `read` `stage` |
| 5 | the recovery predicate reads `earlier.tally` | at `earlier`: `stage` `recovery`, `read` `stage` |
| 6 | the retry relation reads `view.n` | `ill_typed`/`operator-ineligible` at `view.n`, naming the retry stage (model state takes precedence over the stage table) |
| 7 | `capture order: M::Total = target;` | at `target`: `stage` `protocol-capture`, `owner` the protocol, `read` `foreign` |
| 8 | the registration capture `capture target: M::Total = target;` | at the `target` inside the initializer: `read` `forward` |
| 9 | the registration captures `capture a: Boolean = b;` then `capture b: Boolean = true;` | at `b`: `read` `forward` |
| 10 | the registration captures `capture a: Boolean = true;` then `capture b: Boolean = a;`, and `capture x: Boolean = y;` then `capture y: Boolean = x;` | the first pair checks; the second gives exactly one refusal, at `y` in `x`'s initializer, `read` `forward`; `y` checks |
| 11 | a second template `Other` for `Main::Applied` whose registration capture reads `target` of `Refund` | at `target`: `owner` `Other`, `read` `foreign` |
| 12 | the guard reads `ghost.ready` | at `ghost`: `read` `unbound` |
| 13 | `capture order: M::Absent = view.total;` | `missing_declaration`/`missing-name` at `Absent`; `capture` `order`, `name` `Absent` |
| 14 | `capture order: Boolean = view.total;` | `ill_typed`/`type-mismatch` at `view.total`; `capture` `order`, `expected` `Boolean`, `actual` `M::Total` |
| 15 | `capture bumped: M::Tally = view.tally + 1;` | `undefined_expression`/`unproved-range` at `view.tally + 1` |
| 16 | `capture forward: Boolean = true;` inside `Refund` | `ambiguous_declaration`/`ambiguous-name` at the capture, naming the parameter (FR-113-AC-5) |
| 17 | `on each (opened: M::Absent)` | `missing_declaration`/`missing-name` at `Absent` |
| 18 | `when (opened.ready and view.total > 0)` | `ill_typed`/`operator-ineligible` at `view.total`, naming the guard |
| 19 | `when (order >= 0)` | at `order`: `stage` `protocol-activation`, `read` `stage` |
| 20 | `when (opened.count)` | `ill_typed`/`non-boolean-root` at the guard |
| 21 | `on origin` replacing the `on each` header, and the base `order` capture | checks; the clause holds activation `Origin`, no binder, no guard |
| 22 | `on each (opened: M::Tick)` with no `when` | checks; the guard is the always-true guard |

## Expected Results

Each stage reads the names its row of the table allows and no others: a forward
capture, another stage's binder, another template's capture and an undeclared
name each refuse at the read with the `read` the row names, a capture cycle
cannot be written and is reported once as a forward read, and the checked clause
holds each capture with its stage and owner.
