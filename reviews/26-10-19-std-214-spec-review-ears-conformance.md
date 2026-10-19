---
id: SR-13324
title: "EARS review of STD-214 FR-169 BadState rule (PR #1048)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: reviews
---
# SR-13324: EARS review of STD-214 (PR #1048)

## Summary

Ticket: STD-214. This pass checks the EARS form of the requirement
statements this PR changes in FR-169: the new `BadState` rule bullet under
**Check**, and the Outputs sentence that lists `BadState` among FR-338's own
rules. The bullet reads as an event-driven clause (the trigger is expanding
a reached closure state, which evaluates a conjunction to undefined or
refused). It describes behaviour, not a change log.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `BadState` bullet puts three requirements in one clause. It states the trigger (reject `BadState`), a prohibition ("SHALL NOT read an undefined or refused conjunction as false or as true") and an acceptance claim ("a certificate that holds such a state is never accepted, whatever its closure and ranks say"). The claim also conflicts with the trigger's "the checker reached" (SR-13322 FND-002). The parenthetical "an operation that could fire there" is circular when the undefined conjunction is that operation's own precondition. Split the bullet into the trigger and the prohibition. Define the trigger as FR-120's expansion of the state recording `ContractUndetermined`, which is the definition FR-338 uses. | spec/functional/FR-169-settle-a-state-graph-verdict-with-its-settlement-method.md:154-160 |

## Verdict

One low finding. Examined and found clean: the Outputs sentence (lines
103-108), which is a declarative statement of the rule set.
