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

QSL's layer-6 crate `qsl-replay` SHALL hold one function that decides
each concrete step of a refinement, `check_step`, and one that decides each
concrete initial state, `check_initial` (ADR-020 RS-1 to RS-6, AX-2). The
explicit-state product (FR-142, FR-143), replay (FR-145) and the simulation
certificate checker (FR-149) call these and
no other step rule, as every temporal engine answers to the trace evaluator
(ADR-018 SM-1). The initial-state and step rules they decide are QSpec
FR-377's, and the hidden-field candidate set is QSpec FR-378's (ADR-020
QS-2); this requirement states how QSL decides them and what it reports.

## Use case

A concrete step is mapped to an abstract increment. The step check computes
the abstract states before and after, asks the abstract model whether that
increment takes the one to the other, and, when it does not, says which
clause rejected it. The same answer comes from the model checker and from
replay.

## Inputs

```rust
pub enum SideState { Model(ModelState), Protocol(ProtocolState) }  // ADR-027 PS-8

pub enum AbstractSystem<'a> {
    Model(&'a ModelSystem<'a>),        // FR-120
    Protocol(&'a ProtocolSystem<'a>),  // ADR-027 TS-1, for FR-148
}

pub struct RefinementPosition {
    pub concrete: SideState,          // a model state, or a protocol state over a protocol subject
    pub history: HistoryValues,       // FR-138
    pub abstract_states: Vec<SideState>, // one state, or the AX-2 set, sorted by state key
}

pub fn check_initial(
    refinement: &CheckedRefinement,
    abstract_system: AbstractSystem<'_>,
    concrete_initial: &SideState,
) -> StepVerdict;

pub fn check_step(
    refinement: &CheckedRefinement,
    abstract_system: AbstractSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep,              // transition identity, post-state, result
) -> StepVerdict;
```

## Outputs

```rust
pub enum StepVerdict {
    Passes { post: RefinementPosition, taken: Taken },
    Fails(RefinementFailure),
    Undefined(UndefinedEvaluation),   // ADR-020 RE-5: a mapping row, argument or history update
    Undetermined(InconclusiveCause),  // MappingUndetermined, UndecidedSuccessor
}

pub enum Taken { Stutter, Abstract(Vec<ModelTransition>) }  // abstract identities matched
```

`ModelTransition` and `ProtocolTransition` are `quire-semantic-value` types.

`RefinementFailure` is ADR-020 RC-1's: `InitialNotAbstract{initial}`,
`StutterChanged{position}`, `AbstractStepRejected{position, transition,
cause}` with cause `Precondition{clause}`, `Frame{code}` or
`Postcondition`, `NoAbstractMatch{position}`, the liveness kinds of
FR-143, and `Undefined{position, row}`, QSpec FR-379's seventh kind, which
the engine writes from a `StepVerdict::Undefined` (FR-142).

## Behavior

### Initial states (RS-2)

QSpec FR-377's initial-state rule, with QSpec FR-378's initial candidate
set when the abstract side has hidden fields, is the rule `check_initial`
decides.

- `check_initial` SHALL compute `h0` by FR-138's `initial_history` and
  `map(c0, h0)` by FR-140.
- It SHALL take the abstract initial states from the abstract subject's
  own FR-106 snapshots, compare them by state key, and return `Passes`
  with `abstract_states` the one matching state, or with hidden fields the
  candidate set in state-key order.
- When the rule rejects the state it SHALL return
  `Fails(InitialNotAbstract{initial})`.

### Selecting the row

- `check_step` SHALL select the step's row (FR-136): over a model subject,
  its operation's row; over a protocol subject, the row of its step class,
  with an attempt node's row taking precedence over its operation's row
  and a compensation template's row over its operation's row (ADR-027
  PR-1). The concrete terminal stutter step (ADR-018 SM-4, FR-125) SHALL be
  decided as a `stutter` row (RS-6).
- It SHALL compute the post history by FR-138's `step_history` and the post
  mapped state by FR-140. A `MappingFailure::Undefined` from either SHALL
  return `Undefined` with `where` naming the step's post-state position,
  the row and the object (ADR-020 RE-5). A `MappingFailure::Undetermined`
  from either SHALL return `Undetermined(MappingUndetermined)`.
- `check_initial` SHALL return `Undefined` with position 0 when FR-140
  returns `MappingFailure::Undefined` at an initial state.
- An argument expression of the row that evaluates `Undefined` at the
  pre-state SHALL return `Undefined`; one that is refused or incomplete
  SHALL return `Undetermined(MappingUndetermined)`.

### Deciding the step rules

QSpec FR-377's step rules (`stutter`, abstract operation and `any`; ADR-020
RS-3 to RS-5) and, with hidden fields, QSpec FR-378's candidate-set rule
(ADR-020 AX-2) are the rules `check_step` decides. QSL's obligations are
how it evaluates them and what it reports:

- **Abstract-operation evaluation.** `check_step` SHALL evaluate the row's
  receiver and arguments at the pre-state, with the concrete arguments and
  history, enumerating each `_` argument's parameter domain in FR-120's
  canonical order. For each abstract transition identity it SHALL make the
  calls `ModelSystem` makes when it expands the pre mapped state `a`,
  applied to the one candidate post-state `a'`: the effective
  precondition at `a`, `StateModel::check_frame` over `(a, a')`, and the
  effective postcondition over `(a, a', delta)`, binding the abstract
  result to the concrete step's result when the row has `binds_result`.
- **`any` enabling.** An abstract transition identity SHALL count as
  enabled at `a` when FR-120's successor relation gives it a successor.
- **Passing.** A passing step SHALL return `Passes` with `Taken::Stutter`
  when the `stutter` rule admitted it, and otherwise `Taken::Abstract`
  listing every accepted abstract transition identity in canonical order.
- **Failure kinds.** A rejected step SHALL return:
  - `StutterChanged{position}` for a `stutter` row;
  - `AbstractStepRejected` for an abstract-operation row, naming the first
    identity in canonical order and the first check that rejected it:
    `Precondition{clause}`, `Frame{code}` or `Postcondition`;
  - `NoAbstractMatch{position}` for an `any` row, and for any row whose
    post candidate set is empty.
- **Candidate sets.** With hidden fields, `check_step` SHALL apply the
  rules to each member of `pre.abstract_states` and return the post set
  sorted by state key. Each abstract successor computed SHALL count toward
  FR-101's `max_transitions` of the run that called `check_step`.
- An undecided contract conjunction of either model SHALL return
  `Undetermined(UndecidedSuccessor)`.

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
| FR-141-AC-6 | Over `RegisterHistory` with FR-138's `inv` update, `check_step` on `write(r, 0)` from the initial state returns `Undefined` with `where` position 1, the `inv` update and object `r`, cause `division-by-zero`; with the `writes` update of FR-138-AC-4 instead, the second `write(r, 0)` returns `Undetermined(MappingUndetermined)`. | Test (TC-554) |
| FR-141-AC-7 | Over `RingIsQueue` with the row `put -> enq(self.ring, 2 / self.size)`, `check_step` on `put(1)` from the empty queue returns `Undefined` naming the step-row argument, cause `division-by-zero`, with `RefinementFailure::Undefined{position: 1, row: put}` written by the engine; with the original row and a per-evaluation meter budget of zero for the argument `self.ring`, it returns `Undetermined(MappingUndetermined)`. | Test (TC-555) |

## Dependencies

- ADR-020 §2 RS-1 to RS-6, §4 AX-2 and AX-4, §6 RC-1; ADR-018 SM-1 and
  SM-4; ADR-027 PR-1.
- [FR-120](FR-120-simulate-a-checked-package-s-state-family.md) (the
  expansion calls), [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (terminal stutter), [FR-136](FR-136-check-a-refinement-s-step-rows.md),
  [FR-138](FR-138-check-and-compute-history-fields.md),
  [FR-140](FR-140-evaluate-the-refinement-mapping-at-a-state.md).

## References

- QSpec FR-377 (initial-state and step rules) and QSpec FR-378 (the
  candidate set); Linear STD-133.
