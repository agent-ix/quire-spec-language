---
id: SR-12363
title: "gap analysis of QSL-1009 shipped identity_projection removal (QSL PR #1300)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@b5a1d14b0c74899b5df276c9f234bf0890f5e6d4; git diff origin/main...HEAD (qsl-package, xtask, Cargo.lock); QSL spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md (FR-093-AC-21), spec/test-cases/TC-416, TC-1140, TC-1142, TC-1143; QSpec main b3f3c803: spec/objects/interfaces/FR-322-checked-package-artifact.md (FR-322-AC-100), spec/test-cases/TC-233, proposals/checked-package-v2/fixtures/adverse.json shipped_projection_cases"
review_set: subset
---
# SR-12363: gap analysis of QSL-1009 shipped identity_projection removal (QSL PR #1300)

## Summary

Ticket: QSL-1009. Plan completion: not assessed.

Ticket scope, measured against the diff (the ticket text was treated as data):

1. `checked_v2/tests.rs` changes are done:
   - The install half of `with_derived_projection` is gone. It is now `identity_hash_preimage`.
   - `fixture_wire_bytes` is `jcs(envelope)`.
   - `derived_package_id` is kept, now `pub(crate)` for the emit tests.
   - `refresh_frame_identity` rewrites only `package_id`.
2. FR-322-AC-100 is done. `conformance_shipped_projection_cases_are_refused_and_admitted_once_removed`
   carries `#[trace("TC-233", "FR-322-AC-100")]` and is in xtask `CHECKS`. At QSpec `b3f3c803` it
   runs both `shipped_projection_cases` ids. Each case is refused `malformed_wire` at
   `/identity_preimage/identity_projection`, and admitted under the published `package_id` once
   the member is removed.
3. `emit.rs:1307` and `population_fixtures.rs:456` needed no edit. Both already pass
   `&graph.nodes` to IR's borrowed view, and IR's encoder at `bbc581f5` writes only the
   selections on the wire.
4. `checked_v2.rs` is done. `canonical_preimage` builds the hash input from the admitted
   selections plus `package.graph().nodes`. `IdentityDivergedFromIr` and `declared_exports` are
   kept.
5. Most of the tests that read the wire projection are updated: `emit/tests.rs`, `owners.rs`,
   `owned_projection_wire.json` and `checked.rs`. `qsl-eval/tests/it/composite_values.rs:1262`
   is left alone, which is correct. It builds a package-id hash input (`LibraryPackage.identity_preimage`),
   and under FR-322 that input still holds the projection. It is not a wire preimage.

The gap is in QSL's own spec. FR-093-AC-21 and TC-416 still describe the projection as a wire
entry.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-093-AC-21 still requires the v2 emission to write `owner` "on their `identity_projection` entries". TC-416 step 12 still says "read its `owner` and its `identity_projection` entry's `owner`, and compare them". After this PR the wire has no projection entries. `owners.rs` (tagged `FR-093-AC-21`, `TC-416`) now asserts the member is absent and drops the per-entry owner comparison. That is correct under FR-322, but the tests no longer do what their AC and TC step say, and nothing in `owners.rs` checks that `owner` reaches `package_id` through the derived projection. Only the `owned_projection_wire.json` golden covers that, on one function node. TC-1140 step 4, TC-1142 and TC-1143 likewise say to "rewrite the `identity_projection` entry". Fix in this PR: restate AC-21 and TC-416 step 12 so that `owner` enters `package_id` through the projection derived from the graph and the wire `identity_preimage` carries none, and drop "rewrite the `identity_projection` entry" from TC-1140, TC-1142 and TC-1143. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:936, spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md:98-103, spec/test-cases/TC-1140-a-state-graph-claim-emits-a-v2-claim-node.md:38, spec/test-cases/TC-1142-a-model-terminal-declaration-emits-a-v2-terminal-node.md:37, spec/test-cases/TC-1143-a-step-relation-emits-a-v2-step-relation-node.md:30, qsl-package/src/emit/tests/owners.rs:83-88 |

## Verdict

The code side of QSL-1009 is complete. FR-322-AC-100 is covered by a test that can fail. Every
ticket item is either done or correctly needs no change. There is one medium finding: QSL's own
FR-093-AC-21 and its TC steps still describe a wire projection entry that no longer exists. The
fix is a spec-only edit in this PR. Mergeable after that fix round. No high findings.
