// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091 dimension equivalences (QSpec FR-142 "Dimension equivalence"):
//! the assembler's decisions in FR-142's steps, the
//! `quire.dimension-equivalence-node/v1` keys and the classes they derive.

use ix_trace_rs::trace;
use qsl_semantics::check::{
    AssemblyCause, AssemblyError, DimensionEquivalence, EquivalenceClasses, EquivalenceFault,
    EquivalenceRetained, PackageDeclarations, TopologyFault, TopologyRetained,
};
use serde_json::{json, Value};

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

/// Q12's and Q13's source: `Length` (`m`, `km`, `cm`) and `Distance`
/// (`dist`).
const LENGTH: &str = "dimension Length;\nunit m : Length = rational(1, 1);\n\
     unit km : Length = rational(2000, 2) * m;\nunit cm : Length = decimal(1, 2) * m;\n\
     dimension Distance;\nunit dist : Distance = rational(1, 1);\n";

const BASES: &str = "dimension A;\nunit ua : A = rational(1, 1);\n\
     dimension B;\nunit ub : B = rational(1, 1);\n\
     dimension C;\nunit uc : C = rational(1, 1);\n";

/// `document` with the further package dimensions `(name, terms)` (each
/// term a dimension name and its exponent) and units `(name, dimension,
/// target, scale)`.
fn with_quantities(
    document: Vec<u8>,
    dimensions: &[(&str, &[(&str, &str)])],
    units: &[(&str, &str, Option<&str>, &str)],
) -> Vec<u8> {
    let mut package: Value = serde_json::from_slice(&document).expect("JSON");
    let identity = package["package"]["identity"]
        .as_str()
        .expect("a package identity")
        .to_owned();
    let origin = package["dimensions"][0]["origin"].clone();
    let dimension = |name: &str| format!("ix://{identity}/dimension/{name}");
    for (name, terms) in dimensions {
        let mut terms: Vec<Value> = terms
            .iter()
            .map(|(term, exponent)| json!({"dimension": dimension(term), "exponent": exponent}))
            .collect();
        terms.sort_by(|left, right| left["dimension"].as_str().cmp(&right["dimension"].as_str()));
        package["dimensions"]
            .as_array_mut()
            .expect("dimensions")
            .push(json!({
                "identity": dimension(name), "name": name, "base": terms.is_empty(),
                "terms": terms, "origin": origin,
            }));
    }
    for (name, of, target, scale) in units {
        let mut unit = json!({
            "identity": format!("ix://{identity}/unit/{name}"), "name": name,
            "dimension": dimension(of), "scale": scale, "offset": "0", "origin": origin,
        });
        if let Some(target) = target {
            unit["target"] = json!(format!("ix://{identity}/unit/{target}"));
        }
        package["units"].as_array_mut().expect("units").push(unit);
    }
    serde_json::to_vec(&package).expect("JSON")
}

/// [`packages`] with `R` also declaring `Length` (`m`), `Area = Length ^ 2`
/// (`m2`), `Speed = Length / Time` (`mps`) and `Mass` (`kg`), and `F`
/// declaring `Length` (`m`), `Speed = Length / Time` (`mps`) and `Mass`
/// with no unit.
fn quantity_packages() -> [Package; 3] {
    let [(r, rpc, r_document), (f, flow, f_document), clock] = packages();
    [
        (
            r,
            rpc,
            with_quantities(
                r_document,
                &[
                    ("Length", &[]),
                    ("Area", &[("Length", "2")]),
                    ("Speed", &[("Length", "1"), ("Time", "-1")]),
                    ("Mass", &[]),
                ],
                &[
                    ("m", "Length", None, "1"),
                    ("m2", "Area", None, "1"),
                    ("mps", "Speed", None, "1"),
                    ("kg", "Mass", None, "1"),
                ],
            ),
        ),
        (
            f,
            flow,
            with_quantities(
                f_document,
                &[
                    ("Length", &[]),
                    ("Speed", &[("Length", "1"), ("Time", "-1")]),
                    ("Mass", &[]),
                ],
                &[("m", "Length", None, "1"), ("mps", "Speed", None, "1")],
            ),
        ),
        clock,
    ]
}

/// The one error of `body` over [`quantity_packages`].
fn refused_over_quantities(body: &str) -> (AssemblyCause, String, String) {
    let (text, assembled) =
        assemble_selected_as(("agent-ix", "example"), &quantity_packages(), body);
    let errors = match assembled {
        Ok(_) => panic!("{body}: refused"),
        Err(refusal) => refusal.errors,
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

/// What a refusal retains, each as written.
fn retained(cause: &AssemblyCause) -> Vec<String> {
    match cause {
        AssemblyCause::DimensionEquivalence { retained, .. } => {
            retained.iter().map(ToString::to_string).collect()
        }
        other => panic!("an equivalence refusal, not {other:?}"),
    }
}

/// FR-091-AC-60 (TC-483), QSpec FR-142-AC-22: pairs of two base dimensions
/// are admitted, a consistent cycle is admitted, a source derived dimension
/// (`Dur = R::Time`) and the package `Speed`s are admitted when their maps
/// over the base classes agree, and each defect refuses with one error at
/// the name or the declaration by FR-142's first matching step, base pairs
/// before the first dimension turn and every other pair after the last.
#[trace("TC-483", "FR-091-AC-60", "QSpec-TC-187", "QSpec-FR-142-AC-22")]
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
        let (cause, code, at) = refused(&format!("{first}\n{later}"));
        assert_eq!(
            (code.as_str(), at.as_str(), retained(&cause)),
            (
                "invalid_package/conflicting-definition",
                later,
                vec![
                    "R::Time ~ F::Time".to_owned(),
                    "F::Time ~ R::Time".to_owned()
                ]
            ),
            "{later}"
        );
    }

    // Step 4: each unit belongs to the dimension it is named for, a package
    // unit of a derived package dimension included.
    let crossed = "dimension R::Time ~ F::Time by 1 F::s = 1 R::s;";
    let (cause, code, at) = refused(crossed);
    assert_eq!(
        (code.as_str(), at.as_str(), retained(&cause)),
        (
            "invalid_package/incompatible-definition",
            crossed,
            vec!["F::s".to_owned(), "R::Time".to_owned()]
        )
    );
    let area = "dimension R::Time ~ R::Area by 1 R::s = 1 R::m2;";
    let (cause, code, at) = refused_over_quantities(area);
    assert_eq!(
        (code.as_str(), at.as_str(), retained(&cause)),
        (
            "invalid_package/incompatible-definition",
            area,
            vec!["R::Time".to_owned(), "R::Area".to_owned()]
        )
    );
    // A source dimension over a package dimension is in the class of that
    // dimension once an equivalence relates it.
    let dur = "dimension Dur = R::Time;\nunit dsec : Dur = rational(1, 1);\n";
    let pair = "dimension Dur ~ F::Time by 1 dsec = 1 F::s;";
    assert_eq!(
        admitted(&format!(
            "{dur}dimension R::Time ~ F::Time by 1 R::s = 1 F::s;\n{pair}"
        ))
        .len(),
        2
    );
    let (cause, code, at) = refused(&format!("{dur}{pair}"));
    assert_eq!(
        (code.as_str(), at.as_str(), retained(&cause)),
        (
            "invalid_package/incompatible-definition",
            pair,
            vec!["Dur".to_owned(), "F::Time".to_owned()]
        )
    );

    // Step 5: an affine unit, or a number that is not positive.
    let affine = "unit local : Time = rational(1, 1) * s + rational(5, 1);\n";
    let at = "dimension R::Time ~ Time by 1 R::s = 1 local;";
    let (cause, code, text) = refused(&format!("{affine}{at}"));
    assert_eq!(
        (code.as_str(), text.as_str()),
        ("invalid_package/invalid-value", at)
    );
    assert_eq!(
        cause,
        AssemblyCause::DimensionEquivalence {
            fault: EquivalenceFault::InvalidValue,
            retained: vec![
                EquivalenceRetained::Unit {
                    name: "local".to_owned()
                },
                EquivalenceRetained::Value(exact(5, 1)),
            ],
        }
    );
    for (at, value) in [
        (
            "dimension R::Time ~ F::Time by rational(0, 1) R::s = 1 F::s;",
            exact(0, 1),
        ),
        (
            "dimension R::Time ~ F::Time by rational(-1, 1) R::s = 1 F::s;",
            exact(-1, 1),
        ),
    ] {
        let (cause, code, text) = refused(at);
        assert_eq!(
            (code.as_str(), text.as_str()),
            ("invalid_package/invalid-value", at)
        );
        let AssemblyCause::DimensionEquivalence { retained, .. } = cause else {
            panic!("an equivalence refusal");
        };
        assert_eq!(
            retained.last(),
            Some(&EquivalenceRetained::Value(value)),
            "{at}"
        );
    }

    // Step 7: a cycle whose ratios do not multiply to one refuses at the
    // declaration closing it, retaining the cycle in source order.
    let closing = "dimension C ~ A by 1 uc = 1 ua;";
    let (cause, code, at) = refused(&format!(
        "{BASES}dimension A ~ B by 1 ua = 1 ub;\ndimension B ~ C by 1 ub = 1000 uc;\n{closing}"
    ));
    assert_eq!(
        (code.as_str(), at.as_str(), retained(&cause)),
        (
            "invalid_package/conflicting-definition",
            closing,
            vec!["A ~ B".to_owned(), "B ~ C".to_owned(), "C ~ A".to_owned()]
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
    for body in [
        format!("{derived}{pair}"),
        format!("{derived}dimension A ~ B by 1 ua = 1 ub;\ndimension SA ~ SB by 1 usa = 2 usb;"),
    ] {
        let (cause, code, _) = refused(&body);
        assert_eq!(
            (code.as_str(), retained(&cause)),
            (
                "invalid_package/incompatible-definition",
                vec!["SA".to_owned(), "SB".to_owned()]
            ),
            "{body}"
        );
    }
    assert_eq!(
        admitted(&format!("{derived}{pair}\ndimension A ~ B by 1 ua = 1 ub;")).len(),
        2
    );

    // The package speeds: equivalent once their lengths and times are.
    let speeds = "dimension R::Speed ~ F::Speed by 1 R::mps = 1 F::mps;";
    let bases = "dimension R::Length ~ F::Length by 1 R::m = 1 F::m;\n\
                 dimension R::Time ~ F::Time by 1 R::s = 1 F::s;\n";
    let (_, assembled) = assemble_selected_as(
        ("agent-ix", "example"),
        &quantity_packages(),
        &format!("{bases}{speeds}"),
    );
    assert_eq!(
        assembled
            .unwrap_or_else(|refusal| panic!("{:?}", refusal.errors))
            .dimension_equivalences
            .len(),
        3
    );
    let (cause, code, at) = refused_over_quantities(speeds);
    assert_eq!(
        (code.as_str(), at.as_str(), retained(&cause)),
        (
            "invalid_package/incompatible-definition",
            speeds,
            vec!["R::Speed".to_owned(), "F::Speed".to_owned()]
        )
    );

    // Base pairs are decided before the first turn: the base cycle refuses,
    // not the zero scale of `z`. Every other pair is decided after the last
    // turn: the zero scale refuses, not the derived pair.
    let source = "dimension L;\nunit r : L = rational(1, 1);\nunit z : L = rational(0, 1);\n";
    let (cause, code, _) = refused(&format!("{source}dimension L ~ L by 1 r = 1 r;"));
    assert_eq!(
        (cause, code.as_str()),
        (cycle("L"), "invalid_package/definition-cycle")
    );
    let (_, assembled) = assemble(
        ("agent-ix", "example"),
        &format!(
            "{source}dimension Dur = L;\nunit rd : Dur = rational(1, 1);\n\
             dimension L2;\nunit r2 : L2 = rational(1, 1);\n\
             dimension Dur ~ L2 by 1 rd = 1 r2;"
        ),
    );
    let errors = assembled.expect_err("the turns refuse");
    assert!(
        errors.iter().any(|error| matches!(
            &error.cause,
            AssemblyCause::UnitGraphTopology {
                fault: TopologyFault::ZeroScale,
                retained,
            } if retained == &[
                TopologyRetained::Unit { name: "z".to_owned(), key: None },
                TopologyRetained::Scale(quire_exact::Rational::from_integer(quire_exact::Integer::zero())),
            ]
        )),
        "{errors:?}"
    );
    assert!(
        !errors
            .iter()
            .any(|error| matches!(error.cause, AssemblyCause::DimensionEquivalence { .. })),
        "{errors:?}"
    );

    // Decisions do not depend on the source order within a phase.
    let reordered = format!(
        "{BASES}dimension C ~ A by 1000 uc = 1 ua;\ndimension A ~ B by 1 ua = 1 ub;\n\
         dimension B ~ C by 1 ub = 1000 uc;"
    );
    assert_eq!(admitted(&reordered).len(), 3);
}

fn exact(numerator: i64, denominator: i64) -> quire_exact::Rational {
    quire_exact::Rational::new(
        quire_exact::Integer::from(numerator),
        quire_exact::Integer::from(denominator),
    )
    .expect("a rational")
}

/// FR-091-AC-62 (TC-483): `F::Mass` has no unit, so naming `F::kg`
/// refuses `missing-name` at it, and the base-dimension rule refuses a
/// source unit whose dimension carries `F::Mass`, at `F`'s selection,
/// naming the package dimension's identity and the unit, with or without
/// an equivalence; over `dimension R::Time ~ F::Time by 1 R::s = 1 F::s;`
/// both `Time` nodes are one class whose root is the least of the two
/// keys, Q12 relates `Distance` and `Length` in one class, the source
/// `Time` is in no class, and a unit with no equivalence derives none.
#[trace("TC-483", "FR-091-AC-62", "QSpec-TC-187", "QSpec-FR-142-AC-24")]
#[test]
fn an_equivalence_derives_one_class_with_the_least_key_as_root() {
    let masses = "dimension R::Mass ~ F::Mass by 1 R::kg = 1 F::kg;";
    assert_eq!(
        refused_over_quantities(masses),
        (
            AssemblyCause::UnresolvedQuantityName {
                name: "F::kg".to_owned()
            },
            "missing_declaration/missing-name".to_owned(),
            "F::kg".to_owned()
        )
    );
    let force = "dimension Force = F::Mass;\nunit n : Force = rational(1, 1);\n";
    for body in [
        force.to_owned(),
        format!("{force}dimension R::Time ~ F::Time by 1 R::s = 1 F::s;"),
    ] {
        let (cause, code, at) = refused_over_quantities(&body);
        assert_eq!(
            (cause, code.as_str()),
            (
                AssemblyCause::UnanchoredBaseDimension {
                    dimension: "ix://acme/flow/dimension/Mass".to_owned(),
                    unit: "n".to_owned(),
                },
                "missing_declaration/missing-name",
            ),
            "{body}"
        );
        assert!(at.starts_with("model F = \"acme/flow\""), "{at}");
    }

    let equivalences = admitted("dimension R::Time ~ F::Time by 1 R::s = 1 F::s;");
    let [equivalence] = equivalences.as_slice() else {
        panic!("one equivalence");
    };
    let classes = EquivalenceClasses::new(&equivalences);
    let [least, greater] = equivalence.dimensions.clone();
    assert!(least < greater);
    let one = quire_exact::Rational::from_integer(quire_exact::Integer::one());
    assert_eq!(classes.class(least.clone()), (least.clone(), one.clone()));
    assert_eq!(classes.class(greater), (least, one.clone()));
    assert!(admitted("dimension Other;").is_empty());

    let (_, assembled) = assemble(
        ("a", "u"),
        &format!("{LENGTH}dimension Distance ~ Length by 1 dist = 1 km;"),
    );
    let package = assembled.unwrap_or_else(|errors| panic!("{errors:?}"));
    let [q12] = package.dimension_equivalences.as_slice() else {
        panic!("one equivalence");
    };
    let classes = EquivalenceClasses::new(&package.dimension_equivalences);
    let [least, greater] = q12.dimensions.clone();
    assert_eq!(classes.class(least.clone()), (least.clone(), one));
    assert_eq!(classes.class(greater).0, least);
}

/// QSpec FR-142-AC-16, AC-18 and FR-091-AC-55: a package dimension takes its
/// turn when a declaration first reaches it, before that declaration, so the
/// base-dimension rule refuses the first node of the turn list that no unit
/// anchors: `Q` declared before the declaration that names `F::Mass`, or
/// `F::Mass` when `Force` is declared first. Each refusal is
/// `missing_declaration`/`missing-name` naming the node and the unit.
#[trace("TC-483", "FR-091-AC-55", "QSpec-TC-187", "QSpec-FR-142-AC-16")]
#[test]
fn a_package_dimension_takes_its_turn_when_a_declaration_reaches_it() {
    let unit = "unit n : Force = rational(1, 1);\n";
    let (cause, code, at) = refused_over_quantities(&format!(
        "dimension Q;\ndimension Force = F::Mass * Q;\n{unit}"
    ));
    assert_eq!(
        (cause, code.as_str(), at.as_str()),
        (
            AssemblyCause::UnanchoredBaseDimension {
                dimension: "Q".to_owned(),
                unit: "n".to_owned(),
            },
            "missing_declaration/missing-name",
            "Q"
        )
    );
    let (cause, code, at) = refused_over_quantities(&format!(
        "dimension Force = F::Mass * Q;\ndimension Q;\n{unit}"
    ));
    assert_eq!(
        (cause, code.as_str()),
        (
            AssemblyCause::UnanchoredBaseDimension {
                dimension: "ix://acme/flow/dimension/Mass".to_owned(),
                unit: "n".to_owned(),
            },
            "missing_declaration/missing-name",
        )
    );
    assert!(at.starts_with("model F = \"acme/flow\""), "{at}");
}

/// QSpec FR-322 "Nominal nodes": the dimension node an admitted domain
/// package declares and a unit's graph names carries that package's
/// `ModelOwner` (`{identity, node}`) and no `declaration`, while the unit's
/// own dimension keeps its `SourceOwner` and `declaration`.
#[trace("QSpec-FR-322-AC-51", "QSpec-FR-322-AC-81")]
#[test]
fn a_package_dimension_node_is_model_owned_with_no_declaration() {
    use qsl_semantics::check::Owner;
    use quire_semantic_value::checking::CheckingLimits;

    let (_, assembled) = assemble_selected_as(
        ("agent-ix", "example"),
        &packages(),
        "dimension Dur = R::Time;\nunit dsec : Dur = rational(1, 1);\n\
         function f using v(d: Quantity<dsec>): Boolean pure { true }",
    );
    let checked = assembled
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the unit checks");
    let graph = checked.semantic_graph();
    let dimensions: Vec<_> = graph
        .nodes()
        .filter(|node| node.semantic_form() == "dimension")
        .collect();
    let foreign: Vec<_> = dimensions
        .iter()
        .filter(|node| node.declaration().is_none())
        .collect();
    let [time] = foreign.as_slice() else {
        panic!("one package dimension node: {dimensions:?}");
    };
    assert_eq!(time.declaration(), None);
    let Some(Owner::Model(owner)) = time.owner() else {
        panic!("a ModelOwner: {:?}", time.owner());
    };
    assert_eq!(owner.identity(), "acme/rpc");
    assert_eq!(owner.node(), "ix://acme/rpc/dimension/Time");
    // QSpec-FR-322-AC-81: the path of a model-owned dimension.
    assert_eq!(
        time.key().path(),
        vec![
            quire_exact::PathSegment::Text("scalar_type".to_owned()),
            quire_exact::PathSegment::Text("dimension".to_owned()),
            quire_exact::PathSegment::Text("ix://acme/rpc/dimension/Time".to_owned()),
        ]
    );
    // Every other dimension node is the unit's own: declared, source-owned.
    assert!(dimensions.len() > 1, "the unit's own dimensions");
    for own in dimensions
        .iter()
        .filter(|node| node.declaration().is_some())
    {
        assert!(matches!(own.owner(), Some(Owner::Source(_))));
    }
}

/// FR-091-AC-50 (QSpec FR-142-AC-20), the refusal half: with `R`
/// (`acme/rpc`) and `F` (`acme/flow`) selected and the source's own `Time`, a
/// source writing `Z::Time` with no model alias `Z`, or `R::Speed` where
/// `acme/rpc` declares no `Speed`, refuses `missing_declaration`/`missing-name`
/// at the name, and a second `dimension Time;` refuses
/// `ambiguous_declaration`/`ambiguous-name`. The admission half (a source unit
/// typed by `R::Time`, `Quantity<R::s>`) is not implemented: see QSL-1478.
#[trace("TC-483", "QSpec-TC-187")]
#[test]
fn unknown_package_dimensions_and_repeated_source_dimensions_refuse() {
    let missing = |body: &str, at: &str| {
        let (_, code, located) = refused(body);
        assert_eq!(code, "missing_declaration/missing-name", "{body}");
        assert_eq!(located, at, "{body}");
    };
    missing("dimension D = Z::Time;", "Z::Time");
    missing("dimension D = R::Speed;", "R::Speed");
    let (_, code, _) = refused("dimension Time;\n");
    assert_eq!(code, "ambiguous_declaration/ambiguous-name");
}

/// FR-480 "Selections and imports" (QSL-1638): a dimension equivalence's
/// model aliases name the packages its own unit binds them to. Unit `a`,
/// admitted first, binds `R` to `acme/clock`, which declares no unit `s`;
/// unit `b` binds `R` to `acme/rpc`, so its `R::s` resolves and the
/// equivalence relates `acme/rpc`'s `Time` to `acme/flow`'s.
#[trace("FR-480")]
#[test]
fn an_equivalences_aliases_name_its_own_units_packages() {
    let [(_, _, rpc), (_, _, flow), (_, _, clock)] = packages();
    let a = "profile v = \"quire.model.complete/v1\";\nmodel R = \"acme/clock\";\n\
             type Pad = Integer;";
    let b = "profile v = \"quire.model.complete/v1\";\nmodel R = \"acme/rpc\";\n\
             model F = \"acme/flow\";\n\
             dimension R::Time ~ F::Time by 1 R::s = 1 F::s;";
    let declarations = crate::model_operations::assemble_units(
        &[("a", a), ("b", b)],
        &[rpc.as_slice(), flow.as_slice(), clock.as_slice()],
    );
    let [equivalence] = declarations.dimension_equivalences.as_slice() else {
        panic!("one equivalence: {:?}", declarations.dimension_equivalences);
    };
    let mut packages: Vec<&str> = equivalence
        .dimensions
        .iter()
        .map(quire_exact::NodeKey::package)
        .collect();
    packages.sort_unstable();
    assert_eq!(packages, ["acme/flow", "acme/rpc"]);
}

/// FR-480 "Selections and imports" (QSL-1638): two units write the same
/// equivalence spelling over different bindings of `R`. Unit `a` binds `R`
/// to `acme/clock`, which declares no unit `s`, so its `R::s` is unresolved
/// and refuses; it is never read through unit `b`'s binding of `R`.
#[trace("FR-480")]
#[test]
fn two_units_writing_one_spelling_resolve_each_through_its_own_bindings() {
    let [(_, _, rpc), (_, _, flow), (_, _, clock)] = packages();
    let equivalence = "dimension R::Time ~ F::Time by 1 R::s = 1 F::s;";
    let a = format!(
        "profile v = \"quire.model.complete/v1\";\nmodel R = \"acme/clock\";\n\
         model F = \"acme/flow\";\n{equivalence}"
    );
    let b = format!(
        "profile v = \"quire.model.complete/v1\";\nmodel R = \"acme/rpc\";\n\
         model F = \"acme/flow\";\n{equivalence}"
    );
    let refusal = crate::model_operations::try_assemble_units(
        &[("a", &a), ("b", &b)],
        &[rpc.as_slice(), flow.as_slice(), clock.as_slice()],
    )
    .expect_err("unit a's `R::s` is unresolved");
    assert!(
        refusal
            .errors
            .iter()
            .any(|error| matches!(error.cause, AssemblyCause::UnresolvedQuantityName { .. })),
        "{refusal:?}"
    );
}

/// FR-091-AC-50/55: package-qualified dimensions type source units, package
/// units resolve as targets, and imported-node turns retain local defects.
#[trace("TC-483", "FR-091-AC-50", "FR-091-AC-55")]
#[test]
fn package_units_and_dimensions_resolve_in_source_unit_graphs() {
    use qsl_semantics::check::AssemblyCause as Cause;

    let packages = [(
        "M",
        "acme/rpc",
        with_quantities(
            time_package("acme/rpc", false, &[("s", None, "1")]),
            &[("Rate", &[]), ("Empty", &[])],
            &[("us", "Rate", None, "1")],
        ),
    )];
    let body = "unit tick : M::Rate = rational(1, 2) * M::us;\n\
                unit tock : M::Rate = rational(3, 1) * tick;";
    let (text, assembled) = assemble_selected_as(("agent-ix", "example"), &packages, body);
    let assembled = assembled.expect("package dimensions and units resolve for local unit forms");
    let unit_key = |name: &str| {
        assembled
            .nominal_spans
            .iter()
            .find_map(|(key, span)| (text[span.start..span.end] == *name).then(|| key.clone()))
            .unwrap_or_else(|| panic!("unit {name} has a retained declaration span"))
    };
    let tick = assembled.units.unit(unit_key("tick")).expect("tick");
    assert_eq!(tick.canonical().scale().to_string(), "1/2");
    let tock = assembled.units.unit(unit_key("tock")).expect("tock");
    assert_eq!(tock.canonical().scale().to_string(), "3/2");

    let (_, assembled) = assemble_selected_as(
        ("agent-ix", "example"),
        &packages,
        "unit tick : M::Time = rational(1, 2) * M::s;",
    );
    assembled.expect("a package unit can target another package unit");

    let refusal = |body: &str| {
        let (_, assembled) = assemble_selected_as(("agent-ix", "example"), &packages, body);
        assembled
            .expect_err("this package-unit form is refused")
            .errors
    };
    let errors = refusal("unit bad : M::Rate = rational(1, 2) * s;");
    let [error] = errors.as_slice() else {
        panic!("row 3 refuses one error: {errors:?}")
    };
    assert!(matches!(
        &error.cause,
        Cause::UnitGraphTopology {
            fault: TopologyFault::CrossDimensionTarget,
            retained,
        } if matches!(retained.as_slice(), [
            TopologyRetained::Unit { name: unit, .. },
            TopologyRetained::TargetName(target),
            TopologyRetained::Dimension { name: package_dimension, key: Some(_) },
            TopologyRetained::Dimension { name: source_dimension, key: None },
        ] if unit == "bad" && target == "s"
            && package_dimension == "ix://acme/rpc/dimension/Rate"
            && source_dimension == "Time")
    ));

    let errors = refusal("unit shifted : M::Empty = rational(2, 1);");
    let [error] = errors.as_slice() else {
        panic!("row 5 refuses one error: {errors:?}")
    };
    assert!(matches!(
        &error.cause,
        Cause::UnitGraphTopology {
            fault: TopologyFault::NonIdentityRoot,
            retained,
        } if matches!(retained.as_slice(), [
            TopologyRetained::Unit { name, key: None },
            TopologyRetained::Scale(scale),
            TopologyRetained::Offset(offset),
        ] if name == "shifted"
            && scale == &exact(2, 1)
            && offset.is_zero())
    ));

    let errors = refusal("unit r2 : M::Rate = rational(1, 1);");
    let [error] = errors.as_slice() else {
        panic!("row 4 refuses one error: {errors:?}")
    };
    assert!(matches!(
        &error.cause,
        Cause::UnitGraphTopology {
            fault: TopologyFault::TwoRoots,
            retained,
        } if matches!(retained.as_slice(), [
            TopologyRetained::Dimension { name: dimension, key: Some(_) },
            TopologyRetained::Unit { name: package_root, key: Some(_) },
            TopologyRetained::Unit { name: local_root, key: None },
        ] if dimension == "ix://acme/rpc/dimension/Rate"
            && package_root == "ix://acme/rpc/unit/us"
            && local_root == "r2")
    ));

    let errors = refusal("unit z : M::Rate = rational(0, 1) * M::us;");
    let [error] = errors.as_slice() else {
        panic!("row 1 refuses one error: {errors:?}")
    };
    assert!(matches!(
        &error.cause,
        Cause::UnitGraphTopology {
            fault: TopologyFault::ZeroScale,
            retained,
        } if matches!(retained.as_slice(), [
            TopologyRetained::Unit { name, .. },
            TopologyRetained::Scale(scale),
        ] if name == "z" && scale.is_zero())
    ));

    let errors = refusal("unit g : M::Rate = rational(1, 2) * M::ghost;");
    let [error] = errors.as_slice() else {
        panic!("row 2 refuses one error: {errors:?}")
    };
    assert!(matches!(
        &error.cause,
        Cause::UnitGraphTopology {
            fault: TopologyFault::UnknownTarget,
            retained,
        } if matches!(retained.as_slice(), [
            TopologyRetained::Unit { name, .. },
            TopologyRetained::TargetName(target),
        ] if name == "g" && target == "M::ghost")
    ));

    let errors = refusal("unit tick : Rate = rational(1, 2) * M::us;");
    let [error] = errors.as_slice() else {
        panic!("unqualified package dimension refuses one error: {errors:?}")
    };
    assert!(matches!(
        &error.cause,
        Cause::UnresolvedQuantityName { name } if name == "Rate"
    ));
}
