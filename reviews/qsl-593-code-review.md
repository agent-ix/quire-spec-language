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
