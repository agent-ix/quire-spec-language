// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Clock-typed positions" at assembly: which declarations of the
//! unit's own source write a clock-bearing type, and what each becomes.
//!
//! A **pure clock** type has no data part: `Clock`, an alias of a pure
//! clock type, and `Option` or a collection of one. A **clock-bearing** type
//! holds a clock somewhere: a pure clock type, or a record, tuple or union
//! with a clock-bearing component, or an alias of one. The assembler keeps
//! every pure clock type out of the value world:
//!
//! - an alias of a pure clock type has no node;
//! - a record, tuple or union keeps its data components only, a pure clock
//!   field or element dropped and a pure clock union payload left empty;
//! - a function or predicate whose parameter or result is clock-bearing is
//!   a clock-reading declaration, which has no node and which S3 expands at
//!   each call (`timed_model`);
//! - a state clause or operation body that writes a clock type inside an
//!   expression is held back for S3 to refuse.
//!
//! Every `Clock` the source writes in a type position reaches S3's facet
//! gate as a [`ClockTypePosition`], in source order.

use std::collections::{BTreeMap, BTreeSet};

use qsl_forms::{BuiltinType, DeclarationForm, FunctionDeclaration, TypeForm, TypeFormHead};
use qsl_foundation::Span;

use super::{body_type_forms, expression_type_forms, signature_type_forms};

/// Where a `Clock` type form is written (FR-230 "Timed facet gate").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockSite {
    /// A declared type position: a parameter, a result, a record field, a
    /// tuple element, a union payload, an alias target, or the argument of
    /// `Option` or a collection type in one of them.
    Declaration,
    /// A type form inside an expression (`convert<T>`, `allInstances<T>` and
    /// the named types of an accumulation), which no clock rule admits.
    Expression,
}

/// A `Clock` the unit's own source writes in a type position, with the
/// declaration that writes it (FR-230 "Timed facet gate").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockTypePosition {
    /// The name of the function, predicate, record, tuple, union, alias or
    /// clause whose type form writes `Clock`.
    pub owner: String,
    /// Where the `Clock` is written.
    pub site: ClockSite,
    /// The span of the `Clock` type form.
    pub span: Span,
}

/// How a clock-reading declaration reads one parameter or its result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockSlot {
    /// `Clock`, or an alias of it: one clock place.
    Clock,
    /// Another clock-bearing type: an optional clock, a clock collection,
    /// or a record, tuple or union with a clock component.
    Bearing,
    /// A data type.
    Data,
}

/// A function or predicate of the unit's own source whose parameter or
/// result is clock-bearing (FR-230 "Clock-reading declarations"). It has no
/// node in the checked package; S3 checks its body and expands each call.
#[derive(Clone, Debug)]
pub struct ClockReadingDeclaration {
    /// The declaration as S2 built it.
    pub function: FunctionDeclaration,
    /// How each parameter is read, in declaration order.
    pub parameters: Vec<ClockSlot>,
    /// How the result is read.
    pub result: ClockSlot,
}

/// What the assembler does with one form of the unit.
pub(super) enum Disposition {
    /// Leave the form out of the value world: an alias of a pure clock type,
    /// or a declaration S3 refuses whole.
    Drop,
    /// Keep the record, tuple or union with its pure clock components
    /// removed.
    DataPart,
    /// Keep the function or predicate as a clock-reading declaration.
    Reading,
}

/// The classification of one unit's forms.
pub(super) struct ClockForms {
    /// Each form's disposition, by form index; a form not listed is kept.
    pub(super) dispositions: BTreeMap<usize, Disposition>,
    /// Every `Clock` written in a type position, in source order.
    pub(super) positions: Vec<ClockTypePosition>,
    /// The declared aliases of a pure clock type.
    pub(super) pure_aliases: BTreeSet<String>,
    /// The declared type names that are clock-bearing.
    pub(super) bearing_types: BTreeSet<String>,
    /// The declared aliases of the clock type itself: an alias chain that
    /// ends in `Clock`.
    pub(super) clock_aliases: BTreeSet<String>,
}

impl ClockForms {
    /// Whether `form` is a pure clock type: it has no data part.
    pub(super) fn is_pure(&self, form: &TypeForm) -> bool {
        match &form.head {
            TypeFormHead::Builtin(BuiltinType::Clock) => true,
            TypeFormHead::Name(name) => self.pure_aliases.contains(&name.name),
            TypeFormHead::Builtin(BuiltinType::Option) | TypeFormHead::Collection(_) => {
                !form.arguments.is_empty() && form.arguments.iter().all(|arg| self.is_pure(arg))
            }
            _ => false,
        }
    }

    /// Whether `form` holds a clock anywhere.
    pub(super) fn is_bearing(&self, form: &TypeForm) -> bool {
        let mut stack = vec![form];
        while let Some(form) = stack.pop() {
            match &form.head {
                TypeFormHead::Builtin(BuiltinType::Clock) => return true,
                TypeFormHead::Name(name) if self.bearing_types.contains(&name.name) => return true,
                _ => {}
            }
            stack.extend(&form.arguments);
        }
        false
    }

    /// How a clock-reading declaration reads a slot of type `form`.
    pub(super) fn slot(&self, form: &TypeForm) -> ClockSlot {
        let clock = match &form.head {
            TypeFormHead::Builtin(BuiltinType::Clock) => true,
            TypeFormHead::Name(name) => self.clock_aliases.contains(&name.name),
            _ => false,
        };
        if clock {
            ClockSlot::Clock
        } else if self.is_bearing(form) {
            ClockSlot::Bearing
        } else {
            ClockSlot::Data
        }
    }
}

/// The type forms one declaration writes, with where they are written.
struct Written<'f> {
    owner: String,
    /// The type name it declares: an alias, record, tuple or union.
    declares: Option<&'f str>,
    signature: Vec<&'f TypeForm>,
    expressions: Vec<TypeForm>,
    kind: Kind<'f>,
}

enum Kind<'f> {
    Alias(&'f TypeForm),
    Composite,
    Function,
    Clause,
}

fn written(form: &DeclarationForm) -> Option<Written<'_>> {
    Some(match form {
        DeclarationForm::Function(function) => Written {
            owner: function.name.clone(),
            declares: None,
            signature: signature_type_forms(function),
            expressions: body_type_forms(function),
            kind: Kind::Function,
        },
        DeclarationForm::Alias(alias) => Written {
            owner: alias.name.name.clone(),
            declares: Some(alias.name.name.as_str()),
            signature: vec![&alias.target],
            expressions: Vec::new(),
            kind: Kind::Alias(&alias.target),
        },
        DeclarationForm::Record(record) => Written {
            owner: record.name.name.clone(),
            declares: Some(record.name.name.as_str()),
            signature: record.fields.iter().map(|field| &field.type_form).collect(),
            expressions: Vec::new(),
            kind: Kind::Composite,
        },
        DeclarationForm::Tuple(tuple) => Written {
            owner: tuple.name.name.clone(),
            declares: Some(tuple.name.name.as_str()),
            signature: tuple.elements.iter().collect(),
            expressions: Vec::new(),
            kind: Kind::Composite,
        },
        DeclarationForm::Union(union) => Written {
            owner: union.name.name.clone(),
            declares: Some(union.name.name.as_str()),
            signature: union
                .members
                .iter()
                .flat_map(|member| &member.payload)
                .collect(),
            expressions: Vec::new(),
            kind: Kind::Composite,
        },
        DeclarationForm::StateClause(clause) => Written {
            owner: clause.name.name.clone(),
            declares: None,
            signature: Vec::new(),
            expressions: expression_type_forms([&clause.body]),
            kind: Kind::Clause,
        },
        DeclarationForm::OperationBody(body) => Written {
            owner: body.label.name.clone(),
            declares: None,
            signature: Vec::new(),
            expressions: expression_type_forms([&body.body]),
            kind: Kind::Clause,
        },
        _ => return None,
    })
}

/// The span of every `Clock` written in `forms`, in source order.
fn clocks<'f>(forms: impl IntoIterator<Item = &'f TypeForm>) -> Vec<Span> {
    let mut found = Vec::new();
    let mut stack: Vec<&TypeForm> = forms.into_iter().collect();
    while let Some(form) = stack.pop() {
        if form.head == TypeFormHead::Builtin(BuiltinType::Clock) {
            found.push(form.span);
        }
        stack.extend(&form.arguments);
    }
    found.sort_by_key(|span| span.start);
    found
}

/// Classify the clock-bearing declarations among `forms`.
pub(super) fn classify(forms: &[qsl_forms::ParsedForm]) -> ClockForms {
    let read: Vec<Option<Written<'_>>> = forms.iter().map(|form| written(form.form())).collect();
    let mut classified = ClockForms {
        dispositions: BTreeMap::new(),
        positions: Vec::new(),
        pure_aliases: BTreeSet::new(),
        bearing_types: BTreeSet::new(),
        clock_aliases: BTreeSet::new(),
    };
    // The alias and composite names that are pure or bearing, to a fixed
    // point: each pass adds at least one name or ends, so it runs at most
    // once per declaration plus one.
    loop {
        let mut changed = false;
        for entry in read.iter().flatten() {
            let Some(declared) = entry.declares else {
                continue;
            };
            if let Kind::Alias(target) = entry.kind {
                if !classified.pure_aliases.contains(declared) && classified.is_pure(target) {
                    classified.pure_aliases.insert(declared.to_owned());
                    changed = true;
                }
                let clock = match &target.head {
                    TypeFormHead::Builtin(BuiltinType::Clock) => true,
                    TypeFormHead::Name(name) => classified.clock_aliases.contains(&name.name),
                    _ => false,
                };
                if clock && classified.clock_aliases.insert(declared.to_owned()) {
                    changed = true;
                }
            }
            if !classified.bearing_types.contains(declared)
                && entry
                    .signature
                    .iter()
                    .any(|form| classified.is_bearing(form))
            {
                classified.bearing_types.insert(declared.to_owned());
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for (index, entry) in read.iter().enumerate() {
        let Some(entry) = entry else {
            continue;
        };
        let declared = clocks(entry.signature.iter().copied());
        // An expression's clock-bearing type form is located at each
        // `Clock` it writes, or at the form itself when it names a
        // clock-bearing declaration.
        let inside: Vec<Span> = entry
            .expressions
            .iter()
            .filter(|form| classified.is_bearing(form))
            .flat_map(|form| {
                let written = clocks([form]);
                if written.is_empty() {
                    vec![form.span]
                } else {
                    written
                }
            })
            .collect();
        let held = !inside.is_empty();
        let bearing = entry
            .signature
            .iter()
            .any(|form| classified.is_bearing(form));
        let disposition = match entry.kind {
            Kind::Alias(target) if classified.is_pure(target) => Some(Disposition::Drop),
            Kind::Composite if bearing => Some(Disposition::DataPart),
            Kind::Function if bearing => Some(Disposition::Reading),
            Kind::Function | Kind::Clause if held => Some(Disposition::Drop),
            Kind::Alias(_) | Kind::Composite | Kind::Function | Kind::Clause => None,
        };
        if let Some(disposition) = disposition {
            classified.dispositions.insert(index, disposition);
        }
        let mut written: Vec<ClockTypePosition> = declared
            .into_iter()
            .map(|span| ClockTypePosition {
                owner: entry.owner.clone(),
                site: ClockSite::Declaration,
                span,
            })
            .chain(inside.into_iter().map(|span| ClockTypePosition {
                owner: entry.owner.clone(),
                site: ClockSite::Expression,
                span,
            }))
            .collect();
        written.sort_by_key(|position| position.span.start);
        classified.positions.extend(written);
    }
    classified
}

/// `function` as a clock-reading declaration under `classified`.
pub(super) fn reading(
    classified: &ClockForms,
    function: FunctionDeclaration,
) -> ClockReadingDeclaration {
    let parameters = function
        .parameters
        .iter()
        .map(|(_, form)| classified.slot(form))
        .collect();
    let result = classified.slot(&function.result);
    ClockReadingDeclaration {
        function,
        parameters,
        result,
    }
}

/// The data part of a record, tuple or union under `classified`: each
/// pure clock field and tuple element removed, and each union case whose
/// payload is a pure clock type left with no payload. Any other form is
/// returned unchanged.
pub(super) fn data_part(classified: &ClockForms, form: DeclarationForm) -> DeclarationForm {
    match form {
        DeclarationForm::Record(mut record) => {
            record
                .fields
                .retain(|field| !classified.is_pure(&field.type_form));
            DeclarationForm::Record(record)
        }
        DeclarationForm::Tuple(mut tuple) => {
            tuple
                .elements
                .retain(|element| !classified.is_pure(element));
            DeclarationForm::Tuple(tuple)
        }
        DeclarationForm::Union(mut union) => {
            for member in &mut union.members {
                member
                    .payload
                    .retain(|payload| !classified.is_pure(payload));
            }
            DeclarationForm::Union(union)
        }
        other => other,
    }
}
