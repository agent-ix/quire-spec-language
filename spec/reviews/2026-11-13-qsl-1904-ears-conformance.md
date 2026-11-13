---
id: SR-2699002
title: "ears-conformance review of QSL-1904 field-edge reachability population link"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-380-bound-a-domain-package-population-and-link-a-reachability-to-its-node.md, spec/functional/FR-105-emit-state-nodes.md (PR #2699 diff, head 55988bc0c)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-380
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-105
    type: reviews
---
# SR-2699002: ears-conformance review of QSL-1904 field-edge reachability population link

## Summary

Ticket: QSL-1904. EARS grammar check of the requirement statements this PR
adds or edits: FR-380's "Field-edge reachability" (four paragraphs), its
edited evaluation sentence under "Meaning", "The population node" closing
sentence, and FR-105's `reaches_field` member note.

What conforms: the emission rule, "When that population has an authored bound
`N`, S4 SHALL emit ... and SHALL write `population: <id>` ...", is a
well-formed event-driven statement with a named actor (S4) and an observable
response. The edited "Meaning" sentence extends an existing SHALL statement
correctly. The FR-105 note is a clause inside FR-105's existing lowering
statement and needs no SHALL of its own. The FR-380-AC-8 to AC-14 criteria
each name a trigger and an observable output; the compound ones are recorded
in SR-2699001 (FND-006).

Verdict: one medium (the walked-population rule, which changes checker
behaviour, has no SHALL) and one low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The rule that changes checker behaviour, that a field-edge `reaches` walks the population covering the edge type `T` and not the operands' static type, is stated as description with no actor and no SHALL ("walks the objects of `T` ... so it walks the one population", "Its requirements record adds that domain"). Restate it as a ubiquitous requirement on the checker, for example "The checker SHALL take the population a field-edge `reaches` walks as the one population covering `T`, the edge's owner and reference type, whatever the operands' static types", so it can be traced and tested as the record change it is. | FR-380.md:153-163 |
| FND-002 | low | The no-bound paragraph ("the key is absent, not `null`, no population node is emitted ... neither is a refusal") states S4 and checker behaviour with no SHALL. FR-365 "The population link" has the same style, but here it is the only normative statement of the absent case for the field form. | FR-380.md:176-182 |
