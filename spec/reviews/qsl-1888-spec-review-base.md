---
id: SR-599004
title: "Base review of QSL-1888 join outstanding rule and repeat proof rulings"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md, spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md, spec/test-cases/TC-663-s3-checks-protocol-control-constructs.md (PR #2682 diff, head b2f26c872)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: reviews
---
# SR-599004: Base review of QSL-1888 join outstanding rule and repeat proof rulings

## Summary

Ticket: QSL-1888. Base checklist review (statement quality, AC testability,
atomicity) of the two new statements and two new ACs in PR #2682. The FR-390
amendment removes the "S3 discharges the proof" contradiction with FR-218 and
reads correctly. The FR-218 join refusal names its condition, code and locus.

The problem is the termination claim. Both new ACs require that "a
termination claim over the `repeat` settles inconclusive", but no QSL
requirement defines a termination claim, its request or its settlement, and no
QSL crate evaluates one. TC-663 is an S3 checking test and cannot exercise it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-390-AC-74's clause "a termination claim over the `repeat` settles inconclusive" cannot be tested in this repo. No QSL FR defines a termination claim, its request or its outcome; grep finds the phrase only in FR-218 and FR-390's new text; no qsl-eval or qsl-analyze code reads `proof.invariant`/`proof.variant`. TC-663 runs S3 only. A test tagged FR-390-AC-74 can check "no slot" and "both recorded", but it must either skip the settlement clause or call a reader that does not exist. "S3 records both without discharging them" is an absence claim with no observable (no proof-result field is named). Fix: name the component and FR that owns termination-claim settlement (in QSL or the downstream repo), and either move the settlement clause to that owner's AC or mark it as waiting on it; restate "without discharging" as an observable, for example "the checked `repeat` carries no proof result". | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md:989 · spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:183-186 |
| FND-002 | medium | FR-218-AC-40 is compound and repeats FR-390-AC-74. It joins the G3b join refusal with the G5 repeat rule, two unrelated rulings, and its second sentence carries the same untestable termination-claim clause as FR-390-AC-74 (SR-599004 FND-001). One failing half fails the whole AC. Fix: keep AC-40 to the join refusal, and keep the repeat rule in FR-390-AC-74 alone, or as its own FR-218 AC. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:638 |
| FND-003 | low | "the stepper (FR-206) SHALL assume every non-`all` join has an `outstanding` rule" puts a SHALL on FR-206's subject inside FR-218, and "assume" is not an observable behaviour. FR-206 is not amended. Fix: state it as a consequence ("FR-206 therefore never meets such a join") or move it to FR-206 as a guarantee its stepper may rely on. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:212-214 |
| FND-004 | low | FR-218-AC-40 writes `join predicate(f) [a]` without defining `f`. With `f` undeclared, "the same joins with `outstanding cancel` or `outstanding continue` check" fails on `missing_declaration` instead. Fix: name the predicate's signature, for example "`f` a pure function of one Boolean", as FR-206-AC-11 does. | spec/functional/FR-218-check-every-protocol-control-construct-at-s3.md:638 |

## Verdict

REVISE. No high finding in this method; FND-001 and FND-002 are medium and
should be fixed in this PR, together with SR-599003. The FR-390 wording change
itself is correct and resolves the G5 contradiction it was filed for.
