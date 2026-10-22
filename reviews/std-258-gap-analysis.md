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

## Disposition pass 1

Disposition pass 1 at QSL 203b8c172 and QSpec 4248c9f08 (fix commits 203b8c172 and 4248c9f08, diffs only). Gates in detached worktrees: QSL `make check-spec-validation` exits 0 with no diagnostic on FR-280, TC-761 or ADR-029; QSpec `quire validate --scope . 'spec/**/*.md'` gives 86 fail lines (FR-331's three diagnostics are the ones on main); `cargo test --test checked_package_v2 --test backend_provider_v1` passes 126 and 23. QSL PR #1206 is CONFLICTING with main: FR-280-AC-4 (main renamed `BackendDescriptor::admit` to `descriptor_from_manifest`, #1204) beside the edited AC-5 row.

Disposition pass 1: both findings fixed; one new low finding (FND-003, minor).

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-331-AC-33 now also requires the process-provider item over an unemitted package to settle `unsupported` with `unsupported_projection`/`unsupported-requested-capability` naming the kind, the backend and that the package is not emitted. TC-220 LC-07, the row that traces AC-33, checks only that no frame is written; the diagnostic is checked in LC-06, which traces FR-300-AC-6 alone. Fix: add the diagnostic to LC-07's expected result, or list FR-331-AC-33 on LC-06. | quire-specification spec/test-cases/TC-220-rust-lifecycle-api-contract.md:29-30; quire-specification spec/objects/interfaces/FR-331-backend-provider-envelope.md:605 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-331-AC-33 states an observable: "two in-process requests of the same items run the engine twice and the second result comes from the second run"; the shape clause is now "holds ... as typed values"; TC-220 LC-07 counts the engine's runs. See FND-003 for a new coverage gap in LC-07. |
| FND-002 | fixed | TC-761 step 4 expected result adds "The run holds no `EmittedPackage` of `Service`, and `emit_checked` over `Service` is refused after the entry returns as it was before." |
