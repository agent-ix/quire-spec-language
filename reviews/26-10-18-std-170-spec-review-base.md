---
id: SR-12882
title: "Base review of PR #978: FR-138, ADR-020 and TC-543 reference the QSpec history-literal"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ef1b1afa0; FR-138, ADR-020 grammar sketch and AX-1, TC-543; agent-ix/quire-specification@origin/main FR-375 history-literal, FR-375-AC-9, TC-310 mutation 10, FR-378, FR-149"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-543
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-020
    type: reviews
---

## Summary

Ticket: STD-170. PR quire-spec-language#978. The PR swaps `<literal>` for
QSpec FR-375's `history-literal` in FR-138 Inputs and ADR-020's grammar
sketch and AX-1. It states the value-set conformance rule in FR-138's S3
Behavior, adds the out-of-range literal `5` to FR-138-AC-2, and adds that
literal to TC-543 step 2. It also scopes the "native `Integer`, not range"
fixture paragraph to computed update expressions. This base review checks
the checklist, AC to TC coverage and the TC-543 oracle against merged QSpec
FR-375 (origin/main 967ee5a).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | TC-543's Step 2 expected result checks only the code and subcode. FR-138-AC-2 now says the literal refusals happen "at the literal". An implementation that refuses `5` at the row or at the declared type, not at the literal span, would still pass. | spec/test-cases/TC-543-history-fields-check-and-compute-without-changing-the-concrete-model.md:37; spec/functional/FR-138-check-and-compute-history-fields.md:132 |
| FND-002 | low | The PR adds S3 SHALLs that `none` conforms only to `Option<T>` and that an enum member is its three-segment path, but no FR-138 AC or TC-543 step exercises either. Only the `true` and `5` refusals are covered. Today the coverage depends on QSL-866's tests against QSpec FR-375-AC-9. | spec/functional/FR-138-check-and-compute-history-fields.md:76-78 |
| FND-003 | low | Line 124 runs to 98 columns. The rest of the prose wraps at 80. | spec/functional/FR-138-check-and-compute-history-fields.md:124 |

## Verdict

Mergeable. No high or medium findings. All three findings are lows.

The decisions the author made:

- **Fixture paragraph kept, scoped to update expressions: coherent.** QSpec
  FR-375's history-literal paragraph separates a literal (statically known,
  decided at the literal) from FR-149's unproved-range case for computed
  expressions. QSpec FR-378-AC-3 settles an update whose value leaves the
  declared type as `inconclusive`/`mapping-undetermined` at a reachable
  state. FR-138's Layer 5 bullet (`MappingUndetermined`) and FR-138-AC-4
  (`pre(self.writes) + 1`) carry that through unchanged. The rescoped
  paragraph now states the same split as FR-375.
- **Out-of-range literal folded into FR-138-AC-2: fine.** The repository has
  no Rust test tagged `TC-543` or `FR-138-AC-*`, so widening AC-2 cannot
  leave an existing tagged test claiming a variant it never runs. A new AC id
  would add trace churn and close no gap.
- **TC-543 oracle strength: adequate.** A checker that compares only the
  native kind admits `5` for `Int[0, 1]`, so Step 2 fails when the range is
  ignored. `true` covers the wrong-kind case.
- **Overlap with #974 (QSL-557) and #970 (QSL-543): none.** Both are
  code-only (qsl-cst, qsl-forms, qsl-semantics, qsl-replay). Neither touches
  FR-138, ADR-020 or TC-543, and neither parses history rows. #974's body
  says history rows are not parsed, and QSL-866 owns that work. Both PRs
  add a refinement production to `qsl-cst/src/grammar.rs`. That conflict is
  between those two PRs, not with #978.
- **Ids:** no new ids. TC-543 still verifies FR-138-AC-1 to AC-4, and
  FR-138-AC-5 stays on TC-554.
- **Checklist:** `quire validate` exits 0 on the three changed files. There
  are no hashes, digests, pins or tracking records.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | FR-138 References calls FR-375-AC-9 "the covering test". FR-375-AC-9 is an acceptance criterion. The covering test is QSpec TC-310, mutation 10. | spec/functional/FR-138-check-and-compute-history-fields.md:145-147 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | TC-543 Step 2 now asserts that `true` and `5` refuse at the literal's span (2c0bd426d). |
| FND-002 | fixed | FR-138 no longer states its own `none` and enum-path SHALLs. Its S3 bullet defers to FR-375's history-literal rule, and References names FR-375-AC-9 as the coverage for the `none`, `true`/`false` and enum-member cases. No FR-138 rule is left without a test. QSL-866 implements against FR-375-AC-9 (2c0bd426d). |
| FND-003 | fixed | The fixture paragraph is rewrapped to 80 columns (2c0bd426d). |
| FND-004 | fixed | FR-138 References now names QSpec TC-310 mutation 10 (FR-375-AC-9) as the covering test (a08bae161). |
