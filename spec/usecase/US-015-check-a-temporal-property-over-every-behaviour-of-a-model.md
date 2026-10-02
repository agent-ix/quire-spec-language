---
id: US-015
title: "Check a temporal property over every behaviour of a model"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-123
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-128
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-314
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-015: Check a temporal property over every behaviour of a model

## Story

**As a** verification operator with a QSL state model, a few initial
snapshots and a finite key set for each population
**I want** a temporal claim about that model, safety or liveness, bounded or
unbounded, under the fairness I state, checked over every behaviour the
model allows
**So that** I get `proved` when no behaviour violates it, a counterexample I
can replay when one does, and a stated reason when the check cannot decide,
and so that a state where the model gets stuck by mistake is reported to me.

## Context

ADR-018 designs the check. FR-120's `ModelSystem` gives the successor
relation and FR-101's engine explores it, but exploration checks state
invariants only, and a temporal clause is evaluated over one supplied trace.
The explicit-state model checker (ADR-018 EN-1) explores the product of the
model with the clause's property automaton and settles a negotiated item in
QSpec FR-360's vocabulary.

## Acceptance Examples (Illustrative)

### US-015-EX-1: Liveness under fairness is proved

- **Given** the ConfigVersion example unit of ADR-018 §6, universe
  `{a, b}`, and `always eventually holds(c.versionNumber = 2)` under
  `fair weak each attemptUpdate`.
- **When** the operator requests the claim.
- **Then** it settles `proved`, basis `closed-scope`, over that subject.

### US-015-EX-2: The weaker fairness premise gives a replayable lasso

- **Given** the same claim under the unmarked `fair weak attemptUpdate`.
- **When** the operator requests it.
- **Then** it settles `refuted` with a three-step lasso of `upd(a)` steps,
  and replaying the lasso through the model reproduces the refutation.

### US-015-EX-3: A stuck state is reported

- **Given** a model with a reachable state where no operation is enabled
  and no `terminal` member.
- **When** the operator requests any temporal claim over it.
- **Then** the request also carries the subject's deadlock-freedom item,
  which settles `refuted` with a prefix to the stuck state.

### US-015-EX-4: Recovery stability is checked

- **Given** `always (holds(not s.healthy) implies eventually always[0,2]
  holds(s.healthy))` under infinite-trace.
- **When** the operator requests it.
- **Then** it is admitted, and it settles on the model's behaviours like
  any other liveness claim.

## Priority and Risk (Informative)

Priority: High. Every-behaviour verdicts for temporal claims are what TLA+
users expect of a specification language, and the language-feature work
that follows (strong fairness, refinement, reduction, branching queries)
builds on this check.

## Traceability (Informative)

- [FR-123](../functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md)
- [FR-124](../functional/FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md)
- [FR-125](../functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
- [FR-126](../functional/FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
- [FR-127](../functional/FR-127-settle-a-model-check-verdict-as-a-terminal-record.md)
- [FR-128](../functional/FR-128-replay-a-model-counterexample.md)
- [FR-337](../functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md)
- [FR-338](../functional/FR-338-check-an-en-1-closure-certificate.md)
- [FR-339](../functional/FR-339-check-an-en-1-component-certificate.md)
- [FR-314](../functional/FR-314-check-an-smt-proof-certificate.md)
