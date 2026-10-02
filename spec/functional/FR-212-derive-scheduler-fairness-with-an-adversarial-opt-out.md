---
id: FR-212
title: "Derive scheduler fairness over protocol threads, with the scheduling adversarial opt-out"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-161
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-430
    type: depends_on
---
# FR-212: Derive scheduler fairness over protocol threads, with the scheduling adversarial opt-out

## Description

S3 SHALL resolve a fairness constraint of an infinite-trace clause over a
protocol subject to an operation, an attempt node, a branch or the root, or
a compensation template, with `whole` or `each` granularity (ADR-027 PA-1),
and the model checker SHALL read its class as enabled and taken by protocol
steps (ADR-027 PA-2). Unless the protocol clause declares `scheduling
adversarial`, the checker SHALL add to every infinite-trace clause over a
protocol subject one derived `fair weak whole` **scheduler constraint** per
`parallel` branch, the root and each compensation template, with a
`scheduler` origin (ADR-027 PA-3).

## Use case

A verification operator checks that a branch eventually does its work while
a channel keeps duplicating and losing messages. By default the scheduler
serves every runnable thread, so a liveness claim fails only for a reason in
the protocol, and the premise is visible in the result. When they want to
know whether the claim survives a scheduler that starves a thread, they add
`scheduling adversarial` and check again as a separate obligation.

## Inputs

- The parsed fairness constraints of an infinite-trace clause (FR-123) and
  the protocol clause's optional `scheduling adversarial` member.
- The checked protocol clause's branches, root and compensation templates
  (FR-218).

## Outputs

- The clause's fairness set: authored constraints, each with an `authored`
  origin, and, unless the protocol is adversarial, the scheduler
  constraints with a `scheduler` origin.
- The class of each constraint over protocol transition identities, read by
  FR-126's fairness filter.

## Behavior

### Targets at S3

- S3 SHALL resolve a constraint's target to an operation, an attempt node,
  a `parallel` branch, the protocol's root by the protocol's name, or a
  compensation template, and SHALL refuse a name that resolves to none with
  `missing_declaration`/`missing-name` at its span (FR-123's refusal).
- S3 SHALL admit at most one `scheduling` member per protocol clause, and
  SHALL refuse a second with `ambiguous_declaration`/`ambiguous-name` at
  its span, naming the first, as FR-211 refuses a second `terminal`
  member. The member SHALL be part of the checked package.

### Classes

- The model checker SHALL compute each constraint's class over protocol
  transition identities as QSpec FR-430 states for operations, attempt
  nodes, branches, the root and compensation templates, with channel steps,
  `activate`, `spawn`, `retire` and memory steps in no class.
- FR-126's filter, ADR-018 FA-3's weak fairness and FA-4's SCC test SHALL
  read these classes, a class being enabled when one of its steps is
  enabled (FR-206 to FR-209) and taken by a step that belongs to it.

### Scheduler constraints

- Unless the protocol declares `scheduling adversarial`, the checker SHALL
  add to each infinite-trace clause over the subject one `fair weak whole`
  constraint per `parallel` branch, one over the root and one per
  compensation template, each with origin `scheduler`, beside the authored
  constraints.
- The scheduler constraints SHALL be part of the clause's fairness set, its
  obligation identity, every counterexample and replay (FR-217).
- With `scheduling adversarial`, the fairness set SHALL be the authored
  constraints alone.
- The scheduler constraints SHALL change liveness verdicts only: a safety
  item's verdict SHALL be the same with and without them.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-212-AC-1 | The `Chatter` protocol: model `Cell` with `flag: Int[0, 1]`, operation `set` (precondition true, postcondition `self.flag = 1`), universe `{k}`, `flag = 0`; a channel `ch` of capacity 2 with delivery `unknown`; `terminal any`; `run parallel Both { branch s sequence { send Tx on ch; attempt Done on set }; branch r receive Rx of Tx; } join all [s, r]`. `eventually holds(k.flag = 1)` under infinite-trace settles `proved`; its fairness set holds three `scheduler` constraints, over `s`, `r` and the root. | Test (TC-657) |
| FR-212-AC-2 | `Chatter` with `scheduling adversarial` checks to a different package and obligation identity. The same claim settles `refuted` with a lasso whose stem is `fork(Both)`, `send(Tx)` and whose loop is `duplicate(ch, Tx)`, `lose(ch, Tx)`, with `s` resting at `Done` throughout; its fairness set is empty. | Test (TC-657) |
| FR-212-AC-3 | Over `Chatter`, `fair weak whole s` names the branch and `fair weak each Done` the attempt node; `fair weak Missing` refuses `missing_declaration`/`missing-name`; a second `scheduling` member refuses `ambiguous_declaration`/`ambiguous-name` naming the first. In ADR-027 §7, the root's `whole` class holds `fork(Both)`, `join(Both)` and `finish`, and `left`'s holds `attempt(A)`. | Test (TC-657) |
| FR-212-AC-4 | The TP-1 claim `always holds(k.flag <= 1)` over `Chatter` settles `proved` with and without `scheduling adversarial`, over the same reachable states. | Test (TC-657) |

## Dependencies

- ADR-027 §3.3 PA-1 to PA-4; ADR-018 §4 FA-1 to FA-6.
- [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (fairness constraints at S3),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (the fairness filter), FR-205, FR-206, FR-218.
- QSpec owns the fairness targets and their granularity, the `scheduling
  adversarial` member and the `scheduler` origin (ADR-027 QS-6, QS-13):
  QSpec FR-161 and the shared grammar.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-430 (Linear STD-140).
