---
id: TC-631
title: "EN-5 builds the DTMC or MDP product with monitors, accumulators and intermediate states"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: verifies
---
# TC-631: EN-5 builds the DTMC or MDP product with monitors, accumulators and intermediate states

## Description

Verify the product for §15.1, §15.2 and §15.4, intermediate states for residual nondeterminism, `NotMarkov`, and determinism.

Scope: FR-196-AC-1 to FR-196-AC-4 and FR-196-AC-6 to FR-196-AC-9.
Canonical-key procedures below are prospective, not execution evidence.

## Test Procedure

Fixtures: ADR-024 §7.1 and §7.2 models and claims; `Link` with `Deliver`; the two `Health` variants of FR-187-AC-3.

1. Build §15.1's product and check layering and probability sums.
2. Build `P95`'s product at `5 ms` and read the accumulator values and an edge probability.
3. Build `Deliver` over every scheduler and under `Even`.
4. Build the `Health` variants over every scheduler and under a workload; build one request twice.
5. For every FR-196 worked vector, derive its actual model/claim/node/unit
   context and compare exact JCS product-key octets and component bodies.
   Hold the model key fixed and vary monitor horizon/phase, activation,
   amount, each exact weighted sum and pending identity independently.
6. Accumulate steps/reward to threshold equality and two values above it;
   end an active measure without B at its horizon; compare unactivated
   zero, active zero and completed zero. For weighted sums (1,2) and
   (2,4), apply one true contribution of weight one.
7. Create a real SCH-2 residual choice with a drawn empty vector and with
   two random Boolean parameters; vary receiver/non-random arguments and
   reverse the random vector. Compare typed signed zeros/NaN payloads,
   option absence and reference universes in the actual draw domain.
   Verify no monitor/accumulator advance occurs at the intermediate.
8. Supply each FR-196 admission negative; compare Reach/ExpectedReward/
   LongRunFraction actual None with a fabricated zero accumulator, and
   compare digital timed states differing only in one clock/deadline.
   Derive Retx clock identities/caps/scale from the checked declarations;
   compare FR-196's complete digital arrays after delay, loss/reset and
   same-time steps. Test success exactly at D, success after D, until
   prefix failure, terminal idle closure and no admissible same-time
   success. Retain a real residual race winning delay in pending and
   verify one delay/discrete advance at resolution. Refuse every timed
   shape/clock/deadline negative; stop at the actual identity-byte/product
   budget and inspect the outcome.

Tag the tests `#[trace("TC-631", "FR-196-AC-n")]`.

## Expected Results

- Step 1: at most 1,001 undecided states plus two decided states; sums 1; every path decided within 1,001 positions.
- Step 2: values `0`, `1 ms` to `4 ms` and saturated `6 ms`; `343/5000`.
- Step 3: an MDP with two actions per live state; a DTMC with FR-187-AC-2's probabilities.
- Step 4: intermediate states the monitor does not read; `NotMarkov` under a workload and for the empty variant; equal builds.
- Step 5: exact full canonical product keys; every specified distinction
  survives values/ranking/policy/component-bias/rejection round-trip,
  while graph materialization order changes no bytes.
- Step 6: equality remains exact, both finite overflow values normalize
  to one tag, censoring is distinct, and weighted pairs remain distinct
  with subsequent exact terminal ratios 2/3 and 3/5.
- Step 7: actual pending empty vector differs from None; every varied
  identity/value/parameter position changes its key; the intermediate
  retains the source monitor/accumulator and resolution advances once.
- Step 8: malformed/context-inapplicable forms refuse; exact-only forms
  retain real None and timed model keys retain clocks. A resource stop
  gives no truncated/default key or fabricated pending tuple. Clock
  identity/order/scale/caps and optional deadline match the exact digital
  arrays. Delays advance elapsed time without adding predicate positions;
  discrete resets preserve deadline time, success at D accepts and success
  after D rejects. Pending race delay applies exactly once.
