---
id: TC-1330
title: "Native composition carries and checks premises"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-530
    type: verifies
---
# TC-1330: Native composition carries and checks premises

## Description

Verify QSL's native composition request, discharge record and rejections
through real native component packages and independently checked premise records.

## Test Procedure

Run QSpec TC-983's six cases through compile, read-back, negotiation and result
checking over real native `Producer` and `Consumer` packages. Add adversarial
identity, assumption and fairness cases: a swapped component identity, an
assumption relabelled as a guarantee and a fairness kind strengthened by one
step. Record in the test output which behaviour is existing coverage (TL-9
algorithm, FR-348 negotiation, FR-380 transfer) and which is newly delivered.
Tag the tests `#[trace("TC-1330", "<AC id>")]`.

## Expected Results

QSpec TC-983's expected results; each adversarial case is rejected or settles
`premise-mismatch` and the unrelated claim of the request settles.
