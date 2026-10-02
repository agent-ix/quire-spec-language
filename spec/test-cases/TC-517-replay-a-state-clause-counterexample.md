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
is inconclusive, a precondition replays over its pre state alone, a stale
envelope identity refuses before admission, and an admission failure
settles no result.

Scope: FR-122-AC-1 to FR-122-AC-6.

## Test Procedure

Compile FR-108's ConfigVersion unit; build envelopes by hand for
`VersionUnchanged` (an `Invocation` observation) and `ParentOrder` (a
`Current` observation, anchor `handler validate`, self `child`), taking each
clause node and `claim` occurrence for the envelope from the compiled
package, with the unit's source, the domain package and FR-108's
observation documents in the byte provision. For step 4, compile TC-466
step 3's `probe` unit (precondition `ReachesTarget`) and use its chain
`a -> b -> c` pre snapshot with a `PreCall` observation.

1. `VersionUnchanged` over changed-version, on a `Witness`-arm and on an
   `Input`-arm envelope; `ParentOrder` over violating-parent.
2. `VersionUnchanged` over unchanged-version; `ParentOrder` over
   healthy-parent; `ParentOrder` over violating-parent with the request's
   evaluation budget at zero.
3. `VersionUnchanged` over changed-version with the envelope's
   `clause_node` set to `ParentOrder`'s node; with its `occurrence_key` at
   ordinal 1; with both; each again with the invocation removed from the
   provision;
   with a source edit that changes the `package_id`; with `clause` naming
   `Absent`; with `clause` naming `sameIdentity`.
4. `ReachesTarget` with `PreCall` over the chain's pre snapshot, `self`
   `a`, `target` `a`; with `self` `a`, `target` `c`. Form mismatches, each
   once with its documents in the provision and once without:
   `ReachesTarget` with an `Invocation` observation over the `probe`
   invocation; `VersionUnchanged` with a `PreCall` observation;
   `VersionUnchanged` with a `Current` observation over healthy-parent's
   snapshot.
5. `VersionUnchanged` over forbidden-parent-change; over changed-version
   with its pre snapshot removed from the provision; over changed-version
   with its invocation bytes edited under the same digest; `ParentOrder` over
   incomplete-population's snapshot.
6. Step 1's `VersionUnchanged` envelope twice.

Tag the tests `#[trace("TC-517", "FR-122-AC-n")]`.

## Expected Results

- Step 1: `reproduced-with-evaluated-witness`,
  `reproduced-without-witness`, `reproduced-with-evaluated-witness`. Each
  `Witness`-arm result's QSpec FR-351 record holds `false` as deciding element,
  index 0, an empty value path and no trace position; the `Input`-arm
  result holds `false` and no QSpec FR-351 record. Each
  result holds the source digest, `package_id`, the payload's `clause`, the
  envelope's `clause_node` and `occurrence_key`, and the identities and digests of the
  invocation and both snapshots, or of the one current snapshot.
- Step 2: `inconclusive`, `Verdicts` (`violation`, `success`) twice, each
  holding the evaluated `true` and no QSpec FR-351 record;
  `inconclusive`, `NoValue`.
- Step 3: `stale_dependency`/`content-mismatch` (QSpec FR-272-AC-14 and native-diagnostics, quire-specification#174 and #176) naming the envelope's and
  the recompiled clause node, then both occurrences, then both clause
  nodes again (the node goes first), each with no admission, and the same
  three refusals with the invocation removed;
  FR-098's stale `package_id` refusal; `missing_declaration`/`missing-name`
  twice.
- Step 4: `reproduced-with-evaluated-witness`, holding the one pre
  snapshot's identity and digest, no post snapshot, and `self` `a` and
  `target` `a`; `inconclusive`, `Verdicts` (`violation`, `success`); each
  form mismatch refuses `wrong_snapshot`/`wrong-observation` naming the
  clause kind and the form, with no admission, with and without its
  documents.
- Step 5: `ReplayRefusal`s holding `frame_violation`/`unauthorized-change`
  (naming `child` and `parent`), `unavailable_observation`,
  `stale_dependency`/`byte-digest-mismatch` and `Incomplete` `incomplete_population`/
  `incomplete-scope`. None settles a result.
- Step 6: equal results. `StateClauseCounterexample: FamilyPayload`
  compiles, the envelope has no string-keyed field, and a payload's
  `observation` is one of `PreCall`, `Invocation` or `Current`.
