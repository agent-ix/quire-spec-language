---
id: TC-1045
title: "A snapshot carries the state-machine context an invocation's invariant, precondition and postcondition are evaluated against"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-106
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-107
    type: verifies
---
# TC-1045: A snapshot carries the state-machine context an invocation's invariant, precondition and postcondition are evaluated against

## Description

Verify FR-106's `contexts` member of a `pre` or `post` snapshot, the `Context`
selection of a state machine's invariant, the admission checks 6.10 to 6.12, 9
and 11.6 for a context, and that admission is deterministic over reordered
documents.

Scope: FR-106-AC-41, FR-106-AC-42, FR-106-AC-43, FR-106-AC-44,
FR-106-AC-45, FR-106-AC-46 and FR-106-AC-47. TC-1040 steps 6 to 8 use the
documents of steps 1 and 2.

## Preconditions

The `Orders` fixture of TC-1040 (state machine `OrderLifecycle` with the
`Boolean` context fields `placed` and `cancelled`; the record value type
`Flags`; the operation `place(flags: Flags)`, no result, frame modifies
`placed` and `cancelled`; the clauses `Consistent`, `PlaceRequires` and
`FlagsPlaced`), with these declarations added: state machine
`ExpressLifecycle` specializing `OrderLifecycle` with the field `retries:
Int[0, 5]`; state machine `Billing` with the field `paid: Boolean`; state
machine `Owned` with the field `owner: Reference<Pump>`; the object type
`Pump` in the complete-able population `Q`; the process `Fulfilment`. The
invocation `i1` of `place` has `self` `null` and `flags` `{placed: true}`.
Snapshots hold `"populations": []` unless a step says otherwise. Tag the tests
`#[trace("TC-1045", "FR-106-AC-n")]`.

## Test Procedure

1. Documents `P1` (`pre`, context `{placed: false, cancelled: false}`), `Q1`
   (`post`, context `{placed: true, cancelled: false}`) and `Q2` (`post`,
   context `{placed: true, cancelled: true}`), each naming `i1`'s identity.
   Admit `Consistent` by `Context` side `pre` over `P1`, side `post` over
   `Q1`, and side `post` over `Q2`. Rewrite each document with `fields`
   members, `contexts` entries and whitespace reordered and admit again.
2. Side `pre` over `P1` with: `Q1` absent from the provision; `Q1` bytes not
   JSON; `i1`'s `parameters` ill-formed; `i1` without `result`, `created` and
   `deleted`. Side `post` over `Q1` with `Q1` absent from the provision.
3. Remove `contexts` from `P1`; then from `Q1` only; then give `P1` only an
   entry for `Billing`. Admit the invariant side `pre`, side `post` (over
   `Q1`), the precondition `PlaceRequires` by `Invocation` and by `PreCall`
   (snapshot `P1`, no `self`, `flags` `{placed: true}`), and the postcondition
   `FlagsPlaced`. Then give `P1` two entries for `OrderLifecycle`; then an
   entry for `OrderLifecycle` and one for `ExpressLifecycle` (with `retries`
   0); then `OrderLifecycle` and `Billing` (`paid` false).
4. Defect cases over `P1`, side `pre`, one defect each: `contexts` on a
   snapshot that says `current`, selected by `Current` for an object
   invariant; `contexts` an object; an entry without `machine`; without
   `fields`; with an extra member `extra`; `machine` `7`; `fields` `[]`;
   `machine` `ix://test/orders/Pump`, `ix://test/orders/Fulfilment`,
   `ix://test/orders/Missing`; `fields` without `cancelled`; `fields` with
   `state` `{"integer": "1"}` beside both declared fields; with `transitions`
   `{"sequence": []}`; `placed` `{"integer": "1"}`. Then a snapshot with an
   object defect (`Q` object of an unlisted type) and a context defect, and a
   snapshot with defects in two entries.
5. `ExpressLifecycle` with the post context `{placed: true, cancelled: false,
   retries: -1}` selected side `post` for a clause on `ExpressLifecycle`,
   under `PostStateRange::Witness` and under `Refuse`; the same `-1` in the
   `pre` context under `Witness`.
6. Frame: Step 1's `Q2` against `place` modifying `placed` and `cancelled`;
   against `place` modifying only `placed`, for `FlagsPlaced` and for the
   invariant side `post`; the invariant side `pre` under the narrower frame.
   `Q1` rewritten to equal `P1` in another member order. An invocation with
   `self` `{population: Q, key: p1}` for `place`, and one with `self` `null`
   for an object operation. `Consistent` selected by `Current`, by `PreCall`
   and by `Invocation`; an object invariant selected by `Context`; `PlaceRequires`
   by `PreCall` naming `self` `p1`.
7. A snapshot with population `Q` and the machine `Owned`, context `{owner:
   reference Q/p9}`: `Q` complete without `p9`; `Q` `"complete": false`;
   `Q` listing `p1` with `owner` naming `p1`; `Q` incomplete with a context of
   `OrderLifecycle` only. `observation.values` set to the exact count of
   the `P1` document, and one less. A replay environment document (FR-098) that
   holds `contexts`.

## Expected Results

- Step 1: side `pre` over `P1` admits one observation: the pre context, `i1`'s
  four identity labels, no self object, no parameters, no result, and
  `Consistent` is `Completed(true)`. Side `post` over `Q1` admits the post
  context and gives `Completed(true)`; over `Q2` it admits and gives
  `Completed(false)`, a logical violation and no refusal. The reordered
  documents give the same typed observation, the same digest and the same
  results.
- Step 2: side `pre` gives `Completed(true)` for every variant with no
  refusal; side `post` with `Q1` absent returns `Incomplete` with
  `unavailable_observation`/`missing-required-artifact`.
- Step 3: with no context in `P1` or only `Billing`, every selection refuses
  `invalid_model_binding`/`wrong-context` naming `OrderLifecycle` and `P1`
  (the invariant side `post` over a `Q1` with the context also reports `P1`
  first); with no context in `Q1` only, the invariant side `post` and
  `FlagsPlaced` refuse `wrong-context` naming `Q1`, while side `pre` and
  `PlaceRequires` admit. The two `OrderLifecycle` entries refuse
  `invalid_runtime_input`/`duplicate-member` at entry 1; the
  `OrderLifecycle` and `ExpressLifecycle` entries refuse `duplicate-member` at
  entry 1; the `OrderLifecycle` and `Billing` entries admit and the `Billing`
  context is ignored. No case returns `Incomplete` or makes a default context.
- Step 4: in order, `invalid_runtime_input`/`unknown-member` naming `contexts`;
  `wrong-value-kind`; `missing-member` naming `machine`; `missing-member`
  naming `fields`; `unknown-member` naming `extra`; `wrong-value-kind` naming
  `machine`; `wrong-value-kind` naming `fields`; `wrong-role-mapping` for each
  of the three machine identities; `missing-member` naming `cancelled`;
  `unknown-member` naming `state`; `unknown-member` naming `transitions`;
  `wrong-value-kind` at the path `OrderLifecycle`, `placed`. The object defect
  is reported before the context defect, and the first entry's defect before
  the second's.
- Step 5: under `Witness` the post context admits and the post observation
  reports one `OutOfRange` naming `ExpressLifecycle`, `retries`, the range
  `[0, 5]` and the observed `-1`; under `Refuse`, and for the `pre` context
  under `Witness`, `invalid_runtime_input`/`invalid-value` at
  `ExpressLifecycle`, `retries`.
- Step 6: `Q2` against the wide frame admits. Against the narrow frame
  `FlagsPlaced` and the invariant side `post` refuse `frame_violation`/
  `unauthorized-change` naming `OrderLifecycle` and `cancelled`; the invariant
  side `pre` admits. The rewritten `Q1` is no change. `self` an object
  reference for `place` and `self` `null` for an object operation refuse
  `invalid_runtime_input`/`wrong-value-kind`. The three wrongly selected
  invariants, the object invariant by `Context` and `PlaceRequires` by
  `PreCall` naming `self` refuse `wrong_snapshot`/`wrong-observation`.
- Step 7: `Q` complete without `p9` refuses `dangling_reference`/
  `absent-target-in-complete-population` naming `Owned`, `owner`, `p9` and
  `Q`; `Q` incomplete returns `Incomplete` with `incomplete_population`/
  `incomplete-scope` naming `Q`; `owner` naming the listed `p1` admits; the
  snapshot with only an `OrderLifecycle` context admits. The document at the
  exact bound admits and one less returns `stage_limit_exceeded`/
  `node-count-exceeded` naming `observation.values`. The environment document
  refuses `invalid_runtime_input`/`unknown-member` naming `contexts`.
