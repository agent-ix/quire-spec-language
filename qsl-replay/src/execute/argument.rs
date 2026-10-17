// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098: a replay argument as a kernel value of its parameter's declared
//! type.
//!
//! A [`WitnessValue`] tree is walked against the recompiled package's type
//! environment on an explicit heap stack, never the native stack in
//! proportion to its depth (ADR-030 D-1), and each node is built through the
//! kernel's value constructors. A value the declared type does not admit, in
//! shape, declaration, member, arity, range, length, scale or cardinality,
//! refuses `WrongValueKind` naming the parameter's position, before any
//! call and with no charge: the conversion runs over its own unlimited meter.
//! The request's accounting limits bound it instead: a decimal quantity's
//! scale expansion is checked against `scale_expansion`, and the caller
//! checks the converted occurrence and node counts against
//! `value_occurrences` and `work_units`.
//!
//! FR-277/FR-098: every helper event is charged to the run's conversion
//! budget H *before* its work, and the original `Cancel` is polled at each.
//! The conversion events, one unit each, are: `witness-preflight` per witness
//! node (preorder, before its local shape is read); `descriptor-query` per
//! declaration header (a record's, tuple's, union's or enum's, with its
//! conformance to the slot) and per field, position or enum member then read; and
//! `witness-materialization` per converted kernel node (a leaf before it is
//! built, an option, record, union or collection after its children). Membership of
//! an option payload or collection element is QSV's, spending H event by
//! event. The semantic counters stay per argument and separate: converted
//! nodes against `work_units` and occurrences against `value_occurrences`.

use qsl_foundation::diagnostic::InternalFault;
use qsl_package::CheckedPackage;
use qsl_semantics::check::NominalNode;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::DomainPackageRef;
use qsl_semantics::model::observation::population_universe_for;
use qsl_semantics::value::enumeration::mint_variant_id;
use qsl_semantics::value::supplied::{
    AttemptedCount, SuppliedCause, SuppliedContext, SuppliedEvent, SuppliedFailure, SuppliedLimit,
    SuppliedLimitKind, SuppliedLocus, SuppliedPhase, SuppliedRun, SuppliedStop,
};
use quire_exact::{
    admit_text, form, from_admitted_slots, CollectionKind, CollectionType, ConstructionRefusal,
    Decimal, EffectiveId, EnumMember, FieldValue, IeeeValue, IeeeWidth, Integer, Meter, NodeKey,
    NumericMembershipFailure, ObjectId, ObjectReference, OptionValue, Outcome, Presence, Quantity,
    Rational, ScalarLimits, TextPayload, Timestamp, UnionMember, UnionValue, UniverseId, Uuid,
    Value, ValueType,
};
use quire_semantic_value::call::InputRefusal;
use quire_semantic_value::declaration::{CompositeDeclaration, CompositeShape, TypeEnvironment};
use std::collections::{BTreeMap, HashMap};

use super::ReplayRefusal;
use crate::witness::{QuantityMagnitude, WitnessSlot, WitnessValue};

/// The meter a conversion builds values under: no counter limits it.
pub(super) const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// Why a conversion stopped.
#[derive(Debug)]
pub(super) enum Stopped {
    /// The value is refused input, or a broken invariant.
    Refusal(Box<ReplayRefusal>),
    /// A supplied-value conversion phase stopped before any call (FR-277).
    Supplied(SuppliedFailure),
}

impl From<ReplayRefusal> for Stopped {
    fn from(refusal: ReplayRefusal) -> Self {
        Self::Refusal(Box::new(refusal))
    }
}

impl From<SuppliedFailure> for Stopped {
    fn from(failure: SuppliedFailure) -> Self {
        Self::Supplied(failure)
    }
}

impl From<Box<SuppliedStop>> for Stopped {
    fn from(stop: Box<SuppliedStop>) -> Self {
        Self::Supplied(SuppliedFailure::Stop(stop))
    }
}

/// The converted arguments.
#[derive(Debug)]
pub(super) struct Converted {
    /// One kernel value per argument, in parameter order.
    pub(super) values: Vec<Value>,
    /// Every reference the arguments hold, as `(parameter position,
    /// reference)`, in parameter order and, within an argument, in the
    /// conversion's walk order.
    pub(super) references: Vec<(usize, ObjectReference)>,
}

/// An enum declaration of the checked graph: its node key and its members.
pub(super) struct EnumDeclaration {
    key: NodeKey,
    members: Vec<String>,
}

impl EnumDeclaration {
    /// The declared member identifiers.
    pub(super) fn members(&self) -> &[String] {
        &self.members
    }
}

/// Every enum declaration of `package`: the enum declaration nodes of its
/// checked graph and the model variant types of its scope. A variant type is
/// keyed by its checked node (FR-360 item 11), and its cases are in the
/// ascending UTF-8 byte order of their names.
pub(super) fn enum_declarations(package: &CheckedPackage) -> Vec<EnumDeclaration> {
    let declared =
        package
            .graph()
            .semantic_graph()
            .nodes()
            .filter_map(|node| match node.nominal() {
                Some(NominalNode::EnumDeclaration(preimage)) => Some(EnumDeclaration {
                    key: node.key(),
                    members: preimage.members().to_vec(),
                }),
                _ => None,
            });
    let variant_types = package
        .graph()
        .scope()
        .variant_types()
        .iter()
        .map(|variant_type| EnumDeclaration {
            key: variant_type.node(),
            members: variant_type
                .cases()
                .iter()
                .map(|case| case.name().to_owned())
                .collect(),
        });
    declared.chain(variant_types).collect()
}

/// Whether `candidate`'s members are exactly `shape`'s variants.
fn declares_shape(candidate: &EnumDeclaration, shape: &quire_exact::EnumShape) -> bool {
    candidate.members.len() == shape.variants().count()
        && candidate
            .members
            .iter()
            .map(|case| mint_variant_id(candidate.key, case))
            .eq(shape.variants())
}

/// The declaration whose members are exactly `shape`'s variants.
pub(super) fn declaration_of<'e>(
    enums: &'e [EnumDeclaration],
    shape: &quire_exact::EnumShape,
) -> Option<&'e EnumDeclaration> {
    enums
        .iter()
        .find(|candidate| declares_shape(candidate, shape))
}

/// Whether a node of `declared` converts to its value at once, with no
/// children to assemble after.
fn is_leaf(declared: &ValueType) -> bool {
    !matches!(
        declared,
        ValueType::Option(_) | ValueType::Composite(_) | ValueType::Collection(_)
    )
}

/// One slot's state while a record is assembled.
#[derive(Clone, Copy)]
enum Slot {
    Present,
    Absent,
    Null,
}

/// One pending step of the conversion.
enum Step<'a> {
    Convert {
        value: &'a WitnessValue,
        declared: &'a ValueType,
    },
    Option {
        payload_type: &'a ValueType,
        present: bool,
        position: u64,
    },
    Composite {
        key: NodeKey,
        slots: Vec<Slot>,
        position: u64,
    },
    Union {
        member: &'a UnionMember,
        count: usize,
        position: u64,
    },
    Collection {
        declared: &'a CollectionType,
        count: usize,
        position: u64,
    },
}

/// The conversion of one package's arguments.
struct Converter<'a, 'r> {
    types: &'a TypeEnvironment,
    /// The run's budgets and original handle: one for every argument.
    run: &'r mut SuppliedRun,
    /// The preorder position of the witness node being converted.
    position: u64,
    /// The latest event charged, which names an input refusal's event.
    last_event: SuppliedEvent,
    /// Nodes converted in the current argument (FR-098's C).
    argument_nodes: u64,
    /// The checked composites by key bytes, built once: a record's wire
    /// declaration digest is resolved against it, never minted into a key.
    composites: HashMap<[u8; 32], &'a CompositeDeclaration>,
    enums: Vec<EnumDeclaration>,
    limits: &'a ScalarLimits,
    /// Where a reference's universe is read from.
    model: &'a ReferenceModel<'a>,
    /// The references converted so far, with their parameter positions.
    references: Vec<(usize, ObjectReference)>,
}

/// The selected models a reference argument's universe is read from
/// (FR-098: the universe of the declared `Reference<T>`'s object type).
pub(crate) struct ReferenceModel<'a> {
    selections: &'a [DomainPackageRef],
    packages: &'a BTreeMap<[u8; 32], Vec<u8>>,
    limits: ModelNormalizationLimits,
}

impl<'a> ReferenceModel<'a> {
    /// The models `package` selects, over the domain `packages` the request
    /// carries.
    pub(crate) fn new(
        package: &'a CheckedPackage,
        packages: &'a BTreeMap<[u8; 32], Vec<u8>>,
        limits: ModelNormalizationLimits,
    ) -> Self {
        Self {
            selections: package.graph().model_selections(),
            packages,
            limits,
        }
    }

    /// A call that selects no model: no reference type can be declared.
    #[cfg(test)]
    pub(super) fn unselected() -> Self {
        const NONE: &BTreeMap<[u8; 32], Vec<u8>> = &BTreeMap::new();
        Self {
            selections: &[],
            packages: NONE,
            limits: crate::spine::SpineLimits::default().model,
        }
    }

    fn universe(&self, object_type: EffectiveId) -> Result<UniverseId, Stopped> {
        population_universe_for(self.selections, self.packages, self.limits, object_type)
            .map_err(|failure| ReplayRefusal::Admission(failure).into())
    }
}

/// `values[i]` as a kernel value of `declared[i]`, for every parameter, under
/// the run's cumulative conversion budget H and its original handle.
pub(super) fn convert_arguments(
    package: &CheckedPackage,
    declared: &[ValueType],
    values: &[WitnessValue],
    limits: &ScalarLimits,
    run: &mut SuppliedRun,
    model: &ReferenceModel<'_>,
) -> Result<Converted, Stopped> {
    convert_arguments_in(
        package.graph().scope().types(),
        enum_declarations(package),
        declared,
        values,
        limits,
        run,
        model,
    )
}

/// [`convert_arguments`] over a type environment and its enum declarations
/// instead of a package's: no checked package can hold a union until its
/// FR-319 lowering exists, so a union's conversion is reached through here.
pub(super) fn convert_arguments_in(
    types: &TypeEnvironment,
    enums: Vec<EnumDeclaration>,
    declared: &[ValueType],
    values: &[WitnessValue],
    limits: &ScalarLimits,
    run: &mut SuppliedRun,
    model: &ReferenceModel<'_>,
) -> Result<Converted, Stopped> {
    let mut converter = Converter {
        types,
        run,
        position: 0,
        last_event: SuppliedEvent::WitnessPreflight,
        argument_nodes: 0,
        composites: types
            .composites()
            .map(|composite| (*composite.key().as_bytes(), composite))
            .collect(),
        enums,
        limits,
        model,
        references: Vec::new(),
    };
    let mut converted = Vec::with_capacity(values.len());
    for (parameter, (value, declared)) in values.iter().zip(declared).enumerate() {
        converter.position = 0;
        converter.argument_nodes = 0;
        let value = converter.convert(parameter, value, declared)?;
        converter.check_occurrences(parameter, &value)?;
        converted.push(value);
    }
    Ok(Converted {
        values: converted,
        references: converter.references,
    })
}

fn invariant(what: &'static str) -> Stopped {
    ReplayRefusal::Fault(InternalFault::new("replay", what)).into()
}

/// An integer leaf: the scalar `i128` form or an exact integer.
fn integer_of(value: &WitnessValue) -> Option<Integer> {
    match value {
        WitnessValue::Integer(number) => Some(Integer::from(*number)),
        WitnessValue::ExactInteger(number) => Some(number.clone()),
        _ => None,
    }
}

/// The refusal of a native payload the kernel's canonical constructor
/// refused, at the argument `parameter`.
fn native_payload(parameter: usize, refusal: ConstructionRefusal) -> Stopped {
    ReplayRefusal::NativePayload { parameter, refusal }.into()
}

/// A decimal scale as the kernel's `u32`.
fn scale_of(scale: &Integer) -> Option<u32> {
    scale.to_u64().and_then(|scale| u32::try_from(scale).ok())
}

impl<'a> Converter<'a, '_> {
    /// An input refusal of the current node, with the conversion phase's
    /// context: the argument, the latest event, the node's preorder position
    /// and the successful cumulative spend of H.
    fn wrong(&self, parameter: usize) -> Stopped {
        Box::new(SuppliedStop {
            context: SuppliedContext {
                phase: SuppliedPhase::Conversion,
                parameter: Some(parameter),
                event: Some(self.last_event),
                locus: Some(SuppliedLocus::Witness {
                    argument: parameter,
                    position: self.position,
                }),
                consumed: self.run.conversion_spent(),
            },
            cause: SuppliedCause::Input {
                refusal: InputRefusal::WrongValueKind { parameter },
                membership: None,
            },
        })
        .into()
    }

    /// Charge one conversion event to H before its work.
    fn charge(&mut self, parameter: usize, event: SuppliedEvent) -> Result<(), Stopped> {
        self.last_event = event;
        self.run.charge_conversion(
            parameter,
            event,
            SuppliedLocus::Witness {
                argument: parameter,
                position: self.position,
            },
        )?;
        Ok(())
    }

    /// Whether `value` is a member of `value_type` under the environment,
    /// one QSV event at a time against H and the run's original handle. A
    /// value that is not a member stops as the input refusal, a failed
    /// reservation as a storage stop, never as an undecided answer.
    fn admit(
        &mut self,
        parameter: usize,
        value_type: &ValueType,
        value: &Value,
    ) -> Result<(), Stopped> {
        self.run.admit(
            SuppliedPhase::Conversion,
            parameter,
            self.types,
            value_type,
            value,
        )?;
        Ok(())
    }

    /// The stop of a bounded numeric domain comparison at the current
    /// witness node: an operand over B, or the original handle's cancel.
    fn numeric_stop(&mut self, parameter: usize, failure: NumericMembershipFailure) -> Stopped {
        let locus = SuppliedLocus::Witness {
            argument: parameter,
            position: self.position,
        };
        match failure {
            NumericMembershipFailure::UnsupportedSize {
                operand,
                measured_bits,
                ..
            } => Stopped::Supplied(self.run.conversion_magnitude_stop(
                parameter,
                SuppliedEvent::WitnessMaterialization,
                locus,
                operand,
                measured_bits,
            )),
            NumericMembershipFailure::Cancelled(_) => {
                match self
                    .run
                    .check_cancelled(SuppliedPhase::Conversion, parameter)
                {
                    Err(failure) => Stopped::Supplied(failure),
                    Ok(()) => invariant("numeric-cancel-without-cancel"),
                }
            }
        }
    }

    /// Count one converted node against the argument's C bound
    /// (`work_units`) before converting it.
    fn count_node(&mut self, parameter: usize) -> Result<(), Stopped> {
        let consumed = self.argument_nodes;
        let attempted = consumed.checked_add(1);
        match attempted {
            Some(next) if next <= self.limits.work_units => {
                self.argument_nodes = next;
                Ok(())
            }
            Some(_) | None => Err(Box::new(SuppliedStop {
                context: SuppliedContext {
                    phase: SuppliedPhase::Conversion,
                    parameter: Some(parameter),
                    event: Some(SuppliedEvent::ConvertedNode),
                    locus: Some(SuppliedLocus::Witness {
                        argument: parameter,
                        position: self.position,
                    }),
                    consumed,
                },
                cause: SuppliedCause::Limit(SuppliedLimit {
                    limit_kind: SuppliedLimitKind::WorkUnits,
                    limit: self.limits.work_units,
                    next_charge: 1,
                    counter: attempted.map_or(AttemptedCount::Overflow, AttemptedCount::Count),
                    operand: None,
                }),
            })
            .into()),
        }
    }

    /// The converted argument's semantic occurrence count against
    /// `value_occurrences` (FR-098): the denied amount is the count reached.
    fn check_occurrences(&self, parameter: usize, value: &Value) -> Result<(), Stopped> {
        let occurrences = value.occ();
        if occurrences <= Integer::from(self.limits.value_occurrences) {
            return Ok(());
        }
        let reached = occurrences.to_u64();
        Err(Box::new(SuppliedStop {
            context: SuppliedContext {
                phase: SuppliedPhase::Conversion,
                parameter: Some(parameter),
                event: Some(SuppliedEvent::ValueOccurrence),
                locus: None,
                consumed: 0,
            },
            cause: SuppliedCause::Limit(SuppliedLimit {
                limit_kind: SuppliedLimitKind::ValueOccurrences,
                limit: self.limits.value_occurrences,
                next_charge: reached.unwrap_or(u64::MAX),
                counter: reached.map_or(AttemptedCount::Overflow, AttemptedCount::Count),
                operand: None,
            }),
        })
        .into())
    }

    /// Convert argument `parameter`.
    fn convert(
        &mut self,
        parameter: usize,
        root: &'a WitnessValue,
        declared: &'a ValueType,
    ) -> Result<Value, Stopped> {
        let mut pending = vec![Step::Convert {
            value: root,
            declared,
        }];
        let mut values: Vec<Value> = Vec::new();
        while let Some(step) = pending.pop() {
            match step {
                Step::Convert { value, declared } => {
                    self.position = self.argument_nodes;
                    self.charge(parameter, SuppliedEvent::WitnessPreflight)?;
                    self.count_node(parameter)?;
                    if is_leaf(declared) {
                        self.charge(parameter, SuppliedEvent::WitnessMaterialization)?;
                    }
                    self.node(parameter, value, declared, &mut pending, &mut values)?;
                }
                Step::Option {
                    payload_type,
                    present,
                    position,
                } => {
                    self.position = position;
                    self.charge(parameter, SuppliedEvent::WitnessMaterialization)?;
                    let payload = if present {
                        Some(values.pop().ok_or_else(|| invariant("converted-payload"))?)
                    } else {
                        None
                    };
                    values.push(match payload {
                        None => OptionValue::none(payload_type.clone()),
                        // FR-359 item 5: the payload is admitted by conformance
                        // under the environment, so a child of the payload's
                        // record keeps its own type. `OptionValue::present`
                        // is the exact test.
                        Some(payload) => {
                            // A union payload was converted against exactly
                            // this slot's union (a union conforms to nothing
                            // else), so its subtree needs no second walk per
                            // enclosing option: that walk would make a
                            // depth-n chain quadratic. The argument's final
                            // membership still walks it once.
                            if !matches!(payload, Value::Union(_)) {
                                self.admit(parameter, payload_type, &payload)?;
                            }
                            OptionValue::from_admitted(payload_type.clone(), Some(payload))
                        }
                    });
                }
                Step::Composite {
                    key,
                    slots,
                    position,
                } => {
                    self.position = position;
                    self.charge(parameter, SuppliedEvent::WitnessMaterialization)?;
                    let present = slots
                        .iter()
                        .filter(|slot| matches!(slot, Slot::Present))
                        .count();
                    let start = values
                        .len()
                        .checked_sub(present)
                        .ok_or_else(|| invariant("converted-slots"))?;
                    let mut built = values.split_off(start).into_iter();
                    let slots = slots
                        .into_iter()
                        .map(|slot| match slot {
                            Slot::Present => built.next().map(FieldValue::Present),
                            Slot::Absent => Some(FieldValue::Absent),
                            Slot::Null => Some(FieldValue::Null),
                        })
                        .collect::<Option<Box<[_]>>>()
                        .ok_or_else(|| invariant("converted-slots"))?;
                    values.push(from_admitted_slots(key, slots));
                }
                Step::Union {
                    member,
                    count,
                    position,
                } => {
                    self.position = position;
                    self.charge(parameter, SuppliedEvent::WitnessMaterialization)?;
                    let start = values
                        .len()
                        .checked_sub(count)
                        .ok_or_else(|| invariant("converted-payload"))?;
                    let payload = values.split_off(start);
                    values.push(UnionValue::from_admitted(member.clone(), payload));
                }
                Step::Collection {
                    declared,
                    count,
                    position,
                } => {
                    self.position = position;
                    self.charge(parameter, SuppliedEvent::WitnessMaterialization)?;
                    let start = values
                        .len()
                        .checked_sub(count)
                        .ok_or_else(|| invariant("converted-elements"))?;
                    let elements = values.split_off(start);
                    let mut meter = Meter::new(UNLIMITED);
                    // Each element is admitted by conformance under the
                    // environment (FR-359 item 5), then formed: forming
                    // checks the declared cardinality and collapses a
                    // duplicate the kernel's equality relates, which a set or
                    // ordered set refuses.
                    for element in &elements {
                        if !matches!(element, Value::Union(_)) {
                            self.admit(parameter, declared.element(), element)?;
                        }
                    }
                    match form(declared, elements, &mut meter) {
                        Outcome::Completed(Value::Collection(formed)) => {
                            let unique = matches!(
                                declared.kind(),
                                CollectionKind::Set | CollectionKind::OrderedSet
                            );
                            if unique && formed.elements().len() != count {
                                return Err(self.wrong(parameter));
                            }
                            values.push(Value::Collection(formed));
                        }
                        _ => return Err(self.wrong(parameter)),
                    }
                }
            }
        }
        match (values.pop(), values.is_empty()) {
            (Some(value), true) => Ok(value),
            _ => Err(invariant("one-converted-argument")),
        }
    }

    /// Convert one node: a leaf is pushed as a value; a composite pushes the
    /// steps that convert its parts and then assemble them.
    fn node(
        &mut self,
        parameter: usize,
        value: &'a WitnessValue,
        declared: &'a ValueType,
        pending: &mut Vec<Step<'a>>,
        values: &mut Vec<Value>,
    ) -> Result<(), Stopped> {
        match (declared, value) {
            (ValueType::Boolean, WitnessValue::Boolean(flag)) => {
                values.push(Value::Boolean(*flag));
            }
            (ValueType::Integer, _) => {
                values.push(Value::Integer(
                    integer_of(value).ok_or_else(|| self.wrong(parameter))?,
                ));
            }
            // The declared domain of an Integer or Rational leaf is compared
            // under the caller's magnitude bound B (FR-277), never
            // unbounded. An argument's root leaf is decided by supplied
            // admission, which stops an operand over B on its charged value
            // visit; a leaf inside a record or tuple is compared here.
            (ValueType::Int(interval), _) => {
                let number = integer_of(value).ok_or_else(|| self.wrong(parameter))?;
                if self.position != 0 {
                    let bits = self.run.numeric_magnitude_bits();
                    let cancel = self.run.cancel().clone();
                    let inside = interval
                        .try_contains_bounded(&number, bits, &cancel)
                        .map_err(|failure| self.numeric_stop(parameter, failure))?;
                    if !inside {
                        return Err(self.wrong(parameter));
                    }
                }
                values.push(Value::Integer(number));
            }
            (
                ValueType::Rational(domain),
                WitnessValue::Rational {
                    numerator,
                    denominator,
                },
            ) => {
                let rational = Rational::new(numerator.clone(), denominator.clone())
                    .map_err(|_| self.wrong(parameter))?;
                if self.position != 0 {
                    let bits = self.run.numeric_magnitude_bits();
                    let cancel = self.run.cancel().clone();
                    let inside = domain
                        .try_contains_bounded(&rational, bits, &cancel)
                        .map_err(|failure| self.numeric_stop(parameter, failure))?;
                    if !inside {
                        return Err(self.wrong(parameter));
                    }
                }
                values.push(Value::Rational(rational));
            }
            (ValueType::Decimal(decimal_type), WitnessValue::Decimal { coefficient, scale }) => {
                let decimal = Decimal::new(
                    coefficient.clone(),
                    scale_of(scale).ok_or_else(|| self.wrong(parameter))?,
                );
                if !decimal_type.contains(&decimal) {
                    return Err(self.wrong(parameter));
                }
                values.push(Value::Decimal(decimal));
            }
            (ValueType::Float(float_type), WitnessValue::Float32(bits)) => {
                if float_type.width() != IeeeWidth::Binary32 {
                    return Err(self.wrong(parameter));
                }
                values.push(Value::Float(IeeeValue::binary32(*bits)));
            }
            (ValueType::Float(float_type), WitnessValue::Float64(bits)) => {
                if float_type.width() != IeeeWidth::Binary64 {
                    return Err(self.wrong(parameter));
                }
                values.push(Value::Float(IeeeValue::binary64(*bits)));
            }
            (
                ValueType::Quantity(unit),
                WitnessValue::Quantity {
                    magnitude,
                    unit: wire,
                },
            ) => {
                if wire != unit.as_bytes() {
                    return Err(self.wrong(parameter));
                }
                let magnitude = self.magnitude(parameter, magnitude)?;
                values.push(Value::Quantity(Quantity::new(magnitude, *unit)));
            }
            // FR-056: the kernel's canonical constructor decides the payload
            // text; its typed refusal is the cause beneath
            // `invalid_runtime_input`/`invalid-value`.
            (ValueType::Uuid, WitnessValue::Uuid(text)) => {
                let bound = Uuid::from_canonical_text(text)
                    .map_err(|refusal| native_payload(parameter, refusal))?;
                values.push(Value::Uuid(bound));
            }
            (ValueType::Timestamp, WitnessValue::Timestamp(text)) => {
                let bound = Timestamp::from_canonical_text(text)
                    .map_err(|refusal| native_payload(parameter, refusal))?;
                values.push(Value::Timestamp(bound));
            }
            // A String is any Unicode scalar sequence, kept exactly: no payload
            // refusal exists (FR-056). It fits only a String slot.
            (ValueType::String, WitnessValue::String(text)) => {
                values.push(Value::String(text.clone()));
            }
            (ValueType::Text(text_type), WitnessValue::Text(text)) => {
                let payload =
                    TextPayload::from_utf8(text.as_bytes()).map_err(|_| self.wrong(parameter))?;
                let mut meter = Meter::new(UNLIMITED);
                match admit_text(&payload, text_type, &mut meter) {
                    Outcome::Completed(admitted) => values.push(Value::Text(admitted)),
                    _ => return Err(self.wrong(parameter)),
                }
            }
            (
                ValueType::Enum(shape),
                WitnessValue::Enum {
                    declaration,
                    member,
                },
            ) => {
                let declaration_key = declaration.as_bytes();
                self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                let index = self
                    .enums
                    .iter()
                    .position(|candidate| {
                        declares_shape(candidate, shape)
                            && candidate.key.as_bytes() == declaration_key
                    })
                    .ok_or_else(|| self.wrong(parameter))?;
                self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                let owner = self.enums.get(index).ok_or_else(|| self.wrong(parameter))?;
                if !owner.members.contains(member) {
                    return Err(self.wrong(parameter));
                }
                let variant = mint_variant_id(owner.key, member);
                let rank = shape.rank(variant).ok_or_else(|| self.wrong(parameter))?;
                values.push(Value::Enum(EnumMember::new(variant, rank)));
            }
            (
                ValueType::Reference(object_type),
                WitnessValue::Reference {
                    universe,
                    object_type: wire_type,
                    identity,
                },
            ) => {
                // FR-098: value shape (matched above), then universe
                // correspondence, then conformance, as FR-100 orders them.
                let required = self.model.universe(*object_type)?;
                if required.as_bytes() != universe {
                    return Err(ReplayRefusal::ForeignReference {
                        position: parameter,
                        required: *required.as_bytes(),
                        supplied: *universe,
                    }
                    .into());
                }
                // The supplied type is the model's object type of that id, if
                // the model has one; it is never minted from the wire digest.
                let supplied_type = self
                    .types
                    .object_types()
                    .map(|declaration| declaration.key())
                    .find(|key| key.as_bytes() == wire_type);
                let Some(supplied_type) = supplied_type else {
                    return Err(self.wrong(parameter));
                };
                if !self.types.conforms(supplied_type, *object_type) {
                    return Err(self.wrong(parameter));
                }
                let identity =
                    String::from_utf8(identity.clone()).map_err(|_| self.wrong(parameter))?;
                let object = ObjectId::new(identity).map_err(|_| self.wrong(parameter))?;
                let reference =
                    ObjectReference::new(UniverseId::from_digest(*universe), supplied_type, object);
                self.references.push((parameter, reference.clone()));
                values.push(Value::Reference(reference));
            }
            (ValueType::Option(payload_type), WitnessValue::Option(inner)) => {
                pending.push(Step::Option {
                    payload_type,
                    present: inner.is_some(),
                    position: self.position,
                });
                if let Some(inner) = inner {
                    pending.push(Step::Convert {
                        value: inner,
                        declared: payload_type,
                    });
                }
            }
            (
                ValueType::Composite(slot_key),
                WitnessValue::Record {
                    declaration,
                    fields,
                },
            ) => {
                let types: &'a TypeEnvironment = self.types;
                // FR-359 item 7: the record is decoded by the declaration its
                // own `declaration` names, and its fields read by name against
                // that declaration's fields, not the slot's. It is then
                // admitted at the slot by conformance.
                // The wire digest is resolved by lookup against the already
                // checked composites (FR-087-AC-6), never minted into a key.
                self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                let record = *self
                    .composites
                    .get(declaration.as_bytes())
                    .ok_or_else(|| self.wrong(parameter))?;
                let key = record.key();
                if !types.composite_conforms(key, *slot_key) {
                    return Err(self.wrong(parameter));
                }
                let CompositeShape::Record(declared_fields) = record.shape() else {
                    return Err(self.wrong(parameter));
                };
                if fields.len() != declared_fields.len() {
                    return Err(self.wrong(parameter));
                }
                let mut slots = Vec::with_capacity(fields.len());
                let mut present = Vec::new();
                for (field, declared_field) in fields.iter().zip(declared_fields) {
                    self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                    if field.name != declared_field.name() {
                        return Err(self.wrong(parameter));
                    }
                    let optional = declared_field.presence() == Presence::Optional;
                    match &field.slot {
                        WitnessSlot::Present(inner) => {
                            slots.push(Slot::Present);
                            present.push(Step::Convert {
                                value: inner,
                                declared: declared_field.value_type(),
                            });
                        }
                        WitnessSlot::Absent if optional => slots.push(Slot::Absent),
                        WitnessSlot::Null if optional => slots.push(Slot::Null),
                        WitnessSlot::Absent | WitnessSlot::Null => {
                            return Err(self.wrong(parameter))
                        }
                    }
                }
                pending.push(Step::Composite {
                    key,
                    slots,
                    position: self.position,
                });
                pending.extend(present.into_iter().rev());
            }
            (
                ValueType::Composite(key),
                WitnessValue::Tuple {
                    declaration,
                    components,
                },
            ) => {
                let types: &'a TypeEnvironment = self.types;
                self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                let tuple = types.composite(*key).ok_or_else(|| self.wrong(parameter))?;
                let CompositeShape::Tuple(positions) = tuple.shape() else {
                    return Err(self.wrong(parameter));
                };
                if declaration.as_bytes() != key.as_bytes() || components.len() != positions.len() {
                    return Err(self.wrong(parameter));
                }
                for _ in positions {
                    self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                }
                pending.push(Step::Composite {
                    key: *key,
                    slots: vec![Slot::Present; components.len()],
                    position: self.position,
                });
                pending.extend(
                    components
                        .iter()
                        .zip(positions)
                        .rev()
                        .map(|(value, declared)| Step::Convert { value, declared }),
                );
            }
            // FR-321: a union value is decoded by the union its slot declares,
            // whose settled key the wire declaration must name, and by the
            // member the wire names among that union's sealed members. A
            // union takes no part in generalization, so no other declaration
            // is admitted at the slot.
            (
                ValueType::Composite(slot_key),
                WitnessValue::Union {
                    declaration,
                    member,
                    components,
                },
            ) => {
                let types: &'a TypeEnvironment = self.types;
                self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                let settled = types
                    .union_key(*slot_key)
                    .ok_or_else(|| self.wrong(parameter))?;
                if declaration.as_bytes() != settled.as_bytes() {
                    return Err(self.wrong(parameter));
                }
                let (_, declared) = types
                    .union_member_named(*slot_key, member)
                    .ok_or_else(|| self.wrong(parameter))?;
                let binding = declared.member().ok_or_else(|| self.wrong(parameter))?;
                let positions = declared.positions();
                if components.len() != positions.len() {
                    return Err(self.wrong(parameter));
                }
                for _ in positions {
                    self.charge(parameter, SuppliedEvent::DescriptorQuery)?;
                }
                pending.push(Step::Union {
                    member: binding,
                    count: components.len(),
                    position: self.position,
                });
                pending.extend(
                    components
                        .iter()
                        .zip(positions)
                        .rev()
                        .map(|(value, declared)| Step::Convert { value, declared }),
                );
            }
            (ValueType::Collection(declared), _) => {
                let (kind, elements) = match value {
                    WitnessValue::Sequence(elements) => (CollectionKind::Sequence, elements),
                    WitnessValue::OrderedSet(elements) => (CollectionKind::OrderedSet, elements),
                    WitnessValue::Set(elements) => (CollectionKind::Set, elements),
                    WitnessValue::Bag(elements) => (CollectionKind::Bag, elements),
                    _ => return Err(self.wrong(parameter)),
                };
                if declared.kind() != kind {
                    return Err(self.wrong(parameter));
                }
                pending.push(Step::Collection {
                    declared,
                    count: elements.len(),
                    position: self.position,
                });
                pending.extend(elements.iter().rev().map(|value| Step::Convert {
                    value,
                    declared: declared.element(),
                }));
            }
            // A declared type no witness value is a value of (a population)
            // refuses; so does every pairing of a declared type with a value
            // of another form.
            _ => return Err(self.wrong(parameter)),
        }
        Ok(())
    }

    /// A quantity's magnitude as the kernel's exact rational. A decimal
    /// magnitude `c`/`10^s` expands a power of ten, so its scale is checked
    /// against the request's `scale_expansion` limit first.
    fn magnitude(
        &self,
        parameter: usize,
        magnitude: &QuantityMagnitude,
    ) -> Result<Rational, Stopped> {
        match magnitude {
            QuantityMagnitude::Rational {
                numerator,
                denominator,
            } => Rational::new(numerator.clone(), denominator.clone())
                .map_err(|_| self.wrong(parameter)),
            QuantityMagnitude::Decimal { coefficient, scale } => {
                let exponent = scale.to_u64().ok_or_else(|| self.wrong(parameter))?;
                if exponent > self.limits.scale_expansion {
                    return Err(Box::new(SuppliedStop {
                        context: SuppliedContext {
                            phase: SuppliedPhase::Conversion,
                            parameter: Some(parameter),
                            event: Some(SuppliedEvent::WitnessMaterialization),
                            locus: Some(SuppliedLocus::Witness {
                                argument: parameter,
                                position: self.position,
                            }),
                            consumed: 0,
                        },
                        cause: SuppliedCause::Limit(SuppliedLimit {
                            limit_kind: SuppliedLimitKind::ScaleExpansion,
                            limit: self.limits.scale_expansion,
                            next_charge: exponent,
                            counter: AttemptedCount::Count(exponent),
                            operand: None,
                        }),
                    })
                    .into());
                }
                Rational::new(coefficient.clone(), Integer::power_of_ten(exponent))
                    .map_err(|_| self.wrong(parameter))
            }
        }
    }
}
