// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public-API qualification of the native-formal-environment/1 binding profile.
use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::linking::{DeclarationKey, ResolutionTarget};
use quire_spec_language::{
    link, parse, ByteDigest, Code, Limits, LinkLimits, ParsedUnit, Phase, SourceIdentity,
};

fn symbol(name: &str) -> ir::SymbolName {
    ir::SymbolName::new(name).unwrap()
}

fn locus(revision: u64, offset: u32) -> ir::SourceSpan {
    let source = ir::SourceIdentity::new(
        ir::SourceDocumentId::new("formal-model").unwrap(),
        ir::SourceRevision::new(revision).unwrap(),
    );
    ir::SourceSpan::new(
        ir::SourceLocation::new(source.clone(), 1, offset + 1, u64::from(offset)).unwrap(),
        ir::SourceLocation::new(source, 1, offset + 2, u64::from(offset + 1)).unwrap(),
    )
    .unwrap()
}

fn record_type(name: &str) -> ir::ValueType {
    ir::ValueType::Record { name: symbol(name) }
}

fn environment(owner: &str, revision: u64, unused_bound: i64) -> ir::DeclarationEnvironment {
    let integer = ir::ValueType::integer(
        ir::IntegerType::new(
            ir::IntegerDomain::Signed,
            0,
            1000,
            ir::OverflowPolicy::Reject,
        )
        .unwrap(),
    );
    let record =
        |name: &str, offset: u32, fields: Vec<(&str, ir::ValueType)>| ir::TypeDeclaration::Record {
            declaration: ir::RecordDeclaration::new(
                symbol(name),
                locus(revision, offset),
                fields
                    .into_iter()
                    .enumerate()
                    .map(|(index, (name, ty))| {
                        ir::RecordFieldDeclaration::new(
                            symbol(name),
                            ty,
                            locus(revision, offset + u32::try_from(index).unwrap() + 1),
                        )
                    })
                    .collect(),
            )
            .unwrap(),
        };
    let types = vec![
        record(
            "BoundedCounter",
            0,
            vec![
                ("count", integer.clone()),
                ("child", record_type("Child")),
                ("optional", ir::ValueType::option(record_type("Child"))),
                (
                    "items",
                    ir::ValueType::collection(
                        ir::CollectionType::new(record_type("Child"), 3).unwrap(),
                    ),
                ),
            ],
        ),
        record("Child", 20, vec![("count", integer)]),
        record(
            "Unused",
            30,
            vec![(
                "unused",
                ir::ValueType::integer(
                    ir::IntegerType::new(
                        ir::IntegerDomain::Unsigned,
                        0,
                        unused_bound,
                        ir::OverflowPolicy::Reject,
                    )
                    .unwrap(),
                ),
            )],
        ),
        ir::TypeDeclaration::Enum {
            declaration: ir::EnumDeclaration::new(
                symbol("Color"),
                locus(revision, 40),
                vec![
                    ir::EnumVariantDeclaration::new(symbol("Red"), locus(revision, 41)),
                    ir::EnumVariantDeclaration::new(symbol("Blue"), locus(revision, 42)),
                ],
            )
            .unwrap(),
        },
    ];
    ir::DeclarationEnvironment::new(
        ir::RequirementRef::parse("example/model", owner, revision).unwrap(),
        types,
        vec![
            ir::ValueDeclaration::new(
                symbol("self"),
                ir::ValueDeclarationKind::State,
                record_type("BoundedCounter"),
                locus(revision, 50),
            ),
            ir::ValueDeclaration::new(
                symbol("outer"),
                ir::ValueDeclarationKind::Input,
                record_type("Child"),
                locus(revision, 51),
            ),
        ],
        vec![],
    )
    .unwrap()
}

fn canonical(env: &ir::DeclarationEnvironment) -> ir::CanonicalOutput {
    env.canonical_declaration_with_limit(ir::CanonicalProfile::V1, 1_048_576)
        .unwrap()
}

fn import(env: &ir::DeclarationEnvironment, alias: &str) -> String {
    format!(
        "model {alias} = \"{}\" version \"{}\" digest \"{}\";\n",
        env.owner().package(),
        env.owner().revision().get(),
        ByteDigest::of(canonical(env).bytes().as_slice())
    )
}

fn clause(name: &str, expression: &str) -> String {
    format!("invariant {name} on M::BoundedCounter at current {{ {expression} }}\n")
}

fn document(imports: &str, clauses: &str) -> String {
    format!("language \"ix:native\" edition \"0-draft\";\r\nprofile \"state-finite/0-draft\";\r\n// café 😀\r\n{imports}{clauses}")
}

fn read(text: &str) -> ParsedUnit {
    parse(
        SourceIdentity {
            identity: "test:native".into(),
            revision: "opaque:rev-7".into(),
        },
        "link.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap()
}

fn unit(env: &ir::DeclarationEnvironment, expression: &str) -> ParsedUnit {
    read(&document(&import(env, "M"), &clause("P", expression)))
}

#[trace("TC-020", "TC-030", "FR-005-AC-1", "FR-013-AC-1", "FR-013-AC-2")]
#[trace("FR-017-AC-3")]
#[test]
fn exact_source_owner_field_and_digest_domains() {
    let envs = [environment("FR-001", 1, 10)];
    let original = unit(&envs[0], "self.count >= 0");
    let source = original.source().clone();
    let linked = link(original, &envs, LinkLimits::default()).unwrap();
    assert_eq!(linked.unit().source().text(), source.text());
    assert_eq!(linked.unit().source().identity(), source.identity());
    assert_eq!(linked.unit().source().digest(), source.digest());
    assert!(std::ptr::eq(linked.models()[0].environment(), &envs[0]));
    assert_eq!(
        linked.models()[0].digest(),
        ByteDigest::of(canonical(&envs[0]).bytes().as_slice())
    );
    let c = &linked.clauses()[0];
    assert_eq!(c.model(), 0);
    assert_eq!(c.context().identity.owner, *envs[0].owner());
    assert_eq!(
        c.context().identity.key,
        DeclarationKey::Type(symbol("BoundedCounter"))
    );
    assert_eq!(c.context().source, locus(1, 0));
    let occurrences = c.occurrences();
    assert_eq!(occurrences.len(), 3);
    for (occurrence, spelling, key, offset) in [
        (
            &occurrences[0],
            "BoundedCounter",
            DeclarationKey::Type(symbol("BoundedCounter")),
            0,
        ),
        (
            &occurrences[1],
            "self",
            DeclarationKey::Value(symbol("self")),
            50,
        ),
        (
            &occurrences[2],
            "count",
            DeclarationKey::Field {
                record: symbol("BoundedCounter"),
                field: symbol("count"),
            },
            1,
        ),
    ] {
        assert_eq!(source.slice(occurrence.span), Some(spelling));
        let ResolutionTarget::Formal(location) = &occurrence.target else {
            panic!("formal locus expected")
        };
        assert_eq!(location.identity.owner, *envs[0].owner());
        assert_eq!(location.identity.key, key);
        assert_eq!(location.source, locus(1, offset));
    }
    assert_eq!(occurrences[0].expression, None);
    assert!(occurrences[1].expression.is_some());
    let exact = linked.models()[0].digest().to_string();
    for replacement in [
        format!("sha256:{}", canonical(&envs[0]).digest()),
        ByteDigest::of(canonical(&environment("FR-002", 1, 10)).bytes().as_slice()).to_string(),
    ] {
        assert_ne!(replacement, exact);
        let error = link(
            read(&source.text().replace(&exact, &replacement)),
            &envs,
            LinkLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, Code::StaleDependency);
        assert_eq!(error.phase, Phase::Link);
    }
}

#[trace("TC-021", "TC-023", "FR-005-AC-2", "FR-005-AC-4", "FR-017-AC-3")]
#[test]
fn missing_stale_unused_declaration_and_malformed_digest() {
    let original = environment("FR-001", 1, 10);
    let source = unit(&original, "self.count").source().clone();
    let missing = link(read(source.text()), &[], LinkLimits::default()).unwrap_err();
    assert_eq!(missing.code, Code::MissingImport);
    assert_eq!(
        missing.span.start.byte,
        source.text().find("\"example/model\"").unwrap()
    );
    for changed in [environment("FR-001", 2, 10), environment("FR-001", 1, 11)] {
        assert_eq!(
            link(read(source.text()), &[changed], LinkLimits::default())
                .unwrap_err()
                .code,
            Code::StaleDependency
        );
    }
    let envs = [original];
    assert_eq!(
        link(
            read(&source.text().replace("version \"1\"", "version \"01\"")),
            &envs,
            LinkLimits::default()
        )
        .unwrap_err()
        .code,
        Code::StaleDependency
    );
    let parsed = read(source.text());
    let raw = source.slice(parsed.imports()[0].digest.span).unwrap();
    let error = link(
        read(&source.text().replace(raw, "\"invalid\"")),
        &envs,
        LinkLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, Code::InvalidModelBinding);
    assert_eq!(error.span.start.byte, parsed.imports()[0].digest.span.start);
}

#[trace("TC-022", "TC-034", "FR-005-AC-3", "FR-013-AC-6", "FR-017-AC-3")]
#[test]
fn ambiguity_retains_all_sorted_loci_in_every_order() {
    let a = environment("FR-001", 1, 10);
    let b = environment("FR-002", 2, 10);
    let mut expected = None;
    for envs in [[a.clone(), b.clone()], [b.clone(), a.clone()]] {
        for imports in [
            import(&a, "M") + &import(&b, "M"),
            import(&b, "M") + &import(&a, "M"),
        ] {
            let text = document(&imports, &clause("P", "self.count"));
            let error = link(read(&text), &envs, LinkLimits::default()).unwrap_err();
            assert_eq!(error.code, Code::AmbiguousDeclaration);
            assert_eq!(
                &text[error.span.start.byte..error.span.end.byte],
                "BoundedCounter"
            );
            assert_eq!(error.related.len(), 2);
            assert_eq!(error.related[0].identity.owner, *a.owner());
            assert_eq!(error.related[1].identity.owner, *b.owner());
            assert_eq!(error.related[0].source, locus(1, 0));
            assert_eq!(error.related[1].source, locus(2, 0));
            if let Some(prior) = &expected {
                assert_eq!(&error.related, prior);
            }
            expected = Some(error.related);
        }
    }
    let error = link(
        unit(&a, "true"),
        &[a.clone(), a.clone()],
        LinkLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, Code::AmbiguousDeclaration);
    assert_eq!(error.related.len(), 2 * a.types().len());
    assert!(error.related.windows(2).all(|pair| pair[0] <= pair[1]));
}

#[trace("TC-024", "FR-005-AC-5", "FR-017-AC-3")]
#[test]
fn failed_import_permutations_never_reuse_successful_state() {
    let envs = [environment("FR-001", 1, 10), environment("FR-002", 2, 10)];
    let a = import(&envs[0], "M");
    let b = import(&envs[1], "N");
    for defective in [
        a.replace("example/model", "example/missing"),
        a.replace("version \"1\"", "version \"99\""),
        import(&envs[1], "M"),
    ] {
        for imports in [
            format!("{defective}{a}{b}"),
            format!("{a}{defective}{b}"),
            format!("{a}{b}{defective}"),
        ] {
            let healthy = || {
                link(
                    read(&document(&(a.clone() + &b), &clause("P", "self.count"))),
                    &envs,
                    LinkLimits::default(),
                )
                .unwrap()
            };
            assert_eq!(healthy().models().len(), 2);
            let error = link(
                read(&document(&imports, &clause("P", "self.count"))),
                &envs,
                LinkLimits::default(),
            )
            .unwrap_err();
            assert!(matches!(
                error.code,
                Code::MissingImport | Code::StaleDependency | Code::AmbiguousDeclaration
            ));
            assert_eq!(
                healthy().clauses()[0].context().identity.owner,
                *envs[0].owner()
            );
        }
    }
}

#[trace("TC-031", "FR-013-AC-3", "FR-017-AC-3")]
#[test]
fn lexical_scope_and_formal_shapes_preserve_exact_targets() {
    let envs = [environment("FR-001", 1, 10)];
    for expression in [
        "self.child.count",
        "value(self.optional).count",
        "pre(self.child).count",
        "(if true then self.child else outer).count",
        "let p = self.child in p.count",
        "forall(p in self.items: p.count > 0)",
        "exists(p in self.items: p.count > 0)",
        "M::Color::Red = M::Color::Blue",
        "let outer = outer in let outer = outer in outer.count",
    ] {
        let linked = link(unit(&envs[0], expression), &envs, LinkLimits::default()).unwrap();
        for occurrence in linked.clauses()[0].occurrences() {
            match &occurrence.target {
                ResolutionTarget::Formal(location) => {
                    assert_eq!(location.identity.owner, *envs[0].owner())
                }
                ResolutionTarget::Local(declaration) => {
                    assert!(declaration.start < occurrence.span.start);
                    assert_eq!(
                        linked.unit().source().slice(*declaration),
                        linked.unit().source().slice(occurrence.span)
                    );
                }
            }
        }
        if expression.contains("Red") {
            let variants: Vec<_> = linked.clauses()[0]
                .occurrences()
                .iter()
                .filter_map(|o| match &o.target {
                    ResolutionTarget::Formal(l)
                        if matches!(l.identity.key, DeclarationKey::Variant { .. }) =>
                    {
                        Some(l)
                    }
                    ResolutionTarget::Formal(_) | ResolutionTarget::Local(_) => None,
                })
                .collect();
            assert_eq!(variants.len(), 2);
            assert_eq!(
                variants[0].identity.key,
                DeclarationKey::Variant {
                    enumeration: symbol("Color"),
                    variant: symbol("Red")
                }
            );
            assert_eq!(variants[0].source, locus(1, 41));
        }
    }
    let text = "let outer = outer in let outer = outer in outer.count";
    let linked = link(unit(&envs[0], text), &envs, LinkLimits::default()).unwrap();
    let bindings: Vec<_> = linked.clauses()[0]
        .occurrences()
        .iter()
        .filter_map(|o| match o.target {
            ResolutionTarget::Local(span) => Some(span),
            ResolutionTarget::Formal(_) => None,
        })
        .collect();
    let base = linked.unit().source().text().find(text).unwrap();
    assert_eq!(
        bindings.iter().map(|s| s.start - base).collect::<Vec<_>>(),
        vec![4, 25]
    );
    for expression in [
        "missing",
        "self.absent",
        "M::Color::Green",
        "N::Color::Red",
        "let p = p in p",
        "(let p = self.child in p.count) + p.count",
        "forall(p in p: true)",
        "forall(p in self.items: true) and p.count > 0",
        "(if true then self else self.child).count",
        "(not self).count",
        "(-self).count",
        "(self + self).count",
        "value(self).count",
    ] {
        let error = link(unit(&envs[0], expression), &envs, LinkLimits::default()).unwrap_err();
        assert!(
            matches!(error.code, Code::MissingDeclaration | Code::MissingImport),
            "{expression}: {error}"
        );
    }
}

#[trace("TC-032", "FR-013-AC-4", "FR-017-AC-3")]
#[test]
fn unmapped_forms_refuse_while_linking_makes_no_type_judgment() {
    let envs = [environment("FR-001", 1, 10)];
    for expression in ["deref(self)", "reaches(self, self, child)"] {
        let parsed = unit(&envs[0], expression);
        let expected = parsed
            .expression(parsed.clauses()[0].expression)
            .unwrap()
            .span;
        let error = link(parsed, &envs, LinkLimits::default()).unwrap_err();
        assert_eq!(error.code, Code::IllTyped);
        assert_eq!(error.span.start.byte, expected.start);
        assert_eq!(error.span.end.byte, expected.end);
    }
    for kind in ["pre", "post"] {
        let text = document(
            &import(&envs[0], "M"),
            &format!("{kind} P on M::BoundedCounter::attempt {{ true }}"),
        );
        assert_eq!(
            link(read(&text), &envs, LinkLimits::default())
                .unwrap_err()
                .code,
            Code::InvalidModelBinding
        );
    }
    for expression in [
        "value(self.optional).count",
        "self.count",
        "pre(self.count)",
        "present(self.optional)",
        "size(self.items)",
    ] {
        let linked = link(unit(&envs[0], expression), &envs, LinkLimits::default()).unwrap();
        assert_eq!(linked.clauses().len(), 1);
        assert_eq!(
            linked.clauses()[0].context().identity.owner,
            *envs[0].owner()
        );
    }
}

#[trace("TC-033", "FR-013-AC-5", "FR-017-AC-3")]
#[test]
fn inclusive_lowered_and_zero_limits_are_enforced_independently() {
    let envs = [environment("FR-001", 1, 10), environment("FR-002", 2, 10)];
    let text = document(
        &(import(&envs[0], "M") + &import(&envs[1], "N")),
        &clause("P", "self.count >= 0"),
    );
    let bytes: Vec<_> = envs
        .iter()
        .map(|env| canonical(env).bytes().as_slice().len())
        .collect();
    let exact = LinkLimits {
        models: 2,
        imports: 2,
        clauses: 1,
        nodes: read(&text).expressions().len(),
        depth: 3,
        model_bytes: *bytes.iter().max().unwrap(),
        total_model_bytes: bytes.iter().sum(),
    };
    assert_eq!(link(read(&text), &envs, exact).unwrap().models().len(), 2);
    let setters: [fn(&mut LinkLimits) -> &mut usize; 7] = [
        |l| &mut l.models,
        |l| &mut l.imports,
        |l| &mut l.clauses,
        |l| &mut l.nodes,
        |l| &mut l.depth,
        |l| &mut l.model_bytes,
        |l| &mut l.total_model_bytes,
    ];
    for (dimension, field) in setters.iter().enumerate() {
        for zero in [false, true] {
            let mut limits = exact;
            *field(&mut limits) = if zero { 0 } else { *field(&mut limits) - 1 };
            let error = link(read(&text), &envs, limits).unwrap_err();
            assert_eq!(error.code, Code::ResourceExhausted, "dimension {dimension}");
            assert!(error.is_incomplete());
            if dimension >= 5 {
                assert_eq!(
                    error.upstream.unwrap().code,
                    ir::DiagnosticCode::CanonicalizationResourceExhausted
                );
            }
            assert_eq!(link(read(&text), &envs, exact).unwrap().clauses().len(), 1);
        }
        let mut limits = exact;
        *field(&mut limits) = usize::MAX;
        assert_eq!(link(read(&text), &envs, limits).unwrap().models().len(), 2);
    }
}

#[trace("TC-033", "FR-013-AC-5")]
#[test]
fn hard_count_and_traversal_ceilings_cannot_be_raised() {
    let env = environment("FR-001", 1, 10);
    let high = LinkLimits {
        models: usize::MAX,
        imports: usize::MAX,
        clauses: usize::MAX,
        nodes: usize::MAX,
        depth: usize::MAX,
        model_bytes: usize::MAX,
        total_model_bytes: usize::MAX,
    };
    assert_eq!(
        link(unit(&env, "true"), &vec![env.clone(); 65], high)
            .unwrap_err()
            .code,
        Code::ResourceExhausted
    );
    let imports: String = (0..65).map(|i| import(&env, &format!("M{i}"))).collect();
    assert_eq!(
        link(
            read(&document(&imports, &clause("P", "true"))),
            std::slice::from_ref(&env),
            high
        )
        .unwrap_err()
        .code,
        Code::ResourceExhausted
    );
    let clauses: String = (0..257).map(|i| clause(&format!("P{i}"), "true")).collect();
    assert_eq!(
        link(
            read(&document(&import(&env, "M"), &clauses)),
            std::slice::from_ref(&env),
            high
        )
        .unwrap_err()
        .code,
        Code::ResourceExhausted
    );
    for terms in [65, 5001] {
        let expression = std::iter::repeat_n("true", terms)
            .collect::<Vec<_>>()
            .join(" and ");
        let parsed = unit(&env, &expression);
        let error = link(parsed, std::slice::from_ref(&env), high).unwrap_err();
        assert_eq!(error.code, Code::ResourceExhausted);
        assert!(error.is_incomplete());
    }
    let envs = [env];
    assert_eq!(
        link(unit(&envs[0], "true"), &envs, high)
            .unwrap()
            .clauses()
            .len(),
        1
    );
}

#[trace("TC-031", "TC-034", "FR-013-AC-3", "FR-013-AC-6", "FR-017-AC-3")]
#[test]
fn explicit_context_and_neighboring_clauses_cannot_be_guessed() {
    let original = environment("FR-001", 1, 10);
    for values in [
        vec![],
        vec![ir::ValueDeclaration::new(
            symbol("self"),
            ir::ValueDeclarationKind::Input,
            record_type("BoundedCounter"),
            locus(1, 50),
        )],
        vec![ir::ValueDeclaration::new(
            symbol("self"),
            ir::ValueDeclarationKind::State,
            record_type("Child"),
            locus(1, 50),
        )],
    ] {
        let envs = [ir::DeclarationEnvironment::new(
            original.owner().clone(),
            original.types().to_vec(),
            values,
            vec![],
        )
        .unwrap()];
        assert_eq!(
            link(unit(&envs[0], "true"), &envs, LinkLimits::default())
                .unwrap_err()
                .code,
            Code::InvalidModelBinding
        );
    }
    let envs = [original, environment("FR-002", 2, 10)];
    for (name, expected) in [
        ("Color", Code::InvalidModelBinding),
        ("Absent", Code::MissingDeclaration),
    ] {
        let text = document(
            &import(&envs[0], "M"),
            &clause("P", "true").replace("BoundedCounter", name),
        );
        assert_eq!(
            link(read(&text), &envs, LinkLimits::default())
                .unwrap_err()
                .code,
            expected
        );
    }
    let duplicate = document(
        &import(&envs[0], "M"),
        &(clause("P", "true") + &clause("P", "true")),
    );
    assert_eq!(
        link(read(&duplicate), &envs, LinkLimits::default())
            .unwrap_err()
            .code,
        Code::InvalidModelBinding
    );
    let good = clause("Good", "true").replace("M::", "Unique::");
    let bad = clause("Ambiguous", "true");
    let imports = import(&envs[0], "M") + &import(&envs[1], "M") + &import(&envs[0], "Unique");
    for clauses in [good.clone() + &bad, bad.clone() + &good] {
        assert_eq!(
            link(
                read(&document(&imports, &good)),
                &envs,
                LinkLimits::default()
            )
            .unwrap()
            .clauses()
            .len(),
            1
        );
        let error = link(
            read(&document(&imports, &clauses)),
            &envs,
            LinkLimits::default(),
        )
        .unwrap_err();
        assert_eq!(error.code, Code::AmbiguousDeclaration);
        assert_eq!(error.related.len(), 2);
        assert_eq!(error.related[0].identity.owner, *envs[0].owner());
        assert_eq!(error.related[1].identity.owner, *envs[1].owner());
    }
}

fn padded_environment(owner: &str, emitted_bytes: usize) -> ir::DeclarationEnvironment {
    let make = |name: String| {
        ir::DeclarationEnvironment::new(
            ir::RequirementRef::parse("example/padded", owner, 1).unwrap(),
            vec![ir::TypeDeclaration::Record {
                declaration: ir::RecordDeclaration::new(
                    symbol("BoundedCounter"),
                    locus(1, 0),
                    vec![ir::RecordFieldDeclaration::new(
                        symbol(&name),
                        ir::ValueType::Boolean,
                        locus(1, 1),
                    )],
                )
                .unwrap(),
            }],
            vec![ir::ValueDeclaration::new(
                symbol("self"),
                ir::ValueDeclarationKind::State,
                record_type("BoundedCounter"),
                locus(1, 2),
            )],
            vec![],
        )
        .unwrap()
    };
    let overhead = canonical(&make("x".into())).bytes().as_slice().len() - 1;
    let result = make("x".repeat(emitted_bytes - overhead));
    assert_eq!(
        result
            .canonical_declaration_with_limit(ir::CanonicalProfile::V1, 2_097_152)
            .unwrap()
            .bytes()
            .as_slice()
            .len(),
        emitted_bytes
    );
    result
}

#[trace("TC-033", "FR-013-AC-5")]
#[test]
fn hard_canonical_byte_budgets_admit_equality_and_refuse_one_more() {
    let hard = LinkLimits::default();
    let raised = LinkLimits {
        model_bytes: usize::MAX,
        total_model_bytes: usize::MAX,
        ..hard
    };
    let envs: Vec<_> = (0..8)
        .map(|i| padded_environment(&format!("FR-{i:03}"), hard.model_bytes))
        .collect();
    let original = unit(&envs[0], "true").source().clone();
    let linked = link(read(original.text()), &envs, raised).unwrap();
    assert_eq!(linked.models()[0].environment().owner(), envs[0].owner());
    assert_eq!(linked.clauses().len(), 1);
    let oversized = padded_environment("FR-000", hard.model_bytes + 1);
    let error = link(read(original.text()), &[oversized], raised).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert_eq!(
        error.upstream.unwrap().code,
        ir::DiagnosticCode::CanonicalizationResourceExhausted
    );
    let mut aggregate = envs.clone();
    aggregate.push(padded_environment("FR-008", 1024));
    let error = link(read(original.text()), &aggregate, raised).unwrap_err();
    assert_eq!(error.code, Code::ResourceExhausted);
    assert!(error.is_incomplete());
    assert_eq!(
        link(read(original.text()), &envs, raised)
            .unwrap()
            .clauses()
            .len(),
        1
    );
}
