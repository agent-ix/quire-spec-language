---
id: SR-13112
title: "Code review of quire-spec-language PR #1032: QSL-544 part 1, map_state and the ModelSystem refinement seam (FR-140)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@54d8d72b6; PR #1032 (git diff origin/main...HEAD, base 337871901): qsl-eval/src/simulation/model.rs, qsl-eval/src/simulation/model/refine.rs, qsl-eval/src/value/expression/mod.rs, qsl-eval/src/value/mod.rs, qsl-replay/src/lib.rs, qsl-replay/src/refinement.rs, qsl-replay/src/refinement/map.rs, qsl-replay/tests/it/{main,refinement_fixture,refinement_mapping}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: reviews
---
# SR-13112: Code review of quire-spec-language PR #1032

## Summary

Ticket: QSL-544 (E13), part 1 of the slice. Code review with the rust-review
lane folded in.

The PR adds `qsl_replay::map_state` and a `ModelSystem` seam in `qsl-eval`:
`read_row`, `state_key_of`, `object_reference`, `fields_of`, `type_key_of`
and `ModelSystem::over_model`. It also adds `qsl_eval::evaluate_refinement_row`,
which follows the pattern of `evaluate_terminal_predicate`: `Machine::with_pre`
with no pre-state and an empty read map. The main path is correct. Population
rows map in order, objects map by key, each field row is evaluated with
`self` bound to the concrete object and a fresh meter, a top-level reference
is lifted to the abstract reference with the same key, and every failure
path returns no partial state.

Gates run locally (not `make ci`):

- `cargo test -p qsl-replay --test it refinement`: 4 of 4 pass.
- `cargo clippy -p qsl-replay -p qsl-eval --all-targets -- -D warnings`: clean.
- `make arch-lint-api-surface-qsl`: T12-B to T12-F and FR-100-AC-8 pass,
  T12-C included.
- `arch-lint qualified-core`: direction and ambient-input checks pass.
- `make arch-lint-direction` was not run because it needs the IR, RT and CG
  clones.

There is no compatibility shim. `ModelSystem::new` now delegates to a private
`over`, and the old inline `StateModel::new` call is removed, not kept beside
the new one. The two `#[allow(clippy::result_large_err)]` attributes give a
reason and match 23 existing uses in the workspace, so they are not a
finding.

## Verdict

**No blocking defect.** One medium finding: an optional-field unwrapping rule
that no spec states and no test exercises. Three low findings: untested
failure branches, per-row observation rebuilds, and a misleading cause on an
unreachable arm. The deviations the coder listed are judged in SR-13113, the
gap analysis.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `map_object` adds an unspecified rule for an optional abstract field: a row value `Option(None)` leaves the field unset, `Option(Some(v))` stores `v`, and a non-option value is stored as is. FR-140 and ADR-020 RM-3/RM-4 say nothing about optional fields, and no test covers any of the three cases. A wrong rule here would change the mapped state key, and with it every FR-141/FR-142 verdict. Either drop the rule and refuse optional targets until the spec states one, or get the rule specified and add a test per case. | qsl-replay/src/refinement/map.rs:269-279 |
| FND-002 | low | Every failure branch except `Undetermined(Evaluation)` is untested: `MappingFailure::Undefined` (map.rs:258-264, and `reading`'s two `Undefined` arms at refine.rs:119-121 and 135-137), `UnmappedType` (map.rs:195, 349), `Unsupported(NestedReference)` (map.rs:317), `Stopped` (map.rs:236-244, 254, 345, 352), and `over_model`'s `wrong-role-mapping` refusal (model.rs:408-414). The `Undefined` arm is acceptable for part 1. FR-140-AC-3 is assigned to QSL-546 and needs FR-139's `only`. My probes showed the counter fixture cannot reach `Undefined`: `/` fails typing, and `div`/`rem` refuse `MissingSelection{IntegerDivision}`. `UnmappedType` and the `over_model` refusal can be tested now with small fixture variants. | qsl-replay/src/refinement/map.rs:258-264; qsl-eval/src/simulation/model/refine.rs:119-137; qsl-eval/src/simulation/model.rs:408-414 |
| FND-003 | low | `read_row` recomputes the concrete state's FR-101 key and rebuilds its whole synthesized observation on every call. `map_state` calls it once per object per field row, so one `map_state` costs objects × rows observation builds of the full state. The model checker (FR-142) will call it for every reachable concrete state. Build the observation once per `map_state` call and evaluate every row against it. | qsl-eval/src/simulation/model/refine.rs:46-66 |
| FND-004 | low | `reading` sends a `FamilyOutcome` other than `Evaluated`/`FamilyEvaluated` (the probe-only variant, FR-063) to `RowReading::Undetermined(ClauseOutcome::NotBoolean)`. A value-typed row has no Boolean expectation, so that cause would mislead if the arm were ever reached. It should be a `BROKEN` stop, because the arm is unreachable for an admitted row. | qsl-eval/src/simulation/model/refine.rs:140 |
