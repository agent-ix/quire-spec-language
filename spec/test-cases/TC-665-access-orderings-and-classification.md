---
id: TC-665
title: "S3 checks orderings and fences and classifies every access"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: verifies
---
# TC-665: S3 checks orderings and fences and classifies every access

## Description

Verify access classification, admitted orderings with their refusal, the access-shape rule at S3 and in the pre-check, and that orderings change nothing under `sc`.

Scope: FR-220-AC-1 to FR-220-AC-5.

## Test Procedure

1. Check `SB` and read each attempt's class; check the non-atomic and
   `fetch_add` variants.
2. Check each ill-ordered access and fence of FR-220-AC-2, and fences with
   each admitted ordering.
3. Check the two-field attempt under `memory tso` and under `sc`, then
   select `tso` from the request.
4. Check `SB` under `sc` with all `relaxed` and with all `seq_cst`.
5. Run the pre-check on AC-3's two-field attempt and on FR-219-AC-4's `any`-join `parallel`, each with a request selecting `tso`.

Tag the tests `#[trace("TC-665", "FR-220-AC-n")]`.

## Expected Results

- Step 1: the classes of FR-220-AC-1.
- Step 2: `ill_typed`/`operator-ineligible` naming each; the four fences check.
- Step 3: `unsupported_construct`/`declaration-form` naming both locations; checks under
  `sc`; V-8 `WeakAccessShape{MultiLocation}` under the selection.
- Step 4: 16 states and `proved` both times.
- Step 5: V-8 `WeakAccessShape` with `MultiLocation{locations}` naming both locations; V-8 `WeakAccessShape` with `JoinPolicy`; each serialized as QSpec FR-439 writes it.
