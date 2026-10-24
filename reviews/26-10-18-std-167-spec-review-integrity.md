---
id: SR-12819
title: "Integrity review of PR #976: relaxed fence across FR-220, FR-309, ADR-025 and QSpec"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@cfcc4a707; FR-220, TC-665, FR-309-AC-3, ADR-025 MM-6, TC-406, TC-1060, TC-811; agent-ix/quire-specification@origin/main shared-grammar fence-order, FR-435-AC-6, TC-389 AC-10, TC-948 L12"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-309
    type: reviews
---

## Summary

Ticket: STD-167. PR quire-spec-language#976. This review checks the amended
FR-220-AC-2 against every artifact that states what happens to
`fence F ordering relaxed;`, in QSL and in QSpec (quire-specification, read
only, at origin/main).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Cross-spec contradiction. FR-220 `depends_on` QSpec FR-435, and QSpec FR-435-AC-6 still says "A `fence` with `relaxed` refuses `ill_typed`/`operator-ineligible`". QSpec TC-389 row AC-10 expects the same. After this PR, FR-220-AC-2 says S1 refuses it with `invalid_syntax`. The PR is right according to QSpec's own grammar, so the defect is in QSpec: its FR-435-AC-6 contradicts its shared grammar's `fence-order`. If this PR merges alone, a QSpec TC-389 conformance run (or a later implementer reading the upstream AC) expects `ill_typed` at S3 where QSL emits `invalid_syntax` at S1. Then one of the two gets "fixed" back. STD-167 is a QSpec-team (STD) ticket, and its ruling is marked "QSL planner default, not spec-owner". So QSpec FR-435-AC-6 and TC-389 AC-10 need the same amendment, paired with this PR or tracked on STD-167 before it closes. Escape cause: wrong-requirement (upstream). | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:113; quire-specification spec/functional/memory/FR-435-classify-memory-accesses-and-orderings.md:131; quire-specification spec/test-cases/TC-389-memory-access-classification.md:39 |
| FND-002 | low | FR-220 now states an S1 obligation but does not point to FR-309-AC-3, where S2 refuses the same input as `RecoveringCst`. Citing FR-309-AC-3 in the new Behavior bullet would show the reader the S1 to S2 layering. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:89-93 |

## Verdict

The QSL side is consistent. Merge it together with the QSpec FR-435-AC-6 and
TC-389 AC-10 amendment (FND-001), or with that amendment tracked explicitly on
STD-167. FND-002 is a low.

Checked and clean:

- FR-309-AC-3 is not contradicted, and it needs no edit. S1 reports
  `invalid_syntax` at the token and leaves a recovering CST. S2 then refuses
  with `FormsCause::RecoveringCst` and builds no `FenceForm`, and TC-406 maps
  `RecoveringCst` to `invalid_syntax`. The `terminal` declaration already
  layers the same way: FR-124 and TC-1060 at S1 (`invalid_syntax` at the
  token), then FR-382-AC-4 and TC-1059 at S2 (`RecoveringCst`). "Refuses at S1"
  in FR-220 and "refuses as a recovering CST" in FR-309 describe the same
  failure at two stages, and they agree on the code.
- ADR-025 MM-6 already limits the fence ordering to `acquire`, `release`,
  `acq_rel` and `seq_cst`.
- QSpec shared-grammar.md:543 `fence-order` excludes `relaxed`, and :89 makes
  the words contextual.
- QSpec TC-948 L12 (a wire `fence` with `ordering: relaxed` refuses
  `malformed_wire`) is a wire-layer check and is consistent.
- PR #974 / QSL-557. Its parse test refuses a relaxed fence at `relaxed`, and
  its body named exactly this disagreement. The amended FR-220-AC-2 and TC-665
  now match that behaviour.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-220's Dependencies now cites "(quire-specification PR #276)". That is a build-history pointer, and it goes stale once the PR merges. Citing QSpec FR-435-AC-6 alone is enough. | spec/functional/FR-220-check-access-orderings-fences-and-access-classification.md:125-127 |
| FND-004 | low | QSpec PR #276's FR-435-AC-6 reads "refuses `refused { code: invalid_syntax, … }`", with the verb doubled. TC-188 R04 and TC-389's own row use the bare `refused { … }` form. | quire-specification spec/functional/memory/FR-435-classify-memory-accesses-and-orderings.md:131 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Fixed in the companion quire-specification#276 (bf1ceef). FR-435-AC-6 and TC-389 AC-10 now expect `refused { code: invalid_syntax, cause: unexpected-token }` at `relaxed`, and FR-220 cites FR-435-AC-6. The fix holds only if #276 merges with #976 or before it. |
| FND-002 | fixed | Round 1, cf22f23a1: the S1 bullet now ends "as S2 refuses a recovering CST (FR-309-AC-3)". |
