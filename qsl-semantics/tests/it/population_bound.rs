// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-380 / TC-1054 rows 2 to 4 and 12: an authored `bound` on a
//! domain-package population resolves, makes the population a finite domain
//! of the requirements records that name it, and refuses where it is
//! ambiguous. Every expected value is the TC row's, over TC-1046's fixture R.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AssemblyCause, AssemblyRefusal, CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations,
    TypeFormFault,
};
use qsl_semantics::family::ClaimExtent;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::SelectedModels;
use quire_semantic_value::checking::CheckingLimits;

use crate::relationship_navigation::assert_refusal;
use crate::relationship_reachability::{clause, invariant, package};

const NO_LOOP: &str =
    "invariant NoLoop using v on M::Order at current { not reaches(self, self, next) }";

/// The unit over fixture R holding `declarations`, with the extra populations
/// `extra`, and its text.
fn unit(extra: &[(&str, &str)], declarations: &str) -> (String, DeclarationsOrRefusal) {
    let package = package(extra);
    let digest = qsl_semantics::model::key::hex(&package.model_selection.digest);
    let identity = package.model_selection.identity.clone();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model M = {identity:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         {declarations}\n"
    );
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let built = crate::model_operations::parse_and_build(&text);
    let raw = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1054", "fixture", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let assembled =
        PackageDeclarations::assemble(raw.source().reference().clone(), built, selected, vec![]);
    (text, assembled)
}

type DeclarationsOrRefusal = Result<PackageDeclarations, AssemblyRefusal>;

fn assembly_refusal(extra: &[(&str, &str)], declarations: &str) -> (String, AssemblyRefusal) {
    match unit(extra, declarations) {
        (text, Err(refusal)) => (text, refusal),
        (_, Ok(_)) => panic!("{declarations}: the assembler refuses"),
    }
}

fn check_with(
    extra: &[(&str, &str)],
    declarations: &str,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let (_, assembled) = unit(extra, declarations);
    assembled
        .expect("the unit assembles")
        .check(CheckingLimits::default())
}

fn checked(declarations: &str) -> CheckedGraph {
    check_with(&[], declarations).unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"))
}

/// The extents of every `claim` occurrence of the checked graph.
fn claim_extents(graph: &CheckedGraph) -> Vec<&ClaimExtent> {
    graph
        .requirements()
        .iter()
        .filter(|(key, _)| key.origin().role().as_str() == "claim")
        .map(|(_, record)| record.requirements().extent())
        .collect()
}

/// (TC-1054 row 2): an alias naming no model, a population naming
/// no population of the package and a maximum outside `u64` each refuse at
/// their own token.
#[trace("TC-1054", "FR-380-AC-2")]
#[test]
fn an_unresolved_alias_or_population_or_an_oversized_maximum_refuses() {
    for (declaration, name, code, cause) in [
        (
            "bound X on Nope::orders [8]",
            "Nope",
            Code::MissingDeclaration,
            "missing-name",
        ),
        (
            "bound X on M::nothing [8]",
            "nothing",
            Code::MissingDeclaration,
            "missing-name",
        ),
        (
            "bound X on M::orders [18446744073709551616]",
            "18446744073709551616",
            Code::IllTyped,
            "type-mismatch",
        ),
    ] {
        let (text, refusal) = assembly_refusal(&[], declaration);
        let [error] = refusal.errors.as_slice() else {
            panic!("{declaration}: one error: {refusal:?}");
        };
        assert_eq!(error.cause.code(), code, "{declaration}");
        assert_eq!(error.cause.catalog_code().cause(), cause, "{declaration}");
        assert_eq!(
            &text[error.span.start..error.span.end],
            name,
            "{declaration}"
        );
        match (&error.cause, code) {
            (AssemblyCause::UnresolvedPopulationBound { name: held }, Code::MissingDeclaration) => {
                assert_eq!(held, name, "{declaration}");
            }
            (AssemblyCause::IllFormedBounds(TypeFormFault::Malformed), Code::IllTyped) => {}
            (other, _) => panic!("{declaration}: unexpected cause {other:?}"),
        }
    }
}

/// (TC-1054 row 3): a repeated label, two bounds naming one
/// population (equal maximum or not) and a label a clause already uses each
/// refuse `ambiguous_declaration`/`ambiguous-name` at every declaration
/// involved.
#[trace("TC-1054", "FR-380-AC-2")]
#[test]
fn a_shared_label_or_two_bounds_on_one_population_refuse_at_each_declaration() {
    for (declarations, shared, labels) in [
        (
            "bound X on M::orders [8]\nbound X on M::people [2]",
            "X",
            vec!["X", "X"],
        ),
        (
            "bound A on M::orders [8]\nbound B on M::orders [8]",
            "M::orders",
            vec!["A", "B"],
        ),
        (
            "bound A on M::orders [8]\nbound B on M::orders [9]",
            "M::orders",
            vec!["A", "B"],
        ),
        (
            "invariant Same using v on M::Order at current { true }\nbound Same on M::orders [3]",
            "Same",
            vec!["Same", "Same"],
        ),
    ] {
        let (text, refusal) = assembly_refusal(&[], declarations);
        assert_eq!(refusal.errors.len(), 2, "{declarations}: {refusal:?}");
        for error in &refusal.errors {
            assert_eq!(
                error.cause.code(),
                Code::AmbiguousDeclaration,
                "{declarations}"
            );
            assert_eq!(
                error.cause.catalog_code().cause(),
                "ambiguous-name",
                "{declarations}"
            );
            let AssemblyCause::AmbiguousPopulationBound { name, spans } = &error.cause else {
                panic!("{declarations}: {:?}", error.cause);
            };
            assert_eq!(name, shared, "{declarations}");
            assert_eq!(spans.len(), 2, "{declarations}");
            assert!(spans.contains(&error.span), "{declarations}");
        }
        let mut located: Vec<&str> = refusal
            .errors
            .iter()
            .map(|error| &text[error.span.start..error.span.end])
            .collect();
        located.sort_unstable();
        assert_eq!(
            located, labels,
            "{declarations}: each declaration's own name"
        );
    }
}

/// (TC-1054 row 4): a bounded population is a
/// `Population(Some(3))` domain, so it is not among the unbounded domains
/// and a clause whose only domain it is has extent `Bounded`; the same unit
/// without the bound gives the unbounded record; the declarations in either
/// order give equal records.
#[trace("TC-1054", "FR-380-AC-3", "FR-104-AC-11")]
#[test]
fn a_bounded_population_is_not_an_unbounded_domain_and_the_clause_is_bounded() {
    let bound = "bound OrdersBound on M::orders [3]";
    let bounded = checked(&format!("{bound}\n{NO_LOOP}"));
    assert_eq!(
        claim_extents(&bounded),
        [&ClaimExtent::Bounded],
        "the only domain is the bounded `orders`"
    );
    let reversed = checked(&format!("{NO_LOOP}\n{bound}"));
    assert_eq!(
        bounded.requirements(),
        reversed.requirements(),
        "the order of the declarations changes no record"
    );

    let unbounded = checked(NO_LOOP);
    let extents = claim_extents(&unbounded);
    let [ClaimExtent::Unbounded(domains)] = extents[..] else {
        panic!("the unbounded record: {extents:?}");
    };
    assert_eq!(domains.len(), 1, "orders alone");
    let (_, kind) = domains.iter().next().expect("one domain");
    assert_eq!(kind, qsl_foundation::bound::DomainKind::Population);

    let [recorded] = bounded.population_bounds() else {
        panic!("one bound: {:?}", bounded.population_bounds());
    };
    assert_eq!(
        (
            recorded.label.as_str(),
            recorded.model.as_str(),
            recorded.maximum
        ),
        ("OrdersBound", "M", 3)
    );
    assert_eq!(recorded.population.as_str(), "orders");
    assert!(unbounded.population_bounds().is_empty());
}

/// A bound on the one population a clause's context type is a
/// member of, and a context-only clause, give a bounded clause too; a
/// `pre` clause's other domain stays unbounded.
#[trace("TC-1054", "FR-380-AC-3")]
#[test]
fn a_bound_on_the_context_population_alone_bounds_a_context_only_clause() {
    let graph = checked(&format!(
        "bound OrdersBound on M::orders [3]\n{}",
        invariant("Order", "true")
    ));
    assert_eq!(claim_extents(&graph), [&ClaimExtent::Bounded]);

    // `Person::watch` ranges over `people` as well as the walked `orders`: the
    // unbounded `people` keeps the clause unbounded.
    let graph = checked(&format!(
        "bound OrdersBound on M::orders [3]\n{}",
        clause("pre", "Person::watch", "reaches(o, o, next)")
    ));
    let extents = claim_extents(&graph);
    let [ClaimExtent::Unbounded(domains)] = extents[..] else {
        panic!("people stays unbounded: {extents:?}");
    };
    assert_eq!(domains.len(), 1, "people alone");
}

/// (TC-1054 row 12): a bound does not change which
/// population covers a type. A second population `staff` of `Order`, bounded
/// or not, still refuses at the clause's `on`, and at the form of a
/// reachability-only claim.
#[trace("TC-1054", "FR-380-AC-3", "FR-104-AC-11")]
#[test]
fn a_second_bounded_population_of_one_type_still_refuses_ambiguous_name() {
    for staff_bound in ["", "bound StaffBound on M::staff [3]\n"] {
        let declarations = format!("bound OrdersBound on M::orders [3]\n{staff_bound}{NO_LOOP}");
        let (text, assembled) = unit(&[("staff", "Order")], &declarations);
        let refusal = assembled.expect_err("the clause's context is ambiguous");
        let [error] = refusal.errors.as_slice() else {
            panic!("{declarations}: one error: {refusal:?}");
        };
        let AssemblyCause::AmbiguousPopulation {
            context,
            populations,
        } = &error.cause
        else {
            panic!("{declarations}: {:?}", error.cause);
        };
        assert_eq!(context, "M::Order");
        let names: Vec<&str> = populations
            .iter()
            .map(|key| key.node.rsplit('/').next().expect("a node identity"))
            .collect();
        assert_eq!(names, ["orders", "staff"]);
        assert_eq!(&text[error.span.start..error.span.end], "M::Order");

        // A claim whose context is `Person` and which only walks `Order`.
        let declarations = format!(
            "bound OrdersBound on M::orders [3]\n{staff_bound}{}",
            clause("pre", "Person::watch", "reaches(o, o, next)")
        );
        let refusal = assert_refusal(
            check_with(&[("staff", "Order")], &declarations),
            &declarations,
            Code::AmbiguousDeclaration,
            "ambiguous-name",
            vec![],
        );
        let CheckCause::StateModel(model) = &refusal.cause else {
            panic!("a StateModel refusal: {refusal:?}");
        };
        let CheckCause::AmbiguousName { name, .. } = &model.cause else {
            panic!("an ambiguous population name: {refusal:?}");
        };
        assert_eq!(
            name, "M::Order (ix://test/orders/orders, ix://test/orders/staff)",
            "orders and staff, in ascending DeclarationKey order"
        );
    }
}
