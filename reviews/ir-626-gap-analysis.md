---
id: SR-1326
title: "Gap analysis of quire-spec-language PR #643: FCD and quire-rs bump"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8852006dc4363c875a3d64b267d615116e02fb6b; PR #643 diff against origin/main; FR-056-AC-1 (TC-145), FR-030-AC-3 (TC-108), FR-031-AC-2 (TC-109)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: reviews
---
# Gap analysis of quire-spec-language PR #643

## Summary

Ticket: IR-626 (QSL side). A dependency bump with no new requirement and no
spec change. The gap question is whether the tests touched by the bump still
back their tagged criteria.

- TC-145 (FR-056-AC-1): `lifts_the_architecture_bundle_and_admits_it` still
  drives FCD's real `lift` and now checks the whole document against FCD's
  committed expected output, then admits it. Still backs the AC's intake path.
- TC-108 (FR-030-AC-3): the foreign-context loop still mutates each field and
  checks its typed preflight cause, `SemanticCore` included.
- TC-109 (FR-031-AC-2): the extraction-output test still checks the
  `invalid-quire-context` details shape.

No production code without an owning requirement was added. The removed
`provenance` sidecar was a `None` field, so no behaviour was dropped.

## Verdict

No gaps. The version-literal issue is recorded once, in SR-1325 FND-001.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
