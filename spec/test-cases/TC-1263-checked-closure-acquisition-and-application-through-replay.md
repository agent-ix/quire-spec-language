---
id: TC-1263
title: "Checked closure acquisition and application through replay"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: verifies
---
# TC-1263: Checked closure acquisition and application through replay

## Description

Verify FR-485-AC-1 through FR-485-AC-8 through a consumer that may enter QSL only through `qsl_replay`, comparing the facade with the existing compile and evaluation paths.

## Test Procedure

1. Compile a unit with an own function `f(x: Int[0, 9]): Integer` and an imported callable. Compare acquisition with `compile_admitted`, instrument the shared compile path, drop the source and public output values, then create a registry from the retained handle.
2. Look up the own callable, a dependency callable, an enum and parameter node, and a missing node by their wire ids. Compare the own key and signature with the checked graph under a QSL-internal test.
3. Compile an external consumer crate importing only `qsl_replay`, `quire_eval`, `quire_exact` and `quire_semantic_value`; acquire, look up, bind a generated body, begin, step and run. Reject forbidden imports and facade access to the adapter/checked package in compile-fail fixtures.
4. Exercise FR-485-AC-4's admission order, failed and successful Begin, sealing, two later sessions, and a dropped runnable session. Record factory invocations and meter charges.
5. Apply `f(3)` without and with a generated body; have the generated body request another admitted callable. Compare the interpreted run with QSL's current result, location, loss and spend. Repeat at `work_units` one below and exactly equal to measured spend, with wrong arity, wrong type and a cancelled meter handle.
6. Attempt acquisition with malformed source, a same-owner library, a short S1 limit and an S1 limit above the reader limit. Apply a function requiring an unrecorded population through an environment formed from `ObjectClosure`.
7. Inspect production source and run FR-060 T12-A on the consumer checkout for API and ownership boundaries.

## Expected Results

1. Package bytes, identity and context equal; one spine compile; the retained closure remains valid.
2. Own callable resolves to its minted key/signature; missing, foreign and wrong-kind lookups refuse as specified.
3. Allowed imports compile and apply; forbidden paths fail to compile or fail T12-A.
4. Registration and sealing follow the leaf's exact order; one binding remains effective; neither registration nor refused Begin invokes a factory or makes an application charge.
5. Results and charges match the shared leaf path; denied work and cancellation have the existing located incomplete outcome, and no continuation runs after terminal or drop.
6. Acquisition refusals equal `compile_admitted` in class and detail; missing model data is never treated as an admitted empty population.
7. The facade uses the existing std reference adapter before FR-405's switch and one designated evaluator afterward, with one compile path, no dual dispatch, compatibility layer, copied leaf or QSL moved-type re-export.
