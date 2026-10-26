// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091 dimension equivalences (QSpec FR-142 "Dimension equivalence"):
//! the assembler's decisions in FR-142's steps, the
//! `quire.dimension-equivalence-node/v1` keys and the classes they derive.

use ix_trace_rs::trace;
use qsl_semantics::check::{
    AssemblyCause, AssemblyError, DimensionEquivalence, EquivalenceClasses, EquivalenceFault,
    PackageDeclarations,
};

use crate::timed_declarations::{assemble_selected_as, time_package};

type Package = (&'static str, &'static str, Vec<u8>);

/// `acme/rpc` (`R`, root `s`), `acme/flow` (`F`, root `s`, `ms` of
/// `1/1000`) and `acme/clock` (`G`, root `ms`).
fn packages() -> [Package; 3] {
    [
        (
            "R",
            "acme/rpc",
            time_package("acme/rpc", false, &[("s", None, "1")]),
        ),
        (
            "F",
            "acme/flow",
            time_package(
                "acme/flow",
                false,
                &[("s", None, "1"), ("ms", Some("s"), "1/1000")],
            ),
        ),
        (
            "G",
            "acme/clock",
            time_package("acme/clock", false, &[("ms", None, "1")]),
        ),
    ]
}

/// `body` under the source owner `(authority, identity)` over `packages`,
/// assembled.
fn assemble(
    owner: (&str, &str),
    body: &str,
) -> (String, Result<PackageDeclarations, Vec<AssemblyError>>) {
    let (text, assembled) = assemble_selected_as(owner, &packages(), body);
    (text, assembled.map_err(|refusal| refusal.errors))
}

fn admitted(body: &str) -> Vec<DimensionEquivalence> {
    let (_, assembled) = assemble(("agent-ix", "example"), body);
    assembled
        .unwrap_or_else(|errors| panic!("{body}: {errors:?}"))
        .dimension_equivalences
}

/// The one error of `body`: its catalog code and the text it is located at.
fn refused(body: &str) -> (AssemblyCause, String, String) {
    let (text, assembled) = assemble(("agent-ix", "example"), body);
    let errors = match assembled {
        Ok(_) => panic!("{body}: refused"),
        Err(errors) => errors,
    };
    let [error] = errors.as_slice() else {
        panic!("{body}: exactly one error: {errors:?}");
    };
    (
        error.cause.clone(),
        error.cause.catalog_code().to_string(),
        text[error.span.start..error.span.end].to_owned(),
    )
}

fn hex(equivalence: &DimensionEquivalence) -> String {
    equivalence.key.to_string()
}

const BASES: &str = "dimension A;\nunit ua : A = rational(1, 1);\n\
     dimension B;\nunit ub : B = rational(1, 1);\n\
     dimension C;\nunit uc : C = rational(1, 1);\n";

/// FR-091-AC-60 (TC-483), over the base dimensions of the source and of the
/// selected packages: pairs of two base dimensions are admitted, a
/// consistent cycle is admitted, and each defect refuses with one error at
/// the name or the declaration by FR-142's first matching step. Derived
/// dimensions are decided by step 6 over the base classes.
#[trace("TC-483", "FR-091-AC-60")]
#[test]
fn dimension_equivalences_are_decided_by_fr_142_steps() {
    for body in [
        "dimension R::Time ~ F::Time by 1 R::s = 1 F::s;",
        "dimension R::Time ~ Time by 1 R::s = 1 s;",
        &format!(
            "{BASES}dimension A ~ B by 1 ua = 1 ub;\ndimension B ~ C by 1 ub = 1000 uc;\n\
             dimension C ~ A by 1000 uc = 1 ua;"
        ),
    ] {
        assert!(!admitted(body).is_empty(), "{body}");
    }

    let missing = |name: &str| AssemblyCause::UnresolvedQuantityName {
        name: name.to_owned(),
    };
    for (body, name) in [
        ("dimension R::Time ~ Z::Time by 1 R::s = 1 Z::s;", "Z::Time"),
        (
            "dimension R::Time ~ R::Speed by 1 R::s = 1 R::mps;",
            "R::Speed",
        ),
        ("dimension R::Time ~ Nothing by 1 R::s = 1 s;", "Nothing"),
        (
            "dimension R::Time ~ F::Time by 1 R::s = 1 F::second;",
            "F::second",
        ),
    ] {
        assert_eq!(
            refused(body),
            (
                missing(name),
                "missing_declaration/missing-name".to_owned(),
                name.to_owned()
            ),
            "{body}"
        );
    }

    let cycle = |name: &str| AssemblyCause::QuantityCycle {
        edges: vec![(name.to_owned(), name.to_owned())],
    };
    for (body, name) in [
        ("dimension R::Time ~ R::Time by 1 R::s = 1 R::s;", "R::Time"),
        ("dimension Time ~ Time by 1 s = 1 s;", "Time"),
    ] {
        assert_eq!(
            refused(body),
            (
                cycle(name),
                "invalid_package/definition-cycle".to_owned(),
                body.to_owned()
            ),
            "{body}"
        );
    }

    let first = "dimension R::Time ~ F::Time by 1 R::s = 1 F::s;";
    for later in [
        "dimension F::Time ~ R::Time by 1 F::s = 1 R::s;",
        "dimension F::Time ~ R::Time by 1 F::s = 2 R::s;",
    ] {
        assert_eq!(
            refused(&format!("{first}\n{later}")),
            (
                AssemblyCause::DimensionEquivalence {
                    fault: EquivalenceFault::Conflicting,
                    retained: vec![
                        "R::Time ~ F::Time".to_owned(),
                        "F::Time ~ R::Time".to_owned()
                    ],
                },
                "invalid_package/conflicting-definition".to_owned(),
                later.to_owned()
            ),
            "{later}"
        );
    }

    // Step 4: each unit belongs to the dimension it is named for.
    let crossed = "dimension R::Time ~ F::Time by 1 F::s = 1 R::s;";
    assert_eq!(
        refused(crossed),
        (
            AssemblyCause::DimensionEquivalence {
                fault: EquivalenceFault::Incompatible,
                retained: vec!["F::s".to_owned(), "R::Time".to_owned()],
            },
            "invalid_package/incompatible-definition".to_owned(),
            crossed.to_owned()
        )
    );

    // Step 5: an affine unit, or a number that is not positive.
    let affine = "unit local : Time = rational(1, 1) * s + rational(5, 1);\n";
    let at = "dimension R::Time ~ Time by 1 R::s = 1 local;";
    assert_eq!(
        refused(&format!("{affine}{at}")),
        (
            AssemblyCause::DimensionEquivalence {
                fault: EquivalenceFault::InvalidValue,
                retained: vec!["local".to_owned(), "5/1".to_owned()],
            },
            "invalid_package/invalid-value".to_owned(),
            at.to_owned()
        )
    );
    for at in [
        "dimension R::Time ~ F::Time by rational(0, 1) R::s = 1 F::s;",
        "dimension R::Time ~ F::Time by rational(-1, 1) R::s = 1 F::s;",
    ] {
        assert_eq!(
            refused(at),
            (
                AssemblyCause::DimensionEquivalence {
                    fault: EquivalenceFault::InvalidValue,
                    retained: vec!["R::Time ~ F::Time".to_owned()],
                },
                "invalid_package/invalid-value".to_owned(),
                at.to_owned()
            ),
            "{at}"
        );
    }

    // Step 7: a cycle whose ratios do not multiply to one refuses at the
    // declaration closing it, retaining the cycle in source order.
    let closing = "dimension C ~ A by 1 uc = 1 ua;";
    assert_eq!(
        refused(&format!(
            "{BASES}dimension A ~ B by 1 ua = 1 ub;\ndimension B ~ C by 1 ub = 1000 uc;\n{closing}"
        )),
        (
            AssemblyCause::DimensionEquivalence {
                fault: EquivalenceFault::Conflicting,
                retained: vec!["A ~ B".to_owned(), "B ~ C".to_owned(), "C ~ A".to_owned()],
            },
            "invalid_package/conflicting-definition".to_owned(),
            closing.to_owned()
        )
    );

    // Step 6: two derived dimensions are equivalent exactly when their maps
    // over the base classes agree and the written ratio is the one the base
    // pairs fix.
    let derived = format!(
        "{BASES}dimension SA = A;\nunit usa : SA = rational(1, 1);\n\
         dimension SB = B;\nunit usb : SB = rational(1, 1);\n"
    );
    let pair = "dimension SA ~ SB by 1 usa = 1 usb;";
    let incompatible = AssemblyCause::DimensionEquivalence {
        fault: EquivalenceFault::Incompatible,
        retained: vec!["SA".to_owned(), "SB".to_owned()],
    };
    assert_eq!(
        refused(&format!("{derived}{pair}")),
        (
            incompatible.clone(),
            "invalid_package/incompatible-definition".to_owned(),
            pair.to_owned()
        )
    );
    assert_eq!(
        admitted(&format!("{derived}{pair}\ndimension A ~ B by 1 ua = 1 ub;")).len(),
        2
    );
    let doubled = "dimension SA ~ SB by 1 usa = 2 usb;";
    assert_eq!(
        refused(&format!(
            "{derived}dimension A ~ B by 1 ua = 1 ub;\n{doubled}"
        )),
        (
            incompatible,
            "invalid_package/incompatible-definition".to_owned(),
            doubled.to_owned()
        )
    );

    // Decisions do not depend on the source order within a phase.
    let reordered = format!(
        "{BASES}dimension C ~ A by 1000 uc = 1 ua;\ndimension A ~ B by 1 ua = 1 ub;\n\
         dimension B ~ C by 1 ub = 1000 uc;"
    );
    assert_eq!(admitted(&reordered).len(), 3);
}

/// FR-091-AC-61 (TC-483): the assembler mints Q12 and Q13 under owner (`a`,
/// `u`), one key for either written order and either unit spelling of one
/// ratio, and QSpec's three package-time vectors under owner (`agent-ix`,
/// `example`); `nominal_spans` holds each equivalence's span by its key.
#[trace("TC-483", "FR-091-AC-61")]
#[test]
fn dimension_equivalence_keys_reproduce_the_vectors() {
    const LENGTH: &str = "dimension Length;\nunit m : Length = rational(1, 1);\n\
         unit km : Length = rational(2000, 2) * m;\nunit cm : Length = decimal(1, 2) * m;\n\
         dimension Distance;\nunit dist : Distance = rational(1, 1);\n";
    const Q12: &str = "9be8d947f22ee11db4f61907e02f77c54f5e0ae1c832d97e92deea656973ff4c";
    const Q13: &str = "ad8ecf7942364032822ddc4a7b80afea30fc77911ac3680b8d4831108138a4e4";
    let keyed = |owner, declaration: &str| {
        let body = format!("{LENGTH}{declaration}");
        let (text, assembled) = assemble(owner, &body);
        let package = assembled.unwrap_or_else(|errors| panic!("{declaration}: {errors:?}"));
        let [equivalence] = package.dimension_equivalences.as_slice() else {
            panic!("one equivalence");
        };
        let span = package.nominal_spans[&equivalence.key];
        assert_eq!(&text[span.start..span.end], declaration);
        hex(equivalence)
    };
    for declaration in [
        "dimension Distance ~ Length by 1 dist = 1 km;",
        "dimension Length ~ Distance by 1 km = 1 dist;",
        "dimension Distance ~ Length by 1 dist = 1000 m;",
    ] {
        assert_eq!(keyed(("a", "u"), declaration), Q12, "{declaration}");
    }
    assert_eq!(
        keyed(("a", "u"), "dimension Length ~ Distance by 1 cm = 2 dist;"),
        Q13
    );

    for (declaration, vector) in [
        (
            "dimension R::Time ~ F::Time by 1 R::s = 1000 F::ms;",
            "bbf770ea312108434ee12d0ca4448c6d3d53a0004484b93b8ec577ff1ede00d4",
        ),
        (
            "dimension R::Time ~ G::Time by 1 R::s = 1000 G::ms;",
            "d707eb948ab055a64cf90b8009997ebc54c9bfe5361479afb6485c9c2df56c24",
        ),
        (
            "dimension F::Time ~ G::Time by 1 F::s = 1000 G::ms;",
            "584c6bbcb5056cf9dd5d10504339e49c5507348601530e5ef6d67bc5e351714b",
        ),
    ] {
        let [equivalence] = admitted(declaration).try_into().expect("one equivalence");
        assert_eq!(hex(&equivalence), vector, "{declaration}");
    }
}

/// FR-091-AC-62 (TC-483), its class half: over `dimension R::Time ~
/// F::Time by 1 R::s = 1 F::s;` both `Time` nodes are one class whose root
/// is the least of the two keys, the source `Time` is in no class, and a
/// unit with no equivalence derives none.
#[trace("TC-483", "FR-091-AC-62")]
#[test]
fn an_equivalence_derives_one_class_with_the_least_key_as_root() {
    let equivalences = admitted("dimension R::Time ~ F::Time by 1 R::s = 1 F::s;");
    let [equivalence] = equivalences.as_slice() else {
        panic!("one equivalence");
    };
    let classes = EquivalenceClasses::new(&equivalences);
    let [least, greater] = equivalence.dimensions;
    assert!(least.as_bytes() < greater.as_bytes());
    let one = quire_exact::Rational::from_integer(quire_exact::Integer::one());
    assert_eq!(classes.class(least), (least, one.clone()));
    assert_eq!(classes.class(greater), (least, one));
    assert!(admitted("dimension Other;").is_empty());
}
