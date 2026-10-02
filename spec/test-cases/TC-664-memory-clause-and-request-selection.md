---
id: TC-664
title: "The memory clause checks and the request selects the resolved model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-219
    type: verifies
---
# TC-664: The memory clause checks and the request selects the resolved model

## Description

Verify the `memory` clause's checks, request selection of an outermost `parallel`'s model, its refusals, and the resolved models in the obligation identity.

Scope: FR-219-AC-1 to FR-219-AC-5.

## Test Procedure

Use ADR-025 §10's `SB` protocol.

1. Check `SB` with no clause and with `memory tso`; check the nested,
   conflicting-default, `join any` and `outstanding` variants.
2. Resolve requests with no `memory` member, with a `tso` entry, and with
   entries naming the nested `parallel` and an attempt.
3. Check `SB` under the three resolved models and compare obligation
   identities, including source `tso` against a `tso` selection.
4. Select `tso` for a `parallel` with no clause that joins `any`.
5. Write the `SB` request with no `memory` member and with an explicit
   `sc` selection; compare the written effective models and identities;
   compare the checked-package identities of source `tso` and source `sc`
   and their `tso` requests' identities.

Tag the tests `#[trace("TC-664", "FR-219-AC-n")]`.

## Expected Results

- Step 1: `Sc` and `Tso`; the nested variant checks; the
  `invalid_model_binding`/`conflicting-binding` and
  `unsupported_construct`/`declaration-form` refusals of FR-219-AC-1.
- Step 2: `sc`, then `tso` for `SB::Both` and its nested `parallel`; two
  `invalid_runtime_input`/`invalid-value` refusals naming the entries.
- Step 3: three different identities with `proved`, `refuted`, `refuted`;
  equal identities for source and selected `tso`.
- Step 4: V-8 `WeakAccessShape{JoinPolicy}` before exploring; `sc` without
  the selection.
- Step 5: both requests carry `SB::Both` at `sc` and one obligation
  identity; the two packages differ in identity while the two `tso`
  requests share one.
