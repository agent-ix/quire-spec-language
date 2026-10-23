---
id: SR-16343
title: "spec-review of the I3 adapter is enabled only through qsl-replay quire-extraction; root feature removed (quire-spec-language PR 1313)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0a44fcc8; spec/functional/FR-030-consume-quire-extraction.md; spec/functional/FR-031-run-extracted-native-source.md; spec/functional/FR-100-run-a-named-function-through-the-spine.md; spec/functional/FR-108-run-the-configversion-spine-corpus.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-030
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-108
    type: references
---

# Spec review of the I3 adapter is enabled only through qsl-replay quire-extraction; root feature removed (quire-spec-language PR 1313)

## Summary

Ticket: STD-344. One spec-review pass over `git diff origin/main...0a44fcc8`: standing rule (no drop, narrowing or deferral), testable ACs with hand-derived vectors, id uniqueness across all open PR heads, QSpec name cross-check, mergeability, and the repo gate.

Examined, clean:

- ADR-011 X-4, FR-108-AC-5 and TC-468/TC-469 now agree: the I3 checks run under `qsl-replay/quire-extraction`; the root crate has no feature and no `qsl-source` dependency.
- Ids FR-030-AC-1/2/5, FR-031-AC-1..5, FR-100-AC-3 are on no other open head except edits by #1303 and #1326 to FR-100/TC-450 (textual conflicts).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | Permanent refusal that removes a capability: the native-run/1 `extraction` mode is refused `invalid-request` in every build (FR-100-AC-3, FR-031-AC-5), and the extraction report member plus the catalogued codes `extraction-package-conflict`, `extraction-clause-count`, `extraction-requires-run` and `invalid-quire-context` are deleted from FR-031; the PR body says the first two "hold by construction" and says nothing for the last two. The PR calls this a lead default, not an owner answer. Standing rule: blocker; Peter must rule or the capability stays specified. | spec/functional/FR-031-run-extracted-native-source.md | missing-requirement |
| FND-002 | high | Deferral: the mapped compiler's original-region correspondence for a refused extracted body (FR-011/FR-022) is dropped from the spine text and filed as STD-394 ("New ticket for the mapped compiler half"). FR-030/FR-031 no longer say a compile refusal keeps the original document regions, so deleting `mapped` removes a stated behaviour with no replacement in this PR. It is inside the ticket's scope (FR-031 is rewritten here), so it must be specified here, not split out. | spec/functional/FR-030-consume-quire-extraction.md | missing-requirement |

## Verdict

Not mergeable: FND-001 and FND-002 (high, standing rule). Merge-tree against main is clean; open #1303 conflicts in 8 files and #1326 in FR-100/TC-450.
