---
id: SR-17231
title: "Spec review of the FR-315 amendment in QSL-1010 PR B(a), including the AC-12 weakening"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@34dab5e0880b9b27319c8903ae209b579d1935b2; PR #1570 (branch qsl-1010-hyper-identity), QSL-1010 PR B(a): spec/functional/FR-315-encode-the-smt-lib-transition-relation.md; context FR-336, FR-488, FR-172, spec/test-cases/TC-1190"
review_set: subset
date: '2026-10-25'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-315
    type: references
  - target: ix://agent-ix/quire-spec-language/TC-1190
    type: references
---

## Summary

Ticket: QSL-1010. Folded pass (EARS, integrity, failure-domain). AC-12 amendment judgement: **sound**. FR-336's `SubjectIdentity` holds the canonical `package_id` of the emission, which covers every node of the checked package including the clause's `hyperproperty` node (FR-488), so a fairness-only change must move the package member; "differs only in `clause()`" could never hold. The weakened AC-12 still gives CG (IR-974) what it uses the identity for: equality and hashing of an opaque, QSL-minted value that differs for different clauses (FR-172-AC-4), subjects and instances, equal for equal requests. `clause()` stays necessary to name which clause of a package the item is. The moved-to-qsl-replay wording follows FR-336-AC-6; the one-subject-per-alias wording is accurate to what the checked graph can express; the "inside the crate that defines the call" Faults wording is correct.

## Verdict

**CONDITIONAL** — FND-001 is an inconsistency this PR introduces.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-1190 Expected Results step 2 still says the fairness clause "differs only in `clause()`", contradicting the amended FR-315-AC-12 ("differs in `clause()` and in the checked package of `subjects()`, and not in `instance()`"). The test case that verifies AC-12 was not amended with it. | spec/test-cases/TC-1190-the-hp-2-product-identity-binds-each-smt-outcome-to-its-item.md:97 |
| FND-002 | medium | Outcome handoff and FR-315-AC-30 require `HyperProductIdentity::of` to be able to return `Fault` ("in its first four checks"), but the Faults bullet lists only invariants of `encode_hyper_query`'s later stages (match readings, atoms, conditions); no invariant of `of` is named, so AC-30's "checked clause altered after S3 so that `of` returns `Fault(f)`" has no specified alteration or fault name. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1203-1210 |
| FND-003 | low | The instance-binding wording says the universe is "the key list of the populations of its alias's subject that admit the parameter's object type" and, when the subject does not admit, all universes; the code also takes the all-universes branch when instance listing fails for a requires-bound or extent reason on an admitting subject. The spec should say which. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:1170-1176 |
| FND-004 | low | The code documents rows 18 to 23 as rows of "FR-315's refusal table", but the table in the spec ends at order 16 and states no order for the hyper codes; the order is given only in prose. | spec/functional/FR-315-encode-the-smt-lib-transition-relation.md:118-141 |

## Coverage

Examined: FR-315 Hyper self-composition queries (Refusals, Faults), Product identity (Members, Refusals and their order, Outcome handoff), AC-12, AC-13, AC-30, AC-49; context FR-336, FR-488, TC-1190. `quire validate` reports no error for FR-315 beyond the repo-wide module warnings.

## Dispositions

(none yet)
