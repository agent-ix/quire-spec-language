---
id: FR-211
title: "Declare a protocol's intended terminal states and report its deadlocks"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-181
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-429
    type: depends_on
---
# FR-211: Declare a protocol's intended terminal states and report its deadlocks

## Description

A protocol clause SHALL declare its intended terminal states with at most
one `terminal` member, `terminal when P` or `terminal any` (ADR-027 PD-1).
A protocol state with no enabled step SHALL be terminal, and a terminal
state that is not intended SHALL be a deadlock (ADR-027 PB-4, PD-2). The
request writer SHALL add one deadlock-freedom item per distinct protocol
subject among a request's temporal items, unless its protocol declares
`terminal any` (ADR-027 PD-3), and a deadlock counterexample SHALL name
each waiting thread with its rest point and wait cause (ADR-027 PD-4).

## Use case

A verification operator checks a protocol whose join waits for a branch
that can get stuck. They want the stuck state reported without writing a
claim for it, with each waiting thread and the reason it waits, so they can
see which precondition or empty queue holds the join up. A protocol that
stops on purpose in some states says which, and one that may stop anywhere
opts out.

## Inputs

- The parsed `terminal` member of a protocol clause.
- For the request writer: the request's temporal items over protocol
  subjects (FR-205).
- For classification: a protocol state and its enabled steps (FR-206 to
  FR-209).

## Outputs

- On the checked protocol clause: `TerminalDeclaration::{None, When(clause),
  Any}` (FR-124's type), where `clause` is a checked Boolean predicate over
  the model observation and the protocol's captures.
- A `DeadlockFreedom` item per protocol subject, as FR-124 gives one per
  model subject.
- For a deadlock counterexample: `blocked: Vec<BlockedThread{path,
  rest_point, cause: WaitCause}>` for its last state, with `WaitCause` one
  of `Precondition{clause}`, `NoPostState`, `Constraint`,
  `EmptyQueue{channel}`, `JoinWaiting{threads}`, `CheckFalse{node}` and
  `Gate`.

## Behavior

### The `terminal` member

- S3 SHALL admit at most one `terminal` member per protocol clause, and
  SHALL refuse a second with `ambiguous_declaration`/`ambiguous-name` at
  its span, naming the first (QSpec FR-429).
- S3 SHALL check `P` as a Boolean state predicate over the model
  observation and the protocol's captures through the one clause checker
  (FR-104), and SHALL refuse a read of a binder, a parameter or a pre-state
  as FR-104 refuses it in an invariant.
- The declaration SHALL be part of the checked package.
- A protocol subject SHALL read only its protocol's `terminal` member; the
  state model's own member classifies `ModelSystem`'s terminal states.

### Terminal and deadlocked states

- The model checker SHALL classify each protocol state as terminal,
  intended or deadlocked by QSpec FR-429's rules, reading `Any`, `When(P)`
  or `None` from the checked declaration; a terminal state with no live
  instance is intended.
- A boundary state (a state constraint's boundary, a memory-bound-limited
  state or an instance-limited state, FR-209) SHALL be neither terminal nor
  a deadlock.

### The deadlock-freedom item and blocked threads

- The request writer SHALL add one `DeadlockFreedom` item per distinct
  protocol subject among the request's temporal items, and none for a
  subject whose protocol declares `terminal any`. Its obligation identity
  SHALL be the subject and the kind `deadlock-freedom`.
- The item SHALL be checked, settled and replayed as FR-124's item is, with
  `deadlocked` read by this FR.
- A deadlock counterexample SHALL carry, for its last state, each waiting
  thread's path, rest point and wait cause, computed from the step rules
  that disable it (FR-206).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-211-AC-1 | `Fill` (ADR-027 §7) checks with no `terminal` member, with `terminal when` a predicate over `k.v`, and with `terminal any`; the three packages have pairwise different identities. A second `terminal` member refuses `ambiguous_declaration`/`ambiguous-name` naming the first; a `terminal when` predicate that reads binder `a` refuses as FR-104 refuses a non-state read in an invariant. | Test (TC-656) |
| FR-211-AC-2 | Over `Fill` with no `terminal` member, s6 is terminal and intended, and s2 is terminal and deadlocked. The deadlock-freedom item settles `refuted` with the prefix `fork(Both)`, `attempt(A)` and blocked threads `(0, Both, right, 0)` at `B` with `Precondition{TwoPre}` and `(0)` at `join Both` with `JoinWaiting{[(0, Both, right, 0)]}`. With `terminal when k.v = 1`, s2 is intended and the item settles `proved`; with `terminal any`, the request carries no deadlock-freedom item. | Test (TC-656) |
| FR-211-AC-3 | Over `Fill` with `TwoPre: self.v <= 1` and no `terminal` member, the deadlock-freedom item settles `proved`, basis `Exhaustive`, over ten states. Two temporal items over the same protocol subject give one deadlock-freedom item. | Test (TC-656) |
| FR-211-AC-4 | An `on each` protocol whose body's one attempt has a precondition that is false in every reachable model state, with no `terminal` member and `max_live_instances` 2: the state with one live instance, waiting at its attempt, where only `activate` is enabled, is a deadlock with blocked thread `(0)` at the attempt, cause `Precondition`. The state with two live instances is instance-limited and is not a deadlock, and the initial state, with no live instance, is not a deadlock. | Test (TC-656) |

## Dependencies

- ADR-027 §3.1 PB-4, §3.2 PD-1 to PD-5; ADR-018 §10 DL-1 to DL-7.
- [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (the declaration type and the deadlock-freedom item),
  [FR-104](FR-104-check-state-clauses.md) (predicate checking),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (checking), FR-205, FR-206, FR-209, FR-218.
- QSpec owns the protocol-level `terminal` member and the protocol subject's
  intended terminal states, deadlocks and wait causes (ADR-027 QS-7,
  QS-13).

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-429 (Linear STD-140).
