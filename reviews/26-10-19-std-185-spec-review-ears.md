---
id: SR-12905
title: "EARS conformance review of PR #999: new FR-337 and FR-338 behavior statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@546412bb9; FR-337 Behavior: sole-source identity rule, over-reference rule, I2 reader identity rule; FR-338 Behavior: refused-key rule"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-338
    type: reviews
---

## Summary

Ticket: STD-185. PR quire-spec-language#999. Checks the four new `SHALL`
statements against EARS grammar: one system, one response, and an explicit
trigger for unwanted behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "The emitter SHALL emit the clause node as the only source of the clause's identity: a consumer resolves ... No consumer supplies either from outside the package." The emitter is the subject, but most of the statement constrains consumers, which are other systems. Split it: one emitter rule (the node carries the identity and `over` argument), plus a consumer rule stated where the consumer is specified (FR-338's resolution). | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:87-92 |
| FND-002 | low | "The I2 reader SHALL return ... and SHALL refuse a lookup whose node is absent, ..." joins a ubiquitous response and an unwanted-behaviour response in one statement. The refusal should be its own `If <the node is absent, not a temporal clause node, or its occurrence role or ordinal differs>, then the I2 reader SHALL refuse ...` statement. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:97-99 |
| FND-003 | low | The refused-key rule has three SHALL responses, plus a settlement claim that belongs to another system ("FR-127 settles it `inconclusive` ...") and a cardinality note. As EARS unwanted behaviour it reads: `If a certificate state's AutomatonStateKey fails admission, then the checker SHALL return Rejected{ShapeMismatch} at ProductState(that state) and SHALL NOT use the key ...`. The FR-127 consequence belongs in a note or in FR-127, where V-6 already states it. | spec/functional/FR-338-check-an-en-1-closure-certificate.md:562-569 |

## Verdict

The over-reference rule ("The emitter SHALL write the `over` argument as a
reference to the parameter node, never as a value") is a conformant
ubiquitous statement; its trailing clause is rationale. The three findings
are about form only and do not change behaviour.
