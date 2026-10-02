---
id: SR-254
title: "Plan-008 gaps after state-scalar delivery"
type: SpecReview
analysis: gap-analysis
scope: "plan/Plan-008-native-lowering/; spec/native-lowering/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/Plan-008
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TM-006
    type: references
---
## Summary

Current recheck is narrow: reverse ownership and trace backing for correction
`09c50a5` against integrated baseline `87b35ea`, covering only the four external
findings' corrected behavior in FR-034 and TC-112. It is not a status
reconciliation, a backlog campaign or a re-run of the complete Plan-008 verdict
below, which stands unchanged from the earlier `b789eed` assessment. The optional
QUOIN semantic pass was declined by the owner; read-only coverage was used with
an explicit repository scope.

## Verdict

Unchanged and distinct from the correction: FAIL for complete Plan-008
acceptance because Task-020 is incomplete. The owner's explicit
engineering-delivery direction permits this scoped PR while deferred assurance
remains visible.

Separately, for this correction only: FR-034 and TC-112 own every corrected
behavior and are trace-backed. That is a narrow coverage result, not Plan-008
acceptance and not a claim about the PR, the numeric backend or activation
qualification.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Generated activation qualification remains incomplete; tagged TC-094 does not establish a passing run. Unchanged by this correction; no current issue state asserted without evidence. | Task-020; FR-009-AC-5; IT-008-SC-04 |
| FND-002 | medium | Compiler issue #28 owns the status-column mismatch; authored Tested marks are not engine-verified. Unchanged by this correction. | TM-006 |

## Coverage

Reverse ownership of the correction's four changed behaviors, each traced to an
owning requirement line:

- Typed invariant reasons with the retained original projected read, and
  non-exhaustive public error/reason types → FR-034 Inputs paragraph, rewritten
  in this correction.
- Predecessor boundaries (native linking refuses an invariant's invocation
  input; validation refuses `Current` for pre/post and missing
  invocations/parameters, producing no `ValidatedContext`) → the new FR-034
  paragraph after Behavior.
- One charged node per fresh field-alias candidate, including collisions, with
  no charge for alias reuse and no added AST depth → the new FR-034 behavior
  line; also the `LoweringLimits`/`LoweringUsage` doc comments.
- Shared explicit integer target policy → the existing FR-034 line preserving
  the Boolean and integer target contracts; the single-source-of-truth is an
  implementation idiom under it, not separate behavior.

TC-112 gained matching procedure and expected-result text for the seventeen
alias candidates, the exact/one-short/retry budgets, the three-target
Boolean/integer controls, the public predecessor refusals and the explicit
instruction not to fabricate a validated context.

Coverage reconciliation: actual `quire coverage --scope . --json`, Quire 0.31.0,
engine `ca7362d4`; no grep fallback. Global rollup 373/382 backed. No FR-034 or
TC-112 row is unbacked, no unmatched tag and no untracked symbol occurs in
`tests/state_scalar_lowering.rs`, and the three new tests carry resolving
`#[trace]` attributes for TC-112 and FR-034-AC-1/3/4/5. The nine unbacked rows
are inherited and out of this correction's scope (FR-036-AC-5/6/8 and TC-115,
TC-010/NFR-005-M-1, FR-017-AC-2); this recheck makes no claim about them.

No unowned behavior, source stub or test stub was found in the corrected slice.
Numeric backend and graph parity remain wider LC04 work, with no new claim of
their completion.
