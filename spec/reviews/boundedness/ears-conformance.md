---
id: SR-630
title: "EARS conformance review of ADR-014 and its FR-057 and FR-082 amendments"
type: SpecReview
analysis: ears-conformance
scope: "spec/decisions/ADR-014-temporal-trace-and-boundedness-architecture.md; the FR-082 and FR-057 statements changed against origin/main"
review_set: all
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: reviews
---
# SR-630: EARS conformance review of ADR-014 and its FR amendments

## Summary

Reviewed commit `bbe92fec` (branch `spec/17-boundedness-adr`). `quire validate
--summary` over FR-082, FR-057 and ADR-014 reports 3/3 docs grammar-clean with
0 findings, so every finding below is semantic. The FR-057 row is clean. The
dominant defect is in FR-082: the new stage-dependent SHALL paragraph (lines
128-137) sits after an unamended SHALL (lines 115-125) that still states
`resource_exhausted` and `Refused` for every walk, so the two contradict for
check-stage walks. ADR-014's rules are mostly one-subject, one-response
statements. Three of them are ambiguous enough to change what gets built.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-082 "Ancestor and conformance walks are bounded" (lines 115-125) still says the model checker SHALL refuse with the resource-exhaustion cause (`ancestor-steps`), and that reaching the ceiling "is a `Refused` outcome". The new paragraph (lines 128-137) says a model-normalization walk SHALL return `StageFailure::Limit(LimitExceeded)`. Both statements apply to the same check-stage walk. Fix: scope lines 115-125 to name the ceiling and forbid a verdict only, and move the refusal code entirely into the stage-dependent paragraph. Also reword "a `Refused` outcome, never `Incomplete`" to "a refusal (stage failure at check time, `Refused` in S6a), never `Incomplete`". | FR-082 |
| FND-002 | medium | FR-082 new paragraph: "It SHALL return …" and "It SHALL refuse …" take "a walk" as the subject. A walk is not the system that responds. Fix: name the responder, e.g. "When a check-stage walk would exceed the ceiling, the model checker SHALL return …" and "When an S6a population-admission walk would exceed the ceiling, the population admission SHALL refuse …". This is also the EARS `When` form the trigger needs. | FR-082 |
| FND-003 | medium | FR-082 new paragraph assigns limit kind work budget "for `work_units`" to model-normalization walks too, but FR-082 defines a `work_units` budget only for expression-checker admission; model normalization's charge points yield `Incomplete` (line 121). Fix: state that `work_units` → work budget applies to type-environment admission only. | FR-082 |
| FND-004 | medium | FR-082 Outputs (line 48) lists only an admitted result, `Refused` and a typed incomplete result. The amended SHALLs and AC-6/AC-7 now produce `StageFailure::Limit(LimitExceeded)`, which is none of these. Fix: add the stage failure to Outputs. | FR-082 |
| FND-005 | medium | FR-082-AC-3 packs two stage outcomes into one criterion, and its subject "the checker" does not perform S6a population admission. One TC (TC-220) cannot show which outcome it observed. Fix: split into AC-3a (check stage: `LimitExceeded`, nesting depth) and AC-3b (S6a population admission: `resource_exhausted`/`ancestor-steps`), each with its own subject and TC. | FR-082 |
| FND-006 | medium | ADR-014 §1 classification rule classes S6a limits as B-2 by "a budget that S6a evaluation charges", but FR-082 says `ancestor_steps` "is read, never charged". By its own words the rule does not classify the S6a `ancestor_steps` ceiling, yet the ADR asserts it is B-2. The Alternatives entry also rejects giving "one ceiling two codes depending on the caller", which is what §1 and §12 then do for `ancestor_steps`. Fix: restate the rule by reader ("a ceiling or budget read or charged during S6a evaluation is B-2"), and reword the Alternatives rationale to "two codes within one stage". | ADR-014 |
| FND-007 | medium | ADR-014 §4 predicate lets a caller supply a `ProofBound` "for every unbounded domain", and §4's extent rule lists an infinite-trace temporal formula as such a domain. §10 scenario 1 then says "a temporal formula has no `ProofBound` (§4)", but §4 contains no such rule. Implementers of QSL-140/QSL-43 cannot tell whether a temporal `ProofBound` must be refused. Fix: add to §4 "A temporal formula under the infinite-trace profile admits no `ProofBound`; for it the predicate is false", or drop the claim from scenario 1. | ADR-014 |
| FND-008 | medium | ADR-014 A-4 says S6a "never produces `proved` or a true settlement for an infinite-trace formula", then says that over a lasso "it evaluates exactly", which can yield true. Fix: state the response for a true lasso evaluation explicitly, e.g. "Over a lasso it evaluates exactly and reports true as `tested` (§8), never as a settlement of the claim". | ADR-014 |
| FND-009 | low | ADR-014 §2 loop-maximum row: "An unproved termination claim is refused or inconclusive" gives two responses and no condition that selects between them. Fix: name the condition, e.g. refused at S3 when no variant is declared, inconclusive when a declared variant is not proved. | ADR-014 |
| FND-010 | low | ADR-014 TR-2 ("refuses at reconstruction"), TR-3 ("refuses `lower > upper`", "refuses `Unbounded`") and TR-4 ("Overflow … refuses at check") give no responder's cause or catalog code, unlike A-2 and §6 step 4. Fix: name the cause for each, or cite the FR that will. | ADR-014 |
