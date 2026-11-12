---
id: SR-2026111204015000000003
title: "spec review of PR 2684 FR-519-AC-20 and TC-1314 row 20"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; spec/functional/FR-519-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md, spec/test-cases/TC-1314-publish-the-protocol-read-surface-the-quire-protocol-link-layer-imports.md"
review_set: subset
---

## Summary

Ticket: QSL-1890. Spec review of the FR-519 and TC-1314 edits in PR 2684 at 7020b291b. Covers integrity (numbering, scope line, trace tag, view-function list), consistency with QSpec FR-172 "Choice visibility carrier" / FR-172-AC-6 and the FR-466 `choice` layout, and testability of FR-519-AC-20. `quire validate` passes on both files.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS, clean. FR-519-AC-20 follows AC-19 and TC-1314 row 20 maps to it. The scope line now reads AC-1 to AC-20 and the trace-tag line ends at AC-20. The prose and the view-function list both add `choice_visibility`. The AC wording "visible `expression` nodes" is correct and should not be changed to `query`: QSpec spells a node as `node_tag`/`semantic_form`, and a field read is `expression`/`query` (QSpec FR-341 names the `expression`/`query` node). So `expression` is the tag and `query` is the form the package emits; the two do not conflict. Describing `visible` as a set of references matches FR-466 ("a set of the `expression` nodes"), and "in the order the node carries them" is accurate for the wire's set. The AC packs several refusals into one row, which is the established style of AC-18 and AC-19. The refusal-list gap for non-control nodes is recorded once, in code-review SR-2026111204015000000001 FND-001.
