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
