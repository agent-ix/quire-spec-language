---
id: SR-2026111015000000000011
title: "code-review (with rust-review lane) of PR 2678 source route for source-declared object types"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR 2678 diff origin/qsl-1400-source-objects-code...006e5e54c: qsl-semantics check/source_model.rs, check/assemble.rs, check/mod.rs, model/observation.rs, model/observation/document.rs, model/normalize.rs, model/domain_package.rs; qsl-replay, qsl-eval simulation and qsl-analyze monitor call sites"
review_set: subset
---

## Summary

Ticket: QSL-1400, slice 2 (FR-569-AC-9, AC-10, model-typed members, E-S checking and its S4 refusal, FR-105-AC-18, the now part of FR-383-AC-67). The Rust lane (rust-review) is folded in. Focused tests ran green in the worktree: qsl-eval `source_objects` and `source_node` (10 passed), qsl-semantics `source_objects`, `model_population`, `observation`, `admission` and `snapshot` (236 passed), qsl-package `source_objects` (3 passed), and qsl-eval `protocol_slots`, `observation`, `model` and `simulation` (271 passed). Reviewer probes ran in a scratch clone with its own target; the probe log is beside this file.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A source member of an unbounded collection type is refused. The arm `(Some(0), None) if form.bounds.is_empty()` can never match, because `bound(0)` is `None` when `bounds` is empty. So `tags: Set<Int[0, 9]>;` refuses `SourceObject(UnsupportedType)`, while the same type is accepted as a record field (probe). FR-569 says a member may have any type a record field may have. | qsl-semantics/src/check/source_model.rs:755 |
| FND-002 | medium | Model types a source member names are resolved across units, ignoring alias scope. `foreign_packages` and `foreign_models` take every model selection in the package, and `foreign_record` returns the first selection with that alias. In the probe, unit b selects no model, and its `object Z { t: M::Total; }` still assembles and checks against unit a's `M`, although a clause `on M::Order` in b is refused `UnresolvedTypeName`. When two units select different packages as `M` (which the per-unit alias check allows), the member binds to whichever package comes first. | qsl-semantics/src/check/assemble.rs:4086; qsl-semantics/src/check/assemble.rs:4267; qsl-semantics/src/check/source_model.rs:70 |
| FND-003 | low | Name resolution runs unmetered and scales with document names times package records. `declaration_name` formats a candidate for each source view and scans every record of every view for each name. `resolve_names` applies it to every population, type and reference name after reading, outside the observation meter. Building the set of source node names once per admission would make each lookup constant time. | qsl-semantics/src/model/observation.rs:1698; qsl-semantics/src/model/observation/document.rs:266 |

## Coder claims checked

- `CheckedGraph::source_packages` and `model_inputs` are correct. Every admission call site in the diff (replay spine, temporal, execute, object environment, argument, qsl-eval simulation, qsl-analyze monitor) passes `model_inputs()`. `ModelSystem::select` keeps the source packages beside the subset it selects.
- Parity for V1 to V6: the outcomes and ordered charge logs are compared raw. V6 neutralises only the package owner string inside the witness's `Debug` text, and the verdict and the log are compared without neutralising. The witness's type identity is meant to differ (source owner against model owner), so the neutralising hides no real difference.
- The `abstract-instance` change on the model route is required. FR-569's observation section asks for an abstract object to be refused "as for a model type", and D05 (`domain_package.rs` `abstract_type` doc) already states the refusal. No model fixture admits a direct abstract instance through observation, and the focused suites listed above stay green.
- FR-383-AC-67 oracle: the test compares the tuple (value, charge log, consumed units for 4 limit kinds) between E and E-S, and asserts that the control is metered. The logs are empty for forwarded scalars, so the unit counts carry the comparison. That is adequate.
- `DomainPackage.foreign`: it is read only by `normalize` operation-parameter and operation-result checks. It is computed as the package keys that the members name but the package does not define. That is correct.
- `resolve_source_operation` mirrors `resolve_operation` under the empty alias and is used only by tests. Source names are unique package-wide, so the map lookup is unambiguous.
- Model-typed members on the source route work end to end (probe, variant S-M with `object Z { t: M::Total; }`). `t = 3` admits and `ZI` holds. `t = 99` is refused `invalid_runtime_input`/`invalid-value`, as `M::Total`'s bound requires.

## Verdict

The source route is sound on the main path, and the coder's claims hold. Two medium defects remain on edges. Unbounded collection members are refused through a dead match arm. Model types in source members are resolved across unit boundaries.
