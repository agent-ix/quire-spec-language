---
id: SR-7870
title: "QSL-734 PR 797 variant-type export: evidence"
type: SpecReview
analysis: evidence
scope: "agent-ix/quire-spec-language@0223a174423f74e61ce7314fdf7c18745151701b; FR-056-AC-24..27, TC-988..TC-991"
review_set: subset
---

## Summary

Ticket: QSL-734. Checked that each new AC's TC states an expected value a test can assert: a typed code and cause, the node, and a member path in the TC-235 grammar. Test is the right method for all four ACs. TC-988's expectations are concrete (`cases` `[Gold, Standard]`, keys, no change on reorder). The refusal TCs are not.

## Verdict

Findings: one medium, one low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The refusal expectations in TC-989, TC-990 and TC-991 give loci in prose ("at `Gold`", "naming the name, owner and both spans", "naming the case and `payloadType`", "naming the `typeRef`") and then say "Assertions compare typed code, cause and locus". FR-272's typed `malformed-declaration` and `declaration-form` causes carry the IR node identity, the member path and the origin. They do not carry a case name or two spans. Write each expected refusal as TC-235 does: node `ix://<pkg>/Tier` or `.../variant/Tier-Gold` at `TypeDefinition.variants[1]`, `Variant.payloadType` or `Field.typeRef`. | FR-056-AC-26; TC-989; TC-990; TC-991; QSpec FR-272:65; QSpec TC-235 |
| FND-002 | low | AC-27's clause "With no clause or member naming `Tier`, no refusal is raised" is exercised only by TC-988 step 3. TC-988 scopes FR-056-AC-24, and tests.md maps TC-988 to AC-24 only. Move that step into TC-991, or add AC-27 to TC-988's scope and row. | FR-056-AC-27; TC-988; spec/tests.md:284 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| TC-988 | examined | one `enum` keyed by `Tier`'s identity with `name` `Tier` and `cases` `[Gold, Standard]` ... Step 2 changes no key, export or ordering. Step 3 raises no refusal. |
| TC-989 | examined | Step 1 refuses `unsupported_construct`/`declaration-form` at `Gold`, naming the case and `payloadType` |
| TC-990 | examined | Step 1 refuses the second `Gold` ... naming the name, owner and both spans |
| TC-991 | examined | Step 3 refuses the member `unsupported_construct`/`declaration-form` naming the `typeRef` |
