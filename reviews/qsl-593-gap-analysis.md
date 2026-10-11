---
id: SR-4968
title: "QSL-593 gap-analysis"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@0e0661060091e24101f2e462fdf765ce278d45b9; qsl-foundation; qsl-route; qsl-replay identity; FR-288; FR-289; TC-772; TC-773"
review_set: subset
---

## Summary

QSL-593 / PR #729: CONDITIONAL. Shared storage is centralized and linked/process origin values are preserved, but acceptance is incomplete. Existing provider-order tests remain intact. Reviewed frozen head against origin/main; no execution claim.

## Verdict

CONDITIONAL — scoped static review; findings require disposition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | QSL-593 requires delivered tests backing FR-288 and FR-289, but this PR adds only an origin comparison | qsl-route/tests/it/routing.rs:52 |

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
      "severity": "medium",
      "confidence": "high",
      "method": "gap-analysis",
      "check": "coverage",
      "artifact_id": "FR-289",
      "related_ids": [
        "TC-773"
      ],
      "path": "qsl-route/tests/it/routing.rs",
      "lines": "52-76",
      "excerpt": "#[trace(\"TC-772\", \"FR-288-AC-1\", \"FR-288-AC-5\")]",
      "finding": "QSL-593 requires delivered tests backing FR-288 and FR-289, but this PR adds only an origin comparison. A scoped source tag search finds no FR-289-AC-* or TC-773 binding. Existing reverse/permuted registration tests cover order invariance and remain intact; they do not establish TC-773 front-end output invariance, property-generated capability locality, or terminal proof/refutation construction constraints. Supply those scoped acceptance tests or obtain an explicit ticket/spec scope allocation for the undelivered criteria; do not claim FR-289 complete from the origin test."
    }
  ],
  "bindings": [],
  "qualification": "Frozen diff-scoped static review only, as dispatched. No source edits, builds or aggregate matrix. Manual criterion/tag/source comparison is explicitly scoped to QSL-593 and changed paths. git diff --check passed. Plan completion: not assessed."
}
```

## Methods and limitations

Code review includes the Rust lane. Scoped acceptance comparison used the actual FR/TC text and source tags; no repository-wide assurance claim. No applicable AssuranceProfile was located in the current spec tree. Specification text did not change, so separate spec-review subanalyses were not run. Plan completion: not assessed.
