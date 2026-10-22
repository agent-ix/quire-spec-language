---
id: SR-15112
title: "gap analysis of STD-258 new and changed ACs against TC rows (PR #1206, QSpec PR #357)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@48fc9a675a111d90eb03746556d435d7e5eaaa0b; spec/functional/FR-280-serve-the-driver-s-lower-generate-and-prove-operations.md, spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md; agent-ix/quire-specification@203fa2efdba64c0fe18dacbf0659d49420439e2d; spec/functional/tooling/FR-300-expose-rust-lifecycle-api.md, spec/objects/interfaces/FR-331-backend-provider-envelope.md, spec/test-cases/TC-220-rust-lifecycle-api-contract.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-761
    type: reviews
---
# SR-15112: gap analysis of STD-258 (PR #1206, QSpec PR #357)

## Summary

Ticket: STD-258. Plan completion: not assessed. This is a manual mapping of
each new or changed AC to its TC rows, spec only. No provider entry exists in
code yet, so no test is tagged. Mapping:

- FR-280-AC-2 (now the `CheckedPackage`) maps to TC-761 step 2 and its
  expected result.
- FR-280-AC-5 (new) maps to TC-761 step 4. TC-761 Scope lists it.
- FR-300-AC-6 (new) maps to TC-220 LC-06.
- FR-331-AC-33 (new) maps to TC-220 LC-07. TC-220 now `verifies` FR-331.

Every changed AC has a TC row.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-331-AC-33 and TC-220 LC-07 require that an in-process result "is not reused" and offer it "for reuse to a second run". Neither file defines a reuse step, or what a test observes, for a request that has no request identity. The "carries no `package` ... member" clause is a property of the typed value's shape, which no runtime check can fail. Two test authors would build different tests. Fix: name the reuse path (for example the driver's result cache) and the observable, such as the engine running twice, and state the shape clause as a type, or drop it. | quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:597; quire-specification spec/test-cases/TC-220-rust-lifecycle-api-contract.md:30 |
| FND-002 | low | TC-761's step 4 expected result does not check FR-280-AC-5's "No `EmittedPackage` of `Service` exists in the run". Fix: add it to the step 4 expected result. | spec/test-cases/TC-761-qsl-engine-manifests-register-and-the-provider-entry-returns-analyze.md:31 |

## Verdict

Every changed AC has a TC row. Two minor findings, no major ones.
