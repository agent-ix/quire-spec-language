---
id: SR-1371
title: "Gap analysis of quire-spec-language PR #658: ticket deletes and the tests backing them (QSL-473)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@384d6898ec47da42782f8d7fa0a6b2158c95c053; PR #658 diff against origin/main, checked against QSL-473's delete list and acceptance"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-116
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-122
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-357
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-358
    type: reviews
---
# Gap analysis of quire-spec-language PR #658

## Summary

Ticket: QSL-473. Planless gap analysis over the ticket's "Deletes in the same PR" list and acceptance, measured at the head, not taken from the ticket or PR text. Plan completion: not assessed.

Delete list, measured:
- `revision-mismatch` everywhere: no `*.rs` hit at the head. Library/bundle `RevisionMismatch` and the SV definition revision have no hit on main either.
- Sampler version check: none at the head; `sample_request` checks the sampler identity only (qsl-eval/src/simulation/sample.rs:280).
- FR-079/FR-058 tests: `tests/it/lowering_registry_isolation.rs` (FR-079, TC-204) deleted; no `FR-079`/`TC-204`/`FR-058` trace tag remains. The `FR-058-preserve-retry-and-partial-recovery.md` string in src/linking/composed/definition_source.rs:273 and tests/it/composed_definition_source.rs:206 is a QSpec protocol rule key that still exists upstream, not the deleted QSL FR-058, so it correctly stays.
- "catalog revision 1-draft.8" comments: none in code. The `"1-draft.8"` literal at src/linking/composed/definition_source.rs:332 is the Diagnostics registry entry's revision string, data rather than a comment, and outside the ticket's list.
- `SHA256SUMS`: both committed files, the writer, both constants and their tests are deleted. One stale reference remains in the arch-lint exemption list (SR-1370 FND-001).
- No compat layer, shim, alias or fallback.

Tests backing the changed behaviour:
- FR-357-AC-11/16/17 (TC-904): qsl-replay/src/execute/tests.rs asserts `cause() == Some("content-mismatch")` and the Display text.
- FR-116-AC-3/AC-6 (TC-515): frame_replay.rs asserts `stale_dependency/content-mismatch` for a stale frame, anchor, occurrence, clause node and occurrence key.
- FR-122-AC-3 (TC-517): state_clause_replay.rs `assert_content_mismatch`.
- FR-358-AC-3 (TC-907): uses the same `ScalarIdentity` cause path as FR-357.
- TC-121/TC-138 handoff tests: each selected and manifest path is checked as a published file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Every item in the ticket's delete list is gone from code, and the tests for the renamed cause assert it. Remaining defects are in SR-1370 (stale arch-lint exemption, false coverage claim, dead error variant) and SR-1372 (TC-138, FR-050-AC-7, FR-042 Status). Test run at 384d6898e: `cargo test --locked --test it -- compiled_protocol_v2::` passes, 16 passed, 0 failed. The `handoff_writer` module is behind the `handoff-writer` feature and was not run here.
