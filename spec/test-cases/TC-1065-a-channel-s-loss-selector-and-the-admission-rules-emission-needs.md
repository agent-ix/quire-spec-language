---
id: TC-1065
title: "A channel's loss selector and the admission rules emission needs"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-308
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: verifies
---
# TC-1065: A channel's loss selector and the admission rules emission needs

## Description

Verify that S2 builds a channel's overflow policy with its loss selector, that
S3 records it as a premise of its own, that the parser refuses a choice or
parallel with fewer than two cases or branches, and that S3 refuses a
duplicate label, an unsound join, a role on a non-participant model, a channel
carrying a non-message type and a requirement cycle. Scope: FR-308-AC-13 and
FR-218-AC-21 to FR-218-AC-26 and FR-218-AC-31.

Catches an overflow built without its selector, a selector accepted after
`reject`, a selector derived from the delivery policy, two channels differing
in selector treated as one, a one-case `choice` that reaches S4, a `join all`
that omits a branch and a role on a record type.

Fixture **P** is TC-1056's. Each row changes its base protocol `Pipeline`
or the stated construct.

## Test Procedure

| # | Change | Expected |
| --- | --- | --- |
| 1 | channel overflow `reject`, `block`, `loss head`, `loss tail`, `loss new` | S2 builds `Reject`, `Block`, `Loss(Head)`, `Loss(Tail)`, `Loss(New)` with spans slicing to their text; S3 records the same |
| 2 | overflow `loss` with no selector; overflow `block head` | a parser syntax diagnostic; no form |
| 3 | row 1's three loss channels with one delivery; the same with another delivery | three different checked channels; the delivery never changes the recorded overflow policy |
| 4 | a `choice` with cases `shipped` and `shipped`; a `parallel` with branches `east` and `east` | `ambiguous_declaration`/`ambiguous-name` at both labels, in source order, naming the label |
| 5 | a `choice` with one case; a `parallel` with one branch; two of each | a parser syntax diagnostic and no form for the first two; the last two check |
| 6 | `join all [east]`; `join quorum(0) [east, west]`; `join quorum(3) [east, west]`; `join any []` | `invalid_package`/`invalid-value` at the list, and at the count for `quorum` |
| 7 | `join all [east, west]`; `join quorum(2) [east, west]`; `join any [east]`; `join all [east, west, east]` | each checks; the last joins each label once |
| 8 | `role Seller on M::Pt` where `Pt` is a record type; roles on an object type, a process and a state machine; `role w on M` in ADR-027 §7's `Fill` unit (alias `M`, `over (k: M::Cell)`); `role w on M` beside `role u on N` in one clause over two aliases; `role w on M` in clauses whose `over` type is `Int[0, 3]`, a record and `Set<M::Cell>`; the replicated `role r each M from cells` | `invalid_model_binding`/`malformed-declaration` at `M::Pt`; the object type, process and state machine check; every `role ... on <alias>` checks as a role on a whole model, recording its alias's `model_selections` identity; `role r each M from cells` refuses `invalid_model_binding`/`malformed-declaration` at `M`, with no `pop` refusal and no package |
| 9 | `channel Pay ... carries M::Node` where `Node` is an object type; channels carrying a record, an event type, a variant type and a scalar type | `invalid_model_binding`/`malformed-declaration` at `M::Node` and no refusal at the `send` and `receive` binders, which are not compared with a refused `carries` type; the others check |
| 10 | `requires temporal Due;` where `Due` holds `protocol Fill` and the requiring protocol is `Fill`; the same through a second temporal clause; a protocol `Drain` no clause names | `invalid_package`/`definition-cycle` at the requirement, naming the protocol and the clause, for the first two; `Drain` checks |
| 11 | over branches `a`, `b`, `c` and `function f using v(x: Boolean, y: Boolean): Boolean pure { x and y }`: `join predicate(f) [a, c]`; `join predicate(f) [c, a, c]`; a second join naming `f` | each checks to `Predicate{function}` with `f`'s node key and joined labels `a`, `c`; the two joins hold one key |
| 12 | `join predicate(g) [a, c]` with no `g`; `g` declared as a `predicate`; `join predicate(f) [a, b, c]`; `h(x: Boolean, y: Int[0, 1]): Boolean` on `[a, c]`; `k(x: Boolean, y: Boolean): Int[0, 1]` on `[a, c]`; a self-calling `r(x: Boolean, y: Boolean): Boolean` with no `decreases` on `[a, c]` | `missing_declaration`/`missing-name` at `g` (node, name); `ill_typed`/`operator-ineligible` at `g` naming the kind; `ill_typed`/`type-mismatch` at the name for the next three (node, function, expected, actual); `undefined_expression`/`unproved-decrease` at `r`'s declaration and no refusal at the join; no package for any |

## Expected Results

- Each row asserts code, cause, span text and retained members as fields, never
  message text.
