---
id: SR-12351
title: "gap analysis of QSL-1007 derived identity projection (QSL PR #1268)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@785c72b580d0734d9909437fa6d4058df88ba241; qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit.rs, src/population_fixtures.rs, qsl-package/src/checked_v2.rs, qsl-semantics/src/library/package_identity.rs; QSpec PR #371 at ff52d3e8: spec/objects/interfaces/FR-322-checked-package-artifact.md (FR-322-AC-14, FR-322-AC-100), proposals/checked-package-v2/fixtures/adverse.json"
review_set: subset
---
# SR-12351: gap analysis of QSL-1007 derived identity projection (QSL PR #1268)

## Summary

Ticket: QSL-1007. Plan completion: not assessed.

The ticket's green criterion is met: QSL conformance passes with `QSPEC_DIR` at the #371 head. The
reviewer measured `conformance_i2_read`, `conformance_c14_source_map_lookup`,
`conformance_dependency_selection_vectors` and `conformance_fr340_frame_mutations` passing, plus the
full `qsl-package` lib suite (232 tests). `qspec_union_value_application_keys_are_admitted_by_i2`
is part of that suite.

FR-322-AC-14 is covered for QSL's read. Every positive fixture is admitted, and IR checks that its
published `package_id` equals the digest over the selection object plus the derived projection.

Production code sites that must stay until IR-975 (the IR-275 wire slice):

- `qsl-package/src/emit.rs:1307` and `src/population_fixtures.rs:456` fill IR's
  `CheckedPackageIdentityPreimageRefV2.identity_projection`. That is a required field, and IR's
  `encode_checked_package_v2` writes it onto the wire (`encode.rs:278`). Both sites already take it
  from `&semantic_graph.nodes`, so the only part left is the wire member, which IR owns. They cannot
  change without IR's model crate.
- `qsl-package/src/checked_v2.rs:673` encodes the library's package-id preimage from IR's typed
  admitted preimage, which carries the projection.
- `qsl-semantics` `project_declarations` reads the package-id preimage. Under FR-322 that preimage
  still contains the projection, so the resolver itself does not need to change. Only the source of
  its bytes does. The PR body's statement that the `qsl-semantics` library readers use "the IR wire
  shape" is therefore imprecise.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-322-AC-100 (`adverse.json` `shipped_projection_cases`) has no QSL test. QSL's I2 read cannot refuse an `identity_projection` on the wire `malformed_wire` while IR's reader requires that member, and the harness installs one on every read. Deferral is legitimate, blocked by IR-975, but no QSL ticket owns the follow-up. File one blocked by IR-975 (content in the Linear comment). | qsl-package/src/checked_v2/tests.rs:1361-1389, qsl-package/src/checked_v2/tests.rs:1612 |
| FND-002 | medium | After #371 merges, QSL's production emitter still writes `identity_preimage.identity_projection` (through IR's encoder), which the #371 schema refuses, so a QSL-emitted v2 package no longer conforms to FR-322. It is blocked on IR-975's model change, but it needs the same follow-up ticket: adopt IR-975's preimage shape at `emit.rs:1307` and `population_fixtures.rs:456`, and derive the package-id preimage at `checked_v2.rs:673`. | qsl-package/src/emit.rs:1307, src/population_fixtures.rs:456, qsl-package/src/checked_v2.rs:673 |

## Verdict

No high findings. The two medium findings are cross-repo deferrals the PR states openly. Both need
one QSL follow-up ticket blocked by IR-975 before QSL-1007 closes. They do not need a fix in this
PR. Mergeable once that ticket exists.
