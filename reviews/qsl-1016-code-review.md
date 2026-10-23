---
id: SR-16304
title: "Code review of QSL-1016 function-value parity obligation identity (QSL PR #1386)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@4ece0487943c6c9b60cd3132ee166b347f61780d; qsl-replay/src/call_site.rs, qsl-replay/src/execute.rs, qsl-replay/src/execute/parity_identity.rs, qsl-replay/src/execute/value_parity.rs, qsl-replay/src/execute/tests.rs, qsl-replay/src/execute/tests/value_parity_identity.rs, qsl-replay/src/identity.rs, qsl-replay/src/lib.rs, qsl-replay/src/outcome.rs, qsl-replay/src/proof_result.rs, qsl-replay/src/scalar.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-484
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: references
---
# SR-16304: Code review of QSL-1016 function-value parity obligation identity (QSL PR #1386)

## Summary

Ticket: QSL-1016. Code review with the Rust lane (`rust-review`) folded in, scoped to
`git diff origin/main...4ece04879` (11 files, all in `qsl-replay`). The production change is
correct against FR-484. `call_site` and `replay_value_parity` both build the preimage through one
shared builder (`value_parity_preimage`). The refusal order is `NotAValueFunction` (in `select`),
then the occurrence key, then the identity recompute, then the argument join. That matches FR-484
Behavior and FR-357. The findings are a changelog inaccuracy and one duplicated spelling of the
declaration occurrence.

## Verdict

**CONDITIONAL**: low findings only. No correctness defect, panic surface or vendoring found.

Measured:

- Spec vectors. All five FR-484-AC-1/AC-2 digests and the 516-byte text were recomputed
  independently with Python `json.dumps(sort_keys=True, separators=(',',':'))` plus SHA-256. All
  match the spec and the test constants: `7939…b92c`, `34f0…270f`, `9083…66b7`, `3333…cbc0` and
  `f052…76fd`. The P0/T0 node ids appear in QSpec FR-322 and TC-233 in qspec-main-now.
- Refusal order. `settle` runs decode, `function_target`, `request_limits`, `recompile`
  (`PackageIdMismatch`), `select` (`UnknownFunction`/`NotAValueFunction`), `check_identity`
  (occurrence, then recompute), then `arguments`. The `Fault` for a missing occurrence key is
  unreachable, because `ReplayRequest::decode` already refuses `MissingOccurrenceKey` for a
  function target (request.rs:584).
- `ValueIdentity::sent` resolves `supplied` with `unwrap_or_default()`, the same resolution
  `ReplayRequest::decode` uses (request.rs:864). So an omitted B and a stated B of 4096 give one
  claim.
- Re-exports. The new `lib.rs` exports (`ValueParitySite`, `ValueParityPreimage`,
  `ValueParityArgument`, `value_parity_obligation`, `ValueClaim`) are new vocabulary. No moved type
  is re-exported. No vendoring and no copied files.
- Gates. `locked-build.sh … cargo test -p qsl-replay --lib -- tc_1252 tc_904 tc_921 call_site`
  passed 71 tests with 0 failed. fmt, clippy and the full suite were not re-run: the PR body
  reports them clean, and the brief limits this review to light builds.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | PR changelog lists `IdentityEncodeError::PositionOrder` as "New", but `IdentityEncodeError` is not `#[non_exhaustive]`, so the added variant breaks any exhaustive `match` downstream; it belongs under BREAKING | qsl-replay/src/execute/parity_identity.rs:22 |
| FND-002 | low | The function's `declaration` occurrence `(node, "declaration", 0)` is spelled again in `check_identity`, beside `function_claim` (execute.rs:1334) and `call_site` (call_site.rs:594); one shared constructor would keep the mint and check sites from drifting | qsl-replay/src/execute/value_parity.rs:192 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | PR body (not a commit): the changelog now lists `IdentityEncodeError` gaining `PositionOrder` under BREAKING, since the enum is not `non_exhaustive` |
| FND-002 | fixed | 1adffb125 |
