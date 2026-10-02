---
id: FR-125
title: "Read a model subject's behaviours as temporal traces"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-015
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-016
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: depends_on
---
# FR-125: Read a model subject's behaviours as temporal traces

## Description

QSL SHALL read each behaviour of a temporal claim's **model subject** as a
trace for the layer-5 TemporalTrace evaluator, by the QSpec model-subject
semantics (ADR-018 SM-1 to SM-5, QS-1, QS-2, QS-5). The evaluator's value on
one trace is the semantics: the model checker (FR-126), replay (FR-128) and
monitors all use it. The evaluator SHALL read a lasso exactly, past
operators included (ADR-018 SM-8). A model verdict is that value over every
admitted behaviour.

## Use case

A verification operator names the model, its initial snapshots and a key
set for each population, and asks whether a temporal claim holds. They
need one definition of what "a behaviour" and "position `i`" mean, so that
a counterexample the model checker reports evaluates false when they replay
it, and a model that halts reads the same way in every engine.

## Semantic authority and boundary

QSpec owns the model subject, behaviours, positions and their anchors, the
terminal-state readings, the `over` binding and the meaning of interval
operators over an infinite trace (ADR-018 QS-1, QS-2, QS-5, QS-13;
References). The Behavior section restates those rules informatively, as
ADR-018 §2 states them, so that the QSL requirements below have their
terms; QSpec's text is the authority where the two differ. QSL specifies
normatively how it builds the subject from FR-120, how its evaluator reads
a lasso, what it charges and what it reports.

## Inputs

- `ModelSubject` (`qsl_eval::model_check`): the S4 `CheckedPackage`; FR-120's
  package input, snapshot provision and limits; `initial: Vec<DocumentRef>`,
  a list of FR-106 initial snapshots; `universes:
  Vec<PopulationUniverse>`; and any `ProofBound`s (ADR-014 B-4).
- A checked temporal clause (FR-123) with its `over` parameter.

## Outputs

- `ModelSystem::new` over the subject's members (FR-120), whose successor
  relation is the subject's.
- For a behaviour: a sequence of positions, each an FR-120 state
  observation with an anchor, read by the TemporalTrace evaluator.
- `NoInitialState` (ADR-016 EX-10) for a subject with no initial state,
  which FR-127 settles `inconclusive`.

## Behavior

### QSpec model-subject rules (informative restatement)

- **Subject.** The checked package, its initial states as FR-120 admits
  them, its universes and its `ProofBound`s. The subject is part of every
  obligation identity over it (ADR-013 O-09).
- **Behaviour.** A maximal path of the successor relation from an initial
  state: infinite, or ending at a terminal state (FR-124).
- **Positions.** Position 0 is the initial state with an `initialization`
  anchor; position `i > 0` is the post-state of the behaviour's `i`-th
  transition with that transition's operation anchor. The step sequence is
  the event-position sequence authority of both admitted profiles.
- **Terminal states.** Under a bounded profile a behaviour that ends at a
  terminal state is a closed finite execution, read with the profile's
  closed-boundary rule. Under infinite-trace it is extended by a terminal
  stutter step that repeats the terminal state forever, has its own
  transition identity, enables no operation and belongs to no fairness
  constraint. Intended and deadlocked terminal states read the same way
  (ADR-018 DL-6).
- **`over` binding.** One clause instance per object of the `over`
  parameter's population universe.
- **Model verdict.** A formula holds for a subject exactly when the
  evaluator returns `true` on every admitted behaviour, for every `over`
  binding: every behaviour under a bounded profile; every behaviour fair
  under the clause's fairness set under infinite-trace.

### Building the subject

- QSL SHALL build the subject's successor relation as `ModelSystem::new`
  over the subject's package, snapshots, initial states and universes
  (FR-120), and SHALL read a behaviour's positions as FR-120's synthesized
  state observations (ADR-016 ID-10), each with the anchor of the
  transition that produced it.
- The evaluator SHALL evaluate a `holds` atom through the one clause
  evaluator (FR-107) over the observation at its position.
- The evaluator SHALL evaluate a `holds` atom over an object that does not
  exist at a position as FR-107 evaluates a reference to an absent object.
- When the subject's `initial` list is empty, QSL SHALL report
  `NoInitialState` before it explores any state.

### Reading a lasso

- For a lasso with a prefix of `n` positions and a loop of `L` positions,
  the evaluator SHALL read state (atoms, the observation and its anchor) at
  a position `p >= n` from position `n + ((p - n) mod L)`.
- The evaluator SHALL compute each subformula's past reach `R` by ADR-018
  SM-8: 0 for an atom; the largest operand reach for a Boolean or future
  operator; `R + 1` for a previous-position operator; `R + b` for a past
  interval operator with upper bound `b`, with the larger operand reach for
  `since[a,b]` and `triggered[a,b]`; and the largest operand reach plus `L`
  for an unbounded `once`, `historically`, `since` or `triggered`.
- The evaluator SHALL unroll the loop `m = ceil(R(φ) / L) + 1` times for a
  formula `φ`, evaluate every subformula over the `n + m × L` unrolled
  positions with past operators reading back to position 0, and read a
  subformula at a position past the unrolled positions at the position of
  the last unrolled loop copy congruent to it modulo `L`.
- The evaluator SHALL read every atomic predicate as false before position
  0, QSpec FR-092's complete-history rule.
- Under infinite-trace the evaluator SHALL give an interval operator QSpec
  FR-091's (future) or FR-092's (past) offset meaning, with no
  closed-boundary rule (ADR-018 IV-2).

### Undefined evaluation

- At each position a claim reads, the evaluator SHALL read the claim's
  letter: the value of every atom of the claim at that position, and, for
  the deadlock-freedom item, the value of DL-1's `P` (FR-124's `When(P)`)
  at a terminal state (ADR-018 UE-1).
- A TP-2 `on origin` claim SHALL read positions 0 to its horizon `h`;
  every other claim SHALL read every position of the behaviour. A position
  that a bounded profile's closed-boundary rule supplies past a terminal
  state SHALL evaluate no atom.
- When an atom evaluates `Undefined` (the kernel `Outcome::Undefined` or a
  family's `FamilyResult::Undefined`) at a position the claim reads, the
  evaluator SHALL return the claim undefined at that position, with the
  `UndefinedRecord` of the first such atom in clause-node order as its
  cause.

### Work

- The evaluator SHALL charge one TR-5 work unit for each visit of a node at
  a position, unrolled positions included.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-125-AC-1 | Over the `Counter` subject (universe `{c}`, initial value 0, no `terminal` member), the one behaviour is positions `0, 1, 2, 3` with anchors `initialization`, `inc`, `inc`, `inc`. Under infinite-trace it continues with the terminal stutter step repeating position 3, whose transition identity differs from every `inc` identity. | Test (TC-520) |
| FR-125-AC-2 | Over that behaviour, `always[0,5] holds(c.value <= 3)` from position 0 evaluates `false` under event-position false-extension (positions 4 and 5 are past closure) and `true` under infinite-trace (the stutter repeats `value = 3`); `eventually holds(c.value = 3)` under infinite-trace evaluates `true`, and `always eventually holds(c.value = 0)` evaluates `false`. | Test (TC-520) |
| FR-125-AC-3 | On ADR-018 §6's lasso (empty prefix, loop `(0,0) (1,0) (2,0)`), `eventually[0,4] holds(a.versionNumber = 0)` at position 2 reads the states of positions 2 to 6 by wrapping and evaluates `true` (position 3 holds `(0,0)`); `once[0,3] holds(a.versionNumber = 2)` at position 1 reads positions 1 and 0, the rest before position 0 as false, and evaluates `false`; each visit charges one work unit. | Test (TC-520) |
| FR-125-AC-4 | Over ADR-018 §6's subject, a clause with `over (c: Config::ConfigVersion)` yields one instance per object of `{a, b}`. A subject whose `initial` list is empty is `NoInitialState` before any state is explored. | Test (TC-520) |
| FR-125-AC-5 | On the same lasso, `always eventually (holds(a.versionNumber = 0) and once[1,1] holds(a.versionNumber = 2))` has past reach 1 and is evaluated over `m = 2` loop copies, 6 positions: the conjunction is `false` at position 0, which has no previous position, and `true` at position 3, whose previous position holds version 2, and the formula evaluates `true`. The same conjunction read at position 9 by congruence is `true`. | Test (TC-520) |
| FR-125-AC-6 | Over the `Counter` subject, the letter of `always holds(6 / (2 - c.value) >= 0)` under infinite-trace is defined at positions 0 and 1 and undefined at position 2, cause `division-by-zero`. `eventually[0,1] holds(6 / (2 - c.value) = 6)` under event-position false-extension, `on origin`, reads positions 0 and 1 only and evaluates `true`. | Test (TC-537) |

## Dependencies

- ADR-018 §1 UE-1, §2 SM-1 to SM-5 and SM-8, §10 DL-6, §11 IV-2; ADR-014 §3 TR-2 and TR-5, §5 A-4;
  ADR-016 ID-10 (synthesized state observation) and EX-10.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (successor
  relation), [FR-106](FR-106-admit-snapshots-and-invocations.md) (initial
  snapshots), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (clause
  evaluation), [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (checked clause), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (terminal states).

## References

- QSpec FR-361 (model behaviours read as temporal traces) and FR-367
  (interval operators under infinite-trace): the QSpec half of ADR-018 QS-1,
  QS-2, QS-5 and QS-13 (Linear STD-131).
