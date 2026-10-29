---
id: TC-515
title: "The replay facade replays a frame counterexample and keeps its identities"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: verifies
---
# TC-515: The replay facade replays a frame counterexample and keeps its identities

## Description

Verify replay of a `WitnessEnvelope<FrameCounterexample>`: an agreeing
replay reproduces, a replay that finds no violation is inconclusive, a
request whose replay target differs from the envelope's refuses, and a
missing document is a refusal that settles no result.

Scope: FR-116-AC-1, FR-116-AC-2, FR-116-AC-4, FR-116-AC-5, FR-116-AC-8, FR-116-AC-9.

## Test Procedure

Compile FR-108's ConfigVersion unit; build envelopes by hand for
`Config::ConfigVersion::attemptUpdate`, taking the anchor, frame and
occurrence identities from the compiled package, with the unit's source,
the domain package and the invocation documents in the byte provision.

1. forbidden-parent-change with `change` (`child`, `parent`).
2. changed-version with `change` (`child`, `versionNumber`);
   forbidden-parent-change with `change` (`child`, `versionNumber`); an
   invocation whose `created` lists `child`.
3. forbidden-parent-change with the request's `frame_operation` naming
   operation `missing`; with a request whose source does not compile and a
   consistent envelope.
4. forbidden-parent-change with its pre snapshot removed from the provision.
5. Step 1's envelope twice.
6. Over FR-121-AC-18's domain package (`Sub` inherits `attemptUpdate`),
   step 1's envelope rebuilt over that package, the request's and the
   envelope's target equal unless stated: the target `call_site` returns
   for `Config::Sub::attemptUpdate`; the target `{Config::Sub,
   attemptUpdate}`; the request's target `function` `sameIdentity` with the
   envelope's `{Config::ConfigVersion, attemptUpdate}`; the request's
   `{Config::ConfigVersion, attemptUpdate}` with the envelope's
   `{Config::ConfigVersion, probe}`; the third and fourth cases again with a request whose source does not
   compile, and again with the source's byte-provision entry removed.
7. With the request's and the envelope's target equal: `{Config::VersionNumber,
   attemptUpdate}`; `{Config::Absent, attemptUpdate}`; and, over a domain
   package where `Sub` has the supertypes `ConfigVersion` and `Other`, both
   declaring `attemptUpdate`, `{Config::Sub, attemptUpdate}`.

Tag the tests `#[trace("TC-515", "FR-116-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`; the result holds the source
  labels, the package reference, the payload's anchor, frame and occurrence
  identities, the three document identities, and both changes.
- Step 2: `inconclusive`, `Verdicts` (`violation`, `success`);
  `reproduced-with-evaluated-witness` holding the payload's change
  (`child`, `versionNumber`) and the replay's (`child`, `parent`);
  `inconclusive`, `NoValue`.
- Step 3: `missing_declaration`/`missing-name`; the consistent envelope
  refuses at the recompile.
- Step 4: a `ReplayRefusal` holding `unavailable_observation`, which settles
  no result.
- Step 5: equal results. `FrameCounterexample: FamilyPayload` compiles, and
  the envelope has no string-keyed field.
- Step 6: `reproduced-with-evaluated-witness` with the target
  `{Config::ConfigVersion, attemptUpdate}`; `invalid_model_binding`/
  `malformed-declaration` naming `{Config::Sub, attemptUpdate}`;
  `invalid_runtime_input`/`unknown-member` at `/replay/function` naming
  `replay_frame` and `frame_operation`; `invalid-request` naming both
  targets; the same two refusals
  with the source that does not compile; `missing_import`/
  `missing-selection` for both with the provision entry removed.
  `FrameCounterexample` has no member naming the operation (FR-116-AC-8).
- Step 7: `invalid_model_binding`/`malformed-declaration` naming `{Config::VersionNumber, attemptUpdate}`; `missing_declaration`/`missing-name`; `ambiguous_declaration`/`ambiguous-name` naming `{Config::Sub, attemptUpdate}` (FR-116-AC-9).
