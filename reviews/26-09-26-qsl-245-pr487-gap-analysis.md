---
id: SR-728
title: "QSL-245 gap analysis of PR 487 (catalog 1-draft.8 adoption)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-001-AC-6, FR-001-AC-11, FR-010-AC-11, FR-018-AC-8, FR-026-AC-6, FR-096-AC-8, FR-096-AC-13, FR-096-AC-14; spec/tests.md; spec/test-cases/TC-424, TC-425, TC-428, TC-430, TC-431, TC-444, TC-500; code quire-exact/src/outcome.rs, qsl-foundation/src/diagnostic.rs, qsl-foundation/src/source.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-001
    type: reviews
---

## Summary

Ticket: QSL-245 (PR agent-ix/quire-spec-language#487). Each new or changed AC
is mapped to its TC and to the test that backs it. The PR changes no code.
Status is recorded as planned for everything not built.

| AC | TC | Test today | Status in PR |
| --- | --- | --- | --- |
| FR-001-AC-6 (cause and `label`) | TC-424 step 2 | code only (`SourceReadCause::UnnamedSource`) | planned; code backed |
| FR-001-AC-11 | TC-424 step 6 | none | planned |
| FR-010-AC-11 (`blank-label`) | TC-425 step 3 | code only | planned; code backed |
| FR-018-AC-8 (`blank-label`) | TC-431 step 2 | code only | planned; code backed |
| FR-026-AC-6 (`blank-label`) | TC-430 step 2 | code only | planned; code backed |
| FR-096-AC-8 (ten value refusals) | TC-428 step 4 | none (`kernel_refusal_record` returns `None`) | planned |
| FR-096-AC-8 (`CardinalityOutOfBound`, `ForeignReference`) | TC-428 step 4 | `qsl-eval/tests/it/model_reference_queries.rs:3556`, `:3601` | reported as passing; see SR-724 FND-001 for `ForeignReference` `cause()` |
| FR-096-AC-13 | TC-428 step 5 | none | planned |
| FR-096-AC-14 | TC-500 | none (`Undefined` has no `SumOutOfDomain`) | planned |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-001 now says a whitespace-only label refuses the replay with `blank-label`, naming that label. TC-444 step 5 still expects only the `invalid_source_identity` code, and TC-444 is reported as passing. No planned status records the new cause and field on the replay path. | spec/functional/FR-001-read-exact-source.md:147-149; spec/test-cases/TC-444-the-replay-executor-recompiles-selects-calls-and-refuses.md:89 |
| FND-002 | low | Outside the diff, confirmed as claimed: TC-318 has no spec artifact and no `tests.md` row. It exists only as a trace tag at quire-exact/src/outcome.rs:294-308. The doc comment there says `CardinalityOutOfBound` is the only refusal with a `code`, which is already false: `IeeeNanPayloadNotRepresentable`, `IeeeRationalOutOfDomain` and `ForeignReference` return one. Building AC-8 makes it more wrong. Retag or rewrite it when AC-8 is built. | quire-exact/src/outcome.rs:294-308 |

## Verdict

Approve. Every new AC maps to a TC with a planned test, and the status
columns do not overclaim, except for the `ForeignReference` `cause()` noted
in SR-724 FND-001.

## Dispositions

Verified against the fix diff on 2026-09-26.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | TC-444 step 5 now expects `blank-label` with `label` `authority`, its Status and `tests.md` row are Partial, and the cause and field are marked planned. |
| FND-002 | deferred | The `quire-exact` code is outside this spec-only PR. The fix records the stale TC-318 tag and comment in FR-096 Status, for the coder who builds AC-8 to retag and correct. |
