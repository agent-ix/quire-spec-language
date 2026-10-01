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

QSL SHALL define a temporal claim's **model subject** and read each of its
behaviours as a trace for the layer-5 TemporalTrace evaluator (ADR-018 SM-1
to SM-5). The evaluator's value on one trace is the semantics: the
model checker (FR-126), replay (FR-128) and monitors all use it. A model
verdict is that value over every admitted behaviour.

## Use case

A verification operator names the model, its initial snapshots and a key
set for each population, and asks whether a temporal claim holds. They
need one definition of what "a behaviour" and "position `i`" mean, so that
a counterexample the model checker reports evaluates false when they replay
it, and a model that halts reads the same way in every engine.

## Inputs

- `ModelSubject` (`qsl_eval::model_check`): the S4 `CheckedPackage`; FR-120's
  package input, snapshot provision and limits; `initial: Vec<DocumentRef>`,
  a non-empty list of FR-106 initial snapshots; `universes:
  Vec<PopulationUniverse>`; and any `ProofBound`s (ADR-014 B-4).
- A checked temporal clause (FR-123) with its `over` parameter.

## Outputs

- `ModelSystem::new` over the subject's members (FR-120), whose successor
  relation is the subject's.
- For a behaviour: a sequence of positions, each an FR-120 state
  observation with an anchor, read by the TemporalTrace evaluator.
- A subject with no initial state is `NoInitialState` (ADR-016 EX-10),
  which FR-127 settles `inconclusive`.

## Behavior

- **Subject.** A subject SHALL be the checked package, its initial states as
  FR-120 admits them, its universes and its `ProofBound`s. The subject is
  part of every obligation identity over it (ADR-013 O-09).
- **Behaviour.** A behaviour SHALL be a maximal path of FR-120's successor
  relation from an initial state: it is infinite, or it ends at a terminal
  state (FR-124).
- **Positions.** Position 0 SHALL be the initial state, observed with an
  `initialization` anchor. Position `i > 0` SHALL be the post-state of the
  behaviour's `i`-th transition, observed with that transition's operation
  anchor. A `holds` atom SHALL evaluate through the one clause evaluator
  (FR-107) over the observation at its position. The step sequence is the
  event-position sequence authority of both admitted profiles.
- **Terminal states under a bounded profile.** A behaviour that ends at a
  terminal state SHALL be read as a closed finite execution, with the
  profile's closed-boundary rule.
- **Terminal states under infinite-trace.** A behaviour that ends at a
  terminal state SHALL be extended by a terminal stutter step that repeats
  the terminal state forever. The stutter step SHALL have its own
  transition identity, enable no operation and belong to no fairness
  constraint. A terminal state reads this way whether it is intended or
  deadlocked (FR-124).
- **Interval operators over a lasso.** Under infinite-trace an interval
  operator SHALL have QSpec FR-091's (future) or FR-092's (past) offset
  meaning. The evaluator SHALL read a position past the represented trace
  as the loop position it wraps to (ADR-014 TR-2). The evaluator SHALL read
  every atomic predicate as false before the first position.
- **Work.** The evaluator SHALL charge one TR-5 work unit for each visit of
  a node at a position.
- **`over` binding.** The checker SHALL instantiate a clause once per
  object of its `over` parameter's population universe. Where the bound
  object does not exist at a position, the evaluator SHALL evaluate a
  `holds` atom over it as FR-107 evaluates a reference to an absent
  object.
- **Model verdict.** A formula SHALL hold for a subject exactly when the
  evaluator returns `true` on every admitted behaviour, for every `over`
  binding: every behaviour under a bounded profile; every behaviour fair
  under the clause's fairness set (FR-126) under infinite-trace.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-125-AC-1 | Over the `Counter` subject (universe `{c}`, initial value 0, no `terminal` member), the one behaviour is positions `0, 1, 2, 3` with anchors `initialization`, `inc`, `inc`, `inc`. Under infinite-trace it continues with the terminal stutter step repeating position 3, whose transition identity differs from every `inc` identity. | Test (TC-520) |
| FR-125-AC-2 | Over that behaviour, `always[0,5] holds(c.value <= 3)` from position 0 evaluates `false` under event-position false-extension (positions 4 and 5 are past closure) and `true` under infinite-trace (the stutter repeats `value = 3`); `eventually holds(c.value = 3)` under infinite-trace evaluates `true`, and `always eventually holds(c.value = 0)` evaluates `false`. | Test (TC-520) |
| FR-125-AC-3 | On ADR-018 §6's lasso (empty prefix, loop `(0,0) (1,0) (2,0)`), `eventually[0,4] holds(a.versionNumber = 0)` at position 2 reads positions 2 to 6 by wrapping and evaluates `true` (position 3 is `(0,0)`); `once[0,3] holds(a.versionNumber = 2)` at position 1 reads positions 1 and 0, the rest before position 0 as false, and evaluates `false`; each visit charges one work unit. | Test (TC-520) |
| FR-125-AC-4 | Over ADR-018 §6's subject, a clause with `over (c: Config::ConfigVersion)` yields one instance per object of `{a, b}`. A subject whose `initial` list is empty is `NoInitialState` before any state is explored. | Test (TC-520) |

## Dependencies

- ADR-018 §2 SM-1 to SM-5, §11 IV-2; ADR-014 §3 TR-2 and TR-5, §5 A-4;
  ADR-016 ID-10 (synthesized state observation) and EX-10.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (successor
  relation), [FR-106](FR-106-admit-snapshots-and-invocations.md) (initial
  snapshots), [FR-107](FR-107-evaluate-state-clauses-at-s6a.md) (clause
  evaluation), [FR-101](FR-101-explore-finite-models-with-canonical-order-and-pinned-sampler.md),
  [FR-123](FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
  (checked clause), [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
  (terminal states).
- QSpec owns the model subject, positions, terminal stutter and `over`
  binding as normative temporal semantics (ADR-018 QS-1, QS-2, QS-5).
