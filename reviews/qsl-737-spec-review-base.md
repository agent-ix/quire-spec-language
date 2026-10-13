---
id: SR-7878
title: "QSL-737 PR 800 spec review (base): child record value at an ancestor-typed slot"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@67de856995d7f102f050fad3275c167e4f66b26d; spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, spec/functional/FR-056-admit-domain-package-model-declarations.md, spec/functional/FR-090-return-a-family-outcome-or-a-typed-family-refusal.md, spec/test-cases/TC-998..TC-1002, spec/spec.md, spec/tests.md"
review_set: subset
---

## Summary

Ticket: QSL-737. PR 800 (base `impl/business-model-meanings`) adds FR-359
(10 ACs) and TC-998..TC-1002, and amends FR-056 (pointer) and two FR-090
cause rows. No code changes. This base review checks the design against
QSpec FR-208/FR-322/FR-151, QSL FR-056-AC-18, FR-093, FR-106-AC-14/15 and
FR-334. It also checks the claims against the code: QSL `impl/business-model-meanings`
at the reviewed sha, quire-exact `origin/main` a58033d, QSV `origin/main` 4ff3c61
and quire-contract-ir `origin/main` 6e45fd5. No build was run.

## Verdict

FAIL. The core design is consistent with QSpec FR-208:

- a child keeps its identity;
- field reads go by identity;
- unequal when the most-specific keys differ, otherwise compare the effective fields recursively;
- no type-level covariance.

The equality rule matches FR-208's record-value-type meaning ("equal exactly
when both have one most-specific type and every field is equal"). Because the
first test is on the most-specific key, it covers grandchildren. Open
decisions 1 (no covariance, FR-334) and 2 (mixed static types stay refused)
hold. TC-235 M17 reads `Order/total` at static type `Money`, so it observes
"unequal" without a mixed-type `==`.

Several claims the design rests on are false against the code today, and
FR-359 depends on kernel and IR-reader changes that neither the rider list
nor item 8 names:

- quire-exact equality and canonical keys fault on composites of distinct declarations (FND-001).
- The QSL emitter does not emit model-owned records as `model`/`record_value_type` (FND-002).
- The IR reader admits model-operation arguments by exact type node (FND-003).

Item 8 also over-specifies the kernel mechanism (FND-004). FR-093 keeps text
that contradicts item 2 (FND-005). AC-1 silently changes a check cause (FND-006).

## Examined Scope

```yaml
scope:
  - {id: FR-359, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: FR-359-AC-1..AC-10, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: FR-056-AC-18, path: spec/functional/FR-056-admit-domain-package-model-declarations.md, role: context_only}
  - {id: FR-093, path: spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, role: context_only}
  - {id: FR-103-AC-4, path: spec/functional/FR-103-admit-model-operations-and-frames-on-the-spine.md, role: context_only}
  - {id: FR-106-AC-14, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: context_only}
  - {id: FR-106-AC-15, path: spec/functional/FR-106-admit-snapshots-and-invocations.md, role: context_only}
  - {id: QSpec-FR-208, path: quire-specification spec/objects/foundation/FR-208-quire-meaning-vocabulary.md, role: context_only}
  - {id: QSpec-FR-322, path: quire-specification spec/objects/interfaces/FR-322-checked-package-artifact.md, role: context_only}
  - {id: QSpec-FR-151, path: quire-specification spec/functional/type-model/FR-151-resolve-conformance-redefinition-dispatch.md, role: context_only}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Items 4 and 5 (and AC-4, AC-5, AC-7) depend on quire-exact kernel changes that FR-359 item 8 and the PR's rider list do not name. Kernel equality sends two composites with different declarations to the catch-all arm, which returns the checked invariant `ValueKindMismatch`. Canonical-key comparison returns `None` for them, so set, bag and ordered-set construction refuses `CanonicalKeyUnavailable`. So `TaxedMoney == Money` at a `Money` slot faults even after QSV `operand_value` is fixed. A `Set<Money>` holding a `Money` and a `TaxedMoney` cannot form. Nothing specifies the order relation between two distinct declaration keys ("canonical key and order begin from the most-specific declaration key"). That order is a canonical-key rule owned by QSpec and quire-exact. | FR-359 items 4, 5, 8; FR-359-AC-4; FR-359-AC-5; FR-359-AC-7; quire-exact src/equality.rs:218-262; quire-exact src/key.rs:136-169; quire-exact src/collection.rs:489-500 |
| FND-002 | high | Item 3's premise is false in the current emitter. Model-owned records and events reach `close_composite` through `model_composites` and are emitted as `composite_type`/`record` through `insert`, with `Owner::Source`. `record_form` maps `RecordValueType` and `EventType` to `None`, and no non-test Rust in QSL emits `record_value_type`. So no `ModelOwner` "already" joins the record node to the pinned selection. Until the emitter changes, SR-7861 FND-005 (the IR cannot recover the relation) stays true. FR-103-AC-4 already requires the `model`/`record_value_type` node, and its test checks only composite typing. FR-359-AC-3 asserts no `supertypes` member and no `convert` node, but not that the record node is the `model`/`record_value_type` node with its `ModelOwner`. A slice can therefore satisfy AC-3 while the IR stays relation-blind. The emitter fix is needed and should be an explicit AC. | FR-359 item 3; FR-359-AC-3; FR-103-AC-4; qsl-semantics/src/check/lowering.rs:1553-1576; qsl-semantics/src/check/lowering.rs:2055-2093; qsl-semantics/src/check/lowering/model.rs:301-323; qsl-semantics/src/check/assemble.rs:1532-1558,1898-1899; qsl-semantics/tests/it/model_operations.rs:2349-2354 |
| FND-003 | medium | FR-359-AC-8 (a child at a parent-typed model-operation parameter "checks and runs") depends on an unmerged change in the IR reader. `check_model_member` in quire-contract-ir compares each operation argument's type node digest with the parameter slot type's key exactly, and refuses `operator-ineligible` otherwise. QSL compile reads every emitted package back through `emit_and_read_v2`. A compiled program that passes a `TaxedMoney` expression to an operation's `Money` parameter would therefore refuse at read-back. The QSpec FR-322 rider is listed, but FR-359 does not say that read-back of operation calls depends on it. It does not name the quire-contract-ir implementation, and AC-8 has no read-back expectation. Source `function.call` arguments are checked by family only, so they are not affected. | FR-359-AC-8; FR-359 item 6; quire-contract-ir crates/quire-contract-model/src/checked_package/v2/operations.rs:2130-2145; src/protocol_artifact/handoff/writer.rs:1385,1602 |
| FND-004 | medium | Item 8, bullet 2, says the construction check of `OptionValue::present` and every collection constructor "SHALL admit a conforming payload or element when given the environment". That fixes a kernel mechanism, which contradicts open decision 3 ("the owners may pick otherwise"). quire-exact already exposes `OptionValue::from_admitted`, documented for "the caller checking admission (e.g. an upcast-aware lookup)", and `collection::from_admitted`. The Option half of the quire-exact rider is unnecessary: QSL can admit with `TypeEnvironment::admits` and call `from_admitted`. Only collections need kernel help, because `from_admitted` there requires caller-side ordering and charging. The bullet should state the outcome (no checked-invariant fault on a conforming payload or element), not the API. | FR-359 item 8; quire-exact src/value.rs:825-855; quire-exact src/collection.rs:246-261 |
| FND-005 | medium | FR-359 item 2 says a checked `Field` read "SHALL NOT carry a position", and Dependencies says FR-093 "keep[s] its text". FR-093's lowering table still names the checked form `Field{index}`. The two requirements disagree on the checked node's shape, and the PR leaves the stale one unchanged. The FR-093 row should name the field identity in this PR. | FR-359 item 2; FR-359 Dependencies; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:181 |
| FND-006 | medium | FR-359-AC-1 expects `ill_typed`/`operator-ineligible` at check for a parent-typed read of `promised_by`. QSL check today refuses an undeclared record field with `type-mismatch` (`field()` returns `mismatch(location)`). No QSL FR found assigns `operator-ineligible` to a source `e.f` on a record; that code belongs to the IR reader's `record.project` rule (QSpec FR-322-AC-65). The AC changes an existing cause without saying so. It should either keep `type-mismatch` or state the change and its owner. | FR-359-AC-1; qsl-semantics/src/check/check.rs:1923-1938 |

## New findings (disposition pass 1)

Reviewed at d449b0273567cd13de537ad59bce73550045c37c.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | high | Item 3 and AC-3 ("so is a domain event type") would emit an event type as a `model`/`record_value_type` node with its `ModelOwner`. QSpec FR-322 "Model-owned members" step 2 lets a `model`/`record_value_type` node stand only for "a record value type of FR-208's `quire.meaning.model.record-value-type/v1`". The owner join refuses `missing_declaration`/`missing-selection` when the owner names no declaration of the node's kind, and FR-322 defines no event-type node form. A package carrying an event this way cannot read back `Verified`, which contradicts AC-3's own read-back clause. Cross-repo requirement 5 does not ask QSpec FR-322 to admit event types (a new form, or a widened `record_value_type`). AC-3 depends on that unlisted change. | FR-359 item 3; FR-359-AC-3; FR-359 cross-repo requirement 5; QSpec FR-322 "Model-owned members" step 2, FR-322-AC-65 |
| FND-008 | medium | Cross-repo requirement 5 sends the distinct-declaration canonical-key order to QSpec FR-322, and the cross-declaration equality rule only to FR-208. In QSpec, the canonical keys are owned by FR-144 "Element admission and canonical keys". Its record row reads "lexicographic over fields in declaration order", and it requires that "two values have equal keys exactly when they are FR-149 equal". Composite `=` is FR-149's, and FR-143 owns record equality (FR-143-AC-5). The amendment request names the wrong owning requirement for the key order, and omits FR-144, FR-149 and FR-143. | FR-359 cross-repo requirement 5; FR-359 item 5; QSpec FR-144; QSpec FR-149; QSpec FR-143-AC-5 |
| FND-009 | medium | Item 5 justifies the new order as "a function of content alone, since a declaration key is the content identity of the declaration ... it does not depend on source order, names". At evaluation, a model record's `CompositeValue` declaration key is not the IR node's key. It is the handle that `declared_type_handle(owner, "{alias}::{artifact}")` mints from the selecting unit's `SourceOwner` and the source-local alias. So the order between `Money` and `TaxedMoney` in a `Set<Money>` changes with the alias written in `model M = ...`. Item 3 changes only the emitted IR node. FR-359 should require that the evaluator's composite declaration key for a model record or event be its model-owned node key, which FR-322 step 2 keys from content only, so that the claim holds. | FR-359 item 5; FR-359 item 3; qsl-semantics/src/check/assemble.rs:699-716,887-893 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d449b0273 |
| FND-002 | fixed | d449b0273 |
| FND-003 | fixed | d449b0273 |
| FND-004 | fixed | d449b0273 |
| FND-005 | fixed | d449b0273 |
| FND-006 | fixed | d449b0273 |
