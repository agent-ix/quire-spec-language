---
id: SR-1465002
title: "Code and Rust review of PR #2102: QSL-1465 AC-12 assertion"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
date: '2026-11-02'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-375
    type: reviews
---

## Summary

Ticket: QSL-1465. Reviewed the delta origin/peter/qsl-1465-analyze-boxing...origin/peter/qsl-1465-debug-and-ac12 (base is PR 2101's branch). Checked against QSpec FR-375-AC-12 in /Users/peter/dev/quire-specification. Cargo not run.

## Verdict

**PASS (mergeable)** - no findings. AC-12 requires refinement_internals to be the only host of abstract_internal and no entry for absent, [] or another refinement's value; it says nothing of node_id, and node_id is the structural NodeKey since #1885 (29f2a877a). The new assertion fails if abstract_internal is written for internal: [] on R (get returns Some). Hosting elsewhere is still caught by the unchanged listed-body and R2 assertions. The FR-375-AC-12, FR-147-AC-9, TC-310, TC-552 trace tag stays correct. PR body cites a spec path that lives in QSpec, not this repo (cosmetic).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |
