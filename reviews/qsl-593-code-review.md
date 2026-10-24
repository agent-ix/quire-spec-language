---
id: SR-4967
title: "QSL-593 code-review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@0e0661060091e24101f2e462fdf765ce278d45b9; qsl-foundation; qsl-route; qsl-replay identity; FR-288; FR-289; TC-772; TC-773"
review_set: subset
---

## Summary

QSL-593 / PR #729: FAIL. Shared storage is centralized and linked/process origin values are preserved, but acceptance is incomplete. Existing provider-order tests remain intact. Reviewed frozen head against origin/main; no execution claim.

## Verdict

FAIL — scoped static review; findings require disposition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The ticket explicitly forbids compatibility layers, shims and re-exports, but the old replay Backend path remains an alias and qsl-route/src/lib | qsl-replay/src/identity.rs:26 |
| FND-002 | medium | descriptor_from_manifest is only a forwarding wrapper around the still-public BackendDescriptor::admit conversion | qsl-route/src/lib.rs:160 |
| FND-003 | medium | The new test supplies an already-constructed Candidate, digest and capability tuples; it never invokes the driver registry builder, provider manifest reader or plugin hello path | qsl-route/tests/it/routing.rs:52 |

## Evidence and scope

```yaml
{
  "clean": false,
  "scope": [
    {
      "id": "BackendId",
      "path": "qsl-foundation/src/backend.rs",
      "role": "examined",
      "excerpt": "pub struct BackendId(String);"
    },
    {
      "id": "foundation exports",
      "path": "qsl-foundation/src/lib.rs",
      "role": "examined",
      "excerpt": "pub use backend::{BackendId, EmptyBackendIdentity};"
    },
    {
      "id": "manifest digest ownership doc",
      "path": "qsl-foundation/src/digest.rs",
      "role": "examined",
      "excerpt": "/// `qsl_route` and `qsl_replay` both depend on this F crate, so the one"
    },
    {
      "id": "replay alias",
      "path": "qsl-replay/src/identity.rs",
      "role": "examined",
      "excerpt": "pub use qsl_foundation::BackendId as Backend;"
    },
    {
      "id": "manifest wrapper",
      "path": "qsl-route/src/lib.rs",
      "role": "examined",
      "excerpt": "BackendDescriptor::admit(backend, origin, manifest_digest, advertised, domains)"
    },
    {
      "id": "new conversion test",
      "path": "qsl-route/tests/it/routing.rs",
      "role": "examined",
      "excerpt": "#[trace(\"TC-772\", \"FR-288-AC-1\", \"FR-288-AC-5\")]"
    },
    {
      "id": "origin controls",
      "path": "qsl-route/tests/it/provider_origin.rs",
      "role": "examined",
      "excerpt": "#[trace(\"TC-772\", \"FR-288-AC-6\")]"
    },
    {
      "id": "FR-288",
      "path": "spec/functional/FR-288-build-the-registry-from-provider-manifests.md",
      "role": "examined",
      "excerpt": "Layer R shall convert an FR-331 provider manifest into a\n  `BackendDescriptor` by one function, used for compile-time and plugin\n  manifests alike."
    },
    {
      "id": "FR-289",
      "path": "spec/functional/FR-289-keep-results-unchanged-by-provider-installation.md",
      "role": "examined",
      "excerpt": "Layer R shall give equal candidate sets for an item from registries built\n  from the same manifests in any order."
    }
  ],
  "findings": [
    {
      "fnd": "FND-001",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "acceptance",
      "artifact_id": "QSL-593",
      "related_ids": [
        "FR-288"
      ],
      "path": "qsl-replay/src/identity.rs",
      "lines": "26-27",
      "excerpt": "pub use qsl_foundation::BackendId as Backend;",
      "finding": "The ticket explicitly forbids compatibility layers, shims and re-exports, but the old replay Backend path remains an alias and qsl-route/src/lib.rs:52 re-exports the former BackendId/EmptyBackendIdentity paths. qsl-foundation/src/backend.rs:33-35 adds identity() specifically to preserve replay vocabulary. Existing replay consumers still import Backend and call identity(). Migrate those consumers to qsl_foundation::BackendId and its canonical accessor; delete the old-path exports and compatibility accessor in this PR. The canonical export from foundation itself is not the defect."
    },
    {
      "fnd": "FND-002",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "acceptance",
      "artifact_id": "FR-288",
      "related_ids": [],
      "path": "qsl-route/src/lib.rs",
      "lines": "160-171",
      "excerpt": "pub fn descriptor_from_manifest<'a>(\n    backend: Candidate,",
      "finding": "descriptor_from_manifest is only a forwarding wrapper around the still-public BackendDescriptor::admit conversion. No existing conversion caller was migrated; repository search finds only the new test calling the new entry point, while routing helpers and origin tests still call admit. The advertised sole entry point can be deleted without affecting any previous conversion caller. Move the label-to-descriptor conversion into the canonical function and migrate the replaced entry-point callers, removing the old conversion API rather than adding a wrapper. The separate already-typed new constructor need not be removed."
    },
    {
      "fnd": "FND-003",
      "severity": "medium",
      "confidence": "high",
      "method": "code-review",
      "check": "trace",
      "artifact_id": "TC-772",
      "related_ids": [
        "FR-288-AC-1",
        "FR-288-AC-5"
      ],
      "path": "qsl-route/tests/it/routing.rs",
      "lines": "52-76",
      "excerpt": "#[trace(\"TC-772\", \"FR-288-AC-1\", \"FR-288-AC-5\")]",
      "finding": "The new test supplies an already-constructed Candidate, digest and capability tuples; it never invokes the driver registry builder, provider manifest reader or plugin hello path. TC-772 explicitly assigns steps 1-4 / FR-288-AC-1..4 to the driver builder (IR-609) and says no qsl-route test covers them. The new AC-1 binding therefore claims a path not exercised. Retain AC-5 for the origin-holding oracle; record AC-1 at the required driver integration or explicitly retain that separate outstanding obligation.",
      "test_id": "descriptor_from_manifest_is_the_shared_conversion_entry_point",
      "claimed_ac": "FR-288-AC-1",
      "correct_ac": "FR-288-AC-5"
    }
  ],
  "bindings": [
    {
      "test_id": "descriptor_from_manifest_is_the_shared_conversion_entry_point",
      "ac_id": "FR-288-AC-1",
      "trace": "wrong"
    },
    {
      "test_id": "descriptor_from_manifest_is_the_shared_conversion_entry_point",
      "ac_id": "FR-288-AC-5",
      "trace": "correct"
    }
  ],
  "qualification": "Frozen diff-scoped static review only, as dispatched. No source edits, builds or aggregate matrix. Manual criterion/tag/source comparison is explicitly scoped to QSL-593 and changed paths. git diff --check passed. Plan completion: not assessed."
}
```

## Methods and limitations

Code review includes the Rust lane. Scoped acceptance comparison used the actual FR/TC text and source tags; no repository-wide assurance claim. No applicable AssuranceProfile was located in the current spec tree. Specification text did not change, so separate spec-review subanalyses were not run. Plan completion: not assessed.


## Re-review round 1

Reviewed: fe84e5a00c096005eea9762b8243002e45351440

FAIL: prior FND-001/002/003 are fixed, but migration misses one root integration import (new high FND-004).

## Dispositions — round 1

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | fe84e5a00c096005eea9762b8243002e45351440 |
| FND-002 | fixed | fe84e5a00c096005eea9762b8243002e45351440 |
| FND-003 | fixed | fe84e5a00c096005eea9762b8243002e45351440 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | high | Root integration test still imports removed qsl_route BackendId and cannot compile. | tests/it/request_builder.rs:17 |

## Round 1 evidence

```yaml
{
  "scope": [
    {
      "id": "qsl-foundation/src/backend.rs",
      "path": "qsl-foundation/src/backend.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! The provider backend identity shared by routing and replay envelopes.\n\nuse std::fmt;\n\n/// A provider's exact backend identity (FR-288, ADR-013 O-19).\n///\n/// The identity is opaque data. It is kept verbatim from the provider\n/// manifest and is the s"
    },
    {
      "id": "qsl-replay/src/identity.rs",
      "path": "qsl-replay/src/identity.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! Provenance and identity carriers the four #231 envelopes need (ADR-013\n//! O-07, O-09, O-11, O-19), scoped to the layer-6 `replay` module.\n//!\n//! Where each type's canonical home is:\n//!\n//! - [`QualifiedName`] (O-11) is #213 S-3's. PR #262 (#214, o"
    },
    {
      "id": "qsl-replay/src/lib.rs",
      "path": "qsl-replay/src/lib.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! `qsl-replay`: the ADR-011 \u00a76.1 layer **6** crate (ADR-011 \u00a77.3\n//! X-10) -- the CG-facing replay facade (ADR-011 \u00a76.1, ADR-013 O-24 to\n//! O-27, C-13). It depends on layers 1 to 5, F and K, as \u00a76.1 allows.\n//!\n//! Its public API is the compile entry "
    },
    {
      "id": "qsl-replay/src/proof_result.rs",
      "path": "qsl-replay/src/proof_result.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! FR-069 (ADR-013 O-24): the typed proof-result envelope. Reads one FR-331\n//! `quire.backend-provider/v1` terminal record into exactly one of the\n//! seven ADR-013 O-16 outcome categories the proof column produces.\n//!\n//! The category is the one `qsl"
    },
    {
      "id": "qsl-replay/src/request.rs",
      "path": "qsl-replay/src/request.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! FR-071 (ADR-013 O-26): the typed replay request.\n//!\n//! The request is exactly the O-25 packet's package reference, selection\n//! and replay-source members plus the FR-070 envelope's state environment,\n//! accounting limits and S1-to-S4 stage limits"
    },
    {
      "id": "qsl-replay/src/witness.rs",
      "path": "qsl-replay/src/witness.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! FR-070 (ADR-013 O-25): the typed counterexample/witness envelope.\n//!\n//! Two nested types, matching ADR-013 O-25's own split:\n//!\n//! - [`Witness`] is the backend witness: exactly one stored field, the\n//!   admitted transcript, with every other wit"
    },
    {
      "id": "qsl-route/src/lib.rs",
      "path": "qsl-route/src/lib.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! `qsl-route`: the ADR-011 \u00a76.1 layer **R** crate (ADR-011 \u00a77.3\n//! X-9) -- the `#185` capability-backend registry and router (ADR-012 \u00a76,\n//! \u00a77). Among the workspace crates it depends only on `qsl-semantics`\n//! (layer 3) and `qsl-foundation` (layer "
    },
    {
      "id": "qsl-route/src/request.rs",
      "path": "qsl-route/src/request.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! ADR-013 O-20, ADR-014 \u00a74 and \u00a76 step 3: QSL's typed request\n//! representation and its writer.\n//!\n//! The writer turns one checked item's [`Requirements`] into one requested\n//! item: its capability kind, its FR-331 extent classification and, for a\n"
    },
    {
      "id": "qsl-route/tests/it/provider_origin.rs",
      "path": "qsl-route/tests/it/provider_origin.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! FR-288-AC-5 and FR-288-AC-6: a descriptor carries the provider's origin,\n//! and two registrations of one identity that differ only in origin\n//! conflict.\n\nuse std::collections::BTreeSet;\n\nuse ix_trace_rs::trace;\nuse qsl_foundation::bound::DomainKin"
    },
    {
      "id": "qsl-route/tests/it/route_registry.rs",
      "path": "qsl-route/tests/it/route_registry.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! FR-075/FR-076/FR-080: the `#185` registry's routing behaviour -- which\n//! backend is chosen for a requested capability kind, and what is reported\n//! when none is.\n//!\n//! Every test here drives [`qsl_route::Registry`] through its public API only:\n/"
    },
    {
      "id": "qsl-route/tests/it/routing.rs",
      "path": "qsl-route/tests/it/routing.rs",
      "role": "examined",
      "excerpt": "// SPDX-License-Identifier: AGPL-3.0-or-later\n//! TC-155 steps 3 to 6 (FR-057-AC-6, FR-057-AC-8): registration admits\n//! advertised labels under the requested-pair rules, candidate sets follow\n//! the FR-290 rule under every registration order, and routing consumes\n//! settled dispositions as data."
    },
    {
      "id": "reviews/qsl-593-code-review.md",
      "path": "reviews/qsl-593-code-review.md",
      "role": "examined",
      "excerpt": "---\nid: SR-4967\ntitle: \"QSL-593 code-review\"\ntype: SpecReview\nanalysis: code-review\nscope: \"agent-ix/quire-spec-language@0e0661060091e24101f2e462fdf765ce278d45b9; qsl-foundation; qsl-route; qsl-replay identity; FR-288; FR-289; TC-772; TC-773\"\nreview_set: subset\n---\n\n## Summary\n\nQSL-593 / PR #729: FA"
    },
    {
      "id": "reviews/qsl-593-gap-analysis.md",
      "path": "reviews/qsl-593-gap-analysis.md",
      "role": "examined",
      "excerpt": "---\nid: SR-4968\ntitle: \"QSL-593 gap-analysis\"\ntype: SpecReview\nanalysis: gap-analysis\nscope: \"agent-ix/quire-spec-language@0e0661060091e24101f2e462fdf765ce278d45b9; qsl-foundation; qsl-route; qsl-replay identity; FR-288; FR-289; TC-772; TC-773\"\nreview_set: subset\n---\n\n## Summary\n\nQSL-593 / PR #729: "
    },
    {
      "id": "unmigrated root integration consumer",
      "path": "tests/it/request_builder.rs",
      "role": "examined",
      "excerpt": "use qsl_route::{\n    BackendDescriptor, BackendId, Candidate, CandidateOutcome, ManifestDigest, Mode,"
    },
    {
      "id": "root integration inclusion",
      "path": "tests/it/main.rs",
      "role": "examined",
      "excerpt": "mod request_builder;"
    }
  ],
  "dispositions": [
    {
      "fnd": "FND-001",
      "outcome": "fixed",
      "fix_sha": "fe84e5a00c096005eea9762b8243002e45351440",
      "after_excerpt": "use qsl_foundation::BackendId;"
    },
    {
      "fnd": "FND-002",
      "outcome": "fixed",
      "fix_sha": "fe84e5a00c096005eea9762b8243002e45351440",
      "after_excerpt": "let mut advertises = HashSet::new();\n    for (kind, mode) in advertised {"
    },
    {
      "fnd": "FND-003",
      "outcome": "fixed",
      "fix_sha": "fe84e5a00c096005eea9762b8243002e45351440",
      "after_excerpt": "#[trace(\"TC-772\", \"FR-288-AC-5\")]"
    }
  ],
  "new_findings": [
    {
      "fnd": "FND-004",
      "severity": "high",
      "confidence": "high",
      "method": "code-review",
      "check": "compilation",
      "artifact_id": "tests/it/request_builder.rs",
      "related_ids": [
        "QSL-593"
      ],
      "path": "tests/it/request_builder.rs",
      "lines": "17-20",
      "excerpt": "use qsl_route::{\n    BackendDescriptor, BackendId, Candidate, CandidateOutcome, ManifestDigest, Mode,",
      "finding": "The canonical-identity migration removed qsl_route public BackendId but leaves this root integration test importing it from qsl_route. qsl-route/src/lib.rs now has only a private use; tests/it/main.rs:111 includes request_builder unconditionally. Compiling the root integration target therefore fails on the private import (E0603). Import BackendId from qsl_foundation here, matching the migrated route tests. Static name-resolution trace; no build run."
    }
  ],
  "bindings": [],
  "qualification": "Delta-only static review at frozen head. No builds, production edits, aggregate matrix or agents. git diff --check passed; worktree clean. Original SR-4967 and SR-4968 artifacts are present under reviews/ byte-identical to initial reviewed artifacts."
}
```
