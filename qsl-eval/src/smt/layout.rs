// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Behavior: the state copies. Each field of each universe object is
//! one constant of the field's SMT sort, named `s<i>.<object>.<field>`; an
//! optional field also has a `Bool` presence constant
//! `s<i>.<object>.<field>.present`.

use quire_exact::{Integer, Value, ValueType};
use std::ops::Not;

use crate::simulation::model::{SymbolicObject, SymbolicSubject};
use crate::smt::script::{symbol, Script, Sort, Term};
use crate::smt::SmtEncodingRefusal;

/// One state copy: the prefix of the names of its constants, `s<i>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateCopy(String);

impl StateCopy {
    /// The copy at `position` of a single subject.
    pub fn at(position: u64) -> Self {
        Self(format!("s{position}"))
    }
}

/// The SMT sort of one field's value constant.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldSort {
    /// `Bool`.
    Bool,
    /// `Int`, with the inclusive range its declared type admits.
    Int(Option<(Integer, Integer)>),
}

/// The sort `value_type` has, or the name of the type that has none.
fn sort_of(value_type: &ValueType) -> Result<FieldSort, &'static str> {
    match value_type {
        ValueType::Boolean => Ok(FieldSort::Bool),
        ValueType::Integer => Ok(FieldSort::Int(None)),
        ValueType::Int(interval) => Ok(FieldSort::Int(Some((
            interval.lower().clone(),
            interval.upper().clone(),
        )))),
        ValueType::Enum(shape) => {
            let count = shape.variants().count();
            Ok(FieldSort::Int(Some((
                Integer::zero(),
                Integer::from(count.saturating_sub(1)),
            ))))
        }
        ValueType::Rational(_) => Err("a Rational"),
        ValueType::Decimal(_) => Err("a Decimal"),
        ValueType::Float(_) => Err("a Float"),
        ValueType::Quantity(_) => Err("a Quantity"),
        ValueType::Text(_) => Err("a Text"),
        ValueType::String => Err("a String"),
        ValueType::Uuid => Err("a Uuid"),
        ValueType::Timestamp => Err("a Timestamp"),
        ValueType::Option(_) => Err("an Option"),
        ValueType::Composite(_) => Err("a record, tuple or union"),
        ValueType::Collection(_) => Err("a collection"),
        ValueType::Reference(_) => Err("a Reference"),
        ValueType::Population(_) => Err("a Population"),
    }
}

/// One field of one object.
#[derive(Clone, Debug)]
pub struct FieldLayout {
    name: String,
    optional: bool,
    sort: FieldSort,
}

impl FieldLayout {
    /// The field's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the field may be absent.
    pub const fn is_optional(&self) -> bool {
        self.optional
    }

    /// The sort of the field's value constant.
    pub const fn sort(&self) -> &FieldSort {
        &self.sort
    }
}

/// One universe object and its fields.
#[derive(Clone, Debug)]
pub struct ObjectLayout {
    key: String,
    fields: Vec<FieldLayout>,
}

impl ObjectLayout {
    /// The object's fields in ascending name order.
    pub fn fields(&self) -> &[FieldLayout] {
        &self.fields
    }
}

/// The terms of one field at one copy: the value constant and, for an
/// optional field, the presence constant.
#[derive(Clone, Debug)]
pub struct FieldTerms {
    /// The value constant.
    pub value: Term,
    /// The presence constant; `None` for a required field.
    pub present: Option<Term>,
}

/// The objects and fields every state copy of a subject declares.
#[derive(Clone, Debug)]
pub struct StateLayout {
    objects: Vec<ObjectLayout>,
}

/// The term a `Value` of a supported sort denotes.
pub(crate) fn value_term(value: &Value) -> Option<Term> {
    match value {
        Value::Boolean(value) => Some(Term::truth(*value)),
        Value::Integer(value) => Some(Term::Int(value.clone())),
        Value::Enum(member) => Some(Term::int(i64::from(member.rank()))),
        _ => None,
    }
}

impl StateLayout {
    /// The layout of `subject`'s objects.
    ///
    /// # Errors
    ///
    /// [`SmtEncodingRefusal::FieldSort`] naming the first field, in object and
    /// field order, whose sort has no SMT form;
    /// [`SmtEncodingRefusal::UnprintableName`] for a name that cannot be
    /// printed.
    pub(crate) fn of(subject: &SymbolicSubject<'_>) -> Result<Self, SmtEncodingRefusal> {
        let mut objects = Vec::new();
        for object in &subject.objects {
            objects.push(Self::object(object)?);
        }
        Ok(Self { objects })
    }

    fn object(object: &SymbolicObject<'_>) -> Result<ObjectLayout, SmtEncodingRefusal> {
        if object.key.chars().any(|c| c == '.') {
            return Err(SmtEncodingRefusal::AmbiguousName {
                name: object.key.clone(),
            });
        }
        let mut fields = Vec::new();
        for field in &object.fields {
            if field.name.chars().any(|c| c == '.') {
                return Err(SmtEncodingRefusal::AmbiguousName {
                    name: field.name.clone(),
                });
            }
            let sort =
                sort_of(&field.value_type).map_err(|sort| SmtEncodingRefusal::FieldSort {
                    object: object.key.clone(),
                    field: field.name.clone(),
                    sort,
                })?;
            symbol(&format!("s0.{}.{}.present", object.key, field.name))?;
            fields.push(FieldLayout {
                name: field.name.clone(),
                optional: field.optional,
                sort,
            });
        }
        Ok(ObjectLayout {
            key: object.key.clone(),
            fields,
        })
    }

    /// The objects in ascending key order.
    pub fn objects(&self) -> &[ObjectLayout] {
        &self.objects
    }

    fn constant(
        copy: &StateCopy,
        object: &ObjectLayout,
        field: &FieldLayout,
        suffix: &str,
    ) -> Term {
        let plain = format!("{}.{}.{}{suffix}", copy.0, object.key, field.name);
        // The layout validated that the name prints, quoted or not.
        let printed = symbol(&plain).unwrap_or(plain);
        Term::constant(printed)
    }

    /// The terms of `field` of `object` at `copy`.
    pub fn terms(&self, copy: &StateCopy, object: usize, field: usize) -> Option<FieldTerms> {
        let object = self.objects.get(object)?;
        let field = object.fields.get(field)?;
        Some(FieldTerms {
            value: Self::constant(copy, object, field, ""),
            present: field
                .optional
                .then(|| Self::constant(copy, object, field, ".present")),
        })
    }

    /// Declare every constant of `copy`, in ascending (object, field) order;
    /// an optional field's value constant precedes its presence constant.
    pub fn declare(&self, copy: &StateCopy, script: &mut Script) {
        for object in &self.objects {
            for field in &object.fields {
                for (suffix, sort) in std::iter::once((
                    "",
                    match field.sort {
                        FieldSort::Bool => Sort::Bool,
                        FieldSort::Int(_) => Sort::Int,
                    },
                ))
                .chain(field.optional.then_some((".present", Sort::Bool)))
                {
                    if let Term::Const(name) = Self::constant(copy, object, field, suffix) {
                        script.declare(name, sort);
                    }
                }
            }
        }
    }

    /// The range assertion of each ranged value constant of `copy`, in
    /// declaration order.
    pub fn ranges(&self, copy: &StateCopy) -> Vec<Term> {
        let mut ranges = Vec::new();
        for object in &self.objects {
            for field in &object.fields {
                if let FieldSort::Int(Some((lower, upper))) = &field.sort {
                    ranges.push(Self::constant(copy, object, field, "").within(lower, upper));
                }
            }
        }
        ranges
    }

    /// The initial-state predicate on `copy`: some initial state of
    /// `subject` holds there, field for field.
    ///
    /// # Errors
    ///
    /// [`SmtEncodingRefusal::Fault`] when an initial value is of no
    /// supported sort.
    pub(crate) fn initial(
        &self,
        copy: &StateCopy,
        subject: &SymbolicSubject<'_>,
    ) -> Result<Term, SmtEncodingRefusal> {
        let mut states: Vec<Term> = Vec::new();
        for state in &subject.initial {
            let mut conjuncts = Vec::new();
            for (object, values) in self.objects.iter().zip(state) {
                for (field, value) in object.fields.iter().zip(values) {
                    let constant = Self::constant(copy, object, field, "");
                    let present = Self::constant(copy, object, field, ".present");
                    match value {
                        None => conjuncts.push(present.not()),
                        Some(value) => {
                            let term = value_term(value).ok_or_else(|| {
                                SmtEncodingRefusal::Fault(qsl_foundation::InternalFault::new(
                                    "smt-encoding",
                                    "initial-value-sort",
                                ))
                            })?;
                            if field.optional {
                                conjuncts.push(present);
                            }
                            conjuncts.push(constant.equal(term));
                        }
                    }
                }
            }
            let state = Term::all(conjuncts);
            if !states.contains(&state) {
                states.push(state);
            }
        }
        Ok(Term::any(states))
    }

    /// `field` of `object` is the same at `left` and `right`: equal values,
    /// and for an optional field equal presence and, where present, equal
    /// values.
    pub fn unchanged(
        &self,
        left: &StateCopy,
        right: &StateCopy,
        object: usize,
        field: usize,
    ) -> Option<Term> {
        let (before, after) = (
            self.terms(left, object, field)?,
            self.terms(right, object, field)?,
        );
        Some(match (before.present, after.present) {
            (Some(was), Some(is)) => Term::all([
                was.clone().equal(is),
                was.implies(before.value.equal(after.value)),
            ]),
            _ => before.value.equal(after.value),
        })
    }

    /// Some field differs between `left` and `right`.
    pub fn differs(&self, left: &StateCopy, right: &StateCopy) -> Term {
        let mut differences = Vec::new();
        for (object, layout) in self.objects.iter().enumerate() {
            for field in 0..layout.fields.len() {
                if let Some(same) = self.unchanged(left, right, object, field) {
                    differences.push(same.not());
                }
            }
        }
        Term::any(differences)
    }
}
