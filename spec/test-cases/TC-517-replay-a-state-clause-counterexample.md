---
id: TC-517
title: "The replay facade replays a state-clause counterexample and keeps its identities"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: verifies
---
# TC-517: The replay facade replays a state-clause counterexample and keeps its identities

## Description

Verify replay of a `WitnessEnvelope<StateClauseCounterexample>`: a clause
that evaluates `false` reproduces, a clause that holds or completes no value
is inconclusive, a stale identity refuses before admission, an envelope
whose `clause_node` or `occurrence_key` is not its payload's refuses before
the recompile, and an admission failure settles no result.

Scope: FR-122-AC-1 to FR-122-AC-6.

## Test Procedure

Compile FR-108's ConfigVersion unit; build envelopes by hand for
`VersionUnchanged` (an `Invocation` observation) and `ParentOrder` (a
`Current` observation, anchor `handler validate`, self `child`), taking each
clause node and `claim` occurrence from the compiled package, with the
unit's source, the domain package and FR-108's observation documents in the
byte provision.

1. `VersionUnchanged` over changed-version, on a `Witness`-arm and on an
   `Input`-arm envelope; `ParentOrder` over violating-parent.
2. `VersionUnchanged` over unchanged-version; `ParentOrder` over
   healthy-parent; `ParentOrder` over violating-parent with the request's
   evaluation budget at zero.
3. `VersionUnchanged` over changed-version with envelope and payload
   `clause_node` both `ParentOrder`'s node; with envelope and payload
   occurrence both at ordinal 1; with a source edit that changes the
   `package_id`; with `clause` naming `Absent`; with `clause` naming
   `sameIdentity`.
4. Over a request whose source does not compile: step 1's changed-version
   envelope as built; with only its envelope `clause_node` replaced by
   `ParentOrder`'s node; with only its envelope `occurrence_key` at
   ordinal 1.
5. `VersionUnchanged` over forbidden-parent-change; over changed-version
   with its pre snapshot removed from the provision; over changed-version
   with its invocation bytes edited under the same digest; with a `Current`
   observation over healthy-parent's snapshot.
6. Step 1's `VersionUnchanged` envelope twice.

Tag the tests `#[trace("TC-517", "FR-122-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`,
  `reproduced-without-witness`, `reproduced-with-evaluated-witness`. Each
  result holds the source digest, `package_id`, the payload's `clause`,
  `clause_node` and `occurrence`, and the identities and digests of the
  invocation and both snapshots, or of the one current snapshot.
- Step 2: `inconclusive`, `Verdicts` (`violation`, `success`) twice;
  `inconclusive`, `NoValue`.
- Step 3: `stale_dependency`/`revision-mismatch` naming both clause nodes,
  then both occurrences, each with no admission; FR-098's stale
  `package_id` refusal; `missing_declaration`/`missing-name` twice.
- Step 4: the consistent envelope refuses at the recompile; the other two
  refuse `stale_dependency`/`revision-mismatch` naming the envelope's and
  the payload's clause node, then occurrence, with no recompile.
- Step 5: `ReplayRefusal`s holding `frame_violation`/`unauthorized-change`
  (naming `child` and `parent`), `unavailable_observation`,
  `stale_dependency`/`byte-digest-mismatch` and `wrong_snapshot`/
  `wrong-observation`. None settles a result.
- Step 6: equal results. `StateClauseCounterexample: FamilyPayload`
  compiles, the envelope has no string-keyed field, and a payload's
  `observation` is one of `Invocation` or `Current`.
