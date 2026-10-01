---
id: US-020
title: "State that something is possible, always still possible, or reachable in one way"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-166
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-167
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-020: State that something is possible, always still possible, or reachable in one way

## Story

**As a** verification operator with a QSL state model, a few initial
snapshots and a finite key set for each population
**I want** to state that a condition can be reached from every start, that
it can still be reached from every reachable state, or that every state of
one kind has exactly one way to reach a state of another kind
**So that** I get `proved` with a path I can replay when the condition is
reachable, `refuted` with a state from which it is not when it is not, and
a result that says how the witness was found: by a sampled walk, with its
seed and trace index, or by exhaustive exploration. A `proved` always rests
on an exploration that found no state where the claim has no value.

## Context

ADR-022 designs these claims. A temporal claim (US-015) is true or false on
each behaviour; these claims quantify over the paths that branch from a
state, so they are a property family of their own. QSL's explicit-state
engine explores the model's state graph once, samples seeded walks first for
`possible`, and decides each claim by one linear pass over the explored
graph.

## Acceptance Examples (Illustrative)

### US-020-EX-1: The game can be won

- **Given** ADR-022 §7.1's ConfigVersion subject and `possible ReachesTwo`
  over each config.
- **When** the operator requests the claim with the default limits.
- **Then** each instance settles `proved`, basis `decisive-witness`, after
  exploration finds no undefined evaluation of the target, and the result
  names the sampled walk, its seed and its trace index, that found the
  witness.

### US-020-EX-2: Winning stops being possible

- **Given** ADR-022 §7.2's game, where `lose` leads to a terminal `Lost`
  state, and `always possible CanStillWin`.
- **When** the operator requests it.
- **Then** it settles `refuted`, basis `closed-scope`, with the trap `Lost`
  and the path `play, lose` to it, and replay explores the trap's closure
  again before the refutation counts.

### US-020-EX-3: Two ways to finish

- **Given** ADR-022 §7.3's job, where `stepA` and `stepB` run in either
  order before `finish`.
- **When** the operator requests `unique path InOneWay`.
- **Then** it settles `refuted` with the two paths `stepA, stepB, finish`
  and `stepB, stepA, finish`.

### US-020-EX-4: A partial search does not read as proof

- **Given** a subject whose exploration reaches `max_depth` before it finds
  a witness or a trap.
- **When** the operator requests a `possible` claim.
- **Then** it settles `inconclusive`, `BoundReached`, never `proved` or
  `refuted`.

## Priority and Risk (Informative)

Priority: High. Possibility and recoverability are statements TLA+ cannot
make natively, and planners and recovery monitors later read the same
witness shape.

## Traceability (Informative)

- [FR-165](../functional/FR-165-check-state-graph-claims-at-s3.md)
- [FR-166](../functional/FR-166-sample-witnesses-for-possible-claims.md)
- [FR-167](../functional/FR-167-explore-a-state-graph-subject-and-track-open-nodes.md)
- [FR-168](../functional/FR-168-decide-state-graph-claims-over-the-explored-graph.md)
- [FR-169](../functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md)
- [FR-170](../functional/FR-170-replay-state-graph-evidence.md)

## References

- ADR-022. Owning ticket: Linear QSL-369. QSpec half: QSpec
  FR-390 to FR-394 (Linear STD-135).
