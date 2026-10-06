---
id: SR-1353
title: "Code review of quire-spec-language PR #650: report claim identity and operator-parity obligation recompute (QSL-641)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4c2cf5dab9e9f1629d188b9f72001435b4a2d793; PR #650 diff against origin/main: qsl-replay/src/execute.rs, qsl-replay/src/execute/operator_obligation.rs, qsl-replay/src/execute/operator_parity.rs, qsl-replay/src/execute/value_parity.rs, qsl-replay/src/execute/tests.rs, qsl-replay/src/lib.rs, qsl-replay/src/outcome.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/scalar.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
---
# Code review of quire-spec-language PR #650

## Summary

Ticket: QSL-641 (follow-up, IR path). Reviewed head 4c2cf5da. The Rust lane
(rust-review) is folded into this file.

The PR implements four rulings: (1) both parity reports carry the full claim
identity on every outcome through `claim()`; (2) a consumer checks
`report.claim()` equals the claim it sent; (3) the observation digest is
carry-and-bind, and the driver authenticates it; (4) QSL recomputes the
operator-parity obligation identity from the ADR-013 O-09 preimage through
`quire-canonical` and refuses a mismatch.

Checked and correct:

- `replay_operator_parity` builds the `OperatorIdentity` before it decodes
  the request, so the decode-refusal path, every refusal from `settle` and
  every settled outcome carry the same identity. `replay_value_parity` builds
  `ValueIdentity` from the wire before it decodes, and now returns a
  `ValueParityReport` on every path, with FR-098 refusals moved into
  `ValueParityResult::Refused`.
- An agreement's `ScalarClaim` is a clone of the report's identity at both
  levels, so the two cannot drift apart.
- The canonical encoding matches ADR-013 O-09 as amended: member names
  `arguments`, `node`, `obligation_kind`, `occurrence_key`; each argument
  `{operand, position, range}`; `range` `{lower, upper}` as decimal strings;
  operand `{node_id, tag: "graph_child"}` or `{node_id, occurrence_key,
  position, tag: "inline_literal"}`; `occurrence_key` `{ordinal, role}`.
  `quire_canonical::Writer` sorts members by UTF-16 code unit, so the write
  order does not matter. Positions and the ordinal are JSON integers.
  `WireNodeId`'s `Display` is lowercase hex with no prefix. The preimage
  encodes through the one encoder (ADR-013 §2) and names no JSON crate.
- The recompute runs after the node, operator and body checks and refuses
  with the new `ScalarIdentityMismatch::Obligation { claimed, recomputed }`,
  which displays both digests and maps to `stale_dependency` /
  `revision-mismatch`.
- `ScalarClaim` now derives `PartialEq`. The hand-written `same_element`
  comparison is gone because the function claim compares the wire `source`,
  not decoded values.

Focused tests were not run by this review. While the review was running,
the coder was already editing the worktree for the FND-001 fix (an
uncommitted `tests.rs`) and building in its `target/`. A run there would
not have tested 4c2cf5da, so the queued run was cancelled. The findings
come from reading the code at 4c2cf5da.

The two limits the coder flagged are real, and they are the main finding
(FND-001). QSL resolves the claim's node in the recompiled package and
checks its operator and its enclosing function. It never checks the other
preimage members against that package. The occurrence key, the
`GraphChild` node ids, the `InlineLiteral` positions and literal values, and
the obligation kind all come from the caller. So "recompute" binds the
identity to caller input, not to the recompiled package. The test fixture
shows it: the claim for `inc`'s `x + 1` says both operands are
`GraphChild(0x11…)`, a node the package does not hold, and the right operand
is really the literal `1`. That claim settles `Agrees`. Everything except the
obligation kind can be derived from, or checked against, the recompiled
package: the application node's preimage lists its operands (child node ids
or inline literals with their values), and its FR-322 occurrence keys are
on the checked graph. The plan lead has since ruled that QSL checks them,
and the coder is adding the check.

## Verdict

Request changes. The claim-identity carriage (rulings 1 to 3) is correct
on both arms, and the canonical encoding matches O-09. But the recompute
(ruling 4) binds only to caller input until FND-001 lands. FND-002 leaves
the native outcome out of the operator identity, although ruling 1 asks for
the full identity. FND-003's tests would hide FND-001's fix turning every
fixture into a refusal. FND-004 is a size-bound formula that #645 (FR-358)
will break.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The recomputed obligation identity is bound only to caller input, never to the recompiled package. `settle` checks that the node exists, applies the operator and lies in the selected function's body, then hashes the caller's own `occurrence`, operand identities, positions and ranges. It never checks that the occurrence key is one of the node's FR-322 occurrences (with role `expression`), that a `GraphChild` node is the application's child at that position, or that an `InlineLiteral` position holds an inline literal whose value equals the singleton range. A caller can mint a digest over any of these and QSL accepts it. The fixture claims `inc`'s `x + 1` with two `GraphChild(0x11..)` operands and it settles `Agrees`. Fix (plan-lead ruling): read the application node's operands and occurrence keys from the recompiled package and check each preimage member against them. Refuse a mismatch `ScalarIdentity` (`stale_dependency`/`revision-mismatch`) with its own cause, before the digest compare. | qsl-replay/src/execute/operator_parity.rs:109-156, qsl-replay/src/execute/operator_obligation.rs:17-108, qsl-replay/src/execute/tests.rs:1862-1867 |
| FND-002 | medium | `OperatorIdentity` leaves out the native outcome. `into_parts` splits it off ("the digest binds it"), but QSL never authenticates the digest. Two operator claims that differ only in `generated` and share a digest give equal `claim()` values, so the FR-357 consumer check cannot tell them apart. Ruling 1 asks for the full claim identity. FR-357's consumer bullet says a report with another outcome is another claim. `ValueIdentity` does carry `generated`. Fix: add `generated: NativeOutcome` to `OperatorIdentity`, and add it to the AC-13 list of differing claims. | qsl-replay/src/scalar.rs:265-310, qsl-replay/src/execute/tests.rs:2322-2340 |
| FND-003 | medium | The operator AC-13 and AC-14 tests assert only `claim()` (or the digest) on each case. They never assert that the case reached its labelled outcome (agrees, diverged, generated fault, refused operand, incomplete). If every case refused, both tests would still pass. That will happen once FND-001 lands, because the fixtures' `GraphChild(0x11..)` operands do not match the package. The "on every outcome" coverage would then disappear silently. Fix: assert the `OperatorParityResult` variant per case, next to the `claim()` check. | qsl-replay/src/execute/tests.rs:2273-2340, qsl-replay/src/execute/tests.rs:2348-2374 |
| FND-004 | medium | `ValueIdentity::measured_bytes` counts an `Input` source as `assignments.len() * (32 + 8)`. That is a third copy of the formula in request.rs:519 and witness.rs:838, and it is right only while `WitnessValue` is a Boolean or an i64. PR #645 (FR-358) makes `WitnessValue` composite and replaces the request copy with `ReplaySource::measured_bytes`. This copy would then undercount a composite binding, so a `ScalarAgrees` cause could slip past the envelope's encoded-size bound (FR-357-AC-10). Fix: add one `ReplaySource::measured_bytes` in witness.rs, use it from all three sites, and let #645 extend it. | qsl-replay/src/scalar.rs:337-353 |
| FND-005 | low | `recompute` turns an encoder `Err` into the identity `[0; 32]`. A request whose `obligation_identity` is 32 zero bytes then matches. Today the encoder cannot fail on this preimage, but the sentinel is a value a caller can send. Fix: return `Result<ObligationIdentity, InternalFault>` and refuse as `ReplayRefusal::Fault`. | qsl-replay/src/execute/operator_obligation.rs:17-25 |
| FND-006 | low | FR-357 requires the consumer check `report.claim() == sent`, but the API gives a consumer no way to build the claim it sent. `OperatorClaim::into_parts` is `pub(crate)`, so CG has to build `OperatorIdentity` by hand from its claim and the request's obligation. `ValueIdentity` has to be rebuilt by hand from the wire, including the raw `(Option<String>, String)` package tuple. Fix: add public constructors (`OperatorClaim::identity(&self, ObligationIdentity)`, `ValueIdentity::sent(&ReplayRequestWire, &ScalarOutcome)`) and use them in the tests in place of the local `sent` helpers. | qsl-replay/src/scalar.rs:265-287, qsl-replay/src/execute/value_parity.rs:126-140 |

## Dispositions

Round 1, reviewed at 0679c3acb1a4315d8bf15c850a9d44fbcdab799f (fix commits 4c2cf5da..0679c3ac).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 56b87ba1a |
| FND-002 | fixed | 56b87ba1a |
| FND-003 | fixed | 56b87ba1a |
| FND-004 | fixed | 56b87ba1a |
| FND-005 | fixed | 56b87ba1a |
| FND-006 | fixed | 56b87ba1a |
