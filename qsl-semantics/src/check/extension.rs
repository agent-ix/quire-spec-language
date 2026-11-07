// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-354 "Admission" at S3: the `Extension` variant of the checked form
//! (ADR-012 §5.1 row S3) and its one schema-driven builder (§4.2).
//!
//! S2 hands over a selected extension's declaration form with its leading
//! keyword spelled. [`check_forms`] joins the keyword to the held production
//! it names and checks each clause value against the type the production's
//! typed-node schema entry names, through the core's own typer. The builder is
//! written once: its states are the production's clause positions, read from
//! the schema, so an extension adds catalog data and no Rust type.
//!
//! A clause's type is a core `type-ref`, read by the production a function
//! parameter's type uses. A type that names a node kind of the catalog is
//! refused `unsupported_construct`/`not-yet-implemented` (QSpec FR-467
//! "Members" names the node-kind alternative, and neither it nor FR-354 says
//! what source expression denotes another extension's node).

use qsl_forms::{
    parse_type_text, BuiltinType, ClauseKind, ExprNode, ExpressionForm, ExtensionClauseForm,
    ExtensionForm, TypeFormHead,
};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::Span;
use quire_checked::node::{CheckedLiteral, NodeKind};
use quire_exact::{FloatType, IeeeWidth, IntegerInterval, RoundingMode, Value, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::{Location, Origin};

use super::check::{Scope, Signatures, Typer};
use super::ir::CheckedBody;
use super::refusal::{CheckCause, CheckRefusal};
use super::type_form::resolve_type_form;
use crate::family::CheckContext;
use crate::library::extension::{HeldExtensions, Production};

/// Why a selected extension's form was refused at S3.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ExtensionCause {
    /// `ill_typed`/`type-mismatch`: a clause value is not of the type its
    /// typed-node schema entry names. Names the clause, its span, the
    /// expected and the actual type.
    TypeMismatch {
        /// The clause name.
        clause: String,
        /// The span of the clause value.
        span: Span,
        /// The schema's spelling of the clause's type.
        expected: String,
        /// The clause value's type, in the core's spelling.
        actual: String,
    },
    /// `ambiguous_declaration`/`ambiguous-name` (QSpec FR-469): the form's
    /// declared name is also a record type's, or an earlier form's.
    AmbiguousName {
        /// The declared name.
        name: String,
        /// The span of the form's declared name.
        span: Span,
    },
    /// `ill_typed`/`type-mismatch` from `DeclarationCause::Recursion`
    /// (QSpec FR-143 "Recursion rule"): the form's node-kind clauses reach a
    /// form again. Names the cycle in path order, first and last equal.
    Cycle {
        /// The declared names of the forms on the cycle.
        cycle: Vec<String>,
        /// The span of the whole form it is reported at.
        span: Span,
    },
    /// `unsupported_construct`/`declaration-form`: the form's leading keyword
    /// is the keyword of no held production.
    UnheldKeyword {
        /// The leading keyword.
        keyword: String,
        /// The span of the keyword.
        span: Span,
    },
    /// `invalid_syntax`/`unexpected-token`: a clause is absent from its
    /// position, out of the production's order, of an unknown name or
    /// repeated. S1 refuses these first; the builder holds its own
    /// clause-order refusal (ADR-012 §4.2).
    ClauseOrder {
        /// The clause written at the position, when one is.
        found: Option<String>,
        /// The clause the production expects there, when it expects one.
        expected: Option<String>,
        /// The span of the clause written, or of the whole form.
        span: Span,
    },
}

impl ExtensionCause {
    /// The refusal code.
    pub fn code(&self) -> Code {
        match self {
            Self::TypeMismatch { .. } | Self::Cycle { .. } => Code::IllTyped,
            Self::UnheldKeyword { .. } => Code::UnsupportedConstruct,
            Self::AmbiguousName { .. } => Code::AmbiguousDeclaration,
            Self::ClauseOrder { .. } => Code::InvalidSyntax,
        }
    }

    /// The closed cause tag.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::TypeMismatch { .. } | Self::Cycle { .. } => "type-mismatch",
            Self::AmbiguousName { .. } => "ambiguous-name",
            Self::UnheldKeyword { .. } => "declaration-form",
            Self::ClauseOrder { .. } => "unexpected-token",
        }
    }

    /// The span the refusal names.
    pub fn span(&self) -> Span {
        match self {
            Self::TypeMismatch { span, .. }
            | Self::Cycle { span, .. }
            | Self::AmbiguousName { span, .. }
            | Self::UnheldKeyword { span, .. }
            | Self::ClauseOrder { span, .. } => *span,
        }
    }
}

/// The `Origin::TypeDeclaration` name of the form at `index` among the unit's
/// extension forms: its `declaration` occurrence, spanning the whole form.
/// Spaced, so it names no declared type.
pub(crate) fn extension_site(index: usize) -> String {
    format!("extension {index}")
}

/// The `Origin::TypeDeclaration` name of clause `clause` of the form at
/// `index`: the expression tree of the clause value.
pub(crate) fn extension_clause_site(index: usize, clause: usize) -> String {
    format!("extension {index} clause {clause}")
}

/// The root location of [`extension_site`].
pub(crate) fn extension_location(index: usize) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: extension_site(index),
    })
}

/// The root location of [`extension_clause_site`].
pub(crate) fn extension_clause_location(index: usize, clause: usize) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: extension_clause_site(index, clause),
    })
}

/// One checked clause: its name and its checked value.
#[derive(Debug)]
pub(crate) struct CheckedClause {
    pub(crate) name: String,
    pub(crate) value: ClauseValue,
}

/// What a clause's value checked to.
#[derive(Debug)]
pub(crate) enum ClauseValue {
    /// An expression checked against a core type.
    Checked(Box<CheckedExpression>),
    /// A bare identifier naming the extension form at this index among the
    /// unit's extension forms (the clause's schema type is a node kind).
    Form(usize),
}

/// An expression a clause value checked to.
#[derive(Debug)]
pub(crate) struct CheckedExpression {
    pub(crate) expected: ValueType,
    /// The bare scalar type node the clause's subnode is typed at when the
    /// schema spells a scalar word the checked type does not distinguish
    /// (`Text`, checked as the native string).
    pub(crate) scalar: Option<&'static str>,
    pub(crate) body: CheckedBody,
    pub(crate) slot_names: Vec<String>,
}

/// One checked form of a selected extension: the definition and node kind of
/// its production and its clauses in the production's order.
#[derive(Debug)]
pub(crate) struct CheckedExtensionForm {
    pub(crate) definition: DefinitionRef,
    pub(crate) node_kind: String,
    pub(crate) name: String,
    pub(crate) clauses: Vec<CheckedClause>,
}

/// What the check reads besides the forms.
pub(crate) struct ExtensionInput<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) signatures: &'a Signatures,
    pub(crate) held: &'a HeldExtensions,
    pub(crate) limits: CheckingLimits,
    pub(crate) cancel: &'a quire_exact::Cancel,
}

/// The forms of the unit by declared name: the form's index and node kind.
type Declared = std::collections::BTreeMap<String, (usize, String)>;

/// The checked forms, each at its index among the unit's extension forms
/// (`None` where the form was refused), and the refusals in builder order.
pub(crate) struct Checked {
    pub(crate) forms: Vec<Option<CheckedExtensionForm>>,
    pub(crate) refusals: Vec<CheckRefusal>,
}

/// Check each of `forms`, charging `nodes`. The declared names of every form
/// are collected before any clause is resolved, so a node-kind clause may
/// name a form declared after it (FR-354 "Clause checking").
pub(crate) fn check_forms(
    forms: &[ExtensionForm],
    input: &ExtensionInput<'_>,
    cx: &mut CheckContext<'_, Scope>,
    nodes: &mut u64,
) -> Checked {
    let mut declared = Declared::new();
    for (index, form) in forms.iter().enumerate() {
        if let Some((_, production)) = production(input.held, &form.keyword.name) {
            declared
                .entry(form.name.name.clone())
                .or_insert((index, production.node_kind.clone()));
        }
    }
    let mut checked = Checked {
        forms: Vec::with_capacity(forms.len()),
        refusals: Vec::new(),
    };
    for (index, form) in forms.iter().enumerate() {
        match check_form(index, form, input, &declared, cx, nodes) {
            Ok(form) => checked.forms.push(Some(form)),
            Err(refusals) => {
                checked.forms.push(None);
                checked.refusals.extend(refusals);
            }
        }
    }
    refuse_cycles(forms, &mut checked);
    checked
}

/// Refuse every form that its node-kind clauses reach again, naming the
/// cycle through it in path order, and drop it from the checked forms.
fn refuse_cycles(forms: &[ExtensionForm], checked: &mut Checked) {
    let edges: Vec<Vec<usize>> = checked
        .forms
        .iter()
        .map(|form| {
            form.iter()
                .flat_map(|form| &form.clauses)
                .filter_map(|clause| match clause.value {
                    ClauseValue::Form(target) => Some(target),
                    ClauseValue::Checked(_) => None,
                })
                .collect()
        })
        .collect();
    for start in 0..edges.len() {
        // The shortest path from `start` back to itself, breadth first.
        let mut parent: Vec<Option<usize>> = vec![None; edges.len()];
        let mut queue = std::collections::VecDeque::from([start]);
        let mut closes = None;
        'search: while let Some(at) = queue.pop_front() {
            for &next in &edges[at] {
                if next == start {
                    closes = Some(at);
                    break 'search;
                }
                if parent[next].is_none() {
                    parent[next] = Some(at);
                    queue.push_back(next);
                }
            }
        }
        let Some(last) = closes else { continue };
        let mut path = vec![last];
        while let Some(&at) = path.last() {
            if at == start {
                break;
            }
            match parent[at] {
                Some(previous) => path.push(previous),
                None => break,
            }
        }
        path.reverse();
        let cycle: Vec<String> = path
            .iter()
            .chain(std::iter::once(&start))
            .map(|at| forms[*at].name.name.clone())
            .collect();
        checked.refusals.push(refusal(
            start,
            ExtensionCause::Cycle {
                cycle,
                span: forms[start].span,
            },
        ));
    }
    for (index, form) in checked.forms.iter_mut().enumerate() {
        if checked.refusals.iter().any(|refusal| {
            refusal.location == extension_location(index)
                && matches!(
                    refusal.cause,
                    CheckCause::Extension(ref cause) if matches!(**cause, ExtensionCause::Cycle { .. })
                )
        }) {
            *form = None;
        }
    }
}

/// What a clause's schema type spelling denotes.
enum ClauseType {
    /// A core type, with the bare scalar node a scalar word is typed at.
    Value(ValueType, Option<&'static str>),
    /// A node kind of a held extension.
    NodeKind(String),
    /// A spelling outside the v1 set.
    Unsupported,
}

/// The held production `keyword` names, with its extension's definition.
fn production<'h>(
    held: &'h HeldExtensions,
    keyword: &str,
) -> Option<(&'h DefinitionRef, &'h Production)> {
    held.iter().find_map(|extension| {
        extension
            .productions()
            .iter()
            .find(|production| production.keyword == keyword)
            .map(|production| (extension.definition(), production))
    })
}

fn refusal(index: usize, cause: ExtensionCause) -> CheckRefusal {
    CheckRefusal {
        location: extension_location(index),
        cause: CheckCause::Extension(Box::new(cause)),
    }
}

/// The builder's position: the production clause the next written clause is
/// checked as, or past the last.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Position {
    At(usize),
    Done,
}

/// The builder of ADR-012 §4.2, over the clause positions of one production.
struct Builder<'a, 'c, 'x> {
    index: usize,
    form: &'a ExtensionForm,
    production: &'a Production,
    input: &'a ExtensionInput<'a>,
    declared: &'a Declared,
    cx: &'c mut CheckContext<'x, Scope>,
    nodes: &'c mut u64,
    checked: Vec<CheckedClause>,
    refusals: Vec<CheckRefusal>,
}

impl Builder<'_, '_, '_> {
    /// `accept(state, clause)`: one explicit arm for each pair of a position
    /// present or past the last and a clause written or not.
    fn accept(
        &mut self,
        state: Position,
        clause: Option<(usize, &ExtensionClauseForm)>,
    ) -> Position {
        match (state, clause) {
            (Position::At(at), Some((written, clause))) => {
                let expected = self.production.clauses.get(at).map(|c| c.name.as_str());
                if expected != Some(clause.name.name.as_str()) {
                    self.refusals.push(refusal(
                        self.index,
                        ExtensionCause::ClauseOrder {
                            found: Some(clause.name.name.clone()),
                            expected: expected.map(str::to_owned),
                            span: clause.name.span,
                        },
                    ));
                    return state;
                }
                match self.check_clause(at, written, clause) {
                    Ok(checked) => self.checked.push(checked),
                    Err(refusal) => self.refusals.push(refusal),
                }
                if at + 1 < self.production.clauses.len() {
                    Position::At(at + 1)
                } else {
                    Position::Done
                }
            }
            (Position::At(at), None) => {
                self.refusals.push(refusal(
                    self.index,
                    ExtensionCause::ClauseOrder {
                        found: None,
                        expected: self.production.clauses.get(at).map(|c| c.name.clone()),
                        span: self.form.span,
                    },
                ));
                state
            }
            (Position::Done, Some((_, clause))) => {
                self.refusals.push(refusal(
                    self.index,
                    ExtensionCause::ClauseOrder {
                        found: Some(clause.name.name.clone()),
                        expected: None,
                        span: clause.name.span,
                    },
                ));
                state
            }
            (Position::Done, None) => state,
        }
    }

    /// A typer over the input's scope, charging `nodes`.
    fn typer<'t>(
        input: &'t ExtensionInput<'_>,
        nodes: &'t mut u64,
        cx: &'t mut CheckContext<'_, Scope>,
    ) -> Typer<'t> {
        Typer::new(
            input.scope,
            input.signatures,
            input.limits,
            nodes,
            ClauseKind::Body,
        )
        .with_cancel(Some(input.cancel))
        .with_context(cx.reborrow(input.scope))
    }

    /// Check clause `at` of the production against its schema type, through
    /// the core's own checker.
    fn check_clause(
        &mut self,
        at: usize,
        written: usize,
        clause: &ExtensionClauseForm,
    ) -> Result<CheckedClause, CheckRefusal> {
        let production_clause = self
            .production
            .clauses
            .get(at)
            .ok_or_else(|| self.order_fault(clause))?;
        let location = extension_clause_location(self.index, written);
        let span = root_span(&clause.value);
        let (expected, scalar) = match self.expected_type(&production_clause.type_ref, &location) {
            ClauseType::Value(expected, scalar) => (expected, scalar),
            ClauseType::NodeKind(node_kind) => {
                return self.node_kind_clause(clause, &node_kind, &location, span)
            }
            ClauseType::Unsupported => {
                let actual = self.actual_type(clause, &location);
                return Err(refusal(
                    self.index,
                    ExtensionCause::TypeMismatch {
                        clause: clause.name.name.clone(),
                        span,
                        expected: production_clause.type_ref.clone(),
                        actual,
                    },
                ));
            }
        };
        let literal_range = std::cell::Cell::new(None);
        let mut used = *self.nodes;
        let mut typer = Self::typer(self.input, &mut used, self.cx);
        typer.check_declared_type(&expected, &location)?;
        let checked = typer.check_as(&clause.value.expression, &expected, &location);
        let slot_names = typer.slot_names().to_vec();
        drop(typer);
        // The core admits an integer into a narrower `Int[..]` under a range
        // obligation; a literal's range is known, so a literal outside the
        // declared range is a type mismatch here.
        let checked = checked.and_then(|body| match literal_outside(&body, &expected) {
            Some(actual) => Err(CheckRefusal::ill_typed(
                &location,
                quire_exact::IllTypedCause::TypeMismatch,
            ))
            .inspect_err(|_| literal_range.set(Some(actual))),
            None => Ok(body),
        });
        match checked {
            Ok(body) => {
                *self.nodes = used;
                Ok(CheckedClause {
                    name: clause.name.name.clone(),
                    value: ClauseValue::Checked(Box::new(CheckedExpression {
                        expected,
                        scalar,
                        body,
                        slot_names,
                    })),
                })
            }
            Err(CheckRefusal {
                cause: CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                ..
            }) => {
                let actual = literal_range
                    .take()
                    .unwrap_or_else(|| self.actual_type(clause, &location));
                Err(refusal(
                    self.index,
                    ExtensionCause::TypeMismatch {
                        clause: clause.name.name.clone(),
                        span,
                        expected: production_clause.type_ref.clone(),
                        actual,
                    },
                ))
            }
            Err(other) => Err(other),
        }
    }

    /// The type the clause value has on its own, in the core's spelling.
    fn actual_type(&mut self, clause: &ExtensionClauseForm, location: &Location) -> String {
        let mut used = *self.nodes;
        let mut typer = Self::typer(self.input, &mut used, self.cx);
        match typer.infer(&clause.value.expression, None, location) {
            Ok(body) => spelling(body.root().value_type()),
            Err(_) => "unknown".to_owned(),
        }
    }

    /// What a `type-ref` spelling denotes.
    ///
    /// Read in this order (FR-354 "Clause checking"): a scalar word is that
    /// scalar and never a record or node-kind name; any other spelling is
    /// the core `TypeReference` production, of which only `Int[lo, hi]`
    /// (no rounding mode) and a `QualifiedName` are supported, the name being
    /// a node kind of a held extension or else a record type.
    fn expected_type(&self, type_ref: &str, location: &Location) -> ClauseType {
        if let Some((value_type, scalar)) = scalar_word(type_ref) {
            return ClauseType::Value(value_type, scalar);
        }
        // IR's reader matches `Rational` and `Decimal` as scalar words too;
        // the core has no unbounded type of either, so the widest bounds
        // stand for them and the subnode is typed at the bare scalar.
        let wide = match type_ref {
            "Rational" => Some((
                "Rational[-1000000000000000000, 1000000000000000000; 1, 1000000000000000000]",
                "rational",
            )),
            "Decimal" => Some(("Decimal[-1000000, 1000000; 0, 6; nearest-even]", "decimal")),
            _ => None,
        };
        if let Some((spelling, scalar)) = wide {
            let resolved = parse_type_text(spelling).ok().and_then(|form| {
                resolve_type_form(&self.input.scope.in_source(None), &form, location).ok()
            });
            return match resolved {
                Some(value_type) => ClauseType::Value(value_type, Some(scalar)),
                None => ClauseType::Unsupported,
            };
        }
        let Ok(form) = parse_type_text(type_ref) else {
            return ClauseType::Unsupported;
        };
        let supported = match &form.head {
            TypeFormHead::Builtin(BuiltinType::Int) => form.bounds.len() == 2,
            TypeFormHead::Name(_) => form.arguments.is_empty() && form.bounds.is_empty(),
            _ => false,
        };
        if !supported {
            return ClauseType::Unsupported;
        }
        if matches!(form.head, TypeFormHead::Name(_))
            && self
                .input
                .held
                .iter()
                .flat_map(|extension| extension.productions())
                .any(|production| production.node_kind == type_ref)
        {
            return ClauseType::NodeKind(type_ref.to_owned());
        }
        match resolve_type_form(&self.input.scope.in_source(None), &form, location) {
            Ok(resolved @ (ValueType::Int(_) | ValueType::Composite(_))) => {
                ClauseType::Value(resolved, None)
            }
            _ => ClauseType::Unsupported,
        }
    }

    /// A clause typed by the node kind `node_kind`: its value is a bare
    /// identifier, resolved by declared name among the unit's extension forms
    /// as a type name is.
    fn node_kind_clause(
        &mut self,
        clause: &ExtensionClauseForm,
        node_kind: &str,
        location: &Location,
        span: Span,
    ) -> Result<CheckedClause, CheckRefusal> {
        let name = match clause.value.expression.root().node() {
            ExprNode::Name(name) if !name.is_qualified() => name,
            _ => {
                let actual = self.actual_type(clause, location);
                return Err(refusal(
                    self.index,
                    ExtensionCause::TypeMismatch {
                        clause: clause.name.name.clone(),
                        span,
                        expected: node_kind.to_owned(),
                        actual,
                    },
                ));
            }
        };
        match self.declared.get(&name.name) {
            None => Err(CheckRefusal {
                location: location.clone(),
                cause: CheckCause::MissingName(name.name.clone()),
            }),
            Some((_, actual)) if actual != node_kind => Err(refusal(
                self.index,
                ExtensionCause::TypeMismatch {
                    clause: clause.name.name.clone(),
                    span,
                    expected: node_kind.to_owned(),
                    actual: actual.clone(),
                },
            )),
            Some((target, _)) => Ok(CheckedClause {
                name: clause.name.name.clone(),
                value: ClauseValue::Form(*target),
            }),
        }
    }

    /// The refusal for a production clause position the schema lacks, which
    /// the held definition's validation rules out.
    fn order_fault(&self, clause: &ExtensionClauseForm) -> CheckRefusal {
        refusal(
            self.index,
            ExtensionCause::ClauseOrder {
                found: Some(clause.name.name.clone()),
                expected: None,
                span: clause.name.span,
            },
        )
    }
}

/// The span of the whole clause value.
fn root_span(value: &ExpressionForm) -> Span {
    value
        .spans
        .span(value.spans.root())
        .unwrap_or(Span { start: 0, end: 0 })
}

/// A checked value type in the core's `type-ref` spelling.
fn spelling(value_type: &ValueType) -> String {
    match value_type {
        ValueType::Boolean => "Boolean".to_owned(),
        ValueType::Integer => "Integer".to_owned(),
        ValueType::Int(interval) => integer_spelling(interval),
        ValueType::Rational(_) => "Rational".to_owned(),
        ValueType::Decimal(_) => "Decimal".to_owned(),
        ValueType::Float(_) => "Float".to_owned(),
        ValueType::Quantity(_) | ValueType::CarriedQuantity(_) | ValueType::BoundedQuantity(_) => {
            "Quantity".to_owned()
        }
        ValueType::Text(_) => "Text".to_owned(),
        ValueType::String => "Text".to_owned(),
        ValueType::Uuid => "UUID".to_owned(),
        ValueType::Timestamp => "Timestamp".to_owned(),
        ValueType::Map(_) => "Map".to_owned(),
        ValueType::Enum(_) => "Enum".to_owned(),
        ValueType::Option(payload) => format!("Option<{}>", spelling(payload)),
        ValueType::Composite(_) => "Composite".to_owned(),
        ValueType::Collection(_) => "Collection".to_owned(),
        ValueType::Reference(_) => "Reference".to_owned(),
        ValueType::Population(_) => "Population".to_owned(),
    }
}

/// The checked type of a scalar word, matched lexically as Contract IR's
/// reader matches it, and the bare scalar node a word the checked type does
/// not distinguish is typed at. `Rational` and `Decimal` name no unbounded
/// core type to check against and are not read.
#[ix_trace_rs::string_edge]
fn scalar_word(word: &str) -> Option<(ValueType, Option<&'static str>)> {
    let float = |width| ValueType::Float(FloatType::new(width, RoundingMode::Exact));
    Some(match word {
        "Boolean" => (ValueType::Boolean, None),
        "Integer" => (ValueType::Integer, None),
        "Float32" => (float(IeeeWidth::Binary32), None),
        "Float64" => (float(IeeeWidth::Binary64), None),
        "Text" => (ValueType::String, Some("text")),
        "String" => (ValueType::String, None),
        "UUID" => (ValueType::Uuid, None),
        "Timestamp" => (ValueType::Timestamp, None),
        _ => return None,
    })
}

/// The spelling of the integer literal `body` is when its range lies outside
/// the declared integer type `expected`.
fn literal_outside(body: &CheckedBody, expected: &ValueType) -> Option<String> {
    let ValueType::Int(target) = expected else {
        return None;
    };
    let root = body.root();
    let NodeKind::Coerce(operand, _) = root.kind() else {
        return None;
    };
    let NodeKind::Literal(CheckedLiteral(Value::Integer(value))) = root.at(*operand).kind() else {
        return None;
    };
    let inside = *target.lower() <= *value && *value <= *target.upper();
    (!inside).then(|| format!("Int[{value}, {value}]"))
}

fn integer_spelling(interval: &IntegerInterval) -> String {
    format!("Int[{}, {}]", interval.lower(), interval.upper())
}

/// Check one form (ADR-012 §4.2): the builder reads each written clause in
/// order, then finishes. A form with any refusal emits no checked form.
fn check_form(
    index: usize,
    form: &ExtensionForm,
    input: &ExtensionInput<'_>,
    declared: &Declared,
    cx: &mut CheckContext<'_, Scope>,
    nodes: &mut u64,
) -> Result<CheckedExtensionForm, Vec<CheckRefusal>> {
    let Some((definition, production)) = production(input.held, &form.keyword.name) else {
        return Err(vec![refusal(
            index,
            ExtensionCause::UnheldKeyword {
                keyword: form.keyword.name.clone(),
                span: form.keyword.span,
            },
        )]);
    };
    // QSpec FR-469: one declared name is one declaration, whatever its form.
    let record_named = !crate::check::type_form::TypeNames::named_types(
        &input.scope.in_source(None),
        &form.name.name,
    )
    .is_empty();
    let earlier_form = declared
        .get(&form.name.name)
        .is_some_and(|(first, _)| *first != index);
    if record_named || earlier_form {
        return Err(vec![refusal(
            index,
            ExtensionCause::AmbiguousName {
                name: form.name.name.clone(),
                span: form.name.span,
            },
        )]);
    }
    let mut builder = Builder {
        index,
        form,
        production,
        input,
        declared,
        cx,
        nodes,
        checked: Vec::with_capacity(form.clauses.len()),
        refusals: Vec::new(),
    };
    let mut state = if production.clauses.is_empty() {
        Position::Done
    } else {
        Position::At(0)
    };
    for (written, clause) in form.clauses.iter().enumerate() {
        state = builder.accept(state, Some((written, clause)));
    }
    builder.accept(state, None);
    if !builder.refusals.is_empty() {
        return Err(builder.refusals);
    }
    Ok(CheckedExtensionForm {
        definition: definition.clone(),
        node_kind: production.node_kind.clone(),
        name: form.name.name.clone(),
        clauses: builder.checked,
    })
}
