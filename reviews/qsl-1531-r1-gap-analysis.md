---
id: SR-20261103161228
title: 'Gap analysis of PR #2195: QSL-1531 successors for retained composed criteria, gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@173a1b75a; FR-513-AC-3..AC-8 and TC-1308 rows 5-17 against tests/it/call_requirements_items.rs, tests/it/composed_domain_models.rs and the qsl-semantics check changes of PR #2195; ticket QSL-1531'
review_set: subset
date: '2026-11-03'
---

## Summary

Ticket: QSL-1531. PR: quire-spec-language#2195. Plan completion: not assessed (planless). Independent second gap analysis. Criteria already recorded as gaps on this ticket and deferred to QSL-1580 (AC-3 domain namesake ambiguity, AC-4 measured recursion and guarded division, AC-5 unavailable elements, AC-7 report stage) and QSL-1579 (AC-8 Port identity through the checked Role layout) are not re-raised. This pass looks for gaps inside the clauses the PR claims to cover, and for production code that no criterion owns.

Tagged tests per criterion (by `#[trace]` grep on the PR head): AC-3 one test, AC-4 two, AC-5 two, AC-6 one, AC-7 none (deferred), AC-8 one.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-513-AC-3 | spec/functional/FR-513-*.md | examined |
| FR-513-AC-4 | spec/functional/FR-513-*.md | examined |
| FR-513-AC-5 | spec/functional/FR-513-*.md | examined |
| FR-513-AC-6 | spec/functional/FR-513-*.md | examined |
| FR-513-AC-7 | spec/functional/FR-513-*.md | examined |
| FR-513-AC-8 | spec/functional/FR-513-*.md | examined |
| FR-493-AC-5 | spec/functional/FR-493-*.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-513-AC-4's "dependent calls refuse too" (TC-1308 row 9, "each caller of a refused predicate refuses too") has no assertion: `Dependent` is declared but its refusal is never checked. | tests/it/call_requirements_items.rs:370-400 |
| FND-002 | medium | FR-513-AC-4's "a predicate whose result is not `Boolean` refuses at the assembler" is asserted only as "some refusal exists"; the stage, cause and result-form span are unchecked. | tests/it/call_requirements_items.rs:466-475 |
| FND-003 | medium | Code with no owning criterion or test: the `CheckedRole.model_selection` widening to every alias-qualified role (protocol_clause/mod.rs:221-235) changes refinement and replay-subject behaviour, and no test or criterion covers it. Likewise the removed `UnsupportedModelMember` refusal for Component/Endpoint/Allocation records (assemble.rs:1351-1360) has no negative test showing a systems model that still cannot be used as a value type refuses elsewhere. | qsl-semantics/src/check/protocol_clause/mod.rs:221-235 |
| FND-004 | low | FR-493-AC-5 is tagged on `native_package_and_runtime_surfaces_are_absent`, which does not exercise the matrix property the criterion states; the coverage is inflated. | tests/it/retired_native_runtime.rs:60 |

## Verdict

Changes requested on FND-001 to FND-003 (medium). FND-004 is low. Clean: AC-3's four-site call identity, AC-4's unmeasured cycles, arity/type/unit refusals at the call span and the unguarded-division refusal, AC-5's empty identities, invalid-value refusal and order preservation, AC-6's work/result bounds and retry, and AC-8's Port owner, direction, interface type and multiplicity, connection admission, and field/Port/Connection binding through I2. AC-7 has no tagged test (deferred to QSL-1580). `quoin matrix --repo . --json` did not finish within 600 s. The coverage counts above come from a manual `#[trace]` grep of the PR head.
