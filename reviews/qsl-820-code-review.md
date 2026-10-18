---
id: SR-12087
title: "QSL-820 PR #940 code review: authored population bound and the reaches population link"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c0e524159e32cba32d995d2163623878f95d921f; qsl-cst/src/{cst,format,grammar}.rs, qsl-forms/src/{dispatch,lib,state_model,syntax,value}.rs, qsl-semantics/src/check/{assemble,family,ir,lowering,lowering/model,mod,node_key/mod,state_clause,state_model}.rs, qsl-semantics/src/model/{object_environment,observation/document,observation/initial}.rs, qsl-semantics/src/value/member.rs, qsl-eval/src/value/expression/evaluate.rs, qsl-package/src/emit.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/refinement.rs, src/complete/editor.rs and the diff's tests"
review_set: subset
---

## Summary

Ticket: QSL-820. PR quire-spec-language#940 at head c0e524159, diff `origin/main...HEAD` (42 files, +1755/-85). This is a code review with the rust-review lane folded in. Read-only: nothing was built or run. The merged tree has not been compiled yet, so the review looked for compile errors by reading. As briefed, only major bugs are reported and gates are not flagged.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new S2 arm `LeadingTokenKind::Bound => state_model::population_bound(construct)` is exactly what the xtask seam probe rejects. `forms_s2_report` returns `SeamProbeMismatch` for any `state_model::` call in the `dispatch` match ("no StateModel arm in the declaration dispatch (ADR-016 FP-1, FP-3)"), and `seam_probe::run` calls it first. So `make seam-probe` (in `CI_CHECKS`) fails. The TC-791 xtask tests fail too: `state_model_arm_report_reads_the_production_matches` unwraps the report, and the three scratch-copy tests unwrap `state_model_arm_report` on the copied real `dispatch.rs` before they mutate it. FR-380 requires a `StateModel` S2 form, but ADR-016 FP-1/FP-3, TC-791 and the probe still say S2 has no StateModel arm by design, and the PR leaves all three as they are. To fix: amend ADR-016 FP-1/FP-3 (its own reopen condition), TC-791 and `forms_s2_report` so that the one `Bound` arm is expected, with the probe naming it. Renaming the module to get past the probe is not a fix. | qsl-forms/src/dispatch.rs:386, xtask/src/seam_probe.rs:719-733, xtask/src/seam_probe.rs:752-768, xtask/src/seam_probe.rs:842-867 |
| FND-002 | medium | The authored bound is checked only inside a `ReachesEnd` node that is actually evaluated (`check_authored_bound` in `reaches_end`). The checker still marks the clause's context population as `Population(Some(N))`, and drops it from the unbounded domains in `record()`, so a context-only clause such as `invariant I using v on M::Order at current { true }` with `bound OrdersBound on M::orders [3]` gets extent `Bounded` (this PR's own test `a_bound_on_the_context_population_alone_bounds_a_context_only_clause`). Evaluating that clause over an observation listing 4 orders refuses nothing. The same happens to a reaches clause whenever its reaches node is not evaluated, for example when a connective short-circuits. So a claim whose record states a finite domain of at most N runs over N+1 objects and gets an ordinary verdict. FR-380-AC-7 says an observation whose *selected population* holds N+1 objects refuses. The spec sentence this PR added ("It applies to the population a relationship-end `reaches` walks") makes this narrower, but it does not cover the context population, which the record also bounds. Either refuse at clause entry for every bounded population the clause's record names, or do not count the context population as bounded. | qsl-eval/src/value/expression/evaluate.rs:321-340, qsl-eval/src/value/expression/evaluate.rs:2306-2311, qsl-semantics/src/check/state_clause.rs:892-897, qsl-semantics/src/check/state_clause.rs:704-707 |

## Verdict

FIX. FND-001 turns `make ci` red on merge, deterministically. FND-002 lets a claim recorded as bounded be evaluated past its bound and get an ordinary verdict.

Checked and clean, with no finding:

- **Exhaustive matches.** Every exhaustive `DeclarationForm` match in the diff lists `PopulationBound` with no wildcard: assemble.rs and the three `protocol_clause_forms.rs` and two `value_forms.rs` helpers. The same holds for the `Production` lists in `value.rs` (two), `editor.rs` and `complete_grammar.rs`, for `AssemblyCause::code`/`catalog_code`, `assembly_limit` (refinement.rs) and `assembly_message` (spine.rs), and for `Member::population`/`to_wire`/`map_member`. The `_ => "other"` in `protocol_reference_forms.rs` predates this PR.
- **Compile reading.** `PopulationDomain` stays `Copy` (`Option<u64>`). `AuthoredBound` derives everything `NodeKind` (`Clone, Debug, PartialEq`) needs. `Construct` is `Copy`, so `production_node(construct)` followed by `construct.node.span()` compiles. The `value::` helpers state_model.rs imports are `pub(crate)`. The lowering `ReachesEnd` destructure now lists all six fields. The new test module's helpers (`package`, `invariant`, `clause`, `assert_refusal`, `parse_and_build`) are crate-visible and sit under the same `test-support` cfg.
- **Grammar and formatter.** `bound`/`on` are reserved Grammar tokens, so the only Identifier token directly under the node is the label. The formatter breaks the line at the `PopulationBound` node's end offset (the `]`), and the test checks one round trip.
- **Node-key determinism.** The population node is `population_type(near, Some(N))`, so N is in its key. The `Type` occurrence is keyed by `bound <label>` (spaced, so it cannot collide with a type name), located at `bracketed`, and recorded once per (label, node). `order_by` reorders only population nodes, after lowering finishes. `parameter_spans` are keyed by parameter nodes and are not affected. `population: None` is skipped on the wire, so unbounded reachabilities keep their ids. There are no new `NodeKey` or `EffectiveId` mints.
- **Refusals.** `UnresolvedPopulationBound` maps to missing_declaration/missing-name and `AmbiguousPopulationBound` to ambiguous_declaration/ambiguous-name. Both have explicit no-limit arms in `assembly_limit`. The tests match on typed fields and spans.
- **Evaluator, off-by-one and absence.** The refusal fires on `count > maximum`. `CardinalityBound::new(0, N)` is `[0, N]`. A population the snapshot does not list (`population_size` `None`) is not refused. Snapshot sizes are keyed by `population.key.node`, the same identity `find_population` matches, so the admitted path and the bound agree.
- **Panics.** No new `unwrap`, `expect` or indexing on a production path. `u64::try_from(len).unwrap_or(u64::MAX)` saturates.
- **The 85 removed lines** are all replaced or widened in place: imports, signatures, the one-line `;` rule, test helpers made `pub(super)`, and spec AC rewrites. No unrelated deletion.

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | ADR-016 FP-1 and FP-3 now state that the `bound` declaration is the one `StateModel` S2 entry (`LeadingTokenKind::Bound => state_model::population_bound`) and add no S6a entry, `evaluate` hook or `S6aFamilyKind`. FR-300 "Seams" and FR-300-AC-3, and TC-791 (title, description, steps, expected results), expect exactly that arm. `forms_s2_report` in `xtask/src/seam_probe.rs` accepts exactly that arm and rejects any other `state_model::` arm and the arm's absence. TC-791 xtask tests: the report test expects the new S2 line, the extra-arm test still rejects `state_model::declaration`, and a new test rejects a renamed or removed `Bound` arm. |
| FND-002 | fixed | The checked clause records the authored bound of every bounded population its claims name (`CheckedStateClause::population_bounds`), and `evaluate_clause` refuses `cardinality_out_of_bound`/`above-maximum` before any charge when an observation lists more objects than a bound allows. Test `a_bounded_population_refuses_for_a_clause_that_never_evaluates_a_reaches` covers a context-only clause and a short-circuited `reaches`. FR-380 and TC-1054 row 13 state it. |
