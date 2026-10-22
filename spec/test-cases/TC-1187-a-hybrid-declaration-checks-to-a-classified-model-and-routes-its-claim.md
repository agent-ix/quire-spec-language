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
checked hybrid claim, the provider request and S4's emission refusal.

Scope: FR-447-AC-1 to FR-447-AC-5.

## Test Procedure

Fixtures: the `Braking` declaration and the `SlowEnough` claim of FR-447's
use case; an in-process fixture hybrid provider; a process fixture provider
advertising the same kind.

1. Build and check `Braking`; read its `HybridForm` and `CheckedHybridModel`.
2. Check each mutation FR-447-AC-2 lists, one at a time.
3. Check `SlowEnough`, the same claim with `always h.v < 30`, and each claim
   mutation FR-447-AC-3 lists.
4. Write the request for `SlowEnough` and negotiate it with the in-process
   fixture registered, then with no provider; compare the package identities
   of `Braking` and of `Braking` with `Moving`'s `v' = -a` replaced by
   `v' = -2 * a`.
5. Run S4 over the package; route the item to the process fixture provider.

Tag the tests `#[trace("TC-1187", "FR-447-AC-n")]`.

## Expected Results

- Step 1: the form, variables, classes, rates, initial mode and reset that
  FR-447-AC-1 states.
- Step 2: each refusal, cause and span that FR-447-AC-2 states.
- Step 3: horizon 10 and the stated atom; horizon `None`; each claim
  mutation refused as stated.
- Step 4: one `temporal-satisfaction` item, extent `bounded`, carrying the
  hybrid-dynamics requirement, routed to the in-process fixture, which reads
  `Braking` from the checked package; `unsupported`,
  `unsupported-requested-capability` with none; different package
  identities.
- Step 5: `unsupported_construct`/`not-yet-implemented` naming `Braking` and
  `hybrid-model`; the process provider's item `unsupported` with a
  diagnostic.
