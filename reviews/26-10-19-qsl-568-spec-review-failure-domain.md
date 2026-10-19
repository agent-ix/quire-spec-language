---
id: SR-12898
title: "Failure-domain review of PR #990: FR-392 unselected facet and confidence edge cases"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@a18d54884; FR-392 Description, Behavior, AC-3; FR-308 facet sentence; FR-186 Confidence parameters; agent-ix/quire-specification@4f65ed2 proposals/quire-v1/shared-grammar.md lines 875-894"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-392
    type: reviews
---
# SR-12898: failure-domain review of QSL-568 (PR #990)

## Summary

Ticket: QSL-568. This review looks for S2 behaviour that FR-392 leaves
unstated: a unit that does not select the complete facets, and confidence
lists that are partial, repeated or absent.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-392 specifies S2 only "when a `1-draft` unit that selects the complete facets holds a `probabilistic-clause`". It does not say what S2 does when the unit does not select them. QSpec says "a form whose facet is not selected receives `unsupported_construct`" (shared-grammar.md:894). The sibling FR-308 settles the same question explicitly: S2 builds the complete form "whether or not the unit selects" the facet, and the gate is not enforced (FR-361 item 8). With FR-392 as written, one implementer refuses at S2 and another builds, and no AC catches either. State the FR-308 rule, or give the S2 refusal and its code, and add an AC for an unselected-facet unit. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:23-26; spec/functional/FR-308-build-the-complete-protocol-forms-at-s2.md:145-148 |
| FND-002 | low | Neither the FR nor the TC has a case with a repeated confidence item (`with alpha 0.01, alpha 0.02`). FR-392's `Vec` keeps repeats, which is correct. QSpec refuses "a clause that omits or repeats one" (shared-grammar.md:879-881), but FR-186 refuses only a claim that states "one or two" of the three. An S2 that de-duplicates by key, for example into a map, therefore hides the repeat, and S3 then accepts the claim. Add a repeated-item build case to AC-3 that keeps both entries. | spec/functional/FR-392-build-the-probabilistic-claim-form-at-s2.md:55,87-88,99 |

## Verdict

The partial-confidence path is sound at S2. `confidence` is a non-empty list
of any keys, so `with alpha 0.01` parses, and FR-392 records it as written.
An absent clause is an empty `Vec`, which the grammar keeps distinct from a
present one because a present clause has at least one item. The S3 refusal
is FR-186-AC-4's. The facet gate (FND-001) is the treatment the brief asked
about: a construct that is recognised and then refused. It should be stated,
as FR-308 does.
