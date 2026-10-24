---
id: SR-8704
title: "Gap analysis of quire-spec-language PR #804: FR-056-AC-29..31, FR-081-AC-16"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2dff29e41112bae401b988287d5a4db06e9fd632; FR-056-AC-29, FR-056-AC-30, FR-056-AC-31, FR-081-AC-16, TC-1003, TC-1004, QSL-740 ACs 1-3; qsl-semantics/tests/it/variant_type.rs and the production code it exercises"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-056
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-081
    type: reviews
---
# Gap analysis of quire-spec-language PR #804

## Summary

Ticket: QSL-740. Plan completion: not assessed. The criteria were mapped to
tests with `quire matrix --scope . --format tsv` (quire 0.36.2, engine 0.50.2)
at the reviewed sha. FR-056-AC-29 has three tagged tests (variant_type.rs:807,
844, 860), FR-056-AC-30 has two (877, 905), FR-056-AC-31 has one (943), and
FR-081-AC-16 is **untagged**.

QSL-740's acceptance criteria (ticket text, read as data). AC 1, spec first:
met by FR-056 "Effective identity of a variant type" and FR-081. AC 2,
normalization identities, assembler declarations, and vectors including a
same-name conflict: met (TC-1003 step 4 and TC-1004). AC 3, the mis-minted
case names the later case: met as a test of the reader's behaviour. The
ticket's wording ("the later case node, the node that causes the conflict")
is only true in one list order; the spec-review SR covers this.

## Verdict

**Findings: one medium, one low.** No test is a stub. Every new test calls
production code, and each would fail if the behaviour it names were reverted.
The exception is the TC-1003 step 5 test, which checks the reader's existing
behaviour and does not depend on this PR's code. That is correct for a
statement about the reader.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-081-AC-16 has no tagged test. The matrix reports it `untagged`, although its evidence cell says `Test (TC-1003)` and tests.md lists it under TC-1003. The behaviour is exercised by `a_variant_type_and_its_cases_have_effective_identities` and `reordering_the_cases_changes_no_effective_identity`, but both carry `#[trace("TC-1003", "FR-056-AC-29")]` only. Add `FR-081-AC-16` to those two tags. | spec/functional/FR-081-preserve-model-correspondence-and-declaration-identity.md:170; qsl-semantics/tests/it/variant_type.rs:805; qsl-semantics/tests/it/variant_type.rs:842 |
| FND-002 | low | The last sentence of FR-056-AC-31 and TC-1004 step 3 ("one that names one still refuses `unsupported_construct`/`declaration-form`") are not asserted by the TC-1004 test. The behaviour is tested by `a_clause_naming_a_variant_type_refuses_while_the_declaration_is_admitted`, but that test is tagged TC-997/FR-056-AC-27 only. Add `FR-056-AC-31` to that tag, or assert the refusal in the TC-1004 test. | spec/test-cases/TC-1004-variant-type-in-the-type-environment.md:21; qsl-semantics/tests/it/variant_type.rs:531; qsl-semantics/tests/it/variant_type.rs:943 |

## Dispositions

Round 1, reviewed at `f9d73c0b22`.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | f9d73c0b2: `a_variant_type_and_its_cases_have_effective_identities` and `reordering_the_cases_changes_no_effective_identity` carry `#[trace("TC-1003", "FR-056-AC-29", "FR-081-AC-16")]`. The multi-id tag form is already used in model_intake.rs. |
| FND-002 | fixed | f9d73c0b2: `a_clause_naming_a_variant_type_refuses_while_the_declaration_is_admitted` is tagged `FR-056-AC-31`. It does assert the source-naming refusal: one error, `AssemblyCause::UnsupportedVariantType { name: "Config::Tier" }`, and `catalog_code() == "unsupported_construct/declaration-form"`. |
