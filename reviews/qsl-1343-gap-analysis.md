---
id: SR-1928002
title: "gap analysis of PR #1928 (QSL-1343): linear FR-096 anchor walk vs FR-096, FR-092"
type: SpecReview
analysis: gap-analysis
date: 2026-10-31
scope: "agent-ix/quire-spec-language; PR #1928 git diff origin/main...f6447257"
review_set: subset
---
# SR-1928002: gap analysis of PR #1928 (QSL-1343)

## Summary

Ticket: QSL-1343. Planless; plan completion: not assessed. Scope is the PR diff and the requirements it touches: FR-096 (generated-occurrence anchors) and FR-092 (type build of declared composites).

Examined:

- FR-096 anchor results: the behaviour is unchanged (see SR-1928001), and the existing FR-096 region tests in qsl-semantics/src/check/region.rs (TC-426, TC-427) still bind it. No new code path needs a new test.
- FR-092 recursion-group detection through `composites_open`: covered by the existing lowering tests (84 pass) and by the new 65,536-record chain test.
- New test `a_long_admitted_record_chain_compiles` is tagged FR-092-AC-7. That AC's text names 100,000 nested `Option`s and a 30-record chain on a small stack, not a long admitted record chain, so the tag is loose. This is a minor trace imprecision, not a major gap, and is left out of the findings under the major-bugs-only brief.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - | correct-requirement-no-evidence |

## Verdict

No code gap.
