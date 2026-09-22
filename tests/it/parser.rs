// SPDX-License-Identifier: AGPL-3.0-or-later
//! Expression-grammar parsing into the native syntax tree.
use ix_trace_rs::trace;
use qsl_foundation::{Code, SourceIdentity, Span};
use quire_spec_language::syntax::{BinaryOp as B, ExprId, ExprKind as E};
use quire_spec_language::{format::format, parse, Limits, ParsedUnit};

fn document(expression: &str) -> String {
    format!("language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"test/model\" version \"1\" digest \"unresolved\";\ninvariant Test on M::Thing at current {{ {expression} }}\n")
}
fn read(bytes: &[u8], limits: Limits) -> Result<ParsedUnit, Box<qsl_foundation::Diagnostic>> {
    parse(
        SourceIdentity {
            identity: "test:source".into(),
            revision: "test:revision-7".into(),
        },
        "test.native",
        bytes,
        limits,
    )
}
fn unit(expression: &str) -> ParsedUnit {
    read(document(expression).as_bytes(), Limits::default()).unwrap()
}
fn root(unit: &ParsedUnit) -> &E {
    &unit.expression(unit.clauses()[0].expression).unwrap().kind
}
fn binary(unit: &ParsedUnit, id: ExprId, expected: B) -> (ExprId, ExprId) {
    let E::Binary { op, left, right } = unit.expression(id).unwrap().kind else {
        panic!("expected binary")
    };
    assert_eq!(op, expected);
    (left, right)
}

#[trace("TC-012", "FR-002-AC-2")]
#[test]
fn precedence_and_associativity() {
    let u = unit("a implies b or c and d = e + f * g");
    let (_, r) = binary(&u, u.clauses()[0].expression, B::Implies);
    let (_, r) = binary(&u, r, B::Or);
    let (_, r) = binary(&u, r, B::And);
    let (_, r) = binary(&u, r, B::Equal);
    let (_, r) = binary(&u, r, B::Add);
    binary(&u, r, B::Multiply);
    let u = unit("a implies b implies c");
    let (_, r) = binary(&u, u.clauses()[0].expression, B::Implies);
    binary(&u, r, B::Implies);
    let u = unit("a - b - c");
    let (l, _) = binary(&u, u.clauses()[0].expression, B::Subtract);
    binary(&u, l, B::Subtract);
    let u = unit("a < b and c < d");
    assert!(matches!(root(&u), E::Binary { op: B::And, .. }));
    for expression in ["a < b < c", "a = b != c", "a < b + c = d"] {
        assert_eq!(
            read(document(expression).as_bytes(), Limits::default())
                .unwrap_err()
                .code,
            Code::InvalidSyntax
        );
    }
}

#[trace("TC-012", "FR-002-AC-2")]
#[test]
fn complete_compound_words_do_not_change_historical_tokenization() {
    let parsed = unit("toward-zero = 0");
    let (left, _) = binary(&parsed, parsed.clauses()[0].expression, B::Equal);
    binary(&parsed, left, B::Subtract);

    for spelling in [
        "helper",
        "rec",
        "set",
        "bag",
        "Set",
        "Bag",
        "OrderedSet",
        "collect",
        "flatten",
        "cast",
        "tuple",
        "Tuple",
        "Decimal",
        "Rational",
        "allInstances",
    ] {
        let unsupported = read(document(spelling).as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(
            unsupported.code,
            Code::UnsupportedConstruct,
            "historical unsupported token {spelling} changed classification"
        );
    }
}

#[trace("TC-012", "FR-002-AC-5")]
#[test]
fn grouping_and_identifier_spans_are_original_bytes() {
    let expression = "((self.parent)).versionNumber + (2 * 3)";
    let u = unit(expression);
    assert_eq!(
        u.source()
            .slice(u.expression(u.clauses()[0].expression).unwrap().span),
        Some(expression)
    );
    assert!(u
        .expressions()
        .iter()
        .any(|node| matches!(node.kind, E::Group { .. })
            && u.source().slice(node.span) == Some("((self.parent))")));
}

#[trace("TC-013", "FR-003-AC-1", "FR-003-AC-2", "FR-003-AC-3")]
#[test]
fn all_admitted_constructs_roundtrip_with_comments() {
    for expression in [
        "let p = self.parent in if present(p) then deref(value(p)).n > 0 else true",
        "forall(x in self.items: exists(y in self.items: x = y))",
        "not reaches(self, self, parent)",
        "M::Color::Red = M::Color::Blue",
        "-7 rem 3 = -1 and 7 div 3 = 2",
        "result = pre(self.n)",
        "\"caf\\u00e9\\n\\uD83D\\uDE00\" = \"café\\n😀\"",
        "not not false",
        "(if true then 2 else 3) + (let x = 4 in x)",
        "true // retained café comment  \n and false",
    ] {
        let a = unit(expression);
        let text = format(&a).unwrap();
        let b = read(text.as_bytes(), Limits::default()).unwrap();
        assert_eq!(syntax_kinds(&a), syntax_kinds(&b));
        assert_eq!(format(&b).unwrap(), text);
        if expression.contains("//") {
            assert!(text.contains("// retained café comment  \n"));
        }
        if expression.starts_with('"') {
            // Literal escape spellings survive even when decoded values agree.
            assert!(text.contains(expression));
        }
    }
}

#[trace("TC-012")]
#[test]
fn declaration_forms_and_keyword_boundaries() {
    let text = document("trueValue = modelled and presentValue = iffy");
    let u = read(text.as_bytes(), Limits::default()).unwrap();
    assert_eq!(u.imports()[0].alias.value, "M");
    let text = text.replace(
        "invariant Test on M::Thing at current",
        "pre Test on M::Thing::doIt",
    ) + "post Done on M::Thing::doIt { result = pre(self.n) }";
    let u = read(text.as_bytes(), Limits::default()).unwrap();
    assert_eq!(u.clauses().len(), 2);
    let formatted = format(&u).unwrap();
    assert_eq!(
        read(formatted.as_bytes(), Limits::default())
            .unwrap()
            .clauses()
            .len(),
        2
    );
    assert!(read(
        document("true")
            .replace("invariant Test", "invariant if")
            .as_bytes(),
        Limits::default()
    )
    .is_err());
}

#[trace("TC-011")]
#[test]
fn unicode_crlf_and_checked_regions() {
    let text = document("\"café😀\" = \"café😀\"").replace('\n', "\r\n");
    let u = read(text.as_bytes(), Limits::default()).unwrap();
    for (byte, _) in text
        .char_indices()
        .chain(std::iter::once((text.len(), '\0')))
    {
        let p = u.source().position(byte).unwrap();
        let prefix = &text[..byte];
        assert_eq!(p.line, prefix.matches('\n').count() + 1);
        assert_eq!(
            p.column,
            prefix.rsplit('\n').next().unwrap().chars().count() + 1
        );
    }
    let split = text.find('é').unwrap() + 1;
    assert!(u.source().position(split).is_none());
    assert!(u.source().position(text.len() + 1).is_none());
    assert!(u.source().locate(Span { start: 2, end: 1 }).is_none());
    assert!(u
        .source()
        .slice(Span {
            start: split,
            end: split + 1
        })
        .is_none());
}

#[trace("TC-012", "FR-002-AC-3")]
#[test]
fn malformed_and_unsupported_are_distinct_and_located() {
    for expression in [
        "helper(x)",
        "always(true)",
        "[1,2]",
        "1 / 2",
        "1.25",
        "1e3",
        "set(x)",
        "collect(self.items)",
    ] {
        let text = document(expression);
        let e = read(text.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(e.code, Code::UnsupportedConstruct, "{expression}: {e:?}");
        assert_eq!(
            e.span.start.byte,
            text.find(expression).unwrap() + if expression == "1 / 2" { 2 } else { 0 }
        );
        assert!(e.span.end.byte > e.span.start.byte);
        assert_eq!(e.source.revision, "test:revision-7");
    }
    // A user-named call target is a declaration/call form this profile
    // excludes outright (the same concept as native_model/admission.rs's
    // pure-function check), not a real capability the parser recognizes but
    // this build lacks: InvalidPackage, not UnsupportedConstruct.
    {
        let expression = "customHelper(x)";
        let text = document(expression);
        let e = read(text.as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(e.code, Code::InvalidPackage, "{expression}: {e:?}");
        assert_eq!(e.span.start.byte, text.find(expression).unwrap());
        assert!(e.span.end.byte > e.span.start.byte);
        assert_eq!(e.source.revision, "test:revision-7");
    }
    for expression in [
        "helper(]",
        "(true",
        "true)",
        "01",
        "1e+",
        "\"\\uD800\"",
        "\"\\uDC00\"",
        "\"\\x\"",
        "\"\\é\"",
        "\"unterminated",
        "true @ false",
        "true ^ false",
        "self'",
        "0x7fc00001",
        "true\rfalse",
        "a < < b",
        "true false",
    ] {
        let e = read(document(expression).as_bytes(), Limits::default()).unwrap_err();
        assert_eq!(e.code, Code::InvalidSyntax, "{expression}: {e:?}");
        assert!(!e.is_incomplete());
    }
}

#[trace("TC-011", "FR-001-AC-3")]
#[test]
fn exact_header_versions_and_source_validation() {
    for (old, new, code) in [
        ("ix:native", "other", Code::UnknownLanguage),
        ("0-draft\";", "9\";", Code::UnknownEdition),
        ("state-finite/0-draft", "future", Code::UnknownProfile),
    ] {
        let text = document("true").replacen(old, new, 1);
        assert_eq!(
            read(text.as_bytes(), Limits::default()).unwrap_err().code,
            code
        );
    }
    let mut text = document("true").into_bytes();
    text.push(0xff);
    let error = read(&text, Limits::default()).unwrap_err();
    assert_eq!(error.code, Code::InvalidUtf8);
    assert_eq!(error.span.start.byte, text.len() - 1);
    assert_eq!(
        read(document("true //\0").as_bytes(), Limits::default())
            .unwrap_err()
            .code,
        Code::InvalidSyntax
    );
    assert!(read(
        ("\u{feff}".to_owned() + &document("true")).as_bytes(),
        Limits::default()
    )
    .is_err());
}

#[trace("TC-011", "TC-012", "FR-001-AC-4", "FR-002-AC-4")]
#[test]
fn resource_limits_never_become_boolean_results() {
    for limits in [
        Limits {
            source_bytes: 1,
            ..Limits::default()
        },
        Limits {
            tokens: 2,
            ..Limits::default()
        },
        Limits {
            nodes: 0,
            ..Limits::default()
        },
        Limits {
            nesting: 0,
            ..Limits::default()
        },
    ] {
        let error = read(document("true").as_bytes(), limits).unwrap_err();
        assert!(error.is_incomplete());
    }
    let error = read(
        document(&format!("{}true{}", "(".repeat(1000), ")".repeat(1000))).as_bytes(),
        Limits::default(),
    )
    .unwrap_err();
    assert!(error.is_incomplete());
    let expression = std::iter::repeat_n("true", 1000)
        .collect::<Vec<_>>()
        .join(" implies ");
    assert!(read(document(&expression).as_bytes(), Limits::default())
        .unwrap_err()
        .is_incomplete());
}

#[trace("TC-012")]
#[test]
fn long_flat_chains_parse_format_and_drop_on_a_bounded_stack() {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(|| {
            for expression in [
                std::iter::repeat_n("1", 20_000)
                    .collect::<Vec<_>>()
                    .join(" + "),
                "not ".repeat(20_000) + "true",
            ] {
                let a = unit(&expression);
                let formatted = format(&a).unwrap();
                let b = read(formatted.as_bytes(), Limits::default()).unwrap();
                assert_eq!(a.expressions().len(), b.expressions().len());
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[trace("TC-019", "FR-002-AC-6")]
#[test]
fn deterministic_malformed_corpus_does_not_panic() {
    let check = |bytes: &[u8]| match read(bytes, Limits::default()) {
        Ok(unit) => assert_eq!(unit.source().text().as_bytes(), bytes),
        Err(error) => {
            assert_eq!(error.source.identity, "test:source");
            assert_eq!(error.source.revision, "test:revision-7");
            assert!(error.span.start.byte <= error.span.end.byte);
            assert!(error.span.end.byte <= bytes.len());
            assert!(error.span.start.line > 0 && error.span.start.column > 0);
        }
    };
    let valid = document("let p = self.parent in present(p) implies deref(value(p)).n > 0");
    for end in 0..valid.len() {
        check(&valid.as_bytes()[..end]);
    }
    let alphabet = b"\x00\xff\r\n\\\"{}()[]<>=:/019ae ";
    for &a in alphabet {
        for &b in alphabet {
            for &c in alphabet {
                check(&[a, b, c]);
                let mut text = document("true").into_bytes();
                let at = text.len() - 3;
                text.splice(at..at, [a, b, c]);
                check(&text);
            }
        }
    }
}

fn syntax_kinds(unit: &ParsedUnit) -> Vec<E> {
    unit.expressions()
        .iter()
        .map(|expression| {
            let mut kind = expression.kind.clone();
            let reset = |value: &mut qsl_foundation::Spanned<String>| {
                value.span = Span { start: 0, end: 0 }
            };
            match &mut kind {
                E::Name(name)
                | E::Field { name, .. }
                | E::Let { name, .. }
                | E::Quantifier { name, .. } => reset(name),
                E::EnumValue {
                    model,
                    name,
                    variant,
                } => {
                    reset(model);
                    reset(name);
                    reset(variant);
                }
                E::Reaches { field, .. } => reset(field),
                _ => {}
            }
            kind
        })
        .collect()
}

#[trace("TC-012")]
#[test]
fn every_import_and_name_reference_keeps_its_exact_token_locus() {
    let text = document("let object = self in forall(item in object.items: item.color = M::Color::Red and reaches(item, object, parent))");
    let u = read(text.as_bytes(), Limits::default()).unwrap();
    let check = |name: &qsl_foundation::Spanned<String>| {
        assert_eq!(u.source().slice(name.span), Some(name.value.as_str()));
    };
    let import = &u.imports()[0];
    check(&import.alias);
    for literal in [&import.package, &import.version, &import.digest] {
        let raw = u.source().slice(literal.span).unwrap();
        assert_eq!(serde_json::from_str::<String>(raw).unwrap(), literal.value);
        assert!(raw.starts_with('"') && raw.ends_with('"'));
    }
    let clause = &u.clauses()[0];
    for name in [&clause.name, &clause.model, &clause.context] {
        check(name);
    }
    let mut names = 0;
    for expression in u.expressions() {
        match &expression.kind {
            E::Name(name)
            | E::Field { name, .. }
            | E::Let { name, .. }
            | E::Quantifier { name, .. } => {
                check(name);
                names += 1;
            }
            E::EnumValue {
                model,
                name,
                variant,
            } => {
                check(model);
                check(name);
                check(variant);
                names += 3;
            }
            E::Reaches { field, .. } => {
                check(field);
                names += 1;
            }
            _ => {}
        }
    }
    assert_eq!(names, 12);
    let text = text.replace(
        "invariant Test on M::Thing at current",
        "post Test on M::Thing::attempt",
    );
    let u = read(text.as_bytes(), Limits::default()).unwrap();
    let operation = u.clauses()[0].operation.as_ref().unwrap();
    assert_eq!(u.source().slice(operation.span), Some("attempt"));
}
