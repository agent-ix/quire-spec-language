---
id: SR-16277
title: "gap analysis of QSL-1014 checked_v2 reader admits the hybrid node tag (QSL PR #1381)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@89ebc48e365ad269bcb223ccde3b00ff474effd0; qsl-semantics/src/library/package_identity.rs, qsl-package/src/checked_v2/tests.rs, qsl-package/src/emit/tests/admission_corpus.rs; oracle FR-447-AC-13..16 at QSL #1365 head"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-1187
    type: reviews
---
# SR-16277: gap analysis of QSL-1014 checked_v2 reader admits the hybrid node tag (QSL PR #1381)

## Summary

Ticket: QSL-1014. Planless gap analysis over `git diff origin/main...89ebc48e3`. Plan completion: not
assessed. The FR-447-AC-13..16 text comes from QSL #1365 head, which stacks on #1364. At the PR
head, `spec/functional/FR-447-...md` has no AC-13..16, and `quoin matrix --json` (quoin 0.28.3)
does not list FR-447-AC-13. The new `FR-447-AC-13` trace tags only bind once QSL #1364 lands.

AC to test, as measured:

- FR-447-AC-13, the admission of QSpec `positive-all-families.json` (Tank's three `hybrid` nodes):
  `conformance_hybrid_nodes_are_admitted_and_a_swapped_model_body_is_refused` asserts a `Verified`
  read. It fails with `NodeTag` at nodes/39 when `hybrid` is removed from `NODE_TAGS`.
- FR-447-AC-13, "a model body with `modes` before `variables` refuses `malformed_wire`": the same
  test asserts the code.
- FR-447-AC-13, the read-back of AC-5's `CheckedHybridModel`/`CheckedHybridClaim` equal to the
  checked ones: not testable. QSL has no hybrid checked types and no lowering or emission. The PR
  body says so.
- FR-447-AC-14, AC-15 and AC-16: all of them need emitted hybrid packages, so none is testable
  here.

The PR body's "Not in this PR" section is accurate for AC-13's read-back clause and for AC-14 and
AC-15. FR-447-AC-1..5 is out of scope, as the dispatch brief records.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No ticket owns the parts of FR-447-AC-13..16 this PR leaves out. The body sends them to "QSL-900 / FR-447-AC-1..5", but QSL-900's description is "E22-H remainder: FR-250-AC-1 hybrid classification and provider routing" and names no FR-447 AC. QSL-1014's title claims "FR-447-AC-13..16", so closing it on merge reads as all four done. The body never mentions AC-16. The AC-13 refusal "every variable classed `clock` ... at `v`'s class literal" names no model and is not tied to AC-5's package, so it may be testable now against Tank in `positive-all-families.json`, re-keyed the same way. Record the remainder (AC-13 read-back and the remaining refusals, AC-14, AC-15, AC-16) on a ticket QSL-900 depends on or that blocks it, and narrow QSL-1014's claim to what landed. | qsl-package/src/checked_v2/tests.rs:1738-1780 |
| FND-002 | low | Merge order: the new `FR-447-AC-13` trace tags (`qsl-package/src/checked_v2/tests.rs:1743`, `qsl-semantics/src/library/package_identity.rs:648`) name a criterion that exists only in QSL #1364. The body names QSpec #414/#415 and IR #528 as merge partners, but not QSL #1364/#1365. If this lands first, the tags bind to nothing and the matrix drops them without an error. | qsl-package/src/checked_v2/tests.rs:1743, qsl-semantics/src/library/package_identity.rs:648 |

## Verdict

Two findings: one medium tracking gap and one low merge-order note. The code under review has an
owning requirement, every new test backs a clause of FR-447-AC-13 or QSpec FR-322-AC-13, and no new
test is vacuous or a stub. The trace mis-binding of the unit test to `QSpec-FR-481-AC-1` is
recorded once, as SR-16276 FND-001.
