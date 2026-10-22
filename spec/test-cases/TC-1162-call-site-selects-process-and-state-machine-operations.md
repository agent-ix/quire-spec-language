---
id: TC-1162
title: "call_site selects process and state-machine operations"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: verifies
---
# TC-1162: call_site selects process and state-machine operations

## Description

Verify that `call_site`'s operation selection `M::T::op` resolves an owner
`T` that is a process or a state machine, returns its `frame_operation`,
anchor, frame and clauses, and refuses an owner other than an object type, a process or a state machine and an ambiguous operation name with QSpec FR-323's codes.

Scope: FR-121-AC-25, FR-121-AC-26, FR-121-AC-27.

## Test Procedure

1. Over FR-104-AC-10's package (process `Flow`, operation `go`) and its
   unit with `P`, `Q` and `I`, call `call_site` with `Config::Flow::go` and
   with `Config::Flow::stop`.
2. Over TC-1045's package (TC-1040's state machine `OrderLifecycle` with
   operation `place`, and `ExpressLifecycle` specializing it) and TC-1040's
   unit, call `call_site` with `M::OrderLifecycle::place` and with
   `M::ExpressLifecycle::place`; repeat over a unit with no clause or
   attempt naming `place`.
3. Select an operation of a record value type, an event type, an enum, a
   persistence interface and a population of the package; over a package where `C` specializes `A`
   and `B`, each declaring `touch` and each named by a clause of the unit,
   select `M::C::touch`. Map each refusal through
   `TerminalValue::from_call_site_refusal`.

Tag the tests `#[trace("TC-1162", "FR-121-AC-n")]`.

## Expected Results

- Step 1: `frame_operation` `{Config::Flow, go}`, anchor context the `Flow`
  `model`/`process` node, the frame's `generated` occurrence, `clauses`
  `P` then `Q`; `stop` refuses `UnknownOperation`, `missing_declaration`.
- Step 2: `frame_operation` `{M::OrderLifecycle, place}`, anchor context
  the `OrderLifecycle` `model`/`state_machine` node, the operation's `frame`
  node and its `generated` occurrence, `clauses` exactly `PlaceRequires`
  then `FlagsPlaced`; `M::ExpressLifecycle::place` gives the same
  `frame_operation`, anchor, frame and occurrence; the second unit refuses
  `UnknownOperation`.
- Step 3: `NotAnOperationOwner` (code `invalid_model_binding`, message
  leading with `invalid_model_binding/malformed-declaration`) for the five
  owners; `AmbiguousOperation` (code `ambiguous_declaration`, message
  leading with `ambiguous_declaration/ambiguous-name`) for `M::C::touch`,
  not `UnknownOperation`; each pairs
  the `OperationName` with the `package_id` and maps to `Declined`,
  `InvalidInput`, with its code.
