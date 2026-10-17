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
FR-381-AC-12.

Catches a forward binder that is not the effect's record, a template that adds
a second operation anchor, a zero attempt bound that checks, a retry type that
differs from the attempt type, an unsafe retry arithmetic that checks, a commit
that precedes its effect, and a `Full` and a `Partial` recovery merged into one.

Fixture **R** is TC-1057's. Every row changes the base protocol `Refunds` and
asserts the checked clause or the refusal's code, cause, span text and retained
members.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | none | checks; the clause holds `Refund` with effect `Applied`, role `Service`, the anchor and frame nodes of `Node::step`, maximum 3, `within [0,30]`, retry, commit `Node(Committed)` and the recovery predicate |
| 2 | `(forward: M::Plain)` | `ill_typed`/`type-mismatch` at `M::Plain`; `compensation` `Refund`, `expected` `M::Node`, `actual` `M::Plain` |
| 3 | `by Absent` | `missing_declaration`/`missing-name` at `Absent`; `compensation` `Refund`, `name` `Absent` |
| 4 | `on M::Node::missing` | `missing_declaration`/`missing-name` at `missing` (FR-114) |
| 5 | the base protocol, compiled | the package holds one anchor node and one frame node for `Node::step`, shared by `Tried` and `Refund` |
| 6 | a second template `Second` for `Main::Applied` with its own captures | checks; two compensations, in source order, each its own registration |
| 7 | a second template named `Refund` | `ambiguous_declaration`/`ambiguous-name` at both declarations, in source order |
| 8 | `for Main::Tried` and, separately, `commit Main::Applied` | `ill_typed`/`type-mismatch` at the anchor: site `compensate-for` kind `attempt`; site `compensate-commit` kind `effect` (FR-113-AC-7) |
| 9 | `attempts 0 of M::Node` | `ill_typed`/`type-mismatch` at `0`; `compensation` `Refund`, `member` `attempts`, `expected` a count of at least one, `actual` `0` |
| 10 | `attempts 1 of M::Node` and `attempts 18446744073709551615 of M::Node` | each checks; the checked compensation holds 1 and 18446744073709551615 |
| 11 | `attempts 18446744073709551616 of M::Node` | S1 parse diagnostic at the count; no form |
| 12 | `within [31,30]` and `within [0,*]` | each `ill_typed`/`type-mismatch` at the interval; `member` `within`, `expected` a closed interval with `a <= b`, `actual` as written |
| 13 | `within [0,0]` and `within [0,30]` | each checks |
| 14 | `attempts 3 of M::Plain` with the base retry binders `M::Node` | `ill_typed`/`type-mismatch` at `earlier`'s type, then at `later`'s; `expected` `M::Plain`, `actual` `M::Node` |
| 15 | `attempts 3 of M::Plain`, `retry (earlier: M::Plain, later: M::Plain) { earlier.ready = later.ready }` | checks |
| 16 | the retry relation without `earlier.tally < 3 and` | `undefined_expression`/`unproved-range` at `earlier.tally + 1` |
| 17 | the retry relation `earlier.tally` | `ill_typed`/`non-boolean-root` at the relation |
| 18 | `commit never` | checks; the clause holds `Never` |
| 19 | the run holds `attempt Tried`, `commit Committed`, `effect Applied` in that order | `unsupported_construct`/`declaration-form` at `Main::Committed`, retaining the selected protocol profile; `compensation` `Refund`, `member` `commit`, `commit` `Committed`, `effect` `Applied`, `reason` `commit-precedes-effect` |
| 20 | the commit in `parallel` branch `a` and the effect in branch `b` | checks |
| 20a | `choice Pick` before the effect with `case yes` holding `commit Committed` and `case no` holding a `check` | checks: the commit lies in one case and does not dominate the effect |
| 21 | the recovery predicate `recovered.n` | `ill_typed`/`non-boolean-root` at the predicate |
| 22 | the recovery predicate `recovered.tally = recovered.tally + 1` | `undefined_expression`/`unproved-range` at `recovered.tally + 1` |
| 23 | `recover (recovered: M::Absent)` | `missing_declaration`/`missing-name` at `Absent` |
| 24 | `Refund` renamed `Full`, plus a template `Partial` for `Main::Applied` with commit `never`, a different `attempts` and an equal recovery predicate | checks; two compensations `Full` and `Partial` in source order, with their own captures, maxima, commits and predicates |
| 25 | `Full` with `attempts 0` and `Partial` with `within [31,30]` | two refusals, `Full`'s then `Partial`'s, each as rows 9 and 12 state |

## Expected Results

Every template is checked over its own forward effect, role, operation, bound,
deadline, retry, commit and recovery, and each mutation refuses at the exact
member with the code and cause the row names. A zero attempt bound, an inverted
or open deadline, a retry whose type is not the attempt type, an unsafe retry
arithmetic and a commit that precedes its effect do not check. Two templates
stay two obligations, and one template's refusal does not hide another's.
