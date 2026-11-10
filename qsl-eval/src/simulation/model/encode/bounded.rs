// SPDX-License-Identifier: AGPL-3.0-or-later
//! Admission-aware storage for the shared borrowed model projection.

use std::cell::RefCell;
use std::collections::BTreeMap;

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
            let target = values
                .capacity()
                .checked_mul(2)
                .map_or(needed, |doubled| needed.max(doubled));
            let bytes = target
                .checked_mul(size_of::<T>())
                .ok_or(ModelKeyError::SizeOverflow)?;
            self.admit(bytes)?;
            values
                .try_reserve_exact(target - values.len())
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
            _ => ModelKeyError::Unsupported("canonical value"),
        })?;
        Ok(output.bytes)
    }

    fn decimal(
        &self,
        value: &Integer,
        max_bytes: usize,
        offset: usize,
    ) -> Result<String, ModelKeyError> {
        self.poll()?;
        self.budget
            .borrow_mut()
            .integer(value)
            .map_err(|_| ModelKeyError::Denied)?;
        let mut budget = DecimalProvision(self);
        let text = value
            .try_to_decimal_string_with_budget(max_bytes, &mut budget)
            .map_err(|error| match error {
                IntegerDecimalBudgetError::Denied => self
                    .refused
                    .borrow_mut()
                    .take()
                    .unwrap_or(ModelKeyError::Denied),
                IntegerDecimalBudgetError::Decimal(
                    quire_exact::IntegerDecimalError::ByteLimit { required, .. },
                ) => {
                    let actual = required as u128 + offset as u128;
                    if actual > u128::from(self.identity.input_bytes) {
                        ModelKeyError::Encoding(EncodingRefusal::Limit(
                            qsl_foundation::diagnostic::LimitExceeded::new(
                                Setting::IdentityInputBytes,
                                self.identity.input_bytes,
                                actual,
                            ),
                        ))
                    } else {
                        ModelKeyError::SizeOverflow
                    }
                }
                IntegerDecimalBudgetError::Decimal(
                    quire_exact::IntegerDecimalError::Allocation { requested },
                ) => ModelKeyError::Encoding(EncodingRefusal::Allocation { requested }),
                IntegerDecimalBudgetError::Decimal(
                    quire_exact::IntegerDecimalError::OutputSizeOverflow,
                ) => ModelKeyError::SizeOverflow,
            })?;
        Ok(text)
    }

    fn string_length(&self, source: &str) -> Result<usize, ModelKeyError> {
        struct Count<'s, 'a> {
            scope: &'s Scope<'a>,
            bytes: usize,
        }
        impl Sink for Count<'_, '_> {
            fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
                self.scope
                    .work(1)
                    .map_err(|error| self.scope.refuse(error))?;
                self.bytes = self
                    .bytes
                    .checked_add(bytes.len())
                    .ok_or_else(|| self.scope.refuse(ModelKeyError::SizeOverflow))?;
                Ok(())
            }
        }
        self.work(source.len() as u64)?;
        let mut count = Count {
            scope: self,
            bytes: 0,
        };
        let mut reservation = Provision(self);
        let mut writer = Writer::with_reservation(
            &mut count,
            Limits::new(self.identity.input_bytes),
            &mut reservation,
        );
        let result = writer
            .string(source)
            .and_then(|()| writer.finish().map(|_| ()));
        self.poll()?;
        if let Some(error) = self.refused.borrow_mut().take() {
            return Err(error);
        }
        result.map_err(|error| match error {
            Error::Limit(_) | Error::Allocation { .. } => {
                ModelKeyError::Encoding(EncodingRefusal::of(&error, self.identity))
            }
            _ => ModelKeyError::Unsupported("canonical string measurement"),
        })?;
        Ok(count.bytes)
    }

    fn text_equal(&self, left: &str, right: &str) -> Result<bool, ModelKeyError> {
        self.work(1)?;
        if left.len() != right.len() {
            return Ok(false);
        }
        let (mut left, mut right) = (left.bytes(), right.bytes());
        loop {
            self.work(1)?;
            match (left.next(), right.next()) {
                (Some(left), Some(right)) if left == right => {}
                (None, None) => return Ok(true),
                _ => return Ok(false),
            }
        }
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

enum Emit {
    Node(usize),
    Name(&'static str),
    EndObject(u64),
    EndArray,
}

fn write_arena<S: Sink + ?Sized>(
    scope: &Scope<'_>,
    nodes: &[ArenaNode],
    root: usize,
    writer: &mut Writer<'_, S>,
) -> Result<(), Error> {
    let mut pending = Vec::new();
    scope
        .reserve(&mut pending, 1)
        .map_err(|error| scope.refuse(error))?;
    pending.push(Emit::Node(root));
    while !pending.is_empty() {
        scope.work(1).map_err(|error| scope.refuse(error))?;
        let Some(event) = pending.pop() else { break };
        match event {
            Emit::Name(name) => writer.name(name)?,
            Emit::EndArray => writer.end_array()?,
            Emit::EndObject(work) => {
                scope.work(work).map_err(|error| scope.refuse(error))?;
                writer.end_object()?;
                scope.poll().map_err(|error| scope.refuse(error))?;
            }
            Emit::Node(index) => {
                let node = nodes
                    .get(index)
                    .ok_or_else(|| scope.refuse(ModelKeyError::Unsupported("arena node")))?;
                match node {
                    ArenaNode::Null => writer.null()?,
                    ArenaNode::Boolean(value) => writer.bool(*value)?,
                    ArenaNode::Text(text) => {
                        scope
                            .work(text.len() as u64)
                            .map_err(|error| scope.refuse(error))?;
                        writer.string(text)?;
                        scope.poll().map_err(|error| scope.refuse(error))?;
                    }
                    ArenaNode::Ordinal { value, digits } => {
                        scope
                            .work(*digits as u64)
                            .map_err(|error| scope.refuse(error))?;
                        writer.integer(i128::from(*value))?;
                    }
                    ArenaNode::Object(members) => {
                        writer.begin_object()?;
                        let extra = members
                            .len()
                            .checked_mul(2)
                            .and_then(|count| count.checked_add(1))
                            .ok_or_else(|| scope.refuse(ModelKeyError::SizeOverflow))?;
                        scope
                            .reserve(&mut pending, extra)
                            .map_err(|error| scope.refuse(error))?;
                        // These closed schema records have at most eight ASCII
                        // names. The pinned Rust writer's small-slice insertion
                        // sort plus duplicate scan performs fewer than n*n name
                        // comparisons; each visits both names and their closing quotes.
                        let mut longest = 0;
                        for member in members {
                            scope.work(1).map_err(|error| scope.refuse(error))?;
                            longest = longest.max(member.name.unwrap_or_default().len());
                        }
                        let work = members
                            .len()
                            .checked_mul(members.len())
                            .and_then(|count| {
                                longest
                                    .checked_add(1)
                                    .and_then(|width| count.checked_mul(width))
                            })
                            .and_then(|count| count.checked_mul(2))
                            .and_then(|count| u64::try_from(count).ok())
                            .ok_or_else(|| scope.refuse(ModelKeyError::SizeOverflow))?;
                        pending.push(Emit::EndObject(work));
                        for member in members.iter().rev() {
                            scope.work(1).map_err(|error| scope.refuse(error))?;
                            pending.push(Emit::Node(member.node));
                            pending.push(Emit::Name(member.name.ok_or_else(|| {
                                scope.refuse(ModelKeyError::Unsupported("object member"))
                            })?));
                        }
                    }
                    ArenaNode::Array(members) => {
                        writer.begin_array()?;
                        let extra = members
                            .len()
                            .checked_add(1)
                            .ok_or_else(|| scope.refuse(ModelKeyError::SizeOverflow))?;
                        scope
                            .reserve(&mut pending, extra)
                            .map_err(|error| scope.refuse(error))?;
                        pending.push(Emit::EndArray);
                        for member in members.iter().rev() {
                            scope.work(1).map_err(|error| scope.refuse(error))?;
                            pending.push(Emit::Node(member.node));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

enum ArenaNode {
    Null,
    Boolean(bool),
    Text(String),
    Ordinal { value: u64, digits: usize },
    Object(Vec<Member>),
    Array(Vec<Member>),
}
struct Member {
    name: Option<&'static str>,
    node: usize,
    order_bytes: Option<Vec<u8>>,
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
    nodes: Vec<ArenaNode>,
    root: Option<usize>,
    produced: usize,
}
impl<'s, 'a> Bounded<'s, 'a> {
    fn new(scope: &'s Scope<'a>) -> Self {
        Self {
            scope,
            stack: Vec::new(),
            nodes: Vec::new(),
            root: None,
            produced: 0,
        }
    }
    fn encode_node(&self, node: usize) -> Result<Vec<u8>, ModelKeyError> {
        self.scope
            .canonical(|writer| write_arena(self.scope, &self.nodes, node, writer))
    }
    fn finish_bytes(&self) -> Result<Vec<u8>, ModelKeyError> {
        self.encode_node(
            self.root
                .ok_or(ModelKeyError::Unsupported("identity root"))?,
        )
    }
    fn store(&mut self, node: ArenaNode) -> Result<(), ModelKeyError> {
        self.scope.reserve(&mut self.nodes, 1)?;
        let identity = self.nodes.len();
        self.nodes.push(node);
        self.push(identity)
    }
    fn push(&mut self, node: usize) -> Result<(), ModelKeyError> {
        let order_bytes = match self.stack.last().map(|frame| frame.order) {
            Some(ArrayOrder::CanonicalBytes) => Some(self.encode_node(node)?),
            Some(ArrayOrder::MapKeys) => {
                let Some(ArenaNode::Object(members)) = self.nodes.get(node) else {
                    return Err(ModelKeyError::Unsupported("map entry shape"));
                };
                let mut key = None;
                for member in members {
                    self.scope.work(1)?;
                    if member.name == Some("key") {
                        key = Some(member.node);
                        break;
                    }
                }
                Some(self.encode_node(key.ok_or(ModelKeyError::Unsupported("map entry key"))?)?)
            }
            _ => None,
        };
        if let Some(frame) = self.stack.last_mut() {
            self.scope.reserve(&mut frame.members, 1)?;
            frame.members.push(Member {
                name: frame.name.take(),
                node,
                order_bytes,
            });
        } else {
            self.root = Some(node)
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
        // Only FR-181 semantic array ordering is owned here. Every JSON
        // container and member ordering is emitted by the shared writer.
        if !frame.object && !matches!(frame.order, ArrayOrder::Retained) {
            for at in 1..frame.members.len() {
                let mut next = at;
                while next > 0 {
                    let left = frame.members[next - 1]
                        .order_bytes
                        .as_deref()
                        .ok_or(ModelKeyError::Unsupported("array order key"))?;
                    let right = frame.members[next]
                        .order_bytes
                        .as_deref()
                        .ok_or(ModelKeyError::Unsupported("array order key"))?;
                    self.scope.work((left.len().min(right.len()) + 1) as u64)?;
                    if !left.cmp(right).is_gt() {
                        break;
                    }
                    self.scope.work(1)?;
                    frame.members.swap(next - 1, next);
                    next -= 1;
                }
            }
        }
        self.store(if frame.object {
            ArenaNode::Object(frame.members)
        } else {
            ArenaNode::Array(frame.members)
        })
    }
    fn available(&self) -> Result<(usize, usize), ModelKeyError> {
        let separator = usize::from(
            self.stack
                .last()
                .is_some_and(|frame| !frame.object && !frame.members.is_empty()),
        );
        let offset = self
            .produced
            .checked_add(separator)
            .ok_or(ModelKeyError::SizeOverflow)?;
        self.scope.bytes(offset)?;
        Ok((
            usize::try_from(self.scope.identity.input_bytes)
                .unwrap_or(usize::MAX)
                .saturating_sub(offset),
            offset,
        ))
    }
    fn integer(&mut self, value: &Integer) -> Result<(), ModelKeyError> {
        let (available, offset) = self.available()?;
        if available < 2 {
            self.scope
                .bytes(offset.checked_add(2).ok_or(ModelKeyError::SizeOverflow)?)?;
        }
        let text = self
            .scope
            .decimal(value, available.saturating_sub(2), offset + 2)?;
        self.text(&text)
    }

    fn formatted(&mut self, source: impl std::fmt::Display) -> Result<(), ModelKeyError> {
        use std::fmt::Write;
        struct Count<'s, 'a> {
            scope: &'s Scope<'a>,
            bytes: usize,
            error: Option<ModelKeyError>,
        }
        impl std::fmt::Write for Count<'_, '_> {
            fn write_str(&mut self, text: &str) -> std::fmt::Result {
                self.scope.work(text.len() as u64).map_err(|error| {
                    self.error = Some(error);
                    std::fmt::Error
                })?;
                self.bytes = self.bytes.checked_add(text.len()).ok_or_else(|| {
                    self.error = Some(ModelKeyError::SizeOverflow);
                    std::fmt::Error
                })?;
                Ok(())
            }
        }
        let mut count = Count {
            scope: self.scope,
            bytes: 0,
            error: None,
        };
        if write!(count, "{source}").is_err() {
            return Err(count
                .error
                .unwrap_or(ModelKeyError::Unsupported("scalar measurement")));
        }
        let (_, offset) = self.available()?;
        self.scope.bytes(
            offset
                .checked_add(count.bytes)
                .and_then(|bytes| bytes.checked_add(2))
                .ok_or(ModelKeyError::SizeOverflow)?,
        )?;
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
        let length = self.scope.string_length(source)?;
        self.separator()?;
        self.charge(length)?;
        self.scope.work(source.len() as u64)?;
        let mut text = String::new();
        if !source.is_empty() {
            self.scope.admit(source.len())?;
            text.try_reserve_exact(source.len())
                .map_err(|_| EncodingRefusal::Allocation {
                    requested: source.len(),
                })?;
        }
        text.push_str(source);
        self.store(ArenaNode::Text(text))
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
        let (available, offset) = self.available()?;
        let text = self
            .scope
            .decimal(&Integer::from(ordinal), available, offset)?;
        self.separator()?;
        self.charge(text.len())?;
        self.store(ArenaNode::Ordinal {
            value: ordinal,
            digits: text.len(),
        })
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
    fn pending<T>(&mut self, stack: &mut Vec<T>, task: T) -> Result<(), Self::Error> {
        self.scope.work(1)?;
        self.scope.reserve(stack, 1)?;
        stack.push(task);
        Ok(())
    }
    fn visit(&mut self) -> Result<(), Self::Error> {
        self.scope.work(1)
    }
    fn scalar(&mut self, source: Scalar<'_>) -> Result<(), Self::Error> {
        match source {
            Scalar::Null => {
                self.separator()?;
                self.charge(4)?;
                self.store(ArenaNode::Null)
            }
            Scalar::Boolean(value) => {
                self.separator()?;
                self.charge(if value { 4 } else { 5 })?;
                self.store(ArenaNode::Boolean(value))
            }
            Scalar::Text(value) => self.text(value),
            Scalar::Integer(value) => self.integer(value),
            Scalar::Hex { bits, digits } => {
                struct Hex(u64, usize);
                impl std::fmt::Display for Hex {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(f, "{:0width$x}", self.0, width = self.1)
                    }
                }
                self.scope.work(digits as u64)?;
                self.formatted(Hex(bits, digits))
            }
            Scalar::Scale(value) => self.integer(&Integer::from(u64::from(value))),
            Scalar::Uuid(value) => {
                self.scope.work(value.octets().len() as u64)?;
                self.formatted(value)
            }
            Scalar::Timestamp(value) => self.integer(&Integer::from(value.nanoseconds())),
            Scalar::Node(value) => self.node(value),
            Scalar::Effective(value) => self.effective(value),
            Scalar::Declaration(value) => {
                let mut nested = Bounded::new(self.scope);
                projection::project_declaration(&mut nested, value)?;
                let bytes = nested.finish_bytes()?;
                self.text(
                    std::str::from_utf8(&bytes)
                        .map_err(|_| ModelKeyError::Unsupported("declaration UTF-8"))?,
                )
            }
            Scalar::DeclarationNode(value) => {
                let mut last = None;
                let mut path = value.view().path();
                loop {
                    self.scope.work(1)?;
                    match path.next() {
                        Some(segment) => last = Some(segment),
                        None => break,
                    }
                }
                match last {
                    Some(quire_exact::PathSegmentView::Text(node)) => {
                        self.scalar(Scalar::Declaration(super::DeclarationRecord {
                            package: value.view().package(),
                            node,
                        }))
                    }
                    _ => Err(ModelKeyError::Unsupported("a declaration identity")),
                }
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
                let bytes = nested.finish_bytes()?;
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
        self.scope
            .budget
            .borrow_mut()
            .integer(value)
            .map_err(|_| ModelKeyError::Denied)?;
        self.scope.poll()?;
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
    sink.finish_bytes()
}

#[cfg(test)]
pub(super) fn value_for_test(value: &Value, typing: Typing<'_>) -> Result<Vec<u8>, ModelKeyError> {
    struct Admit;
    impl quire_exact::Reservation for Admit {
        fn reserve(&mut self, _: usize) -> Result<(), ReservationDenied> {
            Ok(())
        }
    }
    impl IntegerDecimalBudget for Admit {
        fn work(&mut self, _: u64) -> Result<(), ReservationDenied> {
            Ok(())
        }
    }
    impl ModelKeyBudget for Admit {
        fn integer(&mut self, _: &Integer) -> Result<(), ReservationDenied> {
            Ok(())
        }
    }
    let cancel = Cancel::new();
    let mut budget = Admit;
    let scope = Scope {
        budget: RefCell::new(&mut budget),
        cancel: &cancel,
        identity: IdentityLimits::default(),
        refused: RefCell::new(None),
    };
    let mut sink = Bounded::new(&scope);
    projection::project_value(&mut sink, value, typing, None, false)?;
    sink.finish_bytes()
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
        cancel_after_storage: Option<Cancel>,
        cancel_on_integer: Option<Cancel>,
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
            if self.storage > 0 {
                if let Some(cancel) = &self.cancel_after_storage {
                    cancel.cancel(quire_exact::CancelCause::Requested);
                }
            }
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
            if let Some(cancel) = &self.cancel_on_integer {
                cancel.cancel(quire_exact::CancelCause::Deadline);
            }
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
        assert_eq!(
            scope.decimal(&integer, usize::MAX, 0),
            Err(ModelKeyError::Denied)
        );
        drop(scope);
        assert_eq!(budget.integers, 1);
        assert_eq!(budget.storage, 0);
        assert_eq!(budget.work, 0);
    }

    #[test]
    #[trace("TC-471", "FR-120-AC-4")]
    fn borrowed_node_comparison_denies_and_cancels_inside_actual_path_traversal() {
        use quire_exact::{NodeKey, PathSegment};
        let node = |tail: &str| {
            let mut key =
                NodeKey::new("fixture", vec![PathSegment::Text(tail.to_owned())], 0).unwrap();
            for _ in 0..128 {
                key = NodeKey::new("fixture", vec![PathSegment::Node(key)], 0).unwrap();
            }
            key
        };
        let left = node("left");
        let equal = node("left");
        let other = node("right");
        let cancel = Cancel::new();
        let mut budget = Admission {
            storage_limit: usize::MAX,
            work_limit: u64::MAX,
            ..Admission::default()
        };
        let scope = Scope {
            budget: RefCell::new(&mut budget),
            cancel: &cancel,
            identity: IdentityLimits::default(),
            refused: RefCell::new(None),
        };
        assert_eq!(scope.node_equal(&left, &equal), Ok(true));
        assert_eq!(scope.node_equal(&left, &other), Ok(false));
        drop(scope);
        assert!(budget.storage > 0);
        let mut budget = Admission {
            storage_limit: usize::MAX,
            work_limit: u64::MAX,
            cancel_after_storage: Some(cancel.clone()),
            ..Admission::default()
        };
        let scope = Scope {
            budget: RefCell::new(&mut budget),
            cancel: &cancel,
            identity: IdentityLimits::default(),
            refused: RefCell::new(None),
        };
        assert_eq!(
            scope.node_equal(&left, &equal),
            Err(ModelKeyError::Cancelled(
                quire_exact::CancelCause::Requested
            ))
        );
        drop(scope);
        assert!(budget.storage > 0);
    }

    #[test]
    #[trace("TC-729", "FR-259-AC-3")]
    fn real_deep_value_projection_uses_admitted_heap_frames_and_complete_writer() {
        const DEPTH: usize = 100_000;
        let mut value = Value::Integer(Integer::from(42_i64));
        let mut value_type = quire_exact::ValueType::Integer;
        for _ in 0..DEPTH {
            value = quire_exact::OptionValue::from_admitted(value_type.clone(), Some(value));
            value_type = quire_exact::ValueType::option(value_type);
        }
        // Kernel construction and the final owned-handle drops stay here.
        // Each worker only drops a shallow clone while this root remains held.
        let encode = |stack| {
            let source = value.clone();
            std::thread::Builder::new()
                .stack_size(stack)
                .spawn(move || value_for_test(&source, Typing::none()).expect("real value encodes"))
                .unwrap()
                .join()
                .unwrap()
        };
        let constrained = encode(512 * 1024);
        let control = encode(8 * 1024 * 1024);
        let mut expected = r#"{"type":"option","value":"#.repeat(DEPTH);
        expected.push_str(r#"{"type":"integer","value":"42"}"#);
        expected.push_str(&"}".repeat(DEPTH));
        assert_eq!(
            constrained,
            expected.as_bytes(),
            "independent literal canonical value oracle"
        );
        assert_eq!(
            constrained, control,
            "thread stack size must not change the actual Value projection"
        );

        for cancelled in [false, true] {
            let source = value.clone();
            let (error, depth, integers, storage) = std::thread::Builder::new()
                .stack_size(512 * 1024)
                .spawn(move || {
                    let cancel = Cancel::new();
                    let mut budget = Admission {
                        storage_limit: usize::MAX,
                        work_limit: u64::MAX,
                        deny_integer: !cancelled,
                        cancel_on_integer: cancelled.then(|| cancel.clone()),
                        ..Admission::default()
                    };
                    let scope = Scope {
                        budget: RefCell::new(&mut budget),
                        cancel: &cancel,
                        identity: IdentityLimits::default(),
                        refused: RefCell::new(None),
                    };
                    let mut sink = Bounded::new(&scope);
                    let error =
                        projection::project_value(&mut sink, &source, Typing::none(), None, false)
                            .expect_err("inner scalar admission stops the real deep traversal");
                    let depth = sink.stack.len();
                    assert!(
                        sink.nodes.is_empty(),
                        "deepest integer was denied before retained scalar construction"
                    );
                    drop(sink);
                    drop(scope);
                    (error, depth, budget.integers, budget.storage)
                })
                .unwrap()
                .join()
                .unwrap();
            assert_eq!(
                depth,
                DEPTH + 1,
                "all option objects and the integer leaf object are open at inner scalar admission"
            );
            assert_eq!(integers, 1);
            assert!(storage > 0);
            assert_eq!(
                error,
                if cancelled {
                    ModelKeyError::Cancelled(quire_exact::CancelCause::Deadline)
                } else {
                    ModelKeyError::Denied
                }
            );
        }
        drop(value);
        drop(value_type);
    }

    #[test]
    #[trace("TC-471", "FR-120-AC-4")]
    fn complete_canonical_container_reservation_preserves_caller_denial() {
        fn events(writer: &mut Writer<'_, Output<'_, '_>>) -> Result<(), Error> {
            writer.begin_object()?;
            writer.name("items")?;
            writer.begin_array()?;
            writer.null()?;
            writer.end_array()?;
            writer.end_object()
        }
        let cancel = Cancel::new();
        let mut denied = Admission {
            work_limit: u64::MAX,
            ..Admission::default()
        };
        let scope = Scope {
            budget: RefCell::new(&mut denied),
            cancel: &cancel,
            identity: IdentityLimits::default(),
            refused: RefCell::new(None),
        };
        assert_eq!(
            scope.canonical(events),
            Err(ModelKeyError::Denied),
            "the complete writer must ask before retaining its object frame"
        );
        assert!(
            scope.refused.borrow().is_none(),
            "the scoped writer denial must be consumed"
        );
        drop(scope);
        assert_eq!(denied.storage, 0);
        let mut admitted = Admission {
            storage_limit: usize::MAX,
            work_limit: u64::MAX,
            ..Admission::default()
        };
        let scope = Scope {
            budget: RefCell::new(&mut admitted),
            cancel: &cancel,
            identity: IdentityLimits::default(),
            refused: RefCell::new(None),
        };
        assert_eq!(scope.canonical(events).unwrap(), br#"{"items":[null]}"#);
    }
}
