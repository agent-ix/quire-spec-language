---
id: SR-20261104180255
title: "QSL-1638/QSL-1650 PR 2313 code review (Rust lane): remaining model-alias joins per unit"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; PR #2313 diff only (head 852525fff, trial merge with origin/main 509e09c7c); qsl-semantics/src/check/{assemble.rs,assemble/equivalence.rs,check.rs,hyper_clause.rs,hyper_clause/emission.rs,mod.rs,model_spelling.rs,probabilistic/claim.rs,probabilistic/model.rs,protocol_relationship.rs,refinement.rs,refinement/protocol.rs,temporal_clause/timed.rs,timed_model.rs}; qsl-eval/src/simulation/model/timed.rs; fixtures qsl-analyze/tests/it/monitor_plan.rs, qsl-eval/tests/it/timed_trace.rs, qsl-semantics/tests/it/timed_intervals.rs; consumers read for impact: qsl-package/src/emit.rs, qsl-route/src/deadlock.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: references
---

## Summary

Ticket: QSL-1638 (with QSL-1650). The PR finishes the per-unit alias resolution started in #2301. It adds `ModelSpellings::model` and `Scope::declared_model`, gives each `time` declaration a `time_source:<n>` source-owner key, keys `DuplicateTimeSource` by (alias, package identity), and matches time sources to models through the model node (`ModelTimeSource::times`, `TimedModel::is_timed_by`).

What was checked and holds:

- Time-source owner keys. `Unit::new` counts `TimeSource` forms over the combined forms. `PackageDeclarations::assemble_units` counts them per unit in the same order, because `ParsedUnit::combine` appends each unit's forms in order. Both count before the clock-form `Drop` disposition, so the two indices agree. A single-unit `assemble` has empty `source_owners` and falls back to the primary unit.
- Common layout (the #2287 class: two units each `model M = X;`). `ParsedUnit::combine` keeps one selection per (alias, identity), so only one `AdmittedModel` exists. Every new join returns it. `two_units_timing_one_package_under_one_alias_is_a_duplicate` and qsl-replay `aggregate_dispatch`/`static_subject` (17 passed) cover this.
- The changed joins: refinement sides and the class catalog (`declared_model`, then selection-matched), refinement protocol `side_of`, the `TimedAbstract` check, the hyper `model_of`/`operation_key`/`receiver_object`/relation executions, the emission systems-interface rule, protocol relationship exports, dimension-equivalence `Scope::model`, the probabilistic claim subject and fairness, the tick-anchor join, the timed-model/source joins in mod.rs, and the dense-refinement marker. Each resolves through the declaring unit's `ModelSpellings` and a selection or model-node match.
- `TimedModel` gains a private `model_node` field and the `model_node()` and `is_timed_by()` accessors. It has one constructor (timed_model.rs `check`), so no consumer constructs it. `ModelTimeSource` has a public `model_node` field (unchanged), constructed in mod.rs and in the three fixtures, all updated to the real node key. qsl-eval `TimedSystem::with_max_clocks` now joins through `is_timed_by`.
- Idioms: no panics, no new `unwrap`, iterator joins, `const fn`-preserving struct-variant matches. No compatibility layer. No QSpec text is reproduced.

Evidence (trial merge of 852525fff with origin/main 509e09c7c, clean merge):

- `cargo check --locked --workspace --all-targets`: rc 0.
- `qsl-semantics --lib check::`: 598 passed.
- `qsl-semantics --test it` (timed, refinement, hyper, probabilistic, dimension_equivalence, model_intake, model_operations, terminal, population, protocol): 98 passed without `test-support` and 440 passed with `--features test-support`.
- `qsl-replay --test it aggregate_dispatch static_subject`: 17 passed. `qsl-analyze --test it monitor_plan`: 15 passed.
- `qsl-eval --test it timed` with QSPEC_DIR set to a detached QSpec origin/main worktree: 94 passed.
- `cargo clippy --locked --workspace --all-targets -D warnings`, default and `--all-features`: both rc 0.
- Mutation 1: making `ModelTimeSource::times` alias-only fails `each_units_time_source_times_its_own_units_package` and `a_clock_reading_parameter_names_its_own_units_package`.
- Mutation 2: making `ModelSpellings::model` alias-only fails all six new per-join tests (equivalence, hyper clause, step relation, refinement, claim subject, time source). The tests fail on a revert to alias-only joins, as the PR claims.
- Probe A (scratch test, reverted): unit a `model Config = X; time Config`, unit b `model Other = X; time Other`. The check accepts it with 2 time sources and 2 timed models for the one package (FND-003).
- Probe B (scratch test, reverted): unit a binds `R` to `acme/clock`, which has no unit `s`. Unit b binds `R` to `acme/rpc`. Both write `dimension R::Time ~ F::Time by 1 R::s = 1 F::s`. Assembly refuses only `Conflicting` between the two equivalences. It does not report a's `R::s` as unresolved, so a's form was read through b's bindings (FND-004).

## Verdict

**CONDITIONAL.** The joins this PR changes resolve through the declaring unit and cannot cross-bind another unit's alias. The common layout is unchanged. The `TimedModel.model_node` change breaks no consumer. The tests fail under an alias-only revert. All focused tests, both clippy configurations and the locked workspace check pass on the trial merge.

Fix in this PR:

- FND-001: the emitter's alias joins. This PR newly admits the layout in which they cross-bind.
- FND-003: the duplicate-time-source key.
- FND-004: the equivalence owner key. The PR now resolves aliases through this key.

All three are small. FND-002 is the probabilistic family from the earlier review's own list. Fix it here, or have the lead defer it explicitly with a ticket. Do not carry it as "outside the list".

The author's other remaining joins are follow-ups, not blockers for this PR:

- Terminal joins mod.rs:3190/5617/6129 are safe while `AmbiguousTerminal` refuses two terminals on one alias spelling. That refusal over-refuses the two-package layout, which is a follow-up.
- Refinement row names: no wrong result was found.
- The public alias-addressed `CheckedGraph::timed_model`/`terminal`/`model_package_identity`, read by qsl-eval `TimedSystem` and qsl-route `SubjectReading`, need a package-addressed API. With FND-001 fixed, they only pick the first of two same-alias models.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Before this PR, two units writing `time M` under one alias was refused as `DuplicateTimeSource`. It is now accepted when the units bind `M` to two packages, and the checked graph then holds two `TimedModel`s whose `model()` is `M` (the PR's own `each_units_time_source_times_its_own_units_package` builds exactly this). The S4 emitter still joins timed models to time sources by alias. For each source, `clock_places` and the urgency nodes are collected from every timed model with that alias, and `clause_nodes` comes from the first. Each package's emitted timed model therefore carries the other package's clock places and urgencies. Fix in this PR: use `timed.is_timed_by(source)` at the three joins, and add an emit test for the two-package layout. | qsl-package/src/emit.rs:5532-5536; qsl-package/src/emit.rs:5556-5560; qsl-package/src/emit.rs:5655-5659 |
| FND-002 | medium | Probabilistic operation resolution is still alias-only, and it was in the earlier review's FND-001 list (probabilistic/model.rs:658, now 669). The PR body files it as "outside the review's list". `model_objects` is built by `type_tables` keyed by alias with the last model winning (mod.rs:7248-7263), and `ProbabilisticInput::operation` takes the key from the first model of the alias. Reward joins match `reward.operation().model() == alias` (claim.rs:1564, 1937). In the two-package layout, a probabilistic operation, delay or reward of one unit binds the other unit's package. The newly fixed delay/claim time-source joins then follow that wrong key. The new claim test avoids the path on purpose: it reads no `Service::` operation or enumeration. Fix: resolve through the declaring unit (`scope.declared_model`) and key `model_objects`/rewards by package identity, with a two-package test that reads an operation. Otherwise defer explicitly with a ticket. | qsl-semantics/src/check/probabilistic/model.rs:629-673; qsl-semantics/src/check/mod.rs:7248-7263; qsl-semantics/src/check/probabilistic/claim.rs:1559-1565; qsl-semantics/src/check/probabilistic/claim.rs:1931-1938 |
| FND-003 | medium | `DuplicateTimeSource` is keyed by (alias, identity). FR-230 admits at most one `time` member per state model, and FR-480 lets two units bind two aliases to one identity. In that case `assemble` admits one `AdmittedModel` per alias (assemble.rs:3591), so `time A` in unit a and `time B` in unit b, both on package X, give two time sources for one model with no refusal (Probe A: accepted, 2 time sources). Fix: key `seen_time_sources` by the resolved package identity (the model node), falling back to the alias only when the alias did not resolve. | qsl-semantics/src/check/mod.rs:1729-1760 |
| FND-004 | medium | The dimension-equivalence owner key is the written spelling, `@dimension-equivalence:<d0>:<d1>` (equivalence.rs:492-497). Two units that write the same spelling with different alias bindings share one `source_owners` entry, and the later unit wins. `Scope::model` now resolves aliases through that entry, so the earlier unit's equivalence resolves through the later unit's bindings (Probe B). Fix: key by declaration position, like `time_source:<n>`. | qsl-semantics/src/check/assemble/equivalence.rs:492-497; qsl-semantics/src/check/assemble.rs:3300-3304 |

## Dispositions

| ID | Disposition |
| --- | --- |
| FND-001 | fixed: the three emitter joins read the time source's timed models through `timed_models_of` (`TimedModel::is_timed_by`). Test `two_units_timing_two_packages_under_one_alias_emit_their_own_clock_places` (qsl-package) fails when the helper joins by alias. S4 cannot yet write both timed models of this layout: their declarations are both named `<alias>::Timed` and the source map refuses the duplicate (`SourceMapInvariant`). The test therefore pins the join on the checked graph, not the written package. Follow-up under QSL-1663. |
| FND-002 | deferred: probabilistic operation resolution, `model_objects` and reward joins, to QSL-1663 with the exact sites. Not fixed in this PR. |
| FND-003 | fixed: `DuplicateTimeSource` is keyed by the resolved package identity (the alias only when it did not resolve). Test `two_aliases_of_one_package_timed_twice_are_a_duplicate`; fails with the old key. |
| FND-004 | fixed: the equivalence owner key is the declaration position, `dimension_equivalence:<n>`. Test `two_units_writing_one_spelling_resolve_each_through_its_own_bindings`; fails when the key ignores the position. |
