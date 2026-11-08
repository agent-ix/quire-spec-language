---
id: SR-20261107070951249614
title: IR-1400 QSL issued full-result authority — gap-analysis
type: SpecReview
analysis: gap-analysis
scope: agent-ix/quire-spec-language; IR-1400; PR2569; origin/main...008332773b511d38027c5ca70b2863f1a9adc7e2
review_set: subset
---

## Summary

Review of the frozen QSL full-result authority change for IR-1400. No concrete defect identified in this method's examined scope.

## Verdict

**CONDITIONAL** — Changed-surface traceability is clean; full-repository assurance is not assessed under the explicit scope restriction.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Coverage

Plan completion: not assessed

The authorized review is limited to this blocker and its 19-file diff. New behavioral surface inventoried: the issued carrier returned by the free/package evaluator, result(), and predecessor(); each is owned by FR-516/FR-526. No new untraced behavior or source/test stub was found within this changed surface. FR-526-AC-30 is bound by the real public-package test and specifies the three compile-fail controls. Existing refusal, incomplete, subject, source-map, ledger and charge assertions remain after snapshot-only test migrations. Supplied logs show the exact genuine test passed, all three negative examples passed, and the lineage-charge test passed. This review does not claim downstream integration PR89 is qualified.

The repository-root static computed matrix (`quire matrix --scope . --format json`) was attempted and stopped after about four minutes with no JSON emitted, preserving the bounded blocker review. No matrix result or global criterion count is claimed. Manual trace fallback confirms the new test carries FR-526-AC-30 and TC-1321, and its body reaches the real checked-package evaluator. Matrix completeness, stale-tag enumeration and whole-repository reverse-gap inventory remain unassessed. A whole-repository reverse inventory and repository-wide semantic assurance were not authorized and are not claimed. No repository-wide PASS is issued from this blocker review. The CONDITIONAL scope limit is not an identified code defect or a new blocker ticket; it prevents this report being interpreted as an end-to-end repository gap audit. Optional separate semantic gap-analysis was not invoked; code/test intent was inspected as part of ordinary code review. No plan was supplied.

## Examined scope

```yaml
scope:
- id: FR-516
  path: spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md
  role: examined
  excerpt: 'Entry: `EvidencePackage::evaluate_temporal_evidence(&self, clause:

    &CheckedNodeId, request: &EvidenceRequest, meter: &mut Meter) ->

    Result<EvaluatedEvidence, CallFailure>`. The opaque carrier borrows its original

    `EvidenceResult` through `result()` (FR-526 predecessor authority).

    **Refusal has one outcome.** Every refusal of the call itself is

    `Err(CallFailure::Admission)`, nothing is charged and no result exists: a clause

    not held by the package or not a temporal clause, a request that fails

    construction (an empty trigger identity), '
- id: FR-526-AC-30
  path: spec/functional/FR-526-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md
  role: examined
  excerpt: A real checked-package temporal evaluation returns an evaluator-issued
    immutable full-result carrier. Repeated immutable result and predecessor borrows
    retain the original truth, support, settlement, checked subject and ledger without
    further meter charges. A changed-input correction names the original carrier's
    identity, changes actual truth, support, settlement and latest ledger, advances
    lineage, and leaves the original unchanged, including after mutation of a cloned
    read snapshot. Compile-fail tests reject constructing the carrier, mutating its
    original result and substituting a mutated sn
- id: TC-1321
  path: spec/test-cases/TC-1321-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md
  role: examined
  excerpt: 'Verify FR-526-AC-1 to FR-526-AC-30. Results `r1`, `r2`, `r3` are evaluated
    from **R**''s event-position clause with trigger `t1`, passing the ledger from
    call to call. Row 29 verifies FR-516''s `valuation-atom-unknown`, the successor
    of FR-043-AC-16''s `OperandNotInFormula`. Rows 20 to 28 verify the subject, support,
    predecessor-authority and lineage-limit criteria that succeed FR-052-AC-15, AC-16,
    AC-19 and AC-20. Expected values come from the package, the request and the FR-501
    decision-support rule, never from the evaluator''s own output.


    ## Te'
- id: qsl-eval/src/temporal/evidence/call.rs
  path: qsl-eval/src/temporal/evidence/call.rs
  role: examined
  excerpt: ") -> Result<EvaluatedEvidence, CallFailure> {\n    Ok(EvaluatedEvidence\
    \ {\n        result: conclude(&facts, outcome, meter),\n    })\n}\n\n/// An immutable\
    \ full result issued only by the temporal evidence evaluator.\n///\n/// The owned\
    \ result binds the checked subject and admitted request to the actual\n/// truth,\
    \ support, settlement and returned ledgers. Reading it performs no\n/// evaluation\
    \ or metering. Cloning a read result produces ordinary data, not\n/// another\
    \ issued result. Result identities alone authenticate input members,\n/// not\
    \ caller-supplied output data (FR-526).\n///\n/// The carrier canno"
- id: qsl-eval/src/temporal/evidence/mod.rs
  path: qsl-eval/src/temporal/evidence/mod.rs
  role: examined
  excerpt: pub use call::{evaluate_temporal_evidence, matches_result_identity, EvaluatedEvidence};
- id: qsl-eval/src/temporal/evidence/package.rs
  path: qsl-eval/src/temporal/evidence/package.rs
  role: examined
  excerpt: "use super::call::{\n    evaluate_temporal_evidence, fault, refusal, select_occurrence,\
    \ EvaluatedEvidence,\nuse super::request::{ActivationRow, EvidenceRequest, EvidenceSubject,\
    \ SubjectOccurrence};\n    ) -> Result<EvaluatedEvidence, CallFailure> {"
- id: qsl-eval/src/temporal/evidence/request.rs
  path: qsl-eval/src/temporal/evidence/request.rs
  role: examined
  excerpt: '/// Read data of a temporal evidence call (FR-516 "Outputs").

    ///

    /// Caller-authored or cloned data carries no evaluator authority. Borrow the

    /// original full result from [`super::EvaluatedEvidence::result`].'
- id: qsl-eval/tests/it/checked_route_owner.rs
  path: qsl-eval/tests/it/checked_route_owner.rs
  role: examined
  excerpt: '        .map(|issued| issued.result().clone())'
- id: qsl-eval/tests/it/checked_route_trigger.rs
  path: qsl-eval/tests/it/checked_route_trigger.rs
  role: examined
  excerpt: "    let result = evaluate_temporal_evidence(subject, asked, &mut meter)\n\
    \        .map(|issued| issued.result().clone());"
- id: qsl-eval/tests/it/package_aggregate_route.rs
  path: qsl-eval/tests/it/package_aggregate_route.rs
  role: examined
  excerpt: "            .map(|issued| issued.result().clone())\n                .map(|issued|\
    \ issued.result().clone())\n                    .map(|issued| issued.result().clone())\n\
    \                .map(|issued| issued.result().clone())\n                .map(|issued|\
    \ issued.result().clone())\n            let refused = routes\n               \
    \ .from_read\n                .evaluate_temporal_evidence(clause, &wrong_kind,\
    \ &mut meter)\n                .map(|issued| issued.result().clone());\n     \
    \           .map(|issued| issued.result().clone())\n                .map(|issued|\
    \ issued.result().clone())\n                    .m"
- id: qsl-eval/tests/it/temporal_activation.rs
  path: qsl-eval/tests/it/temporal_activation.rs
  role: examined
  excerpt: "        .map(|issued| issued.result().clone())\n    let refused = evaluate_temporal_evidence(&subject,\
    \ &clash, &mut meter)\n        .map(|issued| issued.result().clone());\n    let\
    \ refused = evaluate_temporal_evidence(&subject, &asked, &mut meter)\n       \
    \ .map(|issued| issued.result().clone());"
- id: qsl-eval/tests/it/temporal_causes.rs
  path: qsl-eval/tests/it/temporal_causes.rs
  role: examined
  excerpt: "    let refused = evaluate_temporal_evidence(&each.subject(&[]), &asked,\
    \ &mut meter)\n        .map(|issued| issued.result().clone());\n    let refused\
    \ = evaluate_temporal_evidence(&origin_env.subject(&[]), &asked, &mut meter)\n\
    \        .map(|issued| issued.result().clone());\n            evaluate_temporal_evidence(&subject,\
    \ &asked, &mut meter)\n                .map(|issued| issued.result().clone()),\n\
    \        let result = evaluate_temporal_evidence(&subject, &asked, &mut meter)\n\
    \            .map(|issued| issued.result().clone());\n    assert!(\n        evaluate_temporal_evidence(&subject,\
    \ &unvalued, &mut"
- id: qsl-eval/tests/it/temporal_identity.rs
  path: qsl-eval/tests/it/temporal_identity.rs
  role: examined
  excerpt: "            .map(|issued| issued.result().clone())\n        .map(|issued|\
    \ issued.result().clone())\n        .map(|issued| issued.result().clone())\n \
    \       .map(|issued| issued.result().clone())\n            .map(|issued| issued.result().clone())"
- id: qsl-eval/tests/it/temporal_relation.rs
  path: qsl-eval/tests/it/temporal_relation.rs
  role: examined
  excerpt: "        .map(|issued| issued.result().clone())\n        let refused =\
    \ evaluate_temporal_evidence(&env.subject(&[]), &same, &mut meter)\n         \
    \   .map(|issued| issued.result().clone());\n    let one = evaluate_temporal_evidence(&env.subject(&[]),\
    \ &asked, &mut first)\n        .map(|issued| issued.result().clone())\n      \
    \  .expect(\"a result\");\n    let two = evaluate_temporal_evidence(&env.subject(&[]),\
    \ &asked, &mut second)\n        .map(|issued| issued.result().clone())\n     \
    \   .expect(\"a result\");\n    .map(|issued| issued.result().clone())\n     \
    \   .map(|issued| issued.result().clone())\n        .ma"
- id: qsl-eval/tests/it/temporal_rulings.rs
  path: qsl-eval/tests/it/temporal_rulings.rs
  role: examined
  excerpt: "    let refused = evaluate_temporal_evidence(&env.subject(&[]), &through,\
    \ &mut meter)\n        .map(|issued| issued.result().clone());\n    let result\
    \ = evaluate_temporal_evidence(&subject, &base, &mut tight)\n        .map(|issued|\
    \ issued.result().clone())\n        .expect(\"an Ok result\");\n    let result\
    \ = evaluate_temporal_evidence(&subject, &asked, &mut meter)\n        .map(|issued|\
    \ issued.result().clone())\n        .expect(\"a result\");\n    let result = evaluate_temporal_evidence(&subject,\
    \ &asked, &mut meter)\n        .map(|issued| issued.result().clone())\n      \
    \  .expect(\"a result\");\n    let first"
- id: tests/it/bare_package.rs
  path: tests/it/bare_package.rs
  role: examined
  excerpt: "        let refused = evaluation\n            .evaluate_temporal_evidence(&id,\
    \ &asked, &mut meter)\n            .map(|issued| issued.result().clone());\n \
    \       let refused = source_route\n            .evaluate_temporal_evidence(&clause,\
    \ &asked, &mut meter)\n            .map(|issued| issued.result().clone());"
- id: tests/it/temporal_package_route.rs
  path: tests/it/temporal_package_route.rs
  role: examined
  excerpt: "    let result = package\n        .evaluate_temporal_evidence(clause,\
    \ asked, &mut meter)\n        .map(|issued| issued.result().clone());\n      \
    \          )\n                .map(|issued| issued.result().clone());\n      \
    \      let refused = package\n                .evaluate_temporal_evidence(clause,\
    \ &foreign, &mut meter)\n                .map(|issued| issued.result().clone());\n\
    \        let refused = back\n            .evaluate_temporal_evidence(&node, &asked,\
    \ &mut meter)\n            .map(|issued| issued.result().clone());\n    let refused\
    \ = back\n        .evaluate_temporal_evidence(&node, &asked, &mut m"
- id: tests/it/temporal_package_route_formula.rs
  path: tests/it/temporal_package_route_formula.rs
  role: examined
  excerpt: "        let refused = emitted\n            .evaluate_temporal_evidence(&clause,\
    \ &asked, &mut meter)\n            .map(|issued| issued.result().clone());\n \
    \       let refused = back\n            .evaluate_temporal_evidence(&clause, &asked,\
    \ &mut meter)\n            .map(|issued| issued.result().clone());\n        let\
    \ refused = package\n            .evaluate_temporal_evidence(&id, &asked, &mut\
    \ meter)\n            .map(|issued| issued.result().clone());\n            .map(|issued|\
    \ issued.result().clone())"
- id: tests/it/temporal_result_evidence.rs
  path: tests/it/temporal_result_evidence.rs
  role: examined
  excerpt: "/// The issued carrier retains the actual complete output, including an\n\
    /// immutable predecessor reference, without another evaluation.\n#[trace(\"FR-526-AC-30\"\
    , \"TC-1321\")]\n#[test]\nfn issued_full_result_borrows_preserve_outputs_and_do_not_evaluate_again()\
    \ {\n    use super::temporal_package_route::unlimited;\n    use qsl_eval::temporal::evidence::{Relation,\
    \ Settlement};\n\n    let built = both();\n    let read = built.read(&built.emission);\n\
    \    let package = read_back(&read);\n    let [clause] = clause_ids(&package).try_into().expect(\"\
    one clause\");\n    let mut asked = both_request(&package, &clause);"
- id: FR-526
  path: spec/functional/FR-526-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md
  role: examined
  excerpt: "- `evaluate_temporal_evidence` SHALL return an opaque `EvaluatedEvidence`\
    \ that\n  uniquely owns the complete original `EvidenceResult`. Its `result()`\
    \ borrows\n  that result immutably, including checked subject, truth, support,\
    \ settlement\n  and ledgers; `predecessor()` borrows this issued result's `ResultIdentity`\
    \ for\n  the next correction. The current previous link remains in `relation`."
- id: FR-526
  path: spec/functional/FR-526-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md
  role: examined
  excerpt: "- Only the evaluator SHALL issue this carrier. It SHALL expose no public\n\
    \  constructor, deserialization construction, mutable borrow or consuming escape\n\
    \  of its original data. Reading the result or predecessor SHALL neither evaluate\n\
    \  again nor charge work. A cloned read snapshot is ordinary caller data and\n\
    \  SHALL NOT substitute for the issued carrier."
- id: FR-526
  path: spec/functional/FR-526-chain-a-temporal-evidence-result-to-its-direct-predecessor-and-refuse-a-relation-that-changes-nothing.md
  role: examined
  excerpt: "- `result_identity` and `matches_result_identity` bind input members only;\n\
    \  neither authenticates caller-authored truth, support, settlement or ledger.\n\
    \  The carrier binds the admitted request and checked subject to the actual\n\
    \  evaluator output by retaining that output privately. This adds no serializer,\n\
    \  predecessor bytes, result-byte storage or recursive chain."
```
