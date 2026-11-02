---
id: SR-202611020542
title: "Gap analysis of QSL-1465 PR #2111 trace and implementation scope"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2739f994b; repository matrix via quoin; reverse-gap and stub inspection of qsl-foundation/src/source/node_table.rs, qsl-package/src/emit/tests/admission_corpus.rs, qsl-analyze/tests/it/schedulability_routing.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-375
    type: references
---

## Summary

Planless gap analysis for QSL-1465 PR #2111. `quoin matrix --repo <repository> --json` completed. The result exceeded the command-output display budget; among the returned rows, FR-001-AC-7 was explicitly `untagged`, and multiple criteria (including FR-001-AC-1) had `no run evidence`. No exact repository-wide count is claimed from the truncated output. The changed admission-corpus classifications are backed by FR-093-AC-19 / TC-416, and the QSL-1451 debug regressions are retained and pass. No TODO/FIXME/unimplemented stubs or new unowned behavior were found in the three changed source/test files. Optional semantic review was skipped. Plan completion: not assessed.

## Superseded

The thread-local node table this review describes was replaced by explicit table threading in PR #2112, because the thread-local violates FR-284-AC-4 (the qualified-core ambient-input scan flags it at qsl-foundation/src/source/node_table.rs). The findings and evidence below remain valid for PR #2111 as reviewed.

## Verdict

**FAIL** — the computed matrix contains untagged criteria and no bound run evidence for the sampled criteria. These repository-wide trace/evidence gaps predate this PR; the PR does not add or widen them.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-001-AC-7 is untagged in the computed matrix. | spec/functional/FR-001-source-identity-and-spans.md; quoin matrix |
| FND-002 | low | The sampled criteria have no bound run evidence in Quoin (`FR-001-AC-1` is one example). | quoin matrix |

## Coverage

- Matrix verification: completed with `quoin matrix --repo /home/peter/dev/worktrees/qsl-1465-verify --json`; output was truncated by the tool display limit, so the total untagged/evidence counts are unavailable.
- Reverse-gap and stub inspection: the three changed files only; no new unowned behavior or stubs found.
- Semantic review: skipped (not requested).
- Plan completion: not assessed.
