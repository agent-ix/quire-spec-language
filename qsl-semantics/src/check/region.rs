// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-096: a check [`Location`] resolves to a [`SourceRegion`] of the unit
//! its declaration was read from.
//!
//! `Origin::Body{index}` starts at the body of function `index`, and
//! `Origin::Measure{index}` at its `decreases` measure. Each path step `i`
//! moves to child `i`, numbered as `Expression::children` numbers them, and
//! the region is the span the node reached carries (FR-091-AC-10) under the
//! unit's `RawSourceRef`. `Origin::TypeDeclaration{name}` names the span of
//! the declared type's name, when the FR-091 assembler read it from the
//! unit. A declaration with no form spans (one built by hand, or
//! synthesized for FR-151 dispatch), a type declared by hand, and
//! `Origin::Expression` have no region: no region of the unit names a
//! position in a tree not read from it. A `generated` occurrence (a node no source position names) is
//! recorded at the body or measure root of the least function declaration
//! that names its node (`lowering::enclosing_declarations`), so it resolves
//! here like any other body location.

use std::collections::BTreeMap;
use std::sync::Arc;

use qsl_forms::DeclarationSpans;
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::source_map::SourceMap;
use qsl_foundation::Span;

use super::{CheckCause, CheckRefusal, CheckedGraph, Location, Origin, PackageDeclarations};

/// The region `location` names, given each function's and each state
/// clause's form spans by declaration index.
fn resolve<'s>(
    source: &RawSourceRef,
    embedding: Option<&SourceMap>,
    spans: impl Fn(usize) -> Option<&'s DeclarationSpans>,
    clause_spans: impl Fn(usize) -> Option<&'s DeclarationSpans>,
    type_spans: &BTreeMap<String, Span>,
    attempt_span: impl Fn(usize, usize) -> Option<Span>,
    location: &Location,
) -> Option<SourceRegion> {
    let span = match &location.origin {
        Origin::Body { index, .. } => spans(*index)?.body.at(&location.path)?,
        Origin::StateClause { index, .. } => clause_spans(*index)?.body.at(&location.path)?,
        Origin::Measure { index, .. } => spans(*index)?.measure.as_ref()?.at(&location.path)?,
        Origin::TypeDeclaration { name } if location.path.is_empty() => *type_spans.get(name)?,
        Origin::ProtocolAttempt { protocol, attempt } if location.path.is_empty() => {
            attempt_span(*protocol, *attempt)?
        }
        Origin::TypeDeclaration { .. } | Origin::ProtocolAttempt { .. } | Origin::Expression => {
            return None
        }
    };
    region(source, embedding, span)
}

/// `span` as a region of `source`. For a body embedded in a document
/// (ADR-013 C-21), `embedding` maps it to the document's region, under the
/// document's own `RawSourceRef`, shifted by the body's offset in the
/// document -- but only when `embedding`'s own body is `source` itself; a
/// map for a different body is ignored, and `span` resolves under `source`
/// as if there were no embedding. A span the map splits into more than one
/// document region (a layout deletion) is FR-096's fourth no-region case:
/// no single document region names it.
fn region(
    source: &RawSourceRef,
    embedding: Option<&SourceMap>,
    span: Span,
) -> Option<SourceRegion> {
    if let Some(map) = embedding.filter(|map| map.body().reference() == source) {
        return match map.map_regions(map.body(), span)?.as_slice() {
            [region] => Some(region.clone()),
            _ => None,
        };
    }
    let start = u64::try_from(span.start).ok()?;
    let end = u64::try_from(span.end).ok()?;
    SourceRegion::new(source.clone(), start, end).ok()
}

impl PackageDeclarations {
    /// FR-096: the region of this unit that `location` names, or `None`
    /// for a position in a tree not read from it.
    pub fn region(&self, location: &Location) -> Option<SourceRegion> {
        resolve(
            &self.source,
            self.embedding.as_deref(),
            |index| self.functions.get(index)?.spans(),
            |index| Some(&self.state_clauses.get(index)?.spans),
            &self.declared_type_spans,
            |protocol, attempt| self.attempt_span(protocol, attempt),
            location,
        )
    }

    /// The declared name span of protocol `protocol`'s attempt `attempt`:
    /// the position `Origin::ProtocolAttempt` names, since an
    /// `operation_anchor`/`frame` node carries no position of its own.
    fn attempt_span(&self, protocol: usize, attempt: usize) -> Option<Span> {
        let declaration = self
            .protocol_attempts
            .get(protocol)?
            .get(attempt)?
            .declaration;
        Some(
            self.protocols
                .get(protocol)?
                .declarations
                .get(declaration)?
                .name
                .span,
        )
    }

    /// FR-096: the region of function `index`'s whole declaration form, or
    /// `None` when it was not read from this unit.
    pub fn declaration_region(&self, index: usize) -> Option<SourceRegion> {
        let spans = self.functions.get(index)?.spans()?;
        region(&self.source, self.embedding.as_deref(), spans.declaration)
    }
}

/// FR-096: the spans [`PackageDeclarations::region`] resolves a location
/// over, taken from the declarations before `check` consumes them, so a
/// check refusal's region resolves without keeping the declarations.
#[derive(Clone, Debug)]
pub struct DeclarationRegions {
    source: RawSourceRef,
    embedding: Option<Arc<SourceMap>>,
    spans: Vec<Option<DeclarationSpans>>,
    /// Each state clause's form spans, by its index in source order.
    clause_spans: Vec<DeclarationSpans>,
    type_spans: BTreeMap<String, Span>,
    /// Each protocol's own attempts' declared name spans,
    /// index-aligned with the package's own `protocols`/`protocol_attempts`.
    /// `pub(super)`: `check::mod`'s own pipeline copies it onto the final
    /// `CheckedGraph`, which needs the same table `Self::region` reads.
    pub(super) attempt_spans: Vec<Vec<Option<Span>>>,
}

impl DeclarationRegions {
    /// FR-096: the region `location` names, by the same rule as
    /// [`PackageDeclarations::region`] over the declarations taken.
    pub fn region(&self, location: &Location) -> Option<SourceRegion> {
        resolve(
            &self.source,
            self.embedding.as_deref(),
            |index| self.spans.get(index)?.as_ref(),
            |index| self.clause_spans.get(index),
            &self.type_spans,
            |protocol, attempt| *self.attempt_spans.get(protocol)?.get(attempt)?,
            location,
        )
    }

    /// FR-096: the region of function `index`'s whole declaration form, or
    /// `None` when it was not read from the unit.
    pub fn declaration_region(&self, index: usize) -> Option<SourceRegion> {
        let spans = self.spans.get(index)?.as_ref()?;
        region(&self.source, self.embedding.as_deref(), spans.declaration)
    }

    /// FR-096: the region a check refusal names. A stage limit a family
    /// `check` located carries its own region (a declaration's span, or the
    /// node `Typer`'s depth stop failed at); a `ProtocolAnchor` cause
    /// (FR-113) carries its own span directly, since a protocol has no
    /// expression tree for a `Location` path to walk; every other refusal
    /// is located by its `location`.
    pub fn refusal_region(&self, refusal: &CheckRefusal) -> Option<SourceRegion> {
        match &refusal.cause {
            CheckCause::ResourceExhausted(limit) if limit.region.is_some() => limit.region.clone(),
            CheckCause::ProtocolAnchor(cause) => {
                region(&self.source, self.embedding.as_deref(), cause.span())
            }
            _ => self.region(&refusal.location),
        }
    }
}

impl PackageDeclarations {
    /// The spans [`Self::region`] reads, and nothing else of the
    /// declarations.
    pub fn regions(&self) -> DeclarationRegions {
        DeclarationRegions {
            source: self.source.clone(),
            embedding: self.embedding.clone(),
            spans: self
                .functions
                .iter()
                .map(|function| function.spans().cloned())
                .collect(),
            clause_spans: self
                .state_clauses
                .iter()
                .map(|clause| clause.spans.clone())
                .collect(),
            type_spans: self.declared_type_spans.clone(),
            // SR-770 FND-007: `None` for an attempt whose declaration index
            // names no declaration of its protocol (a hand-built form, never
            // one S2 built), so its region is simply unresolved instead of
            // a panic in library code.
            attempt_spans: (0..self.protocols.len())
                .map(|protocol| {
                    (0..self.protocol_attempts.get(protocol).map_or(0, Vec::len))
                        .map(|attempt| self.attempt_span(protocol, attempt))
                        .collect()
                })
                .collect(),
        }
    }
}

impl CheckedGraph {
    /// FR-096: the region of the checked unit that `location` names, by the
    /// same rule as [`PackageDeclarations::region`], so a consumer holding
    /// only the checked package resolves an `Evaluation.location`.
    pub fn region(&self, location: &Location) -> Option<SourceRegion> {
        resolve(
            &self.source,
            self.embedding.as_deref(),
            |index| self.form_spans.get(index)?.as_ref(),
            |index| Some(&self.state_clauses.get(index)?.spans),
            &self.type_spans,
            |protocol, attempt| *self.attempt_spans.get(protocol)?.get(attempt)?,
            location,
        )
    }

    /// FR-096: the region of function `index`'s whole declaration form, or
    /// `None` when it was not read from this unit.
    pub fn declaration_region(&self, index: usize) -> Option<SourceRegion> {
        let spans = self.form_spans.get(index)?.as_ref()?;
        region(&self.source, self.embedding.as_deref(), spans.declaration)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_forms::{
        BinaryOperator, BuiltinType, DeclarationSpans, DeclaredClauseKind, Expression,
        ExpressionSpans, FunctionDeclaration, TypeForm,
    };
    use qsl_foundation::diagnostic::Code;
    use qsl_foundation::source::provenance::SourceRegion;
    use qsl_foundation::source_map::{Layout, Segment, SourceMap};
    use qsl_foundation::{Source, SourceIdentity, Span};

    use super::super::family::fixtures::{admitted_source, empty_scope, measure_resolved};
    use super::super::refusal::CheckingLimitKind;
    use super::super::{CheckCause, CheckingLimits, Location, Origin, PackageDeclarations};
    use super::Arc;

    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile \"ix:value\" as v;\n\
        function f using v(a: Boolean, b: Int[0, 10], c: Int[0, 10], d: Int[0, 10], \
        n: Int[0, 10]): Int[0, 20] pure decreases(n) { if a then b else c + d }\n";

    /// The span of the only occurrence of `text` in [`UNIT`] at or after
    /// `from`.
    fn find(text: &str, from: usize) -> Span {
        let start = from
            + UNIT[from..]
                .find(text)
                .expect("fixture text is in the unit");
        Span {
            start,
            end: start + text.len(),
        }
    }

    fn int_form() -> TypeForm {
        TypeForm::builtin(BuiltinType::Int, Span { start: 0, end: 0 })
            .with_bounds(vec!["0".into(), "10".into()])
    }

    fn name(text: &str) -> Box<Expression> {
        Box::new(Expression::Name(text.into()))
    }

    /// `f`, with the spans of its form read from [`UNIT`]: the body
    /// `if a then b else c + d` and the measure `n`.
    fn function_f() -> FunctionDeclaration {
        let body = Expression::If {
            condition: name("a"),
            then: name("b"),
            otherwise: Box::new(Expression::Binary {
                operator: BinaryOperator::Add,
                left: name("c"),
                right: name("d"),
            }),
        };
        let parameters = ["b", "c", "d", "n"]
            .into_iter()
            .map(|parameter| (parameter.to_owned(), int_form()))
            .chain([(
                "a".to_owned(),
                TypeForm::builtin(BuiltinType::Boolean, Span { start: 0, end: 0 }),
            )])
            .collect();
        let result = TypeForm::builtin(BuiltinType::Int, Span { start: 0, end: 0 })
            .with_bounds(vec!["0".into(), "20".into()]);

        let whole = find("if a then b else c + d", 0);
        let mut body_spans = ExpressionSpans::new(whole).unwrap();
        let root = body_spans.root();
        body_spans.push_child(root, find("a", whole.start)).unwrap();
        body_spans.push_child(root, find("b", whole.start)).unwrap();
        let add = body_spans
            .push_child(root, find("c + d", whole.start))
            .unwrap();
        body_spans.push_child(add, find("c", whole.start)).unwrap();
        body_spans.push_child(add, find("d", whole.start)).unwrap();
        let measure = find("(n)", 0);
        let spans = DeclarationSpans {
            declaration: Span {
                start: find("function", 0).start,
                end: whole.end + " }".len(),
            },
            body: body_spans,
            measure: Some(
                ExpressionSpans::new(Span {
                    start: measure.start + 1,
                    end: measure.end - 1,
                })
                .unwrap(),
            ),
        };
        FunctionDeclaration::new(
            "f",
            parameters,
            result,
            Some(Expression::Name("n".into())),
            body,
        )
        .with_spans(spans)
        .expect("the spans have the body's and the measure's shape")
    }

    fn unit() -> PackageDeclarations {
        let source = admitted_source(SourceIdentity::new("a", "u", "git", "1"), UNIT.as_bytes());
        PackageDeclarations {
            functions: vec![function_f()],
            ..PackageDeclarations::new(source)
        }
    }

    fn body(path: &[usize]) -> Location {
        Location {
            origin: Origin::Body {
                function: "f".into(),
                index: 0,
            },
            path: path.to_vec(),
        }
    }

    fn measure(path: &[usize]) -> Location {
        Location {
            origin: Origin::Measure {
                function: "f".into(),
                index: 0,
            },
            path: path.to_vec(),
        }
    }

    /// The node `path` reaches from `root`, one `Expression::children`
    /// step at a time: the node the resolver is meant to have located.
    fn node<'e>(root: &'e Expression, path: &[usize]) -> &'e Expression {
        path.iter().fold(root, |node, &index| {
            node.children()
                .get(index)
                .copied()
                .unwrap_or_else(|| panic!("{path:?} names a node"))
        })
    }

    /// The unit bytes a region names.
    fn text(region: &SourceRegion) -> &'static str {
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        &UNIT[start..end]
    }

    /// TC-426 step 2: each location follows its path to its own node, so
    /// the region's bytes are that expression's source text, under the
    /// unit's reference, from the declarations, from the regions taken from
    /// them and from the checked package alike.
    #[trace("TC-426", "FR-096-AC-1", "FR-091-AC-10")]
    #[test]
    fn a_check_location_resolves_to_the_source_text_of_its_node() {
        let declarations = unit();
        let reference = declarations.source.clone();
        let regions = declarations.regions();
        let cases = [
            (body(&[]), "if a then b else c + d"),
            (body(&[2]), "c + d"),
            (body(&[2, 1]), "d"),
            (body(&[0]), "a"),
            (measure(&[]), "n"),
        ];
        for (location, expected) in &cases {
            let region = declarations
                .region(location)
                .unwrap_or_else(|| panic!("{location:?} resolves"));
            assert_eq!(text(&region), *expected, "{location:?}");
            assert_eq!(region.source(), &reference);
            assert_eq!(regions.region(location), Some(region), "{location:?}");
        }
        // Every leaf the paths reach through `Expression::children` is a
        // name whose spelling is its region's text, so a span tree whose
        // children are out of the form's order fails here.
        let function = &declarations.functions[0];
        for path in [&[0][..], &[1], &[2, 0], &[2, 1]] {
            let Expression::Name(spelling) = node(&function.body, path) else {
                panic!("{path:?} reaches a name");
            };
            let region = declarations.region(&body(path)).unwrap();
            assert_eq!(text(&region), spelling, "{path:?}");
        }
        let Some(Expression::Name(measured)) = &function.measure else {
            panic!("the measure is a name");
        };
        assert_eq!(text(&declarations.region(&measure(&[])).unwrap()), measured);

        let declaration = declarations.declaration_region(0).unwrap();
        assert!(text(&declaration).starts_with("function f using v("));
        assert!(text(&declaration).ends_with("c + d }"));

        let checked = declarations
            .check(CheckingLimits::default())
            .expect("the unit checks");
        for (location, expected) in &cases {
            let region = checked
                .region(location)
                .unwrap_or_else(|| panic!("{location:?} resolves in the checked package"));
            assert_eq!(text(&region), *expected, "{location:?}");
            assert_eq!(region.source(), &reference);
        }
        assert_eq!(checked.declaration_region(0), Some(declaration));
    }

    /// TC-426 step 4 and FR-096's two no-region cases: a standalone
    /// expression, and a function not read from the unit (an FR-151
    /// synthesized one carries no form spans). A path naming no node, and
    /// an index naming no function, have none either.
    #[trace("TC-426", "FR-096-AC-1")]
    #[test]
    fn a_position_not_read_from_the_unit_has_no_region() {
        let mut declarations = unit();
        declarations.functions.push(FunctionDeclaration::clause(
            "synthesized",
            Vec::new(),
            TypeForm::builtin(BuiltinType::Boolean, Span { start: 0, end: 0 }),
            None,
            Expression::Boolean(true),
            DeclaredClauseKind::Precondition,
        ));
        let synthesized = Location {
            origin: Origin::Body {
                function: "synthesized".into(),
                index: 1,
            },
            path: Vec::new(),
        };
        let standalone = Location {
            origin: Origin::Expression,
            path: Vec::new(),
        };
        let unresolved = [
            synthesized,
            standalone,
            body(&[3]),
            body(&[2, 1, 0]),
            measure(&[0]),
            Location {
                origin: Origin::Body {
                    function: "f".into(),
                    index: 9,
                },
                path: Vec::new(),
            },
        ];
        let regions = declarations.regions();
        for location in &unresolved {
            assert_eq!(declarations.region(location), None, "{location:?}");
            assert_eq!(regions.region(location), None, "{location:?}");
        }
        assert_eq!(declarations.declaration_region(1), None);

        let checked = declarations
            .check(CheckingLimits::default())
            .expect("the unit checks");
        for location in &unresolved {
            assert_eq!(checked.region(location), None, "{location:?}");
        }
        assert_eq!(checked.declaration_region(1), None);
    }

    /// The `u.qsl` `Source` [`function_f`]'s unit reads as, used as a C-21
    /// embedding body: identity `"a"`/`"u"`/`"git"`/`"1"`, matching
    /// [`unit`]'s own `RawSourceRef` exactly.
    fn embedding_body() -> Source {
        Source::read(
            SourceIdentity::new("a", "u", "git", "1"),
            "u.qsl",
            UNIT.as_bytes(),
            UNIT.len() + 1,
        )
        .unwrap()
    }

    /// A one-segment C-21 map of the whole of [`UNIT`] at byte offset `k`
    /// of `document_text`, over `body`.
    fn embedding_map(document_text: &str, k: usize, body: Source) -> SourceMap {
        let original = Source::read(
            SourceIdentity::new("a", "doc", "git", "1"),
            "doc.md",
            document_text.as_bytes(),
            document_text.len() + 1,
        )
        .unwrap();
        SourceMap::verify(
            original,
            body,
            Span {
                start: k,
                end: k + UNIT.len(),
            },
            vec![Segment {
                body: Span {
                    start: 0,
                    end: UNIT.len(),
                },
                original: Span {
                    start: k,
                    end: k + UNIT.len(),
                },
            }],
            Layout::default(),
            1,
        )
        .expect("the body is the document's bytes at k")
    }

    /// TC-426 step 3 (ADR-013 C-21): a body embedded at byte offset `k` of
    /// a document with reference `d`, with no layout deletions, resolves
    /// the four locations under `d` to the same spans shifted by `k`, from
    /// the declarations, from the regions taken from them and from the
    /// checked package alike. Dropping the shift (or the document's
    /// reference) makes the byte-for-byte text comparison fail.
    #[trace("TC-426", "FR-096-AC-1")]
    #[test]
    fn an_embedded_body_resolves_its_locations_under_the_document_shifted() {
        let prefix = "# Notes\n\n```qsl\n";
        let document_text = format!("{prefix}{UNIT}```\n");
        let k = prefix.len();
        let map = embedding_map(&document_text, k, embedding_body());
        let document = map.original().reference().clone();
        let mut declarations = unit();
        declarations.embedding = Some(Arc::new(map));
        let standalone = unit();
        let regions = declarations.regions();
        let checked = declarations
            .clone()
            .check(CheckingLimits::default())
            .expect("the unit checks");
        let cases = [
            (body(&[]), "if a then b else c + d"),
            (body(&[2]), "c + d"),
            (body(&[2, 1]), "d"),
            (measure(&[]), "n"),
        ];
        let k64 = u64::try_from(k).expect("the fixture prefix fits u64");
        for (location, expected) in &cases {
            let plain = standalone.region(location).unwrap();
            let embedded = declarations.region(location).unwrap();
            assert_eq!(embedded.source(), &document, "{location:?}");
            assert_eq!(embedded.start(), plain.start() + k64, "{location:?}");
            assert_eq!(embedded.end(), plain.end() + k64, "{location:?}");
            let bytes = &document_text[usize::try_from(embedded.start()).unwrap()
                ..usize::try_from(embedded.end()).unwrap()];
            assert_eq!(bytes, *expected, "{location:?}");
            assert_eq!(regions.region(location), Some(embedded.clone()));
            assert_eq!(checked.region(location), Some(embedded));
        }
        let form = declarations.declaration_region(0).unwrap();
        assert_eq!(form.source(), &document);
        assert_eq!(
            usize::try_from(form.start()).unwrap(),
            k + UNIT.find("function").unwrap()
        );
        // The declaration span resolves the same way from the pre-check
        // regions and the checked package, not just from the declarations:
        // a resolver that only threads the embedding through one of the
        // three `declaration_region`s would still pass the four-location
        // cases above.
        assert_eq!(regions.declaration_region(0), Some(form.clone()));
        assert_eq!(checked.declaration_region(0), Some(form));
        // A location the unit does not read still resolves to none.
        assert_eq!(declarations.region(&body(&[3])), None);
    }

    /// SR-742 FND-001: a `SourceMap` embedding whose own body is a
    /// *different* source than the declarations' unit is ignored, not
    /// consulted. `region`'s identity check must compare the declarations'
    /// `source` against the map's own `map.body().reference()`, not hand
    /// the map its own body back (which always passes `map_regions`'s
    /// internal check and proves nothing about which source it belongs to).
    #[trace("FR-096-AC-1")]
    #[test]
    fn an_embedding_for_a_different_body_is_ignored() {
        let prefix = "# Notes\n\n```qsl\n";
        let document_text = format!("{prefix}{UNIT}```\n");
        let other_body = Source::read(
            SourceIdentity::new("a", "other-unit", "git", "1"),
            "other.qsl",
            UNIT.as_bytes(),
            UNIT.len() + 1,
        )
        .unwrap();
        let map = embedding_map(&document_text, prefix.len(), other_body);
        let mut declarations = unit();
        declarations.embedding = Some(Arc::new(map));
        let standalone = unit();
        let regions = declarations.regions();
        for (location, _) in [
            (body(&[]), ()),
            (body(&[2]), ()),
            (body(&[2, 1]), ()),
            (measure(&[]), ()),
        ] {
            let expected = standalone.region(&location);
            assert_eq!(declarations.region(&location), expected, "{location:?}");
            assert_eq!(regions.region(&location), expected, "{location:?}");
        }
        assert_eq!(
            declarations.declaration_region(0),
            standalone.declaration_region(0)
        );
    }

    /// SR-744 FND-001: FR-096's fourth no-region case. A span the C-21 map
    /// splits into more than one document region (discontiguous segments
    /// either side of an indentation strip and a CRLF normalization, as
    /// `qsl-foundation`'s own C-21 fixture uses) names no single region, so
    /// it resolves to `None` rather than the first of the split regions --
    /// unreachable from any production caller today (`qsl-source` builds
    /// only single-segment maps), but reachable directly through the
    /// embedding once one is attached.
    #[trace("FR-096-AC-1")]
    #[test]
    fn a_span_the_embedding_splits_has_no_region() {
        let original_text = "head\r\n  p <= 2\r\n\tq\r\nend";
        let body_text = "p <= 2\nq";
        let start = original_text.find("  p").unwrap();
        let a = original_text.find('p').unwrap();
        let b = original_text.find('q').unwrap();
        let line = body_text.find('\n').unwrap();
        let original = Source::read(
            SourceIdentity::new("a", "doc", "git", "1"),
            "doc.md",
            original_text.as_bytes(),
            original_text.len() + 1,
        )
        .unwrap();
        let body = Source::read(
            SourceIdentity::new("a", "u", "git", "1"),
            "u.qsl",
            body_text.as_bytes(),
            body_text.len() + 1,
        )
        .unwrap();
        let segment = |body_start: usize, original_start: usize, length: usize| Segment {
            body: Span {
                start: body_start,
                end: body_start + length,
            },
            original: Span {
                start: original_start,
                end: original_start + length,
            },
        };
        let map = SourceMap::verify(
            original,
            body,
            Span {
                start,
                end: original_text.find("end").unwrap(),
            },
            vec![
                segment(0, a, line),
                segment(line, a + line + 1, 1),
                segment(line + 1, b, 1),
            ],
            Layout {
                strip_indentation: true,
                normalize_crlf: true,
                drop_final_newline: true,
            },
            3,
        )
        .expect("the C-21 discontiguous fixture verifies");
        let whole = Span {
            start: 0,
            end: body_text.len(),
        };
        assert_eq!(
            map.map_regions(map.body(), whole)
                .map(|regions| regions.len()),
            Some(3),
            "the query span must actually split into more than one region"
        );
        let source = map.body().reference().clone();
        assert_eq!(super::region(&source, Some(&map), whole), None);
        // The single-segment sub-span still resolves to its own document
        // region: only the multi-region span is refused.
        assert!(super::region(
            &source,
            Some(&map),
            Span {
                start: 0,
                end: line
            }
        )
        .is_some());
    }

    /// FR-096 (SR-745 FND-003): each of nesting depth, node count,
    /// input bytes and work budget is a `CheckRefusal` with code
    /// `stage_limit_exceeded`, carrying its kind, bound and counter, located
    /// at a specific source text. Depth is located at the node whose entry
    /// failed the charge (`Typer`'s own per-node check); node count, input
    /// bytes and work budget are the family's own per-declaration precheck
    /// (`check_node_count`/`check_input_bytes`/`ValueFunctionFamily::check`'s
    /// own work charge), fired before `Typer` starts, and located at the
    /// whole declaration's span. `Typer`'s own package-wide node count and
    /// lowering's own work charge, which locate at a node rather than the
    /// declaration, are separate cases below
    /// (`a_typer_stop_reaches_the_package_wide_node_count`,
    /// `a_lowering_stop_is_located_by_its_location`).
    #[trace("TC-427", "FR-096-AC-4", "FR-096-AC-5", "FR-096-AC-11")]
    #[test]
    fn a_checking_limit_stop_is_located_at_its_node() {
        let metrics = measure_resolved(&empty_scope(), &unit().functions[0]);
        let declaration_text = &UNIT[UNIT.find("function").unwrap()..UNIT.rfind(" }").unwrap() + 2];
        for (limits, kind, code, bound, actual, expected_text) in [
            (
                CheckingLimits::default().with_input_bytes(metrics.input_bytes - 1),
                CheckingLimitKind::InputBytes,
                "input-bytes-exceeded",
                metrics.input_bytes - 1,
                u128::from(metrics.input_bytes),
                declaration_text,
            ),
            (
                CheckingLimits::default().with_work_budget(metrics.work_budget - 1),
                CheckingLimitKind::WorkBudget,
                "work-budget-exceeded",
                metrics.work_budget - 1,
                u128::from(metrics.work_budget),
                declaration_text,
            ),
            (
                CheckingLimits::new(2, 64).unwrap(),
                CheckingLimitKind::Nodes,
                "node-count-exceeded",
                2,
                7,
                declaration_text,
            ),
            (
                CheckingLimits::new(u64::MAX, 1).unwrap(),
                CheckingLimitKind::Depth,
                "nesting-depth-exceeded",
                1,
                2,
                "a",
            ),
        ] {
            let unit = unit();
            let regions = unit.regions();
            let refusals = unit.check(limits).expect_err("the limit stops checking");
            let [refusal] = refusals.as_slice() else {
                panic!("one refusal, got {refusals:?}");
            };
            let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
                panic!("a stage limit, got {refusal:?}");
            };
            assert_eq!(limit.kind, kind);
            assert_eq!(limit.limit, bound);
            assert_eq!(limit.actual, actual);
            assert_eq!(refusal.cause.code(), Code::StageLimitExceeded, "{kind:?}");
            assert_eq!(refusal.cause.cause(), Some(code), "{kind:?}");
            let region = regions
                .refusal_region(refusal)
                .expect("the position was read from the unit");
            assert_eq!(text(&region), expected_text, "{kind:?}");
        }
    }

    /// FR-096 (SR-745 FND-004): the second budget below is
    /// discovered, not a hard-coded offset. It is the smallest work budget
    /// past `declared` whose lowering stop locates at the `c + d` node
    /// (path `[2]`), found by scanning upward from `declared + 1` -- so a
    /// change to the lowering cost model that moves this budget shows up as
    /// a bound scan finding a different value, not as a silently-passing
    /// magic number.
    fn budget_located_at_c_plus_d(declared: u64) -> u64 {
        for budget in (declared + 1)..(declared + 1000) {
            let unit = unit();
            let regions = unit.regions();
            let Err(refusals) = unit.check(CheckingLimits::default().with_work_budget(budget))
            else {
                continue;
            };
            let [refusal] = refusals.as_slice() else {
                continue;
            };
            let Some(region) = regions.refusal_region(refusal) else {
                continue;
            };
            if text(&region) == "c + d" {
                return budget;
            }
        }
        panic!("no budget in range locates the lowering stop at c + d");
    }

    /// FR-096: a lowering stop (`CheckingLimits` work budget spent
    /// past the declaration's own charge) carries no region of its own
    /// (`StageLimitCause::region` is `None`) and is located by its
    /// `location`: the body root for the first lowering charge past the
    /// declaration's, and the `c + d` node (path `[2]`) for a later one.
    #[trace("TC-427", "FR-096-AC-16")]
    #[test]
    fn a_lowering_stop_is_located_by_its_location() {
        let declared = measure_resolved(&empty_scope(), &unit().functions[0]).work_budget;
        let c_plus_d = budget_located_at_c_plus_d(declared);
        for (budget, path, node_text) in [
            (declared, &[][..], "if a then b else c + d"),
            (c_plus_d, &[2][..], "c + d"),
        ] {
            let unit = unit();
            let regions = unit.regions();
            let refusals = unit
                .check(CheckingLimits::default().with_work_budget(budget))
                .expect_err("lowering spends past the budget");
            let [refusal] = refusals.as_slice() else {
                panic!("one refusal, got {refusals:?}");
            };
            let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
                panic!("a stage limit, got {refusal:?}");
            };
            assert_eq!(limit.kind, CheckingLimitKind::WorkBudget);
            assert_eq!(limit.region, None, "a lowering stop names no region");
            assert_eq!(refusal.location, body(path));
            assert_eq!(regions.refusal_region(refusal), regions.region(&body(path)));
            let region = regions.refusal_region(refusal).expect("a node of the unit");
            assert_eq!(text(&region), node_text);
        }
    }

    /// FR-096 (SR-746 FND-001): `Typer`'s package-wide node count
    /// (`check.rs`'s `enter`), not the family's own per-declaration preimage
    /// `check_node_count` precheck (which the previous case above
    /// exercises, and which review found this file
    /// had no test past). `g1` and `g2` each preimage-measure 3 nodes,
    /// individually under the bound, so each passes its own precheck; with
    /// the bound one past `g1`'s count, `Typer`'s package-wide counter,
    /// seeded from `g1`'s final total, crosses the bound two nodes into
    /// `g2`'s own body walk, at `not a` (`g2`'s inner `not`), not at `g1`'s
    /// or `g2`'s declaration span.
    #[trace("TC-427", "FR-096-AC-17")]
    #[test]
    fn a_typer_stop_reaches_the_package_wide_node_count() {
        const TWO_FUNCTIONS: &str = "language \"ix:native\" edition \"1-draft\";\n\
            profile \"ix:value\" as v;\n\
            function g1 using v(a: Boolean): Boolean pure { not not a }\n\
            function g2 using v(a: Boolean): Boolean pure { not not a }\n";

        fn find_in(text: &str, from: usize) -> Span {
            let start = from
                + TWO_FUNCTIONS[from..]
                    .find(text)
                    .expect("fixture text is in the unit");
            Span {
                start,
                end: start + text.len(),
            }
        }

        /// `not not a`: `Not(Not(Name("a")))`, 3 nodes, at `name`'s own
        /// occurrence of the shared body text starting at or after `from`.
        fn not_not_a(name: &str, from: usize) -> FunctionDeclaration {
            let whole = find_in("not not a", from);
            let mut body_spans = ExpressionSpans::new(whole).unwrap();
            let root = body_spans.root();
            let inner = body_spans
                .push_child(root, find_in("not a", whole.start))
                .unwrap();
            body_spans
                .push_child(inner, find_in("a", whole.start + "not ".len()))
                .unwrap();
            let declaration_start = find_in(&format!("function {name}"), 0).start;
            let spans = DeclarationSpans {
                declaration: Span {
                    start: declaration_start,
                    end: whole.end + " }".len(),
                },
                body: body_spans,
                measure: None,
            };
            let boolean = || TypeForm::builtin(BuiltinType::Boolean, Span { start: 0, end: 0 });
            FunctionDeclaration::new(
                name,
                vec![("a".to_owned(), boolean())],
                boolean(),
                None,
                Expression::Not(Box::new(Expression::Not(Box::new(Expression::Name(
                    "a".to_owned(),
                ))))),
            )
            .with_spans(spans)
            .expect("the spans match not not a's shape")
        }

        let g2_from = TWO_FUNCTIONS.find("function g2").unwrap();
        let g1_count = measure_resolved(&empty_scope(), &not_not_a("g1", 0)).node_count;
        let g2_count = measure_resolved(&empty_scope(), &not_not_a("g2", g2_from)).node_count;
        assert_eq!(
            (g1_count, g2_count),
            (3, 3),
            "sanity: both fixtures are 3 nodes each"
        );

        let source = admitted_source(
            SourceIdentity::new("a", "u", "git", "1"),
            TWO_FUNCTIONS.as_bytes(),
        );
        let declarations = PackageDeclarations {
            functions: vec![not_not_a("g1", 0), not_not_a("g2", g2_from)],
            ..PackageDeclarations::new(source)
        };
        let regions = declarations.regions();
        // One past g1's own count: g1 passes its precheck (3 <= 4) and
        // fully types; g2 also passes its own precheck (3 <= 4) in
        // isolation, but Typer's package-wide counter, seeded at g1's
        // final 3, crosses 4 on g2's second node.
        let refusals = declarations
            .check(CheckingLimits::new(g1_count + 1, 64).unwrap())
            .expect_err("g2's Typer walk crosses the package-wide node bound");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal, got {refusals:?}");
        };
        let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
            panic!("a stage limit, got {refusal:?}");
        };
        assert_eq!(limit.kind, CheckingLimitKind::Nodes);
        assert_eq!(limit.limit, g1_count + 1);
        assert_eq!(limit.actual, u128::from(g1_count + 2));
        assert_eq!(limit.region, None, "Typer's own node stop names no region");
        assert_eq!(
            refusal.location,
            Location {
                origin: Origin::Body {
                    function: "g2".into(),
                    index: 1,
                },
                path: vec![0],
            }
        );
        let region = regions
            .refusal_region(refusal)
            .expect("the position was read from the unit");
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        assert_eq!(
            &TWO_FUNCTIONS[start..end],
            "not a",
            "g2's inner not, not either declaration"
        );
    }
}
