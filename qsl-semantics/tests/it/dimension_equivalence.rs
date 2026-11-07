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

    // A bare unit name of a `by` clause that the unit does not declare is an
    // unresolved unit name, even though two selected packages declare `ms`.
    assert_eq!(
        refused("dimension R::Time ~ F::Time by 1 R::s = 1 ms;"),
        (
            AssemblyCause::UnresolvedUnitName {
                name: "ms".to_owned(),
                packages: vec!["acme/flow".to_owned(), "acme/clock".to_owned()],
            },
            "missing_declaration/missing-name".to_owned(),
            "ms".to_owned()
        )
    );

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
/// typed by a package dimension, `Quantity<R::s>`, an equivalence naming a
/// source unit typed by a package dimension) is covered by
/// `package_units_and_dimensions_resolve_in_source_unit_graphs` and the
/// package-unit tests below.
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

/// `identity`'s package: base `Time` (root `s`), then `dimensions` and
/// `units` as [`with_quantities`] takes them.
fn domain(
    identity: &str,
    dimensions: &[(&str, &[(&str, &str)])],
    units: &[(&str, &str, Option<&str>, &str)],
) -> Vec<u8> {
    with_quantities(
        time_package(identity, false, &[("s", None, "1")]),
        dimensions,
        units,
    )
}

/// `body` over `packages`, refused with exactly one error: the cause and the
/// text it is located at.
fn refused_over(packages: &[Package], body: &str) -> (AssemblyCause, String) {
    let (text, assembled) = assemble_selected_as(("agent-ix", "example"), packages, body);
    let errors = match assembled {
        Ok(_) => panic!("{body}: refused"),
        Err(refusal) => refusal.errors,
    };
    let [error] = errors.as_slice() else {
        panic!("{body}: exactly one error: {errors:?}");
    };
    (
        error.cause.clone(),
        text[error.span.start..error.span.end].to_owned(),
    )
}

/// A unit node as the checked graph holds it: its declared name, its model
/// owner's node when it has one, and its dimension and target nodes' model
/// owners' nodes.
type UnitNode = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// The unit nodes `body` over `packages` checks to, with one function using
/// each of `used`.
fn unit_nodes(packages: &[Package], body: &str, used: &[&str]) -> Vec<UnitNode> {
    use qsl_semantics::check::Owner;
    use qsl_semantics::value::nominal_payload::NominalPayload;
    use quire_semantic_value::checking::CheckingLimits;
    let uses: String = used
        .iter()
        .map(|unit| {
            format!("\nfunction use_{unit} using v(d: Quantity<{unit}>): Boolean pure {{ true }}")
        })
        .collect();
    let (_, assembled) =
        assemble_selected_as(("agent-ix", "example"), packages, &format!("{body}{uses}"));
    let checked = assembled
        .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}"))
        .check(CheckingLimits::default())
        .expect("the unit checks");
    let graph = checked.semantic_graph();
    let nodes: Vec<_> = graph.nodes().collect();
    let model_node = |key: &quire_exact::NodeKey| {
        nodes.iter().find_map(|node| match node.owner() {
            Some(Owner::Model(owner)) if node.key() == *key => Some(owner.node().to_owned()),
            _ => None,
        })
    };
    nodes
        .iter()
        .filter(|node| node.semantic_form() == "unit")
        .map(|node| {
            let Some(NominalPayload::Unit(payload)) = node.nominal_payload() else {
                panic!("a unit payload: {node:?}");
            };
            let own = match node.owner() {
                Some(Owner::Model(owner)) => Some(owner.node().to_owned()),
                _ => None,
            };
            (
                node.declaration().map(|name| {
                    name.iter()
                        .map(|part| part.as_str())
                        .collect::<Vec<_>>()
                        .join(".")
                }),
                own,
                model_node(payload.dimension()),
                payload.target().and_then(model_node),
            )
        })
        .collect()
}

fn two_domains() -> [Package; 2] {
    [
        (
            "P",
            "acme/p",
            domain("acme/p", &[("L", &[])], &[("m", "L", None, "1")]),
        ),
        ("Q", "acme/q", domain("acme/q", &[("T", &[])], &[])),
    ]
}

/// FR-091-AC-55 and AC-72 (QSpec FR-142-AC-18, TC-187 U11r): a local unit
/// typed by a package dimension and targeting the package's root joins the
/// package node's graph, and the checked package holds it with the package's
/// unit and dimension as model-owned nodes, so a function using it checks.
#[trace(
    "TC-483",
    "FR-091-AC-55",
    "FR-091-AC-72",
    "QSpec-TC-187",
    "QSpec-FR-142-AC-18"
)]
#[test]
fn a_local_unit_targeting_a_package_unit_checks_with_model_owned_nodes() {
    let units = unit_nodes(
        &two_domains(),
        "unit cm : P::L = rational(1, 100) * P::m;",
        &["cm"],
    );
    let identity = |name: &str| Some(format!("ix://acme/p/{name}"));
    let cm = units
        .iter()
        .find(|(declared, ..)| declared.as_deref() == Some("cm"))
        .expect("the unit's own node");
    assert_eq!(
        (&cm.1, &cm.2, &cm.3),
        (&None, &identity("dimension/L"), &identity("unit/m"))
    );
    let package_root = units
        .iter()
        .find(|(_, own, ..)| own.as_deref() == Some("ix://acme/p/unit/m"))
        .expect("the package root's node");
    assert_eq!(
        (&package_root.0, &package_root.2),
        (&None, &identity("dimension/L"))
    );
}

/// FR-091-AC-72 (QSpec FR-142-AC-18, AC-19): `tick : M::Rate` over `M::us`
/// and `tock` over `tick` check with the package's `Rate` and `us` as
/// model-owned nodes, and `convert<Quantity<M::us>>(d)` in a function body
/// over `Quantity<tick>` checks.
#[trace("TC-483", "FR-091-AC-72", "QSpec-TC-187", "QSpec-FR-142-AC-18")]
#[test]
fn a_function_converting_a_package_typed_unit_to_a_package_unit_checks() {
    use quire_semantic_value::checking::CheckingLimits;
    let packages: [Package; 1] = [(
        "M",
        "acme/rpc",
        domain("acme/rpc", &[("Rate", &[])], &[("us", "Rate", None, "1")]),
    )];
    let body = "unit tick : M::Rate = rational(1, 2) * M::us;\n\
                unit tock : M::Rate = rational(3, 1) * tick;\n\
                function k1 using v(d: Quantity<tick>): Boolean pure \
                { convert<Quantity<M::us>>(d) = 1 M::us }";
    let (_, assembled) = assemble_selected_as(("agent-ix", "example"), &packages, body);
    assembled
        .expect("assembles")
        .check(CheckingLimits::default())
        .expect("k1 checks");
    let units = unit_nodes(
        &packages,
        &body[..body.find("function").unwrap_or(0)],
        &["tock"],
    );
    let tock = units
        .iter()
        .find(|(declared, ..)| declared.as_deref() == Some("tock"))
        .expect("tock's node");
    assert_eq!(
        (&tock.2, &tock.3),
        (&Some("ix://acme/rpc/dimension/Rate".to_owned()), &None)
    );
}

/// FR-091-AC-55 (QSpec FR-142-AC-18, AC-19): a source unit typed by a
/// package dimension and targeting a package unit composes its scale through
/// the package unit's chain, so an equivalence naming it fixes the root ratio
/// from `tick = 1/2 ms = 500 us`.
#[trace("TC-483", "FR-091-AC-55", "QSpec-TC-187", "QSpec-FR-142-AC-18")]
#[test]
fn a_source_unit_over_a_package_unit_composes_through_the_package_chain() {
    let packages: [Package; 1] = [(
        "M",
        "acme/rpc",
        domain(
            "acme/rpc",
            &[("Rate", &[])],
            &[
                ("us", "Rate", None, "1"),
                ("ms", "Rate", Some("us"), "1000"),
            ],
        ),
    )];
    let (_, assembled) = assemble_selected_as(
        ("agent-ix", "example"),
        &packages,
        "dimension Dur;\nunit d : Dur = rational(1, 1);\n\
         unit tick : M::Rate = rational(1, 2) * M::ms;\n\
         dimension Dur ~ M::Rate by 1 d = 1 tick;",
    );
    let declarations = assembled.expect("the equivalence is admitted");
    let [equivalence] = declarations.dimension_equivalences.as_slice() else {
        panic!("one equivalence: {:?}", declarations.dimension_equivalences);
    };
    assert!(
        equivalence.root_scale == exact(500, 1) || equivalence.root_scale == exact(1, 500),
        "{:?}",
        equivalence.root_scale
    );
}

/// FR-091-AC-55 and QSpec FR-142 "Dimension equivalence" step 5: the composed
/// offset of a source unit over an affine package unit is the package
/// unit's, so an equivalence naming it refuses `invalid-value` retaining that
/// offset, at the declaration.
#[trace("TC-483", "FR-091-AC-55", "QSpec-TC-187", "QSpec-FR-142-AC-18")]
#[test]
fn a_source_unit_over_an_affine_package_unit_carries_its_offset_into_step_five() {
    let mut package: Value = serde_json::from_slice(&domain(
        "acme/rpc",
        &[("Rate", &[])],
        &[
            ("us", "Rate", None, "1"),
            ("ms", "Rate", Some("us"), "1000"),
        ],
    ))
    .expect("JSON");
    let units = package["units"].as_array_mut().expect("units");
    for unit in units.iter_mut().filter(|unit| unit["name"] == "ms") {
        unit["offset"] = json!("5");
    }
    let packages: [Package; 1] = [("M", "acme/rpc", serde_json::to_vec(&package).expect("JSON"))];
    let (cause, at) = refused_over(
        &packages,
        "dimension Dur;\nunit d : Dur = rational(1, 1);\n\
         unit tick : M::Rate = rational(1, 2) * M::ms;\n\
         dimension Dur ~ M::Rate by 1 d = 1 tick;",
    );
    assert_eq!(
        cause.catalog_code().to_string(),
        "invalid_package/invalid-value"
    );
    assert!(
        retained(&cause).contains(&exact(5, 1).to_string()),
        "{cause:?}"
    );
    assert!(at.starts_with("dimension Dur ~"), "{at}");
}

/// FR-091-AC-55 and AC-72: a refusal of a package node that is not about one
/// local unit (row 4 beside the package root, row 6 over a cycle of local
/// units, the base-dimension rule) is located at the `model` selection of
/// its package; rows 1 to 3 and row 5 are located at the local unit.
#[trace(
    "TC-483",
    "FR-091-AC-55",
    "FR-091-AC-72",
    "QSpec-TC-187",
    "QSpec-FR-142-AC-18"
)]
#[test]
fn a_package_node_refusal_is_located_at_its_model_selection() {
    let packages = two_domains();
    let selection = "model P = \"acme/p\";";
    let (cause, at) = refused_over(&packages, "unit m2 : P::L = rational(1, 1);");
    assert!(
        matches!(
            &cause,
            AssemblyCause::UnitGraphTopology {
                fault: TopologyFault::TwoRoots,
                ..
            }
        ),
        "{cause:?}"
    );
    assert_eq!(at, selection);
    let (cause, at) = refused_over(
        &packages,
        "unit a : P::L = rational(1, 1) * b; unit b : P::L = rational(1, 1) * a;",
    );
    assert!(
        matches!(
            &cause,
            AssemblyCause::UnitGraphTopology {
                fault: TopologyFault::TargetCycle,
                ..
            }
        ),
        "{cause:?}"
    );
    assert_eq!(at, selection);
    // The base-dimension rule: `L` has no unit in `P`, and `Area` is the
    // local unit's dimension.
    let bare: [Package; 1] = [(
        "P",
        "acme/p",
        domain("acme/p", &[("L", &[]), ("Area", &[("L", "2")])], &[]),
    )];
    let (cause, at) = refused_over(&bare, "unit m2 : P::Area = rational(1, 1);");
    assert!(
        matches!(
            &cause,
            AssemblyCause::UnanchoredBaseDimension { dimension, unit }
                if dimension == "ix://acme/p/dimension/L" && unit == "m2"
        ),
        "{cause:?}"
    );
    assert_eq!(at, selection);
    for (body, local) in [
        ("unit z : P::L = rational(0, 1) * P::m;", "z"),
        ("unit u : P::L = rational(1, 1) * P::ghost;", "P::ghost"),
        ("unit w : Time = rational(1, 1) * P::m;", "w"),
        ("unit t : Q::T = rational(2, 1);", "t"),
    ] {
        let (_, at) = refused_over(&packages, body);
        assert_eq!(at, local, "{body}");
    }
}

/// FR-091-AC-55 and AC-71 (QSpec FR-142 "Dimension equivalence"): when the
/// unit graph is refused, an equivalence naming a source unit typed by a
/// package dimension still resolves that unit's dimension, so the one error
/// is the graph's: row 1 at `z`, the zero scale, and no unresolved name.
#[trace(
    "TC-483",
    "FR-091-AC-55",
    "FR-091-AC-71",
    "QSpec-TC-187",
    "QSpec-FR-142-AC-18"
)]
#[test]
fn a_refused_unit_graph_still_resolves_package_typed_units_for_equivalences() {
    let packages: [Package; 1] = [(
        "M",
        "acme/rpc",
        domain("acme/rpc", &[("Rate", &[])], &[("us", "Rate", None, "1")]),
    )];
    let (cause, at) = refused_over(
        &packages,
        "dimension Dur;\nunit d : Dur = rational(1, 1);\n\
         unit z : Dur = rational(0, 1) * d;\n\
         unit tick : M::Rate = rational(1, 2) * M::us;\n\
         dimension Dur ~ M::Rate by 1 d = 1 tick;",
    );
    assert!(
        matches!(
            &cause,
            AssemblyCause::UnitGraphTopology {
                fault: TopologyFault::ZeroScale,
                retained,
            } if matches!(retained.as_slice(), [
                TopologyRetained::Unit { name, .. },
                TopologyRetained::Scale(scale),
            ] if name == "z" && scale.is_zero())
        ),
        "{cause:?}"
    );
    assert_eq!(at, "z");
}
