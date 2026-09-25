// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-165: the checked-in family-migration recipe
//! (`docs/family-migration-recipe.md`) names every required test category,
//! required conversion category, the ADR-011 M-6e removal condition with an
//! implementing ticket per remaining family, and a real worked-example
//! artifact. FR-066's deliverable is the recipe document itself, not a new
//! runtime type (FR-066's own "recipe is a procedure, not new code"
//! section), so this is a source-inspection test over that document's
//! checked-in text rather than a behavioral test over running code.

use ix_trace_rs::trace;

const RECIPE: &str = include_str!("../../docs/family-migration-recipe.md");

fn required_tests_section() -> &'static str {
    section("## Required tests", "## Required conversions")
}

fn required_conversions_section() -> &'static str {
    section("## Required conversions", "## Removal condition")
}

fn removal_condition_section() -> &'static str {
    section("## Removal condition", "## Worked example")
}

fn worked_example_section() -> &'static str {
    let start = RECIPE
        .find("## Worked example")
        .expect("recipe has a worked-example section");
    &RECIPE[start..]
}

fn section(start_heading: &str, end_heading: &str) -> &'static str {
    let start = RECIPE
        .find(start_heading)
        .unwrap_or_else(|| panic!("recipe has a {start_heading:?} section"));
    let end = RECIPE[start..]
        .find(end_heading)
        .unwrap_or_else(|| panic!("recipe has a {end_heading:?} section after {start_heading:?}"));
    &RECIPE[start..start + end]
}

/// Markdown prose wraps at a fixed column, so a checked-in sentence this
/// test wants to match as one phrase is routinely split across a line break
/// in the source file. Collapsing all whitespace runs (including newlines)
/// to a single space lets phrase assertions below match the document's
/// prose regardless of where its editor wrapped a line.
fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// FR-066-AC-1: the recipe names each of the five required-test categories
/// by name, each with a one-sentence description of what it verifies. Each
/// assertion below checks both the category's name and a phrase from its
/// description, so deleting the description (while leaving a bare heading)
/// fails the test, not just deleting the category entirely.
#[trace("TC-165", "FR-066-AC-1")]
#[test]
fn recipe_names_all_five_required_test_categories_with_descriptions() {
    let tests = normalize(required_tests_section());

    assert!(
        tests.contains("Clause-level unit tests.")
            && tests.contains("checking that clause in isolation"),
        "clause-level unit tests category with its description"
    );
    assert!(
        tests.contains("Builder-ordering tests") && tests.contains("typed clause-order refusal"),
        "builder-ordering tests category with its description"
    );
    assert!(
        tests.contains("seam-probe coverage") && tests.contains("`seam_probe` cfg"),
        "seam-probe coverage category with its description"
    );
    assert!(
        tests.contains("Wire-totality tests") && tests.contains("vacuously exhaustive"),
        "wire-totality tests category with its description"
    );
    assert!(
        tests.contains("backend-absence corpus case")
            && tests.contains("typed backend-absence outcome"),
        "backend-absence corpus category with its description"
    );
}

/// FR-066-AC-2: the recipe names the three required-conversion categories
/// and states that a family crossing into IR, RT or CG has its wire/IR-side
/// conversions owned by those repositories' own tickets, not the QSL
/// migration ticket.
#[trace("TC-165", "FR-066-AC-2")]
#[test]
fn recipe_names_all_three_required_conversion_categories_and_ir_ownership() {
    let conversions = normalize(required_conversions_section());

    assert!(
        conversions.contains("The v2 emitter"),
        "v2 emitter conversion category"
    );
    assert!(
        conversions.contains("The evaluator"),
        "evaluator conversion category"
    );
    assert!(
        conversions.contains("Requirement derivation"),
        "requirement derivation conversion category"
    );
    assert!(
        conversions.contains("the wire and IR-side conversions belong to those repositories' own tickets, not to the QSL migration ticket"),
        "IR/RT/CG-owned conversions statement"
    );
}

/// FR-066-AC-3: the recipe states the removal condition in ADR-011 §7.3
/// M-6e's own terms and names at least one implementing ticket for each of
/// the five remaining families.
#[trace("TC-165", "FR-066-AC-3")]
#[test]
fn recipe_states_removal_condition_and_names_a_ticket_per_remaining_family() {
    let removal_raw = removal_condition_section();
    let removal = normalize(removal_raw);

    assert!(
        removal.contains("the family's old composed-checker path")
            && removal.contains(
                "is deleted in the same pull request that lands that family's S3 family checker and S4 emission"
            ),
        "M-6e removal condition stated in ADR-011 §7.3's own terms"
    );

    for family in [
        "StateModel",
        "SumCase",
        "TemporalTrace",
        "ProtocolClause",
        "Relation",
    ] {
        // The per-family ticket table is untouched by line wrapping, so
        // matching on the raw (non-normalized) text and slicing to the row's
        // own newline correctly scopes the `#`-ticket check to that family's
        // table row rather than to the whole removal-condition section.
        let heading = removal_raw
            .find(&format!("`{family}`"))
            .unwrap_or_else(|| panic!("removal condition names family {family}"));
        let row = &removal_raw[heading..];
        let row_end = row.find('\n').unwrap_or(row.len());
        assert!(
            row[..row_end].contains('#'),
            "family {family} has at least one implementing ticket named in its table row"
        );
    }
}

/// FR-066-AC-4: the worked-example section names at least one real test
/// file and one real deleted symbol from the function-application
/// migration, not a hypothetical placeholder.
#[trace("TC-165", "FR-066-AC-4")]
#[test]
fn recipe_worked_example_names_a_real_test_file_and_a_real_deleted_symbol() {
    let worked_example = worked_example_section();

    assert!(
        worked_example.contains("family_contract_tests"),
        "worked example names a real test file/module (family_contract_tests)"
    );
    assert!(
        worked_example.contains("Real deleted symbols from this migration")
            && worked_example.contains("`FamilyContract::package`"),
        "worked example names a real deleted symbol (FamilyContract::package)"
    );
}
