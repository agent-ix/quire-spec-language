---
id: SR-20261103154605
title: "QSL-1522 r1 EARS analysis of PR #2202 (added SHALL statements)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; PR #2202 head 8bf8bbec7; SHALL statements added or edited in FR-505 Behavior, FR-497 Source form and Rational carrier evaluation, FR-509 Behavior"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-505
    type: references
---

# QSL-1522 r1 EARS analysis of PR #2202

## Summary

Tickets: QSL-1522, QSL-1526. The added or edited SHALL statements examined
were these:

- **FR-505:** "The intake reader SHALL bind ... and SHALL read ..."; "When
  ... non-empty `constraints`, the Rational reader SHALL refuse its first
  defect" (event); "The intake reader SHALL refuse a constraint carried by
  any other node ..." (ubiquitous); "The checker SHALL check a member ..."
  and "The checker SHALL refuse a source `Rational[..]` ..."; "Every member
  ... SHALL have exactly that type's bounds and unit"; "FR-106 admission
  SHALL admit ... only if ...".
- **FR-497:** "A conversion into `Quantity<u, Rational[..]>` SHALL compose
  ...".
- **FR-509:** "S3 SHALL refuse ..."; "S4 SHALL emit ..."; "FR-519's package
  reader SHALL admit ..."; "If ... then FR-519's package reader SHALL refuse
  it" (unwanted behaviour); "Exploration ... SHALL follow ..." with its
  "SHALL NOT drop a count ...".

The quire grammar check raises no EARS warning on the changed files.

## Verdict

**CONFORMS** apart from one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-505 Behavior bullet 1 holds three obligations in one item: "The intake reader SHALL bind a value type as Rational by its `scalar`, never by a `target` alias, and SHALL read its bounds from its own `constraints` only. The intake reader SHALL admit ... empty ... as the unbounded `Rational`". Split it into one statement per SHALL: binding by `scalar`, bounds from own constraints, and empty-constraints admission. | spec/functional/FR-505-admit-bounded-rational-value-types-and-refuse-inconsistent-bounds.md:85-88 |
