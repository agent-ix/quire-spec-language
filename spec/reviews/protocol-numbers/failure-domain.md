---
id: SR-309
title: "Failure domains of exact protocol numbers"
type: SpecReview
analysis: failure-domain
scope: "FR-038; TC-117; protocol_artifact numeric component"
review_set: all
---
## Summary

PASS. Numeric identity, refusal, purity and termination rules are complete for
this component; the tests exercise the actual public constructors and decoder.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No missing failure-domain rule found. | FR-038-AC-1–5; TC-117 |

Integer/rational tags stay distinct; canonical decimal checks, positive
denominators and gcd refuse alternate or unreduced values. unsigned_abs avoids
signed-minimum overflow. Euclid's divisor decreases; conversion has no I/O or
ambient state. Typed numeric errors survive NumberWire conversion; shape failures
propagate through Serde. Tests include independent positive controls and malformed
fields. The enclosing reader owns input budgets, as NumberWire documents.
