---
id: SR-2443
title: "Gap analysis of quire-spec-language PR #663 (QSL-653): from_run and spine::run cancel"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2f0b659461cfc40a8d31e3efa09d32032e4d42e8; PR #663 diff against origin/main bfeb258c; FR-100, FR-286 and their criteria; qsl-replay/src/outcome.rs, qsl-replay/src/spine/call.rs; quire matrix --scope . at the reviewed sha"
review_set: subset
relationships:
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-286
  - type: references
    target: ix://agent-ix/quire-spec-language/FR-100
---
# Gap analysis of quire-spec-language PR #663

## Summary

Ticket: QSL-653. Planless audit of the changed requirements (FR-100, FR-286) against the computed matrix, the tagged tests and the source.

`quire matrix --scope .` (quire 0.36.1) reports every FR-100 criterion (AC-1 to AC-11) and every FR-286 criterion (AC-1 to AC-6) as `tagged`. The new FR-286-AC-6 is bound by `tests::from_run_documents_every_arm` and `tests::a_driver_side_unsupported_engine_document_exits_21`.

Examined:
- FR-286-AC-6 (examined)
- FR-286 statements on `from_run` and on driver failures (examined)
- FR-100 statement on the caller's `&Cancel` (examined)
- FR-100-AC-1 to AC-11 (context_only)
- `OutcomeDocument::from_run`, `RunRefusal::Cancelled`, `spine::run` (examined)

## Verdict

**CONDITIONAL**.

No criterion is untagged, there are no stubs, and every new code path has an owning requirement.

FR-100 now states a cancel behaviour ("a cancel at any stage or during the call returns `RunRefusal::Cancelled`") that no acceptance criterion states. The only test that touches it, FR-286-AC-6, cancels before the run starts.

The semantic review (intent to test to code) was not run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-100's new cancel statement has no criterion. No FR-100 AC says that a cancel during S2 to S4 or during the S6a call gives `RunRefusal::Cancelled` (incomplete, exit 22) rather than a fault. FR-286-AC-6 covers only a handle cancelled before the run starts. The S6a mapping at call.rs:389 is therefore code with a stated requirement but no criterion and no test. If it regressed to `convert_call_failure`'s `cancelled-without-a-shared-handle` fault (exit 30), the matrix would stay green. | spec/functional/FR-100-run-a-named-function-through-the-spine.md:47; qsl-replay/src/spine/call.rs:388-391 |

## Coverage

- FR-100: 11 of 11 criteria tagged.
- FR-286: 6 of 6 criteria tagged, AC-6 new in this PR.
- Reverse gap: `RunRefusal::Cancelled` and `from_run` are owned by FR-100 and FR-286.
- Stubs: none.
- Semantic review: skipped. The run was a reviewer subagent with no opt-in.

Plan completion: not assessed
