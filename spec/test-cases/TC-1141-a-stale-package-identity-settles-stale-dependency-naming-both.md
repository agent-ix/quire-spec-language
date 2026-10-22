---
id: TC-1141
title: "A stale package identity settles stale-dependency naming both identities"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-282
    type: verifies
---
# TC-1141: A stale package identity settles stale-dependency naming both identities

## Description

Verify that a state-graph certificate carries the `package_id` of the
package the engine explored, that `check_state_graph` rejects a certificate
naming another package before any rule, and that a certificate mismatch or a
stale replay request settles `Inconclusive(StaleDependency{requested,
recompiled})` naming both identities.

Scope: FR-169-AC-12, FR-127-AC-12, FR-282-AC-9.

## Test Procedure

Use ADR-022 §7.1's ConfigVersion unit and FR-127-AC-3's weak `each` claim.

1. Run `ReachesTwo` for `c = a` unreduced and read its certificate's
   `package_id`; check the certificate.
2. Replace the certificate's `package_id` with the `package_id` of the same
   unit with its target edited to `c.versionNumber = 3`, check
   it, and settle the item.
3. Replay `ReachesTwo`'s witness with a request whose `package_id` is the
   edited unit's, and settle the item.
4. Replay FR-127-AC-3's counterexample with a request whose `package_id` is
   another package's, settle the item and write its FR-331 record.

Tag the tests `#[trace("TC-1141", "<AC>")]` with the AC each step backs.

## Expected Results

- Step 1 (FR-169-AC-12): the certificate's `package_id` equals the
  recompiled one; the checker accepts it.
- Step 2 (FR-169-AC-12, FR-282-AC-9): `PackageMismatch{certificate,
  recompiled}` with both identities and no rule evaluated; the item settles
  `inconclusive`, `unsettled`, `Inconclusive(StaleDependency{requested,
  recompiled})` with `requested` the certificate's `package_id`.
- Step 3 (FR-169-AC-12): replay refuses `PackageIdMismatch`; the item settles
  `Inconclusive(StaleDependency{requested, recompiled})` with `requested`
  the request's `package_id`, not `ReplayRefused`.
- Step 4 (FR-127-AC-12): the item settles `inconclusive` with cause
  `StaleDependency{requested, recompiled}`, written
  `{"type": "stale-dependency", "requested": …, "recompiled": …}` with both
  `PackageId`s.
