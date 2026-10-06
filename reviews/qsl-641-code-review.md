---
id: SR-1342
title: "Code review of quire-spec-language PR #644: value-parity and operator-level scalar-parity replay (QSL-641)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9922c999ae052ae227e16902ba62d8da05c6ae96; PR #644 diff against origin/main: qsl-replay/src/execute.rs, qsl-replay/src/execute/operator_parity.rs, qsl-replay/src/execute/value_parity.rs, qsl-replay/src/execute/tests.rs, qsl-replay/src/scalar.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/result.rs, qsl-replay/src/lib.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-098
    type: reviews
---
# Code review of quire-spec-language PR #644

## Summary

Ticket: QSL-641. Reviewed head 9922c999. The Rust lane (rust-review) is
folded into this file. No build was run. Per the leader, the focused
qsl-replay tests and clippy are green, and the full `make ci` runs in the
merge batch.

The contract is the QSL-641 description plus its comments in time order:
20:06 (qsl_replay owns both arms), 20:07 (the ScalarAgrees cause), 20:12
(operator-arm inputs and comparison) and 20:55 (R6 and R7).

What conforms:

- The comparison rules (`scalar.rs:557-663`). `Completed(v)` maps to
  `Value(v)`. The exact result runs with `Some(result_range)`, so a value
  out of range becomes `OutOfRange` and cannot equal `Value`. `Refused`
  maps to `OutOfRange`, and `Undefined` to `Undefined`. Causes, charges and
  counters are not compared. `Incomplete` and `ExecutionFault` return
  `GeneratedFault` (Failed) before operand admission or evaluation.
  Nothing settles `Refuted`.
- R6. `replay_operator_parity` reads `wire.obligation_identity` before
  decoding, so the report carries it on every outcome, including a request
  that fails to decode. No `QSLObligationIdentity` type exists in QSL or in
  quire-contract-codegen. `ObligationIdentity` (O-09) is an opaque 32-byte
  wrapper that QSL never hashes or checks, so reusing it fits R6's intent:
  carry the obligation through unchanged. The spec-side wording point is
  SR-1344 FND-004.
- R7. The arm recompiles the proving context by FR-098's rules
  (`PackageIdMismatch`). It then resolves the node in the recompiled graph
  and checks that the node's preimage operation identity is the operator's
  catalog identity. A node or operator mismatch is
  `ScalarIdentity` → `Code::StaleDependency`, displayed
  `stale_dependency/revision-mismatch`, and settles
  `Inconclusive(ReplayRefused(stale_dependency))`. A package mismatch shows
  `stale_dependency/content-mismatch`, which is the existing replay identity
  check. The spec misstates this (SR-1344 FND-002). The arm never goes
  through `select`, so `NotAPredicate` is unreachable from it.
- Function arm. It reuses `recompile`, `select` (with the new `Claim`
  switch) and `arguments`, so S6a admission, unknown, duplicate and unbound
  parameters, and `NotAValueFunction` all follow FR-098. Values are compared
  with O-13 `same_element`.
- The `ScalarOutcome`/`NativeOutcome` split is justified. `NativeOutcome`
  is the 20:12 input shape: integer-only, with two fault variants that never
  reach the comparison. `ScalarOutcome` is the comparison projection that
  both arms and `ScalarAgreement` share. If they were one type, the function
  arm and the envelope cause would gain `Incomplete`/`ExecutionFault`
  variants that mean nothing there. The naming overlap is FND-003.
- The test oracles are independent: literal expected values (inc(3)=4,
  inv(2)=1/2, 40/8=5, 100+100 out of [0,150], divide by 0 undefined), not
  values re-derived through the code under test.

## Verdict

Approve with findings. The comparison, settlement, R6 and the R7 node and
operator checks conform. FND-001 (medium) should be fixed in this PR,
together with SR-1344 FND-001: integer div/rem/mod claims are accepted by
the type but always refuse with a misleading stale-revision cause. FND-002
and FND-003 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ScalarOperation::Divide`, `Remainder` and `Modulo` cannot pass the R7 operator check through `replay_operator_parity`. QSL lowers no `quire.op.integer.div`/`rem`/`mod` node: `lowering.rs:2867-2884` lowers `/` to `quire.op.rational.div`, and `qsl-package/src/emit/tests/golden.rs:327-333` lists integer div/rem as NOT_LOWERED. So every such claim refuses `ScalarIdentity::Operator`, which displays `stale_dependency/revision-mismatch`. That misdiagnoses the claim: the package is not stale, QSL has no such node. CG will read the refusal as a revision skew. Fix: either refuse these operators with their own cause (unsupported operator, not a revision mismatch) before the node lookup, or remove the three variants until QSL lowers them, and say which in FR-357. | qsl-replay/src/scalar.rs:143-167, qsl-replay/src/execute/operator_parity.rs:97-105 |
| FND-002 | low | The operator arm never resolves or checks the request's `selected_function`. That meets R7 as written (package membership of the node and operator), but a request naming a function the package does not declare, or one whose body does not hold the node, still replays. FR-357's Inputs present the enclosing function as part of the proving context. Either resolve it (`UnknownFunction`) and check that the node is in its body, or say in FR-357 that it is carried and not checked. | qsl-replay/src/execute/operator_parity.rs:80-107 |
| FND-003 | low | "Refused" means two things across the arms' public types. `NativeOutcome::Refused` is an out-of-range result and maps to `ScalarOutcome::OutOfRange`. `ScalarOutcome::Refused` is a non-range refusal (an inexact decimal, say). A CG caller of `replay_value_parity` who passes `ScalarOutcome::Refused` for a native range refusal gets `Diverged` against QSL's `OutOfRange`. Rename one of the two, for example `ScalarOutcome::Refused` to `OtherRefusal`, or document the mapping on both. | qsl-replay/src/scalar.rs:39-45, qsl-replay/src/scalar.rs:244-245 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `in_body` resolves each digest it finds in a preimage by scanning every node of the graph and comparing `candidate.to_string()` against it, so the body walk costs body nodes × references × package nodes, with a string allocation per comparison. The result is correct and packages are bounded, so this is a cost, not a wrong result. Build one digest-to-key map per call, or resolve through a keyed lookup, before the walk. | qsl-replay/src/execute/operator_parity.rs:137-169 |

## Dispositions

Round 1, reviewed 5398844084ce266a5010f76b21c6ecdf8015b9b0 (squashed on 4403f2f0e, the same base as 9922c999, so compared by tree diff 9922c999..53988440).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
| FND-002 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
| FND-003 | fixed | 5398844084ce266a5010f76b21c6ecdf8015b9b0 |
