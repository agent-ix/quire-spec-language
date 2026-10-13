---
id: SR-7867
title: "QSL-734 PR 797 variant-type export: base checklist"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0223a174423f74e61ce7314fdf7c18745151701b; spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/test-cases/TC-988..TC-991, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-734. Base checklist over PR 797: the FR-056 "Variant types" subsection, FR-056-AC-24..27, TC-988..TC-991 and their spec/tests.md rows. Every new AC names one TC, every TC has a `verifies` edge to FR-056 and a tests.md row, and `quire validate` (quire 0.36.2) reports no error on the touched files. The new TC ids collide with an open PR.

## Verdict

Findings: one medium (TC id collision with open draft PR 708), one low (tests.md placement).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-988, TC-989, TC-990 and TC-991 are also added, with different content, by open draft PR 708 (branch spec/670-identity-cost: TC-988 model prefix expansion, TC-989 identity prices, TC-990 replacement cohort, TC-991 full-size identity cost) and cited from its FR-081 and tests.md. TC-992 and TC-993 are taken too. Whichever lands second gets duplicate TC ids. Renumber to TC-994..TC-997. | spec/test-cases/TC-988-admit-a-variant-type.md, spec/tests.md:284-287 |
| FND-002 | low | The four tests.md rows sit inside the FR-302/FR-304 block (after TC-987, before TC-810), away from the other FR-056 rows (TC-442, TC-443, TC-985). | spec/tests.md:284-287 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-056-AC-24 | examined | An admitted package containing variant type `Tier` with cases `Standard` and `Gold` ... one `enum` keyed by `Tier`'s declaration identity ... `cases` `[Gold, Standard]` ... | Test (TC-988) |
| FR-056-AC-25 | examined | A variant type with a case carrying `payloadType` refuses `unsupported_construct`/`declaration-form` at that case ... | Test (TC-989) |
| FR-056-AC-26 | examined | A variant type with two cases named `Gold` refuses `invalid_model_binding`/`malformed-declaration` on the second ... | Test (TC-990) |
| FR-056-AC-27 | examined | A clause naming `Tier` refuses `unsupported_construct`/`declaration-form` at the clause, while the package ... admitted ... | Test (TC-991) |
| spec/tests.md rows TC-988..TC-991 | examined | TC-988 ... FR-056-AC-24; TC-989 ... FR-056-AC-25; TC-990 ... FR-056-AC-26; TC-991 ... FR-056-AC-27 |
