---
id: FR-141
title: "Decide one concrete step against the abstract model (check_step)"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-018
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: depends_on
---
# FR-141: Decide one concrete step against the abstract model (check_step)

## Description

QSL's layer-5 `model_check` module SHALL hold one function that decides
each concrete step of a refinement, `check_step`, and one that decides each
concrete initial state, `check_initial` (ADR-020 RS-1 to RS-6, AX-2). The
explicit-state product (FR-142, FR-143) and replay (FR-145) call these and
no other step rule, as every temporal engine answers to the trace evaluator
(ADR-018 SM-1). The step semantics they implement is QSpec's (ADR-020 QS-2).

## Use case

A concrete step is mapped to an abstract increment. The step check computes
the abstract states before and after, asks the abstract model whether that
increment takes the one to the other, and, when it does not, says which
clause rejected it. The same answer comes from the model checker and from
replay.

## Inputs

```rust
pub struct RefinementPosition {
    pub concrete: ModelState,         // or a protocol state over a protocol subject
    pub history: HistoryValues,       // FR-138
    pub abstract_states: Candidates,  // one state, or the AX-2 set with hidden fields
}

pub fn check_initial(
    refinement: &CheckedRefinement,
    abstract_system: &ModelSystem<'_>,
    concrete_initial: &ModelState,
) -> StepVerdict;

pub fn check_step(
    refinement: &CheckedRefinement,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep,              // transition identity, post-state, result
) -> StepVerdict;
```

## Outputs

```rust
pub enum StepVerdict {
    Passes { post: RefinementPosition, taken: Taken },
    Fails(RefinementFailure),
    Undetermined(InconclusiveCause),  // MappingUndetermined, UndecidedSuccessor
}

pub enum Taken { Stutter, Abstract(Vec<ModelTransition>) }  // abstract identities matched
```

`RefinementFailure` is ADR-020 RC-1's: `InitialNotAbstract{initial}`,
`StutterChanged{position}`, `AbstractStepRejected{position, transition,
cause}` with cause `Precondition{clause}`, `Frame{code}` or
`Postcondition`, `NoAbstractMatch{position}`, and the liveness kinds of
FR-143.

## Behavior

### Initial states (RS-2)

- `check_initial` SHALL compute `h0` by FR-138's `initial_history` and
  `map(c0, h0)` by FR-140.
- With no hidden field, it SHALL pass when the mapped state's key equals
  the key of one initial state of the abstract subject (its own FR-106
  snapshots), with `abstract_states` that one state.
- With hidden fields, its candidates SHALL be every abstract initial state
  whose visible fields equal the mapped ones, and it SHALL pass when there
  is at least one.
- Otherwise it SHALL return `Fails(InitialNotAbstract{initial})`.

### Selecting the row

- `check_step` SHALL select the step's row (FR-136): over a model subject,
  its operation's row; over a protocol subject, the row of its step class,
  with an attempt node's row taking precedence over its operation's row
  and a compensation template's row over its operation's row (ADR-027
  PR-1). The concrete terminal stutter step (ADR-018 SM-4, FR-125) SHALL be
  decided as a `stutter` row (RS-6).
- It SHALL compute the post history by FR-138's `step_history` and the post
  mapped state by FR-140. A `MappingUndetermined` from either SHALL return
  `Undetermined(MappingUndetermined)`.

### Rules, with no hidden field

- **`stutter` (RS-3).** The step SHALL pass when the post mapped state's key
  equals the pre mapped state's key, with `Taken::Stutter`; otherwise it
  SHALL fail `StutterChanged{position}`.
- **Abstract operation (RS-4).** The checker SHALL evaluate the row's
  receiver and arguments at the pre-state (with the concrete arguments and
  history), giving one abstract transition identity, or one per value of
  each `_` argument's parameter domain in FR-120's canonical order. For
  each, it SHALL make the calls `ModelSystem` makes when it expands the
  pre mapped state `a`, applied to the one candidate post-state `a'`: the
  effective precondition at `a`, `StateModel::check_frame` over `(a, a')`,
  and the effective postcondition over `(a, a', delta)`, with the abstract
  result bound to the concrete step's result when the row has
  `binds_result`, and otherwise true for some value of the result domain.
  The step SHALL pass when one identity is accepted, with `Taken::Abstract`
  listing every accepted identity. Otherwise it SHALL fail
  `AbstractStepRejected` with the first identity in canonical order and the
  first check that rejected it: `Precondition{clause}`, `Frame{code}` or
  `Postcondition`.
- **`any` (RS-5).** The step SHALL pass when it passes RS-3, or RS-4 for some
  abstract transition identity enabled at `a` (FR-120's successor relation
  gives it a successor), with `Taken` listing what it matched; otherwise it
  SHALL fail `NoAbstractMatch{position}`.
- An undecided contract conjunction of either model SHALL return
  `Undetermined(UndecidedSuccessor)`.

### Rules, with hidden fields (AX-2)

- The rules SHALL be applied to each member of `pre.abstract_states`:
  - after a `stutter` step the post set SHALL be the same set, provided the
    visible part is unchanged, and otherwise the step SHALL fail
    `StutterChanged{position}`;
  - after an abstract-operation or `any` step the post set SHALL be every
    RS-4 or RS-5 abstract successor of a member whose visible fields equal
    the post mapped state's, sorted by state key;
  - a step whose post set is empty SHALL fail `NoAbstractMatch{position}`.
- Each abstract successor computed SHALL count toward FR-101's
  `max_transitions` of the run that called `check_step`.

### Purity

- `check_initial` and `check_step` SHALL be functions of their inputs.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-141-AC-1 | Over ADR-020 §8's `CasRefinesCounter`: `check_initial` on `(0, 0, f, 0, f)` passes; `beginA` from `(0, 0, f, 0, f)` passes with `Taken::Stutter`; `commitA` from `(0, 0, t, 0, f)` to `(1, 0, f, 0, f)` passes with `Taken::Abstract([inc(c)])`. Over the lost-update model, `commitB` from `(1, 0, f, 0, t)` to `(1, 0, f, 0, f)` fails `AbstractStepRejected{transition: inc(c), cause: Postcondition}`. | Test (TC-546) |
| FR-141-AC-2 | The concrete initial state with `value` 1 fails `InitialNotAbstract{initial: 0}`. With `commitA` mapped `-> stutter`, `commitA` from `(0, 0, t, 0, f)` fails `StutterChanged`. With both commit rows `-> any`, the lost-update `commitB` step of AC-1 passes with `Taken::Stutter`, and `commitB` from `(2, 1, f, 0, t)` to `(1, 1, f, 0, f)` fails `NoAbstractMatch`. Over the lost-update model with the §8 rows, `commitA` from `(2, 1, t, 0, f)` to `(2, 1, f, 0, f)` fails `AbstractStepRejected{transition: inc(c), cause: Precondition{clause: CanInc}}`. | Test (TC-546) |
| FR-141-AC-3 | Over `RingIsQueue`, `take` from `head = 1`, `size = 2`, `s0.value = 0`, `s1.value = 1` returning 1 passes RS-4 as `deq(r)` with the result bound; the broken `take` returning 0 from the same state fails `AbstractStepRejected{transition: deq(r), cause: Postcondition}`. `put -> enq(self.ring, _)` with `put(1)` from the empty queue passes with `Taken::Abstract([enq(r, 1)])` only. | Test (TC-546) |
| FR-141-AC-4 | Fixture `Coin`: abstract `Spec::Coin` with `tossed: Boolean`, `side: Int[0, 1]`, `shown: Boolean`, `face: Int[0, 1]`, operation `toss()` (precondition `not self.tossed`, frame `modifies [tossed, side]`, postcondition `self.tossed`) and `show()` (precondition `self.tossed and not self.shown`, frame `modifies [shown, face]`, postcondition `self.shown and self.face = self.side`), initial all false and 0; concrete `Impl::Coin` with `tossed`, `shown`, `face`, operations `toss()` (precondition `not self.tossed`, frame `modifies [tossed]`, postcondition `self.tossed`) and `reveal()` (precondition `self.tossed and not self.shown`, frame `modifies [shown, face]`, postcondition `self.shown`); rows `tossed`, `shown` and `face` mapped by name, `side` hidden, `toss -> Spec::Coin::toss(self)`, `reveal -> Spec::Coin::show(self)`. After concrete `toss` the candidate set holds two states, `side` 0 and `side` 1; after `reveal` to `face = 1` it holds the one with `side` 1. With the row `side = self.face` added, `reveal` to `face = 1` fails `AbstractStepRejected{transition: show(c), cause: Frame{…}}`. | Test (TC-546) |
| FR-141-AC-5 | Over FR-136-AC-4's protocol subject with branch `A`'s attempt node mapped `-> any` and `incA` mapped `-> Spec::Counter::inc(self)`, a step of that attempt node that leaves `value` unchanged passes RS-5 with `Taken::Stutter`, so the node row was selected; the `fork` step, mapped `-> stutter`, passes RS-3. | Test (TC-546) |

## Dependencies

- ADR-020 §2 RS-1 to RS-6, §4 AX-2 and AX-4, §6 RC-1; ADR-018 SM-1 and
  SM-4; ADR-027 PR-1.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the
  expansion calls), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (terminal stutter), [FR-136](FR-136-check-a-refinement-s-step-rows.md),
  [FR-138](FR-138-check-and-compute-history-fields.md),
  [FR-140](FR-140-evaluate-the-refinement-mapping-at-a-state.md).

## References

- The QSpec half (step rules RS-2 to RS-8): Linear STD-133.
