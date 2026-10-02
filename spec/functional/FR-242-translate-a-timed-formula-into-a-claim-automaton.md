---
id: FR-242
title: "Translate a timed formula into a claim automaton"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
---
# FR-242: Translate a timed formula into a claim automaton

## Description

EN-6 SHALL translate each timed claim into a claim automaton whose clocks
join the product's zone (ADR-026 EZ-9). A TT-3 or TT-4 formula, which has
no punctual interval, SHALL translate by the MightyL construction from
MITL to timed automata. A TT-2 formula, every interval bounded, SHALL
translate to a timed automaton with an added horizon clock, punctual
intervals included. A TT-1 formula needs no automaton. The translation
answers to the evaluator: the automaton accepts exactly the timed traces on
which the evaluator returns `false`.

## Use case

A verification operator writes `always (holds(req) implies eventually(0 ms,
5 ms] holds(ack))`. The engine checks it over every behaviour of the model
with no step-count encoding, and a bounded deadline with an exact punctual
point is still decided by the horizon construction.

## Inputs

- A checked timed clause with its `TimedPropertyForm` (FR-234).

## Outputs

- A claim automaton: locations, its own clocks, guards and resets over
  them, acceptance sets for TT-3 and TT-4, rejecting locations for TT-2,
  and the constants it adds to the LU bounds (FR-239).

## Behavior

- A `TimedSafety` or `TimedLiveness` formula SHALL translate by the
  MightyL construction for the negation of the formula, giving a timed
  Büchi automaton; its clocks SHALL join the zone of every symbolic state.
- A `BoundedWindow` formula SHALL translate to a timed automaton with one
  added horizon clock, reset at position 0; the product search SHALL stop
  each path when the horizon clock exceeds the formula's horizon. The
  translation SHALL admit punctual intervals in a `BoundedWindow` formula.
- A `TimedInvariant` clause SHALL read its predicate at each symbolic
  state; a data predicate SHALL decide on the discrete state alone.
- Each automaton location's constants SHALL contribute to the LU bounds.
- The engine SHALL materialize automaton states as the product reaches them
  and count them against `max_automaton_states` (FR-239).
- The translation SHALL agree with the evaluator (FR-231, FR-234): for
  every timed trace, the automaton accepts it exactly when the evaluator
  returns `false`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-242-AC-1 | For each of `eventually[0 ms, 3 ms] holds(q)`, `eventually[0 ms, 3 ms) holds(q)`, `holds(p) until(1 ms, 2 ms] holds(q)` and `always (holds(p) implies eventually[0 ms, 2 ms] holds(q))`, on 1,000 random timed lassos with stamps in steps of `1/4` and length at most 8, the automaton accepts exactly the lassos on which the evaluator returns `false`. | Test (TC-697) |
| FR-242-AC-2 | `eventually[2 ms, 2 ms] holds(q)` on origin translates with a horizon clock, and over a model where `q` first holds at exactly time 2 EN-6 returns `Holds`; where it first holds at `5/2`, `Violated`. | Test (TC-697) |
| FR-242-AC-3 | `always holds(not c.late)` under the timed profile builds no automaton and decides on discrete states, with zero automaton states counted. | Test (TC-697) |

## Dependencies

- ADR-026 §8 EZ-9, §6 DF-5; ADR-018 SM-1 and SM-7.
- [FR-234](FR-234-check-timed-intervals-and-classify-timed-property-forms.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md).

## References

- T. Brihaye, G. Geeraerts, H.-M. Ho and B. Monmege, "MightyL: a
  compositional translation from MITL to timed automata", CAV, 2017
  (ADR-026 References).
