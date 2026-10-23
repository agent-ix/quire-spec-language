---
id: TC-1191
title: "Begin cancellation through contract admission and through the pre-admitted entries"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-401
    type: verifies
---
# TC-1191: Begin cancellation through contract admission and through the pre-admitted entries

## Description

Verify where a cancelled meter handle is observed. `quire-eval`'s Begin with
contract admission polls the meter's installed handle during argument
admission and ends with the pre-charge `CallFailure::Cancelled`. `qsl-eval`'s
`call`, `evaluate` and `call_supplied` admit under a run of their own or of
the caller and begin pre-admitted, so a cancelled meter handle is first
observed at the first meter charge. Scope: FR-262-AC-12, FR-401-AC-9 and
FR-401-AC-10.

## Test Procedure

1. Through `quire-eval`'s Begin with contract admission, begin the admitted
   `g(x: Int[0, 9], y: Int[0, 9]): Boolean` with `3` and `4`, and a checked
   expression with those two parameter values: once with the meter's
   installed handle cancelled with `Requested` before Begin, once with the
   meter's handle tripping with `Deadline` at the supplied-value run's first
   poll point for the second argument, once with no handle cancelled, and
   once with a meter that has no installed handle.
2. Over the TC-918 `Compare` package, call `call` with a meter whose
   installed handle is cancelled with `Requested`, then with `Deadline`.
   Over the FR-090 population query of `model_reference_queries.rs`, call
   `call` and `evaluate` the same way, and again with a handle that trips at
   its n-th poll for each n of the suite's expected prefix.
3. Over the `Compare` package, call `call_supplied` with a supplied-value run
   over an uncancelled handle and a meter whose installed handle is a
   different handle, cancelled with `Requested`, then with `Deadline`.

## Expected Results

1. The two cancelled runs return `CallFailure::Cancelled` with the
   handle's cause, no session, zero consumed counters and no
   `function.call` charge. The other two runs return a session.
2. `call` over `Compare` returns `Ok` with `Outcome::Incomplete` at
   `ChargePoint::FunctionCall`. The population query's `call` and
   `evaluate` return `Ok` with `Outcome::Incomplete`. No charge is
   admitted, `tripped()` equals the cause, and no run returns
   `CallFailure::Cancelled`. A handle that trips at its n-th poll stops at
   the n-th prefix charge, with the earlier prefix admitted.
3. `call_supplied` returns `Ok` with `Outcome::Incomplete` at
   `ChargePoint::FunctionCall`; no charge is admitted, the meter's
   `tripped()` equals the cause, the run's handle is not tripped, and the
   result is never `CallFailure::Cancelled`.
