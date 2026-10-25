---
id: SR-17028
title: "IR-878 PR #1472 base spec review of FR-485"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-485-supplied-admission-and-conversion-owner-allocation.md, spec/reviews/ir878/dependency.md, spec/spec.md, spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md, spec/test-cases/TC-460-s3-refuses-ill-formed-state-clauses.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-485
    type: reviews
---

## Summary

Ticket: IR-878. PR: quire-spec-language#1472 (head 5e56adbb7, diff `origin/main...HEAD`). Reviewer pass (base) over the spec-only re-derivation of closed #995 as FR-485. Review set: base plus integrity, ears-conformance, evidence, scope-boundary, failure-domain (subset chosen by the dispatching brief; no AssuranceProfile in spec/).

Examined units: FR-485 (examined), FR-485 (examined), FR-485-AC-1 (examined), FR-485-AC-2 (examined), FR-485-AC-3 (examined), FR-485 (examined), spec/spec.md (examined), TC-459 (examined), TC-460 (examined), spec/reviews/ir878/dependency.md (examined).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The upstream owners still allocate these obligations to QSL FR-394, which on main is the enum-key replay facade: QSV FR-109 Dependencies and FR-122 Dependencies, and quire-exact FR-372 (formerly AC-5 note) and FR-380 (formerly AC-6 note) all link ix://agent-ix/quire-spec-language/FR-394. With FR-485 landing alone, every upstream allocation resolves to the wrong requirement and nothing points at FR-485; companion retargeting PRs in quire-semantic-value and quire-exact are required. | FR-485:44-50 |
| FND-002 | low | TC-459 and TC-460 drop their FR-104-AC-9 verifies edges. The change is unrelated to IR-878, quire validate passes on main with the edges present, the tests still carry #[trace("TC-459"/"TC-460", "FR-104-AC-9", ...)] and both TC bodies still list FR-104-AC-9 in Scope. Inherited from #995 without a stated reason; split it out or justify it in the PR. | spec/test-cases/TC-459-s3-checks-configversion-state-clauses.md:1-10 |
| FND-003 | low | The PR's own dependency review (SR-17023) says QSpec FR-323 is referenced only in prose because it is not represented as a local artifact, but FR-485 frontmatter carries a depends_on edge to ix://agent-ix/quire-specification/FR-323 (which resolves; FR-323 publishes the 4096-bit B default). The Dependencies bullet for FR-323 is the only unlinked one. The self-review misstates the artifact. | spec/reviews/ir878/dependency.md:18 |

## Verdict

Request changes. quire validate and make check-spec-validation pass, the hash/digest/pin checklist is clean, the spec.md contains-edge and index row for FR-485 are present and placed alongside the evaluator/replay rows, and all ix:// targets resolve (QSV FR-109/FR-122, quire-exact FR-372/FR-380, quire-specification FR-323 with its 4096-bit default). The blocker is that upstream allocations still name QSL FR-394.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The committed copies of the reviewer SR files are not the reviewer's files. They were renumbered from SR-17028..17033 to SR-17048..17053, which breaks the link to the IR-878 tracker comments keyed on those ids. FR-485 was also search-replaced to FR-486 inside the original Findings rows, so integrity FND-001 now falsely says #1470 adds an FR-486 file. Replace them verbatim with the reviewer's current files (ids SR-17028..17033, original rows untouched, Dispositions appended). | spec/reviews/ir878/pr1472-review-*.md:2-10 |
| FND-005 | low | The PR's own base review (SR-17040) is stale at dbb95abc5. It says all criteria use Test, but FR-486-AC-2 is Inspection. It says TC-459 and TC-460 drop their FR-104-AC-9 links, but those links are now restored. | spec/reviews/ir878/base.md:16-22 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Companion drafts quire-semantic-value#66 and quire-exact#83 retarget all four upstream sites (plus QSV TC-908) to FR-486. Both are open drafts and must merge together with #1472. |
| FND-002 | fixed | Verified in dbb95abc5 against origin/main. |
| FND-003 | fixed | Verified in dbb95abc5 against origin/main. |
| FND-004 | fixed | Round 2: verified in 4bbc6e37a. |
| FND-005 | fixed | Round 2: verified in 4bbc6e37a. |
