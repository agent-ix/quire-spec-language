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
  - target: ix://agent-ix/quire-specification/FR-146
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-149
    type: depends_on
---
# FR-577: Discharge a postcondition's range obligations from its operation's preconditions

## Description

FR-104 types an arithmetic operand of an equality against an `Int[lo,hi]`
operand by QSpec FR-149's expected-type rule and records the obligation that
the arithmetic node's interval, "under the facts that hold where it
executes", lies inside `[lo,hi]`. Today those facts are only the clause's own
guards, and a guard adds an interval fact only when it compares a stable path
with an integer literal. So the withdrawal postcondition
`self.balance = pre(self.balance) - amount`, over `balance: Int[0, 100]` and
`withdraw(amount: Int[0, 100])` with the precondition
`amount <= self.balance`, refuses `undefined_expression`/`unproved-range`:
the checker sees `pre(self.balance) - amount` in `[-100, 100]`.

A postcondition is required only of an invocation whose effective
precondition held in its pre observation (QSpec's "effective precondition";
QSpec FR-377, FR-468). This requirement gives the checker exactly two more
facts, and nothing broader:

1. the facts an operation's unit preconditions establish, flowing into each
   postcondition of the same operation and context; and
2. a difference bound from an integer ordering between two non-literal
   stable paths, used only to bound a binary `-` of those two paths.

It also states what the refusal carries when the range is still not proved.

## Inputs

- A checked `post` clause on `M::T::op` and every unit `pre` clause of the
  check unit (FR-102, FR-104), with the assembled operations FR-103 admits.
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
  `M::T`, it SHALL begin `Q`'s definedness walk with the conjunction of the
  facts on the true outcome of every unit `pre` clause whose context resolves
  to the same declaration `M::T` and whose operation resolves to the same
  operation declaration (declaring type and name, FR-104 "Resolution"),
  whatever their source order and whichever `using` alias each names. `Q`'s
  own guards then add facts by FR-104's propagation rules.
- A precondition fact SHALL transfer only when it is keyed on a path rooted
  at `self` or at an operation parameter. It SHALL key, in `Q`, the same
  path at the same observation, rooted at `Q`'s `self` or at `Q`'s parameter
  of the same name. Precondition reads are `pre` (FR-104 "Observations of
  reads"). So such a fact matches a read under `pre(e)` in `Q`, never one of
  `Q`'s `post` reads. A parameter is an immutable input of the invocation,
  and its path matches by name.
- A fact keyed on a `let`, quantifier or comprehension binder of the
  precondition SHALL NOT transfer.
- A precondition whose true outcome is unreachable SHALL contribute no fact.
- The checker SHALL NOT carry precondition facts into an invariant, a state
  constraint, another precondition, a postcondition of another operation, or
  a postcondition naming the same operation through another context type
  (a subtype or supertype of `M::T`). It SHALL NOT take facts from a domain
  package's kept `requires` clause (FR-103), and SHALL give none to a kept
  `ensures` clause.
- These facts SHALL change only which obligations FR-104 discharges. They
  SHALL add no node, occurrence, obligation record or requirement record, and
  checking the unit with its clauses in another order SHALL give the same
  result.
- The range proof of a postcondition is a proof only for invocations whose
  preconditions hold. The evaluator SHALL NOT clamp, wrap or widen the
  arithmetic node's value when an evaluation reaches it at an invocation where
  a precondition does not hold. A value outside `[lo,hi]` SHALL refuse at the
  node's checked coercion with the kernel's `IntegerOutOfDomain` refusal,
  exactly as an out-of-range coercion refuses today, so that evaluation
  completes no Boolean.

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
  - the true outcome of `x > y` records `y - x <= -1`;
  - a false outcome records the bound of the negated ordering.
  A second bound on the same ordered pair keeps the smaller `c`. At a join
  only a bound present on every alternative remains, with the larger `c`.
- When the checker computes the interval of a binary `-` node `a - b` whose
  operands, ignoring parentheses and those conversions, are the paths `u` and
  `v` of a recorded bound, it SHALL intersect the exact interval difference
  with that bound. A bound `u - v <= c` gives an upper end of at most `c`, and
  a bound `v - u <= c` gives a lower end of at least `-c`. The refined
  interval is the `-` node's wherever it stands, so an enclosing arithmetic
  node is computed from it.
- No other relational reasoning SHALL be admitted. Each of the following
  yields no fact and refines no interval:
  - `=` or `!=` between two non-literal operands;
  - an ordering with an operand that is an arithmetic node, call, `size` or
    any other non-path form, so `self.balance + amount <= 100` yields none;
  - a bound used for a `+`, `*` or unary `-` node, or for a difference written
    `-b + a`;
  - a transitive chain: `a <= b` and `b <= c` give no bound on `c - a`.
- Difference facts SHALL apply wherever FR-104's definedness check runs: a
  state clause's own guards, transferred precondition facts, and function
  bodies. They SHALL NOT widen the QSpec FR-151 refinement guard facts an
  operation's `own_postcondition_clauses` record (FR-103: `present(self.f)`
  or one ordering against an integer literal).

### Refusal locus and operand

- An unproved obligation SHALL refuse `undefined_expression`/`unproved-range`
  located at the arithmetic node, as FR-104 states.
- The spine check refusal's rendered message SHALL name, for that refusal:
  - the arithmetic node's source text at its region (the operand);
  - the required interval;
  - the proved interval.
  FR-121 carries the region on `CallSiteRefusal::Compile`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-577-AC-1 | Over the bank domain package (model `Bank`, object type `Account` with `balance: Int[0, 100]`, population `accounts`, operation `withdraw(amount: Int[0, 100])` with frame `modifies [Account/balance]`) and a unit with `pre CanAfford using v on Bank::Account::withdraw { amount <= self.balance }`, `post Debited using v on Bank::Account::withdraw { self.balance = pre(self.balance) - amount }` and `invariant NonNegative using v on Bank::Account at current { self.balance >= 0 }`, the unit checks. `pre(self.balance) - amount` is typed `Int[0, 100]`, and `qsl_replay::call_site` with the `Bank::Account::withdraw` selection returns clauses `CanAfford` then `Debited`. The unit with `CanAfford` declared after `Debited` checks with the same clause node identities. | Test (TC-1416) |
| FR-577-AC-2 | The AC-1 unit without `CanAfford` refuses `undefined_expression`/`unproved-range` at `pre(self.balance) - amount`, with required interval `[0, 100]` and proved interval `[-100, 100]`. `qsl_replay::call_site` refuses `CallSiteRefusal::Compile` with code `undefined_expression`, a `region` whose start and end are the byte offsets of the text `pre(self.balance) - amount` in the unit source, and a message containing `pre(self.balance) - amount`, `[0, 100]` and `[-100, 100]`. | Test (TC-1416) |
| FR-577-AC-3 | With `CanAfford`'s body replaced by `amount < self.balance`, and separately by `self.balance >= amount`, the AC-1 unit checks. The AC-2 unit with `Debited`'s body `amount <= pre(self.balance) implies self.balance = pre(self.balance) - amount` checks, with no precondition. | Test (TC-1416) |
| FR-577-AC-4 | Each of the following refuses `undefined_expression`/`unproved-range` at `pre(self.balance) - amount`: the AC-2 unit with `Debited`'s body `amount <= self.balance implies self.balance = pre(self.balance) - amount`, whose guard reads `post`; and the AC-1 unit whose package also declares `deposit(amount: Int[0, 100])`, with `CanAfford` written on `Bank::Account::deposit` instead of `withdraw`. | Test (TC-1416) |
| FR-577-AC-5 | The AC-1 unit refuses `undefined_expression`/`unproved-range` at `pre(self.balance) - amount` when `CanAfford`'s body is `amount = self.balance`, and when it is `amount + 0 <= self.balance`. With `CanAfford` as `self.balance + amount <= 100` and `Debited` replaced by `post Credited using v on Bank::Account::withdraw { self.balance = pre(self.balance) + amount }`, the unit refuses `undefined_expression`/`unproved-range` at `pre(self.balance) + amount`. | Test (TC-1416) |
| FR-577-AC-6 | `qsl_replay::replay_state_clause` over a `Debited` envelope (FR-122) of the AC-1 package whose invocation has pre `balance = 3`, `amount = 5` and post `balance = 0` does not settle a reproduced violation. Its evaluation of `pre(self.balance) - amount` refuses `IntegerOutOfDomain`, and the replay is `inconclusive` with cause `NoValue`. | Test (TC-1416) |

## Dependencies

- FR-104 (state-clause typing, observations and definedness facts), FR-103
  (operations and kept clauses), FR-121 (the `Compile` region), FR-122
  (state-clause replay).
- QSpec FR-149 (the expected-type range obligation) and QSpec FR-146
  ("Accepted proof forms"). FR-146 admits no relational fact and no fact
  across clauses, and states that a producer admits nothing its rules do not
  prove. This requirement depends on a matching QSpec FR-146 amendment that
  admits exactly the two fact forms above.
