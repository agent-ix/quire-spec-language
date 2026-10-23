---
id: TC-1187
title: "A hybrid declaration checks to a classified model and its claim routes to a hybrid provider"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: verifies
---
# TC-1187: A hybrid declaration checks to a classified model and its claim routes to a hybrid provider

## Description

Verify S2's hybrid form, S3's checks, refusals and classification, the
checked hybrid claim, S4's emitted hybrid nodes, the I2 reader's read-back
and refusals, and the provider request served in process and by a process
provider.

Scope: FR-447-AC-1 to FR-447-AC-5, FR-447-AC-13 to FR-447-AC-15 and FR-447-AC-17.

## Test Procedure

Fixtures: the `Braking` declaration and the `SlowEnough` claim of FR-447's
use case; `MovingFair`, defined as `temporal MovingFair using p over (h:
Braking) clock "model-time" on origin { fair Moving; always [0, 10] true }`;
an in-process fixture hybrid provider; a process fixture hybrid provider
advertising the same kind and requirement, which reads the model and claims
from the package bytes it receives.

1. Build and check `Braking`; read its `HybridForm` and `CheckedHybridModel`.
2. Check each mutation FR-447-AC-2 lists, one at a time.
3. Check `SlowEnough`, the nested, `until`, interval-free and `[0, *]`
   bodies FR-447-AC-3 lists, and each claim mutation it lists.
4. Write the request for `SlowEnough` and negotiate it with the in-process
   fixture registered, then with only the process fixture registered, then
   with no provider; compare the checked package identities of `Braking` and
   of `Braking` with `Moving`'s `v' = -a` replaced by `v' = -(2 * a)`.
5. Run S4 over the package and read the emitted bytes with QSpec's v2
   reader; check that `MovingFair`'s fairness member references the
   `[Braking]` model node and names `Moving`; run S4 over the
   `v' = -(2 * a)` package and compare `package_id`s.
6. Read the emitted package and QSpec's `positive-all-families.json` with
   QSL's I2 reader; feed it each mutation FR-447-AC-13 lists, with
   `dependencies`, keys and `package_id` rebuilt after each edit.
7. Emit the claim body `always [0, 10] true`, and read `SlowEnough`'s
   emitted law.
8. Check and emit `SlowEnough` under a `timed/v1` profile alias and read the
   bytes with QSpec's v2 reader; check `always [0, *] holds(h.v < 30)` under
   `timed/v1`, and `always holds(h.v < 30)` under `timed/v1` and under the
   event-position profile.

9. Write the request for `SlowEnough` and for a temporal clause over a model
   without hybrid dynamics in one unit, and negotiate both with EN-6 and an
   in-process fixture hybrid provider (`dynamics` `["hybrid"]`) registered.

Tag the tests `#[trace("TC-1187", "FR-447-AC-n")]`.

## Expected Results

- Step 1: the form, variables, classes, rates, initial mode and reset that
  FR-447-AC-1 states.
- Step 2: each refusal, cause and span that FR-447-AC-2 states.
- Step 3: the stated formula with horizon 10; horizons 15, 4, `None` and
  `None`; each claim mutation refused as stated.
- Step 4: one `temporal-satisfaction` item, extent `bounded`, carrying the
  hybrid-dynamics requirement, whose `claim` names `SlowEnough`'s
  `temporal_clause` node, routed to the in-process fixture, which reads
  `Braking` from the checked package; with only the process fixture, routed
  to it, which receives the v2 bytes and reads `Braking` and `SlowEnough`
  from their QSpec FR-481 nodes; `unsupported`,
  `unsupported-requested-capability` with none; different checked package
  identities.
- Step 5: the package is emitted and admitted, holding FR-447-AC-5's
  `hybrid_model`, `hybrid_mode`, `hybrid_transition` and `temporal_clause`
  nodes; `MovingFair`'s fairness node names `Moving` by referencing the
  `[Braking]` `hybrid_model` node; the two `package_id`s differ.
- Step 6: the read-back `CheckedHybridModel` and `CheckedHybridClaim`
  (horizon 10) equal the checked ones; `positive-all-families.json` is
  admitted; each mutation refused with the code and locus FR-447-AC-13 states.
- Step 7: a `quire.op.temporal.true` formula node under the `always` node
  and no `holds` node over a Boolean literal; the law `p` selects.
- Step 8: the interval `[0, 10]` with closed ends and horizon 10, emitted in
    the timed form with the `timed/v1` law and admitted; `always [0, *]`
  under `timed/v1` admitted with `upper: None`, emitted with `upper` `null`;
  `always holds(h.v < 30)` under `timed/v1` admitted with horizon `None`, and under the event-position profile refused
  `unsupported_construct`/`expression-form` at the operator's span, naming
  the profile, as FR-447-AC-15 states.
- Step 9: `SlowEnough`'s item carries `"requirements": ["hybrid-dynamics"]`
  and routes to the fixture alone; the other item carries no `requirements`
  and routes to EN-6 alone; neither is `ambiguous-backend`; the request
  identities differ as FR-447-AC-17 states.
