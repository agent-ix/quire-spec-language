---
id: SR-11176
title: "QSL-502 quire-spec-language PR 857 code review: union registration and case checking"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@9290f614ae36ee7c4986a8024dd3917294443667; qsl-semantics/src/check/{assemble,sum_case,check/typing,check,checked_dispatch,claims,ir,lowering,refusal,mod}.rs, qsl-semantics/src/family/requirements.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-replay/src/{spine,execute/composite_domain,result/wire/value}.rs, qsl-forms/src/syntax.rs, test files"
review_set: subset
---

## Summary

Ticket: QSL-502 (first sub-slice of S3). Code review with the rust-review
lane, scoped to `git diff origin/main...HEAD` (5 commits). Read in place. No
build was run (brief: no builds). The coder's `make ci` log was not re-run.

Checked:

- Exhaustiveness against QSpec FR-146. `CaseBuilder::finish` runs
  duplicate-arm, unknown-member, arm-arity, then missing-arm. Each reads only
  the recorded heads. Duplicate counts resolved and arity heads, so
  `Empty` and `Shape::Empty` collide. A qualifier other than the union's
  name is unknown-member. An unknown or arity arm's body is never typed.
  Every arm body is typed before `finish`, so a body refusal wins over
  exhaustiveness. A non-union scrutinee (Integer, enum, variant) refuses
  `ill_typed`/`type-mismatch` at the scrutinee before any head is read.
  QSpec TC-264 E01-E06 give TC-264's verdicts by inspection.
- Result type. The expected path checks every body against the expected
  type and gives the case that type. `case_result` matches FR-146's inferred
  rule: equal types are kept, mixed Integer/Int[..] becomes Integer, and the
  first body that is neither is refused at that body. Folding over the
  running result is equivalent to comparing with the first body, because
  the integer group is closed.
- Cycle reporting. `admit_types_with` now refuses every declaration named
  on a recursion cycle, once each. Other declarations that reference a
  set-aside name are still suppressed as consequences. No existing test or
  spec vector asserts a single refusal for a multi-declaration record or
  tuple cycle. The only QSL recursion vectors (qsl-eval r06) are
  single-declaration cycles. This is the expected change for the shared
  containment graph that FR-316 requires.
- Eval and replay internal-fault arms. `PackageDeclarations::check` lowers
  every composite in the type environment (`mod.rs:1633`), and
  `open_composite` refuses a union. So no `CheckedGraph`, and no
  `CheckedPackage`, has a union in scope. `check_clause_expression` types
  against that scope, so a `Case` node cannot reach `CheckedPackage::evaluate`
  or replay. The arms are unreachable.
- Deleted `UnsupportedUnion`. `git grep` at HEAD finds it only in the
  committed historic review file. No code, CLI or spec reference remains.
- PreserveBind fix. Arm binders are bound through `Rewrite::Bind` with the
  flat `ExprNode::binders` numbering, which `spellings_mut` lists in the
  same order. The existing test still asserts that binders without a
  capture keep their spelling. The new test covers the capture case.
- Tests. The changed expectations (qsl-forms respell, variant_fields
  scrutinee refusal, replaced assembly union test) are the intended new
  behaviour. No assertion was weakened.
- Panics. No `unwrap`, `expect`, `unreachable!` or indexing in new non-test
  code. Broken invariants return typed `InternalFault`s.

## Verdict

FAIL for this method: one blocking finding (FND-001, wrong catalog code
against FR-316-AC-3). Mediums are recorded in gap-analysis SR-11177 and
spec-review SR-11178.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `union U { A, A(Integer) }` refuses `ambiguous_declaration`/`ambiguous-name`, but FR-316-AC-3 (and FR-316 Behavior) require `invalid_semantic_graph` from `DeclarationCause::DuplicateMember`. The test asserts only the typed cause and span, so it passes while the AC fails. The test is tagged FR-316-AC-3, and the PR body does not record the gap. Fix the code mapping, or amend FR-316 to the existing mapping and record it. | qsl-semantics/src/check/assemble.rs:401; qsl-semantics/src/check/assemble/tests.rs:2420-2433 |

## Dispositions

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | cdf6e5d02 (PR #858, on main) amends FR-316-AC-3, FR-316 Behavior, ADR-012 and TC-818 to `ambiguous_declaration`/`ambiguous-name`. 0d98c5aae makes the test assert the code and catalog pair beside the typed DuplicateMember cause and span. Round 1, reviewed at 0d98c5aae. |
