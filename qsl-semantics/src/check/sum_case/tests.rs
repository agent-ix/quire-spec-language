// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-318 over real complete-V1 source: S1, S2, the assembler's union
//! registry, then the S3 typer on a function body. Every assertion reads a
//! typed cause, location or checked node, never message text.
//!
//! The checked graph a `case` yields cannot lower to a package yet (FR-319),
//! so these tests type the body directly, through the same `Typer` that
//! `PackageDeclarations::check` drives.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, ClauseKind};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::SourceIdentity;
use quire_exact::{IllTypedCause, NodeKey, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{Location, Origin};

use super::{ExhaustivenessObligation, SumCaseCause};
use crate::check::check::{bind_parameters, Scope, Signatures, Typer};
use crate::check::ir::{CheckedBody, CheckedNode, NodeKind, Slot};
use crate::check::{CheckCause, CheckRefusal, PackageDeclarations};

const SHAPE: &str = "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
                     union Other { X }\n\
                     enum Kind { A, B }\n";

/// A function over a `Shape` parameter `s` (and a second one `t`), with the
/// given result type and body.
fn function(result: &str, body: &str) -> String {
    format!("function f using v(s: Shape, t: Shape, n: Integer, k: Kind): {result} pure {{ {body} }}")
}

struct Unit {
    package: PackageDeclarations,
}

fn unit(declarations: &str) -> Unit {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n{declarations}\n"
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        crate::model::intake::SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("the unit assembles: {refusal:?}"));
    Unit { package }
}

impl Unit {
    /// The declaration key of the union or enum-free composite `name`.
    fn key(&self, name: &str) -> NodeKey {
        self.package
            .types
            .composites()
            .find(|declaration| declaration.name() == name)
            .unwrap_or_else(|| panic!("the unit declares {name}"))
            .key()
    }

    /// The location the body of function `f` is typed at.
    fn body() -> Location {
        Location::root(Origin::Body {
            function: "f".to_owned(),
            index: 0,
        })
    }

    /// Type function `f`'s body against its declared result under `limits`,
    /// returning the typed body or the first refusal, and the node count the
    /// typer charged.
    fn type_body(
        &self,
        clause: ClauseKind,
        limits: CheckingLimits,
    ) -> (Result<CheckedBody, CheckRefusal>, u64) {
        let package = &self.package;
        let scope = Scope::new(
            package.types.clone(),
            package.operations.clone(),
            package.enums.clone(),
            package.aliases.clone(),
            package.model_operations.clone(),
            package.ieee_profile.clone(),
            Vec::new(),
        );
        let (parameters, result) = package
            .resolved_signatures
            .get(0)
            .cloned()
            .expect("the assembler resolved f's signature");
        let function = &package.functions[0];
        let signatures = Signatures::default();
        let mut nodes = 0;
        let mut typer = Typer::new(&scope, &signatures, limits, &mut nodes, clause);
        let location = Self::body();
        let typed = bind_parameters(&mut typer, &parameters, &location)
            .and_then(|()| typer.check_as(&function.body, &result, &location));
        drop(typer);
        (typed, nodes)
    }
}

/// `declarations` plus function `f`, typed as a function body.
fn check(result: &str, body: &str) -> (Unit, Result<CheckedBody, CheckRefusal>) {
    let unit = unit(&format!("{SHAPE}{}", function(result, body)));
    let (typed, _) = unit.type_body(ClauseKind::Body, CheckingLimits::default());
    (unit, typed)
}

fn refused(result: &str, body: &str) -> (Unit, CheckRefusal) {
    let (unit, typed) = check(result, body);
    match typed {
        Err(refusal) => (unit, refusal),
        Ok(_) => panic!("{body}: the typer refuses"),
    }
}

/// Every `Local` read under `node`, with the type each read has.
fn locals(node: CheckedNode<'_>) -> Vec<(Slot, ValueType)> {
    let mut found = Vec::new();
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        if let NodeKind::Local(slot) = node.kind() {
            found.push((*slot, node.value_type().clone()));
        }
        stack.extend(node.kind().children().into_iter().map(|id| node.at(id)));
    }
    found
}

/// The exhaustiveness refusal `refusal` carries.
fn exhaustiveness(refusal: &CheckRefusal) -> (&NodeKey, &[String], ExhaustivenessObligation) {
    let CheckCause::SumCase(cause) = &refusal.cause else {
        panic!("a SumCase cause, not {:?}", refusal.cause)
    };
    let SumCaseCause::UnprovedExhaustiveness {
        union,
        arms,
        obligation,
    } = cause.as_ref();
    (union, arms, *obligation)
}

#[trace("FR-318-AC-1", "TC-821")]
#[test]
fn a_case_over_a_union_checks_with_positional_binders_per_arm() {
    let (unit, typed) = check(
        "Integer",
        "case s { Circle(r): r * r * 3; Shape::Rect(w, h): w * h; Empty: 0; }",
    );
    let body = typed.expect("the case checks");
    let root = body.root();
    assert_eq!(root.value_type(), &ValueType::Integer);
    let NodeKind::Case {
        scrutinee,
        declaration,
        arms,
    } = root.kind()
    else {
        panic!("one Case node, not {:?}", root.kind())
    };
    assert_eq!(*declaration, unit.key("Shape"));
    assert_eq!(
        root.at(*scrutinee).value_type(),
        &ValueType::Composite(unit.key("Shape"))
    );
    // One arm per member, in the union's declared order, not source order.
    let shape: Vec<(usize, usize)> = arms
        .iter()
        .map(|arm| (arm.member, arm.binders.len()))
        .collect();
    assert_eq!(shape, [(0, 0), (1, 1), (2, 2)]);
    // Each binder is an Integer read only in its own arm.
    for arm in arms {
        let reads = locals(root.at(arm.body));
        for slot in &arm.binders {
            assert!(
                reads.iter().any(|(read, ty)| read == slot && ty == &ValueType::Integer),
                "binder {slot:?} is read as an Integer in its arm"
            );
        }
        for other in arms.iter().filter(|other| other.member != arm.member) {
            assert!(
                reads.iter().all(|(read, _)| !other.binders.contains(read)),
                "an arm reads only its own binders"
            );
        }
    }
}

#[trace("FR-318-AC-1", "TC-821")]
#[test]
fn a_binder_is_visible_only_in_its_own_arm_and_a_case_nests() {
    let (_, refusal) = refused(
        "Integer",
        "case s { Circle(r): 0; Shape::Rect(w, h): r; Empty: 0; }",
    );
    assert_eq!(refusal.cause, CheckCause::MissingName("r".into()));

    let (_, typed) = check(
        "Integer",
        "case s { Empty: case t { Empty: 0; Circle(a): a; Rect(b, c): b + c; }; \
         Circle(r): r; Rect(w, h): w; }",
    );
    typed.expect("a case in an arm body checks");
}

#[trace("FR-318-AC-2", "TC-821")]
#[test]
fn without_an_expectation_the_arm_bodies_decide_the_result_type() {
    let at = |arm: usize| Unit::body().child(0).child(1 + arm);
    let (_, bad) = refused(
        "Integer",
        "let c = case s { Circle(r): r; Rect(w, h): true; Empty: 0; } in 0",
    );
    assert_eq!(bad.cause, CheckCause::IllTyped(IllTypedCause::TypeMismatch));
    assert_eq!(bad.location, at(1), "at the body `true`");

    // Integer and Int[..] bodies infer Integer; one shared type is kept.
    let (_, typed) = check(
        "Integer",
        "let c = case s { Circle(r): r; Rect(w, h): 3; Empty: 0; } in c",
    );
    typed.expect("integer bodies infer Integer");
    let (_, typed) = check(
        "Boolean",
        "let c = case s { Circle(r): r = 0; Rect(w, h): false; Empty: true; } in c",
    );
    let body = typed.expect("one shared type is kept");
    assert_eq!(body.root().value_type(), &ValueType::Boolean);
}

#[trace("FR-318-AC-3", "TC-822")]
#[test]
fn each_exhaustiveness_obligation_refuses_with_its_union_arm_set_and_locus() {
    let arm = |index: usize| Unit::body().child(1 + index);
    let cases: [(&str, ExhaustivenessObligation, Location, &[&str]); 4] = [
        (
            "case s { Empty: 0; Empty: 1; Circle(r): r; Rect(w, h): w; }",
            ExhaustivenessObligation::DuplicateArm,
            arm(1),
            &["Empty", "Empty", "Circle", "Rect"],
        ),
        (
            "case s { Empty: 0; Other::X: 1; Circle(r): r; Rect(w, h): w; }",
            ExhaustivenessObligation::UnknownMember,
            arm(1),
            &["Empty", "Other::X", "Circle", "Rect"],
        ),
        (
            "case s { Empty: 0; Circle(a, b): a; Rect(w, h): w; }",
            ExhaustivenessObligation::ArmArity,
            arm(1),
            &["Empty", "Circle", "Rect"],
        ),
        (
            "case s { Empty: 0; Circle(r): r; }",
            ExhaustivenessObligation::MissingArm,
            Unit::body(),
            &["Empty", "Circle"],
        ),
    ];
    for (body, obligation, location, names) in cases {
        let (unit, refusal) = refused("Integer", body);
        let (union, arms, found) = exhaustiveness(&refusal);
        assert_eq!(found, obligation, "{body}");
        assert_eq!(*union, unit.key("Shape"), "{body}");
        assert_eq!(arms, names, "{body}");
        assert_eq!(refusal.location, location, "{body}");
        assert_eq!(refusal.cause.code(), Code::UndefinedExpression);
        assert_eq!(refusal.cause.cause(), Some("unproved-exhaustiveness"));
    }
}

#[trace("FR-318-AC-4", "TC-823")]
#[test]
fn a_case_reports_only_its_first_refusal_in_the_fixed_order() {
    let (_, refusal) = refused(
        "Integer",
        "case s { Empty: 0; Empty: 1; Circle(r): r; }",
    );
    let (_, _, obligation) = exhaustiveness(&refusal);
    assert_eq!(obligation, ExhaustivenessObligation::DuplicateArm, "before missing-arm");

    // An unknown or mis-sized arm's body is not checked.
    let (_, refusal) = refused(
        "Integer",
        "case s { Empty: 0; Other::X: true + 1; Circle(r): r; Rect(w, h): w; }",
    );
    assert_eq!(exhaustiveness(&refusal).2, ExhaustivenessObligation::UnknownMember);
    let (_, refusal) = refused(
        "Integer",
        "case s { Empty: 0; Circle(a, b): z; Rect(w, h): w; }",
    );
    assert_eq!(exhaustiveness(&refusal).2, ExhaustivenessObligation::ArmArity);

    // Typing comes before reachability.
    let (_, bad) = refused("Integer", "case s { Circle(r): true; Empty: 0; }");
    assert_eq!(bad.cause, CheckCause::IllTyped(IllTypedCause::TypeMismatch));
    assert_eq!(bad.location, Unit::body().child(1), "at the body `true`");

    // A scrutinee that is no union ends the builder before any arm.
    for scrutinee in ["n", "k"] {
        let (_, bad) = refused(
            "Integer",
            &format!("case {scrutinee} {{ Missing: 0; Other::X: 1; }}"),
        );
        assert_eq!(bad.cause, CheckCause::IllTyped(IllTypedCause::TypeMismatch));
        assert_eq!(bad.location, Unit::body().child(0), "at the scrutinee {scrutinee}");
    }
}

#[trace("FR-318-AC-5", "TC-824")]
#[test]
fn a_case_checks_under_the_clause_kind_it_stands_in() {
    let unit = unit(&format!(
        "{SHAPE}{}",
        function("Integer", "case s { Circle(r): r; Rect(w, h): w; Empty: 0; }")
    ));
    for clause in [
        ClauseKind::Body,
        ClauseKind::Invariant,
        ClauseKind::Precondition,
        ClauseKind::Postcondition,
    ] {
        let (typed, _) = unit.type_body(clause, CheckingLimits::default());
        assert!(typed.is_ok(), "{clause:?}: {typed:?}");
    }
}

/// `depth` nested cases, each in the `Empty` arm of the one above.
fn nested(depth: usize) -> String {
    let mut body = "0".to_owned();
    for _ in 0..depth {
        body = format!("case s {{ Empty: {body}; Circle(r): r; Rect(w, h): w; }}");
    }
    body
}

#[trace("FR-318-AC-6", "TC-825")]
#[test]
fn nesting_is_bounded_only_by_the_node_ceiling() {
    let unit = unit(&format!("{SHAPE}{}", function("Integer", &nested(1_000))));
    let (typed, needed) = unit.type_body(ClauseKind::Body, CheckingLimits::default());
    typed.expect("1,000 nested cases check under the default ceilings");
    let (typed, _) = unit.type_body(ClauseKind::Body, CheckingLimits::default().with_nodes(needed - 1));
    let Err(CheckRefusal {
        cause: CheckCause::ResourceExhausted(limit),
        ..
    }) = typed
    else {
        panic!("one node short is a limit, not {typed:?}")
    };
    assert_eq!(limit.kind, crate::check::CheckingLimitKind::Nodes);
    assert_eq!(limit.limit, needed - 1);
    let (typed, _) = unit.type_body(ClauseKind::Body, CheckingLimits::default().with_nodes(needed));
    assert!(typed.is_ok(), "raising the ceiling by one admits it");
}
