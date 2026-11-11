// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569: the unit's own `object` declarations (TC-1408), checked on the
//! source route.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{
    AssemblyCause, AssemblyRefusal, CheckCause, CheckedGraph, PackageDeclarations,
    SourceObjectFault,
};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::DomainPackage;
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::normalize::ModelRefusalCause;
use quire_semantic_value::checking::CheckingLimits;

/// TC-1408 fixture S's header.
const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"quire.model.complete/v1\";\n\
    profile Pp = \"quire.protocol.finite-global/v1\";\n";

/// TC-1408 fixture S: the three object declarations.
const OBJECTS: &str = "\
object Order identity (total) {
  total: Int[0, 9];
  note: Text[0, 8; nfc]?;
  population orders;
  operation size(): Integer;
  operation count(): Integer;
  operation scaled(n: Integer): Integer;
  operation bump(): Boolean modifies self.total;
}
object Sub extends Order {
  operation size(): Integer redefines Order::size;
  operation scaled(m: Integer): Integer redefines Order::scaled;
}
abstract object Shape { population shapes; }
";

/// TC-1408 fixture S: the clauses.
const CLAUSES: &str = "\
invariant I using P on Order at current { self.total >= 0 }
pre Q using P on Order::bump { self.total < 9 }
post R using P on Order::bump { self.total > pre(self.total) }
body B using P on Sub::size() { 7 }
body C using P on Order::size() { 1 }
pre Qs using P on Order::size { self.total >= 0 }
invariant Sized using P on Order at current { self.size() >= 0 }
";

/// One refusal as its catalog code and cause.
type Refused = (Code, &'static str);

fn assemble(declarations: &str) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1408"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        SelectedModels::fixture_entries(Vec::new()),
        Vec::new(),
    )
}

/// The unit's refusals as catalog codes, with the span text each is at.
fn refusals(declarations: &str) -> Vec<(Refused, String)> {
    let text = format!("{HEADER}{declarations}\n");
    let refusal = assemble(declarations).expect_err("the unit is refused");
    refusal
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                text[error.span.start..error.span.end].to_owned(),
            )
        })
        .collect()
}

/// The unit's refusals as catalog codes, with the byte offset each is at,
/// relative to the start of `declarations`.
fn refusals_at(declarations: &str) -> Vec<(Refused, usize)> {
    let refusal = assemble(declarations).expect_err("the unit is refused");
    refusal
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                error.span.start - HEADER.len(),
            )
        })
        .collect()
}

/// The offset in `text` of the `occurrence`th (from 0) `needle`, plus `skip`
/// bytes into it.
fn offset(text: &str, needle: &str, occurrence: usize, skip: usize) -> usize {
    text.match_indices(needle)
        .nth(occurrence)
        .unwrap_or_else(|| panic!("`{needle}` occurs {} times", occurrence + 1))
        .0
        + skip
}

/// Links dispatch and checks `declarations`, as the compile driver does.
fn finish(
    mut declarations: PackageDeclarations,
) -> Result<CheckedGraph, Vec<qsl_semantics::check::CheckRefusal>> {
    let pending = declarations.link_dispatch(ModelNormalizationLimits::default())?;
    declarations
        .check(CheckingLimits::default())
        .map_err(|refusals| pending.report(refusals))
}

fn check(declarations: &str) -> CheckedGraph {
    finish(assemble(declarations).expect("the unit assembles")).expect("the unit checks")
}

/// A bare type name resolves to the source object type; an operation of a
/// source type, a population and a `Reference` of a subtype resolve too.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn fixture_s_clauses_resolve_to_the_source_declarations() {
    let graph = check(&format!(
        "{OBJECTS}{CLAUSES}\
         invariant V4 using P on Order at current {{ forall(o in orders : deref(o).total >= 0) }}\n\
         function sub using P (r: Reference<Sub>): Integer pure {{ deref(r).total }}\n"
    ));
    for name in ["I", "Q", "R", "Qs", "V4"] {
        assert!(graph.state_clause(name).is_some(), "{name} is checked");
    }
}

/// An operation `T` declares nothing of, a source record and an unknown name
/// each refuse `missing-name` at the name.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn an_unresolved_context_refuses_missing_name_at_the_name() {
    let missing = (Code::MissingDeclaration, "missing-name");
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}pre X using P on Order::reset {{ true }}"
        )),
        [(missing, "reset".to_owned())]
    );
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}record Receipt {{ total: Integer; }}\n\
             invariant X using P on Receipt at current {{ true }}"
        )),
        [(missing, "Receipt".to_owned())]
    );
    assert_eq!(
        refusals(&format!(
            "{OBJECTS}invariant X using P on Nothing at current {{ true }}"
        )),
        [(missing, "Nothing".to_owned())]
    );
}

mod orders_model {
    include!("../fixtures/orders_model.rs");
}
use orders_model::model_package;

/// The unit that selects fixture D as `M`, with `declarations`.
fn assemble_model(
    declarations: &str,
    package: DomainPackage,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    let text = format!("{HEADER}model M = \"test/orders\";\n{declarations}\n");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc-1408-d"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        built,
        selected,
        Vec::new(),
    )
}

/// Fixture S's clauses written against the model: `on M::Order`.
const MODEL_CLAUSES: &str = "\
invariant I using P on M::Order at current { self.total >= 0 }
pre Q using P on M::Order::bump { self.total < 9 }
post R using P on M::Order::bump { self.total > pre(self.total) }
body B using P on M::Sub::size() { 7 }
body C using P on M::Order::size() { 1 }
pre Qs using P on M::Order::size { self.total >= 0 }
invariant Sized using P on M::Order at current { self.size() >= 0 }
";

const AMBIGUOUS: Refused = (Code::AmbiguousDeclaration, "ambiguous-name");
const MISSING: Refused = (Code::MissingDeclaration, "missing-name");
const MALFORMED: Refused = (Code::InvalidModelBinding, "malformed-declaration");

/// The text of `declarations` as the model unit `assemble_model` assembles.
fn model_text(declarations: &str) -> String {
    format!("{HEADER}model M = \"test/orders\";\n{declarations}\n")
}

/// The refusals of the model unit with `declarations`, as catalog codes with
/// the offset each is at in [`model_text`].
fn model_refusals_at(declarations: &str) -> Vec<(Refused, usize)> {
    assemble_model(declarations, model_package(false))
        .expect_err("the unit is refused")
        .errors
        .iter()
        .map(|error| {
            (
                (error.cause.code(), error.cause.catalog_code().cause()),
                error.span.start,
            )
        })
        .collect()
}

/// FR-569-AC-3: a source object type or population whose name another
/// declaration of the unit's namespace takes is refused at every declaration
/// of the name.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_name_two_declarations_share_is_refused_at_each() {
    // `record Order` beside `object Order`: both declarations.
    let text = format!("record Order {{ total: Integer; }}\n{OBJECTS}");
    assert_eq!(
        refusals_at(&text),
        [
            (AMBIGUOUS, offset(&text, "Order", 0, 0)),
            (AMBIGUOUS, offset(&text, "object Order", 0, 7)),
        ]
    );
    // Population `orders` beside a function `orders`: the population's name
    // and the function's declaration.
    let text = format!("{OBJECTS}function orders using P (): Integer pure {{ 1 }}\n");
    assert_eq!(
        refusals_at(&text),
        [
            (AMBIGUOUS, offset(&text, "population orders", 0, 11)),
            (AMBIGUOUS, offset(&text, "function orders", 0, 0)),
        ]
    );
}

/// `object M` in a unit with `model M`: both declarations.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn an_object_named_like_a_model_alias_is_refused_at_both() {
    let refused = model_refusals_at("object M { }");
    let text = model_text("object M { }");
    assert_eq!(
        refused,
        [
            (AMBIGUOUS, offset(&text, "model M", 0, 0)),
            (AMBIGUOUS, offset(&text, "object M", 0, 7)),
        ]
    );
}

/// A quantifier binder named like a source population is ambiguous at the
/// use, naming both.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_binder_named_like_a_population_is_ambiguous() {
    let declarations = assemble(&format!(
        "{OBJECTS}invariant Bad using P on Order at current {{ forall(orders in orders : true) }}\n"
    ))
    .expect("the unit assembles");
    let refusals = finish(declarations).expect_err("the binder is ambiguous");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::AmbiguousName { name, loci } = &refusal.cause else {
        panic!("an ambiguous name: {refusal:?}");
    };
    assert_eq!(name, "orders");
    assert_eq!(loci.len(), 2, "the use and the binder: {loci:?}");
    // A binder of another name over the same population checks.
    check(&format!(
        "{OBJECTS}invariant Good using P on Order at current {{ forall(o in orders : true) }}\n"
    ));
}

/// Members of one object type: two fields of a name (the second), a field
/// named like an inherited field (the field), an operation named like an
/// inherited one without `redefines` (the operation).
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_repeated_member_name_is_refused_at_the_later_member() {
    let text = format!("{OBJECTS}object Twin {{ total: Integer; total: Integer; }}\n");
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "total: Integer; total", 0, 16))]
    );
    let text = OBJECTS.replace(
        "object Sub extends Order {",
        "object Sub extends Order {\n  total: Integer;",
    );
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "total: Integer;", 0, 0))]
    );
    let text = OBJECTS.replace(
        "  operation size(): Integer redefines Order::size;",
        "  operation size(): Integer;",
    );
    assert_eq!(
        refusals_at(&text),
        [(
            AMBIGUOUS,
            offset(
                &text,
                "operation size(): Integer;\n  operation scaled(m",
                0,
                10
            )
        )]
    );
}

/// `deletes Order` in an operation whose parameter is also named `Order`.
#[trace("FR-569-AC-3", "TC-1408")]
#[test]
fn a_deletes_name_that_is_a_parameter_and_a_type_is_ambiguous() {
    let text = OBJECTS.replace(
        "  operation count(): Integer;",
        "  operation drop(Order: Reference<Order>) deletes Order;",
    );
    assert_eq!(
        refusals_at(&text),
        [(AMBIGUOUS, offset(&text, "deletes Order", 0, 8))]
    );
}

/// FR-569-AC-4: each declaration check refuses once, with its code and cause,
/// at the offending name.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_declaration_check_refuses_with_its_code_at_the_offending_name() {
    let cycle = (Code::InvalidPackage, "definition-cycle");
    // (declarations, the text the refusal is at, its code and cause)
    let cases: [(&str, &str, Refused); 8] = [
        (
            "object X extends Receipt { }\nrecord Receipt { total: Integer; }",
            "Receipt",
            MALFORMED,
        ),
        ("object X extends Ghost { }", "Ghost", MISSING),
        (
            "object X identity (ghost) { total: Integer; }",
            "ghost",
            MISSING,
        ),
        (
            "object X { total: Integer; operation f() modifies self.ghost; }",
            "ghost",
            MISSING,
        ),
        (
            "object X { operation f() creates Ghost; }",
            "Ghost",
            MISSING,
        ),
        (
            "object X { operation f(n: Integer) deletes n; }",
            "n;",
            MALFORMED,
        ),
        (
            "object X { operation f() redefines Order::reset; }",
            "Order::reset",
            MISSING,
        ),
        (
            "object A extends B { }\nobject B extends A { }",
            "B {",
            cycle,
        ),
    ];
    for (declarations, at, expected) in cases {
        let text = format!("{OBJECTS}{declarations}");
        // The needle's first occurrence inside the case's own declarations.
        let start = text.find(declarations).unwrap()
            + declarations
                .find(at)
                .unwrap_or_else(|| panic!("`{at}` in {declarations}"));
        let at_extends = if at == "B {" {
            // The cycle is reported at the first `extends` entry.
            text.find(declarations).unwrap() + declarations.find("B").unwrap()
        } else {
            start
        };
        assert_eq!(
            refusals_at(&text),
            [(expected, at_extends)],
            "{declarations}"
        );
    }
}

/// `extends M::Order` names a model type: `malformed-declaration` at the
/// entry.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn extending_a_model_type_is_refused_at_the_entry() {
    let declarations = "object X extends M::Order { }";
    let text = model_text(declarations);
    assert_eq!(
        model_refusals_at(declarations),
        [(MALFORMED, offset(&text, "M::Order", 0, 0))]
    );
}

/// A redefinition whose parameter type is not contravariant refuses
/// `variance-parameter` once, naming the redefining member.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_redefinition_with_a_narrower_parameter_refuses_variance_parameter() {
    let text = OBJECTS.replace(
        "operation scaled(m: Integer): Integer redefines",
        "operation scaled(m: Text[0, 8; nfc]): Integer redefines",
    );
    let declarations = assemble(&format!("{text}{CLAUSES}")).expect("the unit assembles");
    let refusals = finish(declarations).expect_err("the parameter is not contravariant");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::ModelConformance(failure) = &refusal.cause else {
        panic!("a conformance failure: {refusal:?}");
    };
    assert!(failure.member.node.ends_with("/Sub/scaled"), "{failure:?}");
    assert!(
        failure.parent.node.ends_with("/Order/scaled"),
        "{failure:?}"
    );
    assert!(
        matches!(&failure.cause, ModelRefusalCause::VarianceParameter { .. }),
        "{failure:?}"
    );
}

/// `redefines Order::reset`: `Order` is a proper supertype that exposes no
/// `reset`, so the refusal is at the member.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_redefinition_of_an_operation_the_supertype_lacks_is_refused_at_the_operation() {
    let text = OBJECTS.replace(
        "operation size(): Integer redefines Order::size;",
        "operation size(): Integer redefines Order::reset;",
    );
    assert_eq!(
        refusals_at(&text),
        [(MISSING, offset(&text, "Order::reset", 0, 7))]
    );
}

/// `A extends B` with `B extends A` is one `definition-cycle` naming both
/// edges.
#[trace("FR-569-AC-4", "TC-1408")]
#[test]
fn a_supertype_cycle_is_one_definition_cycle_naming_both_edges() {
    let refusal = assemble("object A extends B { }\nobject B extends A { }")
        .expect_err("the cycle is refused");
    let [error] = &refusal.errors[..] else {
        panic!("one refusal: {refusal:?}");
    };
    let AssemblyCause::SourceObject(SourceObjectFault::SupertypeCycle { edges }) = &error.cause
    else {
        panic!("a cycle: {error:?}");
    };
    assert_eq!(
        edges,
        &[
            ("A".to_owned(), "B".to_owned()),
            ("B".to_owned(), "A".to_owned())
        ]
    );
    assert_eq!(
        (error.cause.code(), error.cause.catalog_code().cause()),
        (Code::InvalidPackage, "definition-cycle")
    );
}

/// Every checked node type under `node`, with the package identity written
/// `P`, so that the source and model routes compare equal.
fn type_names(node: qsl_semantics::check::CheckedNode<'_>, out: &mut Vec<String>) {
    out.push(
        format!("{:?}", node.value_type())
            .replace("source/qsl-semantics-integration-tests", "P")
            .replace("test/orders", "P"),
    );
    for child in node.children() {
        type_names(child, out);
    }
}

/// The checked types of the clause `name`'s expression.
fn clause_types(graph: &CheckedGraph, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    type_names(
        graph.state_clause(name).expect("the clause").body(),
        &mut out,
    );
    out
}

/// The checked types of the function `name`'s body.
fn function_types(graph: &CheckedGraph, name: &str) -> Vec<String> {
    let mut out = Vec::new();
    let body = (0..)
        .map_while(|index| graph.function_state(index))
        .find(|function| function.name == name)
        .expect("the function")
        .body;
    type_names(body, &mut out);
    out
}

/// The receiver type names of a dispatch table's entries.
fn tables_receivers(table: &qsl_semantics::check::DispatchTable) -> Vec<String> {
    table
        .entries()
        .iter()
        .map(|(receiver, _)| {
            format!("{receiver:?}")
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .trim_end_matches("\" }])")
                .to_owned()
        })
        .collect()
}

/// A checked expression's node kinds, package identities written `P`.
fn shape_of(node: qsl_semantics::check::CheckedNode<'_>, out: &mut String) {
    out.push_str(
        &format!("{:?};", node.kind())
            .replace("source/qsl-semantics-integration-tests", "P")
            .replace("test/orders", "P"),
    );
    for child in node.children() {
        shape_of(child, out);
    }
}

/// One dispatch entry without its receiver's package identity: the receiver
/// type's name, body, precondition and precondition clauses.
type CandidateShape = (String, usize, Option<usize>, Vec<usize>);

/// The dispatch tables' candidates, by receiver type name.
fn tables(graph: &CheckedGraph) -> Vec<Vec<CandidateShape>> {
    graph
        .dispatch_tables()
        .iter()
        .map(|table| {
            table
                .entries()
                .iter()
                .map(|(receiver, candidate)| {
                    let receiver = format!("{receiver:?}");
                    let name = receiver
                        .rsplit("/")
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches("\" }])")
                        .to_owned();
                    (
                        name,
                        candidate.body,
                        candidate.precondition,
                        candidate.precondition_clauses.clone(),
                    )
                })
                .collect()
        })
        .collect()
}

/// The source `Sub`-typed and `Order`-typed expressions of AC-5, with the
/// names `object` qualifies by `qualifier`.
fn ac5_extras(qualifier: &str) -> String {
    format!(
        "bound B on {q}orders [3]\n\
         invariant Cap using P on {q}Order at current {{ count(o in {q}orders where o.total = 0) < 4 }}\n\
         invariant Every using P on {q}Order at current {{ forall(o in allInstances<{q}Order>({q}orders) : deref(o).total >= 0) }}\n\
         invariant SizedSub using P on {q}Sub at current {{ self.size() >= 0 }}\n\
         function totalOf using P (s: Reference<{q}Sub>): Int[0, 9] pure {{ deref(s).total }}\n",
        q = qualifier
    )
}

/// FR-569-AC-5 (S-R): a clause on `Sub::size` joins `Sub.size`'s effective
/// precondition as `Qs2 or Qs` with the own clause first, and S and D link
/// the same dispatch tables and check the same types.
#[trace("FR-569-AC-5", "TC-1408")]
#[test]
fn the_source_and_model_twins_link_equal_tables_and_check_equal_types() {
    let source = check(&format!(
        "{OBJECTS}{CLAUSES}pre Qs2 using P on Sub::size {{ self.total >= 1 }}\n{}",
        ac5_extras("")
    ));
    let model = finish(
        assemble_model(
            &format!(
                "{MODEL_CLAUSES}pre Qs2 using P on M::Sub::size {{ self.total >= 1 }}\n{}",
                ac5_extras("M::")
            ),
            model_package(false),
        )
        .expect("the model twin assembles"),
    )
    .expect("the model twin checks");

    // Dispatch tables: equal, with `Sub.size` carrying its own clause first.
    let (source_tables, model_tables) = (tables(&source), tables(&model));
    assert_eq!(source_tables, model_tables);
    assert!(source_tables.iter().any(|table| {
        table
            .iter()
            .all(|entry| entry.0 == "Order" || entry.0 == "Sub")
    }));
    let precondition_of = |graph: &CheckedGraph, name: &str| -> String {
        let table = graph
            .dispatch_tables()
            .iter()
            .find(|table| tables_receivers(table).iter().any(|r| r == "Sub"))
            .expect("the size table");
        let (_, candidate) = table
            .entries()
            .iter()
            .find(|(receiver, _)| format!("{receiver:?}").contains(&format!("/{name}\"")))
            .unwrap_or_else(|| panic!("{name} has an entry"));
        let function = graph
            .function_state(candidate.precondition.expect("an effective precondition"))
            .expect("the precondition function");
        let mut shape = String::new();
        shape_of(function.body, &mut shape);
        shape
    };
    // `Sub.size`'s effective precondition is `Qs2 or Qs`, the own clause
    // first (`>= 1` is Qs2, `>= 0` is Qs); `Order.size`'s is `Qs` alone.
    let sub = precondition_of(&source, "Sub");
    assert!(sub.starts_with("Connective(Or"), "{sub}");
    assert!(
        sub.find("Integer(1)").unwrap() < sub.find("Integer(0)").unwrap(),
        "Qs2 first: {sub}"
    );
    assert!(!precondition_of(&source, "Order").contains("Connective"));
    assert_eq!(sub, precondition_of(&model, "Sub"));
    assert_eq!(
        precondition_of(&source, "Order"),
        precondition_of(&model, "Order")
    );

    // Checked types: equal on the two routes, and the expected ones.
    for clause in ["Cap", "Every", "SizedSub"] {
        assert_eq!(clause_types(&source, clause), clause_types(&model, clause));
    }
    assert_eq!(
        function_types(&source, "totalOf"),
        function_types(&model, "totalOf")
    );
    let cap = clause_types(&source, "Cap");
    assert!(
        cap.contains(&"Int(IntegerInterval { lower: Integer(0), upper: Integer(3) })".to_owned())
    );
    assert!(cap.iter().any(|t| t.contains("kind: Set")
        && t.contains("bound: Some(CardinalityBound { minimum: 0, maximum: 3 })")
        && t.contains("ix://P/Order")));
    assert!(cap.contains(&"Population(Some(3))".to_owned()));
    let every = clause_types(&source, "Every");
    assert!(every
        .iter()
        .any(|t| t.contains("kind: Set") && t.contains("ix://P/Order")));
    let total_of = function_types(&source, "totalOf");
    assert_eq!(
        total_of[0], "Int(IntegerInterval { lower: Integer(0), upper: Integer(9) })",
        "deref(s).total is the field's Int[0, 9]"
    );
    assert!(total_of[1].contains("ix://P/Sub"), "s is a Reference<Sub>");
}

/// FR-569-AC-5 (S-R'): `Sub` redefines `bump` and also modifies `note`, which
/// `Order.bump` does not; S and D each refuse one `effect-escape` on the
/// `note` field of the redefining member.
#[trace("FR-569-AC-5", "TC-1408")]
#[test]
fn a_redefinition_that_widens_the_frame_refuses_effect_escape_on_both_routes() {
    let text = OBJECTS.replace(
        "operation scaled(m: Integer): Integer redefines Order::scaled;",
        "operation scaled(m: Integer): Integer redefines Order::scaled;\n  \
         operation bump(): Boolean redefines Order::bump modifies self.total, self.note;",
    );
    let source = finish(assemble(&format!("{text}{CLAUSES}")).expect("S-R' assembles"))
        .expect_err("the frame escapes");
    let model = finish(assemble_model(MODEL_CLAUSES, model_package(true)).expect("D-R' assembles"))
        .expect_err("the frame escapes");
    for (route, refusals) in [("S-R'", source), ("D-R'", model)] {
        let [refusal] = &refusals[..] else {
            panic!("{route}: one refusal: {refusals:?}");
        };
        let CheckCause::ModelConformance(failure) = &refusal.cause else {
            panic!("{route}: a conformance failure: {refusal:?}");
        };
        assert_eq!(
            (failure.code, failure.axis),
            (Code::IllTyped, "effect"),
            "{route}"
        );
        assert!(
            failure.member.node.ends_with("/Sub/bump"),
            "{route}: {failure:?}"
        );
        assert!(
            failure.parent.node.ends_with("/Order/bump"),
            "{route}: {failure:?}"
        );
        assert!(
            matches!(&failure.cause, ModelRefusalCause::EffectEscape { field } if field.node.ends_with("/Order/note")),
            "{route}: {failure:?}"
        );
    }
}

/// FR-569-AC-2: `on Order` and `on M::Order` check against different field
/// sets when the source `Order` declares a field the model's lacks.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_bare_and_a_qualified_context_see_different_field_sets() {
    let objects = OBJECTS.replace(
        "  total: Int[0, 9];",
        "  total: Int[0, 9];\n  extra: Integer;",
    );
    // `extra` is a field of the source `Order` only.
    let both = |clause: &str| {
        finish(
            assemble_model(&format!("{objects}{clause}"), model_package(false))
                .expect("the unit assembles"),
        )
    };
    both("invariant OnSource using P on Order at current { self.extra >= 0 }\n")
        .expect("`on Order` sees `extra`");
    let refusals = both("invariant OnModel using P on M::Order at current { self.extra >= 0 }\n")
        .expect_err("`on M::Order` does not");
    assert!(
        matches!(&refusals[..], [refusal] if matches!(&refusal.cause, CheckCause::StateModel(_) | CheckCause::MissingName(_))),
        "{refusals:?}"
    );
    // The model's `note` is seen by both.
    both(
        "invariant A using P on Order at current { present(self.note) or true }\n\
         invariant B using P on M::Order at current { present(self.note) or true }\n",
    )
    .expect("both see `note`");
}

/// A protocol `Sell` over `over_type`, with `w` the role on the whole model
/// `M` and `Seller` a static role on the source `Order`; its `finish` reads
/// `view.extra`, a field of the source `Order` only.
fn sell(over_type: &str) -> String {
    format!(
        "protocol Sell using Pp over (view: {over_type}) on origin {{\n\
           role w on M;\n\
           role Seller on Order;\n\
           run sequence Main {{\n\
             attempt A by Seller on Order::bump contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ view.extra >= 0 }};\n\
         }}\n"
    )
}

/// FR-218-AC-39 (S-M): `w` is the role on the whole model `M` and `Seller` a
/// static role on the source `Order`. `over (view: Order)` has the source
/// `Order` as its model context, so it reads the source-only field `extra`;
/// `over (view: M::Order)` has the model's, which has none.
#[trace("FR-218-AC-39", "FR-569-AC-2", "TC-1408")]
#[test]
fn a_protocol_roles_and_over_resolve_to_the_source_or_the_model_by_spelling() {
    let objects = OBJECTS.replace(
        "  total: Int[0, 9];",
        "  total: Int[0, 9];\n  extra: Integer;",
    );
    let on_source = finish(
        assemble_model(&format!("{objects}{}", sell("Order")), model_package(false))
            .expect("the S-M unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("over Order: {refusals:?}"));
    let [protocol] = on_source.protocols() else {
        panic!("one protocol");
    };
    let role = |name: &str| {
        protocol
            .roles
            .iter()
            .find(|role| role.name == name)
            .unwrap_or_else(|| panic!("role {name}"))
    };
    // `w` is on the whole model `M`: it names the model's package and no
    // object type. `Seller` is on the source `Order`: an object type of the
    // source package.
    assert_eq!(role("w").model_selection.as_deref(), Some("test/orders"));
    assert_eq!(role("w").context, None);
    assert!(role("Seller").context.is_some());
    assert_eq!(role("Seller").model_selection, None);

    // The same protocol over the model's `Order` cannot read `extra`.
    let refusals = finish(
        assemble_model(
            &format!("{objects}{}", sell("M::Order")),
            model_package(false),
        )
        .expect("the S-M unit assembles"),
    )
    .map(|_| ())
    .expect_err("M::Order has no `extra`");
    assert!(!refusals.is_empty());
    // And over the model's `Order` without reading `extra`, it checks: the
    // two contexts differ only in what they see.
    let without = sell("M::Order").replace("view.extra >= 0", "view.total >= 0");
    finish(
        assemble_model(&format!("{objects}{without}"), model_package(false))
            .expect("the S-M unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("over M::Order: {refusals:?}"));
}

/// Source objects are unit-scoped: a clause in another unit of the package
/// that names a source type refuses `missing-name`, whichever unit holds the
/// declaration.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_source_type_is_visible_only_in_the_unit_that_declares_it() {
    let header = "profile P = \"quire.model.complete/v1\";\n";
    let objects = format!("{header}{OBJECTS}");
    let clause = format!("{header}invariant X using P on Order at current {{ self.total >= 0 }}\n");
    // The declaring unit's own clause checks.
    let alone = crate::model_operations::try_assemble_units(
        &[(
            "a",
            &format!(
                "{objects}{clause_body}",
                clause_body = "invariant X using P on Order at current { self.total >= 0 }\n"
            ),
        )],
        &[],
    );
    assert!(alone.is_ok(), "{:?}", alone.err());
    // A clause in another unit, whichever order the units sort in.
    for units in [
        [("a", &objects), ("b", &clause)],
        [("a", &clause), ("b", &objects)],
    ] {
        let refusal = crate::model_operations::try_assemble_units(
            &[
                (units[0].0, units[0].1.as_str()),
                (units[1].0, units[1].1.as_str()),
            ],
            &[],
        )
        .expect_err("the other unit does not see the type");
        let [error] = &refusal.errors[..] else {
            panic!("one refusal: {refusal:?}");
        };
        assert!(
            matches!(&error.cause, AssemblyCause::UnresolvedTypeName { name } if name == "Order"),
            "{error:?}"
        );
        assert_eq!(
            (error.cause.code(), error.cause.catalog_code().cause()),
            MISSING
        );
    }
}

/// The profile selections both units of the two-unit tests write.
const UNIT_PROFILES: &str = "profile P = \"quire.model.complete/v1\";\n\
    profile Pp = \"quire.protocol.finite-global/v1\";\n";

/// A protocol `Sell` over `over_type` with the role `Seller` on `role_type`,
/// acting through `operation`.
fn unit_protocol(over_type: &str, role_type: &str, operation: &str) -> String {
    format!(
        "protocol Sell using Pp over (view: {over_type}) on origin {{\n\
           role Seller on {role_type};\n\
           run sequence Main {{\n\
             attempt A by Seller on {operation} contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ true }};\n\
         }}\n"
    )
}

/// A source population is named in the unit that declares it only: another
/// unit's clause naming it refuses `missing-name`.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_source_population_is_visible_only_in_the_unit_that_declares_it() {
    let declaring = format!("{UNIT_PROFILES}{OBJECTS}");
    let other = format!(
        "{UNIT_PROFILES}object Other {{ }}\n\
         invariant X using P on Other at current {{ forall(o in orders : true) }}\n"
    );
    let units = [("a", declaring.as_str()), ("b", other.as_str())];
    let declarations =
        crate::model_operations::try_assemble_units(&units, &[]).expect("the units assemble");
    let refusals = finish(declarations).expect_err("`orders` is not visible in unit b");
    let [refusal] = &refusals[..] else {
        panic!("one refusal: {refusals:?}");
    };
    // It is refused as a name nothing declares is.
    assert!(
        matches!(&refusal.cause, CheckCause::MissingName(name) if name == "orders"),
        "{refusal:?}"
    );

    // The declaring unit's own clause over the population checks.
    let own = format!(
        "{declaring}invariant Y using P on Order at current {{ forall(o in orders : true) }}\n"
    );
    let stub = format!("{UNIT_PROFILES}object Stub {{ }}\n");
    let units = [("a", own.as_str()), ("b", stub.as_str())];
    finish(crate::model_operations::try_assemble_units(&units, &[]).expect("assembles"))
        .expect("the declaring unit sees its population");
}

const UNIT_TEMPORAL_PROFILE: &str = "profile t = \"quire.temporal.infinite-trace/v1\";\n";

/// A temporal clause over `over_type` whose body names the population `orders`.
fn unit_temporal(over_type: &str) -> String {
    format!(
        "temporal C using t over (w: {over_type}) clock \"model-steps\" \
         on origin {{ eventually holds(forall(x in orders : true)) }}\n"
    )
}

/// A temporal clause names a source population in the unit that declares it
/// only: another unit's clause naming it refuses `missing-name`, and the
/// declaring unit's own clause checks.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_temporal_clause_sees_only_its_own_units_populations() {
    let declaring = format!("{UNIT_PROFILES}{OBJECTS}");
    let other = format!(
        "{UNIT_PROFILES}{UNIT_TEMPORAL_PROFILE}object Watch {{ }}\n{}",
        unit_temporal("Watch")
    );
    let units = [("a", declaring.as_str()), ("b", other.as_str())];
    let declarations =
        crate::model_operations::try_assemble_units(&units, &[]).expect("the units assemble");
    let refusals = finish(declarations).expect_err("`orders` is not visible in unit b");
    assert!(
        refusals.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::MissingName(name) if name == "orders"
        )),
        "{refusals:?}"
    );
    // The declaring unit's own temporal clause over the population checks.
    let own = format!(
        "{UNIT_PROFILES}{UNIT_TEMPORAL_PROFILE}{OBJECTS}{}",
        unit_temporal("Order")
    );
    let stub = format!("{UNIT_PROFILES}object Stub {{ }}\n");
    let units = [("a", own.as_str()), ("b", stub.as_str())];
    finish(crate::model_operations::try_assemble_units(&units, &[]).expect("assembles"))
        .expect("the declaring unit sees its population in a temporal clause");
}

/// A role and an `over` of another unit cannot name a unit's source type;
/// a secondary unit's own clause and protocol resolve the type it declares.
#[trace("FR-569-AC-2", "FR-218-AC-39", "TC-1408")]
#[test]
fn a_role_and_an_over_see_only_their_own_units_source_types() {
    let local = "object Local { operation go(): Boolean; }\n";
    let declaring = format!("{UNIT_PROFILES}{OBJECTS}");
    for (variant, protocol) in [
        ("over", unit_protocol("Order", "Local", "Local::go")),
        ("role", unit_protocol("Local", "Order", "Local::go")),
    ] {
        let user = format!("{UNIT_PROFILES}{local}{protocol}");
        // `a` uses the type `b` declares, whichever way they sort.
        let assembled = crate::model_operations::try_assemble_units(
            &[("a", user.as_str()), ("b", declaring.as_str())],
            &[],
        );
        // The unit that does not declare `Order` cannot name it, whether
        // the refusal is the assembler's or the check's.
        match assembled {
            Err(refusal) => {
                assert!(!refusal.errors.is_empty(), "{variant}");
                for error in &refusal.errors {
                    assert!(
                        matches!(&error.cause, AssemblyCause::UnresolvedTypeName { name } if name == "Order"),
                        "{variant}: {error:?}"
                    );
                    assert_eq!(
                        (error.cause.code(), error.cause.catalog_code().cause()),
                        MISSING,
                        "{variant}"
                    );
                }
            }
            Ok(declarations) => {
                let refusals = finish(declarations).expect_err(variant);
                assert!(
                    refusals.iter().all(|refusal| match &refusal.cause {
                        CheckCause::MissingName(name) => name == "Order",
                        CheckCause::ProtocolAnchor(anchor) => {
                            {
                                let text = format!("{anchor:?}");
                                text.contains("MissingName(\"Order\")")
                                    || text.contains(
                                        "MissingRoleType { role: \"Seller\", context: \"Order\"",
                                    )
                            }
                        }
                        _ => false,
                    }),
                    "{variant}: {refusals:?}"
                );
            }
        }
    }

    // The secondary unit `b` declares `Order` and uses it in a clause and a
    // protocol of its own.
    let unit_stub = format!("{UNIT_PROFILES}object Stub {{ }}\n");
    let own = format!(
        "{declaring}invariant Y using P on Order at current {{ self.total >= 0 }}\n{}",
        unit_protocol("Order", "Order", "Order::bump")
    );
    let graph = finish(
        crate::model_operations::try_assemble_units(
            &[("a", unit_stub.as_str()), ("b", own.as_str())],
            &[],
        )
        .expect("the secondary unit assembles"),
    )
    .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert!(graph.state_clause("Y").is_some());
    assert_eq!(graph.protocols().len(), 1);
}

/// Fixture D with the record value type `Flag { ready: Boolean }` beside its
/// own declarations.
fn model_with_flag() -> DomainPackage {
    use orders_model::{field, key};
    use qsl_semantics::model::domain_package::{
        DomainPackageRecord, NativeValueType, RecordValueTypeRecord, ValueTypeRef,
    };
    let mut package = model_package(false);
    package.records.push(DomainPackageRecord::RecordValueType(
        RecordValueTypeRecord {
            key: key("Flag"),
            identity_fields: Vec::new(),
            supertypes: Vec::new(),
        },
    ));
    package.records.push(field(
        "Flag",
        "ready",
        ValueTypeRef::Native(NativeValueType::Boolean),
        quire_exact::Presence::Required,
        0,
    ));
    package
}

/// FR-569 "Declaration checks": a field, parameter or result of a source
/// object type may name a model-owned value type, a reference to a model
/// object type, or a collection of either; each checks, and a clause over the
/// type reads the model-typed field.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_source_member_may_have_a_model_owned_type() {
    let declarations = "\
object Z {
  total: M::Total;
  flag: M::Flag?;
  link: Reference<M::Order>;
  totals: Set<M::Total>[0, 3];
  population zs;
  operation step(): M::Flag;
  operation add(x: M::Total, o: Reference<M::Order>): M::Total;
}
invariant X using P on Z at current { self.total >= 0 }
invariant Y using P on Z at current { forall(z in zs : deref(z).total <= 9) }
";
    let declarations =
        assemble_model(declarations, model_with_flag()).expect("the member types resolve");
    finish(declarations).expect("the clauses over the model-typed fields check");
}

/// A model type that does not exist, and a model-owned type a field cannot
/// hold, refuse at the type.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_model_typed_member_that_names_nothing_is_refused_at_the_name() {
    let text = "object Z { total: M::Ghost; }\n";
    let refused = assemble_model(text, model_package(false)).expect_err("M::Ghost names nothing");
    let [error] = &refused.errors[..] else {
        panic!("one refusal: {refused:?}");
    };
    assert_eq!(
        (error.cause.code(), error.cause.catalog_code().cause()),
        MISSING
    );
    let source = model_text(text);
    assert_eq!(&source[error.span.start..error.span.end], "M::Ghost");
}

/// FR-105-AC-18 (the source route's part): `pre Q` and `post R` on
/// `Order::bump` check against one operation whose declaring type is the
/// source-declared `Order`, and a clause on `Sub::bump` resolves to `Order`'s
/// `bump`, which `Sub` inherits.
#[trace("FR-105-AC-18", "TC-1408")]
#[test]
fn clauses_on_a_source_operation_share_the_declaring_type() {
    let graph = check(&format!(
        "{OBJECTS}{CLAUSES}pre Qsub using P on Sub::bump {{ self.total < 9 }}\n"
    ));
    let order = graph.state_clause("I").expect("I is checked").context();
    let operation = |name: &str| {
        graph
            .state_clause(name)
            .and_then(|clause| clause.operation())
            .unwrap_or_else(|| panic!("{name} names an operation"))
    };
    let (q, r, qsub) = (operation("Q"), operation("R"), operation("Qsub"));
    for (name, clause) in [("Q", q), ("R", r), ("Qsub", qsub)] {
        assert_eq!(
            clause.declaring, order,
            "{name}'s operation is declared by the source-declared Order"
        );
    }
    assert_eq!(q.declaration, r.declaration, "Q and R share one operation");
    assert_eq!(
        q.declaration, qsub.declaration,
        "a clause on Sub::bump resolves to Order's bump"
    );
    assert_eq!(q.declaration.name(), "bump");
}

/// FR-569 "Declaration checks": a member may have any type a record field may
/// have, so an unbounded collection and a bounded one both assemble.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn an_unbounded_and_a_bounded_collection_member_both_assemble() {
    let graph = check(
        "\
object Z {
  tags: Set<Int[0, 9]>;
  loose: Sequence<Int[0, 9]>;
  few: Set<Int[0, 9]>[0, 3];
  population zs;
}
invariant X using P on Z at current { forall(z in zs : true) }
",
    );
    let package = graph
        .source_packages()
        .first()
        .expect("the unit's objects form a source package");
    let multiplicity = |name: &str| {
        package
            .records
            .iter()
            .find_map(|record| match record {
                qsl_semantics::model::domain_package::DomainPackageRecord::FieldMember(field)
                    if field.key.node.ends_with(&format!("/Z/{name}")) =>
                {
                    Some((field.multiplicity.lower, field.multiplicity.upper))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("member {name}"))
    };
    assert_eq!(multiplicity("tags"), (0, None));
    assert_eq!(multiplicity("loose"), (0, None));
    assert_eq!(multiplicity("few"), (0, Some(3)));
}

/// Two units of one package, `a` and `b`, each parsed, with `models` the
/// admitted selections of both.
fn assemble_two_units(
    a: &str,
    b: &str,
    models: Vec<qsl_semantics::model::intake::SelectedModel>,
) -> Result<PackageDeclarations, AssemblyRefusal> {
    use qsl_semantics::check::AssemblyLimits;
    let units: Vec<_> = [("a", a), ("b", b)]
        .iter()
        .map(|(identity, text)| {
            let source = format!("{HEADER}{text}\n");
            let parsed = qsl_cst::parse(
                qsl_foundation::SourceIdentity::new("test", *identity),
                "unit.native",
                source.as_bytes(),
                qsl_cst::Limits::default(),
            )
            .expect("S1 reads the unit");
            assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
            let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
            (parsed.source().reference().clone(), unit)
        })
        .collect();
    PackageDeclarations::assemble_units(
        crate::TEST_PACKAGE_NAME.to_owned(),
        units,
        SelectedModels::fixture_entries(models),
        Vec::new(),
        AssemblyLimits::default(),
        &quire_exact::Cancel::new(),
    )
}

/// The admitted selection of `package` as alias `M`.
fn selected_as_m(package: DomainPackage) -> qsl_semantics::model::intake::SelectedModel {
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        package,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    selected[0].clone()
}

/// A second domain package, `test/other`, whose `Total` is a different
/// declaration from fixture D's.
fn other_package() -> DomainPackage {
    use qsl_semantics::model::domain_package::{
        DomainPackageRecord, DomainPackageRef, ScalarTypeRecord,
    };
    DomainPackage::new(
        DomainPackageRef {
            identity: "test/other".to_owned(),
        },
        vec![DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: quire_checked::key::DeclarationKey {
                package: "test/other".to_owned(),
                node: "ix://test/other/Total".to_owned(),
            },
            domain: qsl_semantics::model::domain_package::ScalarValueDomain::Integer {
                lower: 0,
                upper: 99,
            },
        })],
    )
}

/// The node of the value type of source member `Owner.name`, as a package
/// type.
fn member_type_node(graph: &CheckedGraph, owner: &str, name: &str) -> String {
    use qsl_semantics::model::domain_package::{DomainPackageRecord, ValueTypeRef};
    graph
        .source_packages()
        .iter()
        .flat_map(|package| &package.records)
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(field)
                if field.key.node.ends_with(&format!("/{owner}/{name}")) =>
            {
                match &field.value_type {
                    ValueTypeRef::Package(key) => Some(key.node.clone()),
                    other => panic!("{owner}.{name} is not package-typed: {other:?}"),
                }
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("member {owner}.{name}"))
}

/// A model type a source member names resolves through the declaring unit's
/// own `model` aliases: unit `b` selects no `M`, so its `M::Total` names
/// nothing, although unit `a` selects one.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn a_model_type_resolves_only_through_the_declaring_units_own_aliases() {
    let selecting = "model M = \"test/orders\";\nobject A { t: M::Total; }\n";
    let bare = "object B { t: M::Total; }\n";
    let models = vec![selected_as_m(model_package(false))];
    // `a` alone: the member resolves.
    assemble_two_units(selecting, "object Plain { }\n", models.clone())
        .expect("a's own alias resolves");
    // `b` names `M::Total` without selecting `M`, whichever way they sort.
    for (a, b) in [(selecting, bare), (bare, selecting)] {
        let refused = assemble_two_units(a, b, models.clone())
            .expect_err("the unit that selects no M cannot name M::Total");
        let [error] = &refused.errors[..] else {
            panic!("one refusal: {refused:?}");
        };
        assert_eq!(
            (error.cause.code(), error.cause.catalog_code().cause()),
            MISSING,
            "{error:?}"
        );
    }
}

/// Two units that select different packages as `M` each bind their own.
#[trace("FR-569-AC-2", "TC-1408")]
#[test]
fn two_units_selecting_different_packages_as_m_each_bind_their_own() {
    let a = "model M = \"test/orders\";\nobject A { t: M::Total; }\n";
    let b = "model M = \"test/other\";\nobject B { t: M::Total; }\n";
    let models = vec![
        selected_as_m(model_package(false)),
        selected_as_m(other_package()),
    ];
    for (first, second) in [(a, b), (b, a)] {
        let declarations = assemble_two_units(first, second, models.clone())
            .expect("each unit's M::Total resolves");
        let graph = finish(declarations).expect("the units check");
        assert_eq!(member_type_node(&graph, "A", "t"), "ix://test/orders/Total");
        assert_eq!(member_type_node(&graph, "B", "t"), "ix://test/other/Total");
    }
}
