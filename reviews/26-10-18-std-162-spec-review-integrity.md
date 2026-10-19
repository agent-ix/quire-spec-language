---
id: SR-12888
title: "Integrity review of STD-162 state-graph instancing move (PR #981)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language; PR #981 (git diff origin/main...HEAD at 12778aab4); FR-125 Instancing, FR-165, ADR-022 SG-5, and the statements they must agree with (FR-120 universes, QSpec FR-361 over binding, QSpec FR-390, ADR-022 GM-3, QSL FR-361-AC-4)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
---

## Summary

Ticket: STD-162. Consistency review of the moved instancing rule against
`ModelSubject.universes` (FR-125 Inputs, FR-120 `PopulationUniverse`), the
QSpec rule FR-125 says it follows (QSpec FR-361 "Binding of the clause's
`over` parameter"), QSpec FR-390, and ADR-022. Examined: FR-125 Instancing
(both bullets), the FR-125 "QSpec model-subject rules" bullet, ADR-022 SG-5
and the paragraph after the SG table, and QSL FR-361-AC-4. Context only: FR-120
Inputs (`universes`, one per population, `keys: Vec<String>`), FR-120-AC-7
(no universe gives `RequiresBound`), QSpec FR-390 "over" paragraph and AC-7,
and PR #968's `StateGraphDeclaration`, which records the population "when
exactly one is".

Author's open point (3), the zero-object universe: yes, this gap is worth
stating. It is part of FND-001.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-125 Instancing says "one instance per object of the universe of the `over` parameter's type", but universes are per population (FR-120: a declaration key and `keys`). FR-125 line 76 says it reads the `over` binding as QSpec FR-361 states it: "once for each object of the subject's universe whose type conforms to `T`". The two rules differ for a subtype population (FR-106-AC-8's `archive` of `Sub`), for a type covered by several populations, and for a type covered by none. #968 resolves the population only "when exactly one is". Also unstated: a subject with no universe for that population (FR-120 gives `RequiresBound`; is that the result here?), a universe with zero keys (zero instances, so the item is vacuously true with no result, or refused?), and what "the universe's canonical order" is (`keys` as listed, or sorted). Fix: name the objects (every key of every universe population whose member type conforms to `T`), the order (for example population declaration key, then key, ascending), the missing-universe outcome, and the empty-set outcome. | spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md:76-78,97-104 | wrong-requirement |
| FND-002 | low | ADR-022's new sentence calls the temporal-operator refusal "GM-3's `expression-form`", but GM-3 covers anchor and pre-state reads. The rule that a predicate has no temporal operator is SG-4, and its refusal is QS-1's. Cite SG-4. | spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:97-99 | wrong-requirement |

## Verdict

No high finding. FND-001 should be fixed before merge, because this PR moves
the instancing rule into FR-125 and the rule is written more narrowly than
the QSpec binding it cites. It is a wording fix in FR-125 Instancing, plus
one FR-125-AC-7 vector for the empty universe if the outcome is stated.

Checked and sound: the move fits ADR-018 SM-2 (universes belong to the
subject, not to S3) and QSpec FR-390-AC-7 ("an `over` clause yields one item
per universe key", that is, at request time). It keeps the S3 requirement
record per claim, with per-instance items at request time. ADR-022 SG-5's new
text agrees with FR-165 and FR-125. QSL FR-361-AC-4 (the model-profile gate
for a state-graph clause) does not depend on where instancing happens and is
unaffected. The FR-125-AC-7 order "`a` then `b`" agrees with any reasonable
order over `{a, b}`.

Author's open point (1), `result`: the shared grammar has `'result'` as a
reserved primary keyword (`proposals/quire-v1/shared-grammar.md:56,135,350`)
and calls `self`, `result` and `pre(...)` "caller-side anchor operations"
(line 884). #968 has a distinct `ExprNode::Result`. A result read is
therefore a separate syntactic form, not a name, and FR-165 is right to
refuse it with `expression-form` beside the anchor and pre-state reads, not
with `missing-name`.
