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
