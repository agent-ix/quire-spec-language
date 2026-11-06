---
id: SR-20261105101046
title: "Base specification review of QSL-1753 recovery case-arm admission"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; QSL-1753; tests/it/bare_recovery_binder.rs, FR-390-AC-68, TC-1071, TC-1333"
review_set: subset
---

## Summary

Source-only review of draft PR #2425. No concrete defect found in the changed source/specification; runtime and Clippy remain unqualified pending the owner-managed QSL lock update consuming IR #921.

## Verdict

**PASS (source-only)**. The change removes the obsolete BinderScope prohibition and states the intended arm-local scope and genuine admitted/value parity fixture consistently across FR-390-AC-68 and both TC rows. Original refusal, unknown-member and shadowing expectations are retained. Existing artifact IDs and relationships are unchanged. No digest, dependency pin, file-tracking framework, new owner shape or scope reduction is introduced. The spec-first commit precedes the test commit. No applied AssuranceProfile was found in this scope. Runtime acceptance is not claimed.

## Scope

- FR-390-AC-68 (examined), spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md: Over recipe **R**'s `Full` recovery predicate (`sum(receipt in recovered.receipts: receipt) = target and not activated`), the slot lists the sources `target`, `activated` and `recovered` and not `receipt`, on the bare route; the same holds with the `sum` replaced by a `let` binder bound to it, with two nested collection queries of distinct names, with each of `forall`, `exists`, `filter` and `map`, and with a binder the body never reads (alone and nested), none of those binders being a source. The genuine `case Shape::Circle(sum<M::Total>(receipt in recoveredFull.receipts: receipt)) { Empty: 0
- FR-390 (examined), spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md: The contract reader admits a `case` arm binder in its arm body;
   that binder is not a source of the protocol slot. Its scope does not extend
   to the scrutinee or another arm.
- TC-1071 (examined), spec/test-cases/TC-1071-refuse-a-protocol-slot-selection-or-supply-before-any-charge.md: | 26 | From the checked-handoff package of recipe **R**, select `Full`'s `recovery.predicate`; then the same predicate with its `sum` bound by a `let`, and with two nested `count`/`sum` queries, with `forall`, `exists`, `filter`, `map` and an unread binder, and with a genuine `Shape::Circle(sum(...))` case whose `Circle(r)` arm returns `r`; supply the three sources, then add a key for the introduced binder; a unit whose query binder is named `targetFull`, as the source is | strict reader admission succeeds, the sources are `target`, `activated`, `recovered` in each form and never the introduce
- TC-1333 (examined), spec/test-cases/TC-1333-evaluate-a-protocol-slot-from-the-checked-package-s-own-nodes.md: | 19 | 19 | **R**'s `Full` and `Partial` `recovery.predicate` over the bare route, with supplies that differ in receipts, target and activation; `Full` with its sum inside `Shape::Circle` and a `Circle(r)` case arm, over matching, mismatching, empty and activated supplies; a `recovered` supply that is an integer; a supply without `target` | the values FR-533-AC-19 states; the case-arm package is strictly admitted, `r` is not a source, and its Boolean outcomes and ordered charge logs equal the source route's; both malformed supplies refuse as `CallFailure::Admission` |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Evidence and limits

No Cargo build, test, gate, lock or source edit was performed by the reviewer. Changed-spec validation succeeded; warnings concern installed duplicate module definitions, not this diff. The draft is not merge-ready until its real owner dependency and runtime evidence are available. The lead owns one final scoped gate. Optional criterion-strength, failure-domain, architecture, risk, integrity, evidence and dependency lenses are not selected for this small test/spec delta.
