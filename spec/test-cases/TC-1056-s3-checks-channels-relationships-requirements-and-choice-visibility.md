---
id: TC-1056
title: "S3 checks channels, relationships, requirements and the owner and visible facts of a decision"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
---
# TC-1056: S3 checks channels, relationships, requirements and the owner and visible facts of a decision

## Description

Verify that S3 checks each channel's roles, payload, ordering and premises,
each relationship declaration and `related by` clause, each `requires temporal`
requirement and the owner and visible facts of each `choice` and `repeat`, and
refuses each mutation with its code, cause, span and retained members. Scope:
FR-218-AC-10 to FR-218-AC-20 and FR-218-AC-26.

Catches a channel with an undeclared role that checks, a send typed differently
from its channel that checks, a relationship bound to its written name instead of
its key, an end of the wrong type that checks, a requirement that resolves
through an import, a decision on a fact its owner cannot observe, and a gap or
overlap that passes because the guards are constant.

Fixture **P**: a domain package `M` with the object types `Node{total: Total,
tally: Int[0, 3], n: Int[0, 9], ok: Boolean}` with the operation `Node::step`, and `Peer{id: Int[0, 9]}`; the record value type
`Plain{ready: Boolean, peer: Reference<Peer>}`; a relationship `pipe` whose
first end is `Peer` and second end is `Node`; a relationship `owns` between the
same two types; a port `out`; model alias `M`, profile alias
`P`, a temporal clause `Due` and a predicate `Accept` in the unit. The base
protocol `Pipeline` declares `role Seller on M::Node;`, `role Buyer on
M::Node;`, `channel Pay from Seller to Buyer carries M::Plain ordering
unordered delivery at-least-once capacity 2 overflow block;` and a `run`
`sequence Main` holding `send Sent via Pay as (sent: M::Plain) { true };` and
`receive Got via Pay of Main::Sent as (got: M::Plain) { true };`. It checks.
The channel carries a record, because a channel does not carry an object type
(FR-218), and every binder a decision reads through its fields is a record: a
field read of an object binder is a model-state read, which a guard, a
`visible` entry and a FIFO key bar (FR-218). The protocol input `view` is the
one object-typed binder, read only as a relationship end and in the rows that
name it.

## Test Procedure

Check one unit per row, the base protocol with the stated change, and assert the
checked clause or the refusal's code, cause, span text and retained members.

| # | Change | Expected |
| --- | --- | --- |
| 1 | `to Buyer` replaced by `to Absent` | `missing_declaration`/`missing-name` at `Absent`; `channel` `Pay`, `end` `to`, `name` `Absent` |
| 2 | `from Seller to Buyer` replaced by `from Nobody to Absent` | two refusals, `from` then `to`, each `missing-name` at its own name |
| 3 | `carries M::Plain` replaced by `carries M::Absent` | `missing_declaration`/`missing-name` at `Absent`; `channel` `Pay` |
| 4 | a second `channel Pay ...` | `ambiguous_declaration`/`ambiguous-name` at both declarations, in source order |
| 5 | `relationship Pay = M::pipe;` added | checks: a channel and a relationship may share a name |
| 6 | `send Sent via Missing` | `missing_declaration`/`missing-name` at `Missing`; `node` `Sent` |
| 7 | `send Sent via Pay as (sent: M::Node)` | `ill_typed`/`type-mismatch` at `M::Node`; `channel` `Pay`, `expected` `M::Plain`, `actual` `M::Node` |
| 8 | `receive Got ... as (got: M::Node)` | the same refusal at the receive's binder type |
| 9 | a second channel `Other` and `receive Got via Other of Main::Sent` | `ill_typed`/`type-mismatch` at the `of` anchor naming `Pay` and `Other` (FR-113-AC-7) |
| 10 | `ordering fifo by (m: M::Plain) {m.ready}` | checks; the checked channel holds `Fifo` with binder `m` and key `m.ready` |
| 11 | row 10 with `(m: M::Node)` | `ill_typed`/`type-mismatch` at `M::Node`; `expected` `M::Plain`, `actual` `M::Node` |
| 12 | row 10 with key `{m.ready and view.total > 0}`, `view` the protocol input | `ill_typed`/`operator-ineligible` at the read of `view.total`, naming the key |
| 13 | delivery `exactly-once-premise`, capacity `symbolic(Bound)`, overflow `loss head`, `ordering unordered` | checks; the checked channel records those four premises, `Bound` unresolved and the selector `Head` |
| 14 | row 13 with delivery `at-most-once` | checks; a different checked channel from row 13's; ordering, capacity and overflow unchanged; the same with `loss tail` and with `loss new` are two further different checked channels |
| 15 | `relationship Pipe = M::pipe;` | checks; the checked relationship holds `pipe`'s declaration key and end types `Peer`, `Node` in that order |
| 16 | `relationship Pipe = M::nothing;` | `missing_declaration`/`missing-name` at `nothing` |
| 17 | `relationship Pipe = M::out;` and `relationship Pipe = M::Node;` | each `invalid_model_binding`/`wrong-export` at the name; required kind `relationship`, actual kind `port` and `object type`, both declaration keys |
| 18 | two `relationship Pipe = ...;` | `ambiguous_declaration`/`ambiguous-name` at both declarations |
| 19 | row 15 and `related by Pipe (sent.peer, view)` on `Got`, `view` the protocol input of type `M::Node` | checks; `Got` holds one checked clause with both ends |
| 20 | row 19 with `related by Absent (sent.peer, view)` | `missing_declaration`/`missing-name` at `Absent`; `node` `Got` |
| 21 | row 19 with the ends swapped, `(view, sent.peer)` | `ill_typed`/`type-mismatch` at each end; `end` `first` expected `Peer` actual `Node`, then `end` `second` expected `Node` actual `Peer` |
| 22 | row 19 plus `related by Owns (sent.peer, view)` (with `relationship Owns = M::owns;`) and a repeat of `related by Pipe (sent.peer, view)` | checks; `Got` holds three checked clauses in source order |
| 23 | `requires temporal Due;` | checks; the requirement is bound to `Due`'s node key |
| 24 | `requires temporal Nothing;`, and `requires temporal Due;` where `Due` is declared only in an imported unit | each `missing_declaration`/`missing-name` at the name; no package |
| 25 | `requires temporal Accept;` | `ill_typed`/`operator-ineligible` at `Accept`; the resolved kind named |
| 26 | `requires temporal Due;` written twice | checks; one requirement recorded, at the first position |
| 27 | a `choice Decide by Nobody visible (true) {...}` | `missing_declaration`/`missing-name` at `Nobody` |
| 28 | after `Got`, `choice Decide by Buyer visible (got.ready) { case yes when { got.ready } check A using P { true }; case no when { not got.ready } check B using P { true }; }` | checks; the checked choice holds owner `Buyer` and the listed fact |
| 29 | row 28 with owner `Seller` | `wrong_snapshot`/`wrong-observation` at each read of `got.ready`; `owner` `Seller`, `reason` `other-role` |
| 30 | row 28 reading `sent.ready` | `wrong_snapshot`/`wrong-observation`; `reason` `ineligible-kind` (a `send` record) |
| 31 | row 28 with `visible (true)` | `wrong_snapshot`/`wrong-observation` at each guard read; `reason` `unlisted` |
| 32 | row 28 with two receives `gotA`, `gotB`, `visible (gotA.ready and gotB.ready)` and guards reading `gotA.ready` | `wrong_snapshot`/`wrong-observation`; `reason` `unlisted`: a composite entry discloses none of its operands |
| 33 | row 28 with guards `let d = got.ready in true` and `false` and `visible (true)` | `wrong_snapshot`/`wrong-observation` at `got.ready`; `reason` `unlisted`; with `visible (got.ready)` it checks |
| 34 | the choice reads the protocol input `view.n` listed in `visible` | `ill_typed`/`operator-ineligible` at each read of `view.n`, naming the guard: the `over` input is model state and the rule takes precedence over the eligibility table |
| 35 | the receive inside `branch left` of a `parallel` and the choice in `branch right`; the receive inside one case of an earlier optional `choice`; the receive as the `match` of an `await`, with the choice after the `await` | each refuses as FR-113 refuses an unresolved name, naming `got` at its `visible` entry and at each guard read: three uses in each case |
| 36 | a `repeat Again by Buyer visible (got.ready) max 2 while { got.ready } ...` whose `receive Got` is bound inside its own body | refuses as an out-of-scope read at the `visible` entry and at the guard, naming `got` twice |
| 37 | row 28 with guards `got.ready` and `true`; with guards `got.ready` and `false`; with guards `got.ready` and `not got.ready` | `undefined_expression`/`unproved-exhaustiveness` at the choice with `failed` `[duplicate-arm]`; the same with `failed` `[missing-arm]`; the third checks |
| 38 | constant guards `true` and `not false`; constant guards `false` and `not true` | `undefined_expression`/`unproved-exhaustiveness`; `failed` `[duplicate-arm]`, then `[missing-arm]` |
| 39 | two receives `gotA`, `gotB`, `visible (gotA.ready, gotB.ready)`, with guards `gotA.ready` and `not gotB.ready`; `not gotA.ready` and `gotA.ready and not gotB.ready`; `gotA.ready` and `not gotA.ready or gotB.ready` | `undefined_expression`/`unproved-exhaustiveness` with `failed` `[missing-arm, duplicate-arm]`, `[missing-arm]` and `[duplicate-arm]` respectively |
| 40 | complementary guards `gotA.ready and gotB.ready`, `gotA.ready and not gotB.ready`, `not gotA.ready and gotB.ready`, `not gotA.ready and not gotB.ready` over both listed facts | checks |
| 41 | row 28 with a protocol capture `lim` and a guard `got.ready and lim >= 0` not listed in `visible` | checks: a protocol capture needs no listing |
| 42 | row 10 with key `{m}` (a record type) and with a key of a type over which the clause checker admits no `=` | the first checks; the second `ill_typed`/`operator-ineligible` at the key, `operator` `=` and the type |
| 43 | before `Got`, `attempt Tried by Seller on M::Node::step contracts [] as (attempted: M::Node) { true };`, and after `Got` `choice Decide by Seller visible (attempted.ok) { case yes when { attempted.ok } check A using P { true }; case no when { not attempted.ok } check B using P { true }; }` | `ill_typed`/`operator-ineligible` at the `visible` entry and at each guard read of `attempted.ok`, naming the `visible` entry and the guards: a field read of an object binder is a model-state read; with `attempted: M::Plain` and `visible (attempted.ready)` the same choice checks |
| 44 | row 10 with key `{m.peer.id = 1}` | `ill_typed`/`operator-ineligible` at `m.peer.id`, naming the key: a read through a reference field is a model-state read |

## Expected Results

A channel, relationship, requirement or decision that names something the
protocol or the unit does not declare, types a payload or end differently from
its declaration, or reads a fact its owner cannot see, refuses at the exact
name with the code and cause the row names, and produces no checked clause. A
protocol that does none of these checks, and its checked clause holds each
channel's premises as written, each relationship and requirement bound to its
declaration key, and each decision's owner and listed facts.
