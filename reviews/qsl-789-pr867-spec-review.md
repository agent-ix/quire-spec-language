---
id: SR-11261
title: "Spec review of quire-spec-language PR #867: navigation typing over a relationship end"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@818fe453e4af3ef46aba31e117d59261be8e3245; spec/functional/FR-363-type-navigation-over-a-relationship-end.md, spec/test-cases/TC-1031-r-name-over-a-relationship-end-types-by-the-destination-multiplicity.md, spec/test-cases/TC-1032-a-checked-navigation-lowers-to-quire-op-model-navigate.md, spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md, spec/functional/FR-300-check-model-forms-through-the-state-model-family.md, spec/functional/FR-085-resolve-relationship-end-references.md, spec/functional/FR-086-bind-systems-model-references.md, spec/spec.md"
review_set: subset
---
# Spec review of quire-spec-language PR #867

## Summary

Ticket: QSL-789. One pass of spec-review with the contradiction, ambiguity,
completeness, testability and spec-integrity sub-analyses over the PR diff.
Spec-only PR, so no code review. Only major issues are reported. Mediums and
lows are recorded in the Verdict.

Oracles (QSpec, by id): FR-152 "Navigation", FR-322-AC-56, FR-322-AC-57,
TC-281 RN-01 to RN-19, TC-197 Y07 to Y09, TC-235 M21. Also IR main's
`navigate` member check, and QSL `check.rs` / `state_model.rs` for the
unknown-field refusal.

Examined:
- FR-363 Description, Inputs, Outputs, Resolution, Direction, Destination multiplicity, Lowering (examined)
- FR-363-AC-1 to FR-363-AC-6 (examined)
- TC-1031 fixture N and vectors N01 to N12 (examined)
- TC-1032 procedure and expected results (examined)
- FR-093 `Navigate` row, FR-093-AC-3 and FR-093-AC-13 exclusion lists (examined)
- FR-094 `relationship_end` bullet, FR-300 Inputs/Outputs/Downstream, FR-085 and FR-086 scope edits, spec.md index (examined)
- Open PR #868 TC-1034 and FR-364 (context_only)
- QSL tests reading FR-093/FR-094 text and the FR-093-AC-13 NOT_LOWERED list (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Open PR #868 uses the opposite role rule. Its TC-1034 navigates `navigate(parts, s1)` from a `Sys`, `navigate(home, p1)` from a `Pump` and `navigate(pump, t1)` from a `Tank`. That names each traversal by the destination end's role. Under FR-363's near-end rule, which QSpec FR-322-AC-56/57 and IR main also use, `parts` and `home` are inverse ends navigable only from `Pump` and `Tank`, and `pump` is the forward end navigable only from `Pump`. All three would refuse, not give the stated results. Two open QSL specs give one source form two meanings. FR-363 matches the ruling, so the fix belongs in #868: rename TC-1034's roles to the near-end rule, or swap the fixture's role placement. It must be settled before both merge. | spec/functional/FR-363-type-navigation-over-a-relationship-end.md:54-66 |
| FND-002 | high | FR-363 does not say that it departs from QSpec. QSpec TC-197 Y07 and Y08, and a QSpec model-definition sentence, name a traversal by the destination end's role (`self.parts` from `Sys`, `self.home` from `Pump`). FR-363 names it by the near end, as FR-322-AC-56/57 and IR do. Dependencies says "QSpec FR-152 Navigation and FR-322-AC-56 and AC-57 own ... the role lookup". FR-152 states no role lookup, and its oracle rows contradict FR-363. Credit the role lookup to FR-322-AC-56/57 alone. Then state that TC-197 Y07 to Y09 name the far end's role and that QSL follows FR-322. Without this, a port of Y07 and Y08 (FR-152-AC-7 and AC-3) has no rule for which source wins. FND-001 shows this has already happened once. | spec/functional/FR-363-type-navigation-over-a-relationship-end.md:139-142 |

## Verdict

**FAIL: 2 blocking findings (FND-001, FND-002).**

Planner rulings, each verified:
1. Near-end rule: the source role names the forward end, the target role names the inverse end, and the destination multiplicity comes from the far end. The spec matches IR main's `check_relationship_member` and RN-14. Holds.
2. A non-`Reference` receiver refuses `ill_typed`/`type-mismatch` at the receiver operand, with no lifting. `state_model.rs` gives `mismatch(operand_location)`. Holds.
3. An unknown name refuses `ill_typed`/`type-mismatch`. Verified at check.rs:2512-2515 (record field) and state_model.rs:174-177 (object attribute). Both use `mismatch` = `IllTypedCause::TypeMismatch`. Holds.
4. `deref(r).end` refuses `type-mismatch` at the form. Holds.
5. A destination end with no multiplicity refuses `unsupported_construct`/`expression-form` with no default, consistent with FR-085-AC-2. Holds.
6. The operator class `query` is stated as fact in FR-093's row, with no QSpec citation. QSpec main states no operator class for `navigate`, and no QSpec PR on it is open. IR main's own navigate tests build `operator: "query"`, so the reader agrees. Recorded (medium): the text states it as fact rather than deferring.
7. Direction is checked before multiplicity, stated in the spec and tested by N11. Holds.
8. Runtime evaluation is excluded ("Evaluating a navigation node is outside this requirement"). Holds.

Vectors: N01 to N12 each give the right code, cause and type under fixture N, and together they cover AC-1 to AC-5. The subtype receiver (N03) mirrors RN-06. TC-1032's member `relationship_end{declaration: relationship node, name: role}`, its single `[ref(r)]` argument and its `result_type` match FR-322-AC-57's node shape.

Test-parsed tables: no test parses FR-093's application table or the AC-3/AC-13 lists. The FR-093 and FR-094 readers parse golden-vector blocks only, and none changed. QSpec's positive fixtures hold no `navigate` node, so `NOT_LOWERED` in golden.rs needs no entry. No duplicate ids: FR-363 and TC-1031/1032 appear in no other open PR (#862 to #866, #868). The diff has no ticket ids and no tracking, hash or version ceremony, and it removes the old #147 references.

Recorded (medium/low, not blocking):
- Medium: TC-1031 N12's `p.order` uses a parameter `p`, but the procedure writes every vector in an invariant, which has no parameters. The vector needs a function (as in TC-1032) or a let-bound receiver.
- Medium: QSL's existing TC-235 M21 fixture (`tc235_record_event.rs`) gives `Fulfilment/order`'s target end `{lower: 0}` with no upper, which is unbounded. Under FR-363, `present(self.order)` then refuses `expression-form`. QSpec M21 states no multiplicity. The coder must give the fixture `{0,1}` to reach the ticket's outcome.
- Low: "navigable from `T`" could be read as including direction. N09 settles it: candidates are formed by receiver conformance only, and direction is checked afterwards.

## Dispositions

Round 1, reviewed at 62a14fb43c5d6ff465aafbd0758e0cc8ea604b4c.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | deferred | The fix is owned by open PR #868, whose TC-1034 at fe2672a8 still names traversals by the far end's role (`navigate(parts, s1)`, `navigate(home, p1)`, `navigate(pump, t1)`). #868 must move to the near-end rule before it merges. #867 itself is now unambiguous: c70128899 adds a worked `Order/lines` example that agrees with FR-322-AC-56/57, IR and N01 to N04. |
| FND-002 | fixed c70128899 | Dependencies now credits the role lookup and the lowered-node checks to FR-322-AC-56/57 alone, and FR-152 only with direction and result types. It also states that TC-197 Y07 to Y09 name the far end's role and that ports use the near-end role. |

Recorded medium N12, resolved at c70128899. N12 is now `self.order.order` on `M::Fulfilment`. The inner `self.order` checks as `Option<Reference<M::Order>>` (destination `{0,1}`), so the outer read reaches the non-`Reference` receiver rule. It refuses `ill_typed`/`type-mismatch` at the operand `self.order`, which is valid inside an invariant. A lifting mutant would refuse at the form instead (`order` is not navigable from `Order`), so the test must assert the locus.

Merge check: origin/main merged cleanly. spec.md lists FR-363 once in both the relationships block and the index. TC-1029 is now only #866's, on main. FR-364 and TC-1033 to TC-1035 (#868) do not collide.
