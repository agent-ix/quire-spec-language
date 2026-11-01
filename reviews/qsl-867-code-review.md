---
id: SR-202610310139
title: "Code review — QSL-867 hyper typer unification"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; qsl-semantics/src/check/check.rs, qsl-semantics/src/check/hyper_clause.rs, qsl-semantics/src/check/hyper_clause/lowering.rs, qsl-semantics/src/check/hyper_typing.rs, qsl-semantics/src/check/mod.rs, qsl-semantics/tests/it/hyper_clauses.rs"
review_set: subset
---

## Summary

Reviewed PR #2015 at `d63f4b94052c3454aff8d04af7c7824e1b6c79ce`, including the Rust lane, behavior, refusal attribution, and tagged tests. Focused hyper tests passed, but FR-172-AC-6's required code, tag, and source-locus assertions are incomplete, and one test comment now describes the removed divergence as remaining work.

## Verdict

**CONDITIONAL** — the implementation and focused checks pass; address the AC-6 assertion gap and stale test comment before merge.

## Assurance Context

No applicable AssuranceProfile was found in `spec/`; the `spec/assurance` files are measurement profiles or architecture context, not an AssuranceProfile.

## Scope

- `FR-172-AC-6`: examined the rational-range, ambiguous-literal, and root-shadowing requirements and their mapped tests.
- `TC-597` step 6: examined the expected result requiring exact code, cause, and corresponding expression location.
- All six changed Rust source/test files listed in the frontmatter were examined. No workflow or dependency files changed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new FR-172-AC-6 test checks only `BodyUnproved`/`RationalRange` and `BodyAmbiguousLiteral`; it does not assert the required `undefined_expression`/`unproved-range` and `ill_typed`/`ambiguous-literal` codes/tags or that the refusal span identifies `rational(3, 4)` / the ambiguous literal. A wrong `HyperCause::code`, `tag`, or span can therefore pass. Assert the code, tag, and exact span for the rational-range relation and both relation/atom ambiguity cases. | qsl-semantics/tests/it/hyper_clauses.rs:1263 |
| FND-002 | low | The comment above `a_shared_value_form_is_admitted_as_the_value_typer_admits_it` says the value and hyper typers still differ and unification is remaining work, although this PR removes the rational special case and adds a test asserting unified refusals. Update the comment to describe the remaining test's actual scope. | qsl-semantics/tests/it/hyper_clauses.rs:1224 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The regression test now asserts `Code::UndefinedExpression`/`unproved-range` and the `rational(3, 4)` span, plus `Code::IllTyped`/`ambiguous-literal` and the ambiguous literal span for relation and atom cases. |
| FND-002 | fixed | The comment now describes relation/value-typer parity without claiming unification remains unfinished. |
| FND-003 | fixed | The production import is unconditional, so default-feature compilation resolves `Scope` for the body refusal helpers. |
| FND-004 | fixed | `Found::Raw` now passes `CheckCause::ResourceExhausted` through without hyper-body translation, retaining the node-limit cause. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The prior fix round left `Scope` imported only under `cfg(test)`/`test-support`, but the new production `body_refusal` helpers use `Scope`; a default-feature `cargo check -p qsl-semantics` therefore failed to compile at those helpers. Make the import available in the production module. | qsl-semantics/src/check/hyper_clause.rs:49 |



## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | `body_refusal_at` translated every `CheckRefusal` into a hyper-body diagnostic; a `CheckCause::ResourceExhausted` therefore became `BodyIllTyped`, hiding the node ceiling and returning a type-error cause instead of the original limit cause. Preserve resource-exhaustion refusals through the hyper adapter. | qsl-semantics/src/check/hyper_clause.rs:1850 |
