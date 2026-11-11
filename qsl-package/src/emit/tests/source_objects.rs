// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-569-AC-11: a unit that declares a source object type
//! is refused at emission, once, at its first `object`, whichever other
//! declarations it holds.

use super::*;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile P = \"quire.model.complete/v1\";\n";

/// TC-1408 fixture S.
const S: &str = "\
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
invariant I using P on Order at current { self.total >= 0 }
pre Q using P on Order::bump { self.total < 9 }
post R using P on Order::bump { self.total > pre(self.total) }
body B using P on Sub::size() { 7 }
body C using P on Order::size() { 1 }
pre Qs using P on Order::size { self.total >= 0 }
invariant Sized using P on Order at current { self.size() >= 0 }
";

/// S, S-R and S-P: the refusal is `SourceObjectEmission` at `object Order`.
#[trace("FR-569-AC-11", "TC-1408")]
#[test]
fn fixtures_s_s_r_and_s_p_refuse_at_the_first_object() {
    let variants = [
        ("S", String::new()),
        (
            "S-R",
            "pre Qs2 using P on Sub::size { self.total >= 1 }\n".to_owned(),
        ),
        (
            "S-P",
            "invariant V4 using P on Order at current { forall(o in orders : deref(o).total >= 0) }\n"
                .to_owned(),
        ),
    ];
    for (name, extra) in variants {
        let text = format!("{HEADER}{S}{extra}");
        let packages =
            qsl_semantics::model::intake::package_input(std::iter::empty::<&[u8]>()).unwrap();
        let mut declarations =
            assemble_with_models(text.as_bytes(), &packages).expect("the unit assembles");
        let pending = declarations
            .link_dispatch(Default::default())
            .unwrap_or_else(|refusals| panic!("{name}: {refusals:?}"));
        let graph = declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{name}: {:?}", pending.report(refusals)));
        let package = CheckedPackage::link(graph);
        let at = text.find("object Order").unwrap();
        let refusal = emit_checked(&package).expect_err("emission refuses");
        let EmitRefusal::SourceObjectEmission { span } = &refusal else {
            panic!("{name}: {refusal:?}");
        };
        assert_eq!(
            (span.start, span.end),
            (at, at + text[at..].find("\n}\n").unwrap() + "\n}".len()),
            "{name}: the span is the first object declaration"
        );
        assert_eq!(
            (refusal.code(), refusal.cause()),
            (Code::UnsupportedConstruct, Some("not-yet-implemented")),
            "{name}"
        );
    }
}

mod orders_model {
    include!("../../../../qsl-semantics/tests/fixtures/orders_model.rs");
}

/// TC-1408 variant S-M: S with `model M = "test/orders";` selecting fixture
/// D's package and the protocol `Sell`, whose roles and `over` name the model
/// and the source `Order`. It checks, and emission refuses once at the first
/// `object`.
#[trace("FR-569-AC-11", "TC-1408")]
#[test]
fn fixture_s_m_refuses_at_the_first_object() {
    let text = format!(
        "{HEADER}profile Pp = \"quire.protocol.finite-global/v1\";\n\
         model M = \"test/orders\";\n{S}\
         protocol Sell using Pp over (view: M::Order) on origin {{\n\
           role w on M;\n\
           role Seller on Order;\n\
           run sequence Main {{\n\
             attempt A by Seller on Order::bump contracts [] as (b: Boolean) {{ true }};\n\
           }}\n\
           finish Done as (d: Boolean) {{ true }};\n\
         }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:s-m"),
        "program.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let selected = qsl_semantics::model::intake::SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        orders_model::model_package(false),
        qsl_semantics::model::accounting::ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the model admits");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "pkg".to_owned(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("S-M assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("{:?}", pending.report(refusals)));
    let package = CheckedPackage::link(graph);
    let at = text.find("object Order").unwrap();
    let refusal = emit_checked(&package).expect_err("emission refuses");
    let EmitRefusal::SourceObjectEmission { span } = &refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(span.start, at);
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}

#[path = "../../../../qsl-semantics/tests/support/tc1070.rs"]
mod tc1070_source;

/// TC-1407 fixture E-S and its domain package's input.
fn fixture_e_s() -> (String, qsl_semantics::model::intake::PackageInput) {
    let (text, document) = tc1070_source::source_node();
    let bytes = crate::emit::tests::wire_bytes(&document).expect("the document encodes");
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()])
        .expect("E-S's domain package admits");
    (text, packages)
}

/// TC-1407 step 0 and FR-569-AC-11: E-S checks (its protocol is over the
/// source-declared `Node`, whose field and result name `M`'s types) and S4
/// refuses it once, at `object Node`, writing no package.
#[trace("FR-569-AC-11", "FR-383-AC-67", "TC-1407")]
#[test]
fn fixture_e_s_checks_and_refuses_at_object_node() {
    let (text, packages) = fixture_e_s();
    let mut declarations = assemble_with_models(text.as_bytes(), &packages).expect("E-S assembles");
    let pending = declarations
        .link_dispatch(Default::default())
        .unwrap_or_else(|refusals| panic!("{refusals:?}"));
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("E-S checks: {:?}", pending.report(refusals)));
    let package = CheckedPackage::link(graph);
    let at = text.find("object Node").unwrap();
    let refusal = emit_checked(&package).expect_err("emission refuses");
    let EmitRefusal::SourceObjectEmission { span } = &refusal else {
        panic!("{refusal:?}");
    };
    assert_eq!(span.start, at, "the first object declaration");
    assert_eq!(
        (refusal.code(), refusal.cause()),
        (Code::UnsupportedConstruct, Some("not-yet-implemented"))
    );
}
