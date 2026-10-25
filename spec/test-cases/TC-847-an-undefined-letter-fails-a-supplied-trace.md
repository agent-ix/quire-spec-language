---
id: TC-847
title: "An undefined letter fails a clause over a supplied trace or lasso, and replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-071
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-072
    type: verifies
---
# TC-847: An undefined letter fails a clause over a supplied trace or lasso, and replays

## Description

Verify that the temporal evaluator returns `Undefined` at the first position
whose letter is undefined, over a finite trace, a finite prefix and a lasso;
that it maps to violation with cause `UndefinedEvaluation`; that a bounded
`on origin` clause reads only its horizon; and that replay reproduces the
undefined value at its position.

Scope: FR-327-AC-5, FR-327-AC-7, FR-328-AC-5, FR-329-AC-6, FR-330-AC-5,
FR-330-AC-8, FR-331-AC-4, FR-331-AC-5, FR-331-AC-7, FR-331-AC-8, FR-071-AC-8,
FR-072-AC-6, FR-098-AC-28.

A window aggregate short of data is the separate `InsufficientData`
outcome, category `undefined`, which never hides a false activation, an
undefined atom or a refused condition (FR-327-AC-7, FR-330-AC-8).

## Test Procedure

Fixture: the `Counter` unit, the closed trace and the prefix with `c.value`
0, 1, 2, and the observed lasso with an empty prefix and loop 0, 1, 2.
Each clause's undefined letter is `3 mod (2 - c.value)` at `c.value` 2, the
Euclidean remainder by zero; the step 4 unit compiles from source with no
refusal.

1. Evaluate the two clauses of FR-327-AC-5 over the closed trace.
2. Evaluate FR-328-AC-5's clause over the prefix.
3. Evaluate FR-329-AC-6's clause over the lasso.
4. Run FR-330-AC-5's request from source.
5. Construct, encode and decode both FR-331-AC-4 requests, assert the decoded
   state-clause target and envelope node/occurrence identify the same clause,
   then replay the two undefined-evaluation packets through
   `qsl_replay::replay`; submit an altered clause-node packet and replay an
   observed-lasso packet for `ReachesFair` whose fairness set matches the
   clause under FR-331-AC-5. Submit a packet with `ReplaySource::Input` and
   one whose `WitnessEnvelope` position differs from its payload position.
   Replay FR-331-AC-1's false packet and its true-result control, and replay
   FR-328-AC-1's finite-prefix packet that evaluates `Pending`. Replay a
   formula packet over FR-327-AC-7's one-position trace that yields
   `InsufficientData`, then replay the false packet with the temporal Work
   budget set to zero so evaluation returns `Incomplete`. Submit an observed
   arm with a missing document digest, model payload, and a function target;
   for the undefined packet, submit same-position payloads with only the
   source locus and only the cause changed.
6. Evaluate each clause of FR-327-AC-7, in its order, over the closed trace, and the two
   clauses over the one-position trace with `c.value` 2.
7. Run FR-330-AC-8's request from source.

Tag the tests `#[trace("TC-847", "<AC id>")]`.

## Expected Results

- Step 1: `Undefined{where: 2, cause: division-by-zero}`, category
  violation; `Completed(true)`.
- Step 2: `Undefined{where: 2, cause: division-by-zero}`.
- Step 3: `Undefined{where: 2, cause: division-by-zero}`.
- Step 4: `violation`, exit 10, position 2, cause `UndefinedEvaluation`.
- Step 5: the packet whose undefined result agrees at position 2, source
  locus and cause settles `reproduced-with-evaluated-witness` and carries
  decoded position 2 and the typed undefined value; the packet with position
  1 retains that packet position separately from the evaluator's undefined
  value at position 2 and settles `inconclusive`, `ReplayParity`. The altered
  clause-node packet refuses `stale_dependency`/`content-mismatch` naming
  `clause_node` before admission. `ReachesFair` refuses
  with `ReplayRefusal::MissingFairnessPremise`, naming `fair weak whole inc`,
  catalog code `unsupported_projection`/`missing-fairness-premise`, O-16
  unsupported, before formula evaluation and with no result. The
  `ReplaySource::Input` packet refuses `invalid_runtime_input`/`invalid-value`
  at `/replay/source`; the packet/envelope position mismatch refuses
  `invalid_runtime_input`/`invalid-value` at
  `/replay/observed_counterexample/trace_position`; both refuse before
  document admission. The missing-document packet refuses
  `missing_import`/`missing-selection` naming its digest record; the model
  payload refuses `invalid_runtime_input`/`invalid-value` at
  `/replay/observed_counterexample/trace`; the function target refuses
  `invalid_runtime_input`/`unknown-member` at
  `/replay/observed_counterexample`; each before document admission.
  The false packet returns the typed `Completed(false)` result with O-16
  category `violation` and reproduces at its position; the true packet
  retains `Completed(true)` with O-16 category `success` and settles
  `inconclusive`, `ReplayParity`; the finite-prefix packet retains `Pending`
  and settles `inconclusive`, `ReplayParity`. The one-position trace retains
  the typed `InsufficientData{where: 0, atom, count: 1, min_count: 2}` value
  and settles `inconclusive`, `ReplayParity`; zero temporal Work retains the
  typed `Incomplete` charge point and settles `inconclusive`, `ReplayParity`.
  The same-position `Undefined` values with a different locus or a different
  cause each settle `inconclusive`, `ReplayParity`, retaining that actual
  typed value. O-16 categories are `violation` for false and undefined,
  `success` for true, `inconclusive` for pending, `undefined` for insufficient data,
  `incomplete` for exhausted Work and `unsupported` for missing fairness.
- Step 6: `InsufficientData{where: 2, count: 0, min_count: 1}`, category
  `undefined`; `Completed(false)` at 1; `Completed(false)` at 2;
  `UndefinedEvaluation{where: 1, cause: division-by-zero}`;
  `UndefinedEvaluation` at 2; `InsufficientData{where: 0, count: 1,
  min_count: 2}`; `InsufficientData` at 0 for the `on origin` clause;
  `InsufficientData{where: 0, count: 1, min_count: 2}`, not `Completed(false)`,
  for the clause whose false `holds` atom shares position 0 with the short `B`; over
  the one-position trace, `UndefinedEvaluation` at 0 for both orders.
- Step 7: stage `evaluate`, `undefined`, exit 10, position 2, `count` 0,
  `min_count` 1, no `truth`, no counterexample.
