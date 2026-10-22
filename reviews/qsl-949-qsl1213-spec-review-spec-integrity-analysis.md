---
id: SR-15470
title: "integrity review of FR-093 and FR-094 quantity literal reason (QSL PR #1213)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@3b7c8b893fb6d4c77bca2bfd77d82c0e6d417d37; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-094
    type: reviews
---
# SR-15470: integrity review of FR-093 and FR-094 quantity literal reason (QSL PR #1213)

## Summary

Ticket: QSL-949 (nearest planner-owned ticket; the PR is a cleanup with no ticket of its own).
Diff reviewed: `git diff origin/main...3b7c8b893` (FR-093 "Quantity literals are outside V1 scope",
FR-094 quantity literal paragraph). Read with ADR-014 §4, FR-097, FR-093-AC-33, TC-483, IR
quire-contract-ir at d0737774 (the QSL lock pin) and CG quire-contract-codegen main be88468.

Removed reason, verified true to remove: IR #497 (a3cfb79, IR-938) is an ancestor of the QSL lock pin
d0737774. IR `v2/operations.rs` admits `LiteralKind::Quantity` at family `quantity`, and IR FR-038-AC-299
to AC-302 / TC-230 are implemented. The "Remaining work: IR-955" lines are gone. No other QSL spec text
blames the IR reader (git grep). FR-093-AC-33 and TC-483 are unchanged, and no AC id changed.

New reason, not grounded: the PR states the remaining limit as "CG's bounded lowering answers such a
domain `RequiresBound`, so the quantity literal stays outside V1". The documents say the opposite for a
literal:

- ADR-014 §4 extent rule: domains are the claim's argument and bound-variable types, and "a literal
  contributes none, because a literal is one value and not a domain". The quantity row of the extent
  table is about a quantity-typed position, not a literal. FR-097-AC-2 says the same ("Over a claim's
  argument types ... quantity is exactly one unbounded domain").
- IR FR-038-AC-73 and `v2/lower.rs::quantity_position_ends` (d0737774): under `require_bounds`, IR returns
  `RequiresBound` only for a *position* typed at a quantity (a parameter's own type, a composite member).
  "a record with no quantity position whose closure reaches a unit only as a `literal.type` annotation
  ... lower[s]"; the code comment says "a `literal.type` annotation never raise[s]".
- The quantity positions that do raise `RequiresBound` are quantity parameters, which FR-093 itself puts
  in scope. So the bound rule does not separate the literal (refused) from the parameter (admitted).
- ADR-014 §4 "Available finite bound": a quantity domain is not boundable, so `finite_bound_available` is
  false and a bounded-only candidate settles `unsupported`/`unbounded-extent`, not `requires-bound`
  (FR-097-AC-3, AC-4). "CG answers `RequiresBound`" also misstates CG's disposition for a quantity domain.
- A backend disposition is decided per route at negotiation. It cannot be the reason a check-stage
  refusal applies on every route. QSL routes quantity literals to exact evidence elsewhere (FR-195
  thresholds, FR-185 supports).

What is true instead: after IR-938 no downstream reader or CG rule refuses a quantity literal. The
refusal is QSL's own V1 lowering scope. QSL's checked-package lowering emits no `value_kind: quantity`
literal (`check/node_key` `LiteralValue` has no quantity variant; `check/probabilistic.rs` raises
`ProbabilisticCause::QuantityLiteral`). This is what QSL-949's spec-check comment found, and FR-093-AC-33's
`not-yet-implemented` cause already says it.

Gate: `QSPEC_DIR=/Users/peter/dev/quire-specification make check-spec-validation` exit 0 at 3b7c8b893, with
no warnings on FR-093 or FR-094.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: FR-093's new paragraph ("A quantity is an unbounded domain ... CG's bounded lowering answers such a domain `RequiresBound`, so the quantity literal stays outside V1") is not grounded. ADR-014 §4 says a literal contributes no domain. IR FR-038-AC-73 lowers a closure that reaches a unit only as `literal.type` without `RequiresBound`. The quantity domains that do raise are parameters, which FR-093 admits. An unboundable quantity domain settles `unsupported`/`unbounded-extent`, not `requires-bound` (ADR-014 §4, FR-097-AC-4). Replace it with the true reason, stated as behaviour: QSL's V1 lowering emits no `quantity` literal node, so the literal is refused `unsupported_construct`/`not-yet-implemented` (FR-093-AC-33); IR's reader admits FR-322-AC-79 literals, so admitting one is a QSL lowering change. Or state the refusal as V1 scope with no downstream reason. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:160-163 |
| FND-002 | high | major: FR-094 repeats the same claim ("A quantity is an unbounded domain that no `FiniteBound` ranges over (ADR-014 extent rule, FR-097-AC-2), which CG answers `RequiresBound`"), so it is wrong for the same reasons as FND-001. Apply the same correction, or keep only the FR-093 link and the sentence on unit key rules. | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:324-326 |

## Verdict

Not mergeable: FND-001 and FND-002 (major). The removals are correct: the IR reader admits quantity
literals at the QSL lock pin, IR-955 is gone, no other text blames the reader, FR-093-AC-33 and TC-483
still hold, no AC id changed, and the text is behaviour-only. Only the new reason is wrong.

## Dispositions

Round 1, reviewed QSL #1213 at c4c4fc3e1 (fix commit over 3b7c8b893). Code facts verified at that head:
`check/node_key/mod.rs` `LiteralValue` has no quantity variant (Boolean, Integer, Rational, Text, String,
Uuid, Timestamp, None, Enum); `check/lowering.rs` maps `Value::Quantity(_)` to
`probabilistic::quantity_literal`, which raises `ProbabilisticCause::QuantityLiteral`, coded
`unsupported_construct`/`not-yet-implemented`. No `RequiresBound` or IR-955 text remains in FR-093 or
FR-094; no AC id changed. Gate `make check-spec-validation` exit 0.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-093 now states the refusal as QSL's own V1 lowering scope (no `value_kind: quantity` literal; `check` raises `ProbabilisticCause::QuantityLiteral`); verified against the code at c4c4fc3e1 |
| FND-002 | fixed | FR-094 now says the refusal is QSL's own V1 lowering scope, with no `RequiresBound` claim |
