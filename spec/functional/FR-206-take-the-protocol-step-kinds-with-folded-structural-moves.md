---
id: FR-206
title: "Take the protocol step kinds, with structural moves folded and attempts atomic"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-024
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-114
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-052
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-053
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-170
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-171
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-426
    type: depends_on
---
# FR-206: Take the protocol step kinds, with structural moves folded and attempts atomic

## Description

The protocol system SHALL take the step kinds `attempt`, `event`, `send`,
`receive`, `duplicate`, `lose`, `fork`, `join`, `timeout`, `finish`,
`fence` and memory steps, each enabled and applied as QSpec FR-426 states
(ADR-027 ST-1 to ST-10). The protocol system SHALL enable a `fence` step by its memory model's
fence rule alone and SHALL make it wait on nothing else (ADR-027 SE-5). The protocol system SHALL fold every structural
move into the step that causes it (ADR-027 FO-1, FO-2). The protocol
system SHALL apply each step atomically, an attempt as one FR-120
application (ADR-027 AT-1 to AT-3). FR-207, FR-208 and FR-209 cover the compensation,
replicated-role and activation steps (ST-11 to ST-15).

## Use case

A verification operator models two branches that each invoke an operation
on a shared cell and a join that waits for both. They need each branch's
attempt to mean exactly what the operation's contract says, a stuck branch
to wait rather than fail, and entering a sequence or choosing a case not to
add states that no behaviour distinguishes.

## Inputs

- A protocol state (FR-205) and the checked protocol clause (FR-218).
- The memory model's gate, observation and write (FR-215 `MemoryModel`),
  the identity under `Sc`.

## Outputs

- For each enabled step at a state: its kind, owner and successor state,
  with every created or moved thread settled.

## Behavior

- When a step occurs, the protocol system SHALL settle each thread the step
  moved or created by QSpec FR-426's structural moves until it reaches a
  rest point. A structural move SHALL never be a step and never a position.
- The protocol system SHALL give the steps of QSpec FR-426's step table,
  with its enabledness and effects, including `finish` only once every
  other thread of the instance has left `running` (ADR-027 ST-9).
- The protocol system SHALL apply each `attempt` through the same FR-120
  application function `ModelSystem` uses (FR-215), over the clauses the
  attempt's `contracts` select and the thread observation the memory model
  gives, writing the delta through the memory model.
- The protocol system SHALL read the memory model's gate, split, merge and
  internal steps only through the `MemoryModel` seam (FR-215), the identity
  under `Sc`.
- The protocol system SHALL bind each `send`'s binder to a send record
  `{channel, payload, outcome, displaced}`, with `channel` in the channel's
  qualified name (ADR-027 ST-3, PS-7).
- Under `outstanding cancel`, the protocol system SHALL make `cancelled`, in
  the join step, every branch thread still `running` together with every
  thread at or below it, and SHALL settle and remove every parallel
  instance those threads forked; none of them takes a further step
  (ADR-027 ST-7).
- A thread whose rest point has no enabled step SHALL wait, and a
  `cancelled` thread SHALL take no step. The protocol system SHALL leave no
  thread between two positions inside an attempt.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-206-AC-1 | ADR-027 §7 (`Fill`, `TwoPre: self.v = 0`): the reachable states are exactly s0 to s6 with the enabled steps of its table: `fork(Both)` at s0; `attempt(A)` to s2 and `attempt(B)` to s3 at s1; none at s2; `attempt(A)` to s4 at s3; `join(Both)` to s5 at s4; `finish` to s6 at s5; none at s6. Seven states and six transitions; no state has a thread inside a `sequence` between its children. | Test (TC-651) |
| FR-206-AC-2 | An attempt of `bump()` (frame `modifies self.v`, postcondition `self.v >= pre(self.v) + 1`, `v: Int[0, 3]`) from `c.v = 0` gives three `attempt` steps, to `v = 1`, `2` and `3`, each equal to the successors FR-120 gives the same application. An attempt whose `contracts` list omits a precondition clause is enabled where only that clause is false. An attempt whose binder constraint no record satisfies has no step and its thread waits. | Test (TC-651) |
| FR-206-AC-3 | Three branches each with one always-enabled attempt: `join any … outstanding cancel` is enabled after the first branch completes, and its step sets both other threads `cancelled`, which take no further step; `join any … outstanding continue` leaves them `running`, and each still takes its attempt; `join quorum(2)` is not enabled after one completion and is after two; `join predicate(f)` with `f` true exactly when branch `a` is `completed` is enabled after `a` completes and not after only `b` and `c`. | Test (TC-651) |
| FR-206-AC-4 | A `fifo` channel of capacity 1 under `block`: a second `send` is not enabled while the queue holds one entry. Under `reject` the second send is enabled, leaves the queue unchanged and its binder holds a send record with outcome `rejected`. Under delivery `at-least-once`, `duplicate` is enabled when capacity admits it and `lose` never is; under `at-most-once`, `lose` is enabled and `duplicate` never is. An `unordered` channel holding two distinct payloads gives two `receive` steps; a `fifo` channel gives one, for the head. | Test (TC-651) |
| FR-206-AC-5 | A `choice` whose guard reads an earlier binder, a `repeat` and a `check` add no step and no position. For `repeat R maximum 2 { attempt X }` with guard `true` and an `exhausted` control `attempt Z`, where `X` and `Z` are always enabled: the thread rests at `X`, takes `X`, rests at `X` again, takes `X` a second time, and then rests at `Z`; every behaviour takes `X` exactly twice, and `Z` is enabled in no state before the second `X`. With the `exhausted` control replaced by a following sibling `attempt W`, the thread rests at `W` after the second `X`. A `check` whose value is false leaves its thread resting there, with no enabled step for it, in every later state. An `await` with a timeout control gives `timeout(a)` at every state where its thread waits, and the matching event node's step settles the owner into `then`. For a `repeat` with no maximum and no invariant or variant whose guard is `true`, whose body is `attempt X` and whose `exhausted` control is `attempt Z`: the thread rests at `X` again after every `X`, and no reachable state has the thread resting at `Z`. | Test (TC-651) |
| FR-206-AC-6 | Two branches, `a` one always-enabled attempt and `b` a `sequence` of two always-enabled attempts, joined `any … outstanding continue`, then `finish` with constraint `true`: in a state where the root rests at `finish` and `b` is still `running`, `finish` is not enabled; once `b` completes, `finish` is enabled and its step marks the instance finished. | Test (TC-651) |
| FR-206-AC-7 | Over a `fifo` channel `ch` of capacity 2, each `send` binds a send record `{channel, payload, outcome, displaced}` with `channel` in the qualified name. On an empty queue the outcome is `accepted` and `displaced` is none. With the queue holding `P1` then `P2`, `overflow reject` gives `rejected` and `overflow loss new` gives `lost`, each leaving the queue `[P1, P2]`; `overflow loss head` gives `accepted` with `displaced` `P1` and the queue `[P2, new]`; `overflow loss tail` gives `accepted` with `displaced` `P2` and the queue `[P1, new]`. | Test (TC-651) |
| FR-206-AC-8 | A thread resting at `fence F ordering seq_cst;` has one `fence` step, which changes no model state and moves the thread to its next sibling; under `sc` it is enabled in every state. Under `tso` the `seq_cst` fence is enabled exactly when its thread's buffer is empty, and an `acquire` fence is enabled in every state. Two branches that each hold a fence reach no deadlock under `sc` that the same protocol without the fences does not reach. | Test (TC-651) |
| FR-206-AC-9 | `parallel Outer { branch a attempt Fast; branch b sequence { parallel Inner { branch x attempt X1; branch y attempt Y1 } join any [x, y] outstanding continue; attempt After } } join any [a] outstanding cancel`, with every attempt always enabled: in a state where `b` has forked `Inner` and `y` is `running` and `a` has completed, the `join(Outer)` step makes `b`, `x` and `y` `cancelled`, settles and removes `Inner`'s instance, and in no later state does any of them take a step; `Y1` and `After` are enabled in no state after that join. | Test (TC-651) |

## Dependencies

- ADR-027 §2.1 FO-1 and FO-2, §1 PS-6, §2.2 ST-1 to ST-10 and "Waiting", §2.3 AT-1
  to AT-3, §2.5 SE-5 (fences).
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the one
  application rule), [FR-114](FR-114-bind-a-protocol-attempt-to-its-operation-frame.md)
  (an attempt's operation and contracts), FR-205 (state), FR-215 (memory
  model seam), FR-218 (checked constructs).
- QSpec owns the step kinds, their enabledness and effects, folding and
  atomicity, and the untimed reading of `await` (ADR-027 QS-2): QSpec
  FR-426, over FR-052, FR-053, FR-170 and FR-171.

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-426 (Linear STD-140).
