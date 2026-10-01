---
id: SR-903
title: "QSL-337 code review of PR 540 (call_site widened to domain packages, dependencies and operations)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@a44e9c9a5ebae435347134a03ddc9d6da1631f1e; qsl-replay/src/call_site.rs; qsl-replay/src/lib.rs; qsl-replay/src/execute/tests.rs; qsl-replay/src/spine/clause/tests.rs; qsl-replay/src/spine/clause/tests/call_site.rs; qsl-semantics/src/check/mod.rs (read, unchanged); qsl-semantics/src/check/state_clause.rs (read, unchanged); qsl-replay/src/spine.rs (read, unchanged); qsl-replay/src/witness/frame.rs (read, unchanged)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-337. PR: quire-spec-language#540 at a44e9c9a, diff
`origin/main...HEAD` (merge base c7fd631d). This file covers code review
with the rust-review lane folded in.

The PR makes `call_site` generic over a sealed `CallSiteSelection`
(`QualifiedName` -> `FunctionSite`, `OperationName` -> `OperationSite`),
adds domain-package and dependency inputs, adds
`CallSiteRefusal::UnknownOperation`, and re-exports `DependencyInput`,
`DependencyInputRefusal`, `OperationName`, `SourceHolder`, `SuppliedLibrary`,
`Origin` and `Role`.

What was checked:

- **Architecture boundary.** `arch-lint api-surface --cg
  /home/peter/dev/quire-contract-codegen` passes T12-A..E and FR-100-AC-8
  (exit 0, run by the reviewer). No `spine` type reaches `call_site`'s
  signature: the re-exported spine types are named at the crate root.
  `Origin`/`Role` are not T-12-governed constructors; `Origin` is the
  minimal addition that makes `OccurrenceKey::new(WireNodeId, Origin)`
  callable, and `Role` is `Origin::new`'s argument. Main had no `Origin`
  re-export, so the FR-116 witness types were not buildable by CG before.
- **Sealed trait versus enum.** The trait is correctly sealed (`Locate` lives
  in a private module). The associated `Site` type gives each selection a
  statically typed answer, where an enum would force CG to match a runtime
  variant it already knows. It adds no `QualifiedName`-only refusal
  (TC-258): both unknown arms pair the selection with the `package_id`.
- **Operation lookup.** `resolve_operation` then `operation_frame` is the
  same resolution FR-115's `Frame` selection uses, so every
  `UnknownOperation` arm (alias, type, operation, unnamed operation) goes
  through one `ok_or_else`. The clause filter compares `ClauseOperation`
  (declaring type and declaration), the same identity `operation_frame`
  matches frames by. Excluding invariants is correct for an operation
  selection: `CheckedStateClause::operation()` is `None` for them.
- **No anchor occurrence key.** `FrameCounterexample` carries `anchor:
  WireNodeId`, `frame` and the frame's `occurrence` only
  (`witness/frame.rs:136-151`), so no anchor occurrence is needed.
- **No clause kind.** FR-122 (PR #539, in review) names the clause by its
  declared name and picks the observation arm by kind; for a pre or post
  clause both use `Invocation`, so `ClauseSite` without a kind does not block
  the postcondition slice. Not a finding.
- **Test oracle.** The AC-3 test reads the expected anchor, frame and clause
  node ids from `semantic_graph().nodes()` by `NodeTag::State` and
  `semantic_form()`, not from `CheckedGraph::operation_frame` or
  `state_clauses()`, which `call_site` uses. The expected occurrences are
  built from constants (`generated`/`claim`, ordinal 0) through the facade's
  own `OccurrenceKey`/`Origin`/`Role`. That is independent, not
  tautological. AC-1 and AC-4 are proved by driving the real `replay`.
- **Mutation.** Replacing the filter at `call_site.rs:257` with
  `clause.operation().is_some()` leaves all five TC-516 tests green
  (`mutant-m1.log`). See FND-001.
- **Rust idioms.** No panics, no `unwrap` in shipped code, no `unsafe`,
  errors boxed as before, `Fault` on a broken identifier invariant. The
  breaking move of `parameters` into `site.parameters` has no shim, which is
  right: CG has nothing merged against it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The clause filter's operation identity is never exercised. The fixture has one operation and one clause, so a mutant that keeps every `pre`/`post` clause regardless of operation (`clause.operation().is_some()`) passes every test. Declaration order and `pre` clauses are also untested. A unit with a second operation (the `config_version_domain_document_with_operations` builder already supports one) and a `pre` on `attemptUpdate` plus a `post` on the other operation would catch it: CG would otherwise build a replay request for a clause of the wrong operation. | qsl-replay/src/call_site.rs:257; qsl-replay/src/spine/clause/tests/call_site.rs:55-95 |
| FND-002 | low | The new comment "The inputs `call_site` takes and the typed operation name it and FR-115 select by ..." sits above `pub use execute::{...}`, but describes the `pub use spine::{...}` block 17 lines further down. | qsl-replay/src/lib.rs:45-48,62-64 |
| FND-003 | low | `CallSiteRefusal::Compile(String)` now also carries caller-input refusals: dependency input, model selection and import. CG can tell "you supplied no library" from "the unit does not compile" only by matching a substring, which the PR's own AC-4 test does (`message.contains("missing_import")`). A catalog `code`/`cause` beside the message would type it without exposing `spine`. | qsl-replay/src/call_site.rs:117-123; qsl-replay/src/execute/tests.rs:1110 |

## Verdict

No correctness bug found. The API is the minimal honest surface for CG's
IR-412 slice, arch-lint passes, and the tests use an independent oracle.
FND-001 should be fixed in this PR (a small test addition). FND-002 and
FND-003 are low.

## Dispositions

Round 1, reviewed at 5bac0589a6fa5754e5eaf0033385b5c308d94651 (rebased onto bad4944c).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 5bac0589: `call_site_returns_only_the_selected_operations_clauses_in_declaration_order` (two-operation unit, pre then post on `attemptUpdate`, post on `probe`); the `clause.operation().is_some()` mutant now fails it at tests/call_site.rs:220 (reviewer's mutant-m1.log, exit 101) |
| FND-002 | fixed | 5bac0589: the comment now sits directly above `pub use spine::{...}` (lib.rs:58-64) |
| FND-003 | fixed | 5bac0589: `CallSiteRefusal` gains `ModelIntake{alias,message}`, `DependencyInput(DependencyInputRefusal)`, `Import(String)`, `Dependency{path,message}` via an exhaustive `From<CompileRefusal>`; AC-4 now matches `Import(_)` rather than a substring |
