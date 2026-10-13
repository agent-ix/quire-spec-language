---
id: SR-9032
title: "QSL-726 spec review of PR #816 FR-302 and tests.md edits"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3e004e2039af35d8fa340899aca3ed4852748fda; base impl/business-model-meanings (now includes #811, #813); spec/functional/FR-302-link-dispatch-during-unit-compile.md; spec/tests.md; context: spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md; spec/test-cases/TC-793, TC-795, TC-1011"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: reviews
---
## Summary

Ticket: QSL-726, AC-3. This is a base spec review of the PR's two spec edits: the new sentences in FR-302's "Every called family links at S3" section, and three 🚧→✅ flips in `spec/tests.md` (TC-793, TC-795 and TC-1011). FR-096 was read as context because the PR's E4 fix changes how regions resolve.

The coder's report says both "FR-302 gets one sentence" and "the E4 fix changed nothing in the spec". The first is a minor miscount: the PR adds two sentences. The second is wrong (FND-002).

## Verdict

**FAIL.** The new FR-302 sentence makes the over-broad linking in SR-9030 FND-002 part of the spec. FR-096 is contradicted and the PR does not amend it.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new sentence "A called member name selects the family root of that name in each selected model" defines the called family by name alone. It contradicts the requirement's opening: S3 SHALL produce a checked package "only when every called family links", for "each distinct operation a dispatched call in the unit names". Under the new sentence, an unrelated same-named family that no call reaches counts as called, and its link failure refuses the compile. State the rule by the call: the family whose table the call's receiver static type names. Leave the name-based pre-pass to the implementation. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:54-58 |
| FND-002 | medium | FR-096's list of the "exactly four cases" with no region has a first case: "a position in a function that the `model` bridge synthesized for FR-151 dispatch ... built from a domain-package declaration". The PR breaks this case. Bridge functions are now built from unit declarations. A body function carries its `operation-body` spans. A precondition function resolves to its unit `pre` clauses through `synthesized_spans`. Every clause function gets a `generated` occurrence. The case is now false for the production path, and nothing states where a synthesized precondition function is located. Amend FR-096: narrow case 1 to functions built from domain-package declarations, state the placement rule for body functions and precondition functions, and add an AC and TC for it. | spec/functional/FR-096-stage-limits-refusal-records-and-readers-carry-a-locus.md:113-127 |
| FND-003 | low | The PR flips TC-795 to ✅. On the merged ref, TC-795 still ends with an interim paragraph: "Until unit compile links dispatch ... the bridge's own test backs the charge record ... The unit-level steps become the test once compile links dispatch." After this PR that condition is met, so the paragraph is stale, and it contradicts the ✅. Delete it in the same change that rewrites the test to the `insufficient-next-charge` record (SR-9031 FND-001). | spec/test-cases/TC-795-dispatch-family-walk-refuses-at-the-caller-limit.md; spec/tests.md:286 |
| FND-004 | low | The new sentence "A call whose receiver's static type is the root's declaring type or any type conforming to it ... names that root's table" has no criterion for an abstract conforming type. TC-793 covers one concrete subtype that redefines `size` and one that inherits `area`. The case the code currently misses (SR-9030 FND-003) has no AC to fail. Add it to FR-302-AC-1, or add a new AC. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:56-58 |
