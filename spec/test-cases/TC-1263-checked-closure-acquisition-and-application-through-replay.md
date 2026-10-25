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
2. Call `lookup_callable` with the unit's `package_id` and the own callable, dependency callable, enum, parameter and missing node ids; repeat the imported callable lookup with its dependency's `package_id`, then register and apply that returned key in the acquired closure. Try an unknown owner and a minted key from a package outside this closure. Compare the returned keys and signatures with the checked graphs under a QSL-internal test.
3. Compile an external consumer crate importing only `qsl_replay`, `quire_eval`, `quire_exact` and `quire_semantic_value`; acquire, look up, bind a generated body, begin, step and run. Reject forbidden imports and facade access to the adapter/checked package in compile-fail fixtures.
4. Exercise FR-485-AC-4's admission order, failed and successful Begin, sealing, two later sessions, and a dropped runnable session. Record factory invocations and meter charges.
5. Apply `f(3)` without and with a generated body; have the generated body request another admitted callable. Compare the interpreted run with QSL's current result, location, loss and spend. Repeat at `work_units` one below and exactly equal to measured spend, and with wrong arity and wrong type. Cancel the meter's installed handle with `Requested` before `begin_call` in one run; in another, begin successfully with an uncancelled handle, cancel it before the session's first `function.call` charge, then step.
6. Attempt acquisition with malformed source, a same-owner library, a short S1 limit and an S1 limit above the reader limit. Begin `h(p: Population<T>[3]): Boolean` with `Value::Population` carrying a `PopulationId` not recorded in `ApplicationEnvironment` constructed from `ObjectClosure`. Observe Begin before any body read, both before and after FR-405's evaluator switch.
7. Inspect production source and run FR-060 T12-A on the consumer checkout for API and ownership boundaries.

## Expected Results

1. Package bytes, identity and context equal; one spine compile; the retained closure remains valid.
2. Own and imported callables resolve to their minted keys/signatures when the correct owner id is supplied; the imported id with the unit owner refuses `Foreign` naming its actual owner; missing owner/id and wrong-kind lookups refuse as specified. The imported key registers and applies in the same closure; the outside key returns `CallableNotAdmitted`.
3. Allowed imports compile and apply; forbidden paths fail to compile or fail T12-A.
4. Registration and sealing follow the leaf's exact order; one binding remains effective; neither registration nor refused Begin invokes a factory or makes an application charge.
5. Results and charges match the shared leaf path. Cancellation before Begin returns `CallFailure::Cancelled(Requested)`, no session or charge; cancellation after successful Begin returns the located incomplete evaluation at the denied `function.call`, with `tripped() = Requested` and no admitted charge. Denied work retains the preceding charge prefix; no continuation runs after terminal or drop.
6. Acquisition refusals equal `compile_admitted` in class and detail. The missing `PopulationId` returns `CallFailure::Input(InputRefusal::WrongValueKind)` for parameter `0` at Begin, with no session, charge or body read, on both designated evaluators; no admitted empty population is fabricated.
7. The facade uses the existing std reference adapter before FR-405's switch and one designated evaluator afterward, with one compile path, no dual dispatch, compatibility layer, copied leaf or QSL moved-type re-export.
