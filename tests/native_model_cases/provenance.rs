// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-043: false but constructor-valid loci across every admitted locus class.

use super::*;
use ix_trace_rs::trace;
use qsl_foundation::Source;
use quire_spec_language::formal_source::FormalSource;

#[derive(Clone, Copy, Debug)]
enum Target {
    Type,
    Field,
    Variant,
    Value,
    Scalar,
    Object,
    Operation,
}

fn change_locus(
    input: &mut Parts,
    target: Target,
    change: impl FnOnce(&ir::SourceSpan) -> ir::SourceSpan,
) -> ir::SourceSpan {
    let old = match target {
        Target::Type | Target::Field => {
            let ir::TypeDeclaration::Record { declaration } = input
                .environment
                .types()
                .iter()
                .find(|d| d.name().as_str() == "Unused")
                .unwrap()
            else {
                panic!("unused record")
            };
            if matches!(target, Target::Type) {
                declaration.source()
            } else {
                declaration.fields()[0].source()
            }
        }
        Target::Variant => {
            let ir::TypeDeclaration::Enum { declaration } = input
                .environment
                .types()
                .iter()
                .find(|d| d.name().as_str() == "Flag")
                .unwrap()
            else {
                panic!("Flag enum")
            };
            declaration.variants()[0].source()
        }
        Target::Value => input
            .environment
            .values()
            .iter()
            .find(|v| v.name().as_str() == "unused")
            .unwrap()
            .source(),
        Target::Scalar => &input.roles.scalars[0].source,
        Target::Object => &input.roles.objects[0].source,
        Target::Operation => &input.roles.operations[0].source,
    };
    let changed = change(old);
    match target {
        Target::Type | Target::Field => replace_record(input, "Unused", |r| {
            let fields = r
                .fields()
                .iter()
                .map(|f| {
                    ir::RecordFieldDeclaration::new(
                        f.name().clone(),
                        f.value_type().clone(),
                        if matches!(target, Target::Field) {
                            changed.clone()
                        } else {
                            f.source().clone()
                        },
                    )
                })
                .collect();
            ir::RecordDeclaration::new(
                r.name().clone(),
                if matches!(target, Target::Type) {
                    changed.clone()
                } else {
                    r.source().clone()
                },
                fields,
            )
            .unwrap()
        }),
        Target::Variant => {
            let mut types = input.environment.types().to_vec();
            let index = types
                .iter()
                .position(|d| d.name().as_str() == "Flag")
                .unwrap();
            let ir::TypeDeclaration::Enum { declaration } = &types[index] else {
                panic!("Flag enum")
            };
            let mut variants = declaration.variants().to_vec();
            variants[0] =
                ir::EnumVariantDeclaration::new(variants[0].name().clone(), changed.clone());
            types[index] = ir::TypeDeclaration::Enum {
                declaration: ir::EnumDeclaration::new(
                    declaration.name().clone(),
                    declaration.source().clone(),
                    variants,
                )
                .unwrap(),
            };
            input.environment = ir::DeclarationEnvironment::new(
                input.environment.owner().clone(),
                types,
                input.environment.values().to_vec(),
                Vec::new(),
            )
            .unwrap();
        }
        Target::Value => {
            let values = input
                .environment
                .values()
                .iter()
                .map(|v| {
                    ir::ValueDeclaration::new(
                        v.name().clone(),
                        v.kind(),
                        v.value_type().clone(),
                        if v.name().as_str() == "unused" {
                            changed.clone()
                        } else {
                            v.source().clone()
                        },
                    )
                })
                .collect();
            input.environment = ir::DeclarationEnvironment::new(
                input.environment.owner().clone(),
                input.environment.types().to_vec(),
                values,
                Vec::new(),
            )
            .unwrap();
        }
        Target::Scalar => input.roles.scalars[0].source = changed.clone(),
        Target::Object => input.roles.objects[0].source = changed.clone(),
        Target::Operation => input.roles.operations[0].source = changed.clone(),
    }
    changed
}

fn map_endpoints(
    span: &ir::SourceSpan,
    change: impl Fn(&ir::SourceLocation) -> ir::SourceLocation,
) -> ir::SourceSpan {
    ir::SourceSpan::new(change(span.start()), change(span.end()))
        .expect("adverse span must pass IR construction")
}

#[test]
#[trace("TC-043", "FR-015-AC-4")]
fn tc_043_all_locus_classes_reject_false_coordinates_and_foreign_identity() {
    for target in [
        Target::Type,
        Target::Field,
        Target::Variant,
        Target::Value,
        Target::Scalar,
        Target::Object,
        Target::Operation,
    ] {
        for mutation in 0..4 {
            let mut input = extra_inventory();
            // Prove this richer source-derived model is admitted before mutation.
            NativeModel::new(
                input.source.clone(),
                input.environment.clone(),
                input.roles.clone(),
                ModelLimits::default(),
            )
            .unwrap();
            let bad = change_locus(&mut input, target, |span| {
                map_endpoints(span, |p| {
                    let (source, line, column) = match mutation {
                        0 => (p.source().clone(), p.line() + 1, p.column()),
                        1 => (p.source().clone(), p.line(), p.column() + 1),
                        2 => (
                            ir::SourceIdentity::new(
                                ir::SourceDocumentId::new("ForeignSource").unwrap(),
                                p.source().revision(),
                            ),
                            p.line(),
                            p.column(),
                        ),
                        3 => (
                            ir::SourceIdentity::new(
                                p.source().document().clone(),
                                ir::SourceRevision::new(2).unwrap(),
                            ),
                            p.line(),
                            p.column(),
                        ),
                        _ => unreachable!(),
                    };
                    ir::SourceLocation::new(source, line, column, p.byte_offset()).unwrap()
                })
            });
            let error = refuse(input, Code::InvalidModelBinding);
            if !matches!(target, Target::Object) {
                assert_eq!(error.related.len(), 1, "{target:?}/{mutation}");
                assert_eq!(error.related[0].source, bad);
            }
        }
    }
}

#[test]
#[trace("TC-043", "FR-015-AC-4")]
fn tc_043_split_unicode_scalar_is_refused_for_each_locus_class() {
    for target in [
        Target::Type,
        Target::Field,
        Target::Variant,
        Target::Value,
        Target::Scalar,
        Target::Object,
        Target::Operation,
    ] {
        let mut input = extra_inventory();
        // FormalSource is a byte/locus interface, not a JSON reader. Append a
        // synthetic multibyte provenance region without changing existing loci.
        let text = format!("{}\né🦀", input.source.source().text());
        let native = Source::read(
            input.source.source().identity().clone(),
            "unicode-locus",
            text.as_bytes(),
            1_048_576,
        )
        .unwrap();
        input.source = FormalSource::new(native, input.source.identity().clone());
        NativeModel::new(
            input.source.clone(),
            input.environment.clone(),
            input.roles.clone(),
            ModelLimits::default(),
        )
        .unwrap();
        let start = text.len() - "é🦀".len();
        let line =
            u32::try_from(text[..start].bytes().filter(|b| *b == b'\n').count() + 1).unwrap();
        let identity = input.source.identity().clone();
        let bad = change_locus(&mut input, target, |_| {
            let point =
                ir::SourceLocation::new(identity, line, 1, u64::try_from(start + 1).unwrap())
                    .unwrap();
            ir::SourceSpan::new(point.clone(), point).unwrap()
        });
        assert!(!text.is_char_boundary(usize::try_from(bad.start().byte_offset()).unwrap()));
        refuse(input, Code::InvalidModelBinding);
    }
}
