---
id: TC-904
title: "Scalar-parity claims replay to diverged, agrees, refused and faulted outcomes, and never to Refuted"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: verifies
---
# TC-904: Scalar-parity claims replay to diverged, agrees, refused and faulted outcomes, and never to Refuted

## Description

Verify `qsl_replay::replay_operator_parity` and
`qsl_replay::replay_value_parity` settle each outcome of FR-357, and that the
predicate arm of `replay` still refuses a non-`Boolean` function.

Scope: FR-357-AC-1 to FR-357-AC-11.

## Test Procedure

Function level, over `inc(x: Int[0, 9]): Integer { x + 1 }`,
`inv(x: Int[1, 9]): Rational[0, 1; 1, 9] { 1 / x }` and the
predicate `small`:

1. Replay `inc(3)` against generated 5, `inc(3)` against out-of-range and
   `inv(2)` against generated 1/3.
2. Replay `inc(3)` against 4 and `inv(2)` against 1/2.
3. Replay `inc` and `inv` with `x = 12` and with a Boolean binding.
4. Replay `inc` with `inv`'s parameter node, and with no binding.
5. Replay `small` as a value-parity claim; replay `inc` as a predicate.
6. Replay `inc(3)` with a zero work budget.

Operator level, through `replay_operator_parity` and the call site of each
operator's own function (`inc` `+`, `dec` `-`, `dbl` `*`, `neg` unary `-`),
over operands drawn from `[0, 100]` with result range `[-200, 200]`:

7. `Completed`, `RefusedOutOfRange` and `Undefined`, matching and not
   matching the exact outcome, for each operator.
8. Native `Incomplete` and `ExecutionFault`, with the exact limits starved;
   exact limits starved with a matching native value.
9. An operand of 101; a literal operand with its singleton range.
10. The terminal value of an agreement and of a divergence.
11. An `add` claim with a stale `package_id`, a node the package does not
    hold, a `mul` node under `inc`, a `mul` claim on the `+` node, and an
    undeclared enclosing function; each checked for the request's obligation
    identity.

Read an envelope carrying a `ScalarAgrees` cause, and one whose cause is over
the encoded-size bound.

## Expected Results

- Steps 1 to 3: `Diverged`, `Agrees`, `RefusedInput`.
- Steps 4 and 5: `UnknownParameter`, `UnboundParameter`,
  `NotAValueFunction`, `NotAPredicate`.
- Step 6: `Incomplete`, settling `Incomplete`.
- Step 7: agreement only on the proof-projection matches; the rest diverge;
  `Undefined` always diverges.
- Step 8: `GeneratedFault`, settling `Failed`; `Incomplete` for the starved
  exact evaluation.
- Step 9: `RefusedInput` for 101, an agreement for the literal at its own
  value and a refusal at any other.
- Step 10: `Inconclusive(ScalarAgrees)` and `Failed`; no result settles
  `Refuted`. The envelope round-trips the agreement and refuses the oversized
  one.
- Step 11: `PackageIdMismatch` (`content-mismatch`); `ScalarIdentity`
  (`revision-mismatch`) with cause `Node`, `Function` and `Operator`;
  `UnknownFunction` (`missing-name`); each carries the obligation identity.

## Status

Implemented. The tests are in `qsl-replay` (`execute/tests.rs`, `scalar.rs`,
`proof_result.rs`).
