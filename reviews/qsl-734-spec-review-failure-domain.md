---
id: SR-7869
title: "QSL-734 PR 797 variant-type export: failure domain"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@0223a174423f74e61ce7314fdf7c18745151701b; FR-056 Variant types, AC-24..27, TC-988..TC-991"
review_set: subset
---

## Summary

Ticket: QSL-734. Checked whether each new refusal is reachable at intake, given that FR-056 ends intake on a reader refusal before declaration binding. The FCD extraction frontend and the `agent-ix-semantic-ir` reader (filament-core-data b3defd7) refuse several of the cases first.

## Verdict

Findings: one medium, two low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Two of AC-26's cases are refused before intake. (1) Two cases named `Gold`: the lift raises DUPLICATE_IDENTITY or UNSLUGGABLE_NAME (FCD FR-095 collision rule; two enumeration rows slugging alike), so there are no bytes, and a hand-authored document that repeats the identity is refused by the reader's `duplicate_identities` (semantic-ir rules.rs:1133). (2) A case with no name: the schema's `variant` requires `name` with minLength 1, so the reader refuses it. The FR-056 refusal is reachable only from a hand-authored document whose duplicate cases carry distinct identities. TC-990 steps 1 and 3 say "Lift", which cannot produce the expected `malformed-declaration`. | FR-056-AC-26; TC-990; FCD semantic-ir.schema.json `variant`; FCD FR-095 |
| FND-002 | low | TC-989 says "Lift `Tier` ... where `Gold` carries `payloadType`". FCD FR-093 lowers enumeration `Values` rows to variants with no `payloadType`, so the lift never emits one. The test needs a hand-authored document, where the reader also requires `payloadType` to resolve (UNRESOLVED_VARIANT_PAYLOAD). | TC-989; FCD FR-093:89 |
| FND-003 | low | "Case names compare as exact UTF-8 byte sequences" suggests that any two byte-distinct names are distinct cases. Names that slug alike (`Snap Shot`, `Snap_Shot`) mint one identity and never reach intake as two cases. Also unstated: a construct with meaning variant-type whose shape is not `enumeration`. Presumably FR-154 row 3, `malformed-declaration`, applies. | FR-056:435-442 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-056-AC-25 | examined | A variant type with a case carrying `payloadType` refuses `unsupported_construct`/`declaration-form` at that case |
| FR-056-AC-26 | examined | A variant type with no case, and a case with no name or no span, each refuse `invalid_model_binding`/`malformed-declaration` |
| FR-056 reader-first rule | context_only | If `agent-ix-semantic-ir` refuses the selected bytes, then the compiler SHALL end intake with no declaration and SHALL retain every reader diagnostic |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | dcfafda2f |
| FND-002 | fixed | dcfafda2f |
| FND-003 | fixed | dcfafda2f |
