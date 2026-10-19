---
id: TC-1058
title: "S3 checks a compensation's registration, attempt bound, retry relation, commit and recovery"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: verifies
---
# TC-1058: S3 checks a compensation's registration, attempt bound, retry relation, commit and recovery

## Description

Verify that S3 checks each `compensate` template's forward effect, role,
operation, attempt bound and type, deadline, retry relation, commit boundary and
recovery predicate, keeps distinct templates distinct, and refuses each mutation
with its code, cause, span and retained members. Scope: FR-381-AC-5 to
FR-381-AC-13.

Catches a forward binder that is not the effect's record, a template that adds
a second operation anchor, a zero attempt bound that checks, an attempt type
that is an object type and checks, a retry type that differs from the attempt
type, an unsafe retry arithmetic that checks, a commit
that precedes its effect, and a `Full` and a `Partial` recovery merged into one.

Fixture **R** is TC-1057's, with its record value types `Receipt`, `Attempt` and
`Flag`, its event type `Pulse`, its object type `Node`, and a variant type `Mode`. Every row changes the base protocol `Refunds` and
asserts the checked clause or the refusal's code, cause, span text and retained
members.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | none | checks; the clause holds `Refund` with effect `Applied`, role `Service`, the anchor and frame nodes of `Node::step`, maximum 3, `within [0,30]`, retry, commit `Node(Committed)` and the recovery predicate |
| 2 | `(forward: M::Flag)` | `ill_typed`/`type-mismatch` at `M::Flag`; `compensation` `Refund`, `expected` `M::Receipt`, `actual` `M::Flag` |
| 3 | `by Absent` | `missing_declaration`/`missing-name` at `Absent`; `compensation` `Refund`, `name` `Absent` |
| 4 | `on M::Node::missing` | `missing_declaration`/`missing-name` at `missing` (FR-114) |
| 5 | the base protocol, compiled | the package holds one anchor node and one frame node for `Node::step`, shared by `Tried` and `Refund` |
| 6 | a second template `Second` for `Main::Applied` with its own captures | checks; two compensations, in source order, each its own registration |
| 7 | a second template named `Refund` | `ambiguous_declaration`/`ambiguous-name` at both declarations, in source order |
| 8 | `for Main::Tried` and, separately, `commit Main::Applied` | `ill_typed`/`type-mismatch` at the anchor: site `compensate-for` kind `attempt`; site `compensate-commit` kind `effect` (FR-113-AC-7) |
| 9 | `attempts 0 of M::Attempt` | `ill_typed`/`type-mismatch` at `0`; `compensation` `Refund`, `member` `attempts`, `expected` a count of at least one, `actual` `0` |
| 10 | `attempts 1 of M::Attempt` and `attempts 18446744073709551615 of M::Attempt` | each checks; the checked compensation holds 1 and 18446744073709551615 |
| 11 | `attempts 18446744073709551616 of M::Attempt` | S1 parse diagnostic at the count; no form |
| 12 | `within [31,30]` and `within [0,*]` | each `ill_typed`/`type-mismatch` at the interval; `member` `within`, `expected` a closed interval with `a <= b`, `actual` as written |
| 13 | `within [0,0]` and `within [0,30]` | each checks |
| 14 | `attempts 3 of M::Flag` with the base retry binders `M::Attempt` | `ill_typed`/`type-mismatch` at `earlier`'s type, then at `later`'s; `expected` `M::Flag`, `actual` `M::Attempt` |
| 15 | `attempts 3 of M::Flag`, `retry (earlier: M::Flag, later: M::Flag) { earlier.ready = later.ready }`; and `attempts 3 of M::Pulse`, `retry (earlier: M::Pulse, later: M::Pulse) { earlier.ready = later.ready }` | each checks: a record type and an event type are admitted |
| 15a | `attempts 3 of M::Node`, an object type, with `retry (earlier: M::Node, later: M::Node) { true }` | exactly one refusal, `invalid_model_binding`/`malformed-declaration` at `M::Node` in `attempts`; `compensation` `Refund`, `member` `attempts`, `name` `M::Node`; none at the retry binders |
| 15b | `attempts 3 of M::Total`, a scalar type, with `retry (earlier: M::Total, later: M::Total) { true }`; and `attempts 3 of M::Absent` | the first the refusal of row 15a at `M::Total`; the second `missing_declaration`/`missing-name` at `Absent` |
| 15c | `attempts 3 of Set<M::Total>[0, 3]`, a composite type written inline, and `attempts 3 of M::Mode`, each with `retry (earlier: T, later: T) { true }`, `T` the attempt type as written | the refusal of row 15a at `Set<M::Total>[0, 3]` and at `M::Mode`, one each |
| 16 | the retry relation without `earlier.tally < 3 and` | `undefined_expression`/`unproved-range` at `earlier.tally + 1` |
| 17 | the retry relation `earlier.tally` | `ill_typed`/`non-boolean-root` at the relation |
| 18 | `commit never` | checks; the clause holds `Never` |
| 19 | the run holds `attempt Tried`, `commit Committed`, `effect Applied` in that order | `unsupported_construct`/`declaration-form` at `Main::Committed`, retaining the selected protocol profile; `compensation` `Refund`, `member` `commit`, `commit` `Committed`, `effect` `Applied`, `reason` `commit-precedes-effect` |
| 19a | the run holds `attempt Tried`, then `parallel P` with `branch a` holding `commit Committed` and `branch b` holding `check Idle using P { true }`, `join all [a, b]`, then `effect Applied` after the parallel; the template's `commit` is `Main::P::a::Committed` | `unsupported_construct`/`declaration-form` at `Main::P::a::Committed` with `reason` `commit-precedes-effect`: the `join all` requires `a`, so the `parallel` passes the commit and the later `effect` in the sequence is dominated |
| 19b | row 19a with `join any [a, b]` | checks: the `join any [a, b]` requires neither branch, so the `parallel` passes no commit |
| 19c | row 19a with `join any [a]` and with `quorum(2) [a, b]` | each refuses as row 19a: the join requires `a` |
| 19d | row 19a with `quorum(1) [a, b]` | checks: the join requires neither branch |
| 19e | `attempt Tried`, then `repeat Again by Service visible (attempted.ready) max 2 while { attempted.ready } sequence Body { commit Committed by Service as (committed: M::Flag) { committed.ready }; event Beat by Service as (beat: M::Flag) { true }; } exhausted check Out using P { true };`, then `effect Applied` after the repeat; the template's `commit` is `Main::Again::Body::Committed` | checks: S3 treats every `repeat` as skippable whatever its guard, so the guard `attempted.ready`, which is not constant, is not what admits it |
| 20 | the commit in `parallel` branch `a` and the effect in branch `b` | checks: no `sequence` orders the two branches |
| 20a | `choice Pick` with the effect after the `choice`, `Pick` holding `case yes` holding `commit Committed` and `case no` holding a `check` | checks: the commit lies in one case and does not dominate the effect |
| 20b | `join all [s, r]` with `branch s` holding `sequence S { commit C ...; send Tx ...; }` and `branch r` holding `sequence R { attempt T ...; receive Rx of Main::P::s::S::Tx ...; effect E of Main::P::r::R::T ...; }`; the template is `compensate Refund for Main::P::r::R::E` and its `commit` is `Main::P::s::S::C` | checks: every execution reaching `E` has passed `C` through the `send`/`receive` edge, but S3 decides dominance on the control tree only and admits it (a later extension) |
| 21 | the recovery predicate `recovered.n` | `ill_typed`/`non-boolean-root` at the predicate |
| 22 | the recovery predicate `recovered.tally = recovered.tally + 1` | `undefined_expression`/`unproved-range` at `recovered.tally + 1` |
| 23 | `recover (recovered: M::Absent)` | `missing_declaration`/`missing-name` at `Absent` |
| 24 | `Refund` renamed `Full`, plus a template `Partial` for `Main::Applied` with commit `never`, a different `attempts` and an equal recovery predicate | checks; two compensations `Full` and `Partial` in source order, with their own captures, maxima, commits and predicates |
| 25 | `Full` with `attempts 0` and `Partial` with `within [31,30]` | two refusals, `Full`'s then `Partial`'s, each as rows 9 and 12 state |

## Expected Results

Every template is checked over its own forward effect, role, operation, bound,
deadline, retry, commit and recovery, and each mutation refuses at the exact
member with the code and cause the row names. A zero attempt bound, an inverted
or open deadline, an attempt type that is not a record or event type, a retry whose type is not the attempt type, an unsafe retry
arithmetic and a commit that precedes its effect do not check. Two templates
stay two obligations, and one template's refusal does not hide another's.
