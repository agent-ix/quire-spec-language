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
stale identity
refuses before admission, and an envelope whose `clause_node` or
`occurrence_key` is not its payload's refuses before the recompile.

Scope: FR-116-AC-1 to FR-116-AC-6.

## Test Procedure

Compile FR-108's ConfigVersion unit; build envelopes by hand for
`Config::ConfigVersion::attemptUpdate`, taking the anchor, frame and
occurrence identities from the compiled package, with the unit's source,
the domain package and the invocation documents in the byte provision.

1. forbidden-parent-change with `change` (`child`, `parent`).
2. changed-version with `change` (`child`, `versionNumber`);
   forbidden-parent-change with `change` (`child`, `versionNumber`); an
   invocation whose `created` lists `child`.
3. forbidden-parent-change with the frame identity taken from a package
   whose `attemptUpdate` frame also modifies `parent`; with a source edit
   that changes the `package_id`; with `operation` naming `missing`; with
   a request whose source does not compile, once with a consistent
   envelope, once with the envelope's `clause_node` replaced by the other
   package's frame node, and once with its `occurrence_key` at another
   ordinal.
4. forbidden-parent-change with its pre snapshot removed from the provision;
   with its invocation bytes edited under the same digest.
5. Step 1's envelope twice.

Tag the tests `#[trace("TC-515", "FR-116-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`; the result holds the source
  digest, `package_id`, the payload's anchor, frame and occurrence
  identities, the three document identities and digests, and both changes.
- Step 2: `inconclusive`, `Verdicts` (`violation`, `success`);
  `reproduced-with-evaluated-witness` holding the payload's change
  (`child`, `versionNumber`) and the replay's (`child`, `parent`);
  `inconclusive`, `NoValue`.
- Step 3: `stale_dependency`/`content-mismatch` (QSpec FR-272-AC-14 and native-diagnostics, quire-specification#174 and #176) naming both frame
  identities, with no admission; FR-098's stale `package_id` refusal;
  `missing_declaration`/`missing-name`; the consistent envelope refuses at
  the recompile, and the other two refuse `stale_dependency`/
  `content-mismatch` naming the envelope's and the payload's frame node,
  then occurrence, with no recompile.
- Step 4: a `ReplayRefusal` holding `unavailable_observation`; a
  `ReplayRefusal` holding `stale_dependency`/`byte-digest-mismatch`. Neither
  settles a result.
- Step 5: equal results. `FrameCounterexample: FamilyPayload` compiles, and
  the envelope has no string-keyed field.
