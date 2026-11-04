// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035-AC-7: a spelling the recognizer reserves as unsupported cannot name
//! a function or predicate under `1-draft`; a longer identifier that starts
//! with one still can.
use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource};
use qsl_foundation::{SourceIdentity, Span};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n";
const RESERVED: [&str; 4] = ["helper", "rec", "cast", "Tuple"];
const NEAR: [&str; 4] = ["helperz", "recount", "caster", "TupleX"];

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("test", "test:reserved-declaration-names"),
        "reserved.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("valid UTF-8 within the source ceiling")
}

fn function(name: &str) -> String {
    format!("{HEADER}function {name} using v(x: Integer): Integer pure {{ x + 1 }}\n")
}

fn predicate(name: &str) -> String {
    format!("{HEADER}predicate {name} using v(x: Integer): Boolean {{ x = 1 }}\n")
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_reserved_spelling_cannot_name_a_function_or_predicate() {
    for name in RESERVED {
        for text in [function(name), predicate(name)] {
            let parsed = parse(&text);
            assert!(!parsed.is_admissible(), "{text}");
            let [refusal] = parsed.diagnostics() else {
                panic!("one refusal for {text}: {:?}", parsed.diagnostics());
            };
            assert_eq!(refusal.code, CompleteCode::InvalidSyntax, "{text}");
            assert_eq!(refusal.cause, CompleteCause::UnexpectedToken, "{text}");
            let start = text.find(&format!(" {name} using")).expect("the name") + 1;
            assert_eq!(
                refusal.byte_span(),
                Some(Span {
                    start,
                    end: start + name.len(),
                }),
                "{text}"
            );
            assert!(
                refusal.message.contains(&format!("`{name}`")),
                "{text}: {}",
                refusal.message
            );
        }
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_longer_identifier_starting_with_a_reserved_spelling_names_a_function() {
    for name in NEAR {
        for text in [function(name), predicate(name)] {
            let parsed = parse(&text);
            assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
        }
    }
}
