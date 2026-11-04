// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-035-AC-7: a spelling the recognizer reserves as unsupported cannot name
//! a native declaration or binder under `1-draft`, in any name position; a
//! longer identifier that starts with one still can, and a qualified
//! `member-name` position still admits the exact spelling.
use ix_trace_rs::trace;
use qsl_cst::{CompleteCause, CompleteCode, Limits, ParsedSource};
use qsl_foundation::{SourceIdentity, Span};

const LANGUAGE: &str = "language \"ix:native\" edition \"1-draft\";\n";
const PROFILE: &str = "profile v = \"quire.value.complete/v1\";\n";
const RESERVED: [&str; 4] = ["helper", "rec", "cast", "Tuple"];
const NEAR: [&str; 4] = ["helperz", "recount", "caster", "TupleX"];

/// Every native declaration and binder name position, one per family. `@`
/// marks the name; the text around it is a whole `1-draft` unit.
const POSITIONS: [(&str, &str); 17] = [
    (
        "function name",
        "{P}function @ using v(x: Integer): Integer pure { x + 1 }",
    ),
    (
        "predicate name",
        "{P}predicate @ using v(x: Integer): Boolean { x = 1 }",
    ),
    (
        "function `using` alias",
        "{P}function f using @(x: Integer): Integer pure { x + 1 }",
    ),
    (
        "predicate `using` alias",
        "{P}predicate p using @(x: Integer): Boolean { x = 1 }",
    ),
    (
        "parameter",
        "{P}function f using v(@: Integer): Integer pure { 1 }",
    ),
    (
        "`let` binder",
        "{P}function f using v(x: Integer): Integer pure { let @ = x in 1 }",
    ),
    ("record name", "{P}record @ { datum: Integer; }"),
    ("record field", "{P}record R { @: Integer; }"),
    ("enum name", "{P}enum @ { a, b }"),
    ("enum member", "{P}enum E { a, @ }"),
    ("tuple name", "{P}tuple @(Integer, Integer);"),
    ("union name", "{P}union @ { a, b }"),
    ("union member", "{P}union U { a, @(Integer) }"),
    ("type alias", "{P}type @ = Integer;"),
    (
        "profile alias",
        "profile @ = \"quire.value.complete/v1\";\nrecord R { datum: Integer; }",
    ),
    (
        "import alias",
        "{P}import \"acme/base\" as @;\nrecord R { datum: Integer; }",
    ),
    (
        "model alias",
        "{P}model @ = \"acme/model\";\nrecord R { datum: Integer; }",
    ),
];

fn unit(template: &str, name: &str) -> String {
    format!(
        "{LANGUAGE}{}\n",
        template.replace("{P}", PROFILE).replace('@', name)
    )
}

fn parse(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("test", "test:reserved-declaration-names"),
        "reserved.native",
        text.as_bytes(),
        Limits::default(),
    )
    .expect("valid UTF-8 within the source ceiling")
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_reserved_spelling_cannot_name_any_native_declaration_or_binder() {
    for (position, template) in POSITIONS {
        let at = LANGUAGE.len()
            + template
                .replace("{P}", PROFILE)
                .find('@')
                .expect("the name position");
        for name in RESERVED {
            let text = unit(template, name);
            let parsed = parse(&text);
            assert!(!parsed.is_admissible(), "{position}: {text}");
            let [refusal] = parsed.diagnostics() else {
                panic!(
                    "{position}: one refusal for {text}: {:?}",
                    parsed.diagnostics()
                );
            };
            assert_eq!(refusal.code, CompleteCode::InvalidSyntax, "{position}");
            assert_eq!(refusal.cause, CompleteCause::UnexpectedToken, "{position}");
            assert_eq!(
                refusal.byte_span(),
                Some(Span {
                    start: at,
                    end: at + name.len(),
                }),
                "{position}: {text}"
            );
            assert!(
                refusal.message.contains(&format!("`{name}`")),
                "{position}: {}",
                refusal.message
            );
        }
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_longer_identifier_starting_with_a_reserved_spelling_names_every_position() {
    for (position, template) in POSITIONS {
        for name in NEAR {
            let text = unit(template, name);
            let parsed = parse(&text);
            assert!(
                parsed.is_admissible(),
                "{position}: {text}: {:?}",
                parsed.diagnostics()
            );
        }
    }
}

#[trace("TC-113", "FR-035-AC-7")]
#[test]
fn a_qualified_member_name_still_admits_a_reserved_spelling() {
    for name in RESERVED {
        for declarations in [
            format!("model M = \"acme/model\";\nrecord R {{ datum: M::{name}; }}"),
            format!("function f using v(x: M::R): Integer pure {{ x.{name} }}"),
        ] {
            let text = format!("{LANGUAGE}{PROFILE}{declarations}\n");
            let parsed = parse(&text);
            assert!(parsed.is_admissible(), "{text}: {:?}", parsed.diagnostics());
        }
    }
}
