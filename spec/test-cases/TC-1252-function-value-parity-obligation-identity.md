---
id: TC-1252
title: "A function-value parity claim's obligation identity is minted by call_site, recomputed by replay and bound with every retained input"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-484
    type: verifies
---
# TC-1252: A function-value parity claim's obligation identity is minted by call_site, recomputed by replay and bound with every retained input

## Description

Verify that `qsl_replay::value_parity_obligation` encodes FR-484's preimage
byte for byte, that `call_site` mints the identity of a non-`Boolean`
function and none for a predicate, and that `replay_value_parity` checks
the request's occurrence key and obligation identity against the
recompiled package and carries every retained input into its claim.

Scope: FR-484-AC-1 to FR-484-AC-9.

## Fixture

- The synthetic preimage `V2`: `node` 64 `2` hex digits; arguments
  (`ebe64b4c3bc2cc5f3460a43480f8979f5df24c5e7665d9e89890f43517dd8eda`, 0,
  `652cc5b63910aca98b8c91b1c1ba42a8da1f568517c70f37de083414cd192477`),
  QSpec FR-322's P4 parameter `x: Int[0, 9]` with its T4 type, and
  (64 `3` digits, 1, 64 `4` digits).
- The unit `U`: `function inc using v(x: Int[0, 9]): Integer pure { x + 1 }`,
  `function add using v(x: Int[0, 9], y: Int[0, 9]): Integer pure { x + y }`
  and `function small using v(x: Int[0, 9]): Boolean pure { x < 3 }`.
- The unit `U'`: `U` with `function grow using v(n: Integer): Integer pure
  { n + 1 }` added, whose parameter has no finite bound.

## Test Procedure

| Vector | Input | Expected result | Criteria |
| --- | --- | --- | --- |
| VP-01 | `value_parity_obligation(V2)`; the canonical text of `V2`'s first argument alone | The 516-byte text of FR-484-AC-1 and digest `793909279b4897ab8f03f3c14547fc08861d353d979e471aa423c2bce9d8b92c`; the argument text with digest `34f0d98f1306d5e3720eec3469a6181c73f809ac51c6fe608e97129e7f94270f` | FR-484-AC-1 |
| VP-02 | `V2` with no arguments; with its first argument only; with its two parameters declared in the other order | `908362fdf1c3d0bb20aadf2249e285964ad024f5fd994d6c01e88024839166b7`; `3333f20dd244bd7043f410893713ed40ffc067b45cae410050898de89834cbc0`; `f052b4583fc320a5a470209188b911e75fe02d55a28d079a5e614ac0f44e76fd` | FR-484-AC-2 |
| VP-03 | `V2` with each of `node`, one `parameter`, one `position`, one `type` changed, an argument added and one removed; `V2` with its second argument at `position` 3; an operator-parity preimage with `V2`'s `node` | Each a different digest; `PositionOrder` naming index 1 and position 3; a digest unequal to `V2`'s | FR-484-AC-3 |
| VP-04 | `call_site` over `U` selecting `inc`, `add` and `small`; the canonical text of each returned preimage written out from the compiled package's node ids | `inc` and `add`: `value_parity` whose node, arguments and types are the package's and whose identity equals the independent text's digest; `small`: `None` | FR-484-AC-4 |
| VP-05 | A request for `inc` built from VP-04's `CallSite` and `FunctionSite` only, `x` = 3, replayed with `generated` 4 and 5 | `Agrees`; `Diverged`; each `claim()` equal to the retained inputs | FR-484-AC-5 |
| VP-06 | VP-05's request with `add`'s identity; the all-zero identity; an identity over `inc`'s preimage with `type` 64 `4` digits; the first again with an assignment naming `add`'s `y` | `ScalarIdentity` cause `Obligation` naming both digests, settling `Inconclusive(ReplayRefused(stale_dependency))`, for each, the last included | FR-484-AC-6 |
| VP-07 | VP-05's request with `add`'s `declaration` occurrence; with the `expression` occurrence of `inc`'s `x + 1`; with (`inc`'s node, `declaration`, 1); the first with a stale `package_id`; the request retargeted to `small` | `ClaimOccurrence` naming the key and `inc`, no call and no charge, for the first three; `PackageIdMismatch`; `NotAValueFunction` | FR-484-AC-7 |
| VP-08 | Two VP-05 claims differing only in `content_identity`, replayed to `Agrees`, `Diverged`, `RefusedInput` (`x` = 12) and an `Obligation` refusal; a `content_identity` naming no artifact; two claims differing only in the occurrence key | Each report carries its own content identity; the claims are unequal; no refusal for the unmatched identity; the occurrence-only claims are unequal | FR-484-AC-8 |
| VP-09 | Over `U'`, the `WitnessPacket<NoPayload>` for `inc` at `x` = 3 and for `grow` at `n` = 41, each member filled as FR-484 "The input-arm packet" names, `backend` `generator-under-test`; reconstruct, serialize and read back; build the request from the admitted envelope and replay `grow` against 42 and `inc` against 5 | Both reconstruct with `declared_domains` `[]`, `trace_position` none, `clause_node` the function node and the `declaration` occurrence; every member round-trips; `Agrees` for `grow`, `Diverged` for `inc` | FR-484-AC-9 |

## Expected Results

- VP-01 to VP-03: the stated bytes and digests, and every changed member
  changes the digest.
- VP-04: the minted identity equals the independently written preimage's;
  a predicate has none.
- VP-05 to VP-09: the stated settlements, each with the claim identity
  carried unchanged, and VP-09's packets admitted with no member chosen by
  the producer.
