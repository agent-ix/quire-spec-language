---
id: SR-1594001
title: "code-review and rust-review of PR #1594 (QSL-580a, FR-230 timed members at S3)"
type: SpecReview
analysis: code-review
date: 2026-10-26
scope: "agent-ix/quire-spec-language; PR #1594 git diff origin/main...origin/qsl-580a-timed-s3 (a02c4c9a1): qsl-semantics/src/check/{timed_model.rs,mod.rs,check.rs,refusal.rs,lowering/model.rs}, qsl-forms/src/{syntax.rs,spans.rs}, qsl-replay/src/spine.rs, qsl-semantics/tests/it/{timed_model.rs,timed_declarations.rs}"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# SR-1594001: code-review and rust-review of PR #1594 (QSL-580a, FR-230 timed members at S3)

## Summary

Ticket: QSL-580. Reviewed the PR diff only, with the rust-review lane folded in. S3 reads clocks out of state clauses into a guard DNF and resets, rewrites each clause's data reading by atom polarity, and types the predicates with the one clause checker. I traced the DNF construction and the polarity rewrite over `and`, `or`, `implies` and `not`. The rewrite is a relaxation, so it is sound: each clock atom becomes the Boolean that makes the data reading an over-approximation, and the convex parts carry the clock part. Coder note on `with_unwritten_timed_members`: the builder is `#[cfg(any(test, feature = "test-support"))]`, and only the root crate's dev-dependency enables `test-support` (guarded by `no_shipped_dependency_enables_test_support`), so it is a test-only seam. The backing field `PackageDeclarations::emit_unwritten_timed_members` is `pub` and ungated, as `emit_unwritten_terminals` and `emit_unwritten_refinements` already are (FND-005). Keeping `AssemblyCause::TimedNotChecked(Clock)` for a QSL-source `Clock` type position is correct: no source clock position is checked yet (FR-230-AC-9 to AC-28), and the refusal is fail-closed. Focused tests at a02c4c9a1: `cargo test -p qsl-semantics --test it -- timed_model timed_declarations`, 46 passed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A domain-package `Clock` field of a model with no `time` member that no clause reads is admitted with no refusal. The model is not timed, so `TimedModelNotEmitted` never fires either, and the clock silently drops out of the checked package. FR-230 requires `clock-without-dense-time`. The variant's own doc comment says this case is located at the model alias, but the code returns no span and skips the refusal. The PR body discloses it. | qsl-semantics/src/check/timed_model.rs:1184-1206; qsl-semantics/src/check/refusal.rs:591-599 |
| FND-002 | low | `Clocks::read` recognises a clock read only as `self.f`. A read through any other path, such as a parameter `o.x` or a quantified `c.x`, is not seen as a clock. It reaches the clause checker, which has no `Clock` field in its type environment, so it refuses `missing-name` instead of `clock-read-outside-constraint` at the read. The refusal fails closed, but with the wrong cause. Traced, not reproduced. | qsl-semantics/src/check/timed_model.rs:522-534; qsl-semantics/src/check/assemble.rs:1134-1139 |
| FND-003 | low | Only top-level postcondition conjuncts are read as resets. A clock write nested under `implies`, `or` or `if`, such as `self.ready implies self.x = 0 ms`, refuses `clock-read-outside-constraint`. FR-230 says any other write to a clock refuses `clock-write`. | qsl-semantics/src/check/timed_model.rs:901-944 |
| FND-004 | low | `emit_unwritten_timed_members` is a `pub` field of the public `PackageDeclarations`. Production code in any crate can set it and bypass the fail-closed `TimedModelNotEmitted` refusal, even though its builder is `test-support`-gated. This is the existing pattern of the terminal and refinement opt-outs. Make the field `pub(crate)`, or gate it like the builder. | qsl-semantics/src/check/check.rs:494-500,546-558 |
| FND-005 | low | Two assertions are weaker than their comments claim. `assert!(graph.state_clause("Reply").is_some())` is said to show that the clause's data reading no longer reads the clock, but it only checks that the clause exists. `part.data().is_some()` stands in for "each part carries `self.ready`" without checking which predicate it carries. Assert the rewritten body and the part's predicate. | qsl-semantics/tests/it/timed_model.rs:226,235,275 |

## Verdict

**Not mergeable** until FND-001 is fixed: a clock field silently dropped from an untimed model is exactly the silent omission the fail-closed refusal exists to prevent. The fix is small (refuse at the model alias, as the cause's doc already says). FND-002 to FND-005 are low and can ride the same fix round. Correct: the guard DNF and its node-limit charge, the polarity rewrite, exact unit conversion, `urgent O` resolution, the refusal codes and tags, the `qsl-forms` cut/negate/conjoin helpers (iterative, no recursion), and the test-only status of the opt-out builder. Rust lane: no `unwrap` or `expect` in production paths, no recursion over expression trees, saturating counts at the limit, and the repo's `#[allow(..., reason)]` convention is followed.

## Coverage

Methods: code-review with the rust-review lane. Gap analysis is SR-1594002.
