---
id: US-021
title: "Compare runs over every behaviour of a model"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-171
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-173
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-174
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-175
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-176
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-177
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-178
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-179
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-180
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-181
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-182
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-183
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-184
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-021: Compare runs over every behaviour of a model

## Story

**As a** verification operator with one or two QSL state models, their
initial snapshots and a finite key set for each population
**I want** to state a property that compares runs, such as "runs with the
same public inputs give the same public outputs" or "for every run there is
another run with the other secret that looks the same", and have it checked
over every behaviour of the models, with the self-composition built by the
compiler
**So that** I get `proved` when no tuple of runs violates it, a tuple of
traces I can replay when one does, and a stated reason, naming the limit
and how to raise it, when the check cannot decide.

## Context

ADR-023 designs the check. QSpec defines relational properties and
hyperproperties over declared finite trace sets; ADR-023 extends both to the
behaviours of model subjects (ADR-018). The explicit-state engine checks
universal properties by self-composition, `∀∃` properties with a safety body
by a witness-set construction, step relations by enumerating tuples of
reachable transitions, and single-existential claims through the possible
family (ADR-022).

## Acceptance Examples (Illustrative)

### US-021-EX-1: Noninterference is refuted on the leaky vault

- **Given** ADR-023 §8's vault with the leaky post clause and §1's
  `NonInterference` clause.
- **When** the operator requests it.
- **Then** it settles `refuted` with two lockstep traces that start with
  different secrets, take the same inputs and differ in `l`, and replaying
  them reproduces the refutation.

### US-021-EX-2: Noninterference is proved on the secure vault

- **Given** the secure post clause.
- **When** the operator requests the same clause.
- **Then** it settles `proved`, and the compiler halved the pair product by
  copy-swap symmetry without the operator declaring it.

### US-021-EX-3: The secret is not revealed

- **Given** ADR-023 §8.2's `Opaque` clause, `forall a exists b`.
- **When** the operator requests it over the leaky vault.
- **Then** it settles `refuted` with one trace of `a` and the position at
  which no matching run `b` is left, and replay recomputes that absence.

### US-021-EX-4: Internal steps do not read as a leak

- **Given** ADR-023 §13's vault where the secret adds internal `mix` steps.
- **When** the operator requests noninterference with `align skip {
  V::Vault::mix }`.
- **Then** it settles `proved`, while the lockstep clause settles `refuted`.

### US-021-EX-5: A budget stops the run and says so

- **Given** a `∀∃` clause whose witness set grows past `max_witness_set`.
- **When** the operator requests it.
- **Then** it settles `failed`, `resource-incomplete`, naming
  `max_witness_set`, its value and how to raise it.

## Priority and Risk (Informative)

Priority: High. Noninterference and input-matched comparisons are what
security and efficiency claims need, and hand-written self-composition hides
the property and squares the model.

## Traceability (Informative)

- [FR-171](../functional/FR-171-build-forms-for-hyper-clauses-over-behaviours.md)
- [FR-172](../functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md)
- [FR-173](../functional/FR-173-classify-hyper-clauses-into-forms.md)
- [FR-174](../functional/FR-174-reduce-hyper-products-and-check-copy-swap-symmetry.md)
- [FR-175](../functional/FR-175-evaluate-a-body-over-a-tuple-of-behaviours.md)
- [FR-176](../functional/FR-176-check-a-universal-hyperproperty-by-self-composition.md)
- [FR-177](../functional/FR-177-check-a-forall-exists-safety-hyperproperty-by-witness-sets.md)
- [FR-178](../functional/FR-178-check-a-projection-aligned-hyperproperty.md)
- [FR-179](../functional/FR-179-check-a-step-relation-over-reachable-transitions.md)
- [FR-180](../functional/FR-180-hand-over-a-step-relation-code-claim.md)
- [FR-181](../functional/FR-181-check-a-single-existential-claim-through-the-possible-family.md)
- [FR-182](../functional/FR-182-settle-a-hyper-verdict.md)
- [FR-183](../functional/FR-183-replay-a-hyper-counterexample.md)
- [FR-184](../functional/FR-184-bound-hyper-runs-with-caller-budgets.md)

## References

- ADR-023. Owning ticket: Linear QSL-370. QSpec half: QSpec
  FR-395 to FR-403 (Linear STD-136).
