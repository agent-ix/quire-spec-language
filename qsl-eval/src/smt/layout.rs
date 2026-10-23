// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Behavior: the state copies. Each field of each universe object is
//! one constant of the field's SMT sort, named `s<i>.<object>.<field>`; an
//! optional field also has a `Bool` presence constant
//! `s<i>.<object>.<field>.present`.

use quire_exact::{Integer, Value, ValueType};
use std::ops::Not;

use crate::simulation::model::SymbolicSubject;
use crate::smt::script::{symbol, Script, Sort, Term};
use crate::smt::{Refusal, SmtEncodingRefusal};

/// One state copy: the prefix of the names of its constants, `s<i>`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateCopy(String);

impl StateCopy {
    /// The copy at `position` of a single subject.
    pub fn at(position: u64) -> Self {
        Self(format!("s{position}"))
    }
}

/// The state an expression reads: a declared copy, or a candidate post-state
/// that takes literals for the fields an operation may write and the base
/// copy's constants for every other field (FR-120 "Post-states").
#[derive(Clone, Debug)]
pub enum StateRef {
    /// A declared copy.
    Copy(StateCopy),
    /// A candidate over `base`.
    Candidate {
        /// The copy the unwritten fields keep.
        base: StateCopy,
        /// The literal of each written field, by (object, field) position.
        written: std::rc::Rc<Vec<((usize, usize), FieldTerms)>>,
    },
}

impl StateRef {
    /// The terms of `field` of `object` in this state.
    pub fn terms(&self, layout: &StateLayout, object: usize, field: usize) -> Option<FieldTerms> {
        match self {
            Self::Copy(copy) => layout.terms(copy, object, field),
            Self::Candidate { base, written } => written
                .iter()
                .find(|(at, _)| *at == (object, field))
                .map(|(_, terms)| terms.clone())
                .or_else(|| layout.terms(base, object, field)),
        }
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
        Value::Boolean(value) => Some(Term::literal(*value)),
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
    /// [`Refusal::FieldSort`] naming the first field, in object and
    /// field order, whose sort has no SMT form;
    /// [`Refusal::ConstantName`] for a name that cannot be
    /// printed.
    pub(crate) fn of(subject: &SymbolicSubject<'_>) -> Result<Self, SmtEncodingRefusal> {
        // Names first, then sorts, in the order of FR-315's refusal table.
        for object in &subject.objects {
            for name in std::iter::once(&object.key).chain(object.fields.iter().map(|f| &f.name)) {
                if name.chars().any(|c| c == '.') {
                    return Err(Refusal::ConstantName { name: name.clone() }.into());
                }
            }
            for field in &object.fields {
                symbol(&format!("s0.{}.{}.present", object.key, field.name))?;
            }
        }
        let mut objects = Vec::new();
        for object in &subject.objects {
            let mut fields = Vec::new();
            for field in &object.fields {
                let sort = sort_of(&field.value_type).map_err(|sort| Refusal::FieldSort {
                    object: object.key.clone(),
                    field: field.name.clone(),
                    sort,
                })?;
                fields.push(FieldLayout {
                    name: field.name.clone(),
                    optional: field.optional,
                    sort,
                });
            }
            objects.push(ObjectLayout {
                key: object.key.clone(),
                fields,
            });
        }
        Ok(Self { objects })
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
                        script.declare(&name, sort);
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
    /// [`Refusal::ItemForm`] when an initial value is of no
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
                                SmtEncodingRefusal::fault("an initial value of no SMT sort")
                            })?;
                            if field.optional {
                                conjuncts.push(present);
                            }
                            conjuncts.push(constant.equal(term));
                        }
                    }
                }
            }
            states.push(Term::all(conjuncts));
        }
        Ok(Term::any(states))
    }

    /// `field` of `object` agrees at `later` and `earlier` (FR-315 "Field
    /// equality"): for a required field `(= V<later> V<earlier>)`, for an
    /// optional field `(and (= P<later> P<earlier>) (or (not P<later>)
    /// (= V<later> V<earlier>)))`, so an absent field's value constant is not
    /// part of the state.
    pub fn unchanged(
        &self,
        later: &StateCopy,
        earlier: &StateCopy,
        object: usize,
        field: usize,
    ) -> Option<Term> {
        let (is, was) = (
            self.terms(later, object, field)?,
            self.terms(earlier, object, field)?,
        );
        Some(match (is.present, was.present) {
            (Some(is_present), Some(was_present)) => Term::all([
                is_present.clone().equal(was_present),
                Term::any([is_present.not(), is.value.equal(was.value)]),
            ]),
            _ => is.value.equal(was.value),
        })
    }

    /// The field equalities of `later` with `earlier`, in declaration order.
    pub fn agreements(&self, later: &StateCopy, earlier: &StateCopy) -> Vec<Term> {
        let mut agreements = Vec::new();
        for (object, layout) in self.objects.iter().enumerate() {
            for field in 0..layout.fields.len() {
                agreements.extend(self.unchanged(later, earlier, object, field));
            }
        }
        agreements
    }
}
