---
id: TC-906
title: "Composite and leaf-family arguments replay, and refuse by kind, by domain and at the request's limits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: verifies
---
# TC-906: Composite and leaf-family arguments replay, and refuse by kind, by domain and at the request's limits

## Description

Verify that `qsl_replay::replay` converts a composite `WitnessValue` to a
kernel value of the parameter's declared type, admits it through S6a, and
replays a nested-record counterexample from both arms. Each kind, identity
and domain defect refuses `WrongValueKind` before the call, and the
`value_occurrences` and `work_units` limits stop the replay before the call, settling `Incomplete` by counter name.

Scope: FR-098-AC-8, FR-098-AC-9, FR-098-AC-10, FR-098-AC-12,
FR-277-AC-4, FR-277-AC-5.

## Test Procedure

The unit declares `record Inner { a: Int[0, 9]; b?: Boolean; }`,
`union Shape { Circle(Int[0, 9]), Empty }`,
`record Outer { i: Inner; o: Option<Int[0, 9]>; s: Sequence<Int[0, 9]>[0, 3]; u: Shape; }`
and `p(x: Outer): Boolean { x.i.a < 5 }`. The counterexample `C` is
`Outer { i: Inner { a: 7, b: true }, o: 4, s: [1, 2], u: Circle(3) }`.
Requests are built from the unit's spine compile, as in TC-444.

1. Replay `C` as an `Input` assignment, then as a `Witness` entry in
   FR-070's witness value text.
2. Replay `C` with one defect at a time: a sequence given for `o`; `i`'s
   `name` set to `Outer`'s node id; `i` missing `a`; `i` with an undeclared
   field `z`; `a` absent; `u` with member `Square`; `Circle` with no
   component; `a = 12`; `s` with four elements. Replay `g(v)`, whose `v` is
   a `Set` of a `Decimal` type with scales 0 to 2, with the set `1.0`,
   `1.00`.
3. Replay `C` with the accounting limit `value_occurrences` one below its
   occurrence count, then raised to fit. Replay it with `work_units` one
   below its node count, then raised to fit.

4. A second unit declares `enum Color { Red, Green }`, a unit `m`, an object
   type `T`, and one predicate per leaf family: `c(v: Color)`,
   `t(v: Text[0, 8])`, `r(v: Rational)`, `d(v)` over a `Decimal` type with scales 0 to 2,
   `f(v: Float64)`, `q(v)` over a quantity in `m` and `same(v: Reference<T>)`, which
   compares `v` with itself. Each body is false at its counterexample.
   Replay each from a `Witness` entry in FR-070's witness value text. Then
   replay each with one defect: `v` naming another enum's declaration; a
   nine-scalar text; a decimal with scale 3; a `float32` for `f`; a quantity
   in another unit; a reference whose `object_type` is another type's.
   Finally, replay `deref(v: Reference<T>)`, which reads a field of `v`.

5. Replay the unchanged full TC-830 step 4 Tree through both Input and
   Witness public replay arms, using the TC-831 recursive sum and FR-357's
   existing value-parity entry for a non-Boolean selection. First require
   default success and direct/replay equality of evaluation charge traces.
   Independently count semantic occurrences, converted nodes C and the
   approved conversion helper sequence H. Repeat occurrence bounds 29,998
   and 29,999, accounting work bounds C-1 and C, conversion helper bounds
   H-1 and H, and admission helper A-1/A controls from TC-830. Isolate each
   bound with all other pre-call bounds permitting progress. A C-bound
   success means conversion succeeds; require enough evaluation work for
   a completed replay. Then hold all pre-call bounds sufficient and run
   the TC-831 evaluation 39,996/39,997 controls. The research prediction
   129,994 is neither H nor a runtime result.
6. Bind multiple composite arguments in declared parameter order. Count
   C and occurrences independently per argument, but A/H cumulatively
   across the whole operation and all delegated helpers. Choose a helper
   bound that permits the first argument and denies an actual event of
   the next; repeat at the exact total. Verify that no membership/helper
   total seeds C and that delegation spends each actual event once in its
   owner budget.
7. At preflight, descriptor inspection, type resolution, materialization
   and delegated membership boundaries, cancel the original handle before
   work and during later arguments/helpers, with Requested and Deadline.
   Exercise zero bounds, checked counter overflow, wrong kind/domain and
   measured storage/capacity failure before mutation. Compare the complete
   typed payload with the independent next-event sequence. Include a
   logical cache-hit lookup and an admitted nested value to detect a free
   lookup or unauthorized descendant revalidation.
8. For numeric membership, enum/registry/ancestor lookup and type resolution,
   bind each logical helper invocation to FR-277's independent input-size
   contract. Inspect the actual published size fields/effective bounds and
   source-backed termination/work proof. A replay byte ceiling alone must
   not qualify direct kernel inputs. Test exact/one-less supported input
   bounds separately from A/H; cancellation before entry and on helper
   return must preserve the original cause. Do not claim an internal
   per-limb/comparison callback that the helper does not expose. A missing
   size/termination proof is an explicit shared source gate, not a pass.
9. Include decimal membership with an actual shifted comparison. Separately
   assess the helper's temporary storage, fallible reservation and original
   Cancel boundary for decimal-digit formatting/shifted multiplication.
   Input numeric-size bounds and integer/rational comparison-cost candidates
   must not substitute for those guarantees. If the current bounded-helper
   capability is unsupported, retain the explicit native helper/capability
   cause and phase before invocation. If the cause's native carrier/catalog
   projection is missing, report that mapping unavailable, never a guessed
   kernel cause or qualified default success.
   Retain the peer's tiny falsifier: decimal domain [0,100], value 1@0 and
   scale 2. Tiny numerical inputs do not remove the helper's temporary
   allocation/cancellation qualification dependency.

Tag the tests `#[trace("TC-906", "FR-098-AC-8")]`,
`#[trace("TC-906", "FR-098-AC-9")]` and
`#[trace("TC-906", "FR-098-AC-10")]`. Steps 5-7 additionally bind
`FR-098-AC-12`, `FR-277-AC-4` and `FR-277-AC-5` by their criterion IDs.

## Expected Results

- Step 1: each replay evaluates `x.i.a` to `7` and settles as FR-098-AC-2
  states: `reproduced-without-witness` for `Input` and
  `reproduced-with-evaluated-witness` for `Witness`.
- Step 2: each refuses `WrongValueKind` (`invalid_runtime_input`) naming
  position 0, before the call, with no charge.
- Step 3: each lowered limit makes no call and settles `inconclusive` with
  cause `NoValue`, carrying the outcome `Incomplete` that names
  `value_occurrences` or `work_units`, its configured value and the count
  reached. Each raised limit replays as in step 1, with the same charges as
  a replay of `C` under unlimited accounting.
- Step 4: each leaf-family predicate settles `reproduced-with-evaluated-witness`.
  Each defect refuses `WrongValueKind` naming position 0, before the call.
  `deref` settles `inconclusive` with cause `NoValue`, since a replayed
  function has no object environment.
- Step 5: the unchanged full Tree completes by default and at the exact
  independent bounds. Each pre-call denial settles incomplete/
  inconclusive/NoValue with zero evaluation consumption and the correct
  QSL phase limit record, never a fabricated kernel FunctionCall. C remains
  per-argument converted nodes. Evaluation controls reach the evaluator
  and have TC-831's actual charge sequence and denial. Wire assertions
  require the reviewed shared phase projection; an unavailable projection
  is reported unavailable, never treated as a passing serialization.
- Step 6: successful helper spend is cumulative; semantic counters retain
  their per-argument rule. No argument/helper reset, double debit or partial
  success. Exact helper ceiling permits progress when other bounds fit.
- Step 7: original cancellation cause with no partial replay output;
  invalid input retains WrongValueKind and its existing refusal boundary.
  Limit/storage/capacity remain distinct and zero-evaluation outcomes;
  denied work remains unspent. Wrong next-event metadata, cancellation as a
  limit, storage as a limit, fresh Cancel, fake FunctionCall or cache bypass
  fails the control.
- Step 8: finite invocation count plus independently bounded helper inputs
  establishes bounded work only when the owner's source proof is available.
  No unbounded numeric leaf, type-link cycle, registry key or member scan
  may hide behind a logical event. If the proof fails, record the exact
  owning IR helper-contract gap and preserve QSL-503's blocking edge; the
  draft and its review remain a proposal. No copied helper or fake outer
  work/cancellation claim is accepted.
- Step 9: no blanket admission safety or decimal temporary-storage/Cancel
  claim follows from numeric bounds. Unsupported bounded capability remains
  unsupported, distinct from a measured storage failure, capacity failure,
  limit, actual caller cancellation or invalid value. No default bypass,
  partial admitted value or evaluation consumption is accepted. The IR
  source gap remains routed while this draft's review proceeds.

## Status

Passed locally, `qsl-replay/src/execute/tests/composite.rs`, except the union cases
(pending QSL-503) and a quantity-typed parameter in source (pending STD-113).
This historical status does not cover QSL-681 steps 5-9: those controls are
proposed and unexecuted, pending shared event, union and wire alignment.
