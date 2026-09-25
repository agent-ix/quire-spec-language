// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-012 §2's sixth contract part, `requirements` (FR-062-AC-4), and
//! ADR-014 §4's extent of a requested item (QSL-140).
//!
//! A family whose claim form carries an FR-057 capability kind records one
//! [`Requirements`] value per checked item: the kind and the item's
//! [`ClaimExtent`]. A form with no kind records none. The family computes
//! the extent at `check`, where the stage limits are, with
//! [`classify_extent`], and `FamilyContract::requirements` reads it back:
//! the method is pure and total.
//!
//! Three concepts stay apart (ADR-014 §4). *Finite* is a property of a
//! concrete value and is never an extent. *Extent* is a property of a
//! requested item: [`ClaimExtent`]. It is not named `Extent`, because
//! `model::Extent{Closed, Open}` is QSpec FR-153 population closure. *Mode*
//! is a property of a backend advertisement: `qsl_route::Mode`.
//!
//! **Extent rule** (ADR-014 §4). Over the transitive closure of a claim's
//! argument and bound-variable types, each of these positions is one
//! unbounded domain, keyed by the carrying node and the child-index path:
//!
//! | Position | [`DomainKind`] | Boundable by |
//! | --- | --- | --- |
//! | collection with no bound | `Collection` | `FiniteBound::Cardinality` |
//! | population with no maximum | `Population` | `FiniteBound::Cardinality` |
//! | `Integer` with no range | `Integer` | `FiniteBound::IntegerRange` |
//! | recursive record or tuple (QSpec FR-143) | `Recursive` | `FiniteBound::Depth` |
//! | quantity (its magnitude is an unbounded `Rational`) | `Quantity` | nothing |
//! | loop admitted under QSpec FR-228-AC-5 | `Loop` | nothing |
//! | infinite-trace temporal formula | `InfiniteTrace` | nothing |
//!
//! [`classify_extent`] finds the five type positions. The loop and
//! infinite-trace domains belong to the families that admit those forms
//! (QSL-42, QSL-43), which add them with [`ClaimExtent::from_domains`].
//! A quantity is unbounded and takes no finite bound: no `FiniteBound`
//! variant ranges over a rational magnitude, so a bounded-only backend
//! settles it `unsupported`, never `supported` (ADR-013 C-22). Every other
//! type (`Int[a,b]`, `Rational`, `Decimal`, `Text`, `Float`, `Boolean`,
//! `Enum`, a reference) carries its own finite domain or is not a value
//! domain, so it adds none.
//!
//! No domain gives [`ClaimExtent::Bounded`]. No stage limit, accounting
//! limit, profile ceiling or backend budget ever makes a domain bounded
//! (ADR-014 §1).

use std::collections::BTreeMap;

use qsl_foundation::bound::{DomainKey, FiniteBoundKind};
use qsl_foundation::diagnostic::{LimitExceeded, LimitKind};
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::InternalFault;
use quire_exact::{NodeKey, ValueType};

use crate::check::Capability;
use crate::value::declaration::{CompositeShape, TypeEnvironment};

/// The kind of one unbounded domain (ADR-014 §4's table).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DomainKind {
    /// A collection type with no cardinality bound.
    Collection,
    /// A population with no declared maximum.
    Population,
    /// `Integer` with no range.
    Integer,
    /// A recursive record or tuple type (QSpec FR-143), at its first
    /// position on the path.
    Recursive,
    /// A loop admitted with an invariant and a well-founded variant and no
    /// finite maximum (QSpec FR-228-AC-5).
    Loop,
    /// A temporal formula under `quire.temporal.infinite-trace/v1`. A finite
    /// prefix never proves infinite satisfaction (QSpec FR-161).
    InfiniteTrace,
    /// A quantity type: its magnitude is an unbounded `Rational`, and no
    /// finite bound ranges over it.
    Quantity,
}

impl DomainKind {
    /// The [`FiniteBoundKind`] a proof bound for this domain must have, or
    /// `None` when no finite bound can stand for it.
    pub const fn finite_kind(self) -> Option<FiniteBoundKind> {
        match self {
            Self::Collection | Self::Population => Some(FiniteBoundKind::Cardinality),
            Self::Integer => Some(FiniteBoundKind::IntegerRange),
            Self::Recursive => Some(FiniteBoundKind::Depth),
            Self::Loop | Self::InfiniteTrace | Self::Quantity => None,
        }
    }
}

/// The non-empty set of an item's unbounded domains, each with its kind,
/// in [`DomainKey`] order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnboundedDomains(BTreeMap<DomainKey, DomainKind>);

impl UnboundedDomains {
    /// Every domain and its kind, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&DomainKey, DomainKind)> {
        self.0.iter().map(|(key, kind)| (key, *kind))
    }

    /// The kind of `key`, if it is one of these domains.
    pub fn kind(&self, key: &DomainKey) -> Option<DomainKind> {
        self.0.get(key).copied()
    }

    /// How many domains there are. Never zero.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Always `false`: an unbounded extent has at least one domain.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether a finite bound can stand for every domain (ADR-014 §4
    /// "Available finite bound").
    pub fn all_boundable(&self) -> bool {
        self.0.values().all(|kind| kind.finite_kind().is_some())
    }
}

/// The extent of a requested item (ADR-014 §4). Wire spelling: QSpec
/// FR-290 `bounded`/`unbounded`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimExtent {
    /// No unbounded domain.
    Bounded,
    /// At least one unbounded domain.
    Unbounded(UnboundedDomains),
}

impl ClaimExtent {
    /// `Bounded` for no domains, otherwise `Unbounded` over exactly these.
    pub fn from_domains(domains: BTreeMap<DomainKey, DomainKind>) -> Self {
        if domains.is_empty() {
            Self::Bounded
        } else {
            Self::Unbounded(UnboundedDomains(domains))
        }
    }

    /// The FR-290 wire spelling: `bounded` or `unbounded`.
    pub const fn to_wire(&self) -> &'static str {
        match self {
            Self::Bounded => "bounded",
            Self::Unbounded(_) => "unbounded",
        }
    }
}

/// One item's requirements (ADR-012 §2, FR-062-AC-4; ADR-014 §4, §6 step
/// 1): the FR-057 capability kind it requests and its extent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Requirements {
    kind: Capability,
    extent: ClaimExtent,
}

impl Requirements {
    /// The requirements of an item requesting `kind` over `extent`.
    pub fn new(kind: Capability, extent: ClaimExtent) -> Self {
        Self { kind, extent }
    }

    /// The requested capability kind.
    pub fn kind(&self) -> Capability {
        self.kind
    }

    /// The item's extent.
    pub fn extent(&self) -> &ClaimExtent {
        &self.extent
    }
}

/// Why [`classify_extent`] stopped without an extent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClassifyFailure {
    /// The walk visited more type positions than its ceiling allows: a
    /// check-stage limit (ADR-014 B-3), kind node count.
    Limit(LimitExceeded),
    /// A checked type named a record or tuple the environment does not
    /// hold, or a position index past `u32`: a broken check invariant.
    Fault(InternalFault),
}

/// The fault stage name [`classify_extent`] reports.
const STAGE: &str = "check.requirements";

/// One pending type position.
struct Position<'t> {
    node: WireNodeId,
    path: Vec<u32>,
    value_type: &'t ValueType,
    /// The records and tuples on the path to this position, each with the
    /// path at which it was entered, outermost first.
    composites: Vec<(NodeKey, Vec<u32>)>,
}

/// Classify the extent of an item whose arguments and bound variables are
/// `roots`: each root is the carrying node's wire id and its checked type.
///
/// The walk uses an explicit stack. It visits at most `position_limit`
/// type positions, each position once per path that reaches it, and
/// refuses past that with a node-count stage limit. The limit carries no
/// locus: this walk sees types, not source. The family's `check` that calls
/// it attaches its declaration's `Locus::Region` with `LimitExceeded::at`,
/// as FR-096's family-`check` row requires. A record or tuple that
/// reaches itself is one `Recursive` domain at the position where the
/// recursion starts, and the walk does not descend into it again.
pub fn classify_extent(
    roots: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
) -> Result<ClaimExtent, ClassifyFailure> {
    let mut domains = BTreeMap::new();
    let mut pending: Vec<Position<'_>> = roots
        .iter()
        .rev()
        .map(|(node, value_type)| Position {
            node: *node,
            path: Vec::new(),
            value_type,
            composites: Vec::new(),
        })
        .collect();
    let mut visited: u64 = 0;
    while let Some(position) = pending.pop() {
        if visited >= position_limit {
            return Err(ClassifyFailure::Limit(LimitExceeded::new(
                LimitKind::NodeCount,
                position_limit,
                u128::from(visited) + 1,
            )));
        }
        visited += 1;
        let key = || DomainKey::new(position.node, position.path.clone());
        match position.value_type {
            ValueType::Integer => {
                domains.insert(key(), DomainKind::Integer);
            }
            ValueType::Population(None) => {
                domains.insert(key(), DomainKind::Population);
            }
            ValueType::Quantity(_) => {
                domains.insert(key(), DomainKind::Quantity);
            }
            ValueType::Collection(collection) => {
                if collection.bound().is_none() {
                    domains.insert(key(), DomainKind::Collection);
                }
                pending.push(child(&position, 0, collection.element(), None)?);
            }
            ValueType::Option(payload) => {
                pending.push(child(&position, 0, payload, None)?);
            }
            ValueType::Composite(declaration) => {
                if let Some((_, entered)) = position
                    .composites
                    .iter()
                    .find(|(open, _)| open == declaration)
                {
                    domains.insert(
                        DomainKey::new(position.node, entered.clone()),
                        DomainKind::Recursive,
                    );
                    continue;
                }
                let Some(composite) = types.composite(*declaration) else {
                    return Err(ClassifyFailure::Fault(InternalFault::new(
                        STAGE,
                        "checked-composite-type-not-in-environment",
                    )));
                };
                let members: Vec<&ValueType> = match composite.shape() {
                    CompositeShape::Record(fields) => {
                        fields.iter().map(|field| field.value_type()).collect()
                    }
                    CompositeShape::Tuple(positions) => positions.iter().collect(),
                };
                // Pushed last-first so the stack visits them in order.
                for (index, member) in members.into_iter().enumerate().rev() {
                    pending.push(child(&position, index, member, Some(*declaration))?);
                }
            }
            ValueType::Boolean
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Text(_)
            | ValueType::Enum(_)
            | ValueType::Reference(_)
            | ValueType::Population(Some(_)) => {}
        }
    }
    Ok(ClaimExtent::from_domains(domains))
}

/// The child position `index` of `parent`, typed `value_type`; `entered`
/// is the record or tuple `parent` is, when it is one.
fn child<'t>(
    parent: &Position<'t>,
    index: usize,
    value_type: &'t ValueType,
    entered: Option<NodeKey>,
) -> Result<Position<'t>, ClassifyFailure> {
    let index = u32::try_from(index).map_err(|_| {
        ClassifyFailure::Fault(InternalFault::new(STAGE, "type-position-index-past-u32"))
    })?;
    let mut composites = parent.composites.clone();
    if let Some(declaration) = entered {
        composites.push((declaration, parent.path.clone()));
    }
    let mut path = parent.path.clone();
    path.push(index);
    Ok(Position {
        node: parent.node,
        path,
        value_type,
        composites,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::declaration::{CompositeDeclaration, FieldDeclaration};
    use ix_trace_rs::trace;
    use quire_exact::{
        CardinalityBound, CollectionKind, CollectionType, Integer, IntegerInterval, Presence,
    };

    const LIMIT: u64 = 1_000;

    fn node(fill: u8) -> WireNodeId {
        WireNodeId::from_digest([fill; 32])
    }

    fn int_0_9() -> ValueType {
        ValueType::Int(IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).unwrap())
    }

    fn sequence(element: ValueType, bound: Option<(u64, u64)>) -> ValueType {
        ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            element,
            bound.map(|(minimum, maximum)| CardinalityBound::new(minimum, maximum).unwrap()),
        ))
    }

    fn point_key() -> NodeKey {
        NodeKey::from_digest([1; 32])
    }

    fn tree_key() -> NodeKey {
        NodeKey::from_digest([2; 32])
    }

    /// `record Point { x: Int[0, 9]; y: Integer; }` and the recursive
    /// `record Tree { kids: Sequence<Tree>[0, 3]; }`.
    fn types() -> TypeEnvironment {
        TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    point_key(),
                    "Point",
                    CompositeShape::Record(vec![
                        FieldDeclaration::new("x", int_0_9(), Presence::Required),
                        FieldDeclaration::new("y", ValueType::Integer, Presence::Required),
                    ]),
                ),
                CompositeDeclaration::new(
                    tree_key(),
                    "Tree",
                    CompositeShape::Record(vec![FieldDeclaration::new(
                        "kids",
                        sequence(ValueType::Composite(tree_key()), Some((0, 3))),
                        Presence::Required,
                    )]),
                ),
            ],
            [],
        )
        .expect("FR-143 admits Point and Tree")
    }

    fn extent(value_type: &ValueType) -> ClaimExtent {
        classify_extent(&[(node(7), value_type)], &types(), LIMIT).unwrap()
    }

    fn unbounded(domains: &[(Vec<u32>, DomainKind)]) -> ClaimExtent {
        ClaimExtent::from_domains(
            domains
                .iter()
                .map(|(path, kind)| (DomainKey::new(node(7), path.clone()), *kind))
                .collect(),
        )
    }

    /// TC-437 (ADR-014 §4 extent rule): each unbounded type position is one
    /// domain keyed by its carrying node and child-index path, with its
    /// kind; every type that carries its own finite domain is bounded.
    #[trace("TC-437", "FR-097-AC-2")]
    #[test]
    fn tc_437_each_unbounded_type_position_is_one_domain() {
        assert_eq!(
            extent(&ValueType::Integer),
            unbounded(&[(vec![], DomainKind::Integer)])
        );
        assert_eq!(
            extent(&sequence(int_0_9(), None)),
            unbounded(&[(vec![], DomainKind::Collection)])
        );
        assert_eq!(
            extent(&sequence(ValueType::Integer, Some((0, 3)))),
            unbounded(&[(vec![0], DomainKind::Integer)])
        );
        assert_eq!(
            extent(&sequence(ValueType::Integer, None)),
            unbounded(&[
                (vec![], DomainKind::Collection),
                (vec![0], DomainKind::Integer),
            ])
        );
        assert_eq!(
            extent(&ValueType::option(ValueType::Integer)),
            unbounded(&[(vec![0], DomainKind::Integer)])
        );
        assert_eq!(
            extent(&ValueType::Population(None)),
            unbounded(&[(vec![], DomainKind::Population)])
        );
        assert_eq!(
            extent(&ValueType::Composite(point_key())),
            unbounded(&[(vec![1], DomainKind::Integer)])
        );
        assert_eq!(
            extent(&ValueType::Composite(tree_key())),
            unbounded(&[(vec![], DomainKind::Recursive)])
        );
        // Every finite-domain type is bounded.
        for bounded in [
            int_0_9(),
            ValueType::Boolean,
            ValueType::Population(Some(3)),
            sequence(int_0_9(), Some((0, 3))),
            ValueType::option(int_0_9()),
        ] {
            assert_eq!(extent(&bounded), ClaimExtent::Bounded, "{bounded:?}");
        }
    }

    /// TC-437: domains of several roots are keyed by their own nodes, and
    /// classifying the same roots twice gives equal extents.
    #[trace("TC-437", "FR-097-AC-2")]
    #[test]
    fn tc_437_roots_key_their_own_domains_and_classification_repeats() {
        let integer = ValueType::Integer;
        let collection = sequence(int_0_9(), None);
        let roots = [(node(1), &integer), (node(2), &collection)];
        let first = classify_extent(&roots, &types(), LIMIT).unwrap();
        assert_eq!(first, classify_extent(&roots, &types(), LIMIT).unwrap());
        let ClaimExtent::Unbounded(domains) = &first else {
            panic!("two unbounded roots");
        };
        let listed: Vec<_> = domains
            .iter()
            .map(|(key, kind)| (key.clone(), kind))
            .collect();
        assert_eq!(
            listed,
            vec![
                (DomainKey::new(node(1), vec![]), DomainKind::Integer),
                (DomainKey::new(node(2), vec![]), DomainKind::Collection),
            ]
        );
        assert!(domains.all_boundable());
        assert_eq!(first.to_wire(), "unbounded");
        assert_eq!(ClaimExtent::Bounded.to_wire(), "bounded");
    }

    /// TC-437: the walk stops at its position ceiling with a node-count
    /// stage limit (ADR-014 B-3), not with an extent; one more position
    /// admits it.
    #[trace("TC-437", "FR-097-AC-2")]
    #[test]
    fn tc_437_the_walk_stops_at_its_position_ceiling() {
        // Point, its two fields: three positions.
        let point = ValueType::Composite(point_key());
        let roots = [(node(7), &point)];
        assert_eq!(
            classify_extent(&roots, &types(), 2),
            Err(ClassifyFailure::Limit(LimitExceeded::new(
                LimitKind::NodeCount,
                2,
                3
            )))
        );
        assert!(classify_extent(&roots, &types(), 3).is_ok());
    }

    /// TC-437: a checked composite missing from the environment is a broken
    /// invariant, never a bounded extent.
    #[trace("TC-437", "FR-097-AC-2")]
    #[test]
    fn tc_437_a_composite_outside_the_environment_is_a_fault() {
        let missing = ValueType::Composite(NodeKey::from_digest([9; 32]));
        let Err(ClassifyFailure::Fault(fault)) =
            classify_extent(&[(node(7), &missing)], &types(), LIMIT)
        else {
            panic!("expected a fault");
        };
        assert_eq!(
            fault.invariant(),
            "checked-composite-type-not-in-environment"
        );
    }

    /// ADR-014 §4: loop and infinite-trace domains are not boundable, the
    /// four type domains are.
    #[trace("TC-437", "FR-097-AC-2")]
    #[test]
    fn tc_437_only_the_type_domains_are_boundable() {
        assert_eq!(
            DomainKind::Collection.finite_kind(),
            Some(FiniteBoundKind::Cardinality)
        );
        assert_eq!(
            DomainKind::Population.finite_kind(),
            Some(FiniteBoundKind::Cardinality)
        );
        assert_eq!(
            DomainKind::Integer.finite_kind(),
            Some(FiniteBoundKind::IntegerRange)
        );
        assert_eq!(
            DomainKind::Recursive.finite_kind(),
            Some(FiniteBoundKind::Depth)
        );
        assert_eq!(DomainKind::Loop.finite_kind(), None);
        assert_eq!(DomainKind::InfiniteTrace.finite_kind(), None);
        let mixed = ClaimExtent::from_domains(BTreeMap::from([
            (DomainKey::new(node(1), vec![]), DomainKind::Integer),
            (DomainKey::new(node(2), vec![]), DomainKind::Loop),
        ]));
        let ClaimExtent::Unbounded(domains) = mixed else {
            panic!("two domains");
        };
        assert!(!domains.all_boundable());
    }

    /// A claim form with an FR-057 kind, checked through the real contract:
    /// a `value-validity` claim over its argument types. No claim family is
    /// migrated onto `FamilyContract` yet (QSL-42 and QSL-43 migrate the
    /// first), so this test family stands in for one. Its `check` calls the
    /// real [`classify_extent`] under the context's node-count limit, and
    /// its `requirements` reads back what `check` recorded.
    struct ValidityClaim;

    struct CheckedClaim {
        requirements: Requirements,
    }

    impl crate::family::FamilyContract for ValidityClaim {
        type Form = Vec<(WireNodeId, ValueType)>;
        type Checked = CheckedClaim;
        type Cause = InternalFault;
        type Declarations<'a> = TypeEnvironment;

        fn check<'a>(
            form: &Self::Form,
            cx: &mut crate::family::CheckContext<'a, TypeEnvironment>,
        ) -> crate::family::CheckOutcome<CheckedClaim, InternalFault> {
            let roots: Vec<(WireNodeId, &ValueType)> = form
                .iter()
                .map(|(node, value_type)| (*node, value_type))
                .collect();
            let extent = classify_extent(&roots, cx.declarations(), cx.limits().node_count)
                .map_err(|failure| match failure {
                    ClassifyFailure::Limit(exceeded) => {
                        qsl_foundation::diagnostic::StageFailure::Limit(exceeded)
                    }
                    ClassifyFailure::Fault(fault) => {
                        qsl_foundation::diagnostic::StageFailure::Refused(fault)
                    }
                })?;
            Ok(qsl_foundation::diagnostic::Staged::new(CheckedClaim {
                requirements: Requirements::new(Capability::ValueValidity, extent),
            }))
        }

        fn requirements(checked: &CheckedClaim) -> Option<Requirements> {
            Some(checked.requirements.clone())
        }
    }

    fn check_claim(form: &[(WireNodeId, ValueType)]) -> CheckedClaim {
        use crate::family::{
            CheckContext, DiagnosticSink, FamilyContract, ScopeStack, StageLimits,
        };
        let types = types();
        let mut meter = quire_exact::Meter::new(quire_exact::ScalarLimits {
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
        });
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &types,
            StageLimits {
                nesting_depth: 8,
                input_bytes: u64::MAX,
                node_count: LIMIT,
            },
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        ValidityClaim::check(&form.to_vec(), &mut cx)
            .expect("the claim checks")
            .into_value()
    }

    /// TC-437 (ADR-014 §10 scenario 3): a claim form with a
    /// kind yields exactly one `Requirements` naming that kind, here with
    /// the set's domain as its unbounded extent; two calls on the same
    /// checked node return equal values.
    #[trace("TC-437", "FR-097-AC-2")]
    #[test]
    fn tc_437_a_claim_family_records_its_classified_extent() {
        use crate::family::FamilyContract;
        let set = ValueType::collection(CollectionType::new(CollectionKind::Set, int_0_9(), None));
        let checked = check_claim(&[(node(7), set)]);
        let first = ValidityClaim::requirements(&checked).expect("one requirements value");
        assert_eq!(first.kind(), Capability::ValueValidity);
        assert_eq!(
            first.extent(),
            &unbounded(&[(vec![], DomainKind::Collection)])
        );
        assert_eq!(ValidityClaim::requirements(&checked), Some(first));

        let bounded = check_claim(&[(node(7), int_0_9())]);
        assert_eq!(
            ValidityClaim::requirements(&bounded),
            Some(Requirements::new(
                Capability::ValueValidity,
                ClaimExtent::Bounded
            ))
        );
    }

    /// TC-437 (ADR-014 §4 amended; ADR-013 C-22): a quantity is one
    /// unbounded, unboundable domain, so an item over it has no finite bound
    /// available and a bounded-only backend can never settle it supported.
    #[trace("TC-437", "FR-097-AC-2")]
    #[test]
    fn tc_437_a_quantity_is_an_unboundable_domain() {
        let metre =
            ValueType::Quantity(quire_exact::UnitId::declared(NodeKey::from_digest([5; 32])));
        let extent = extent(&metre);
        assert_eq!(extent, unbounded(&[(vec![], DomainKind::Quantity)]));
        let ClaimExtent::Unbounded(domains) = extent else {
            panic!("a quantity is unbounded");
        };
        assert!(!domains.all_boundable());
        assert_eq!(DomainKind::Quantity.finite_kind(), None);
    }
}
