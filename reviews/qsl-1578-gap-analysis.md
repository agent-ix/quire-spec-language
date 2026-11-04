---
id: SR-1578102
title: "Gap analysis of PR #2240: FR-480-AC-9 repeated model aliases"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@2b4bbc1a5; FR-480-AC-9 and the FR-480 repeated-alias bullet vs qsl-semantics tests and src/check changes in PR #2240; ticket QSL-1578"
review_set: subset
date: '2026-11-04'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-480
    type: references
---

## Summary

Ticket: QSL-1578. Planless, PR-scoped gap analysis. Plan completion: not assessed. Every clause
of FR-480-AC-9 was mapped to the tests tagged `FR-480-AC-9`, and each test was checked against
the production path it exercises.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-480-AC-9 | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:242 | examined |
| FR-480 repeated-alias bullet | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:120-133 | examined |
| a_second_alias_names_every_package_type_as_the_first_alias_does | qsl-semantics/src/check/assemble/tests.rs | examined |
| a_second_alias_projects_no_package_fact_twice | qsl-semantics/src/check/assemble/tests.rs | examined |
| a_second_alias_neither_shadows_nor_is_shadowed_by_a_unit_declaration | qsl-semantics/src/check/assemble/tests.rs | examined |
| repeated_aliases_root_each_relationship_once_at_the_first_alias | qsl-semantics/src/check/lowering/model/tests.rs | examined |
| a_second_alias_names_the_same_variant_record_and_object_types | qsl-semantics/tests/it/variant_fields.rs | examined |
| inherited_members_resolve_through_a_second_alias | qsl-semantics/tests/it/model_operations.rs | examined |
| model_spelling unit tests | qsl-semantics/src/check/model_spelling.rs | examined |
| aggregate_dispatch FR-480-AC-3 tests | qsl-replay/tests/it/aggregate_dispatch.rs | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-480 bullet promises two things no test covers: `N::T` as a `lookup` type argument, and the two-unit case ("one unit, or two units") for scalar, composite and variant spellings. AC-3's qsl-replay tests cover only object-type operation contexts across units. Neither promise is in FR-480-AC-9, so this is a gap in the statement, not in the AC. Fix: add a `lookup<History::ConfigVersion>` case to the position loop in `a_second_alias_names_every_package_type_as_the_first_alias_does`, or narrow the bullet. | spec/functional/FR-480-compile-and-link-a-package-from-several-source-units.md:120-133 |
| FND-002 | low | FR-480-AC-9 says the profile/model alias collision refuses `ambiguous_declaration`/`ambiguous-name`, but the test only matches `AssemblyCause::DuplicateAlias`. The mapping to that code and cause does exist (assemble.rs:534, 623), but nothing asserts it, so an AC-level change to the mapping would not fail this test. Fix: also assert `error.cause.code()` and the cause string in the test. | qsl-semantics/src/check/assemble/tests.rs:4396-4420 |

## Verdict

Mergeable. Every AC-9 clause has a tagged test that drives the real assembler and checker,
with no mocks. The no-duplication test compares the checked graph of the once-selected package
against the twice-selected one (model nodes, `model_root` count, effect rows, operation table),
which is a real differential oracle. Both findings are low.

Focused test evidence at 2b4bbc1a5: `cargo test -p qsl-semantics --lib` filtered to the 7 tests the PR touches (a_second_alias x3, repeated_aliases_root_each_relationship_once_at_the_first_alias, model_spelling x2, model_population_projection_is_unique_across_package_aliases) passed 7/7. `cargo test -p qsl-semantics --test it` filtered to the 3 tests it touches (variant_fields x2, model_operations::inherited_members_resolve_through_a_second_alias) passed 3/3. `quire validate` on FR-480 is clean.
