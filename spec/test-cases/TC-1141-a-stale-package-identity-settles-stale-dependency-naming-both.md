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
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-339
    type: verifies
---
# TC-1141: A stale package identity settles stale-dependency naming both identities

## Description

Verify that a state-graph, EN-1 closure or EN-1 component certificate
carries the `package_id` of the package the engine explored, that
`check_state_graph`, `check_closure` and `check_components` reject a
certificate naming another package before any rule, and that a certificate
mismatch or a stale replay request settles `Inconclusive(StaleDependency{requested,
recompiled})` naming both identities.

Scope: FR-169-AC-12, FR-127-AC-12, FR-282-AC-9, FR-338-AC-11,
FR-339-AC-4, FR-127-AC-13, FR-282-AC-10.

## Test Procedure

Use ADR-022 §7.1's ConfigVersion unit and FR-127-AC-3's weak `each` claim
for steps 1 to 4, FR-338-AC-1's TP-1 claim for step 5 and FR-339-AC-1's
weak `each` claim for step 6.

1. Run `ReachesTwo` for `c = a` unreduced and read its certificate's
   `package_id`; check the certificate.
2. Replace the certificate's `package_id` with the `package_id` of the same
   unit with its target edited to `c.versionNumber = 3`, check
   it, and settle the item.
3. Replay `ReachesTwo`'s witness with a request whose `package_id` is the
   edited unit's, and settle the item.
4. Replay FR-127-AC-3's counterexample with a request whose `package_id` is
   another package's, settle the item and write its FR-331 record.
5. Run FR-338-AC-1's claim and check its `ClosureCertificate`. Replace the
   certificate's `package_id` with that of the same unit with the claim's
   bound edited to `1001`, check it, settle the item and write its FR-331
   record. Remove the initial product state from the stale certificate and
   check it again. Check the stale certificate once more with a request
   whose clause root is the edited unit's clause node, which does not
   resolve in FR-338-AC-1's recompiled package, and settle the item.
6. Run FR-339-AC-1's claim and check its `ComponentCertificate`. Replace
   its closure's `package_id` with that of the same unit with the claim's
   target edited to `c.versionNumber = 1`, check it, settle the item and
   write its FR-331 record. Change the stale certificate's witness of
   `{(*, 0, q1)}` to `UnfairWeak` naming `upd(a)` and check it again.

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
- Step 5 (FR-338-AC-11, FR-282-AC-10, FR-127-AC-13): the certificate's
  `package_id` equals the recompiled one and `check_closure` accepts it;
  the stale certificate returns `PackageMismatch{certificate, recompiled}`
  with both identities, and the item settles `inconclusive`, `unsettled`,
  `Inconclusive(StaleDependency{requested, recompiled})` with `requested`
  the certificate's `package_id`, written `stale-dependency`, never
  `certificate-rejected`; with its initial product state removed it still
  returns `PackageMismatch`, not `InitialMissing`; with the unresolved
  clause root it returns `PackageMismatch` and settles
  `Inconclusive(StaleDependency{requested, recompiled})`, not `Failed`.
- Step 6 (FR-339-AC-4, FR-282-AC-10, FR-127-AC-13): the closure's
  `package_id` equals the recompiled one and `check_components` accepts the
  certificate; the stale certificate returns `PackageMismatch{certificate,
  recompiled}` with both identities, and the item settles
  `Inconclusive(StaleDependency{requested, recompiled})` with `requested`
  the closure's `package_id`, written `stale-dependency`, never
  `certificate-rejected`; with the `upd(a)` witness it still returns
  `PackageMismatch`, not `WitnessFails`.
