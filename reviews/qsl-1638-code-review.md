---
id: SR-20261104163645
title: "QSL-1638 code review (Rust lane) of commit ee0f6c400"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; commit ee0f6c400 (PR #2301), QSL-1638 qsl-semantics files only; qsl-semantics/src/check/{assemble.rs,check.rs,hyper_clause.rs,mod.rs,model_spelling.rs,protocol_slots.rs,refinement.rs,terminal.rs,timed_model.rs,timed_model/reading.rs,lowering/model.rs,lowering/terminal.rs,protocol_clause/mod.rs,protocol_clause/coverage.rs}; qsl-semantics/src/check/assemble/tests.rs; qsl-semantics/tests/it/timed_model.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
---

## Summary

Ticket: QSL-1638. The commit landed on main titled "QSL-1645: ..." (PR #2301). Only its 16 qsl-semantics files are reviewed here; the qsl-replay fixture is not.

The change keys these model-alias lookups by the writing unit through `ModelSpellings::selection`:

- whole-model protocol roles and Port roles (Port map keyed by package identity and artifact)
- the assembler's terminal join, with a new `TerminalClauseDeclaration::package` field used by the checker and lowering
- the population-bound join, keyed by (alias, package identity)
- the timed-model reader's `M::T` parameter type

For one alias bound to two packages in two units, each changed lookup now resolves the writing unit's own package. For the common layout, where two units each write `model M = X;`, both units bind `M` to the one `packages` slot of `X`, and the assembler admits one `AdmittedModel` per selection with the same alias and identity. Every changed lookup therefore finds the same package, and duplicate Port keys collapse in the `BTreeMap`.

Single-unit packages fall back to the primary unit through `ModelSpellings::package`. Import aliases sit in `others`, so `selection` returns `None` for them, as the removed `model_selections` map did. `TerminalClauseDeclaration` has one struct literal in the workspace (assemble.rs:5355). `TerminalInput` has one, and `port_roles` has one caller. No other crate constructs these.

Test evidence on ee0f6c400 (origin/main), focused runs:

- `cargo test -p qsl-semantics --lib -- check::`: 597 passed.
- `cargo test -p qsl-semantics --features test-support --test it -- timed_model model_identity_links dispatch_linking library_resolution`: 45 passed. Without `test-support` the `timed_model` module is compiled out and only 16 run.
- `cargo test -p qsl-replay --test it -- aggregate_dispatch static_subject`: 17 passed. This is the common-layout check that caught #2287's regression.

Mutation spot checks:

- Dropping the package match in `terminal_declarations` fails `a_terminal_on_a_shared_alias_is_on_its_own_units_package` (assemble/tests.rs:4857).
- Making `Declarations::written_models` package-wide fails `a_clock_reading_parameter_names_its_own_units_package` (tests/it/timed_model.rs:1146).

## Verdict

**CONDITIONAL.** The changed lookups are correct for both layouts. They are idiomatic: an iterator-returning `port_roles`, a private `Package` struct, `SourceScope` accessors replacing the scope-wide ones, and no panics. The tests are real.

The ticket's goal is that one alias bound to two packages in two units resolves per unit, and that is still not met package-wide. The time-source join and about a dozen other `model.alias() == Some(alias)` joins still pick the first admitted model. The new clock-reading test already has this layout: its unit `b` writes `time Config`. It passes only because it asserts no refusal (FND-001). The test-support hyper-clause scope gained spellings but no source owners (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Several model joins still find a model by alias alone, so in the one-alias-two-packages layout a later unit's declaration binds the first unit's package. FR-480 "Selections and imports" says a model alias resolves only within the unit that declares it. The affected joins: the time-source join that builds each timed model, the time-source `DuplicateTimeSource` check (keyed by the alias name) and model-node lookup, the tick time-source join, the refinement abstract/concrete model joins, the hyper relation execution joins, the protocol relationship export join, the equivalence join, and the probabilistic claim/model joins. Fix: resolve each through `spellings.selection(source_owners.get(<declaration name>), alias)` and match `model.selection()` identity, as `terminal_declarations` now does, and key `seen_time_sources` by (alias, identity). | qsl-semantics/src/check/timed_model.rs:4351-4355; qsl-semantics/src/check/mod.rs:1735-1750; qsl-semantics/src/check/mod.rs:5069-5071; qsl-semantics/src/check/mod.rs:4032-4039; qsl-semantics/src/check/mod.rs:4947-4949; qsl-semantics/src/check/mod.rs:5353-5355; qsl-semantics/src/check/refinement.rs:820; qsl-semantics/src/check/refinement.rs:2326; qsl-semantics/src/check/refinement/protocol.rs:815; qsl-semantics/src/check/protocol_relationship.rs:120; qsl-semantics/src/check/assemble/equivalence.rs:348; qsl-semantics/src/check/hyper_clause.rs:1619; qsl-semantics/src/check/hyper_clause/emission.rs:68; qsl-semantics/src/check/probabilistic/claim.rs:746; qsl-semantics/src/check/probabilistic/claim.rs:1894; qsl-semantics/src/check/probabilistic/model.rs:658 |
| FND-002 | low | `check_hyper_clauses` (test-support) now passes the package's `model_spellings` but no `with_source_owners`. Every per-unit lookup in that scope therefore reads every declaration as the primary unit's, which is inconsistent with the main checker scope (mod.rs:2167-2168). Fix: add `.with_source_owners(package.source_owners.clone())`. | qsl-semantics/src/check/hyper_clause.rs:3485 |
