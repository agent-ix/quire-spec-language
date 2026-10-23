---
id: SR-16209
title: "spec-review of FR-172 Body expressions and FR-172-AC-6 (QSL PR 1301)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@79242bc7; spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md; spec/test-cases/TC-597-s3-checks-hyper-and-relation-clauses.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-597
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-113
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-171
    type: references
---

# Spec review of FR-172 Body expressions (QSL PR 1301)

## Summary

Ticket: STD-342 (spec gap for QSL-867). One spec-review pass over
`git diff origin/main...79242bc7`. Oracles: `qsl-semantics/src/check/refusal.rs`,
`check/check.rs`, `check/check/typing.rs`, `check/hyper_typing.rs` and
`check/hyper_clause.rs` at the head; reviews/26-10-18-qsl-550-code-review.md
FND-008 and FND-009 (the divergences QSL-867 records).

Gates: `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation`
exits 0 at the head. origin/main (71a080048, STD-330 #1296) is one commit
ahead of the head's merge base and merges with no conflict. No other open
QSL PR touches FR-172 or TC-597; FR-172-AC-6 is unique.

Examined, clean:

- Codes and causes exist in qsl-semantics: `Unproved(Obligation::RationalRange)`
  maps to `undefined_expression` (refusal.rs:2203) and `unproved-range`
  (refusal.rs:2276-2280), raised at the literal (check.rs:2539-2544 when the
  value falls outside the expected Rational domain); `IllTypedCause::AmbiguousLiteral`
  with no hint (check.rs:2528-2533) is `ill_typed`/`ambiguous-literal`
  (qsl-replay outcome.rs:1444); `AmbiguousName` maps to
  `ambiguous_declaration`/`ambiguous-name` (refusal.rs:2196-2198, 2267-2269),
  and hyper `BodyShadows` reuses that pair (hyper_clause.rs:768, 801).
- `Rational[0, 1; 1, 2]` is the existing type spelling, and 3/4 lies outside
  that domain, so the `unproved-range` case can fail.
- Bullet 1 (value-typer rules for every non-atom, non-label, non-read body
  expression) is behaviour-only and testable through TC-597 step 6.
- The PR body flags both decisions as lead defaults, not owner answers.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The shadow rule and FR-172-AC-6 locate the refusal "at the binder", but a `let` has no binder span of its own: the value typer refuses a shadowing `let` at the `let` expression's location (typing.rs LetFrame `location`, check.rs:2205-2220 `bind`), and hyper_typing's `BodyShadows` carries "the `let`'s span" (hyper_clause.rs:692-699, hyper_typing.rs:253-266). Bullet 1 requires the same span as the value typer for a `let` shadowing another `let`, so the two shadow cases in one body would carry different loci, and "the binder" is not a span the tree records. Fix: say "at the `let`" in the bullet, AC-6 and TC-597 step 6, or define the binder span. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:173-176,194; spec/test-cases/TC-597-s3-checks-hyper-and-relation-clauses.md:37 | wrong-requirement |
| FND-002 | medium | FR-172-AC-6 and TC-597 step 6 take their oracle from "the value typer gives the same expression as a value clause body" "over the same parameters", but `x` in `Det` is a relation execution, and a value clause has no execution root (hyper_typing.rs:15-19: the value typer "has no execution, trace-read or step-label root to bind"). `half(x.i) < rational(3, 4)` therefore has no value-clause counterpart as written, and two implementers would build different comparison clauses. Fix: name the value clause, for example one over a parameter `x` of a record type with field `i` of `x.i`'s type, or state the expected locus directly. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:194; spec/test-cases/TC-597-s3-checks-hyper-and-relation-clauses.md:26 | wrong-requirement |
| FND-003 | medium | The shadow rule covers `hyper` bodies only, and the PR body states it does not cover relation execution names. S3 already refuses a `let` in a `relation` body that names one of its executions: hyper_typing.rs:261 refuses when `env.roots` holds the name, and a relation's roots are its executions (`Root::Execution`, hyper_typing.rs:32-37). After this PR no requirement owns that refusal, so an implementation that dropped it would meet the spec. Fix: extend the bullet to "a trace variable, object parameter or execution variable" and add the relation case to AC-6, or state that the relation case checks. | spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:173-176 | missing-requirement |

## Verdict

Not mergeable yet. The codes and causes are right. FND-001 is a locus that
neither typer produces, FND-002 is an oracle a test cannot build as
written, and FND-003 leaves an existing refusal with no owning requirement.
All three are small edits to FR-172 and TC-597 in this PR. The branch is one
commit behind origin/main and merges cleanly.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-004 | low | The PR body was not updated after the fix commit (1d22e4f26). It still says the shadow refusal is "at the binder", that "the rule covers `let` binders only" and that "It does not cover relation execution names". The spec now locates the refusal at the `let` and covers execution variables. Read under the 2026-10-22 standing rule, the body states a narrowing the spec no longer makes. It also describes the lead defaults Peter is asked to veto incorrectly. Fix: rewrite the Change and Decisions sections of the PR body to match FR-172 at d7330993. No spec change is needed. | PR #1301 body; spec/functional/FR-172-check-hyper-and-relation-clauses-over-model-subjects.md:165-178 | wrong-requirement |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 1d22e4f26: the bullet says "a `let` that shadows an enclosing `let` is refused at the inner `let`, as the value typer refuses it", the shadow rule refuses "at that `let`", and AC-6 and TC-597 step 6 say "at that `let`". |
| FND-002 | fixed | 1d22e4f26: AC-6 and TC-597 step 6 name the comparison clause: "a value clause over a parameter `x` of a record type with one field `i` of `x.i`'s type". |
| FND-003 | fixed | 1d22e4f26: the rule covers "a `hyper` body or a `relation` body" and "trace variables, object parameters or execution variables", and AC-6 adds "`Det` with body `let x = 1 in x = 1` ... naming execution variable `x`". |
