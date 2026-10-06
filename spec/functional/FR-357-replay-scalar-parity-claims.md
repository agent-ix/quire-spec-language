---
id: FR-357
title: "Replay scalar-parity claims: an operator at recorded operands, a non-Boolean function at its parameter bindings"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-010
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-069
    type: traces_to
---
# FR-357: Replay scalar-parity claims: an operator at recorded operands, a non-Boolean function at its parameter bindings

## Description

QSL SHALL replay a scalar-parity claim through two entries of the layer-6
replay facade, `qsl_replay::replay_operator_parity` and
`qsl_replay::replay_value_parity`. A scalar claim says the generated code
agrees with QSL's exact semantics: its observed outcome equals the exact
one. It is a claim about the lowering, never a property of a spec. A
counterexample is the pair of the exact evaluation's inputs and the generated
outcome. Neither entry settles `Refuted`: that stays with a `Boolean`
predicate that is false at its bindings (FR-098).

## Inputs

- Operator level: the original proving context as an FR-071 replay request
  (the call site's `package_id`, package reference, dependencies and byte
  provision; the enclosing function, which may be non-`Boolean`, as its
  `selected_function`; the opaque `ObligationIdentity` the caller minted from
  the owning obligation's preimage as its `obligation_identity`, which QSL
  never hashes or checks; its `source` is not read); the
  scalar node the catalog operator is tied to; the
  operator (`add`, `subtract`, `multiply` or `negate`) with its ordered
  decoded `i64` operands, each with its
  own range (a literal's range is the singleton `(value, value)`); the proving
  package's result range; `ScalarLimits`; the native outcome of the same
  proved generated artifact (`Completed(value)`, `RefusedOutOfRange`, `Undefined`,
  `Incomplete` or `ExecutionFault`); and the canonical generated-content
  identity of that observation, carried into the claim identity and never
  recomputed. Integer division, remainder and modulo claims are not
  accepted: QSL lowers no integer `div`, `rem` or `mod` application node today
  (its `/` lowers to `quire.op.rational.div`, and `quire.op.integer.div` and
  `quire.op.integer.rem` are among the identities no FR-093 row lowers, see
  `NOT_LOWERED` in the emit golden tests and FR-091-OQ-5), so there is no
  node to check such a claim against. An operator claim for them is added
  when QSL lowers integer division.
- Function level: an FR-071 replay request selecting a function whose
  declared result is not `Boolean`, whose `source` binds that function's
  declared parameters and nothing else, and the generated outcome (a value,
  an out-of-range outcome, `Undefined` or `OtherRefusal`, a refusal other
  than for range).

## Outputs

- Operator level: an `OperatorParityReport` holding the request's obligation
  identity and an `OperatorParityResult`. Function level: a
  `ValueParityResult`, or a `ReplayRefusal` for a failure FR-098 refuses.
  Each result has a `terminal_value` for the proof envelope (FR-069).

## Behavior

- The operator entry SHALL recompile the context's source by FR-098's
  rules and require the recompiled `package_id` to equal the request's; a
  mismatch refuses `PackageIdMismatch` (`stale_dependency`/`content-mismatch`).
  It SHALL resolve `selected_function` in the recompiled package (a name the
  package does not declare refuses `UnknownFunction`,
  `missing_declaration`/`missing-name`), and SHALL require the claim's scalar
  node to be a node of the package, to lie in that function's body, and to be
  an application of the claim's operator; each mismatch refuses
  `ReplayRefusal::ScalarIdentity` (`stale_dependency`/`revision-mismatch`)
  with its own cause. Each refusal settles
  `Inconclusive(ReplayRefused(code))`. The entry SHALL NOT replay the
  enclosing function as a predicate or function obligation, and
  `NotAPredicate` is not reachable from it.
- The operator entry SHALL carry the request's obligation identity unchanged
  to every settlement: the report holds it on every outcome, and a
  `ScalarAgrees` claim identity holds it.
- The operator entry SHALL admit each operand against its own range before
  any evaluation, and SHALL refuse an operand outside it with
  `invalid_runtime_input`, which settles
  `Inconclusive(ReplayRefused(invalid_runtime_input))`.
- The operator entry SHALL run the quire-exact operator on the operands under
  the claim's limits, with the result range as its result bound, and compare
  on the proof projection only: `Completed(v)` agrees iff the exact result is
  in range and equals `v`; `RefusedOutOfRange` agrees iff the exact result is outside
  the result range; `Undefined` agrees iff the exact operator is undefined
  there. It SHALL NOT compare refusal causes or charge totals.
- A native `Incomplete` or `ExecutionFault` SHALL settle `Failed` as a
  generated-artifact fault, without running the comparison.
- The function entry SHALL recompile the package and admit the bindings
  against the function's declared signature by FR-098's rules (S6a, an input
  outside the signature is refused), evaluate `f(b)` with the exact evaluator
  and compare its outcome with the generated one. A binding naming a node that
  is no parameter, a parameter bound twice and a parameter left unbound each
  refuse; no operand drawn per operation, literal or intermediate subterm, is a
  function-level witness.
- The function entry SHALL refuse a function declared `Boolean` with
  `ReplayRefusal::NotAValueFunction`, before any call.
- A divergence (the exact outcome differs from the generated one) is a
  lowering defect attributed to the generator and settles `Failed`. An
  agreement means the counterexample does not reproduce and settles
  `Inconclusive` with the typed cause `ScalarAgrees`, which holds the claim
  identity (operator or function, its operands or bindings, and for an
  operator claim its node and generated-content identity), the agreed outcome
  and no predicate verdict. The encoded-size bound of the proof envelope
  (FR-069-AC-4) counts it.
- An exact evaluation that reaches a limit before an outcome settles
  `Incomplete`, never a verdict.
- FR-098's predicate arm is unchanged: it refuses a function whose declared
  result is not `Boolean` as `NotAPredicate`, and only when predicate replay
  is asked for.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-357-AC-1 | Function level: a generated outcome that differs from QSL's exact `f(b)`, a different value or a generated out-of-range outcome where QSL has a value, settles `Diverged` with both outcomes and the evaluation's charges, for an integer function and a `Rational`-valued one. | Test (TC-904) |
| FR-357-AC-2 | Function level: a generated outcome equal to QSL's, a value, settles `Agrees`, which is `Inconclusive(ScalarAgrees)`, for an integer function and a `Rational`-valued one. | Test (TC-904) |
| FR-357-AC-3 | Function level: bindings that fail S6a admission (a value outside a parameter's declared domain, or a Boolean for an integer parameter) settle `RefusedInput`, which is `Inconclusive(ReplayRefused)` with the admission code, and no outcome is compared, for an integer function and a `Rational`-valued one. | Test (TC-904) |
| FR-357-AC-4 | Function level: only the declared parameters are bound: a binding naming a non-parameter node refuses `UnknownParameter`, and a parameter left unbound refuses `UnboundParameter`. | Test (TC-904) |
| FR-357-AC-5 | Function level: a function declared `Boolean` refuses `NotAValueFunction` before any call. | Test (TC-904) |
| FR-357-AC-6 | Predicate replay of a non-`Boolean` function still refuses `NotAPredicate`, and a `Boolean` predicate replays as before. | Test (TC-904) |
| FR-357-AC-7 | Operator level, through `replay_operator_parity`, for add, subtract, multiply and negate: `Completed(v)` and `RefusedOutOfRange` each agree exactly when the exact outcome matches on the proof projection and diverge otherwise, and a native `Undefined` diverges from these operators, none of which is undefined. | Test (TC-904) |
| FR-357-AC-8 | Operator level: a native `Incomplete` or `ExecutionFault` settles `Failed` as a generated-artifact fault without a comparison, even under starved limits. | Test (TC-904) |
| FR-357-AC-9 | Operator level: an operand outside its own range settles `Inconclusive(ReplayRefused(invalid_runtime_input))` with nothing evaluated; a literal's range is the singleton `(value, value)`. | Test (TC-904) |
| FR-357-AC-10 | Function and operator level: an agreement settles `Inconclusive(ScalarAgrees)` naming the claim and the outcome, a divergence settles `Failed`, an exact evaluation that reaches its limits (function arm and operator arm each) settles `Incomplete`, and no scalar result settles `Refuted`; a `ScalarAgrees` cause is counted by the envelope's encoded-size bound and round-trips through the envelope. | Test (TC-904, TC-177, TC-178, TC-179) |
| FR-357-AC-11 | Operator level: the call site of a non-`Boolean` function recompiles, its scalar node is found in the selected function's body as an application of the operator, and an agreement, a divergence and a refused operand each settle with the request's obligation identity carried unchanged. A stale `package_id` refuses `PackageIdMismatch` (`content-mismatch`); a node the package does not hold, a node outside the selected function's body and an operator the node is not an application of each refuse `ScalarIdentity` (`revision-mismatch`) with their own cause; an undeclared enclosing function refuses `UnknownFunction` (`missing-name`). Each settles `Inconclusive(ReplayRefused(code))` with the obligation identity carried. | Test (TC-904) |

## Dependencies

- [FR-098](FR-098-execute-a-replay-request.md): the request, recompile,
  selection and admission the function entry reuses.
- [FR-069](FR-069-implement-typed-proof-result-envelope.md): the terminal
  values and inconclusive causes.

## Status

Implemented. TC-904 passes locally.
