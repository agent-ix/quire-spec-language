---
id: SR-12913
title: "Base review of STD-189 (PR #1002): FR-381 control-tree commit dominance, negated-comparison fixtures, TC-1058 rows 19a-20b"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@2e8dd2460; spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md Compensation templates commit bullet and AC-11; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md Choice and repeat decisions; spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md rows 19-20b; spec/test-cases/TC-1056-s3-checks-channels-relationships-requirements-and-choice-visibility.md row 45; agent-ix/quire-specification@ad2b37e proposals/quire-v1/shared-grammar.md, spec/functional/protocol/FR-171, FR-426"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-381
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1058
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1056
    type: reviews
---
# SR-12913: base review of STD-189 (PR #1002)

## Summary

Ticket: STD-189. Scope is `git diff origin/main...HEAD` at 2e8dd2460: the FR-381 commit bullet and AC-11, the FR-218 partition example, TC-1056 row 45 and TC-1058 rows 19a to 20b.

The control-tree rule in FR-381 is coherent. A `sequence` orders a passing child before a later child that contains the effect. Passing is defined by recursion over the commit itself, a `sequence`, and a `parallel` whose join requires the branch. The join-requires-branch classification is folded into that recursion. A `choice` and a `repeat` never pass. Cross-branch causal edges are explicitly excluded. Every TC-1058 row from 19a to 20a follows from that text: rows 19a and 19c refuse; rows 19b, 19d, 19e, 20, 20a and 20b check. All join forms in them parse under shared-grammar `join-policy`. Row 20b's expected result, admitted, is the conservative one, because a refusal must be a certain dominance and the control tree under-approximates it. `not (got.count > 0)` is fixed in FR-218 and TC-1056 row 45. A scan of every inline code span in `spec/` finds no other `not a <op> b` fixture; FR-165's `not c.n = 2` is the deliberate grammar example. The diff adds only row 20b and allocates no new ids, so there are no collisions. All four files pass `quire validate`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Row 20b's fixture does not resolve as written. A `branch` holds one control (grammar `branch = 'branch', ident, control`), so `commit C; send Tx` must sit in a named `sequence`, and that name is a path segment (row 19e: `Main::Again::Body::Committed`). FR-113 resolves each later segment among names the previous target declares directly, so the template's `commit Main::P::s::C` and the `receive Rx of s::Tx` anchor both refuse `missing_declaration`/`missing-name` instead of checking. The row also never says the template's `for` names `E` (the base template is `for Main::Applied`), nor that the parallel is `P`. Name the branch sequences (for example `sequence S` and `sequence R`), write `Main::P::s::S::C` and `s::S::Tx`, and state `compensate Refund for Main::P::r::R::E`. Nit: the row sits between 20 and 20a; place it after 20a. | spec/test-cases/TC-1058-s3-checks-compensation-registration-retry-commit-and-recovery.md:62 |
| FND-002 | low | With row 19d's predicate half dropped, FR-381's normative clause that every `join predicate` requires no branch has no AC and no TC vector. Dropping it is right for now: QSpec admits a predicate join (shared-grammar `join-policy`, FR-171, FR-426) and QSL FR-218 states no refusal of it, but the implementation refuses it `unsupported_construct`/`not-yet-implemented` (main: `qsl-semantics/src/check/protocol_controls.rs` module doc and its TC-1065 test; #989 branch: `protocol_clause/coverage.rs` classifies `K::JoinPredicate` as not covered), so a vector expecting `checks` could not pass, and stating that refusal as the expected result would write an implementation gap into the spec. Record the gap: open a follow-up ticket to restore the `join predicate(Ready) [a, b]` checks vector in AC-11 and TC-1058 when predicate joins are checked, and name it in the PR or ticket. | spec/functional/FR-381-check-protocol-captures-activation-and-compensation-recovery-at-s3.md:236-239 |

## Verdict

Not mergeable yet: fix FND-001 first. FND-002 can be deferred to a named follow-up. Everything else in scope is correct. That covers the control-tree rule and its join classification, AC-11, rows 19a to 20a, 20b's conservative admitted expectation, and both negated-comparison fixes.
