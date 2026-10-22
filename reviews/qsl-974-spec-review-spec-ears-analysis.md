---
id: SR-15481
title: "EARS review of QSL-974 quantity literal statements (QSL PR #1214)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@b155d3ea321b8dbde055fe4a282ec7789ec3a3a7; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
# SR-15481: EARS review of QSL-974 quantity literal statements (QSL PR #1214)

## Summary

Ticket: QSL-974. Statements examined: the new FR-093 "Quantity literals" section (lines 146-227),
its table row, and the rewritten FR-094 quantity paragraph (FR-094:319-325). The section holds one
SHALL (lines 148-149). The Typing, Lowering, Domain and Probabilistic paragraphs are explanatory,
and the FR-094 paragraph is descriptive and needs no SHALL. The gate's grammar lint reports
nothing on these lines.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | minor: the one SHALL, "A quantity literal `n u` in a value expression ... SHALL check and lower to one FR-322 literal node", makes the literal the subject, not the component that acts. Write it as a ubiquitous requirement on `check`: "`check` SHALL type a quantity literal `n u` in a value expression as `Quantity<u>` and lower it to one `value`/`literal` node of `value_kind` `quantity`". The replaced section's "QSL SHALL NOT substitute a rational scalar type" is now the descriptive "`check` never emits one" (line 187); keep it as a SHALL NOT. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:148-149,186-187 |
| FND-002 | low | minor: the Refusals list (lines 210-224) is written as "is refused ... when:" and has no SHALL, so the four unwanted-behaviour rules are not normative statements. Only AC-34 makes them testable. Write them as EARS unwanted-behaviour requirements, for example "If a quantity literal's unit name resolves to no declared unit, then `check` SHALL refuse it `missing_declaration`/`missing-name`, naming the name, and emit no literal node", with one per bullet. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:210-224 |

## Verdict

Mergeable on EARS grounds. Both findings are minor wording fixes.

## Dispositions

Round 1, reviewed QSL #1214 at 3fd9a7398 (fix commit over b155d3ea3); code facts re-verified in the QSL tree at that head.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | "`check` SHALL type a quantity literal ... as `Quantity<u>` and lower it"; the SHALL NOT on substituting a rational scalar type is restored |
| FND-002 | fixed | each refusal is now an "If ..., then `check` SHALL refuse ..." unwanted-behaviour statement |
