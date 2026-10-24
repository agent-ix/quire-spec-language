---
id: SR-17015
title: "Code review of PR #1453 TC-260 three-member ValueTypeRef union test (QSL-1012 follow-up)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language PR #1453 at 82a57836e; xtask/src/typestate_scan.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-088
    type: reviews
---

## Summary

Ticket: QSL-1012 (follow-up to #1431). Code review with the rust-review lane folded in. The PR's base is 965460aa2. Its only code change is the xtask test `tc_260_value_type_ref_is_the_three_member_union`, renamed from `..._two_member_union`, which now expects `Native(NativeValueType)`, `Package(DeclarationKey)` and `Unit(String)`. No production code changed.

Checks run:

- At the PR head, `cargo test --locked -p xtask --lib typestate_scan` passes 12 of 12.
- Mutation check: deleting the `("Unit", vec![&unit])` row makes the test fail, with left showing three variants and right two. The assertion compares an ordered list of (variant, payload type set) against the scanned enum, so the test is sound against both a missing and an added variant.
- The `#[trace("TC-260", "FR-088-AC-8")]` binding is unchanged and correct.
- The test is checked against current main 1f5903829 by merging it in locally (the merge is clean). The test FAILS there: main's `ValueTypeRef` (qsl-semantics/src/model/domain_package.rs:343-353) gained a fourth variant, `Clock`, in #1451 (9cf7144af), after this PR's base.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The PR does not fix main any more, because it is stale. Since 9cf7144af (#1451), main's `ValueTypeRef` is `{Clock, Native(NativeValueType), Package(DeclarationKey), Unit(String)}`. Merged with origin/main 1f5903829, `tc_260_value_type_ref_is_the_three_member_union` fails: left `[("Clock", []), ("Native", ..), ("Package", ..), ("Unit", ..)]`, right is the PR's three rows. Fix: rebase onto main and expect `("Clock", vec![])` too (the unit variant has no payload), with the test name and doc comment matching. This also depends on the spec decision in SR-17016 FND-001. | xtask/src/typestate_scan.rs:1446-1474; qsl-semantics/src/model/domain_package.rs:343-353 (main) |

## Verdict

FAIL, not mergeable as is. The test change is sound and kills both mutations. But against current main it asserts a three-member union where the code has four, so merging it leaves TC-260 red. Rebase, add `Clock` to the expected list, and get the matching spec decision from the spec owner (SR-17016).
