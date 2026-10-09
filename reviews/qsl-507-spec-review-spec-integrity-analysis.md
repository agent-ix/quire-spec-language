---
id: SR-2451
title: "QSL-507 spec integrity review of PR #669 fix round: FR-325 fairness and capture forms, AC-4 to AC-6, TC-835"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@07631d03dcd2da522baaa2e6c9f3e57fc5995dc0; spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md; spec/test-cases/TC-835-s2-parses-temporal-operators-with-an-optional-interval.md; spec/tests.md (TC-835 row); qsl-forms/src/temporal_clause.rs; qsl-forms/src/syntax.rs; context: spec/functional/FR-123-check-fairness-and-interval-operators-of-infinite-trace-clauses.md, spec/functional/FR-129-check-strong-fairness-constraints.md, spec/decisions/ADR-019-strong-fairness.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-835
    type: reviews
---
# QSL-507 spec integrity review of PR #669 fix round

## Summary

Ticket: QSL-507. PR: quire-spec-language#669. This review was added in disposition pass 1, because the fix round (958a07c5b) changed FR-325 and TC-835. The earlier review pass reviewed no spec text of its own. Scope is the spec text the fix round changed, checked for consistency with the S3 FRs that consume the S2 forms (FR-123, FR-129) and with the code.

Examined, with no finding:

- **FR-325-AC-4.** It now states two units that differ only in profile identity, with equal-length headers, compared by `PartialEq` and `Debug`. TC-835 step 4 and the test agree.
- **FR-325-AC-5 and TC-835 step 5.** Three fairness forms, given by kind, granularity, operation and span. They are consistent with FR-123 "Inputs" (kind, operation name, optional granularity) and with ADR-019 SY-3 (unmarked granularity `whole`, left `None` for S3).
- **FR-325-AC-6 and TC-835 step 6.** Two capture forms (parameter, value, span), with the formula unchanged up to spans. They are consistent with FR-325 "Outputs" and "Behavior", which leave a capture's meaning to S3 (QSpec FR-093).
- **Description and the remaining "Behavior" bullets.** "Parsing selects no meaning". Each interval shape is built as written. Profile independence is now stated for "the same clause text".
- **spec/tests.md.** The TC-835 row lists FR-325-AC-1 to AC-6.

The new "Behavior" statements are plain ubiquitous `SHALL` statements, so a separate EARS analysis was not run. Other spec-review sub-analyses were not run: no domain objects, dependency edges or scope boundaries changed.

## Verdict

One medium finding. FR-325 makes S2 resolve an unwritten fairness kind to `Weak`. FR-129 makes that resolution the S3 checker's job over a form with an optional kind. The two FRs disagree on which stage owns the decision, and the S2 type cannot represent "no kind written". Everything else in the fix-round spec text is consistent, testable, and matched by its TC-835 step and test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-325 "Behavior" says "An unwritten kind SHALL build `Weak`", and FR-325 "Outputs" gives the S2 form `kind: FairnessKind::{Weak, Strong}`, which is not optional. The code matches FR-325: `fairness_form` maps no `strong` token to `FairnessKind::Weak` (temporal_clause.rs:118-122), and `FairnessConstraintForm.kind` is a plain `FairnessKind`. But FR-129 "Inputs" says the parsed fairness constraint forms each carry "an optional kind", and FR-129 "Behavior" says "When a constraint form names no kind, the checker SHALL check it as `FairnessKind::Weak`". So two FRs assign the same resolution (ADR-019 SY-2) to different stages. FR-129's "names no kind" branch can never be reached at S3, so the "no kind" half of FR-129-AC-1 can no longer fail in an S3 test. The S2 choice also contradicts FR-325's own "Parsing selects no meaning" and its own treatment of granularity, which is left `None` for S3 to read as `whole`. And the S2 type `FairnessConstraintForm` is documented "as written" while it carries a resolved kind. Today the outcome is the same, because SY-2 gives an unmarked constraint the identity of `weak`. The defect is spec ownership, not a wrong value. Fix, preferred: build `kind: Option<FairnessKind>` (None when no kind is written), as granularity already does, and reword FR-325 "Outputs", "Behavior" and AC-5 to match FR-129. Alternative: amend FR-129 "Inputs" and "Behavior" to say that S2 resolves the unmarked kind. QSL-508 (T2) owns FR-129, so it could take the FR-129 edit. | spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md:53-56; spec/functional/FR-325-parse-temporal-operators-with-an-optional-interval.md:66-69; spec/functional/FR-129-check-strong-fairness-constraints.md:44-62; qsl-forms/src/temporal_clause.rs:100-122; qsl-forms/src/syntax.rs:2663-2692 |

## Dispositions

Disposition pass 1, reviewed at agent-ix/quire-spec-language@07631d03dcd2da522baaa2e6c9f3e57fc5995dc0.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | still-open | New this round. FR-325 and FR-129 still assign the unmarked fairness kind to different stages. |
