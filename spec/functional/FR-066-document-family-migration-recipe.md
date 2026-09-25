---
id: FR-066
title: "Document the semantic-family migration recipe"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-005
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: traces_to
---
# FR-066: Document the semantic-family migration recipe

## Description

Function declaration and application is this ticket's sole migrated family;
`StateModel`, `SumCase`, `TemporalTrace`, `ProtocolClause` and `Relation`, and
the remaining `Value` forms, migrate under their own tickets. QSL SHALL
publish, as a checked-in document in this repository, the migration recipe
those later tickets follow: the required tests, the required conversions,
and the conditions under which a family's old composed-checker path is
removed.

The recipe is a documented procedure: each later ticket's implementation
deletes its family's old composed-checker path in the same change that
lands that family's checked-family replacement (ADR-011 §7.3 M-6e), so at no
point does an old path and its replacement both run for the same family.
[FR-065](FR-065-migrate-function-application-to-checked-family.md) performs
this deletion for function declaration and application directly, as this
ticket's own migrated family; this recipe covers the families that follow
it.

## Inputs

- ADR-012 §3 (family-owned responsibilities), §5.3 (evidence), §14.1
  (work-assignment table) and ADR-011 §7.3 (M-6e lane-deletion table).
- The function-application migration this ticket completes, as the recipe's
  own worked example.

## Outputs

- A checked-in recipe document naming, for a family migrating onto the
  contract:
  - the required tests (clause-level unit tests, builder-ordering tests
    where the family's construct has independently meaningful clauses, the
    family's seam-probe coverage, wire-totality tests for the family's
    checked-node and cause enums, and one backend-absence corpus case per
    capability kind the family's claim forms request);
  - the required conversions (the family's checked-node variant into the v2
    emitter, into the evaluator, and into requirement derivation; and, for a
    family whose forms cross into IR, RT or CG, the wire and IR-side
    conversions those repositories own);
  - the removal condition: the family's old composed-checker path is deleted
    in the same pull request that lands that family's S3 family checker and
    S4 emission, per ADR-011 §7.3 M-6e, with the family's own implementing
    ticket named as the one that performs the deletion.

## Behavior

### The recipe is a procedure, not new code

This requirement's deliverable is documentation only. It SHALL name no new
runtime type, trait or module; the contract those steps apply is FR-062's,
and the deletion timing they describe is ADR-011 §7.3's.

### The recipe cites the function-application migration as its worked example

The recipe SHALL walk through the function-application migration
([FR-065](FR-065-migrate-function-application-to-checked-family.md)) as a
concrete instance of each of its required-test, required-conversion and
removal-condition categories, naming the actual test files, conversion
functions and deleted symbols that migration produced.

### The recipe names every remaining family and its ticket

For each of `StateModel`, `SumCase`, `TemporalTrace`, `ProtocolClause` and
`Relation`, the recipe SHALL name the family's design ticket (ADR-012 §1)
and implementing ticket(s) (ADR-012 §14.1), so a later contributor can find
where that family's migration is tracked without re-deriving the mapping
from the ADRs.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-066-AC-1 | The recipe document exists at a checked-in path in this repository and names, for a migrating family, each of the five required-test categories (clause-level unit, builder-ordering, seam-probe, wire-totality, backend-absence corpus) by name, with a one-sentence description of what each verifies; a reviewer checklist that omits any one of the five categories does not satisfy this criterion. | Test (TC-165) |
| FR-066-AC-2 | The recipe names each of the three required-conversion categories (v2 emitter, evaluator, requirement derivation) and states, for the family whose forms cross into IR, RT or CG, that the wire and IR-side conversions belong to those repositories' own tickets, not to the QSL migration ticket. | Test (TC-165) |
| FR-066-AC-3 | The recipe states the removal condition in the same terms as ADR-011 §7.3 M-6e (the old composed-checker path is deleted in the PR that lands the S3 family checker and S4 emission) and names, for each of `StateModel`, `SumCase`, `TemporalTrace`, `ProtocolClause` and `Relation`, at least one implementing ticket from ADR-012 §14.1. | Test (TC-165) |
| FR-066-AC-4 | The recipe's worked example section names at least one actual test file and one actual deleted symbol from the function-application migration ([FR-065](FR-065-migrate-function-application-to-checked-family.md)'s implementation), not a hypothetical placeholder. | Test (TC-165) |

## Dependencies

- [ADR-012](../decisions/ADR-012-semantic-family-extension-contracts.md) §3
  (family-owned responsibilities table), §5.3 (evidence), §14.1
  (implementation work-assignment table).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §7.3 (M-6e lane deletion table).
- [FR-065](FR-065-migrate-function-application-to-checked-family.md) supplies
  the worked example this recipe cites.
- [US-005](../usecase/US-005-trust-checked-identity-across-packaging.md).

## Status

Specified under
[#214](https://github.com/agent-ix/quire-spec-language/issues/214).
Implemented as [docs/family-migration-recipe.md](../../docs/family-migration-recipe.md),
citing FR-065's implementation for its worked-example section.

**By Acceptance Criterion (QSL-151):** all four ACs are backed by a tagged
test in `tests/it/family_migration_recipe.rs` (`TC-165`), which reads the
checked-in recipe document via `include_str!` and asserts its required
content programmatically -- a source-inspection test, since this
requirement's own deliverable is documentation, not runtime code (see this
FR's "recipe is a procedure, not new code" section). Each test was verified
to fail when the content it checks is removed from the document:
AC-1/`recipe_names_all_five_required_test_categories_with_descriptions`,
AC-2/`recipe_names_all_three_required_conversion_categories_and_ir_ownership`,
AC-3/`recipe_states_removal_condition_and_names_a_ticket_per_remaining_family`,
AC-4/`recipe_worked_example_names_a_real_test_file_and_a_real_deleted_symbol`.
