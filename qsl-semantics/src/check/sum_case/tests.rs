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
use quire_exact::{Identifier, IllTypedCause, NodeKey, UnionMember, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, TypeEnvironment, UnionMemberDeclaration,
};
use quire_semantic_value::location::{Location, Origin};

use super::mint_union_member_key;
use super::{ExhaustivenessObligation, SumCaseCause};
use crate::check::check::{bind_parameters, Scope, Signatures, Typer};
use crate::check::ir::{CheckedBody, CheckedNode, NodeKind, Slot};
use crate::check::{
    CheckCause, CheckRefusal, CheckingLimitKind, EnumBinding, PackageDeclarations,
    QualifiedCandidate,
};
use crate::value::enumeration::{mint_union_variant_id, mint_variant_id};

const SHAPE: &str = "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
                     union Other { X }\n\
                     enum Kind { A, B }\n";

/// A function over a `Shape` parameter `s` (and a second one `t`), with the
/// given result type and body.
fn function(result: &str, body: &str) -> String {
    format!(
        "function f using v(s: Shape, t: Shape, n: Integer, k: Kind): {result} pure {{ {body} }}"
    )
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
        self.type_body_in(&self.scope(), clause, limits)
    }

    /// The scope the package's own declarations give.
    fn scope(&self) -> Scope {
        let package = &self.package;
        Scope::new(
            package.types.clone(),
            package.operations.clone(),
            package.enums.clone(),
            package.aliases.clone(),
            package.model_operations.clone(),
            package.ieee_profile.clone(),
            Vec::new(),
        )
    }

    /// [`Self::type_body`] over a hand-built `scope`.
    fn type_body_in(
        &self,
        scope: &Scope,
        clause: ClauseKind,
        limits: CheckingLimits,
    ) -> (Result<CheckedBody, CheckRefusal>, u64) {
        let package = &self.package;
        let (parameters, result) = package
            .resolved_signatures
            .get(0)
            .cloned()
            .expect("the assembler resolved f's signature");
        let function = &package.functions[0];
        let signatures = Signatures::default();
        let mut nodes = 0;
        let mut typer = Typer::new(scope, &signatures, limits, &mut nodes, clause);
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
                reads
                    .iter()
                    .any(|(read, ty)| read == slot && ty == &ValueType::Integer),
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
    let (_, refusal) = refused("Integer", "case s { Empty: 0; Empty: 1; Circle(r): r; }");
    let (_, _, obligation) = exhaustiveness(&refusal);
    assert_eq!(
        obligation,
        ExhaustivenessObligation::DuplicateArm,
        "before missing-arm"
    );

    // An unknown or mis-sized arm's body is not checked.
    let (_, refusal) = refused(
        "Integer",
        "case s { Empty: 0; Other::X: true + 1; Circle(r): r; Rect(w, h): w; }",
    );
    assert_eq!(
        exhaustiveness(&refusal).2,
        ExhaustivenessObligation::UnknownMember
    );
    let (_, refusal) = refused(
        "Integer",
        "case s { Empty: 0; Circle(a, b): z; Rect(w, h): w; }",
    );
    assert_eq!(
        exhaustiveness(&refusal).2,
        ExhaustivenessObligation::ArmArity
    );

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
        assert_eq!(
            bad.location,
            Unit::body().child(0),
            "at the scrutinee {scrutinee}"
        );
    }
}

#[test]
fn a_case_checks_under_the_clause_kind_it_stands_in() {
    let unit = unit(&format!(
        "{SHAPE}{}",
        function(
            "Integer",
            "case s { Circle(r): r; Rect(w, h): w; Empty: 0; }"
        )
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
    let (typed, _) = unit.type_body(
        ClauseKind::Body,
        CheckingLimits::default().with_nodes(needed - 1),
    );
    let Err(CheckRefusal {
        cause: CheckCause::ResourceExhausted(limit),
        ..
    }) = typed
    else {
        panic!("one node short is a limit, not {typed:?}")
    };
    assert_eq!(limit.kind, CheckingLimitKind::Nodes);
    assert_eq!(limit.limit, needed - 1);
    let (typed, _) = unit.type_body(
        ClauseKind::Body,
        CheckingLimits::default().with_nodes(needed),
    );
    assert!(typed.is_ok(), "raising the ceiling by one admits it");

    // Through the whole check, the denied node has a region of the source.
    let refusals = unit
        .package
        .check(CheckingLimits::default().with_nodes(needed - 1))
        .expect_err("one node short");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, not {refusals:?}")
    };
    let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
        panic!("a node limit, not {refusal:?}")
    };
    assert_eq!(limit.kind, CheckingLimitKind::Nodes);
    assert!(limit.region.is_some(), "the denied node is located");
}

/// A well-typed `case` checks, but FR-093 keys no union type or `case` node
/// yet, so the package that holds one is refused at lowering with the
/// unsupported-feature cause: an explicit refusal, never a package built
/// without the node (FR-319, FR-320).
#[test]
fn a_package_holding_a_union_is_refused_at_lowering_until_unions_are_keyed() {
    let unit = unit(&format!(
        "{SHAPE}{}",
        function(
            "Integer",
            "case s { Circle(r): r; Rect(w, h): w; Empty: 0; }"
        )
    ));
    let refusals = unit
        .package
        .check(CheckingLimits::default())
        .expect_err("the checked graph cannot lower a union yet");
    assert!(!refusals.is_empty());
    for refusal in &refusals {
        assert!(
            matches!(refusal.cause, CheckCause::UnsupportedFeature { .. }),
            "{refusal:?}"
        );
        assert_eq!(refusal.cause.code(), Code::UnknownRequiredFeature);
    }
}

/// A `case` and its exhaustiveness obligation add no claim (FR-057); the
/// scalar applications in its arm bodies are claim sites of the existing
/// walk, found under each arm's payload binders.
#[test]
fn a_case_adds_no_claim_and_its_arm_bodies_are_claim_sites() {
    let (unit, typed) = check(
        "Integer",
        "case s { Circle(r): r * r * 3; Rect(w, h): w * h; Empty: 0; }",
    );
    let body = typed.expect("the case checks");
    let (parameters, _) = unit
        .package
        .resolved_signatures
        .get(0)
        .cloned()
        .expect("f's signature");
    let claims = crate::check::claims::claims_of(
        body.root(),
        &parameters,
        &Unit::body(),
        &unit.package.types,
        u64::MAX,
    )
    .expect("the claims walk finds the binders of every arm");
    // `r * r`, `(r * r) * 3` and `w * h`; none at the `case` itself.
    assert_eq!(claims.len(), 3);
    assert!(claims
        .iter()
        .all(|claim| claim.site().location() != &Unit::body()));
}

/// A function `f` with the given parameter list over the standard unions and
/// `union F { Measured(Float64[nearest-even]) }`.
fn with_parameters(parameters: &str) -> Unit {
    unit(&format!(
        "{SHAPE}union F {{ Measured(Float64[nearest-even]) }}\n\
         function f using v({parameters}): Integer pure {{ 0 }}"
    ))
}

#[trace("TC-833")]
#[test]
fn collections_of_a_union_are_admitted() {
    let unit = with_parameters(
        "a: Set<Shape>[0, 4], b: Bag<Shape>[0, 4], c: OrderedSet<Shape>[0, 4], \
         d: Sequence<Shape>[0, 4], e: Sequence<F>[0, 4]",
    );
    let (typed, _) = unit.type_body(ClauseKind::Body, CheckingLimits::default());
    typed.expect("the collection types of a union are admitted");
}

#[trace("FR-323-AC-3", "TC-833")]
#[test]
fn a_keyed_collection_of_an_ieee_bearing_union_is_refused() {
    for collection in ["Set<F>[0, 4]", "Bag<F>[0, 4]", "OrderedSet<F>[0, 4]"] {
        let unit = with_parameters(&format!("a: {collection}"));
        let (typed, _) = unit.type_body(ClauseKind::Body, CheckingLimits::default());
        let refusal = typed.expect_err(collection);
        assert_eq!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            "{collection}"
        );
    }
}

/// The payload arguments of a `UnionValue` node, and its member position.
fn union_value(body: &CheckedBody) -> (NodeKey, usize, Vec<(&NodeKind, ValueType)>) {
    let root = body.root();
    let NodeKind::UnionValue {
        declaration,
        member,
        arguments,
    } = root.kind()
    else {
        panic!("one UnionValue node, not {:?}", root.kind())
    };
    let arguments = arguments
        .iter()
        .map(|id| (root.at(*id).kind(), root.at(*id).value_type().clone()))
        .collect();
    (*declaration, *member, arguments)
}

#[trace("FR-317-AC-1", "TC-819")]
#[test]
fn a_union_construction_checks_as_the_member_over_its_union() {
    let (unit, typed) = check("Shape", "Shape::Empty");
    let body = typed.expect("a nullary member checks bare");
    assert_eq!(
        body.root().value_type(),
        &ValueType::Composite(unit.key("Shape"))
    );
    let (declaration, member, arguments) = union_value(&body);
    assert_eq!((declaration, member), (unit.key("Shape"), 0));
    assert!(arguments.is_empty());

    let (unit, typed) = check("Shape", "Shape::Rect(2, 3)");
    let body = typed.expect("a payload member checks with its arguments");
    assert_eq!(
        body.root().value_type(),
        &ValueType::Composite(unit.key("Shape"))
    );
    let (declaration, member, arguments) = union_value(&body);
    assert_eq!((declaration, member), (unit.key("Shape"), 2));
    let types: Vec<ValueType> = arguments.into_iter().map(|(_, ty)| ty).collect();
    assert_eq!(types, [ValueType::Integer, ValueType::Integer]);

    // The member is also the one of the `Other` union when written there.
    let (unit, typed) = check("Other", "Other::X");
    let (declaration, member, _) = union_value(&typed.expect("another union's member"));
    assert_eq!((declaration, member), (unit.key("Other"), 0));

    // A payload is checked against its declared position type.
    let (_, bad) = refused("Shape", "Shape::Rect(2, true)");
    assert_eq!(bad.cause, CheckCause::IllTyped(IllTypedCause::TypeMismatch));
    assert_eq!(bad.location, Unit::body().child(1), "at `true`");
}

#[trace("FR-317-AC-2", "TC-819")]
#[test]
fn a_construction_of_the_wrong_shape_refuses_at_the_construction() {
    for body in [
        "Shape::Square(1)",
        "Shape::Empty(1)",
        "Shape::Circle",
        "Shape::Rect(1)",
    ] {
        let (_, bad) = refused("Shape", body);
        assert_eq!(
            bad.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch),
            "{body}"
        );
        assert_eq!(bad.location, Unit::body(), "{body}");
    }
    // A union name is a type, so an unknown qualifier is an unresolved name.
    let (_, bad) = refused("Shape", "Missing::Empty");
    assert_eq!(bad.cause, CheckCause::MissingName("Missing::Empty".into()));
}

/// Type `Shape::Rect(2, 3)` (and the one-argument `Shape::Rect(1)`) as a
/// `decreases` measure, under the clause kind a measure is typed in.
fn measure_unit(measure: &str) -> Unit {
    unit(&format!(
        "{SHAPE}function f using v(): Integer pure decreases({measure}) {{ 0 }}"
    ))
}

fn type_measure(unit: &Unit) -> Result<CheckedBody, CheckRefusal> {
    let scope = unit.scope();
    let function = &unit.package.functions[0];
    let signatures = Signatures::default();
    let mut nodes = 0;
    let mut typer = Typer::new(
        &scope,
        &signatures,
        CheckingLimits::default(),
        &mut nodes,
        ClauseKind::Body,
    );
    let measure = function.measure.as_ref().expect("f has a measure");
    typer.check_as(measure, &ValueType::Integer, &Unit::body())
}

#[trace("FR-317-AC-4", "TC-819")]
#[test]
fn a_construction_checks_the_same_under_every_clause_kind() {
    let good = unit(&format!(
        "{SHAPE}{}",
        function(
            "Integer",
            "case Shape::Rect(2, 3) { Empty: 0; Circle(r): r; Rect(w, h): w; }"
        )
    ));
    let bad = unit(&format!(
        "{SHAPE}{}",
        function(
            "Integer",
            "case Shape::Rect(1) { Empty: 0; Circle(r): r; Rect(w, h): w; }"
        )
    ));
    for clause in [
        ClauseKind::Body,
        ClauseKind::Invariant,
        ClauseKind::Precondition,
        ClauseKind::Postcondition,
    ] {
        let (typed, _) = good.type_body(clause, CheckingLimits::default());
        assert!(typed.is_ok(), "{clause:?}: {typed:?}");
        let (typed, _) = bad.type_body(clause, CheckingLimits::default());
        let refusal = typed.expect_err("Shape::Rect(1) is refused");
        assert_eq!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch),
            "{clause:?}"
        );
        assert_eq!(refusal.location, Unit::body().child(0), "{clause:?}");
    }

    // A `decreases` measure.
    let measure = "case Shape::Rect(2, 3) { Empty: 0; Circle(r): r; Rect(w, h): w; }";
    type_measure(&measure_unit(measure)).expect("the measure checks");
    let measure = "case Shape::Rect(1) { Empty: 0; Circle(r): r; Rect(w, h): w; }";
    let refusal = type_measure(&measure_unit(measure)).expect_err("Shape::Rect(1) is refused");
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
    assert_eq!(refusal.location, Unit::body().child(0));
}

/// A union named `q` with the given members, under `key`'s digest byte.
fn union_q(key: u8, members: &[(&str, Vec<ValueType>)]) -> CompositeDeclaration {
    CompositeDeclaration::new(
        NodeKey::from_digest([key; 32]),
        "q",
        CompositeShape::Union(
            members
                .iter()
                .map(|(name, payload)| {
                    UnionMemberDeclaration::resolved(
                        Identifier::new(*name).expect("an identifier"),
                        payload.clone(),
                    )
                })
                .collect(),
        ),
    )
}

/// The scope of `unit` with `extra` composites and `enums` added to it.
fn scope_over(unit: &Unit, extra: Vec<CompositeDeclaration>, enums: Vec<EnumBinding>) -> Scope {
    let package = &unit.package;
    let declared: Vec<CompositeDeclaration> =
        package.types.composites().cloned().chain(extra).collect();
    Scope::new(
        TypeEnvironment::new(declared, []).expect("the composites are admitted"),
        package.operations.clone(),
        package.enums.iter().cloned().chain(enums).collect(),
        package.aliases.clone(),
        package.model_operations.clone(),
        package.ieee_profile.clone(),
        Vec::new(),
    )
}

const Q: &str = "union q { m(Integer) }\nenum E { m, z }\n";

fn ambiguity(refusal: &CheckRefusal) -> (&str, &[QualifiedCandidate]) {
    let CheckCause::AmbiguousQualified(ambiguous) = &refusal.cause else {
        panic!("an ambiguous qualified reference, not {:?}", refusal.cause)
    };
    (&ambiguous.name, &ambiguous.candidates)
}

#[trace("FR-317-AC-3", "TC-820")]
#[test]
fn a_qualified_member_with_two_candidates_is_ambiguous_and_names_each() {
    for call in ["q::m(1)", "q::m"] {
        let unit = unit(&format!("{Q}function f using v(): q pure {{ {call} }}"));
        let first = unit.key("q");
        let located = |refusal: &CheckRefusal| {
            assert_eq!(refusal.cause.code(), Code::AmbiguousDeclaration);
            assert_eq!(refusal.cause.cause(), Some("ambiguous-name"));
            assert_eq!(refusal.location, Unit::body(), "{call}");
        };

        // Two unions named `q` that both declare `m`.
        let other = union_q(9, &[("m", vec![ValueType::Integer])]);
        let scope = scope_over(&unit, vec![other], Vec::new());
        let (typed, _) = unit.type_body_in(&scope, ClauseKind::Body, CheckingLimits::default());
        let refusal = typed.expect_err("two unions declare q::m");
        located(&refusal);
        let (name, candidates) = ambiguity(&refusal);
        assert_eq!(name, call.split('(').next().unwrap());
        assert_eq!(candidates.len(), 2, "{call}");
        for key in [first, NodeKey::from_digest([9; 32])] {
            assert!(
                candidates.contains(&QualifiedCandidate::Union(key)),
                "{call}: {candidates:?}"
            );
        }

        // A union and an enum named `q` that both declare `m`.
        let mut renamed = unit.package.enums[0].clone();
        renamed.name = "q".to_owned();
        let enum_key = renamed.declaration.key();
        let scope = scope_over(&unit, Vec::new(), vec![renamed]);
        let (typed, _) = unit.type_body_in(&scope, ClauseKind::Body, CheckingLimits::default());
        let refusal = typed.expect_err("a union and an enum declare q::m");
        located(&refusal);
        assert_eq!(
            ambiguity(&refusal).1,
            [
                QualifiedCandidate::Union(first),
                QualifiedCandidate::Enum(enum_key)
            ],
            "{call}"
        );
    }
}

#[trace("FR-317-AC-3", "TC-820")]
#[test]
fn a_qualified_member_with_one_candidate_resolves_to_it() {
    // Two unions named `q`, only the second declares `m`: it resolves there.
    let unit = unit("union q { n }\nfunction f using v(): q pure { q::n }");
    let first = unit.key("q");
    let other = union_q(9, &[("m", vec![])]);
    let scope = scope_over(&unit, vec![other], Vec::new());
    let (typed, _) = unit.type_body_in(&scope, ClauseKind::Body, CheckingLimits::default());
    let (declaration, member, _) = union_value(&typed.expect("one union declares q::n"));
    assert_eq!((declaration, member), (first, 0));

    // The union `q` alone.
    let (unit, typed) = check("Shape", "Shape::Circle(4)");
    let (declaration, member, _) = union_value(&typed.expect("the union alone"));
    assert_eq!((declaration, member), (unit.key("Shape"), 1));
}

#[trace("FR-317-AC-3", "TC-820")]
#[test]
fn an_enum_member_and_a_tuple_constructor_resolve_as_before_unions() {
    let (_, typed) = check("Kind", "Kind::A");
    let body = typed.expect("an enum member checks");
    assert!(
        matches!(body.root().kind(), NodeKind::Literal(_)),
        "{:?}",
        body.root().kind()
    );

    let unit = unit(&format!(
        "{SHAPE}tuple Pair(Integer, Integer);\n\
         function f using v(): Pair pure {{ Pair(1, 2) }}"
    ));
    let (typed, _) = unit.type_body(ClauseKind::Body, CheckingLimits::default());
    let body = typed.expect("a tuple constructor checks");
    let NodeKind::Tuple {
        declaration,
        arguments,
    } = body.root().kind()
    else {
        panic!("a Tuple node, not {:?}", body.root().kind())
    };
    assert_eq!(*declaration, unit.key("Pair"));
    assert_eq!(arguments.len(), 2);
}

/// QSpec FR-441's member key, computed here from its preimage text rather
/// than by the producer under test: the SHA-256 of the RFC 8785 bytes of
/// `{declaration_node_id, member, version}`, member keys sorted.
fn member_key_hex(declaration: NodeKey, member: &str) -> String {
    use sha2::{Digest, Sha256};
    let preimage = format!(
        "{{\"declaration_node_id\":{{\"digest\":\"{declaration}\",\
         \"domain\":\"quire.checked-semantic-node/v1\"}},\
         \"member\":\"{member}\",\"version\":\"quire.union-member-node/v1\"}}"
    );
    Sha256::digest(preimage.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The scope of `unit` over `types`.
fn scope_of_types(unit: &Unit, types: TypeEnvironment) -> Scope {
    let package = &unit.package;
    Scope::new(
        types,
        package.operations.clone(),
        package.enums.clone(),
        package.aliases.clone(),
        package.model_operations.clone(),
        package.ieee_profile.clone(),
        Vec::new(),
    )
}

/// `unit`'s types with every union sealed under the final key `final_key`
/// gives it, each member bound to the key `mint_union_member_key` mints over
/// that final key. The final declaration key of a union is the checked node
/// id of its type node (FR-319), which lowering owns; a test chooses one.
fn sealed_types(unit: &Unit, final_key: impl Fn(NodeKey) -> NodeKey) -> TypeEnvironment {
    let mut types = unit.package.types.clone();
    let unions: Vec<(NodeKey, Vec<Identifier>)> = types
        .composites()
        .filter_map(|declaration| match declaration.shape() {
            CompositeShape::Union(members) => Some((
                declaration.key(),
                members
                    .iter()
                    .map(|member| member.identifier().clone())
                    .collect(),
            )),
            CompositeShape::Record(_) | CompositeShape::Tuple(_) => None,
        })
        .collect();
    for (handle, members) in unions {
        let key = final_key(handle);
        let bindings = members
            .into_iter()
            .map(|member| {
                UnionMember::from_admitted(key, mint_union_member_key(key, member.as_str()), member)
            })
            .collect();
        types
            .seal_union_verified(handle, key, bindings)
            .expect("the union seals under its final key");
    }
    types
}

#[trace("FR-319-AC-2", "TC-826")]
#[test]
fn a_union_member_variant_id_is_its_member_key_and_selects_only_its_union() {
    let unit = unit(
        "union A { X }\nunion B { X }\nenum E { X }\n\
         union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
         function f using v(): Integer pure { 0 }",
    );
    // A final key distinct from the handle, so a test cannot confuse them.
    let final_of = |handle: NodeKey| {
        let mut digest = *handle.as_bytes();
        digest[0] ^= 0xff;
        NodeKey::from_digest(digest)
    };
    let types = sealed_types(&unit, final_of);

    // Each member's `VariantId` is its FR-441 member key, computed here from
    // the preimage text.
    let shape = final_of(unit.key("Shape"));
    for (position, member) in ["Empty", "Circle", "Rect"].into_iter().enumerate() {
        let variant = mint_union_variant_id(mint_union_member_key(shape, member));
        assert_eq!(
            variant.to_string(),
            member_key_hex(shape, member),
            "{member}"
        );
        assert_eq!(
            mint_union_member_key(shape, member).to_string(),
            variant.to_string(),
            "the VariantId is the member key retyped"
        );
        let (found, declared) = types
            .union_member(unit.key("Shape"), variant)
            .expect("the sealed union resolves its member");
        assert_eq!(found, position);
        assert_eq!(declared.identifier().as_str(), member);
    }

    // `A::X`, `B::X` and an enum `E::X` are three different identities.
    let (a, b) = (final_of(unit.key("A")), final_of(unit.key("B")));
    let enum_key = unit.package.enums[0].declaration.key();
    let ids = [
        mint_union_variant_id(mint_union_member_key(a, "X")),
        mint_union_variant_id(mint_union_member_key(b, "X")),
        mint_variant_id(enum_key, "X"),
    ];
    assert!(ids[0] != ids[1] && ids[0] != ids[2] && ids[1] != ids[2]);

    // A `VariantId` selects only its own union's member.
    assert!(types.union_member(unit.key("A"), ids[0]).is_some());
    assert!(types.union_member(unit.key("A"), ids[1]).is_none());
    assert!(types.union_member(unit.key("A"), ids[2]).is_none());
    assert!(types.union_member(unit.key("B"), ids[0]).is_none());
}

#[trace("FR-319-AC-3", "TC-826")]
#[test]
fn union_equality_compares_one_union_and_refuses_two() {
    for body in [
        "Shape::Rect(2, 3) = Shape::Rect(2, 3)",
        "Shape::Rect(2, 3) = Shape::Rect(3, 2)",
        "Shape::Empty = Shape::Circle(0)",
    ] {
        let unit = unit(&format!(
            "{SHAPE}function f using v(): Boolean pure {{ {body} }}"
        ));
        let scope = scope_of_types(&unit, sealed_types(&unit, |handle| handle));
        let (typed, _) = unit.type_body_in(&scope, ClauseKind::Body, CheckingLimits::default());
        let typed = typed.unwrap_or_else(|refusal| panic!("{body}: {refusal:?}"));
        assert!(
            matches!(typed.root().kind(), NodeKind::Equality(..)),
            "{body}: {:?}",
            typed.root().kind()
        );
    }

    // Two unions of the same member name are different types.
    let unit = unit(
        "union A { X(Integer) }\nunion B { X(Integer) }\n\
         function f using v(): Boolean pure { A::X(1) = B::X(1) }",
    );
    let scope = scope_of_types(&unit, sealed_types(&unit, |handle| handle));
    let (typed, _) = unit.type_body_in(&scope, ClauseKind::Body, CheckingLimits::default());
    let refusal = typed.expect_err("A::X(1) = B::X(1) is refused");
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
    assert_eq!(refusal.location, Unit::body());
}
