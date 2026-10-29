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
recompiled package and carries every retained input into its claim, with
the semantic profile selections fixed empty whatever the lock selects.

Scope: FR-484-AC-1 to FR-484-AC-13.

## Fixture

- The synthetic preimage `V2`: `node` 64 `2` hex digits; arguments
  (`<FR-092 P4 digest>`, 0,
  `<FR-092 T4 digest>`),
  QSpec FR-322's P4 parameter `x: Int[0, 9]` with its T4 type, and
  (64 `3` digits, 1, 64 `4` digits).
- The unit `U`: `function inc using v(x: Int[0, 9]): Integer pure { x + 1 }`,
  `function add using v(x: Int[0, 9], y: Int[0, 9]): Integer pure { x + y }`
  and `function small using v(x: Int[0, 9]): Boolean pure { x < 3 }`.
- The unit `U'`: `U` with `function grow using v(n: Integer): Integer pure
  { n + 1 }` and `function wide using v(m: Int[0, 2199023255552]): Integer
  pure { m + 1 }` added: `grow`'s parameter has no finite bound, and
  `wide`'s upper bound is 2^41.
- The unit `UC`, over ADR-018 §6's ConfigVersion domain package document
  `D` (`example/config-version` version `1.0.0`, supplied in the byte
  provision under its `sha256-jcs` digest `<D>`):

  ```
  language "ix:native" edition "1-draft";
  profile v = "quire.model.complete/v1";
  profile t = "quire.temporal.infinite-trace/v1";
  profile p = "quire.protocol.finite-global/v1";
  model Config = "example/config-version";
  temporal ReachesTwo using t over (c: Config::ConfigVersion) clock "model-steps" on origin { always eventually holds(c.versionNumber = 2) }
  protocol Flow using v over (input: Config::ConfigVersion) on origin {
  role R on Config::ConfigVersion;
  run sequence Main { }
  finish End as (outcome: Boolean) { true };
  }
  function inc using v(x: Int[0, 9]): Integer pure { x + 1 }
  ```

  Its lock's `profile_selections` holds one `temporal_profile` and one
  `protocol_profile` row.

## Test Procedure

| Vector | Input | Expected result | Criteria |
| --- | --- | --- | --- |
| VP-01 | `value_parity_obligation(V2)`; the canonical text of `V2`'s first argument alone | The 516-byte text of FR-484-AC-1 and its SHA-256; the argument text and its SHA-256 | FR-484-AC-1 |
| VP-02 | `V2` with no arguments; with its first argument only; with its two parameters declared in the other order | the SHA-256 of the empty-argument text; of the first-argument text; of the swapped-order text, the three all different | FR-484-AC-2 |
| VP-03 | `V2` with each of `node`, one `parameter`, one `position`, one `type` changed, an argument added and one removed; `V2` with its second argument at `position` 3; an operator-parity preimage with `V2`'s `node` | Each a different digest; `PositionOrder` naming index 1 and position 3; a digest unequal to `V2`'s | FR-484-AC-3 |
| VP-04 | `call_site` over `U` selecting `inc`, `add` and `small`; the canonical text of each returned preimage written out from the compiled package's node ids | `inc` and `add`: `value_parity` whose node, arguments and types are the package's and whose identity equals the independent text's digest; `small`: `None` | FR-484-AC-4 |
| VP-05 | A request for `inc` built from VP-04's `CallSite` and `FunctionSite` only, `x` = 3, replayed with `generated` 4 and 5 | `Agrees`; `Diverged`; each `claim()` equal to the retained inputs | FR-484-AC-5 |
| VP-06 | VP-05's request with `add`'s identity; the all-zero identity; an identity over `inc`'s preimage with `type` 64 `4` digits; the first again with an assignment naming `add`'s `y` | `ScalarIdentity` cause `Obligation` naming both digests, settling `Inconclusive(ReplayRefused(stale_dependency))`, for each, the last included | FR-484-AC-6 |
| VP-07 | VP-05's request with `add`'s `declaration` occurrence; with the `expression` occurrence of `inc`'s `x + 1`; with (`inc`'s node, `declaration`, 1); the first with a stale `package_id`; the request retargeted to `small` | `ClaimOccurrence` naming the key and `inc`, no call and no charge, for the first three; `PackageIdMismatch`; `NotAValueFunction` | FR-484-AC-7 |
| VP-08 | Two VP-05 claims differing only in the occurrence key, replayed to `Agrees`, `Diverged`, `RefusedInput` (`x` = 12) and an `Obligation` refusal | The claims are unequal; a `ValueClaim` has no content identity (removed) | FR-484-AC-8 |
| VP-09 | Over `U'`, the `WitnessPacket<NoPayload>` for `inc` at `x` = 3 and for `grow` at `n` = 41, each member filled as FR-484 "The input-arm packet" names, `backend` `generator-under-test`; reconstruct, serialize and read back; build the request from the admitted envelope and replay `grow` against 42 and `inc` against 5 | Both reconstruct with `declared_domains` `[]`, `profile_selections` `[]`, `trace_position` none, `clause_node` the function node and the `declaration` occurrence; every member round-trips; `Agrees` for `grow`, `Diverged` for `inc` | FR-484-AC-9 |
| VP-10 | `wide` at `m` = 2^40 with B 4096, B 40 and B omitted, each against generated 1099511627777; `grow` at `n` = 2^40 with B 40 against 1099511627777; two `wide` claims differing only in A, then only in H | `Agrees`; `PreCallIncomplete` (`Incomplete(ResourceExhausted)`) with the limit cause on `supplied.numeric_magnitude_bits`, limit 40, nothing compared, and a `claim()` differing only in B; a `claim()` equal to the B 4096 one; `Agrees` for `grow`; the A-only and H-only pairs unequal | FR-484-AC-10 |
| VP-11 | `UC` emitted; the RFC 8785 text of each lock `profile_selections` row; the `WitnessPacket<NoPayload>` for `inc` at `x` = 3 filled by "The input-arm packet"; reconstruct; build and decode the request from the envelope; replay against 4 and 5; the same packet over `U` | Exactly the two rows `{"definition":{"authority":"agent-ix","identity":"quire.temporal.infinite-trace/v1"},"role":"temporal_profile"}` and `{"definition":{"authority":"agent-ix","identity":"quire.protocol.finite-global/v1"},"role":"protocol_profile"}`; packet `profile_selections` `[]`; the envelope admitted; the request decodes; `Agrees`, `Diverged`, each `claim()` holding `UC`'s `package_id`; over `U` the same `profile_selections` `[]` | FR-484-AC-11 |
| VP-12 | VP-05's request with `profile_selections` `[("quire.profile.v1", "x")]`, against 4 and 5, then with the all-zero `obligation_identity`, then with a stale `package_id`; a request with `[("quire.profile.unknown/v1", "x")]`; requests with `[("quire.temporal.infinite-trace/v1", "temporal_profile")]` and `[("quire.protocol.finite-global/v1", "protocol_profile")]` | `UnusedProfileSelections` (`invalid_runtime_input`/`unknown-member`) holding the one selection, display text ``invalid_runtime_input/unknown-member: `profile_selections` is supplied for a function-value parity claim, which has no proving run``, no recompile, call or charge, `Inconclusive(ReplayRefused(invalid_runtime_input))` with the claim carried, for each of the first four; `unknown_profile`/`unsupported-selection` at decode for the last three | FR-484-AC-12 |
| VP-13 | VP-12's request with `[("quire.profile.v1", "x")]` retargeted to `frame_operation` (`Config::ConfigVersion`, `attemptUpdate`) and to `state_clause` (`VersionUnchanged`) | The FR-357-AC-20 refusal, `invalid_runtime_input`/`unknown-member` at `/replay/frame_operation` and `/replay/state_clause` naming the target, the entry and `function`, not `UnusedProfileSelections`, before the recompile | FR-484-AC-13 |

## Expected Results

- VP-01 to VP-03: the stated bytes and digests, and every changed member
  changes the digest.
- VP-04: the minted identity equals the independently written preimage's;
  a predicate has none.
- VP-05 to VP-10: the stated settlements, each with the claim identity
  carried unchanged, and VP-09's packets admitted with no member chosen by
  the producer.
- VP-11: a claim over a package whose lock selects temporal and protocol
  profiles builds, admits and replays with the empty semantic profile
  selections, and no lock row reaches the packet or the request.
- VP-12: a value-parity request carrying any semantic profile selection is
  refused before the recompile, and a lock row is no admissible selection.
- VP-13: the replay-target check comes before the profile-selection check.
