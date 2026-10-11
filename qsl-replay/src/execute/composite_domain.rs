// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-358 steps 3 to 5: the positions of a claimed equality's parameter
//! operands, their declared bounds, and the harness bounds checked against
//! them.
//!
//! A parameter operand's declared type is walked through the transitive
//! closure of its declarations on an explicit heap stack (ADR-030 D-1), as
//! ADR-014 §4's extent walk does, and each position is keyed by
//! `DomainKey::Node { node: parameter, path }`. A position carries its
//! authored bound; a request `DeclaredDomain` may replace only an unbounded
//! one (ADR-014 §1), and a harness bound covers a position when it reaches
//! the declared bound (ADR-014 §8).

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;
use std::rc::Rc;

use qsl_foundation::bound::{DomainKey, FiniteBound, FiniteBoundKind, ProofBound};
use qsl_foundation::diagnostic::{Code, InternalFault};
use qsl_foundation::digest::WireNodeId;
use qsl_package::CheckedPackage;
use quire_exact::{IntegerInterval, NodeKey, ValueType};
use quire_semantic_value::declaration::CompositeShape;

use super::argument::{declaration_of, enum_declarations};
use super::ReplayRefusal;
use crate::identity::DeclaredDomain;

/// What a request, a claim or a derivation got wrong about a claim's
/// bounds, naming the domain key where there is one.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ParityBoundRefusal {
    /// A request `DeclaredDomain` is keyed at no derived position.
    #[error("the declared domain at {key} is no position of the claim's operands")]
    DeclaredUnknownKey {
        /// The declared domain's key.
        key: DomainKey,
    },
    /// A request `DeclaredDomain` is over a position with an authored bound:
    /// ADR-014 B-4 replaces only an unbounded domain.
    #[error("the declared domain at {key} is over a position with an authored bound")]
    DeclaredOverAuthored {
        /// The declared domain's key.
        key: DomainKey,
    },
    /// Two request `DeclaredDomain`s share a key.
    #[error("two declared domains share the key {key}")]
    DeclaredDuplicate {
        /// The shared key.
        key: DomainKey,
    },
    /// A request `DeclaredDomain`'s kind differs from its position's.
    #[error(
        "the declared domain at {key} is a {supplied:?} bound, and the position takes {expected:?}"
    )]
    DeclaredKind {
        /// The declared domain's key.
        key: DomainKey,
        /// The kind the position takes.
        expected: FiniteBoundKind,
        /// The kind the declared domain has.
        supplied: FiniteBoundKind,
    },
    /// A harness bound is keyed at no derived position.
    #[error("the harness bound at {key} is no position of the claim's operands")]
    HarnessUnknownKey {
        /// The harness bound's key.
        key: DomainKey,
    },
    /// Two harness bounds share a key.
    #[error("two harness bounds share the key {key}")]
    HarnessDuplicate {
        /// The shared key.
        key: DomainKey,
    },
    /// A harness bound's kind differs from its position's, or the position
    /// takes no bound kind at all.
    #[error(
        "the harness bound at {key} is a {supplied:?} bound, and the position takes {expected:?}"
    )]
    HarnessKind {
        /// The harness bound's key.
        key: DomainKey,
        /// The kind the position takes, or `None` for a position no bound
        /// kind describes.
        expected: Option<FiniteBoundKind>,
        /// The kind the harness bound has.
        supplied: FiniteBoundKind,
    },
    /// A `Variants` harness bound names a variant the declared enum does not
    /// admit.
    #[error("the harness bound at {key} names variant {variant:?}, which the declared enum does not admit")]
    HarnessUnknownVariant {
        /// The harness bound's key.
        key: DomainKey,
        /// The variant it names.
        variant: String,
    },
    /// The operands' types hold more positions than the check stage's node
    /// limit allows.
    #[error("the claim's operand types hold more than {limit} positions")]
    PositionLimit {
        /// The limit.
        limit: u64,
    },
}

impl ParityBoundRefusal {
    /// The catalog code of this refusal: a position ceiling is a stage
    /// limit, and every other refusal is invalid input to the claim.
    pub fn code(&self) -> Code {
        match self {
            Self::PositionLimit { .. } => Code::StageLimitExceeded,
            Self::DeclaredUnknownKey { .. }
            | Self::DeclaredOverAuthored { .. }
            | Self::DeclaredDuplicate { .. }
            | Self::DeclaredKind { .. }
            | Self::HarnessUnknownKey { .. }
            | Self::HarnessDuplicate { .. }
            | Self::HarnessKind { .. }
            | Self::HarnessUnknownVariant { .. } => Code::InvalidRuntimeInput,
        }
    }
}

fn refuse(refusal: ParityBoundRefusal) -> ReplayRefusal {
    ReplayRefusal::ParityBound(Box::new(refusal))
}

fn fault(invariant: &'static str) -> ReplayRefusal {
    ReplayRefusal::Fault(InternalFault::new("replay", invariant))
}

/// A position's authored bound.
enum Authored {
    /// An `Int[lower, upper]` leaf.
    Range(IntegerInterval),
    /// An `Integer` leaf: unbounded.
    AnyInteger,
    /// A `K<T>[minimum, maximum]` collection: this maximum.
    Cardinality(u64),
    /// A `K<T>` collection: unbounded.
    AnyCollection,
    /// A recursive declaration: its depth is unbounded (ADR-014 §2).
    Recursive,
    /// An enum: every variant its declaration admits, ascending.
    Variants(Vec<String>),
    /// A text, rational, decimal, float, quantity, reference or population
    /// leaf: its whole declared domain, which no bound kind describes.
    Whole,
}

impl Authored {
    /// The bound kind a position takes, when one describes it.
    fn kind(&self) -> Option<FiniteBoundKind> {
        match self {
            Self::Range(_) | Self::AnyInteger => Some(FiniteBoundKind::IntegerRange),
            Self::Cardinality(_) | Self::AnyCollection => Some(FiniteBoundKind::Cardinality),
            Self::Recursive => Some(FiniteBoundKind::Depth),
            Self::Variants(_) => Some(FiniteBoundKind::Variants),
            Self::Whole => None,
        }
    }

    /// Whether the position has an authored bound, which a request's
    /// `DeclaredDomain` may not replace.
    fn is_authored(&self) -> bool {
        match self {
            Self::Range(_) | Self::Cardinality(_) | Self::Variants(_) | Self::Whole => true,
            Self::AnyInteger | Self::AnyCollection | Self::Recursive => false,
        }
    }
}

/// A position's declared bound: its authored one, or the request's
/// substitution for an unbounded one.
enum Declared {
    Range(IntegerInterval),
    Cardinality(u64),
    Depth(NonZeroU64),
    Variants(Vec<String>),
    /// No authored bound and no request `DeclaredDomain`: never covered.
    Unbounded,
    /// A position no bound kind describes: never covered.
    Whole,
}

impl Declared {
    /// Whether `harness` reaches this declared bound.
    fn reached_by(&self, harness: &FiniteBound) -> bool {
        match (self, harness) {
            (Self::Range(declared), FiniteBound::IntegerRange(drawn)) => {
                drawn.lower() <= declared.lower() && drawn.upper() >= declared.upper()
            }
            (Self::Cardinality(declared), FiniteBound::Cardinality { maximum }) => {
                maximum >= declared
            }
            (Self::Depth(declared), FiniteBound::Depth { maximum }) => maximum >= declared,
            (Self::Variants(declared), FiniteBound::Variants { members }) => {
                declared.iter().all(|variant| members.contains(variant))
            }
            _ => false,
        }
    }
}

/// One derived position.
struct Position {
    authored: Authored,
    declared: Declared,
}

/// The derived positions of a claim's parameter operands, in
/// [`DomainKey`] order.
pub(super) struct Positions(BTreeMap<DomainKey, Position>);

/// One pending type position of the walk.
struct Pending<'t> {
    root: WireNodeId,
    path: Path,
    value_type: &'t ValueType,
    /// The records and tuples on the path to this position, innermost first.
    composites: Option<Rc<Open>>,
}

/// A child-index path, shared by every position below it: each step links
/// to its parent, so a position costs one link, not a copy of its path.
type Path = Option<Rc<Step>>;

/// One step of a [`Path`].
struct Step {
    index: u32,
    parent: Path,
}

/// A record or tuple entered on the way to a position, with the path at
/// which it was entered.
struct Open {
    declaration: NodeKey,
    entered: Path,
    parent: Option<Rc<Open>>,
}

/// The child indexes `path` holds, root first.
fn indexes(path: &Path) -> Vec<u32> {
    let mut found = Vec::new();
    let mut next = path;
    while let Some(step) = next {
        found.push(step.index);
        next = &step.parent;
    }
    found.reverse();
    found
}

/// Derive the positions of `parameters`, each a node id and its declared
/// type (FR-358 step 3), visiting at most `limit` type positions.
pub(super) fn derive(
    parameters: &[(WireNodeId, &ValueType)],
    package: &CheckedPackage,
    limit: u64,
) -> Result<Positions, ReplayRefusal> {
    let types = package.graph().scope().types();
    let enums = enum_declarations(package);
    let mut found: BTreeMap<DomainKey, Authored> = BTreeMap::new();
    let mut pending: Vec<Pending<'_>> = parameters
        .iter()
        .rev()
        .map(|(root, value_type)| Pending {
            root: *root,
            path: None,
            value_type,
            composites: None,
        })
        .collect();
    let mut visited: u64 = 0;
    while let Some(position) = pending.pop() {
        if visited >= limit {
            return Err(refuse(ParityBoundRefusal::PositionLimit { limit }));
        }
        visited += 1;
        let key = || DomainKey::Node {
            node: position.root,
            path: indexes(&position.path),
        };
        match position.value_type {
            ValueType::Boolean => {}
            ValueType::Int(interval) => {
                found.insert(key(), Authored::Range(interval.clone()));
            }
            ValueType::Integer => {
                found.insert(key(), Authored::AnyInteger);
            }
            ValueType::Enum(shape) => {
                let declaration = declaration_of(&enums, shape)
                    .ok_or_else(|| fault("enum-position-has-a-declaration"))?;
                let mut members = declaration.members().to_vec();
                members.sort_unstable();
                found.insert(key(), Authored::Variants(members));
            }
            ValueType::Collection(collection) => {
                found.insert(
                    key(),
                    collection.bound().map_or(Authored::AnyCollection, |bound| {
                        Authored::Cardinality(bound.maximum())
                    }),
                );
                pending.push(child(&position, 0, collection.element(), None)?);
            }
            ValueType::Option(payload) => {
                pending.push(child(&position, 0, payload, None)?);
            }
            ValueType::Composite(declaration) => {
                let mut open = &position.composites;
                let mut recursive = None;
                while let Some(link) = open {
                    if link.declaration == *declaration {
                        recursive = Some(&link.entered);
                        break;
                    }
                    open = &link.parent;
                }
                if let Some(entered) = recursive {
                    found.insert(
                        DomainKey::Node {
                            node: position.root,
                            path: indexes(entered),
                        },
                        Authored::Recursive,
                    );
                    continue;
                }
                let composite = types
                    .composite(*declaration)
                    .ok_or_else(|| fault("composite-type-in-environment"))?;
                let members: Vec<&ValueType> = match composite.shape() {
                    CompositeShape::Record(fields) => {
                        fields.iter().map(|field| field.value_type()).collect()
                    }
                    CompositeShape::Tuple(positions) => positions.iter().collect(),
                };
                for (index, member) in members.into_iter().enumerate().rev() {
                    pending.push(child(&position, index, member, Some(*declaration))?);
                }
            }
            ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Text(_)
            | ValueType::Quantity(_)
            | ValueType::Reference(_)
            | ValueType::Population(_)
            | ValueType::Uuid
            | ValueType::Timestamp => {
                found.insert(key(), Authored::Whole);
            }
        }
    }
    Ok(Positions(
        found
            .into_iter()
            .map(|(key, authored)| {
                let declared = match &authored {
                    Authored::Range(interval) => Declared::Range(interval.clone()),
                    Authored::Cardinality(maximum) => Declared::Cardinality(*maximum),
                    Authored::Variants(members) => Declared::Variants(members.clone()),
                    Authored::Whole => Declared::Whole,
                    Authored::AnyInteger | Authored::AnyCollection | Authored::Recursive => {
                        Declared::Unbounded
                    }
                };
                (key, Position { authored, declared })
            })
            .collect(),
    ))
}

/// The child position `index` of `parent`; `entered` is the record or tuple
/// `parent` is, when it is one.
fn child<'t>(
    parent: &Pending<'t>,
    index: usize,
    value_type: &'t ValueType,
    entered: Option<NodeKey>,
) -> Result<Pending<'t>, ReplayRefusal> {
    let index = u32::try_from(index).map_err(|_| fault("type-position-index-fits-u32"))?;
    let composites = match entered {
        Some(declaration) => Some(Rc::new(Open {
            declaration,
            entered: parent.path.clone(),
            parent: parent.composites.clone(),
        })),
        None => parent.composites.clone(),
    };
    Ok(Pending {
        root: parent.root,
        path: Some(Rc::new(Step {
            index,
            parent: parent.path.clone(),
        })),
        value_type,
        composites,
    })
}

impl Positions {
    /// FR-358 step 4: each position's declared bound, the request's
    /// `DeclaredDomain` for its key where the request carries one.
    pub(super) fn declare(&mut self, requested: &[DeclaredDomain]) -> Result<(), ReplayRefusal> {
        let mut seen = BTreeSet::new();
        for declared in requested {
            let key = declared.domain().clone();
            if !seen.insert(key.clone()) {
                return Err(refuse(ParityBoundRefusal::DeclaredDuplicate { key }));
            }
            let Some(position) = self.0.get_mut(&key) else {
                return Err(refuse(ParityBoundRefusal::DeclaredUnknownKey { key }));
            };
            if position.authored.is_authored() {
                return Err(refuse(ParityBoundRefusal::DeclaredOverAuthored { key }));
            }
            let bound = declared.bound();
            position.declared = match (&position.authored, bound) {
                (Authored::AnyInteger, FiniteBound::IntegerRange(interval)) => {
                    Declared::Range(interval.clone())
                }
                (Authored::AnyCollection, FiniteBound::Cardinality { maximum }) => {
                    Declared::Cardinality(*maximum)
                }
                (Authored::Recursive, FiniteBound::Depth { maximum }) => Declared::Depth(*maximum),
                _ => {
                    let expected = position
                        .authored
                        .kind()
                        .ok_or_else(|| fault("unbounded-position-takes-a-kind"))?;
                    return Err(refuse(ParityBoundRefusal::DeclaredKind {
                        key,
                        expected,
                        supplied: bound.kind(),
                    }));
                }
            };
        }
        Ok(())
    }

    /// FR-358 step 5: refuse a harness bound keyed at no position, a second
    /// one on a key, one of the wrong kind and a `Variants` bound naming a
    /// variant its enum does not admit. The returned bounds are the harness's,
    /// by key.
    pub(super) fn harness<'b>(
        &self,
        bounds: &'b [ProofBound],
    ) -> Result<BTreeMap<&'b DomainKey, &'b FiniteBound>, ReplayRefusal> {
        let mut by_key: BTreeMap<&DomainKey, &FiniteBound> = BTreeMap::new();
        for bound in bounds {
            let key = bound.domain();
            if by_key.contains_key(key) {
                return Err(refuse(ParityBoundRefusal::HarnessDuplicate {
                    key: key.clone(),
                }));
            }
            let Some(position) = self.0.get(key) else {
                return Err(refuse(ParityBoundRefusal::HarnessUnknownKey {
                    key: key.clone(),
                }));
            };
            let expected = position.authored.kind();
            if expected != Some(bound.bound().kind()) {
                return Err(refuse(ParityBoundRefusal::HarnessKind {
                    key: key.clone(),
                    expected,
                    supplied: bound.bound().kind(),
                }));
            }
            if let (Authored::Variants(admitted), FiniteBound::Variants { members }) =
                (&position.authored, bound.bound())
            {
                if let Some(variant) = members.iter().find(|member| !admitted.contains(member)) {
                    return Err(refuse(ParityBoundRefusal::HarnessUnknownVariant {
                        key: key.clone(),
                        variant: variant.clone(),
                    }));
                }
            }
            by_key.insert(key, bound.bound());
        }
        Ok(by_key)
    }

    /// Whether every position is covered: a harness bound has its key and
    /// reaches its declared bound. A position with no harness bound, an
    /// unbounded one and a whole-domain leaf are never covered; no position
    /// at all is covered vacuously.
    pub(super) fn covered(&self, harness: &BTreeMap<&DomainKey, &FiniteBound>) -> bool {
        self.0.iter().all(|(key, position)| {
            harness
                .get(key)
                .is_some_and(|bound| position.declared.reached_by(bound))
        })
    }
}
