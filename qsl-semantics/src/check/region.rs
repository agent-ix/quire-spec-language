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

use qsl_forms::DeclarationSpans;
use qsl_foundation::diagnostic::{LimitExceeded, Locus};
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::Span;

use super::{CheckCause, CheckRefusal, CheckedGraph, Location, Origin, PackageDeclarations};

/// The region `location` names, given each function's form spans by
/// declaration index.
fn resolve<'s>(
    source: &RawSourceRef,
    spans: impl Fn(usize) -> Option<&'s DeclarationSpans>,
    type_spans: &BTreeMap<String, Span>,
    location: &Location,
) -> Option<SourceRegion> {
    let span = match &location.origin {
        Origin::Body { index, .. } => spans(*index)?.body.at(&location.path)?,
        Origin::Measure { index, .. } => spans(*index)?.measure.as_ref()?.at(&location.path)?,
        Origin::TypeDeclaration { name } if location.path.is_empty() => *type_spans.get(name)?,
        Origin::TypeDeclaration { .. } | Origin::Expression => return None,
    };
    region(source, span)
}

/// `span` as a region of `source`.
fn region(source: &RawSourceRef, span: Span) -> Option<SourceRegion> {
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
            |index| self.functions.get(index)?.spans(),
            &self.declared_type_spans,
            location,
        )
    }

    /// FR-096: the region of function `index`'s whole declaration form, or
    /// `None` when it was not read from this unit.
    pub fn declaration_region(&self, index: usize) -> Option<SourceRegion> {
        let spans = self.functions.get(index)?.spans()?;
        region(&self.source, spans.declaration)
    }
}

/// FR-096: the spans [`PackageDeclarations::region`] resolves a location
/// over, taken from the declarations before `check` consumes them, so a
/// check refusal's region resolves without keeping the declarations.
#[derive(Clone, Debug)]
pub struct DeclarationRegions {
    source: RawSourceRef,
    spans: Vec<Option<DeclarationSpans>>,
    type_spans: BTreeMap<String, Span>,
}

impl DeclarationRegions {
    /// FR-096: the region `location` names, by the same rule as
    /// [`PackageDeclarations::region`] over the declarations taken.
    pub fn region(&self, location: &Location) -> Option<SourceRegion> {
        resolve(
            &self.source,
            |index| self.spans.get(index)?.as_ref(),
            &self.type_spans,
            location,
        )
    }

    /// FR-096: the region of function `index`'s whole declaration form, or
    /// `None` when it was not read from the unit.
    pub fn declaration_region(&self, index: usize) -> Option<SourceRegion> {
        let spans = self.spans.get(index)?.as_ref()?;
        region(&self.source, spans.declaration)
    }

    /// FR-096: the region a check refusal names. A stage limit a family
    /// `check` located carries its own region (a declaration's span, or the
    /// node `Typer`'s depth stop failed at); every other refusal is located
    /// by its `location`.
    pub fn refusal_region(&self, refusal: &CheckRefusal) -> Option<SourceRegion> {
        match &refusal.cause {
            CheckCause::ResourceExhausted(limit) if limit.region.is_some() => limit.region.clone(),
            _ => self.region(&refusal.location),
        }
    }

    /// FR-096: a `CheckingLimits` stop as T-4's [`LimitExceeded`], carrying
    /// the `Locus::Region` of [`Self::refusal_region`]: the node whose
    /// entry failed the charge for `Typer` and lowering, or the declaration
    /// charged for package checking. `None` when `refusal` is not a
    /// checking-limit stop. The locus is absent when the position resolves
    /// to no region.
    pub fn limit_exceeded(&self, refusal: &CheckRefusal) -> Option<LimitExceeded> {
        let CheckCause::ResourceExhausted(limit) = &refusal.cause else {
            return None;
        };
        Some(
            LimitExceeded::new(limit.kind.foundation_kind(), limit.limit, limit.actual)
                .at(self.refusal_region(refusal).map(Locus::Region)),
        )
    }
}

impl PackageDeclarations {
    /// The spans [`Self::region`] reads, and nothing else of the
    /// declarations.
    pub fn regions(&self) -> DeclarationRegions {
        DeclarationRegions {
            source: self.source.clone(),
            spans: self
                .functions
                .iter()
                .map(|function| function.spans().cloned())
                .collect(),
            type_spans: self.declared_type_spans.clone(),
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
            |index| self.form_spans.get(index)?.as_ref(),
            &self.type_spans,
            location,
        )
    }

    /// FR-096: the region of function `index`'s whole declaration form, or
    /// `None` when it was not read from this unit.
    pub fn declaration_region(&self, index: usize) -> Option<SourceRegion> {
        let spans = self.form_spans.get(index)?.as_ref()?;
        region(&self.source, spans.declaration)
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_forms::{
        BinaryOperator, BuiltinType, DeclarationSpans, DeclaredClauseKind, Expression,
        ExpressionSpans, FunctionDeclaration, TypeForm,
    };
    use qsl_foundation::diagnostic::{LimitKind, Locus};
    use qsl_foundation::source::provenance::SourceRegion;
    use qsl_foundation::{SourceIdentity, Span};

    use super::super::family::fixtures::{admitted_source, empty_scope, measure_resolved};
    use super::super::{CheckingLimits, Location, Origin, PackageDeclarations};

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

    /// FR-096, QSL-245: each `CheckingLimits` ceiling `Typer` reaches
    /// (package-wide node count, nesting depth) and each declaration-level
    /// ceiling (input bytes, work budget) surfaces as a T-4
    /// `LimitExceeded` with its kind, bound and counter, located by the
    /// `Locus::Region` of the node whose entry failed. A refusal that is
    /// no limit stop is none. The declaration-level input-bytes and work
    /// stops are located at the declaration.
    #[trace("TC-427", "FR-096-AC-4", "FR-096-AC-5", "FR-096-AC-11")]
    #[test]
    fn a_checking_limit_stop_is_a_limit_exceeded_located_at_its_node() {
        let metrics = measure_resolved(&empty_scope(), &unit().functions[0]);
        for (limits, kind, bound, actual) in [
            (
                CheckingLimits::default().with_input_bytes(metrics.input_bytes - 1),
                LimitKind::InputBytes,
                metrics.input_bytes - 1,
                u128::from(metrics.input_bytes),
            ),
            (
                CheckingLimits::default().with_work_budget(metrics.work_budget - 1),
                LimitKind::WorkBudget,
                metrics.work_budget - 1,
                u128::from(metrics.work_budget),
            ),
            (
                CheckingLimits::new(2, 64).unwrap(),
                LimitKind::NodeCount,
                2,
                7,
            ),
            (
                CheckingLimits::new(u64::MAX, 1).unwrap(),
                LimitKind::NestingDepth,
                1,
                2,
            ),
        ] {
            let unit = unit();
            let regions = unit.regions();
            let declaration = regions.declaration_region(0);
            let refusals = unit.check(limits).expect_err("the limit stops checking");
            let [refusal] = refusals.as_slice() else {
                panic!("one refusal, got {refusals:?}");
            };
            let exceeded = regions
                .limit_exceeded(refusal)
                .expect("a checking limit stop");
            assert_eq!(exceeded.kind(), kind);
            assert_eq!(exceeded.configured_bound(), bound);
            assert_eq!(exceeded.actual(), actual);
            let expected = regions
                .refusal_region(refusal)
                .expect("the position was read from the unit");
            assert!(!text(&expected).is_empty());
            if matches!(kind, LimitKind::InputBytes | LimitKind::WorkBudget) {
                // FR-096: a declaration-level limit is at the declaration.
                assert_eq!(Some(expected.clone()), declaration);
            }
            assert_eq!(exceeded.locus(), Some(&Locus::Region(expected)));
        }
    }
}
