---
id: FR-577
title: "Discharge a postcondition's range obligations from its operation's preconditions"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-003
    type: implements
  - target: ix://agent-ix/quire-spec-language/FR-104
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-569
    type: references
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-149
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-151
    type: depends_on
---
# FR-577: Discharge a postcondition's range obligations from its operation's preconditions

## Description

FR-104 types an arithmetic operand of an equality against an `Int[lo,hi]`
operand by QSpec FR-149's expected-type rule and records the obligation that
the arithmetic node's interval, "under the facts that hold where it
executes", lies inside `[lo,hi]`. The facts that hold where a postcondition
executes are its own guard facts, two more fact forms this requirement
defines, and nothing broader:

1. the facts an operation's unit preconditions establish, which flow into
   each postcondition of the same operation and context when the operation
   is not part of a redefinition family; and
2. a difference bound from an integer ordering between two non-literal
   stable paths, used only to bound a binary `-` of those two paths.

So the withdrawal postcondition `self.balance = pre(self.balance) - amount`,
over `balance: Int[0, 100]` and `withdraw(amount: Int[0, 100])` with the
precondition `amount <= self.balance`, checks: the precondition's bound
`amount - self.balance <= 0` at the `pre` observation gives
`pre(self.balance) - amount` the interval `[0, 100]`.

The transfer rests on QSpec FR-151's effective precondition: an operation
`m` that redefines nothing and is redefined by nothing has
`eff(m) = own(m)`, the conjunction of its own precondition clauses, and its
postcondition is required only where `eff(m)` held in the pre observation.
For a redefinition family QSpec FR-151 gives `eff(m) = own(m) or eff(r)`
(FR-569), so a unit precondition need not hold where the postcondition is
required, and no fact transfers. A protocol attempt (FR-206) evaluates the
postconditions its `contracts` list selects without the preconditions it
omits. Its evaluation is defined by the checked coercion the range
obligation sits on, not by the static proof (see "Evaluation outside the
precondition").

This requirement does not change FR-103's refinement guard facts
(`own_postcondition_clauses`), which QSpec FR-151 owns.

## Inputs

- A checked `post` clause on `M::T::op` and every unit `pre` clause of the
  check unit (FR-102, FR-104), with the assembled operations and their
  `redefines` keys that FR-103 admits.
- FR-104's definedness facts: interval, presence, nonzero and membership
  facts keyed by (observation, path).

## Outputs

- The same checked clauses, nodes, identities and requirement records FR-104
  produces: an obligation this requirement discharges is accepted, and one it
  does not discharge is refused exactly as FR-104 refuses it.
- An `undefined_expression`/`unproved-range` refusal whose rendered message
  names the unproved operand and both intervals.

## Behavior

### Precondition facts

- When the checker checks a `post` clause `Q` on operation `op` of context
  `M::T`, and `op`'s declaration neither redefines another operation nor is
  redefined by an operation of the effective view (QSpec FR-151, FR-103
  `redefines`), the checker SHALL begin `Q`'s definedness walk with the
  conjunction of the facts on the true outcome of every unit `pre` clause
  whose context resolves to the same declaration `M::T` and whose operation
  resolves to the same operation declaration (FR-104 "Resolution"), whatever
  their source order and whichever `using` alias each names. `Q`'s own guards
  then add facts by FR-104's propagation rules.
- When `op`'s declaration redefines another operation or is redefined by one,
  the checker SHALL transfer no precondition fact into `Q`.
- In that conjunction the checker SHALL intersect the interval facts of one
  (observation, path) key and keep, for one ordered pair, the difference
  bound with the smaller `c`. If an intersection is empty, the checker SHALL
  treat the conjunction as unreachable and transfer no fact from it.
- The checker SHALL transfer a precondition fact only when it is keyed on a
  path rooted at `self` or at an operation parameter.
- The checker SHALL key a transferred fact in `Q` on the same path at the same
  observation, rooted at `Q`'s `self` or at `Q`'s parameter of the same name.
- The checker SHALL NOT transfer a fact keyed on a path rooted at a `let`,
  quantifier or comprehension binder of the precondition. A `let` that
  aliases a stable path keys its facts on the aliased path (FR-104), so those
  facts are keyed at `self` or a parameter.
- The checker SHALL transfer no fact from a precondition whose true outcome is
  unreachable.
- The checker SHALL NOT carry precondition facts into an invariant, a state
  constraint, another precondition, a postcondition of another operation, or
  a postcondition naming the same operation through another context type
  (a subtype or supertype of `M::T`).
- The checker SHALL NOT take facts from a domain package's kept `requires`
  clause (FR-103), and SHALL give none to a kept `ensures` clause.
- The checker SHALL add no node, occurrence, obligation record or
  requirement record for a transferred fact.

Precondition reads are `pre` (FR-104 "Observations of reads"), so a
transferred fact matches a read under `pre(e)` in `Q` and never one of `Q`'s
`post` reads. A parameter is an immutable input of the invocation.

### Evaluation outside the precondition

- When an evaluation of `Q` reaches an `Int[lo,hi]` arithmetic node whose
  value lies outside `[lo,hi]`, the evaluator SHALL refuse at the node's
  checked coercion with the kernel's `IntegerOutOfDomain` refusal, and SHALL
  NOT clamp, wrap or widen the value. This holds wherever the evaluation
  occurs, including an invocation where a precondition does not hold.
- When a protocol attempt (FR-206) whose `contracts` list selects `Q` but
  omits a precondition that discharged `Q`'s obligation is applied where that
  precondition is false, the protocol system SHALL record the refusal as
  FR-120's `ModelFinding::ContractUndetermined` for `Q` and give that
  application no successor. Such an attempt is defined, not refused at check.

### Difference facts

- A difference subject is an integer ordering `x op y`, `op` one of `<`, `<=`,
  `>` or `>=`, where `x` and `y`, through the conversions FR-104 reads
  through, are each a path an integer interval fact is keyed on (FR-104
  "Definedness facts") and neither is an integer literal. On each outcome the
  checker SHALL record one difference bound `u - v <= c` over the two
  (observation, path) keys:
  - the true outcome of `x <= y` records `x - y <= 0`;
  - the true outcome of `x < y` records `x - y <= -1`;
  - the true outcome of `x >= y` records `y - x <= 0`;
  - the true outcome of `x > y` records `y - x <= -1`.
- On a false outcome the checker SHALL record the bound of the negated
  ordering.
- When a second bound on the same ordered pair holds on one path, the checker
  SHALL keep the bound with the smaller `c`.
- At a join the checker SHALL keep only a bound present on every
  alternative, with the larger `c`.
- When the checker computes the interval of a binary `-` node `a - b` whose
  operands, ignoring parentheses and those conversions, are the paths `u` and
  `v` of a recorded bound, it SHALL intersect the exact interval difference
  with that bound. A bound `u - v <= c` gives an upper end of at most `c`, and
  a bound `v - u <= c` gives a lower end of at least `-c`. The refined
  interval is the `-` node's wherever it stands, so an enclosing arithmetic
  node is computed from it.
- The checker SHALL admit no other relational reasoning. Each of the
  following yields no fact and refines no interval:
  - `=` or `!=` between two non-literal operands;
  - an ordering with an operand that is an arithmetic node, call, `size` or
    any other non-path form, so `self.balance + amount <= 100` yields none;
  - a bound used for a `+`, `*` or unary `-` node, or for a difference written
    `-b + a`;
  - a transitive chain: `a <= b` and `b <= c` give no bound on `c - a`.
- The checker SHALL apply difference facts wherever FR-104's definedness
  check runs: a state clause's own guards, transferred precondition facts,
  and function bodies.

### Refusal locus and operand

- The checker SHALL refuse an unproved obligation
  `undefined_expression`/`unproved-range` located at the arithmetic node, as
  FR-104 states.
- The spine check refusal's rendered message SHALL name, for that refusal,
  the arithmetic node's source text at its region (the operand), the
  required interval and the proved interval, in that order.
- The message SHALL spell an interval `[lo, hi]`: each end a base-10 integer
  with a leading `-` when negative, the ends separated by `, `, an unbounded
  lower end `-inf` and an unbounded upper end `+inf`.

FR-121 carries the region on `CallSiteRefusal::Compile`.

## Acceptance Criteria

The bank fixture: domain package `test/bank` with model `Bank`, object type
`Account` (`balance: Int[0, 100]`, `limit: Int[0, 100]`), population
`accounts`, and operation `withdraw(amount: Int[0, 100])` with frame
`modifies [Account/balance]`. The bank unit: `pre CanAfford using v on Bank::Account::withdraw { amount <= self.balance }`,
`post Debited using v on Bank::Account::withdraw { self.balance = pre(self.balance) - amount }`
and `invariant NonNegative using v on Bank::Account at current { self.balance >= 0 }`.
`Debited1` is `post Debited1 using v on Bank::Account::withdraw { self.balance = (pre(self.balance) - amount) - 1 }`.
"Refuses at `D`" means refuses `undefined_expression`/`unproved-range` with
the refusal located at the node `D`.

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-577-AC-1 | The bank unit checks, with `pre(self.balance) - amount` typed `Int[0, 100]`. | Test (TC-1416) |
| FR-577-AC-2 | The bank unit with `CanAfford` declared after `Debited` checks with the same clause node identities as the bank unit. | Test (TC-1416) |
| FR-577-AC-3 | The bank unit without `CanAfford` refuses at `pre(self.balance) - amount`, with required interval `[0, 100]` and proved interval `[-100, 100]`. | Test (TC-1416) |
| FR-577-AC-4 | For the AC-3 unit, `qsl_replay::call_site` refuses `CallSiteRefusal::Compile` with code `undefined_expression` and a message containing, in order, `pre(self.balance) - amount`, `[0, 100]` and `[-100, 100]`. | Test (TC-1416) |
| FR-577-AC-5 | `function g using v(n: Integer, r: Int[0, 100]): Boolean pure { r = n - 1 }` refuses at `n - 1`, and the `call_site` `Compile` message contains `[-inf, +inf]`. | Test (TC-1416) |
| FR-577-AC-6 | The bank unit with `Debited` replaced by `Debited1` and `CanAfford`'s body `amount < self.balance` checks. | Test (TC-1416) |
| FR-577-AC-7 | The bank unit with `CanAfford`'s body `self.balance >= amount` checks. | Test (TC-1416) |
| FR-577-AC-8 | The bank unit with `CanAfford`'s body `not (amount > self.balance)` checks. | Test (TC-1416) |
| FR-577-AC-9 | The bank unit with `Debited` replaced by `Debited1` and `CanAfford`'s body `amount <= self.balance and amount < self.balance` checks. | Test (TC-1416) |
| FR-577-AC-10 | The bank unit with `Debited` replaced by `Debited1`, `CanAfford` kept and `pre Strict using v on Bank::Account::withdraw { amount < self.balance }` added checks. | Test (TC-1416) |
| FR-577-AC-11 | The bank unit with `Debited` replaced by `Debited1` and `CanAfford`'s body `amount < self.balance or amount <= self.balance` refuses at `(pre(self.balance) - amount) - 1`. | Test (TC-1416) |
| FR-577-AC-12 | The bank unit with `CanAfford`'s body `amount <= self.balance or amount <= 5` refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-13 | The AC-3 unit with `Debited`'s body `amount <= pre(self.balance) implies self.balance = pre(self.balance) - amount` checks. | Test (TC-1416) |
| FR-577-AC-14 | The AC-3 unit with `Debited`'s body `amount <= self.balance implies self.balance = pre(self.balance) - amount`, whose guard reads `post`, refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-15 | Over the bank fixture with `deposit(amount: Int[0, 100])` added, the bank unit with `CanAfford` on `Bank::Account::deposit` refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-16 | The bank unit with `CanAfford`'s body `amount = self.balance` refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-17 | The bank unit with `CanAfford`'s body `amount + 0 <= self.balance` refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-18 | The bank unit with `CanAfford`'s body `self.balance + amount <= 100` and `Debited` replaced by `post Credited using v on Bank::Account::withdraw { self.balance = pre(self.balance) + amount }` refuses at `pre(self.balance) + amount`. | Test (TC-1416) |
| FR-577-AC-19 | The bank unit with `CanAfford`'s body `amount <= self.limit and self.limit <= self.balance` refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-20 | The bank unit with `CanAfford`'s body `let x = amount + 1 in x <= self.balance` and `Debited`'s body `let x = amount + 1 in self.balance = pre(self.balance) - x` refuses at `pre(self.balance) - x`. | Test (TC-1416) |
| FR-577-AC-21 | The bank unit with `CanAfford`'s body `let b = self.balance in amount <= b` checks. | Test (TC-1416) |
| FR-577-AC-22 | The bank unit with `CanAfford`'s body `amount < 0 and amount <= self.balance`, whose true outcome is unreachable, refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-23 | The bank unit with `CanAfford`'s body `amount <= 3 and amount <= self.balance` and `pre Big using v on Bank::Account::withdraw { amount >= 5 }` added refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-24 | Over the bank fixture with `Premium` a subtype of `Account` that inherits `withdraw`, the bank unit with `CanAfford` on `Bank::Premium::withdraw` refuses at `pre(self.balance) - amount` in `Debited`. | Test (TC-1416) |
| FR-577-AC-25 | Over the AC-24 fixture, the bank unit with `Debited` on `Bank::Premium::withdraw` refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-26 | Over the AC-24 fixture with `Premium` redeclaring `withdraw(amount: Int[0, 100])` with `redefines` `Account/withdraw` and frame `modifies [Account/balance]`, the bank unit refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-27 | Over the AC-26 fixture, the bank unit with `CanAfford` and `Debited` both on `Bank::Premium::withdraw` refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-28 | The bank unit with `pre Fits using v on Bank::Account::withdraw { self.limit = self.balance - amount }` added refuses at `self.balance - amount` in `Fits`. | Test (TC-1416) |
| FR-577-AC-29 | The bank unit with `CanAfford`'s body `self.limit <= self.balance` and `invariant Room using v on Bank::Account at current { self.limit = self.balance - self.limit }` added refuses at `self.balance - self.limit`. | Test (TC-1416) |
| FR-577-AC-30 | Over the bank fixture whose `withdraw` keeps the `quire` `requires` clause `amount <= self.balance`, the AC-3 unit refuses at `pre(self.balance) - amount`. | Test (TC-1416) |
| FR-577-AC-31 | `function f using v(a: Int[0, 100], b: Int[0, 100], r: Int[0, 100]): Boolean pure { a <= b implies r = b - a }` checks. | Test (TC-1416) |
| FR-577-AC-32 | `qsl_replay::replay_state_clause` over a `Debited` envelope (FR-122) of the bank unit's package whose invocation has pre `balance = 3`, `amount = 5` and post `balance = 0` refuses `IntegerOutOfDomain` at `pre(self.balance) - amount` and settles `inconclusive` with cause `NoValue`, not a reproduced violation. | Test (TC-1416) |
| FR-577-AC-33 | A protocol over the bank package whose attempt of `withdraw` lists `contracts` `[Debited]` only, applied at a state with `balance = 3` and the argument `amount = 5`, records one `ModelFinding::ContractUndetermined` for `Debited` and gives that application no successor. | Test (TC-1416) |

## Dependencies

- FR-104 (state-clause typing, observations and definedness facts), FR-103
  (operations, `redefines` and kept clauses), FR-121 (the `Compile` region),
  FR-122 (state-clause replay), FR-120 and FR-206 (application and protocol
  attempts), FR-569 (effective preconditions of source-declared families).
- QSpec FR-149 (the expected-type range obligation), QSpec FR-151 (the
  effective precondition `eff(m)`) and QSpec FR-146 ("Accepted proof
  forms"). FR-146 admits no relational fact and no fact across clauses, and
  states that a producer admits nothing its rules do not prove. This
  requirement depends on a matching QSpec FR-146 amendment that admits
  exactly the two fact forms above.
