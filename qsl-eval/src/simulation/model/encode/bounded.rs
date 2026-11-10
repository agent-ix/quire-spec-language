// SPDX-License-Identifier: AGPL-3.0-or-later
//! Admission-aware storage for the shared borrowed model projection.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ops::Range;

use qsl_foundation::{IdentityLimits, Setting};
use qsl_semantics::model::state::{ModelState, StateField, StateModel, StateObject};
use quire_canonical::{Error, Limits, Sink, Writer};
use quire_checked::key::DeclarationKey;
use quire_exact::{
    Cancel, Integer, IntegerDecimalBudget, IntegerDecimalBudgetError, NodeKey, ReservationDenied,
    Value, VariantId,
};
use quire_semantic_value::declaration::{CompositeDeclaration, TypeEnvironment};

use super::super::{ModelKeyBudget, ModelKeyError};
use super::projection::{self, ArrayOrder, Scalar, StructuralSink, ValueSink};
use super::Typing;
use crate::simulation::EncodingRefusal;

struct Scope<'a> {
    budget: RefCell<&'a mut dyn ModelKeyBudget>,
    cancel: &'a Cancel,
    identity: IdentityLimits,
    refused: RefCell<Option<ModelKeyError>>,
}

impl Scope<'_> {
    fn poll(&self) -> Result<(), ModelKeyError> {
        self.cancel.poll();
        match self.cancel.tripped() {
            Some(cause) => Err(ModelKeyError::Cancelled(cause)),
            None => Ok(()),
        }
    }

    fn work(&self, units: u64) -> Result<(), ModelKeyError> {
        self.poll()?;
        self.budget
            .borrow_mut()
            .work(units)
            .map_err(|_| ModelKeyError::Denied)?;
        self.poll()
    }

    fn reserve<T>(&self, values: &mut Vec<T>, extra: usize) -> Result<(), ModelKeyError> {
        let needed = values
            .len()
            .checked_add(extra)
            .ok_or(ModelKeyError::SizeOverflow)?;
        if needed > values.capacity() {
            let bytes = needed
                .checked_mul(size_of::<T>())
                .ok_or(ModelKeyError::SizeOverflow)?;
            self.admit(bytes)?;
            values
                .try_reserve_exact(needed - values.len())
                .map_err(|_| EncodingRefusal::Allocation { requested: bytes })?;
        }
        Ok(())
    }

    fn admit(&self, bytes: usize) -> Result<(), ModelKeyError> {
        self.poll()?;
        self.budget
            .borrow_mut()
            .reserve(bytes)
            .map_err(|_| ModelKeyError::Denied)?;
        self.poll()
    }

    fn bytes(&self, size: usize) -> Result<(), ModelKeyError> {
        if size as u128 > u128::from(self.identity.input_bytes) {
            return Err(
                EncodingRefusal::Limit(qsl_foundation::diagnostic::LimitExceeded::new(
                    Setting::IdentityInputBytes,
                    self.identity.input_bytes,
                    size as u128,
                ))
                .into(),
            );
        }
        Ok(())
    }

    fn append(&self, output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), ModelKeyError> {
        self.bytes(
            output
                .len()
                .checked_add(bytes.len())
                .ok_or(ModelKeyError::SizeOverflow)?,
        )?;
        self.work(bytes.len() as u64)?;
        self.reserve(output, bytes.len())?;
        output.extend_from_slice(bytes);
        Ok(())
    }

    fn canonical(
        &self,
        encode: impl FnOnce(&mut Writer<'_, Output<'_, '_>>) -> Result<(), Error>,
    ) -> Result<Vec<u8>, ModelKeyError> {
        let mut output = Output {
            scope: self,
            bytes: Vec::new(),
        };
        let mut reservation = Provision(self);
        let mut writer = Writer::with_reservation(
            &mut output,
            Limits::new(self.identity.input_bytes),
            &mut reservation,
        );
        let result = encode(&mut writer).and_then(|()| writer.finish().map(|_| ()));
        self.poll()?;
        if let Some(error) = self.refused.borrow_mut().take() {
            return Err(error);
        }
        result.map_err(|error| match error {
            Error::Limit(_) | Error::Allocation { .. } => {
                ModelKeyError::Encoding(EncodingRefusal::of(&error, self.identity))
            }
            Error::ReservationDenied { .. } => ModelKeyError::Denied,
            _ => ModelKeyError::Unsupported("canonical leaf"),
        })?;
        Ok(output.bytes)
    }

    fn text(&self, source: &str) -> Result<Vec<u8>, ModelKeyError> {
        // JCS escaping is one bounded atomic scan; source bytes are admitted
        // before it starts and cancellation is polled again after completion.
        self.bytes(source.len())?;
        self.work(source.len() as u64)?;
        self.canonical(|writer| writer.string(source))
    }

    fn integer(&self, value: &Integer) -> Result<Vec<u8>, ModelKeyError> {
        let text = self.decimal(value)?;
        self.text(&text)
    }
    fn decimal(&self, value: &Integer) -> Result<String, ModelKeyError> {
        self.poll()?;
        self.budget
            .borrow_mut()
            .integer(value)
            .map_err(|_| ModelKeyError::Denied)?;
        let mut budget = DecimalProvision(self);
        let text = value
            .try_to_decimal_string_with_budget(
                usize::try_from(self.identity.input_bytes).unwrap_or(usize::MAX),
                &mut budget,
            )
            .map_err(|error| match error {
                IntegerDecimalBudgetError::Denied => self
                    .refused
                    .borrow_mut()
                    .take()
                    .unwrap_or(ModelKeyError::Denied),
                IntegerDecimalBudgetError::Decimal(
                    quire_exact::IntegerDecimalError::ByteLimit { required, .. },
                ) => ModelKeyError::Encoding(EncodingRefusal::Limit(
                    qsl_foundation::diagnostic::LimitExceeded::new(
                        Setting::IdentityInputBytes,
                        self.identity.input_bytes,
                        required as u128,
                    ),
                )),
                IntegerDecimalBudgetError::Decimal(
                    quire_exact::IntegerDecimalError::Allocation { requested },
                ) => ModelKeyError::Encoding(EncodingRefusal::Allocation { requested }),
                IntegerDecimalBudgetError::Decimal(
                    quire_exact::IntegerDecimalError::OutputSizeOverflow,
                ) => ModelKeyError::SizeOverflow,
            })?;
        Ok(text)
    }

    fn text_equal(&self, left: &str, right: &str) -> Result<bool, ModelKeyError> {
        self.work(1)?;
        if left.len() != right.len() {
            return Ok(false);
        }
        for (left, right) in left.bytes().zip(right.bytes()) {
            self.work(1)?;
            if left != right {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn node_equal(&self, left: &NodeKey, right: &NodeKey) -> Result<bool, ModelKeyError> {
        use quire_exact::{NodeKeyView, PathSegmentView};
        let head = |left: NodeKeyView<'_>, right: NodeKeyView<'_>| -> Result<bool, ModelKeyError> {
            self.work(1)?;
            Ok(left.ordinal() == right.ordinal()
                && self.text_equal(left.package(), right.package())?)
        };
        let (left, right) = (left.view(), right.view());
        if !head(left, right)? {
            return Ok(false);
        }
        let mut stack = Vec::new();
        self.reserve(&mut stack, 1)?;
        stack.push((left.path(), right.path()));
        while let Some((left, right)) = stack.last_mut() {
            self.work(1)?;
            match (left.next(), right.next()) {
                (Some(PathSegmentView::Text(left)), Some(PathSegmentView::Text(right))) => {
                    if !self.text_equal(left, right)? {
                        return Ok(false);
                    }
                }
                (Some(PathSegmentView::Node(left)), Some(PathSegmentView::Node(right))) => {
                    if !head(left, right)? {
                        return Ok(false);
                    }
                    self.reserve(&mut stack, 1)?;
                    stack.push((left.path(), right.path()));
                }
                (None, None) => {
                    stack.pop();
                }
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    fn declaration_equal(
        &self,
        left: &DeclarationKey,
        right: &DeclarationKey,
    ) -> Result<bool, ModelKeyError> {
        Ok(self.text_equal(&left.package, &right.package)?
            && self.text_equal(&left.node, &right.node)?)
    }

    fn refuse(&self, error: ModelKeyError) -> Error {
        *self.refused.borrow_mut() = Some(error);
        Error::ReservationDenied { requested: 0 }
    }
}

struct Provision<'s, 'a>(&'s Scope<'a>);
impl quire_canonical::Reservation for Provision<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
        self.0.admit(bytes).map_err(|error| {
            self.0.refuse(error);
            quire_canonical::ReservationDenied
        })
    }
}
struct DecimalProvision<'s, 'a>(&'s Scope<'a>);
impl quire_exact::Reservation for DecimalProvision<'_, '_> {
    fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
        self.0.admit(bytes).map_err(|error| {
            self.0.refuse(error);
            ReservationDenied
        })
    }
}
impl IntegerDecimalBudget for DecimalProvision<'_, '_> {
    fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
        self.0.work(units).map_err(|error| {
            self.0.refuse(error);
            ReservationDenied
        })
    }
}
struct Output<'s, 'a> {
    scope: &'s Scope<'a>,
    bytes: Vec<u8>,
}
impl Sink for Output<'_, '_> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        self.scope
            .append(&mut self.bytes, bytes)
            .map_err(|error| self.scope.refuse(error))
    }
}

struct Fragment {
    bytes: Vec<u8>,
    key: Option<Range<usize>>,
}
struct Member {
    name: Option<&'static str>,
    value: Fragment,
}
struct Frame {
    members: Vec<Member>,
    object: bool,
    order: ArrayOrder,
    name: Option<&'static str>,
}
struct Bounded<'s, 'a> {
    scope: &'s Scope<'a>,
    stack: Vec<Frame>,
    root: Option<Fragment>,
    produced: usize,
}
impl<'s, 'a> Bounded<'s, 'a> {
    fn new(scope: &'s Scope<'a>) -> Self {
        Self {
            scope,
            stack: Vec::new(),
            root: None,
            produced: 0,
        }
    }
    fn push(&mut self, value: Fragment) -> Result<(), ModelKeyError> {
        if let Some(frame) = self.stack.last_mut() {
            self.scope.reserve(&mut frame.members, 1)?;
            frame.members.push(Member {
                name: frame.name.take(),
                value,
            });
        } else {
            self.root = Some(value);
        }
        Ok(())
    }
    fn charge(&mut self, bytes: usize) -> Result<(), ModelKeyError> {
        let next = self
            .produced
            .checked_add(bytes)
            .ok_or(ModelKeyError::SizeOverflow)?;
        self.scope.bytes(next)?;
        self.produced = next;
        Ok(())
    }
    fn separator(&mut self) -> Result<(), ModelKeyError> {
        if self
            .stack
            .last()
            .is_some_and(|frame| !frame.object && !frame.members.is_empty())
        {
            self.charge(1)?;
        }
        Ok(())
    }
    fn leaf(&mut self, bytes: Vec<u8>) -> Result<(), ModelKeyError> {
        self.separator()?;
        self.charge(bytes.len())?;
        self.push(Fragment { bytes, key: None })
    }
    fn begin(&mut self, object: bool, order: ArrayOrder) -> Result<(), ModelKeyError> {
        self.separator()?;
        self.charge(2)?;
        self.scope.work(1)?;
        self.scope.reserve(&mut self.stack, 1)?;
        self.stack.push(Frame {
            members: Vec::new(),
            object,
            order,
            name: None,
        });
        Ok(())
    }
    fn end(&mut self) -> Result<(), ModelKeyError> {
        let mut frame = self
            .stack
            .pop()
            .ok_or(ModelKeyError::Unsupported("container protocol"))?;
        // Stable insertion ordering allocates no hidden sort scratch. Work and
        // cancellation are admitted before every comparison and movement.
        if frame.object || !matches!(frame.order, ArrayOrder::Retained) {
            for at in 1..frame.members.len() {
                let mut next = at;
                while next > 0 {
                    let left = &frame.members[next - 1];
                    let right = &frame.members[next];
                    let order = if frame.object {
                        self.scope.work(
                            (left
                                .name
                                .unwrap_or_default()
                                .len()
                                .min(right.name.unwrap_or_default().len())
                                + 1) as u64,
                        )?;
                        left.name.cmp(&right.name)
                    } else {
                        let key = |member: &Member| match &member.value.key {
                            Some(range) if matches!(frame.order, ArrayOrder::MapKeys) => {
                                range.clone()
                            }
                            _ => 0..member.value.bytes.len(),
                        };
                        let left = &left.value.bytes[key(left)];
                        let right = &right.value.bytes[key(right)];
                        self.scope.work((left.len().min(right.len()) + 1) as u64)?;
                        left.cmp(right)
                    };
                    if !order.is_gt() {
                        break;
                    }
                    self.scope.work(1)?;
                    frame.members.swap(next - 1, next);
                    next -= 1;
                }
            }
        }
        let mut bytes = Vec::new();
        let mut key = None;
        self.scope
            .append(&mut bytes, if frame.object { b"{" } else { b"[" })?;
        for (at, member) in frame.members.into_iter().enumerate() {
            if at != 0 {
                self.scope.append(&mut bytes, b",")?;
            }
            if let Some(name) = member.name {
                let text = self.scope.text(name)?;
                self.scope.append(&mut bytes, &text)?;
                self.scope.append(&mut bytes, b":")?;
                if name == "key" {
                    key = Some(bytes.len()..bytes.len() + member.value.bytes.len());
                }
            }
            self.scope.append(&mut bytes, &member.value.bytes)?;
        }
        self.scope
            .append(&mut bytes, if frame.object { b"}" } else { b"]" })?;
        self.push(Fragment { bytes, key })
    }
    fn formatted(&mut self, source: impl std::fmt::Display) -> Result<(), ModelKeyError> {
        use std::fmt::Write;
        struct Text<'s, 'a> {
            scope: &'s Scope<'a>,
            bytes: Vec<u8>,
            error: Option<ModelKeyError>,
        }
        impl std::fmt::Write for Text<'_, '_> {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                self.scope
                    .append(&mut self.bytes, text.as_bytes())
                    .map_err(|error| {
                        self.error = Some(error);
                        std::fmt::Error
                    })
            }
        }
        let mut text = Text {
            scope: self.scope,
            bytes: Vec::new(),
            error: None,
        };
        if write!(text, "{source}").is_err() {
            return Err(text
                .error
                .unwrap_or(ModelKeyError::Unsupported("scalar formatting")));
        }
        let text = std::str::from_utf8(&text.bytes)
            .map_err(|_| ModelKeyError::Unsupported("scalar UTF-8"))?;
        self.text(text)
    }
}

impl StructuralSink for Bounded<'_, '_> {
    type Error = ModelKeyError;
    fn begin_object(&mut self) -> Result<(), Self::Error> {
        self.begin(true, ArrayOrder::Retained)
    }
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error> {
        self.scope.work(1)?;
        let comma = usize::from(
            self.stack
                .last()
                .is_some_and(|frame| !frame.members.is_empty()),
        );
        self.charge(name.len() + 3 + comma)?;
        self.stack
            .last_mut()
            .ok_or(ModelKeyError::Unsupported("member protocol"))?
            .name = Some(name);
        Ok(())
    }
    fn end_object(&mut self) -> Result<(), Self::Error> {
        self.end()
    }
    fn text(&mut self, source: &str) -> Result<(), Self::Error> {
        self.leaf(self.scope.text(source)?)
    }
    fn node(&mut self, source: &NodeKey) -> Result<(), Self::Error> {
        qsl_foundation::node_wire::project_node_key(source, self)
    }
    fn effective(&mut self, source: &quire_exact::EffectiveId) -> Result<(), Self::Error> {
        projection::project_effective(self, source)
    }
    fn null(&mut self) -> Result<(), Self::Error> {
        self.scalar(Scalar::Null)
    }
}
impl qsl_foundation::node_wire::NodeIdentitySink for Bounded<'_, '_> {
    type Error = ModelKeyError;
    fn visit(&mut self) -> Result<(), Self::Error> {
        self.scope.work(1)
    }
    fn begin_object(&mut self) -> Result<(), Self::Error> {
        StructuralSink::begin_object(self)
    }
    fn name(&mut self, name: &'static str) -> Result<(), Self::Error> {
        StructuralSink::name(self, name)
    }
    fn ordinal(&mut self, ordinal: u64) -> Result<(), Self::Error> {
        let text = self.scope.decimal(&Integer::from(ordinal))?;
        self.leaf(text.into_bytes())
    }
    fn text(&mut self, text: &str) -> Result<(), Self::Error> {
        StructuralSink::text(self, text)
    }
    fn begin_array(&mut self) -> Result<(), Self::Error> {
        self.begin(false, ArrayOrder::Retained)
    }
    fn end_array(&mut self) -> Result<(), Self::Error> {
        self.end()
    }
    fn end_object(&mut self) -> Result<(), Self::Error> {
        self.end()
    }
    fn push_path<'a>(
        &mut self,
        stack: &mut Vec<quire_exact::NodePathIter<'a>>,
        path: quire_exact::NodePathIter<'a>,
    ) -> Result<(), Self::Error> {
        self.scope.reserve(stack, 1)?;
        stack.push(path);
        Ok(())
    }
}

impl ValueSink for Bounded<'_, '_> {
    fn visit(&mut self) -> Result<(), Self::Error> {
        self.scope.work(1)
    }
    fn scalar(&mut self, source: Scalar<'_>) -> Result<(), Self::Error> {
        match source {
            Scalar::Null => self.leaf(self.scope.canonical(|writer| writer.null())?),
            Scalar::Boolean(value) => self.leaf(self.scope.canonical(|writer| writer.bool(value))?),
            Scalar::Text(value) => self.text(value),
            Scalar::Integer(value) => self.leaf(self.scope.integer(value)?),
            Scalar::Hex { bits, digits } => {
                struct Hex(u64, usize);
                impl std::fmt::Display for Hex {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "{:0width$x}", self.0, width = self.1)
                    }
                }
                self.formatted(Hex(bits, digits))
            }
            Scalar::Scale(value) => {
                self.leaf(self.scope.integer(&Integer::from(u64::from(value)))?)
            }
            Scalar::Uuid(value) => self.formatted(value),
            Scalar::Timestamp(value) => {
                self.leaf(self.scope.integer(&Integer::from(value.nanoseconds()))?)
            }
            Scalar::Node(value) => self.node(value),
            Scalar::Effective(value) => self.effective(value),
            Scalar::Declaration(value) => {
                let mut nested = Bounded::new(self.scope);
                projection::project_declaration(&mut nested, value)?;
                let bytes = nested
                    .root
                    .ok_or(ModelKeyError::Unsupported("declaration identity"))?
                    .bytes;
                self.text(
                    std::str::from_utf8(&bytes)
                        .map_err(|_| ModelKeyError::Unsupported("declaration UTF-8"))?,
                )
            }
            Scalar::DeclarationKey(value) => {
                self.scalar(Scalar::Declaration(super::DeclarationRecord::of_key(value)))
            }
            Scalar::ModelReference {
                model,
                object_type,
                object,
                ..
            } => {
                let (effective, universe) = model
                    .reference_components_with_work(object_type, &mut |units| {
                        self.scope.work(units)
                    })?
                    .ok_or(ModelKeyError::Unsupported("unresolved object reference"))?;
                let universe = universe
                    .identity_view()
                    .map_err(|_| ModelKeyError::Unsupported("universe identity"))?;
                projection::project_reference_parts(
                    self,
                    |sink| universe.project(sink),
                    effective,
                    object,
                )
            }
            Scalar::PopulationName { population, .. } => {
                projection::project_population(self, population)
            }
            Scalar::UnitText(value) => {
                let mut nested = Bounded::new(self.scope);
                projection::project_unit(&mut nested, value)?;
                let bytes = nested
                    .root
                    .ok_or(ModelKeyError::Unsupported("unit identity"))?
                    .bytes;
                self.text(
                    std::str::from_utf8(&bytes)
                        .map_err(|_| ModelKeyError::Unsupported("unit UTF-8"))?,
                )
            }
            Scalar::Replacement(_) => Err(ModelKeyError::Unsupported("retained placeholder")),
            Scalar::Unencodable(what) => Err(ModelKeyError::Unsupported(what)),
        }
    }
    fn begin_array(&mut self, order: ArrayOrder) -> Result<(), Self::Error> {
        self.begin(false, order)
    }
    fn end_array(&mut self) -> Result<(), Self::Error> {
        self.end()
    }
    fn composite<'t>(
        &mut self,
        types: &'t TypeEnvironment,
        key: &NodeKey,
    ) -> Result<Option<&'t CompositeDeclaration>, Self::Error> {
        for declared in types.composites() {
            self.scope.work(1)?;
            // The owner key clone only retains its Arc. No identity tree is copied.
            if self.scope.node_equal(&declared.key(), key)? {
                return Ok(Some(declared));
            }
        }
        Ok(None)
    }
    fn variant(&mut self, left: &VariantId, right: &VariantId) -> Result<bool, Self::Error> {
        Ok(self.scope.text_equal(left.case(), right.case())?
            && self.scope.node_equal(left.owner(), right.owner())?)
    }
    fn fields(
        &mut self,
        _: &StateModel,
        _: &DeclarationKey,
    ) -> Result<Option<Vec<StateField>>, Self::Error> {
        Err(ModelKeyError::Unsupported("uncached model fields"))
    }
    fn cached_fields<'t>(
        &mut self,
        fields: &'t BTreeMap<DeclarationKey, Vec<StateField>>,
        key: &DeclarationKey,
    ) -> Result<Option<&'t [StateField]>, Self::Error> {
        for (candidate, fields) in fields {
            self.scope.work(1)?;
            if self.scope.declaration_equal(candidate, key)? {
                return Ok(Some(fields.as_slice()));
            }
        }
        Ok(None)
    }
    fn field<'v>(
        &mut self,
        object: &'v StateObject,
        key: &DeclarationKey,
    ) -> Result<Option<&'v Value>, Self::Error> {
        for (candidate, value) in &object.fields {
            self.scope.work(1)?;
            if self.scope.declaration_equal(candidate, key)? {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    fn declaration_equal(
        &mut self,
        left: &DeclarationKey,
        right: &DeclarationKey,
    ) -> Result<bool, Self::Error> {
        self.scope.declaration_equal(left, right)
    }
    fn clock_count(&mut self, value: &Integer) -> Result<Option<u64>, Self::Error> {
        self.scope.work(1)?;
        if value.magnitude_bits() > 64 {
            return Ok(None);
        }
        Ok(value.to_u64())
    }
}

pub(crate) fn model_bytes(
    model: &StateModel,
    state: &ModelState,
    typing: Typing<'_>,
    identity: IdentityLimits,
    budget: &mut dyn ModelKeyBudget,
    cancel: &Cancel,
) -> Result<Vec<u8>, ModelKeyError> {
    let scope = Scope {
        budget: RefCell::new(budget),
        cancel,
        identity,
        refused: RefCell::new(None),
    };
    let mut sink = Bounded::new(&scope);
    scope.work(1)?;
    projection::project_model_state(&mut sink, model, state, typing)?;
    scope.poll()?;
    sink.root
        .map(|value| value.bytes)
        .ok_or(ModelKeyError::Unsupported("model identity"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[derive(Default)]
    struct Admission {
        storage: usize,
        work: u64,
        storage_limit: usize,
        work_limit: u64,
        integers: usize,
        deny_integer: bool,
    }
    impl quire_exact::Reservation for Admission {
        fn reserve(&mut self, bytes: usize) -> Result<(), ReservationDenied> {
            let next = self.storage.checked_add(bytes).ok_or(ReservationDenied)?;
            if next > self.storage_limit {
                return Err(ReservationDenied);
            }
            self.storage = next;
            Ok(())
        }
    }
    impl IntegerDecimalBudget for Admission {
        fn work(&mut self, units: u64) -> Result<(), ReservationDenied> {
            let next = self.work.checked_add(units).ok_or(ReservationDenied)?;
            if next > self.work_limit {
                return Err(ReservationDenied);
            }
            self.work = next;
            Ok(())
        }
    }
    impl ModelKeyBudget for Admission {
        fn integer(&mut self, _: &Integer) -> Result<(), ReservationDenied> {
            self.integers += 1;
            if self.deny_integer {
                Err(ReservationDenied)
            } else {
                Ok(())
            }
        }
    }

    #[test]
    #[trace("TC-471", "FR-120-AC-4")]
    fn denied_model_projection_growth_leaves_actual_buffers_empty() {
        let cancel = Cancel::new();
        let mut budget = Admission {
            work_limit: u64::MAX,
            ..Admission::default()
        };
        let scope = Scope {
            budget: RefCell::new(&mut budget),
            cancel: &cancel,
            identity: IdentityLimits::default(),
            refused: RefCell::new(None),
        };
        let mut sink = Bounded::new(&scope);
        assert_eq!(sink.begin_object(), Err(ModelKeyError::Denied));
        assert_eq!(sink.stack.capacity(), 0);
        let mut output = Vec::new();
        assert_eq!(scope.append(&mut output, b"x"), Err(ModelKeyError::Denied));
        assert_eq!(output.capacity(), 0);
        drop(sink);
        drop(scope);
        assert_eq!(budget.storage, 0);

        let mut budget = Admission {
            storage_limit: usize::MAX,
            work_limit: u64::MAX,
            ..Admission::default()
        };
        let scope = Scope {
            budget: RefCell::new(&mut budget),
            cancel: &cancel,
            identity: IdentityLimits { input_bytes: 0 },
            refused: RefCell::new(None),
        };
        let mut sink = Bounded::new(&scope);
        assert!(matches!(
            sink.begin_object(),
            Err(ModelKeyError::Encoding(EncodingRefusal::Limit(_)))
        ));
        assert_eq!(sink.stack.capacity(), 0);
        drop(sink);
        drop(scope);
        assert_eq!(budget.storage, 0);
    }

    #[test]
    #[trace("TC-471", "FR-120-AC-4")]
    fn integer_representation_denial_precedes_decimal_work_and_storage() {
        let integer = "987654321098765432109876543210987654321098765432109876543210"
            .parse::<Integer>()
            .unwrap();
        let cancel = Cancel::new();
        let mut budget = Admission {
            storage_limit: usize::MAX,
            work_limit: u64::MAX,
            deny_integer: true,
            ..Admission::default()
        };
        let scope = Scope {
            budget: RefCell::new(&mut budget),
            cancel: &cancel,
            identity: IdentityLimits::default(),
            refused: RefCell::new(None),
        };
        assert_eq!(scope.integer(&integer), Err(ModelKeyError::Denied));
        drop(scope);
        assert_eq!(budget.integers, 1);
        assert_eq!(budget.storage, 0);
        assert_eq!(budget.work, 0);
    }
}
