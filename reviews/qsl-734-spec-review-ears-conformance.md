---
id: SR-7871
title: "QSL-734 PR 797 variant-type export: EARS conformance"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@0223a174423f74e61ce7314fdf7c18745151701b; FR-056 Variant types subsection"
review_set: subset
---

## Summary

Ticket: QSL-734. EARS form of the new FR-056 statements. The admission statement uses event-driven form ("When ..., intake SHALL admit"). The four refusals use unwanted-behaviour form ("If ..., then intake SHALL refuse"). Three binding rules are written as plain description with no SHALL.

## Verdict

One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | These sentences are normative but have no SHALL: "Case names compare as exact UTF-8 byte sequences"; "reordering the `variants` list in the document changes no key, export or ordering"; and the case node identity sentence. AC-24 tests the first two. Make them ubiquitous SHALL statements, for example "The compiler SHALL compare case names as exact UTF-8 byte sequences". | FR-056:435-442 |

## Examined obligations

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-056 Variant types admission | examined | When a type definition binds to `quire.meaning.model.variant-type/v1`, intake SHALL admit it as one original declaration and the compiler SHALL derive these export records |
| FR-056 payload refusal | examined | If a case has a `payloadType`, then intake SHALL refuse the case with `unsupported_construct`/`declaration-form` |
| FR-056 duplicate/empty refusal | examined | If two cases of one variant type have the same `name`, then intake SHALL refuse the second |
| FR-056 member typeRef refusal | examined | If a member's `typeRef` names a declaration bound to `quire.meaning.model.variant-type/v1`, then intake SHALL refuse the member |
| FR-056 clause refusal | examined | If a clause names a declaration bound to `quire.meaning.model.variant-type/v1`, then the compiler SHALL refuse the clause |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | dcfafda2f |
