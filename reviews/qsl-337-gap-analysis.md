---
id: SR-904
title: "QSL-337 gap analysis of PR 540 (FR-121 AC-1 to AC-5, TC-516, ticket scope)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@a44e9c9a5ebae435347134a03ddc9d6da1631f1e; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; spec/test-cases/TC-516-locate-a-function-call-site.md; qsl-replay/src/call_site.rs; qsl-replay/src/lib.rs; qsl-replay/src/execute/tests.rs; qsl-replay/src/spine/clause/tests/call_site.rs; ticket QSL-337 body and its 2026-09-30 scope comment; FR-122 at PR #539 head 15d200d6 (read, context)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-516
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
---
## Summary

Ticket: QSL-337. PR: quire-spec-language#540 at a44e9c9a. No plan bundle;
the acceptance text is the ticket body, its scope comment, and FR-121's ACs.

| Item | Delivered | Evidence |
| --- | --- | --- |
| Domain-package input | Yes | `call_site(.., packages, ..)` via `package_input`; AC-3 test |
| Operation selection by typed `OperationName` | Yes | `impl Locate for OperationName`; AC-3, AC-5 tests |
| package_id, anchor, frame, clause node ids and occurrence keys | Yes | `OperationSite`, `ClauseSite` |
| Function selection unchanged | Yes, moved to `site.parameters` | AC-1, AC-2 tests |
| Missing operation refuses `missing_declaration/missing-name` with package (FR-088-AC-6) | Yes | `UnknownOperation{selection, package}`; AC-5 test |
| Comment item 1: re-export `Origin` | Yes, with `Role` | `lib.rs:88`; AC-3 test builds keys through the facade |
| Comment item 2: dependency input | Yes | AC-4 test replays against `test/units` |

Trace bindings: every TC-516 test carries `#[trace("TC-516", "FR-121-AC-n")]`
for the AC it asserts; all five are correct. All new code traces to FR-121:
the sealed trait, both site types, `UnknownOperation`, the `Fault` arm for a
clause name, and the re-exports. The reviewer ran the five tests
(exit 0) and the coder's `make ci` log at a44e9c9 ends `exit=0`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-121 Behavior says an operation "that no clause or attempt of the unit names (FR-105 emits no frame for it) SHALL refuse `CallSiteRefusal::UnknownOperation`". No AC states it and no test drives it. AC-5's `Config::ConfigVersion::nope` fails at the type-view lookup, not at the frame arm. An operation declared in the domain package but named by no clause is the likeliest CG mistake. `config_version_domain_document_with_operations` can add one. | spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md:123-127; qsl-replay/src/spine/clause/tests/call_site.rs:98-123 |
| FND-002 | medium | Through the facade there is no way to get an invariant clause's `state_clause` node or `claim` occurrence. The operation selection is right to exclude invariants. But FR-122 (PR #539, in review) AC-1 needs a `ParentOrder` envelope carrying `clause_node` and `occurrence_key`, so CG would fall back to `spine`, which FB-05 forbids. The sealed trait makes a clause-name selection an additive fix. Acceptable to defer to QSL-336. | qsl-replay/src/call_site.rs:250-261 |

## Verdict

Ticket and scope-comment items are all delivered and tested. FND-001 is a
spec statement with no AC or test and should be closed in this PR. FND-002
is a gap for the next consumer, FR-122; it can be deferred to QSL-336 with a
note.
