// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resolves a `qsl_forms::TypeForm` -- S2's syntactic declared type, with no
//! semantic identity (ADR-011 §1's stage table; §2.2 row E2) -- to the
//! kernel `ValueType` at E3 (ADR-013 O-14/C-26: "Checked type node ->
//! kernel `ValueType` (QSL checker)").
//!
//! Bound literals are read in the grammar's own spellings (`qsl-cst`'s
//! `RoundingMode` and `TextProfile` productions: `nearest-even`,
//! `unicode-scalars`, `nfc`, ...). Resolution reports a [`TypeFormError`]:
//! the typed [`TypeFormFault`] and the span of the type form it concerns.
//! The FR-091 assembler reports that as its own error; every other caller
//! goes through [`resolve_type_form`], which refuses through the existing
//! check diagnostics (`CheckCause::MissingName`/`AmbiguousName`, or
//! `CheckCause::IllTyped(IllTypedCause::TypeMismatch)` for a malformed
//! bound or argument), never a new catalog code.
//!
//! Names resolve through [`TypeNames`]: the package [`Scope`], or the
//! assembler's own table of the unit's declarations while it is still
//! building the package.

use std::marker::PhantomData;
use std::ops::ControlFlow;

use super::check::Scope;
use super::refusal::{CheckCause, CheckRefusal};
use qsl_forms::{BuiltinType, NameForm, TypeForm, TypeFormHead};
use qsl_foundation::Span;
use quire_exact::{
    CardinalityBound, CarriedQuantityType, CollectionKind, CollectionType, DecimalType,
    EffectiveId, FloatType, IeeeWidth, IllTypedCause, Integer, IntegerInterval, QuantityCarrier,
    RationalDomain, RoundingMode, TextProfile, TextType, UnitId, ValueType,
};
use quire_semantic_value::location::Location;

/// What a qualified type name resolves against.
pub(crate) trait TypeNames {
    /// Every type `name` binds, in resolution order. Empty when it binds
    /// none; more than one is ambiguous.
    fn named_types(&self, name: &str) -> &[ValueType];

    /// The object type named `name`, by its effective identity.
    fn object_type_named(&self, name: &str) -> Option<EffectiveId>;

    /// The unit declared under `name`, in the unit namespace (QSpec FR-142
    /// "Quantity type reference"): only `Quantity<u>` looks here.
    fn unit_named(&self, name: &str) -> Option<UnitId>;
}

impl TypeNames for Scope {
    fn named_types(&self, name: &str) -> &[ValueType] {
        Scope::named_types(self, name)
    }

    fn object_type_named(&self, name: &str) -> Option<EffectiveId> {
        Scope::object_type_named(self, name)
    }

    fn unit_named(&self, name: &str) -> Option<UnitId> {
        Scope::unit_named(self, name)
    }
}

/// Which integer of a type form or an expression lies outside i128.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IntegerSite {
    /// An `Int[lo, hi]` type form's lower bound.
    Lower,
    /// An `Int[lo, hi]` type form's upper bound.
    Upper,
    /// An integer literal of an expression.
    Literal,
}

/// The i128 limit an integer crosses.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum I128Limit {
    /// `i128::MIN`, crossed by a value below the range.
    Min,
    /// `i128::MAX`, crossed by a value above the range.
    Max,
}

/// FR-091's `IntegerOutsideI128` cause: an integer bound or literal outside
/// `i128::MIN..=i128::MAX`. The span of the type form or literal is carried
/// by the refusal that holds this cause.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct IntegerOutsideI128 {
    /// The bound or literal it is.
    pub site: IntegerSite,
    /// The integer as written, as its canonical decimal string.
    pub value: String,
    /// The limit it crosses.
    pub limit: I128Limit,
}

impl IntegerOutsideI128 {
    /// `Some` when `value` lies outside `i128::MIN..=i128::MAX`.
    pub(crate) fn of(site: IntegerSite, value: &Integer) -> Option<Self> {
        let limit = if *value < Integer::from(i128::MIN) {
            I128Limit::Min
        } else if *value > Integer::from(i128::MAX) {
            I128Limit::Max
        } else {
            return None;
        };
        Some(Self {
            site,
            value: value.to_string(),
            limit,
        })
    }
}

/// Why a type form did not resolve.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum TypeFormFault {
    /// An unqualified name binds no type.
    MissingName(String),
    /// A qualified name binds no type.
    MissingQualifiedName(qsl_forms::NameForm),
    /// The qualified name binds more than one type.
    AmbiguousName(String),
    /// A bound is missing, is not a number or a grammar spelling, or the
    /// form has the wrong number of type arguments.
    Malformed,
    /// An `Int` or `Rational` interval whose lower bound is above its
    /// upper bound (`IntegerInterval`'s own refusal).
    EmptyInterval,
    /// An `Int` bound outside `i128::MIN..=i128::MAX` (FR-091).
    IntegerOutsideI128(IntegerOutsideI128),
    /// A `Rational` denominator interval that reaches below one
    /// (`RationalDomain`'s own refusal).
    DenominatorBelowOne,
    /// A `Decimal` whose bounds or scales are out of order
    /// (`DecimalType`'s own refusal).
    MalformedDecimal,
    /// A `Text` minimum above its maximum (`TextType`'s own refusal).
    EmptyTextBounds,
    /// A collection cardinality minimum above its maximum
    /// (`CardinalityBound`'s own refusal).
    EmptyCardinality,
    /// The `Clock` type, which S2 builds (FR-393) and no S3 check resolves
    /// yet (FR-230).
    ClockNotChecked,
}

/// A [`TypeFormFault`] and the span of the type form it concerns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeFormError {
    /// Why the form did not resolve.
    pub fault: TypeFormFault,
    /// The span of the form (or nested argument form) it concerns.
    pub span: Span,
}

impl TypeFormError {
    /// This error as a check refusal at `location`.
    fn into_refusal(self, location: &Location) -> CheckRefusal {
        let cause = match self.fault {
            TypeFormFault::MissingName(name) => CheckCause::MissingName(name),
            TypeFormFault::MissingQualifiedName(name) => CheckCause::MissingQualifiedName(name),
            TypeFormFault::AmbiguousName(name) => CheckCause::AmbiguousName {
                name,
                loci: vec![location.clone()],
            },
            TypeFormFault::IntegerOutsideI128(outside) => CheckCause::IntegerOutsideI128(outside),
            TypeFormFault::ClockNotChecked => CheckCause::TimedNotChecked {
                construct: super::refusal::TimedConstruct::Clock,
                span: self.span,
            },
            TypeFormFault::Malformed
            | TypeFormFault::EmptyInterval
            | TypeFormFault::DenominatorBelowOne
            | TypeFormFault::MalformedDecimal
            | TypeFormFault::EmptyTextBounds
            | TypeFormFault::EmptyCardinality => {
                return CheckRefusal::ill_typed(location, IllTypedCause::TypeMismatch)
            }
        };
        CheckRefusal {
            location: location.clone(),
            cause,
        }
    }
}

fn fault(form: &TypeForm, fault: TypeFormFault) -> TypeFormError {
    TypeFormError {
        fault,
        span: form.span,
    }
}

fn malformed(form: &TypeForm) -> TypeFormError {
    fault(form, TypeFormFault::Malformed)
}

/// The native value type a spelling names (FR-091): `String`, `UUID` and
/// `Timestamp` are unreserved names, so the native type is one candidate
/// beside the declared forms of the same name.
#[ix_trace_rs::string_edge]
pub(crate) fn native_scalar_named(name: &str) -> Option<ValueType> {
    match name {
        "String" => Some(ValueType::String),
        "UUID" => Some(ValueType::Uuid),
        "Timestamp" => Some(ValueType::Timestamp),
        _ => None,
    }
}

/// Resolve a qualified type name against `names`: an alias, a record or
/// tuple, an enum, a model object type, or one of the native scalar
/// spellings. A name that binds more than one of these is ambiguous.
fn named_type(
    names: &impl TypeNames,
    name: &qsl_forms::NameForm,
    form: &TypeForm,
) -> Result<ValueType, TypeFormError> {
    let native = native_scalar_named(&name.name);
    match (names.named_types(&name.name), native) {
        ([], None) if name.is_qualified() => Err(fault(
            form,
            TypeFormFault::MissingQualifiedName(name.clone()),
        )),
        ([], None) => Err(fault(form, TypeFormFault::MissingName(name.name.clone()))),
        ([], Some(native)) => Ok(native),
        ([value_type], None) => Ok(value_type.clone()),
        _ => Err(fault(form, TypeFormFault::AmbiguousName(name.name.clone()))),
    }
}

/// Resolve a qualified type name against `scope`'s declared aliases,
/// composites, enums and object types: an alias, a record or tuple, an
/// enum, or a model object type. `allInstances<T>(p)`/`lookup<T>(p, r)`
/// (FR-153) name their queried type `T` this way (ADR-013 O-11: "name ->
/// node id by the checker only"), resolving to `ValueType::Reference`.
pub(crate) fn resolve_named_type(
    scope: &Scope,
    name: &str,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    let span = Span { start: 0, end: 0 };
    let name = NameForm::new(name, span);
    let form = TypeForm::new(TypeFormHead::Name(name.clone()), span);
    named_type(scope, &name, &form).map_err(|error| error.into_refusal(location))
}

fn bound_text(form: &TypeForm, index: usize) -> Result<&str, TypeFormError> {
    form.bounds
        .get(index)
        .map(String::as_str)
        .ok_or_else(|| malformed(form))
}

/// A collection or population form writes either no bound (the unbounded
/// type, FR-333) or exactly `count` of them; any other count is malformed.
fn optional_bounds(form: &TypeForm, count: usize) -> Result<(), TypeFormError> {
    match form.bounds.len() {
        0 => Ok(()),
        written if written == count => Ok(()),
        _ => Err(malformed(form)),
    }
}

fn bound_integer(form: &TypeForm, index: usize) -> Result<Integer, TypeFormError> {
    bound_text(form, index)?
        .parse()
        .map_err(|_| malformed(form))
}

fn bound_u64(form: &TypeForm, index: usize) -> Result<u64, TypeFormError> {
    bound_text(form, index)?
        .parse()
        .map_err(|_| malformed(form))
}

/// A rounding mode as the grammar spells it (`qsl-cst` `RoundingMode`).
#[ix_trace_rs::string_edge]
pub(crate) fn parse_rounding_mode(text: &str) -> Option<RoundingMode> {
    match text {
        "exact" => Some(RoundingMode::Exact),
        "toward-zero" => Some(RoundingMode::TowardZero),
        "toward-positive" => Some(RoundingMode::TowardPositive),
        "toward-negative" => Some(RoundingMode::TowardNegative),
        "nearest-even" => Some(RoundingMode::NearestEven),
        "nearest-away" => Some(RoundingMode::NearestAway),
        _ => None,
    }
}

/// A text profile as the grammar spells it (`qsl-cst` `TextProfile`).
#[ix_trace_rs::string_edge]
fn parse_text_profile(text: &str) -> Option<TextProfile> {
    match text {
        "unicode-scalars" => Some(TextProfile::UnicodeScalars),
        "nfc" => Some(TextProfile::Nfc),
        "nfd" => Some(TextProfile::Nfd),
        "nfkc" => Some(TextProfile::Nfkc),
        "nfkd" => Some(TextProfile::Nfkd),
        "binary-utf8" => Some(TextProfile::BinaryUtf8),
        _ => None,
    }
}

fn only_argument(form: &TypeForm) -> Result<&TypeForm, TypeFormError> {
    match form.arguments.as_slice() {
        [argument] => Ok(argument),
        _ => Err(malformed(form)),
    }
}

fn interval(form: &TypeForm, lower: usize) -> Result<IntegerInterval, TypeFormError> {
    IntegerInterval::new(bound_integer(form, lower)?, bound_integer(form, lower + 1)?)
        .map_err(|_| fault(form, TypeFormFault::EmptyInterval))
}

/// An `Int[lo, hi]` form's interval, each bound inside i128 (FR-091). The
/// lower bound is judged before the upper.
fn int_interval(form: &TypeForm) -> Result<IntegerInterval, TypeFormError> {
    for (index, site) in [(0, IntegerSite::Lower), (1, IntegerSite::Upper)] {
        if let Some(outside) = IntegerOutsideI128::of(site, &bound_integer(form, index)?) {
            return Err(fault(form, TypeFormFault::IntegerOutsideI128(outside)));
        }
    }
    interval(form, 0)
}

/// `Float32[mode]`/`Float64[mode]`; the bare spelling is strict `exact`
/// (FR-091-OQ-4).
fn float_type(form: &TypeForm, width: IeeeWidth) -> Result<ValueType, TypeFormError> {
    let rounding = match form.bounds.first() {
        Some(text) => parse_rounding_mode(text).ok_or_else(|| malformed(form))?,
        None => RoundingMode::Exact,
    };
    Ok(ValueType::Float(FloatType::new(width, rounding)))
}

/// What one type form needs before its own type can be built: nothing (a
/// leaf, resolved already), or the type of its one argument.
enum Step<'f> {
    /// A form with no type argument, already resolved.
    Leaf(ValueType),
    /// `Option<T>`: wraps its argument's type.
    Option {
        form: &'f TypeForm,
        argument: &'f TypeForm,
    },
    /// A collection: builds its element's type into a collection type with
    /// this form's cardinality bounds.
    Collection {
        kind: CollectionKind,
        form: &'f TypeForm,
        argument: &'f TypeForm,
    },
}

fn resolve_builtin<'f>(
    names: &impl TypeNames,
    builtin: BuiltinType,
    form: &'f TypeForm,
) -> Result<Step<'f>, TypeFormError> {
    let leaf = match builtin {
        BuiltinType::Boolean => Ok(ValueType::Boolean),
        BuiltinType::Integer => Ok(ValueType::Integer),
        BuiltinType::Int => {
            optional_bounds(form, 2)?;
            int_interval(form).map(ValueType::Int)
        }
        BuiltinType::Rational => RationalDomain::new(interval(form, 0)?, interval(form, 2)?)
            .map(ValueType::Rational)
            .map_err(|_| fault(form, TypeFormFault::DenominatorBelowOne)),
        BuiltinType::Decimal => {
            let lower = bound_integer(form, 0)?;
            let upper = bound_integer(form, 1)?;
            let min_scale = bound_u64(form, 2)?;
            let max_scale = bound_u64(form, 3)?;
            let rounding = match form.bounds.get(4) {
                Some(text) => parse_rounding_mode(text).ok_or_else(|| malformed(form))?,
                None => RoundingMode::default(),
            };
            DecimalType::new(lower, upper, min_scale, max_scale, rounding)
                .map(ValueType::Decimal)
                .map_err(|_| fault(form, TypeFormFault::MalformedDecimal))
        }
        BuiltinType::Float32 => float_type(form, IeeeWidth::Binary32),
        BuiltinType::Float64 => float_type(form, IeeeWidth::Binary64),
        BuiltinType::Text => {
            let min = bound_u64(form, 0)?;
            let max = bound_u64(form, 1)?;
            let profile = match form.bounds.get(2) {
                Some(text) => parse_text_profile(text).ok_or_else(|| malformed(form))?,
                None => TextProfile::UnicodeScalars,
            };
            TextType::new(min, max, profile)
                .map(ValueType::Text)
                .map_err(|_| fault(form, TypeFormFault::EmptyTextBounds))
        }
        BuiltinType::Option => {
            return Ok(Step::Option {
                form,
                argument: only_argument(form)?,
            })
        }
        BuiltinType::Clock => Err(fault(form, TypeFormFault::ClockNotChecked)),
        BuiltinType::Reference => {
            let target = only_argument(form)?;
            let TypeFormHead::Name(name) = &target.head else {
                return Err(malformed(form));
            };
            names
                .object_type_named(&name.name)
                .map(ValueType::Reference)
                .ok_or_else(|| {
                    let cause = if name.is_qualified() {
                        TypeFormFault::MissingQualifiedName(name.clone())
                    } else {
                        TypeFormFault::MissingName(name.name.clone())
                    };
                    fault(target, cause)
                })
        }
        BuiltinType::Quantity => {
            let (target, carrier) = match form.arguments.as_slice() {
                [target] => (target, None),
                [target, carrier] => (target, Some(carrier)),
                _ => return Err(malformed(form)),
            };
            let TypeFormHead::Name(name) = &target.head else {
                return Err(malformed(form));
            };
            let unit = names.unit_named(&name.name).ok_or_else(|| {
                let cause = if name.is_qualified() {
                    TypeFormFault::MissingQualifiedName(name.clone())
                } else {
                    TypeFormFault::MissingName(name.name.clone())
                };
                fault(target, cause)
            })?;
            match carrier {
                None => Ok(ValueType::Quantity(unit)),
                Some(carrier) => {
                    carried_quantity_type(names, unit, carrier).map(ValueType::carried_quantity)
                }
            }
        }
    };
    leaf.map(Step::Leaf)
}

/// Resolve `Quantity<u, C>`'s bounded scalar carrier. The `Int` spelling may
/// add a rounding mode because an integer type by itself has no rounding
/// metadata; an omitted mode is exact.
fn carried_quantity_type(
    names: &impl TypeNames,
    unit: UnitId,
    form: &TypeForm,
) -> Result<CarriedQuantityType, TypeFormError> {
    let carrier = match &form.head {
        TypeFormHead::Builtin(BuiltinType::Int) if form.arguments.is_empty() => {
            if !matches!(form.bounds.len(), 2 | 3) {
                return Err(malformed(form));
            }
            let domain = int_interval(form)?;
            let rounding = match form.bounds.get(2) {
                Some(text) => parse_rounding_mode(text).ok_or_else(|| malformed(form))?,
                None => RoundingMode::Exact,
            };
            QuantityCarrier::Integer { domain, rounding }
        }
        TypeFormHead::Builtin(BuiltinType::Decimal) if form.arguments.is_empty() => {
            optional_bounds(form, 5)?;
            let decimal = match resolve_builtin(names, BuiltinType::Decimal, form)? {
                Step::Leaf(value) => match &value {
                    ValueType::Decimal(decimal) => decimal.clone(),
                    _ => return Err(malformed(form)),
                },
                Step::Option { .. } | Step::Collection { .. } => return Err(malformed(form)),
            };
            QuantityCarrier::Decimal(decimal)
        }
        TypeFormHead::Builtin(BuiltinType::Rational) if form.arguments.is_empty() => {
            optional_bounds(form, 4)?;
            let domain = match resolve_builtin(names, BuiltinType::Rational, form)? {
                Step::Leaf(value) => match &value {
                    ValueType::Rational(domain) => domain.clone(),
                    _ => return Err(malformed(form)),
                },
                Step::Option { .. } | Step::Collection { .. } => return Err(malformed(form)),
            };
            QuantityCarrier::Rational(domain)
        }
        _ => return Err(malformed(form)),
    };
    Ok(CarriedQuantityType::new(unit, carrier))
}

/// The object type `T` of a `Population<T>[N]` form, by its effective
/// identity, or `None` for any other form. FR-094 builds the population's
/// type node from `T`, which `ValueType::Population` does not carry; a `T`
/// that names no object type refuses `ill_typed`/`type-mismatch`.
pub(crate) fn population_target(
    scope: &Scope,
    form: &TypeForm,
    location: &Location,
) -> Result<Option<EffectiveId>, CheckRefusal> {
    if !matches!(form.head, TypeFormHead::Population) {
        return Ok(None);
    }
    let argument = only_argument(form).map_err(|error| error.into_refusal(location))?;
    match &resolve_type_form(scope, argument, location)? {
        ValueType::Reference(target) => Ok(Some(target.clone())),
        _ => Err(CheckRefusal::ill_typed(
            location,
            IllTypedCause::TypeMismatch,
        )),
    }
}

/// Resolve a syntactic `TypeForm` (S2) to the kernel `ValueType` (E3,
/// ADR-013 O-14/C-26) against `names`, reporting the fault and the span of
/// the form it concerns.
///
/// `Population<T>[N]` (FR-153) resolves to `ValueType::Population(N)`: the
/// kernel type carries only the declared maximum, and `allInstances<T>(p)`/
/// `lookup<T>(p, r)` name `T` at their own call site, so `T` is carried as
/// syntax but not resolved here.
pub(crate) fn resolve_form(
    names: &impl TypeNames,
    form: &TypeForm,
) -> Result<ValueType, TypeFormError> {
    let mut resolver = Resolver {
        names,
        resolved: Vec::new(),
        forms: PhantomData,
        #[cfg(test)]
        gauge: crate::stack_peak::Gauge::new("resolve_form"),
    };
    match quire_walk::walk(&mut resolver, form) {
        ControlFlow::Continue(()) => resolver.resolved.pop().ok_or_else(|| malformed(form)),
        ControlFlow::Break(error) => Err(error),
    }
}

/// One type form's head: a leaf resolves at once; `Option` and a
/// collection wait for their argument.
fn resolve_head<'f>(names: &impl TypeNames, form: &'f TypeForm) -> Result<Step<'f>, TypeFormError> {
    match &form.head {
        TypeFormHead::Builtin(builtin) => resolve_builtin(names, *builtin, form),
        TypeFormHead::Collection(kind) => Ok(Step::Collection {
            kind: *kind,
            form,
            argument: only_argument(form)?,
        }),
        TypeFormHead::Population => {
            optional_bounds(form, 1)?;
            let maximum = form
                .bounds
                .first()
                .map(|_| bound_u64(form, 0))
                .transpose()?;
            Ok(Step::Leaf(ValueType::Population(maximum)))
        }
        TypeFormHead::Name(name) => named_type(names, name, form).map(Step::Leaf),
    }
}

/// [`resolve_form`]'s walk (ADR-030): each form's exit builds its type from
/// its argument's, which the argument's own exit left on `resolved`, so a
/// type nested to any depth resolves on a constant native stack. The heap
/// stack grows by one frame per type form, which S1 charged as a node.
struct Resolver<'n, 'f, N> {
    names: &'n N,
    resolved: Vec<ValueType>,
    forms: PhantomData<&'f TypeForm>,
    #[cfg(test)]
    gauge: crate::stack_peak::Gauge,
}

impl<'f, N: TypeNames> quire_walk::Walk for Resolver<'_, 'f, N> {
    type Node = &'f TypeForm;
    type Frame = Step<'f>;
    type Stop = TypeFormError;

    fn enter(
        &mut self,
        form: &'f TypeForm,
        children: &mut quire_walk::Children<'_, &'f TypeForm>,
    ) -> ControlFlow<TypeFormError, Step<'f>> {
        let step = match resolve_head(self.names, form) {
            Ok(step) => step,
            Err(error) => return ControlFlow::Break(error),
        };
        match &step {
            Step::Leaf(_) => {
                #[cfg(test)]
                self.gauge.enter(0);
            }
            Step::Option { argument, .. } | Step::Collection { argument, .. } => {
                #[cfg(test)]
                self.gauge.enter(1);
                children.push(argument);
            }
        }
        ControlFlow::Continue(step)
    }

    fn exit(&mut self, step: Step<'f>) -> ControlFlow<TypeFormError> {
        #[cfg(test)]
        {
            self.gauge.exit();
            crate::stack_peak::note("resolve_form_built", self.resolved.len());
        }
        match self.build(step) {
            Ok(resolved) => {
                self.resolved.push(resolved);
                ControlFlow::Continue(())
            }
            Err(error) => ControlFlow::Break(error),
        }
    }
}

impl<N: TypeNames> Resolver<'_, '_, N> {
    /// The type `step` names, its argument's type taken from `resolved`.
    fn build(&mut self, step: Step<'_>) -> Result<ValueType, TypeFormError> {
        match step {
            Step::Leaf(resolved) => Ok(resolved),
            Step::Option { form, .. } => {
                let inner = self.resolved.pop().ok_or_else(|| malformed(form))?;
                Ok(ValueType::option(inner))
            }
            Step::Collection { kind, form, .. } => {
                let element = self.resolved.pop().ok_or_else(|| malformed(form))?;
                optional_bounds(form, 2)?;
                let bound = if form.bounds.is_empty() {
                    None
                } else {
                    let minimum = bound_u64(form, 0)?;
                    let maximum = bound_u64(form, 1)?;
                    Some(
                        CardinalityBound::new(minimum, maximum)
                            .map_err(|_| fault(form, TypeFormFault::EmptyCardinality))?,
                    )
                };
                Ok(ValueType::collection(CollectionType::new(
                    kind, element, bound,
                )))
            }
        }
    }
}

/// Resolve a syntactic `TypeForm` against the package `scope`, refusing at
/// `location` (see the module doc).
pub(crate) fn resolve_type_form(
    scope: &Scope,
    form: &TypeForm,
    location: &Location,
) -> Result<ValueType, CheckRefusal> {
    resolve_form(scope, form).map_err(|error| error.into_refusal(location))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::family::fixtures::{empty_scope, root_location, scope_with};
    use ix_trace_rs::trace;
    use quire_exact::CollectionKind;
    use quire_semantic_value::declaration::{ObjectTypeDeclaration, TypeEnvironment};

    const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

    struct OneUnit(UnitId);

    impl TypeNames for OneUnit {
        fn named_types(&self, _name: &str) -> &[ValueType] {
            &[]
        }

        fn object_type_named(&self, _name: &str) -> Option<EffectiveId> {
            None
        }

        fn unit_named(&self, name: &str) -> Option<UnitId> {
            (name == "m").then(|| self.0.clone())
        }
    }

    fn quantity_form(carrier: TypeForm) -> TypeForm {
        TypeForm::builtin(BuiltinType::Quantity, SPAN)
            .with_arguments(vec![TypeForm::name("m", SPAN), carrier])
    }

    /// FR-091-AC-11 (TC-398 step 3): the assembler's resolution `match` has
    /// no catch-all arm, neither `_` nor a bare binding.
    #[ix_trace_rs::trace("FR-091-AC-11", "TC-398")]
    #[test]
    fn resolve_form_has_no_catch_all_arm() {
        use syn::visit::Visit;
        struct Finder(Vec<usize>);
        impl<'ast> Visit<'ast> for Finder {
            fn visit_expr_match(&mut self, expression: &'ast syn::ExprMatch) {
                for arm in &expression.arms {
                    if matches!(arm.pat, syn::Pat::Wild(_) | syn::Pat::Ident(_)) {
                        self.0
                            .push(syn::spanned::Spanned::span(&arm.pat).start().line);
                    }
                }
                syn::visit::visit_expr_match(self, expression);
            }
        }
        let file = syn::parse_file(include_str!("type_form.rs")).expect("type_form.rs parses");
        let function = file
            .items
            .iter()
            .find_map(|item| match item {
                syn::Item::Fn(function) if function.sig.ident == "resolve_head" => Some(function),
                _ => None,
            })
            .expect("fn resolve_head exists");
        let mut finder = Finder(Vec::new());
        finder.visit_item_fn(function);
        assert!(
            finder.0.is_empty(),
            "resolve_head has a catch-all arm at lines {:?}",
            finder.0
        );
    }

    /// `resolve_form`'s task stack and its result stack each grow by at
    /// most one entry per type form, and S1 charged at least one CST node
    /// per type form: a 1,000-deep `Option` and a 1,000-deep `Sequence`
    /// peak within the form count.
    #[ix_trace_rs::trace("TC-724", "FR-257-AC-1")]
    #[test]
    fn resolve_form_stacks_grow_within_the_form_count() {
        const DEPTH: usize = 1_000;
        let mut option = builtin(BuiltinType::Integer, &[]);
        let mut sequence = builtin(BuiltinType::Integer, &[]);
        for _ in 0..DEPTH {
            option = TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![option]);
            sequence = TypeForm::new(TypeFormHead::Collection(CollectionKind::Sequence), SPAN)
                .with_arguments(vec![sequence])
                .with_bounds(vec!["0".to_owned(), "1".to_owned()]);
        }
        for form in [option, sequence] {
            resolve(&form).expect("a deep type resolves");
            for stack in ["resolve_form", "resolve_form_built"] {
                let peak = crate::stack_peak::take(stack);
                assert!(peak > 0, "{stack} was not used");
                assert!(peak <= DEPTH + 1, "{stack} peaked at {peak}");
            }
        }
    }

    fn builtin(builtin: BuiltinType, bounds: &[&str]) -> TypeForm {
        TypeForm::builtin(builtin, SPAN)
            .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
    }

    fn resolve_in(scope: &Scope, form: &TypeForm) -> Result<ValueType, CheckRefusal> {
        resolve_type_form(scope, form, &root_location())
    }

    fn resolve(form: &TypeForm) -> Result<ValueType, CheckRefusal> {
        resolve_in(&empty_scope(), form)
    }

    fn assert_mismatch(form: &TypeForm) {
        let refusal = resolve(form).expect_err("the form is malformed");
        assert!(
            matches!(
                refusal.cause,
                CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            ),
            "{form:?} refused with {:?}",
            refusal.cause
        );
    }

    fn object_type(name: &str) -> TypeEnvironment {
        TypeEnvironment::new(
            [],
            [ObjectTypeDeclaration::new(
                quire_exact::EffectiveId::new(
                    None,
                    quire_exact::OriginalDeclarationKey::new("fixture", "o7"),
                ),
                name,
                Vec::new(),
            )],
        )
        .expect("one object type admits")
    }

    /// FR-091-OQ-4 (TC-405): `Float32[mode]`/`Float64[mode]` resolve to a
    /// float type carrying the mode; a bare float is strict `exact`.
    #[ix_trace_rs::trace("FR-091-AC-19", "TC-405")]
    #[test]
    fn float_types_carry_their_rounding_mode() {
        let float = |form: &TypeForm| match resolve(form).expect("a float type resolves") {
            ValueType::Float(float) => (float.width(), float.rounding()),
            other => panic!("a float type, not {other:?}"),
        };
        assert_eq!(
            float(&builtin(BuiltinType::Float64, &["nearest-even"])),
            (IeeeWidth::Binary64, RoundingMode::NearestEven)
        );
        assert_eq!(
            float(&builtin(BuiltinType::Float32, &["toward-zero"])),
            (IeeeWidth::Binary32, RoundingMode::TowardZero)
        );
        assert_eq!(
            float(&builtin(BuiltinType::Float64, &[])),
            (IeeeWidth::Binary64, RoundingMode::Exact)
        );
        assert_mismatch(&builtin(BuiltinType::Float64, &["NearestEven"]));
    }

    /// Bound literals use the grammar's own spellings (`qsl-cst`
    /// `RoundingMode`, `TextProfile`); the Rust variant names are not
    /// language spellings and are refused.
    #[test]
    fn bounds_resolve_in_the_grammars_spellings_only() {
        let decimal = resolve(&builtin(
            BuiltinType::Decimal,
            &["0", "100", "0", "2", "nearest-even"],
        ))
        .expect("nearest-even is a grammar rounding mode");
        let ValueType::Decimal(ref decimal) = decimal else {
            panic!("a decimal type, not {decimal:?}");
        };
        assert_eq!(decimal.rounding(), RoundingMode::NearestEven);
        let text = resolve(&builtin(BuiltinType::Text, &["1", "5", "nfc"]))
            .expect("nfc is a grammar text profile");
        let ValueType::Text(text) = text else {
            panic!("a text type, not {text:?}");
        };
        assert_eq!(text.profile(), TextProfile::Nfc);
        assert_mismatch(&builtin(
            BuiltinType::Decimal,
            &["0", "100", "0", "2", "NearestEven"],
        ));
        assert_mismatch(&builtin(BuiltinType::Text, &["1", "5", "Nfc"]));
        assert_mismatch(&builtin(BuiltinType::Text, &["1", "5", "unicode"]));
    }

    /// FR-497-AC-1: carrier bounds and rounding are part of the carried
    /// quantity type identity; an omitted Int mode means exact.
    #[ix_trace_rs::trace("FR-497-AC-1", "TC-1287")]
    #[test]
    fn carried_quantity_resolution_retains_carrier_identity() {
        let unit = UnitId::declared(
            quire_exact::NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Text("unit".to_owned())],
                1,
            )
            .expect("a fixture node key is well formed"),
        );
        let names = OneUnit(unit.clone());
        let int_form = |bounds: &[&str]| {
            TypeForm::builtin(BuiltinType::Int, SPAN)
                .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
        };
        let exact = resolve_form(&names, &quantity_form(int_form(&["-2", "2"])))
            .expect("a bounded Int carrier resolves");
        let nearest = resolve_form(
            &names,
            &quantity_form(int_form(&["-2", "2", "nearest-even"])),
        )
        .expect("an Int carrier may carry a rounding mode");
        let decimal = resolve_form(
            &names,
            &quantity_form(
                TypeForm::builtin(BuiltinType::Decimal, SPAN).with_bounds(
                    ["0", "1", "0", "4", "nearest-even"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
            ),
        )
        .expect("a bounded Decimal carrier resolves");
        let rational = resolve_form(
            &names,
            &quantity_form(
                TypeForm::builtin(BuiltinType::Rational, SPAN).with_bounds(
                    ["-3", "7", "1", "12"]
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                ),
            ),
        )
        .expect("a bounded Rational carrier resolves with exact mode");

        let ValueType::CarriedQuantity(exact) = &exact else {
            panic!("carried quantity, not {exact:?}");
        };
        assert_eq!(exact.unit(), &unit);
        assert_eq!(
            exact.carrier(),
            &QuantityCarrier::Integer {
                domain: IntegerInterval::new(Integer::from(-2_i64), Integer::from(2_i64))
                    .expect("ordered bounds"),
                rounding: RoundingMode::Exact,
            }
        );
        assert_ne!(
            exact.as_ref(),
            match &nearest {
                ValueType::CarriedQuantity(nearest) => nearest.as_ref(),
                other => panic!("carried quantity, not {other:?}"),
            }
        );
        assert!(matches!(
            &decimal,
            ValueType::CarriedQuantity(carried)
                if matches!(carried.carrier(), QuantityCarrier::Decimal(decimal)
                    if decimal.rounding() == RoundingMode::NearestEven)
        ));
        assert!(matches!(
            &rational,
            ValueType::CarriedQuantity(carried)
                if matches!(carried.carrier(), QuantityCarrier::Rational(domain)
                    if domain.numerator().lower() == &Integer::from(-3_i64)
                        && domain.numerator().upper() == &Integer::from(7_i64)
                        && domain.denominator().lower() == &Integer::one()
                        && domain.denominator().upper() == &Integer::from(12_i64))
        ));
    }

    /// FR-497-AC-1/2: bounded Int, Decimal and Rational forms are carriers;
    /// bare numeric types and inverted bounds refuse.
    #[ix_trace_rs::trace(
        "FR-497-AC-1",
        "FR-497-AC-2",
        "TC-1287",
        "QSpec-TC-187",
        "QSpec-FR-142-AC-27"
    )]
    #[test]
    fn carried_quantity_rejects_unbounded_or_invalid_carriers() {
        let names = OneUnit(UnitId::declared(
            quire_exact::NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Text("unit".to_owned())],
                1,
            )
            .expect("a fixture node key is well formed"),
        ));
        let bare_integer = quantity_form(TypeForm::builtin(BuiltinType::Integer, SPAN));
        assert!(resolve_form(&names, &bare_integer).is_err());
        let bare_decimal = quantity_form(TypeForm::builtin(BuiltinType::Decimal, SPAN));
        assert!(resolve_form(&names, &bare_decimal).is_err());
        let bare_rational = quantity_form(TypeForm::builtin(BuiltinType::Rational, SPAN));
        assert!(resolve_form(&names, &bare_rational).is_err());
        let text = quantity_form(
            TypeForm::builtin(BuiltinType::Text, SPAN)
                .with_bounds(["0", "8", "nfc"].into_iter().map(str::to_owned).collect()),
        );
        assert!(resolve_form(&names, &text).is_err());
        let record = quantity_form(TypeForm::name("Record", SPAN));
        assert!(resolve_form(&names, &record).is_err());
        let inverted = quantity_form(
            TypeForm::builtin(BuiltinType::Int, SPAN)
                .with_bounds(vec!["2".to_owned(), "-2".to_owned()]),
        );
        assert!(matches!(
            resolve_form(&names, &inverted)
                .expect_err("an inverted interval refuses")
                .fault,
            TypeFormFault::EmptyInterval
        ));
        let reversed_scale = quantity_form(
            TypeForm::builtin(BuiltinType::Decimal, SPAN).with_bounds(
                ["0", "1", "4", "2", "exact"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            ),
        );
        assert!(matches!(
            resolve_form(&names, &reversed_scale)
                .expect_err("a reversed scale interval refuses")
                .fault,
            TypeFormFault::MalformedDecimal
        ));
    }

    /// QSpec FR-149-AC-27: `Rational[-3, 7; 1, 12]` is well formed and its
    /// membership reads the reduced numerator and denominator; each
    /// malformed bound pair refuses `ill_typed` at the type, bare and as the
    /// carrier of `Quantity<m, C>`.
    #[trace("QSpec-TC-981", "QSpec-FR-149-AC-27")]
    #[test]
    fn a_rational_type_is_well_formed_exactly_when_its_bounds_are_ordered_and_positive() {
        let rational = |bounds: &[&str]| builtin(BuiltinType::Rational, bounds);
        let ValueType::Rational(ref domain) =
            resolve(&rational(&["-3", "7", "1", "12"])).expect("the type is well formed")
        else {
            panic!("a rational type");
        };
        let value = |numerator: i64, denominator: i64| {
            quire_exact::Rational::new(Integer::from(numerator), Integer::from(denominator))
                .expect("a nonzero denominator")
        };
        for (numerator, denominator) in [(1, 4), (-3, 1), (0, 1), (7, 12)] {
            assert!(
                domain.contains(&value(numerator, denominator)),
                "{numerator}/{denominator} is a member"
            );
        }
        for (numerator, denominator) in [(8, 1), (-4, 1), (1, 13), (127, 5000)] {
            assert!(
                !domain.contains(&value(numerator, denominator)),
                "{numerator}/{denominator} is not a member"
            );
        }

        let names = OneUnit(UnitId::declared(
            quire_exact::NodeKey::new(
                "fixture",
                vec![quire_exact::PathSegment::Text("unit".to_owned())],
                1,
            )
            .expect("a fixture node key is well formed"),
        ));
        for bounds in [
            ["8", "7", "1", "12"],
            ["0", "1", "0", "12"],
            ["0", "1", "-1", "12"],
            ["0", "1", "13", "12"],
        ] {
            assert_mismatch(&rational(&bounds));
            let refusal = resolve_form(&names, &quantity_form(rational(&bounds)))
                .expect_err("a malformed carrier refuses")
                .into_refusal(&root_location());
            assert!(
                matches!(
                    refusal.cause,
                    CheckCause::IllTyped(IllTypedCause::TypeMismatch)
                ),
                "Quantity<m, Rational{bounds:?}> refused with {:?}",
                refusal.cause
            );
        }
    }

    /// Inverted, missing and non-numeric bounds are refused as a type
    /// mismatch, per builtin.
    #[test]
    fn malformed_bounds_are_refused() {
        assert_mismatch(&builtin(BuiltinType::Int, &["9", "0"]));
        assert_mismatch(&builtin(BuiltinType::Int, &["0"]));
        assert_mismatch(&builtin(BuiltinType::Int, &["zero", "9"]));
        assert_mismatch(&builtin(BuiltinType::Rational, &["0", "1", "0", "5"]));
        assert_mismatch(&builtin(BuiltinType::Decimal, &["0", "9", "3", "1"]));
        assert_mismatch(&builtin(BuiltinType::Text, &["5", "1", "nfc"]));
        assert_mismatch(&builtin(BuiltinType::Text, &["-1", "5"]));
        assert_mismatch(
            &TypeForm::new(TypeFormHead::Population, SPAN)
                .with_bounds(vec!["1".to_owned(), "2".to_owned()]),
        );
    }

    /// FR-333-AC-1: an absent collection bound resolves to `bound: None` for
    /// every kind, and a present bound is a different type; `[3, 2]` refuses
    /// `type-mismatch` at the form's span.
    #[trace("TC-843", "FR-333-AC-1", "FR-333-AC-3")]
    #[test]
    fn an_absent_collection_bound_resolves_to_none() {
        let collection = |kind: CollectionKind, bounds: &[&str]| {
            TypeForm::collection(kind, SPAN)
                .with_arguments(vec![TypeForm::builtin(BuiltinType::Int, SPAN)
                    .with_bounds(vec!["0".to_owned(), "9".to_owned()])])
                .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
        };
        for kind in [
            CollectionKind::Sequence,
            CollectionKind::Set,
            CollectionKind::Bag,
            CollectionKind::OrderedSet,
        ] {
            let resolved =
                resolve(&collection(kind, &[])).expect("an unbounded collection resolves");
            let ValueType::Collection(ref unbounded) = resolved else {
                panic!("a collection type");
            };
            assert_eq!(unbounded.kind(), kind);
            assert_eq!(unbounded.bound(), None);
        }
        let unbounded = resolve(&collection(CollectionKind::Set, &[])).unwrap();
        let bounded = resolve(&collection(CollectionKind::Set, &["0", "8"])).unwrap();
        let ValueType::Collection(ref inner) = bounded else {
            panic!("a collection type");
        };
        assert_eq!(
            inner.bound(),
            Some(CardinalityBound::new(0, 8).expect("a nonempty bound"))
        );
        assert_ne!(unbounded, bounded);
        let inverted = resolve_form(
            &empty_scope(),
            &collection(CollectionKind::Set, &["3", "2"]),
        )
        .expect_err("an inverted bound refuses");
        assert_eq!(inverted.fault, TypeFormFault::EmptyCardinality);
        assert_eq!(inverted.span, SPAN);
        let refusal = resolve(&collection(CollectionKind::Set, &["3", "2"]))
            .expect_err("an inverted bound refuses");
        assert!(matches!(
            refusal.cause,
            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
        ));
    }

    /// FR-333-AC-2: `Population<T>` resolves to `Population(None)` and
    /// `Population<T>[4]` to `Population(Some(4))`.
    #[trace("TC-843", "FR-333-AC-2")]
    #[test]
    fn an_absent_population_maximum_resolves_to_none() {
        let population = |bounds: &[&str]| {
            TypeForm::new(TypeFormHead::Population, SPAN)
                .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
        };
        assert_eq!(
            resolve(&population(&[])).expect("an unbounded population resolves"),
            ValueType::Population(None)
        );
        assert_eq!(
            resolve(&population(&["4"])).expect("a bounded population resolves"),
            ValueType::Population(Some(4))
        );
    }

    /// `Option`, a collection and `Reference` each take exactly one type
    /// argument; a collection also needs two cardinality bounds in order.
    #[test]
    fn argument_counts_and_cardinality_bounds_are_checked() {
        let integer = || TypeForm::builtin(BuiltinType::Integer, SPAN);
        assert_mismatch(&TypeForm::builtin(BuiltinType::Option, SPAN));
        assert_mismatch(
            &TypeForm::builtin(BuiltinType::Option, SPAN)
                .with_arguments(vec![integer(), integer()]),
        );
        let sequence = |arguments: Vec<TypeForm>, bounds: &[&str]| {
            TypeForm::collection(CollectionKind::Sequence, SPAN)
                .with_arguments(arguments)
                .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
        };
        assert_mismatch(&sequence(Vec::new(), &["0", "3"]));
        assert_mismatch(&sequence(vec![integer()], &["0"]));
        assert_mismatch(&sequence(vec![integer()], &["3", "1"]));
        assert!(resolve(&sequence(vec![integer()], &["0", "3"])).is_ok());
        assert_mismatch(&TypeForm::builtin(BuiltinType::Reference, SPAN));
        assert_mismatch(
            &TypeForm::builtin(BuiltinType::Reference, SPAN).with_arguments(vec![integer()]),
        );
    }

    /// `Reference<T>` names a declared object type; an undeclared one is a
    /// missing name.
    #[test]
    fn a_reference_to_an_undeclared_object_type_is_a_missing_name() {
        let reference = |name: &str| {
            TypeForm::builtin(BuiltinType::Reference, SPAN)
                .with_arguments(vec![TypeForm::name(name, SPAN)])
        };
        let scope = scope_with(object_type("M::A"), Vec::new());
        assert_eq!(
            resolve_in(&scope, &reference("M::A")).expect("M::A is declared"),
            ValueType::Reference(quire_exact::EffectiveId::new(
                None,
                quire_exact::OriginalDeclarationKey::new("fixture", "o7")
            ))
        );
        let refusal = resolve_in(&scope, &reference("M::B")).unwrap_err();
        assert!(
            matches!(refusal.cause, CheckCause::MissingQualifiedName(name) if name.name == "M::B")
        );
    }

    /// A name bound by both an alias and an object type is ambiguous; a
    /// name bound by nothing is missing.
    #[test]
    fn names_resolve_uniquely_or_are_refused() {
        let scope = scope_with(object_type("X"), vec![("X".to_owned(), ValueType::Boolean)]);
        let refusal = resolve_in(&scope, &TypeForm::name("X", SPAN)).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::AmbiguousName { name, .. } if name == "X"));
        let refusal = resolve(&TypeForm::name("Y", SPAN)).unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::MissingName(name) if name == "Y"));
    }

    /// `Population<T>[N]` has its own head, so a user type named
    /// `Population` resolves as that user type, not as a population.
    #[test]
    fn a_user_type_named_population_is_not_shadowed() {
        let scope = scope_with(
            TypeEnvironment::default(),
            vec![("Population".to_owned(), ValueType::Boolean)],
        );
        assert_eq!(
            resolve_in(&scope, &TypeForm::name("Population", SPAN)).expect("the alias resolves"),
            ValueType::Boolean
        );
        assert_eq!(
            resolve_in(
                &scope,
                &TypeForm::new(TypeFormHead::Population, SPAN).with_bounds(vec!["3".to_owned()])
            )
            .expect("a population type resolves"),
            ValueType::Population(Some(3))
        );
    }
}
