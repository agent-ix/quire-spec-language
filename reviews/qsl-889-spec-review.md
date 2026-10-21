---
id: SR-12287
title: "Spec review of quire-spec-language PR #1171: FR-336 identity projection and AC-5, TC-846 step 5 (QSL-889)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@96338902801d6e5a2557b94630025898632d6a96; spec/functional/FR-336-bound-a-model-subject-by-its-universes.md (two new Behavior bullets, AC-5); spec/test-cases/TC-846-bound-a-model-subject-by-its-universes.md (scope, step 5)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: reviews
  - target: ix://agent-ix/quire-spec-language/TC-846
    type: reviews
---
# Spec review of quire-spec-language PR #1171

## Summary

Ticket: QSL-889. Base spec-review of the FR-336 and TC-846 edits only, with
EARS, integrity and failure-domain checks folded in. Read against ADR-018
SM-2 (the model subject), ADR-013 O-09 (clause and obligation identity),
ADR-014 §1 (caller budgets) and FR-120 admission.

Checked and clean:

- The "EN-1 request path SHALL write a clause's item ..." bullet is EARS
  ubiquitous and testable, and it states the refusal outcome. Its rule that
  the record comes from the clause's own record in the subject's package
  agrees with FR-336-AC-1 (the record ignores universes).
- Excluding the expansion limits, the meter and the observation limits is
  consistent with ADR-014 §1: a reached ceiling is the caller's budget and
  never changes what a finished run holds.
- FR-336-AC-5 has a Test verification method, points at TC-846, and is
  followed by TC-846 step 5 with an expected result. The TC scope was
  widened to AC-5.
- No research content, no ledger or pin, no vendored text.

## Verdict

FIX. FND-001 blocks. The new identity-projection bullet leaves the checked
package out of the subject's identity, which contradicts ADR-018 SM-2 and
FR-336's own Inputs ("the checked package, initial states, universes and
`ProofBound`s"). It lets two models share one obligation, as SR-12285
FND-001 shows in code. FND-002 to FND-004 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The identity-projection bullet lists "the initial states, each population's universe, the domain packages the subject is admitted against, and the proof bounds". It omits the checked package. ADR-018 SM-2 defines a model subject as "a checked package with a state model, ... initial states, ... universes ... and any `ProofBound`s", and FR-336 Inputs lists the checked package first. An O-09 clause node id covers the clause and the nodes it references, not the package's other contracts and operations, so the occurrence does not stand in for it. Add the checked package's canonical identity to the projection, and a "differs only in the checked package" arm to FR-336-AC-5. | spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:86-92; spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:107 |
| FND-002 | low | "The order and repetition of any of these SHALL NOT change the identity". FR-120 admission refuses a repeated universe key and a repeated population (`conflicting-identity`), so for universes the repetition clause specifies the identity of a subject that never reaches a verdict. Say "order", and limit "repetition" to the initial states, where it is admissible. | spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:89-90 |
| FND-003 | low | The exclusion sentence names the expansion limits, the meter and the observation limits. It does not name the other two subject members that are also absent from the projection: the normalization limits (also a caller budget) and the snapshot provision (bound through each initial state's digest). Name both so the projection is a complete partition of `ModelSimulationRequest`'s members. | spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:90-92 |
| FND-004 | low | "The domain packages the subject is admitted against" could mean the FR-056 provision map (what the code reads) or the model's selected domain packages (what admission checks). Two implementers would read it differently, and the readings split identities differently (SR-12285 FND-002). Say "the domain packages the checked package's model selects, by digest". | spec/functional/FR-336-bound-a-model-subject-by-its-universes.md:88-89 |
