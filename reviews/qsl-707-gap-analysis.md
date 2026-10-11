---
id: SR-7813
title: "Gap analysis — QSL-707 PR scope"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@bbfc7c9b13d2253194aa97574e36513072fdeb0d; Cargo.lock, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-spec-language/FR-095", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-087", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-096", type: references }
  - { target: "ix://agent-ix/quire-spec-language/FR-320", type: references }
---

## Summary

Manual PR-scope acceptance-criteria-to-test and reverse-gap review of QSL-707 PR #738 at `bbfc7c9b13d2253194aa97574e36513072fdeb0d`. Seven existing bindings for reader, source-map, import-view, and limit behavior were checked; the changed union-key classifier has no test invocation, and FR-320-AC-4 has no trace-tagged test.

## Verdict

**FAIL** — FR-320-AC-4 is directly implicated by the changed union-key test helper, but has no trace-tagged test, and the new aggregate `union_value` classifier branch is not exercised.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Aggregate-bodied `union_value` key classification is untested; FR-320-AC-4 has no trace-tagged test | qsl-package/src/emit/tests.rs:455 |

## Coverage

- Reconciliation: manual PR-scope AC-to-tests fallback. `quoin matrix --repo /home/peter/dev/worktrees/qsl-707-nullary-union --json` and `quire matrix --scope /home/peter/dev/worktrees/qsl-707-nullary-union --format json` produced module warnings but no matrix result before interruption; no whole-repository matrix counts are claimed.
- Plan completion: not assessed
- Examined: FR-095-AC-3 (TC-421), FR-087-AC-3 (TC-253), FR-087-AC-4 (TC-254), FR-096-AC-10 (TC-429), FR-255-AC-1 (TC-720), FR-255-AC-4 (TC-721), FR-264-AC-4 (TC-739), and FR-320-AC-4 (no binder found).
- Manual binding check: TC-421 looks up each wire source-map occurrence and compares regions and provenance, then compares occurrence counts; TC-253 exercises whole I2 fixture reads; TC-254 verifies import-view export keys; TC-429/TC-720/TC-721/TC-739 assert limit outcomes, actual counts, loci, and raise-and-reread behavior. A repository-wide search finds no `FR-320-AC-4` trace tag and no test constructing a wire node with `node_tag: value` plus `semantic_form: union_value`; the new branch in `rebuilt_preimage` therefore has no exercising test.
- Reverse-gap review of changed files: only test fixture helpers and one dependency lock entry changed; no new production behavior was introduced. The changed classifier is test logic whose union-specific path lacks evidence. Full source-tree inventory was unavailable with the stalled matrix run.
- Semantic review: skipped; not requested.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e2341608b249e3bbdb138dedab9ce570219957dd |

`after_excerpt` (new test evidence for FR-320-AC-4; verbatim):

```rust
        assert_eq!(
            crate::emit::tests::rebuilt_key(node, &[]),
            expected_key,
            "{member}: aggregate-bodied union value uses its application key"
        );
```

The same commit corrects inline frame fixtures to point `semantic_type` at the keyable type node's derived `node_id`; the negative frame case now uses that valid type identity too. Static review found the fixture correction consistent with the frame test intent. No builds or tests were run.
