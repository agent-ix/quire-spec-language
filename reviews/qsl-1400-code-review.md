---
id: SR-2026111015000000000009
title: "code-review (with rust-review lane) of PR 2676 source object types, first slice"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR 2676 diff origin/main...c4a3417cb: qsl-cst grammar, qsl-forms object_declaration, qsl-semantics check/source_model.rs and assemble.rs, qsl-package emit SourceObjectEmission, qsl-replay CompileRefusal::cause"
review_set: subset
---

## Summary

Ticket: QSL-1400. First code slice of FR-569: grammar and reserved words, `ObjectForm`, source objects assembled as one synthetic domain package under the empty alias, and a single `SourceObjectEmission` refusal at S4. Rust lane (rust-review) folded in. Reviewer probes ran in a scratch clone (probe log beside this file); focused tests ran green in the worktree: qsl-cst and qsl-forms (all), qsl-semantics `source_objects` and `operation_bodies` (25 passed).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-569-AC-3's binder case is not implemented. A quantifier binder named like a source population (`forall(orders in orders : true)`) checks with no `ambiguous-name` refusal; the probe confirmed it. | qsl-semantics/src/check/check.rs:2370 |
| FND-002 | medium | The source model is bound only to unit index 0 (`model_selection_units.push(0)`). In a multi-unit package, a clause in another unit that names a source type through `spelled(Some(unit), ..)` finds no "" alias and refuses `missing-name`. Objects declared in a secondary unit are still bound to the primary unit. | qsl-semantics/src/check/assemble.rs:3953 |
| FND-003 | low | The collision check adds a function's whole-declaration span, not its name. It skips the function entirely when `spans()` is None, so the collision would be missed. | qsl-semantics/src/check/assemble.rs:1442 |
| FND-004 | low | Any `normalize_on` refusal or incomplete outcome over the synthetic package becomes `ModelType{alias:"", node: identity}` at the first object's name. This loses the real cause and locus. | qsl-semantics/src/check/assemble.rs:1520 |
| FND-005 | low | `source_model::build` does unmetered work before the metered normalization. The ancestor BFS uses `Vec::remove(0)` and `contains`, and the field and operation shadow checks re-walk the chain for each member. A long `extends` chain is O(n^3), with no AssemblyLimits charge. Use `VecDeque` and a set, and meter the work. | qsl-semantics/src/check/source_model.rs:282 |
| FND-006 | low | `emit_checked` and `emit_package` run `require_protocol_results_for_refusals` before `emit_package_inner`. A source-object unit with a refused protocol therefore gets `ProtocolResultsRequired`, not the single `SourceObjectEmission` that FR-569 Interim requires. | qsl-package/src/emit.rs:4420 |
| FND-007 | low | Diagnostic messages reuse unrelated causes. `AliasCycle` reports a supertype cycle. `UnresolvedTypeName` reports a missing identity field, frame field or operation. A model-only unit's `pre X on M::bump` now reports type `M` unresolved, where it was `invalid_syntax` on main. | qsl-semantics/src/check/source_model.rs:142 |

## Scope and evidence

- FR-569 Surface syntax, The source model, Name resolution and collisions, Declaration checks, Interim behavior (spec/functional/FR-569-declare-object-types-populations-and-operations-in-qsl-source.md): examined.
- Soundness probes (scratch clone): with `model M` and `object Order { extra }`, `on Order` sees `extra` and `on M::Order { self.extra }` refuses. `extends M::Order` refuses `malformed-declaration`. `Reference<Order>` in a source record resolves. `Sub.scaled(m: Text)` refuses VarianceParameter. S-R' refuses EffectEscape on `Order/note`. A bare name never reached a model declaration, and `M::X` never reached a source one.
- S4: `first_source_object` is checked before any node is built. `emit_checked_v2::run` writes `--out` only after `emit` succeeds, and the handoff writer leaves no directory. No side effect comes before the refusal.
- `CompileRefusal::cause()` now returns `EmitRefusal::cause()`. Callers are `OutcomeDiagnostic::from_compile_refusal` (qsl-replay/src/outcome.rs:1472), `ReplayRefusal::cause` for Recompile (qsl-replay/src/execute.rs:634) and the command layer. Every emit refusal that has a catalog cause now reports it instead of null: OutputLimit, the temporal clock selection refusals, StaleProtocolReference and not-yet-implemented. The change is correct, and no existing test pins a null cause for these.
- The change to `operation_bodies::a_target_with_the_wrong_segment_count_refuses` is not a regression. A 2-segment target in a model-only unit still refuses `missing_declaration` at the same target span. Only the payload name changes, from the full target to its first segment. FR-569's grammar makes the 2-segment form a source `Type::op`.

## Verdict

The approach is sound for single-unit packages: names never cross between source and model, and S4 refuses first. FND-001 (an AC-3 case is missing) must be fixed. FND-002 needs a decision: support multi-unit, or refuse it explicitly.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | Spec and code disagree on identity. FR-569 now says "The assembler checks the list and keeps it beside the source model". The code validates `identity` and then drops it: nothing outside `check_object` reads `ObjectForm.identity`, the synthetic records have no identity slot, and no side table exists. Carry it now, as the lead asked, or reword the spec. | qsl-semantics/src/check/source_model.rs:334 |
| FND-009 | low | FR-569 now scopes source object *types* to their declaring unit, but `NamedScope::of` collects every model's populations under the empty alias across the whole package. A clause in unit B can therefore name unit A's population `orders`. The spec does not say whether populations are unit-scoped, and the scoping is enforced only through `object_type()`. | qsl-semantics/src/check/check.rs:2411 |
| FND-010 | low | `SourceObjectEmission` carries its span, but `CompileRefusal::region()` maps every `Emit(_)` to None, so the replay and handoff outcome diagnostic has no locus for "at the span of the unit's first `object` declaration". AC-11's span is asserted only on `EmitRefusal` in qsl-package. | qsl-replay/src/spine.rs:610 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | `named_population` refuses `AmbiguousName` when the source population name is the binder or a local in scope (typing.rs:3412, 3484). `a_binder_named_like_a_population_is_ambiguous` asserts one refusal for `orders`, and a differently named binder still checks. Both loci sit at the use (location and child 0), not at the binder declaration, which is acceptable. |
| FND-002 | fixed | The fix chose unit scoping: each declaring unit gets its own source model, bound to that unit (assemble.rs:3992-4023), and the FR-569 spec states the rule. `a_source_type_is_visible_only_in_the_unit_that_declares_it` covers both unit orders. Populations are not scoped this way (FND-009). |
| FND-003 | fixed | A function now always contributes a span, an empty span when it has none (assemble.rs:1481). The spec says "A function contributes its declaration span", and the AC-3 test asserts the function's offset. |
| FND-004 | fixed | `SourceObjectFault::Normalization` and `Limit` carry the normalizer's own code and cause (assemble.rs:1560-1577). |
| FND-005 | fixed | The ancestor walk uses `VecDeque` plus a seen set, and the shadow checks use `own_field` and `own_operation`, so neither re-walks the chain. The work is bounded by declaration count; no meter was added, which is acceptable at this size. |
| FND-006 | fixed | The `first_source_object` check moved into `require_protocol_results_for_refusals`, ahead of the protocol-results check (emit.rs:4402). The qsl-package test `a_source_object_refusal_wins_over_a_refused_protocol` passes. |
| FND-007 | fixed | New faults `SupertypeCycle`, `MissingMember` and `ModelAliasAsType`, with messages in spine.rs:966-979. |
| FND-008 | fixed | Reworded per the lead's decision. FR-569 now says "The assembler checks the list (the refusals below stand); emission's source-object identity rows read it from the source declaration once emission lands." The code matches: `ObjectForm.identity` is validated at assembly and nothing claims it is carried. |
| FND-009 | still-open | Population scoping is wired only for unit-level state clauses (`ClauseDeclarations.unit`). The temporal, hyper, timed, protocol-slot, protocol-control and state-graph paths call `NamedScope::of(.., None)`, and `AdmittedModel::visible_in(None)` returns true, which fails open. Reviewer probe at 0cfc65eb5: unit a declares fixture S; unit b declares `object Watch` and `temporal C using t over (w: Watch) clock "model-steps" on origin { eventually holds(forall(x in orders : true)) }`. Unit b checks and is ADMITTED, in both unit orders. FR-569 now says "a clause that names the population SHALL be refused". Fix: pass the declaring unit on every clause path, or have a source model refuse visibility when the unit is None. |
| FND-010 | fixed | `CompileRefusal::Emit(EmitRefusal, Option<SourceRegion>)`. `emit_refusal` attaches the region of `first_source_object`'s declaring unit for `SourceObjectEmission`, and every other constructor passes None. The root test `a_unit_with_a_source_object_is_refused_at_emission_and_writes_nothing` asserts the region equals the `object Order` declaration. It passes, along with qsl-replay lifecycle (70) and the refinement test. |
| FND-009 | fixed | Fixed at 71ced382c. `AdmittedModel::visible_in(None)` is now false for a unit-scoped source model. Scoping applies only in multi-unit packages, where `source_owners` is non-empty. Temporal, protocol, state-graph, hyper and model-context function paths now pass their declaring unit. Reviewer re-probe in a scratch clone: a unit-b temporal clause naming unit a's `orders` refuses `MissingName("orders")` in both unit orders. A temporal clause in the declaring unit, primary or secondary, is admitted, as are a single-unit temporal clause and a secondary unit's own state clause. Hyper can reach a population through its `holds` body, but it passes `cx.source_owner()`, and a missing unit fails closed, so no cross-unit leak is possible. Timed clauses (`unit` None plus `of_alias(model)`) cannot name the empty source alias. The other None paths, model operation clauses, type invariants and refinements, now fail closed. No test is needed for hyper or timed. |
