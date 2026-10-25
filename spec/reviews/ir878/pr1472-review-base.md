---
id: SR-17048
title: "IR-878 PR #1472 base spec review of FR-486"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-486-supplied-admission-and-conversion-owner-allocation.md, spec/reviews/ir878/dependency.md, spec/spec.md, spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md, spec/test-cases/TC-460-s3-refuses-ill-formed-state-clauses.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-486
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (base) over the spec-only re-derivation of closed #995 as FR-486. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-486 (examined), FR-486 (examined), FR-486-AC-1 (examined), FR-486-AC-2 (examined), FR-486-AC-3 (examined), FR-486 (examined), spec/spec.md (examined), TC-459 (examined), TC-460 (examined), spec/reviews/ir878/dependency.md (examined).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The upstream owners still allocate these obligations to QSL FR-394, which on main is the enum-key replay facade: QSV FR-109 Dependencies and FR-122 Dependencies, and quire-exact FR-372 (formerly AC-5 note) and FR-380 (formerly AC-6 note) all link ix://agent-ix/quire-spec-language/FR-394. With FR-486 landing alone, every upstream allocation resolves to the wrong requirement and nothing points at FR-486; companion retargeting PRs in quire-semantic-value and quire-exact are required. | FR-486:44-50 |
| FND-002 | low | TC-459 and TC-460 drop their FR-104-AC-9 verifies edges. The change is unrelated to IR-878, quire validate passes on main with the edges present, the tests still carry #[trace("TC-459"/"TC-460", "FR-104-AC-9", ...)] and both TC bodies still list FR-104-AC-9 in Scope. Inherited from #995 without a stated reason; split it out or justify it in the PR. | spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md:1-10 |
| FND-003 | low | The PR's own dependency review (SR-17023) says QSpec FR-323 is referenced only in prose because it is not represented as a local artifact, but FR-486 frontmatter carries a depends_on edge to ix://agent-ix/quire-specification/FR-323 (which resolves; FR-323 publishes the 4096-bit B default). The Dependencies bullet for FR-323 is the only unlinked one. The self-review misstates the artifact. | spec/reviews/ir878/dependency.md:18 |

## Verdict

Request changes. quire validate and make check-spec-validation pass, the hash/digest/pin checklist is clean, the spec.md contains-edge and index row for FR-486 are present and placed alongside the evaluator/replay rows, and all ix:// targets resolve (QSV FR-109/FR-122, quire-exact FR-372/FR-380, quire-specification FR-323 with its 4096-bit default). The blocker is that upstream allocations still name QSL FR-394.
