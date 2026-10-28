---
id: SR-17182
title: "QSL-1072 PR #1523 EARS conformance review of FR-487 and the FR-315 amendment"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-487-lower-hyper-atoms-and-match-conjuncts-to-checked-nodes.md, spec/functional/FR-315-encode-the-smt-lib-transition-relation.md (Joint steps, Property, Product identity)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-487
    type: reviews
---

## Summary

Ticket: QSL-1072. PR: quire-spec-language#1523 (head <sha removed>). This lens checks that FR-487's statements, and the three amended FR-315 statements, follow EARS form. `quire validate` emits no `ears:*` warning for FR-487, TC-1264 or FR-315. A manual read found each Behavior bullet ubiquitous (`The S3 hyper check SHALL ...`, `evaluate_hyper_holds SHALL ...`) or unwanted-behaviour (`If ..., then ... SHALL ...`), each with a named subject and response.

Examined units: FR-487 (statement and every Behavior bullet), plus FR-315 Joint steps, Property and Product-identity refusal order (amended text). All were examined.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "Every other implementation of `StepMatch`, test seams included, SHALL return the same shapes" makes other code, test code included, the subject, and "the same shapes" is not a defined response. State it as a constraint on the trait: `StepMatch` SHALL return `MatchReading`/`MatchIncomplete` with `conjunct` as defined above. | FR-487:269-270 |
| FND-002 | low | In "Reading the lowered nodes in an SMT query", the subject is "the reader", a term FR-487 never defines (FR-315's `Reader` is meant). Name `encode_hyper_query` (FR-315), or FR-315's expression reader, as the subject. | FR-487:280-285 |

## Verdict

Conforms, with two low wording findings.

## Dispositions

Round 1, head <sha removed> (fix commit <sha removed>).

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Now "`StepMatch::universal` and `StepMatch::existential` SHALL return `Result<MatchReading, MatchIncomplete>` with these shapes ..." |
| FND-002 | fixed | The subject is now "FR-315's `encode_hyper_query` SHALL read these nodes", and atoms are read "through FR-315's expression reader (`Reader::boolean`)". |
