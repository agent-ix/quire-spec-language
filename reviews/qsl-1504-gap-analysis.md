---
id: SR-2026110216377
title: "Gap analysis of the FR-527 mapping classifier (QSL PR #2170)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; spec/functional/FR-527-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md, spec/test-cases/TC-1322-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md, spec/functional/FR-045-classify-temporal-mapping-support.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-527
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1322
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-045
    type: reviews
---
# SR-2026110216377: Gap analysis of the FR-527 mapping classifier (QSL PR #2170)

## Summary

Ticket: QSL-1504. Plan completion: not assessed. PR #2170 at 6f037fe08.

Every FR-527 acceptance criterion maps to a test or doctest with value assertions. AC-1, 2, 4, 6, 7 share the 24-case table, which I read cell by cell against FR-045's eight rows: infinite trace, finite window and mixed direction accumulate; past-only under either false-extension profile gives `mltl.origin-complete-history/v1` under both closures; event-position future gives closed-trace or online-prefix by closure with no valuation; fixed-sample future gives the same two with `total_sample_valuation` true; no bounded operator gives the same two targets with no valuation. AC-3 covers `once`, `historically`, `since`, `triggered` and `once[1,1]` under both profiles and closures. AC-15 runs over the three packages of R through the checked-handoff tree and asserts the expected support per kind. AC-18 runs `resolve_profiles` over the refused and admitted identities, timed included. AC-24 is `#[ignore]`d and its helper is `unimplemented!`, so no running test reaches it.

`quire matrix --scope .` reports AC-11 and AC-14 as `untagged`: both are compile-fail doctests, which cannot carry `#[trace]`. AC-20 is Inspection (`method-without-symbol`); the classifier body reads only the view's `profile()`, `formula()`, `declaration()`, `activation()`, `package().name()` and `surrounding`. AC-24 is `tagged-by-ignored-test`, as FR-515-AC-13 and AC-14 are.

Legacy `src/temporal/mapping.rs` is unchanged (`git diff origin/main...HEAD -- src/temporal` is empty). No `#[allow]`, no `unwrap` on a production path, no pins.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-527-AC-11 and FR-527-AC-14 show as `untagged` in the computed matrix, so TC-1322 rows 11 and 14 have no binder. The proofs exist (compile-fail doctests, verified sensitive by mutation), but a doctest takes no trace tag. The repo's precedent for a compile-fail AC is a tagged `#[test]` that reads the doctest source and asserts the doctest and its error code are present (FR-515-AC-9 in `tests/it/retired_checked_views_paths.rs`, FR-515-AC-8 in `qsl-package/tests/checked_views_surface.rs`). Add the same for AC-11 and AC-14, tagged `FR-527-AC-11`/`FR-527-AC-14` and `TC-1322`. | qsl-package/src/checked_views/mapping.rs:121-148, qsl-package/src/checked_views/mapping.rs:243-253, spec/test-cases/TC-1322-classify-native-to-tl-mapping-support-from-the-checked-temporal-clause-node.md:27 |

## Verdict

One medium coverage finding (matrix binders for AC-11 and AC-14). Everything else examined is covered and value-asserting.

