//! `ValueType`'s structural traits, walked without native recursion.
//!
//! A `ValueType` nests through exactly two variants, `Option` and
//! `Collection`, and each holds one nested type, so a type is a chain.
//! `Clone`, `PartialEq`, `Hash` and `Drop` follow the chain with a cursor
//! and need no stack at all: each step handles one link's own data
//! (`ValueType::shallow_clone`, `ValueType::shallow_eq`,
//! `ValueType::shallow_hash`) and moves to `ValueType::child`. `Debug`
//! must close each link after its child, so it runs on `Value`'s `Debug`
//! worklist, which holds a constant number of steps per link.
//!
//! The derived traits of `CollectionType` stay derived: each calls these
//! impls once for its element, and these impls never call back into them.

use alloc::{boxed::Box, vec::Vec};
use core::fmt;
use core::hash::{Hash, Hasher};

use super::{schedule, tuple_steps, Bracket, CollectionType, DebugWriter, Step, ValueType};

impl ValueType {
    /// The one type nested directly in this type, if any.
    fn child(&self) -> Option<&ValueType> {
        match self {
            Self::Option(payload) => Some(payload),
            Self::Collection(collection) => Some(collection.element()),
            Self::Boolean
            | Self::Integer
            | Self::Int(_)
            | Self::Rational(_)
            | Self::Decimal(_)
            | Self::Float(_)
            | Self::Quantity(_)
            | Self::Text(_)
            | Self::Enum(_)
            | Self::Composite(_)
            | Self::Reference(_)
            | Self::Population(_) => None,
        }
    }

    /// The slot of the one type nested directly in this type, if any.
    fn child_mut(&mut self) -> Option<&mut ValueType> {
        match self {
            Self::Option(payload) => Some(payload),
            Self::Collection(collection) => Some(collection.element_mut()),
            Self::Boolean
            | Self::Integer
            | Self::Int(_)
            | Self::Rational(_)
            | Self::Decimal(_)
            | Self::Float(_)
            | Self::Quantity(_)
            | Self::Text(_)
            | Self::Enum(_)
            | Self::Composite(_)
            | Self::Reference(_)
            | Self::Population(_) => None,
        }
    }

    /// A copy of this link alone: its nested type, if it has one, is the
    /// placeholder `Boolean` for [`Clone::clone`] to overwrite.
    fn shallow_clone(&self) -> Self {
        match self {
            Self::Boolean => Self::Boolean,
            Self::Integer => Self::Integer,
            Self::Int(interval) => Self::Int(interval.clone()),
            Self::Rational(domain) => Self::Rational(domain.clone()),
            Self::Decimal(declared) => Self::Decimal(declared.clone()),
            Self::Float(declared) => Self::Float(*declared),
            Self::Quantity(unit) => Self::Quantity(*unit),
            Self::Text(declared) => Self::Text(*declared),
            Self::Enum(shape) => Self::Enum(shape.clone()),
            Self::Option(_) => Self::Option(Box::new(Self::Boolean)),
            Self::Composite(declaration) => Self::Composite(*declaration),
            Self::Collection(collection) => Self::collection(CollectionType::new(
                collection.kind(),
                Self::Boolean,
                collection.bound(),
            )),
            Self::Reference(object_type) => Self::Reference(*object_type),
            Self::Population(maximum) => Self::Population(*maximum),
        }
    }

    /// Whether this link and `other`'s agree on variant and on everything
    /// but their nested types.
    fn shallow_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean, Self::Boolean)
            | (Self::Integer, Self::Integer)
            | (Self::Option(_), Self::Option(_)) => true,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::Rational(left), Self::Rational(right)) => left == right,
            (Self::Decimal(left), Self::Decimal(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Quantity(left), Self::Quantity(right)) => left == right,
            (Self::Text(left), Self::Text(right)) => left == right,
            (Self::Enum(left), Self::Enum(right)) => left == right,
            (Self::Composite(left), Self::Composite(right)) => left == right,
            (Self::Collection(left), Self::Collection(right)) => {
                left.kind() == right.kind() && left.bound() == right.bound()
            }
            (Self::Reference(left), Self::Reference(right)) => left == right,
            (Self::Population(left), Self::Population(right)) => left == right,
            (
                Self::Boolean
                | Self::Integer
                | Self::Int(_)
                | Self::Rational(_)
                | Self::Decimal(_)
                | Self::Float(_)
                | Self::Quantity(_)
                | Self::Text(_)
                | Self::Enum(_)
                | Self::Option(_)
                | Self::Composite(_)
                | Self::Collection(_)
                | Self::Reference(_)
                | Self::Population(_),
                _,
            ) => false,
        }
    }

    /// Hash this link's variant and everything but its nested type: exactly
    /// what `ValueType::shallow_eq` compares.
    fn shallow_hash<H: Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            Self::Boolean | Self::Integer | Self::Option(_) => {}
            Self::Int(interval) => interval.hash(state),
            Self::Rational(domain) => domain.hash(state),
            Self::Decimal(declared) => declared.hash(state),
            Self::Float(declared) => declared.hash(state),
            Self::Quantity(unit) => unit.hash(state),
            Self::Text(declared) => declared.hash(state),
            Self::Enum(shape) => shape.hash(state),
            Self::Composite(declaration) => declaration.hash(state),
            Self::Collection(collection) => {
                collection.kind().hash(state);
                collection.bound().hash(state);
            }
            Self::Reference(object_type) => object_type.hash(state),
            Self::Population(maximum) => maximum.hash(state),
        }
    }
}

impl Clone for ValueType {
    /// Copies the chain top-down: each link is copied with a placeholder
    /// child, which the next step overwrites in place.
    fn clone(&self) -> Self {
        let mut root = self.shallow_clone();
        let mut source = self;
        let mut target = &mut root;
        while let (Some(next_source), Some(slot)) = (source.child(), target.child_mut()) {
            *slot = next_source.shallow_clone();
            source = next_source;
            target = slot;
        }
        root
    }
}

impl PartialEq for ValueType {
    /// Compares the two chains link by link.
    fn eq(&self, other: &Self) -> bool {
        let (mut left, mut right) = (self, other);
        loop {
            if !left.shallow_eq(right) {
                return false;
            }
            // `shallow_eq` matched the variants, so both have a child or
            // neither has.
            match (left.child(), right.child()) {
                (Some(next_left), Some(next_right)) => (left, right) = (next_left, next_right),
                _ => return true,
            }
        }
    }
}

impl Hash for ValueType {
    /// Hashes the chain link by link, so equal types hash equal.
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut link = Some(self);
        while let Some(current) = link {
            current.shallow_hash(state);
            link = current.child();
        }
    }
}

impl Drop for ValueType {
    /// Detaches the chain one link at a time, so each link drops with only
    /// a leaf below it and the stack stays a fixed few frames deep.
    fn drop(&mut self) {
        let mut next = self.child_mut().map(take_type);
        while let Some(mut link) = next {
            next = link.child_mut().map(take_type);
        }
    }
}

/// Move the type out of `slot`, leaving the leaf `Boolean`.
fn take_type(slot: &mut ValueType) -> ValueType {
    core::mem::replace(slot, ValueType::Boolean)
}

impl fmt::Debug for ValueType {
    /// Prints what `#[derive(Debug)]` printed, in compact and alternate
    /// mode, from `Value`'s `Debug` worklist instead of recursion. Leaves
    /// print through their own `Debug`. In alternate mode a leaf gets plain
    /// `{:#?}`, so the width, fill, precision and hex flags of the caller's
    /// format spec reach compact-mode leaves only.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        DebugWriter::new(formatter)
            .write(Step::Type(self))
            .map(|_peak| ())
    }
}

/// Push the steps that print one type, outermost name first. A leaf
/// variant's payload prints through its own `Debug`; a nesting variant
/// leaves its closing steps on the worklist while its child prints.
pub(super) fn schedule_type<'a>(stack: &mut Vec<Step<'a>>, value_type: &'a ValueType) {
    let (name, leaf): (&'static str, &'a dyn fmt::Debug) = match value_type {
        ValueType::Boolean => return stack.push(Step::Text("Boolean")),
        ValueType::Integer => return stack.push(Step::Text("Integer")),
        ValueType::Option(payload) => {
            return schedule(stack, tuple_steps("Option", Step::Type(payload)));
        }
        ValueType::Collection(collection) => {
            return schedule(
                stack,
                tuple_steps("Collection", Step::CollectionType(collection)),
            );
        }
        ValueType::Int(leaf) => ("Int", leaf),
        ValueType::Rational(leaf) => ("Rational", leaf),
        ValueType::Decimal(leaf) => ("Decimal", leaf),
        ValueType::Float(leaf) => ("Float", leaf),
        ValueType::Quantity(leaf) => ("Quantity", leaf),
        ValueType::Text(leaf) => ("Text", leaf),
        ValueType::Enum(leaf) => ("Enum", leaf),
        ValueType::Composite(leaf) => ("Composite", leaf),
        ValueType::Reference(leaf) => ("Reference", leaf),
        ValueType::Population(leaf) => ("Population", leaf),
    };
    schedule(stack, tuple_steps(name, Step::Leaf(leaf)));
}

/// Push `CollectionType { kind: .., element: .., bound: .. }`, the derived
/// layout, with the element as a `Step::Type`.
pub(super) fn schedule_collection_type<'a>(
    stack: &mut Vec<Step<'a>>,
    collection_type: &'a CollectionType,
) {
    let (kind, element, bound) = collection_type.debug_fields();
    schedule(
        stack,
        [
            Step::Text("CollectionType"),
            Step::Open(Bracket::Brace),
            Step::Text("kind: "),
            Step::Leaf(kind),
            Step::Next,
            Step::Text("element: "),
            Step::Type(element),
            Step::Next,
            Step::Text("bound: "),
            Step::Leaf(bound),
            Step::Close(Bracket::Brace),
        ],
    );
}

#[cfg(test)]
mod tests {
    use super::super::{
        DebugWriter, EffectiveId, EnumShape, FloatType, Integer, IntegerInterval, NodeKey,
        RationalDomain, Step, TextType, UnitId, ValueType, VariantId,
    };
    use crate::collection::{CardinalityBound, CollectionKind, CollectionType};
    use crate::decimal::{DecimalType, RoundingMode};
    use crate::ieee::IeeeWidth;
    use crate::text::TextProfile;
    use alloc::vec::Vec;
    use core::cell::Cell;
    use core::fmt;
    use core::hash::{Hash, Hasher};
    use ix_trace_rs::trace;
    use std::collections::hash_map::DefaultHasher;

    /// Levels of nesting in the deep types. A recursive walk spends at
    /// least one stack frame per level, so this many levels overflows the
    /// 512 KiB stack [`on_small_stack`] gives each walk.
    const DEEP: usize = 100_000;

    /// Run `walk` on a thread with a 512 KiB stack (TC-735).
    fn on_small_stack(walk: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(walk)
            .expect("spawn the walk thread")
            .join()
            .expect("the walk completes without overflowing the stack");
    }

    fn digest(byte: u8) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        bytes
    }

    fn interval(lower: i64, upper: i64) -> IntegerInterval {
        IntegerInterval::new(Integer::from(lower), Integer::from(upper)).expect("ordered")
    }

    fn hash_of(value_type: &ValueType) -> u64 {
        let mut hasher = DefaultHasher::new();
        value_type.hash(&mut hasher);
        hasher.finish()
    }

    /// `DEEP` `Option`s around `Boolean`, built bottom-up by a loop.
    fn deep_option() -> ValueType {
        (0..DEEP).fold(ValueType::Boolean, |inner, _| ValueType::option(inner))
    }

    /// `DEEP` links alternating `Option` and unbounded `Sequence` around
    /// `Boolean`, so both nesting variants are walked deep.
    fn deep_mixed() -> ValueType {
        (0..DEEP).fold(ValueType::Boolean, |inner, level| {
            if level % 2 == 0 {
                ValueType::option(inner)
            } else {
                ValueType::collection(CollectionType::new(CollectionKind::Sequence, inner, None))
            }
        })
    }

    /// One type per variant, plus a bounded collection nested in options.
    fn samples() -> Vec<ValueType> {
        let key = NodeKey::from_digest(digest(9));
        let nested = ValueType::option(ValueType::collection(CollectionType::new(
            CollectionKind::Set,
            ValueType::option(ValueType::Int(interval(0, 9))),
            Some(CardinalityBound::new(1, 3).expect("ordered")),
        )));
        vec![
            ValueType::Boolean,
            ValueType::Integer,
            ValueType::Int(interval(-2, 7)),
            ValueType::Rational(
                RationalDomain::new(interval(0, 4), interval(1, 8)).expect("positive"),
            ),
            ValueType::Decimal(
                DecimalType::new(
                    Integer::zero(),
                    Integer::from(100_i64),
                    0,
                    2,
                    RoundingMode::Exact,
                )
                .expect("well formed"),
            ),
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            ValueType::Quantity(UnitId::declared(key)),
            ValueType::Text(TextType::new(0, 10, TextProfile::UnicodeScalars).expect("bounds")),
            ValueType::Enum(EnumShape::new(false, [VariantId::from_digest(digest(3))])),
            ValueType::Composite(key),
            ValueType::Reference(EffectiveId::from_digest(digest(6))),
            ValueType::Population(Some(5)),
            ValueType::Population(None),
            ValueType::collection(CollectionType::new(
                CollectionKind::Sequence,
                ValueType::Boolean,
                None,
            )),
            nested,
        ]
    }

    /// What `#[derive(Debug)]` printed for each of [`samples`], compact.
    const DERIVED_COMPACT: [&str; 15] = [
        "Boolean",
        "Integer",
        "Int(IntegerInterval { lower: Integer(-2), upper: Integer(7) })",
        "Rational(RationalDomain { numerator: IntegerInterval { lower: Integer(0), upper: Integer(4) }, denominator: IntegerInterval { lower: Integer(1), upper: Integer(8) } })",
        "Decimal(DecimalType { lower: Integer(0), upper: Integer(100), min_scale: 0, max_scale: 2, rounding: Exact })",
        "Float(FloatType { width: Binary64, rounding: Exact })",
        "Quantity(UnitId(quire.checked-semantic-node/v1, 0000000000000000000000000000000000000000000000000000000000000009))",
        "Text(TextType { min: 0, max: 10, profile: UnicodeScalars })",
        "Enum(EnumShape { ordered: false, variants: [VariantId(0000000000000000000000000000000000000000000000000000000000000003)] })",
        "Composite(NodeKey(0000000000000000000000000000000000000000000000000000000000000009))",
        "Reference(EffectiveId(0000000000000000000000000000000000000000000000000000000000000006))",
        "Population(Some(5))",
        "Population(None)",
        "Collection(CollectionType { kind: Sequence, element: Boolean, bound: None })",
        "Option(Collection(CollectionType { kind: Set, element: Option(Int(IntegerInterval { lower: Integer(0), upper: Integer(9) })), bound: Some(CardinalityBound { minimum: 1, maximum: 3 }) }))",
    ];

    /// What `#[derive(Debug)]` printed for the last of [`samples`], in
    /// alternate mode: both nesting variants, a struct leaf and an
    /// `Option` leaf.
    const DERIVED_ALTERNATE_NESTED: &str = "Option(
    Collection(
        CollectionType {
            kind: Set,
            element: Option(
                Int(
                    IntegerInterval {
                        lower: Integer(
                            0,
                        ),
                        upper: Integer(
                            9,
                        ),
                    },
                ),
            ),
            bound: Some(
                CardinalityBound {
                    minimum: 1,
                    maximum: 3,
                },
            ),
        },
    ),
)";

    /// A `fmt::Write` sink that counts opening and closing parentheses, so
    /// formatting a deep type needs no buffer the size of its output.
    #[derive(Default)]
    struct ParenCount {
        open: usize,
        close: usize,
    }

    impl fmt::Write for ParenCount {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            for byte in text.bytes() {
                match byte {
                    b'(' => self.open += 1,
                    b')' => self.close += 1,
                    _ => {}
                }
            }
            Ok(())
        }
    }

    /// A 100,000-deep `Option` type clones, compares equal to its clone,
    /// hashes equal to its clone, formats for debug and drops, with its
    /// clone, on a 512 KiB stack.
    #[test]
    #[trace("TC-735", "FR-262-AC-2")]
    fn tc_735_a_deep_option_type_clones_compares_hashes_formats_and_drops() {
        on_small_stack(|| {
            let value_type = deep_option();
            let copy = value_type.clone();
            assert!(value_type == copy);
            assert_eq!(hash_of(&value_type), hash_of(&copy));
            let wrapped = ValueType::option(copy.clone());
            assert!(value_type != wrapped);
            assert_ne!(hash_of(&value_type), hash_of(&wrapped));
            drop(wrapped);

            let mut sink = ParenCount::default();
            fmt::write(&mut sink, format_args!("{value_type:?}")).expect("format");
            assert_eq!(sink.open, DEEP);
            assert_eq!(sink.close, DEEP);

            drop(value_type);
            drop(copy);
        });
    }

    /// `Debug` prints exactly what `#[derive(Debug)]` printed for every
    /// variant in compact mode, and for both nesting variants in alternate
    /// mode.
    #[test]
    #[trace("TC-735", "FR-262-AC-2")]
    fn tc_735_value_type_debug_matches_the_derived_format() {
        let samples = samples();
        for (value_type, compact) in samples.iter().zip(DERIVED_COMPACT) {
            assert_eq!(format!("{value_type:?}"), compact);
        }
        let nested = samples.last().expect("samples");
        assert_eq!(format!("{nested:#?}"), DERIVED_ALTERNATE_NESTED);
    }

    /// Every sample compares equal to and hashes equal to its clone, and
    /// compares unequal to and hashes differently from every other sample.
    #[test]
    #[trace("TC-735", "FR-262-AC-2")]
    fn tc_735_value_types_equal_and_hash_equal_their_clones_only() {
        let samples = samples();
        for (index, value_type) in samples.iter().enumerate() {
            let copy = value_type.clone();
            assert!(*value_type == copy);
            assert_eq!(hash_of(value_type), hash_of(&copy));
            for (other_index, other) in samples.iter().enumerate() {
                assert_eq!(value_type == other, index == other_index);
                assert_eq!(hash_of(value_type) == hash_of(other), index == other_index);
            }
        }
        let bounded = |maximum| {
            ValueType::collection(CollectionType::new(
                CollectionKind::Set,
                ValueType::Boolean,
                Some(CardinalityBound::new(0, maximum).expect("ordered")),
            ))
        };
        assert!(bounded(3) != bounded(4));
        assert_ne!(hash_of(&bounded(3)), hash_of(&bounded(4)));
    }

    /// The `Debug` worklist is the one heap stack these walks keep, and it
    /// holds at most a constant number of steps per node of the type being
    /// printed, so the nodes the type was charged for when it was built
    /// bound it. Each `Option` link is one node; each `Collection` link is
    /// two, the `ValueType` and its `CollectionType`.
    #[test]
    #[trace("TC-735", "FR-262-AC-2")]
    fn tc_735_the_debug_worklist_grows_by_a_constant_per_type_node() {
        /// Formats a type through the worklist and records its peak.
        struct Peak<'a>(&'a ValueType, &'a Cell<usize>);

        impl fmt::Debug for Peak<'_> {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.1
                    .set(DebugWriter::new(formatter).write(Step::Type(self.0))?);
                Ok(())
            }
        }

        /// Steps one node leaves on the worklist while its child prints.
        const STEPS_PER_NODE: usize = 3;
        /// Steps of the innermost leaf's own expansion.
        const LEAF_STEPS: usize = 4;

        on_small_stack(|| {
            for (value_type, nodes) in [
                (deep_option(), DEEP + 1),
                // Half the links are collections, of two nodes each.
                (deep_mixed(), DEEP + DEEP / 2 + 1),
            ] {
                let peak = Cell::new(0);
                let mut sink = ParenCount::default();
                fmt::write(&mut sink, format_args!("{:?}", Peak(&value_type, &peak)))
                    .expect("format");
                assert_eq!(sink.open, sink.close);
                assert!(peak.get() >= DEEP, "the worklist holds every open link");
                assert!(peak.get() <= STEPS_PER_NODE * nodes + LEAF_STEPS);
            }
        });
    }
}
