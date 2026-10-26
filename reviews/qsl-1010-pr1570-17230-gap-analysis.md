---
id: SR-17230
title: "Gap analysis of QSL-1010 PR B(a): FR-315-AC-12, AC-13 and AC-49 coverage"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@34dab5e0880b9b27319c8903ae209b579d1935b2; PR #1570 (branch qsl-1010-hyper-identity), QSL-1010 PR B(a): spec/functional/FR-315-encode-the-smt-lib-transition-relation.md (AC-12, AC-13, AC-49), spec/test-cases/TC-1190, qsl-replay/tests/it/hyper_identity.rs"
review_set: subset
date: '2026-10-25'
---

## Summary

Ticket: QSL-1010. Planless run (Plan completion: not assessed). AC-12 is asserted in two tests (members, instance, proof bound, meter budget; fairness set), the script-equality and three-equal-`product` clauses wait for `encode_hyper_query` in PR B(b). AC-13's instance-binding, hyper-form, subject-binding (missing, foreign, excess) and product-identity (no emission) arms and AC-49's unassembled clause are asserted through `of`; the `encode_hyper_query` side of "same code by both" is B(b). No stub or tautology found. The amended AC-12 weakens the fairness clause but still separates two fairness variants; the one thing it no longer shows is that `clause()` carries information `subjects()` does not.

## Verdict

**CONDITIONAL** — coverage is as scoped for B(a); FND-001 is a small test addition.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No test builds two clauses in one checked package over the same subject, so nothing shows that `clause()` distinguishes identities independently of `subjects()`. After the AC-12 amendment the fairness test shows only that both members move together; the case that justifies keeping `clause` as a member (two hyperproperty nodes of one package) is untested. | qsl-replay/tests/it/hyper_identity.rs:123-143 |

## Coverage

Examined: FR-315-AC-12, AC-13, AC-49 against the tests in hyper_identity.rs; `quire matrix` was not run (planless, focused). Deferred to PR B(b) by scope: script byte-equality and the three equal `product` values of AC-12, the encode-side halves of AC-13 and AC-49, AC-14 (facade compile tests), AC-30.

## Dispositions

(none yet)
