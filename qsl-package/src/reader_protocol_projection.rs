// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reader-owned protocol expression nodes projected from admitted I2 terms
//! (FR-390 "Composed roots from the reader").
//!
//! The whole package is projected once, when its reader-backed view is
//! created: every slot of every protocol node, each a tree of positions in
//! FR-383's slot-walk order, its applications paired with the admitted
//! `protocol_locations` rows of the slot's own origin. A node reads its
//! location from that row or, for a leaf, `let`, conditional or case, from its
//! own admitted node, which the wire does not carry yet (QSpec
//! FR-322-AC-310), so it reads none. Nothing here takes a location from the
//! source package.

use std::collections::{BTreeMap, BTreeSet};

use qsl_semantics::check::wire_id::WireGraph;
use quire_checked::node::{
    Arithmetic, CaseArm, CheckedLiteral, Connective, ConnectiveOrigin, IntegerDivisionOperator,
    NodeKind, OrderedKind, RecordField, RecordSlot, Visit,
};
use quire_contract_model::{
    CheckedNodeId, CheckedPackageV2, CheckedProtocolLocation, CheckedProtocolSemanticLocation,
    CheckedSemanticNodeV2, CheckedTerm, OperationModeKind,
};
use quire_exact::{
    admit_text, ArithmeticOperator, Meter, NodeKey, OrderingOperator, ScalarLimits, TextPayload,
    Timestamp, Uuid, Value, ValueType,
};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::{
    CheckedEquality, EqualityOperand, EqualityOperator, FieldRef,
};
use quire_semantic_value::location::{Location, Origin};

use crate::equality_file::{EqualityWitnessRefusal, ReadEqualityRows};
use crate::wire_form::{has_form, split_tag_form, OperationSuffix, WireForm, WireTag};

/// One projected protocol slot: owned evaluator nodes, the admitted function
/// table its calls reach, and the frame its locals and case binders use.
/// Wire terms and producer rows stay inside qsl-package.
pub struct ReaderProtocolProjection {
    nodes: Vec<ReaderProjectedNode>,
    functions: Vec<ReaderFunction>,
    root: usize,
    frame_slots: usize,
    locals: BTreeMap<NodeKey, usize>,
}

impl ReaderProtocolProjection {
    /// Index of the selected root node.
    pub fn root(&self) -> usize {
        self.root
    }

    /// The exact node kind at an arena index.
    pub fn kind(&self, index: usize) -> Option<&NodeKind<usize, CheckedLiteral>> {
        self.nodes.get(index).map(|node| &node.kind)
    }

    /// The selected exact type of an arena node.
    pub fn value_type(&self, index: usize) -> Option<&ValueType> {
        self.nodes.get(index).map(|node| &node.value_type)
    }

    /// The admitted location of an arena node: its paired
    /// `protocol_locations` row for a slot-walk application, `None` where
    /// the admitted wire carries none for it (FR-390, QSpec FR-322-AC-310).
    pub fn location(&self, index: usize) -> Option<&Location> {
        self.nodes
            .get(index)
            .and_then(|node| node.location.as_ref())
    }

    /// Ordered child node indices.
    pub fn children(&self, index: usize) -> Option<&[usize]> {
        self.nodes.get(index).map(|node| node.children.as_slice())
    }

    /// Function table entry for a checked call.
    pub fn function(&self, index: usize) -> Option<ReaderProtocolFunction<'_>> {
        let function = self.functions.get(index)?;
        Some(ReaderProtocolFunction {
            name: &function.name,
            body: function.body,
            slots: function.slots,
        })
    }

    /// The number of frame slots the slot's root context uses: its binders
    /// and the binders of every `case` it holds outside a function.
    pub fn frame_slots(&self) -> usize {
        self.frame_slots
    }

    /// The local slot a supplied binder is read through.
    pub fn local_slot(&self, binder: &NodeKey) -> Option<usize> {
        self.locals.get(binder).copied()
    }

    /// The location of each application position of the slot's walk, in
    /// walk order (FR-383).
    #[cfg(test)]
    pub(crate) fn term_locations(&self) -> Vec<Option<&Location>> {
        walk(&self.nodes, self.root)
            .into_iter()
            .filter_map(|index| self.nodes.get(index))
            .filter(|node| node.term.is_some())
            .map(|node| node.location.as_ref())
            .collect()
    }
}

/// One admitted pure function available to a projected protocol expression.
pub struct ReaderProtocolFunction<'a> {
    /// Function name used by evaluator diagnostics.
    pub name: &'a str,
    /// Function body node index.
    pub body: usize,
    /// Number of checked parameter slots.
    pub slots: usize,
}

/// The projection of one slot, or why it has none.
pub enum ReaderSlot {
    /// The slot's admitted expression projected to evaluator nodes.
    Projected(ReaderProtocolProjection),
    /// An admitted form or operation without a projection (FR-390
    /// "Typed projection admission"), a typed refusal of this slot.
    Unsupported(UnsupportedView),
    /// The admitted package broke a projection invariant.
    Fault(InternalFault),
}

/// Every protocol slot of one reader-backed package, keyed by the slot's root
/// node and its type-declaration origin name (FR-383's site name).
pub struct ReaderProtocolSlots {
    slots: BTreeMap<(NodeKey, String), ReaderSlot>,
}

impl ReaderProtocolSlots {
    /// The projection of the slot rooted at `root` under `origin`.
    pub(crate) fn slot(&self, root: &NodeKey, origin: &str) -> Option<&ReaderSlot> {
        self.slots.get(&(root.clone(), origin.to_owned()))
    }

    /// The slot projection under `origin`, whatever its root.
    #[cfg(test)]
    pub(crate) fn slot_named(&self, origin: &str) -> Option<&ReaderSlot> {
        self.slots
            .iter()
            .find(|((_, name), _)| name == origin)
            .map(|(_, slot)| slot)
    }
}

/// FR-390's typed unsupported view: the catalog pair
/// `unknown_required_feature`/`unsupported-feature`, the owning package, a
/// typed locus and one typed reason.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedView {
    package: String,
    locus: ViewLocus,
    reason: UnsupportedReason,
}

impl UnsupportedView {
    /// A binder type with no kernel representation, at its node.
    pub(crate) fn missing_type(package: &str, key: NodeKey) -> Self {
        Self {
            package: package.to_owned(),
            locus: ViewLocus::node(key),
            reason: UnsupportedReason::MissingTypeRepresentation,
        }
    }

    /// A witness refusal (FR-390, FR-531): the package-scoped locus, no node
    /// key and no location, because producer-row validity is a property of
    /// the package.
    pub(crate) fn witness(package: &str, reason: UnsupportedReason) -> Self {
        Self {
            package: package.to_owned(),
            locus: ViewLocus::Package,
            reason,
        }
    }

    /// The order view creation reports the first unsupported projection by
    /// (FR-390-AC-67): ascending package identity, then locus (`Package`
    /// before `Node`), then ascending node key, then FR-390's reason order. A
    /// location reason breaks its own tie as FR-390-AC-33..38 state: a visit
    /// without its row before a row without its visit, then the unpaired
    /// index.
    pub(crate) fn order(&self) -> (&str, u8, Option<&NodeKey>, u8, bool, usize) {
        let (surplus_rows, index) = match &self.reason {
            UnsupportedReason::MissingLocationProjection(missing) => {
                (missing.rows > missing.visits, missing.index)
            }
            _ => (false, 0),
        };
        (
            &self.package,
            u8::from(matches!(self.locus, ViewLocus::Node { .. })),
            self.locus.key(),
            self.reason.rank(),
            surplus_rows,
            index,
        )
    }

    /// The catalog code and cause every unsupported view carries.
    pub fn catalog(&self) -> (&'static str, &'static str) {
        ("unknown_required_feature", "unsupported-feature")
    }

    /// The owning package identity.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// Where the unsupported projection is.
    pub fn locus(&self) -> &ViewLocus {
        &self.locus
    }

    /// Why the projection is unsupported.
    pub fn reason(&self) -> &UnsupportedReason {
        &self.reason
    }
}

/// The locus of an [`UnsupportedView`] (FR-390 "Typed projection admission").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ViewLocus {
    /// The package as a whole, with no node key and no location: the locus of
    /// every witness refusal, because producer-row validity is package-scoped.
    Package,
    /// The affected admitted node and its own admitted location (`None` where
    /// the wire carries none for it).
    Node {
        /// The node's structural key.
        key: NodeKey,
        /// The node's own admitted location.
        location: Option<Location>,
    },
}

impl ViewLocus {
    /// The affected node's structural key; `None` at the `Package` locus.
    pub fn key(&self) -> Option<&NodeKey> {
        match self {
            Self::Package => None,
            Self::Node { key, .. } => Some(key),
        }
    }

    /// The affected node's admitted location; `None` at the `Package` locus
    /// and where the wire carries none.
    pub fn location(&self) -> Option<&Location> {
        match self {
            Self::Package => None,
            Self::Node { location, .. } => location.as_ref(),
        }
    }

    fn node(key: NodeKey) -> Self {
        Self::Node {
            key,
            location: None,
        }
    }
}

/// The typed reason of an [`UnsupportedView`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnsupportedReason {
    /// An admitted type without a kernel representation.
    MissingTypeRepresentation,
    /// An admitted literal without a kernel value.
    MissingLiteralProjection,
    /// An admitted expression form or operation without a `NodeKind`.
    MissingOperationProjection {
        /// The operation identity, or the node's tag and form.
        operation: String,
    },
    /// A slot-walk visit without its `protocol_locations` row, or a row
    /// without its visit.
    MissingLocationProjection(Box<MissingLocation>),
    /// The identity rows of a bytes-only supply are absent (FR-531).
    MissingIdentityWitness,
    /// The identity rows name no identity the package admits (FR-531).
    UnknownIdentityWitness,
    /// The identity rows were produced for another package (FR-531).
    ForeignIdentityWitness,
    /// A slot reaches an equality application and the supply holds no
    /// equality rows (FR-531 "The equality file").
    MissingEqualityWitness,
    /// The equality rows join to no equality application the slot reaches.
    UnknownEqualityWitness,
    /// The equality rows were produced for another package.
    ForeignEqualityWitness,
}

impl UnsupportedReason {
    /// The reason's stable name; an operation reason carries its identity.
    pub fn name(&self) -> String {
        match self {
            Self::MissingTypeRepresentation => "missing-type-representation".to_owned(),
            Self::MissingLiteralProjection => "missing-literal-projection".to_owned(),
            Self::MissingOperationProjection { operation } => {
                format!("missing-operation-projection:{operation}")
            }
            Self::MissingLocationProjection(_) => "missing-location-projection".to_owned(),
            Self::MissingIdentityWitness => "missing-identity-witness".to_owned(),
            Self::UnknownIdentityWitness => "unknown-identity-witness".to_owned(),
            Self::ForeignIdentityWitness => "foreign-identity-witness".to_owned(),
            Self::MissingEqualityWitness => "missing-equality-witness".to_owned(),
            Self::UnknownEqualityWitness => "unknown-equality-witness".to_owned(),
            Self::ForeignEqualityWitness => "foreign-equality-witness".to_owned(),
        }
    }

    /// The position of this reason in FR-390's reason order, the last key of
    /// the order view creation reports the first unsupported projection by.
    fn rank(&self) -> u8 {
        match self {
            Self::MissingTypeRepresentation => 0,
            Self::MissingIdentityWitness => 1,
            Self::UnknownIdentityWitness => 2,
            Self::ForeignIdentityWitness => 3,
            Self::MissingEqualityWitness => 4,
            Self::UnknownEqualityWitness => 5,
            Self::ForeignEqualityWitness => 6,
            Self::MissingLiteralProjection => 7,
            Self::MissingOperationProjection { .. } => 8,
            Self::MissingLocationProjection(_) => 9,
        }
    }
}

impl From<crate::bare::IdentityWitnessRefusal> for UnsupportedReason {
    fn from(refusal: crate::bare::IdentityWitnessRefusal) -> Self {
        use crate::bare::IdentityWitnessRefusal as Refusal;
        match refusal {
            Refusal::Missing => Self::MissingIdentityWitness,
            Refusal::Unknown => Self::UnknownIdentityWitness,
            Refusal::Foreign => Self::ForeignIdentityWitness,
        }
    }
}

impl From<EqualityWitnessRefusal> for UnsupportedReason {
    fn from(refusal: EqualityWitnessRefusal) -> Self {
        match refusal {
            EqualityWitnessRefusal::Missing => Self::MissingEqualityWitness,
            EqualityWitnessRefusal::Unknown => Self::UnknownEqualityWitness,
            EqualityWitnessRefusal::Foreign => Self::ForeignEqualityWitness,
        }
    }
}

/// The evidence of a [`UnsupportedReason::MissingLocationProjection`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissingLocation {
    /// The slot's type-declaration origin name.
    pub origin: String,
    /// The application's structural key.
    pub application: NodeKey,
    /// The unpaired visit number, or the unpaired row's index among the
    /// application's rows of this origin.
    pub index: usize,
    /// The application's rows of this origin.
    pub rows: usize,
    /// The application's visits in this slot's walk.
    pub visits: usize,
    /// The producer-defect classification, `invalid_package`/`invalid-value`.
    pub defect: (&'static str, &'static str),
    /// `/checked_term_data/protocol_locations/<index>/location/origin` of the
    /// origin's first row for the application, or
    /// `/checked_term_data/protocol_locations` when it has none.
    pub pointer: String,
}

/// Why view creation refused the whole package.
pub(crate) enum PackageRefusal {
    Unsupported(UnsupportedView),
    Fault(InternalFault),
}

/// One slot to project: its root, origin name and binders in local-slot order.
pub(crate) struct SlotRequest {
    pub(crate) root: NodeKey,
    pub(crate) origin: String,
    pub(crate) binders: Vec<NodeKey>,
}

/// Project every slot of `admitted` and pair each projected slot's walk with
/// its origin's `protocol_locations` rows (FR-390 view creation).
pub(crate) fn project_package(
    source_graph: &qsl_semantics::check::CheckedGraph,
    admitted: &CheckedPackageV2,
    emission: &crate::Emission,
    package: &str,
    requests: Vec<SlotRequest>,
    work_limit: u64,
) -> Result<ReaderProtocolSlots, PackageRefusal> {
    let index = WireIndex::new(
        ProjectionSource::Source {
            graph: source_graph,
            emission,
        },
        admitted,
        work_limit,
    )
    .map_err(PackageRefusal::Fault)?;
    let rows = OriginRows::new(admitted);
    let mut slots = BTreeMap::new();
    let mut refusal: Option<UnsupportedView> = None;
    for request in requests {
        let slot = match index.project_slot(&request) {
            Ok(mut projection) => {
                let found = rows.pair(&mut projection, &request.origin, package, &index)?;
                refusal = first_refusal(refusal, found);
                ReaderSlot::Projected(projection)
            }
            Err(BuildError::Unsupported { reason, node }) => {
                let unsupported = UnsupportedView {
                    package: package.to_owned(),
                    locus: ViewLocus::node(index.key(&node).map_err(PackageRefusal::Fault)?),
                    reason,
                };
                refusal = first_refusal(refusal, Some(unsupported.clone()));
                ReaderSlot::Unsupported(unsupported)
            }
            Err(BuildError::Fault(fault)) => return Err(PackageRefusal::Fault(fault)),
            Err(BuildError::Equality(_)) => {
                return Err(PackageRefusal::Fault(self::fault(
                    "source-route-equality-has-producer-rows",
                )));
            }
        };
        slots.insert((request.root, request.origin), slot);
    }
    match refusal {
        Some(view) => Err(PackageRefusal::Unsupported(view)),
        None => Ok(ReaderProtocolSlots { slots }),
    }
}

/// Whether `identity` names one of the equality operations whose decision
/// the equality rows carry.
fn is_equality_identity(identity: &str) -> bool {
    matches!(
        identity,
        "quire.op.string.eq"
            | "quire.op.string.ne"
            | "quire.op.uuid.eq"
            | "quire.op.uuid.ne"
            | "quire.op.timestamp.eq"
            | "quire.op.timestamp.ne"
            | "quire.op.boolean.eq"
            | "quire.op.boolean.ne"
            | "quire.op.integer.eq"
            | "quire.op.integer.ne"
            | "quire.op.rational.eq"
            | "quire.op.rational.ne"
            | "quire.op.decimal.eq"
            | "quire.op.decimal.ne"
            | "quire.op.quantity.eq"
            | "quire.op.quantity.ne"
            | "quire.op.structural.eq"
            | "quire.op.structural.ne"
    )
}

/// The refusal FR-390 reports: the first by [`UnsupportedView::order`], never
/// the walk order that found it.
fn first_refusal(
    current: Option<UnsupportedView>,
    found: Option<UnsupportedView>,
) -> Option<UnsupportedView> {
    match (current, found) {
        (Some(left), Some(right)) => Some(if right.order() < left.order() {
            right
        } else {
            left
        }),
        (left, right) => left.or(right),
    }
}

enum BuildError {
    Unsupported {
        reason: UnsupportedReason,
        node: CheckedNodeId,
    },
    /// The bytes route holds no admitted equality decision for an
    /// application (FR-531 "The equality file").
    Equality(EqualityWitnessRefusal),
    Fault(InternalFault),
}

impl From<InternalFault> for BuildError {
    fn from(fault: InternalFault) -> Self {
        Self::Fault(fault)
    }
}

fn unsupported_operation(node: &CheckedNodeId, operation: impl Into<String>) -> BuildError {
    BuildError::Unsupported {
        reason: UnsupportedReason::MissingOperationProjection {
            operation: operation.into(),
        },
        node: node.clone(),
    }
}

/// The admitted package's nodes indexed by id and key, built once per view.
struct WireIndex<'wire> {
    source: ProjectionSource<'wire>,
    admitted: &'wire CheckedPackageV2,
    nodes: BTreeMap<&'wire CheckedNodeId, &'wire CheckedSemanticNodeV2>,
    ids: BTreeMap<NodeKey, &'wire CheckedNodeId>,
    graph: WireGraph,
    identities: quire_contract_model::SelectedExactIdentities,
    /// The producer's equality rows, rejoined once on the first equality.
    equality_rows: std::cell::OnceCell<crate::emit::equality_reader::EqualityRows>,
    work_limit: u64,
}

/// What the projection reads beside the admitted package.
pub(crate) enum ProjectionSource<'wire> {
    /// The in-process route: the checked source graph's identity rows and
    /// presence scope, and the emission's equality rows.
    Source {
        graph: &'wire qsl_semantics::check::CheckedGraph,
        emission: &'wire crate::Emission,
    },
    /// The bytes route: the identity witness the supply gave and, where the
    /// handoff carried one, the equality rows (FR-531).
    Bytes {
        identities: quire_contract_model::SelectedExactIdentities,
        equality: Option<&'wire ReadEqualityRows>,
    },
}

impl<'wire> WireIndex<'wire> {
    fn new(
        source: ProjectionSource<'wire>,
        admitted: &'wire CheckedPackageV2,
        work_limit: u64,
    ) -> Result<Self, InternalFault> {
        let (source, identities) = match source {
            ProjectionSource::Source { graph, emission } => {
                let rows = emission.identity_rows(admitted)
                    .map_err(|_| fault("reader-selected-identities-are-admitted"))?;
                let identities = rows.selected(admitted)
                    .map_err(|_| fault("reader-selected-identities-are-admitted"))?;
                (ProjectionSource::Source { graph, emission }, identities)
            }
            ProjectionSource::Bytes {
                identities,
                equality,
            } => (
                ProjectionSource::Bytes {
                    identities: identities.clone(),
                    equality,
                },
                identities,
            ),
        };
        let graph = crate::checked_v2::wire_graph(admitted)
            .map_err(|_| fault("reader-protocol-graph-has-structural-keys"))?;
        let mut nodes = BTreeMap::new();
        let mut ids = BTreeMap::new();
        for node in &admitted.graph().nodes {
            nodes.insert(&node.node_id, node);
            let key = graph
                .key(&node.node_id)
                .map_err(|_| fault("reader-protocol-graph-has-structural-keys"))?;
            ids.insert(key, &node.node_id);
        }
        Ok(Self {
            source,
            admitted,
            nodes,
            ids,
            graph,
            identities,
            equality_rows: std::cell::OnceCell::new(),
            work_limit,
        })
    }

    fn node(&self, id: &CheckedNodeId) -> Option<&'wire CheckedSemanticNodeV2> {
        self.nodes.get(id).copied()
    }

    /// The admitted equality decision of application `id`: from the
    /// emission's producer rows on the source route, from the handoff's
    /// equality file on the bytes route (FR-531 "The equality file").
    fn equality_decision(
        &self,
        admitted: &CheckedPackageV2,
        id: &CheckedNodeId,
    ) -> Result<quire_semantic_value::declaration::AdmittedEqualityDecision, BuildError> {
        match &self.source {
            ProjectionSource::Source { emission, .. } => {
                let row = emission
                    .equality_producer_identity_rows()
                    .iter()
                    .find(|row| row.checked_node() == id)
                    .ok_or_else(|| fault("reader-equality-has-producer-identity-row"))?;
                let rows = self.equality_rows.get_or_init(|| emission.equality_rows());
                emission
                    .admitted_equality_from(rows, admitted, id, row.occurrence(), self.work_limit)
                    .map_err(|_| fault("reader-equality-decision-is-admitted").into())
            }
            ProjectionSource::Bytes { equality, .. } => equality
                .ok_or(BuildError::Equality(EqualityWitnessRefusal::Missing))?
                .decision(admitted, id, self.work_limit)
                .map_err(BuildError::Equality),
        }
    }

    /// The equality-witness refusal of
    /// the equality applications reachable from `root`, whatever the walk
    /// reached first (FR-390-AC-67: `Package` before `Node`).
    fn equality_refusal(&self, root: &CheckedNodeId) -> Option<EqualityWitnessRefusal> {
        let mut seen = BTreeSet::new();
        let mut pending = vec![root.clone()];
        let mut first: Option<EqualityWitnessRefusal> = None;
        while let Some(id) = pending.pop() {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Some(node) = self.node(&id) else { continue };
            pending.extend(node.body.reference_targets().into_iter().cloned());
            let is_equality = matches!(
                &node.body,
                CheckedTerm::Application { operation, .. }
                    if operation.identity().is_some_and(is_equality_identity)
            );
            if !is_equality {
                continue;
            }
            if let Err(BuildError::Equality(refusal)) = self.equality_decision(self.admitted, &id) {
                // One supply gives one reason: Missing (no rows) and Foreign
                // (another package's rows) hold for every application, and a
                // present supply can only fail as Unknown.
                first.get_or_insert(refusal);
            }
        }
        first
    }

    fn types(&self) -> TypeReader<'_> {
        TypeReader {
            admitted: self.admitted,
            identities: &self.identities,
            nodes: &self.nodes,
            graph: &self.graph,
            work_limit: self.work_limit,
        }
    }

    fn key(&self, id: &CheckedNodeId) -> Result<NodeKey, InternalFault> {
        self.graph
            .key(id)
            .map_err(|_| fault("reader-node-has-structural-key"))
    }

    fn project_slot(&self, request: &SlotRequest) -> Result<ReaderProtocolProjection, BuildError> {
        let root = *self
            .ids
            .get(&request.root)
            .ok_or_else(|| fault("reader-selected-protocol-node-is-retained"))?;
        let mut locals = BTreeMap::new();
        let mut local_keys = BTreeMap::new();
        for (slot, binder) in request.binders.iter().enumerate() {
            let id = *self
                .ids
                .get(binder)
                .ok_or_else(|| fault("reader-protocol-binder-is-retained"))?;
            locals.insert(id.clone(), slot);
            local_keys.insert(binder.clone(), slot);
        }
        let mut builder = ReaderBuilder {
            index: self,
            nodes: Vec::new(),
            functions: Vec::new(),
            function_indexes: BTreeMap::new(),
            memo: BTreeMap::new(),
            root_slots: request.binders.len(),
        };
        builder.read_functions(root)?;
        let root_index = builder.project_node(root, Context::Root, &locals)?;
        Ok(ReaderProtocolProjection {
            nodes: builder.nodes,
            functions: builder.functions,
            root: root_index,
            frame_slots: builder.root_slots,
            locals: local_keys,
        })
    }
}

/// One application's rows of one origin, each with its index among all rows.
type ApplicationRows<'wire> = Vec<(usize, &'wire CheckedProtocolSemanticLocation)>;

/// The admitted `protocol_locations` rows by origin name and application,
/// each application's rows in ascending (occurrence, `term_ordinal`) order.
struct OriginRows<'wire> {
    rows: BTreeMap<&'wire str, BTreeMap<&'wire CheckedNodeId, ApplicationRows<'wire>>>,
}

const ROWS_POINTER: &str = "/checked_term_data/protocol_locations";

impl<'wire> OriginRows<'wire> {
    fn new(admitted: &'wire CheckedPackageV2) -> Self {
        let mut rows: BTreeMap<_, BTreeMap<_, Vec<_>>> = BTreeMap::new();
        let all = admitted
            .checked_term_data()
            .map_or(&[][..], |data| data.protocol_locations.as_slice());
        for (position, row) in all.iter().enumerate() {
            let quire_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name } =
                &row.location.origin
            else {
                // A protocol-attempt origin names no slot.
                continue;
            };
            rows.entry(name.as_ref())
                .or_default()
                .entry(&row.application)
                .or_default()
                .push((position, row));
        }
        for applications in rows.values_mut() {
            for group in applications.values_mut() {
                group.sort_by(|(_, left), (_, right)| {
                    (&left.occurrence.ordinal, left.term_ordinal)
                        .cmp(&(&right.occurrence.ordinal, right.term_ordinal))
                });
            }
        }
        Self { rows }
    }

    /// Pair visit `k` of each application in the slot's walk with the
    /// application's row `k` of the slot's origin, and refuse a visit
    /// without a row or a row without a visit (FR-390-AC-33..38, AC-57).
    fn pair(
        &self,
        projection: &mut ReaderProtocolProjection,
        origin: &str,
        package: &str,
        index: &WireIndex<'_>,
    ) -> Result<Option<UnsupportedView>, PackageRefusal> {
        let order = walk(&projection.nodes, projection.root);
        let empty = BTreeMap::new();
        let applications = self.rows.get(origin).unwrap_or(&empty);
        let mut visits: BTreeMap<CheckedNodeId, usize> = BTreeMap::new();
        for &position in &order {
            for application in projection.nodes[position].terms() {
                *visits.entry(application.clone()).or_default() += 1;
            }
        }
        let missing = |application: &CheckedNodeId, at: usize, rows: usize, visits: usize| {
            let pointer = applications
                .get(application)
                .and_then(|group| group.first())
                .map_or_else(
                    || ROWS_POINTER.to_owned(),
                    |(row, _)| format!("{ROWS_POINTER}/{row}/location/origin"),
                );
            match index.key(application) {
                Ok(key) => Ok(UnsupportedView {
                    package: package.to_owned(),
                    locus: ViewLocus::node(key.clone()),
                    reason: UnsupportedReason::MissingLocationProjection(Box::new(
                        MissingLocation {
                            origin: origin.to_owned(),
                            application: key,
                            index: at,
                            rows,
                            visits,
                            defect: ("invalid_package", "invalid-value"),
                            pointer,
                        },
                    )),
                }),
                Err(fault) => Err(PackageRefusal::Fault(fault)),
            }
        };
        let mut refusal = None;
        let mut seen: BTreeMap<CheckedNodeId, usize> = BTreeMap::new();
        for &position in &order {
            let terms: Vec<CheckedNodeId> = projection.nodes[position].terms().cloned().collect();
            for application in terms {
                let visit = seen.entry(application.clone()).or_default();
                let group = applications
                    .get(&application)
                    .map_or(&[][..], Vec::as_slice);
                let total = visits.get(&application).copied().unwrap_or_default();
                let Some((_, row)) = group.get(*visit) else {
                    refusal = first_refusal(
                        refusal,
                        Some(missing(&application, *visit, group.len(), total)?),
                    );
                    continue;
                };
                let location = location(&row.location)
                    .ok_or_else(|| PackageRefusal::Fault(fault("reader-location-ordinal-fits")))?;
                *visit += 1;
                // The node's own term is its last; an absorbed conversion's
                // row is paired and consumed before it.
                if projection.nodes[position].term.as_ref() == Some(&application) {
                    projection.nodes[position].location = Some(location);
                }
            }
        }
        for (application, group) in applications {
            let total = visits.get(*application).copied().unwrap_or_default();
            if group.len() > total {
                refusal = first_refusal(
                    refusal,
                    Some(missing(application, total, group.len(), total)?),
                );
            }
        }
        Ok(refusal)
    }
}

/// Why the walk of a bytes-route slot produced no visits.
pub(crate) enum WalkFailure {
    /// An admitted form, operation or type without a projection.
    Unsupported(UnsupportedView),
    /// The equality rows do not give an application its decision.
    Equality(EqualityWitnessRefusal),
    /// The admitted package broke a projection invariant.
    Fault(InternalFault),
}

/// The slot walks of a package read from its bytes alone (FR-533 "Slot
/// naming on the bytes route"): the projection of FR-390 over the package's
/// own nodes, the identity witness and the equality rows.
pub(crate) struct BareWalker<'wire> {
    index: WireIndex<'wire>,
}

impl<'wire> BareWalker<'wire> {
    pub(crate) fn new(
        admitted: &'wire CheckedPackageV2,
        identities: quire_contract_model::SelectedExactIdentities,
        equality: Option<&'wire ReadEqualityRows>,
        work_limit: u64,
    ) -> Result<Self, InternalFault> {
        Ok(Self {
            index: WireIndex::new(
                ProjectionSource::Bytes {
                    identities,
                    equality,
                },
                admitted,
                work_limit,
            )?,
        })
    }

    /// `project_slot`, reporting the first refusal of the whole walk by
    /// FR-390-AC-67's order: an equality-witness refusal anywhere in the
    /// walk (a `Package` locus) is reported before a `Node` refusal the walk
    /// reached first.
    fn slot(&self, request: &SlotRequest) -> Result<ReaderProtocolProjection, WalkFailure> {
        let failure = match self.index.project_slot(request) {
            Ok(projection) => return Ok(projection),
            Err(BuildError::Fault(fault)) => return Err(WalkFailure::Fault(fault)),
            Err(failure) => failure,
        };
        let root = self.index.ids.get(&request.root).ok_or_else(|| {
            WalkFailure::Fault(fault("reader-selected-protocol-node-is-retained"))
        })?;
        if let Some(refusal) = self.index.equality_refusal(root) {
            return Err(WalkFailure::Equality(refusal));
        }
        match failure {
            BuildError::Equality(refusal) => Err(WalkFailure::Equality(refusal)),
            BuildError::Unsupported { reason, node } => {
                Err(WalkFailure::Unsupported(UnsupportedView {
                    package: self.index.admitted.name().to_owned(),
                    locus: ViewLocus::node(self.index.key(&node).map_err(WalkFailure::Fault)?),
                    reason,
                }))
            }
            BuildError::Fault(fault) => Err(WalkFailure::Fault(fault)),
        }
    }

    /// The applications the walk of the slot rooted at `root` visits, each
    /// with its number of visits (FR-383 "Protocol term locations").
    pub(crate) fn visits(
        &self,
        root: &NodeKey,
        binders: &[NodeKey],
    ) -> Result<BTreeMap<CheckedNodeId, usize>, WalkFailure> {
        let request = SlotRequest {
            root: root.clone(),
            origin: String::new(),
            binders: binders.to_vec(),
        };
        let projection = self.slot(&request)?;
        let mut visits: BTreeMap<CheckedNodeId, usize> = BTreeMap::new();
        for position in walk(&projection.nodes, projection.root) {
            for application in projection.nodes[position].terms() {
                *visits.entry(application.clone()).or_default() += 1;
            }
        }
        Ok(visits)
    }

    /// The projection of the slot rooted at `root` under `origin` (`None`
    /// for a slot with no application to locate), its applications paired
    /// with the origin's `protocol_locations` rows (FR-390 view creation,
    /// FR-533). The first unsupported projection by [`UnsupportedView::order`]
    /// is the failure.
    pub(crate) fn project(
        &self,
        root: &NodeKey,
        origin: Option<&str>,
        binders: &[NodeKey],
    ) -> Result<ReaderProtocolProjection, WalkFailure> {
        let package = self.index.admitted.name();
        let request = SlotRequest {
            root: root.clone(),
            origin: origin.unwrap_or_default().to_owned(),
            binders: binders.to_vec(),
        };
        let mut projection = self.slot(&request)?;
        let rows = OriginRows::new(self.index.admitted);
        match rows.pair(&mut projection, &request.origin, package, &self.index) {
            Ok(None) => Ok(projection),
            Ok(Some(view)) => Err(WalkFailure::Unsupported(view)),
            Err(PackageRefusal::Unsupported(view)) => Err(WalkFailure::Unsupported(view)),
            Err(PackageRefusal::Fault(fault)) => Err(WalkFailure::Fault(fault)),
        }
    }

    /// The number of rows each type-declaration origin holds for each
    /// application, by origin name.
    pub(crate) fn origin_rows(&self) -> BTreeMap<&'wire str, BTreeMap<CheckedNodeId, usize>> {
        OriginRows::new(self.index.admitted)
            .rows
            .into_iter()
            .map(|(origin, applications)| {
                (
                    origin,
                    applications
                        .into_iter()
                        .map(|(application, rows)| (application.clone(), rows.len()))
                        .collect(),
                )
            })
            .collect()
    }

    /// The key of `application`, as the package's wire graph spells it.
    pub(crate) fn key(&self, application: &CheckedNodeId) -> Result<NodeKey, InternalFault> {
        self.index.key(application)
    }
}

/// The positions of a slot in FR-383's walk order: pre-order, children in
/// `NodeKind` operand order.
fn walk(nodes: &[ReaderProjectedNode], root: usize) -> Vec<usize> {
    let mut order = Vec::with_capacity(nodes.len());
    let mut stack = vec![root];
    while let Some(index) = stack.pop() {
        order.push(index);
        if let Some(node) = nodes.get(index) {
            stack.extend(node.children.iter().rev().copied());
        }
    }
    order
}

/// Where a projected node lives: the slot's own walk, or one function body.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Context {
    Root,
    Function(usize),
}

struct ReaderBuilder<'index, 'wire> {
    index: &'index WireIndex<'wire>,
    nodes: Vec<ReaderProjectedNode>,
    functions: Vec<ReaderFunction>,
    function_indexes: BTreeMap<CheckedNodeId, usize>,
    /// Function bodies share a node reached twice; the slot's own walk
    /// holds one position per parent (FR-390).
    memo: BTreeMap<(usize, CheckedNodeId), usize>,
    root_slots: usize,
}

impl<'wire> ReaderBuilder<'_, 'wire> {
    fn admitted(&self) -> &'wire CheckedPackageV2 {
        self.index.admitted
    }

    fn node(&self, id: &CheckedNodeId) -> Result<&'wire CheckedSemanticNodeV2, InternalFault> {
        self.index
            .node(id)
            .ok_or_else(|| fault("reader-expression-child-is-admitted"))
    }

    fn read_functions(&mut self, root: &CheckedNodeId) -> Result<(), BuildError> {
        let mut reachable = BTreeSet::new();
        let mut pending = vec![root.clone()];
        while let Some(id) = pending.pop() {
            if !reachable.insert(id.clone()) {
                continue;
            }
            let node = self
                .index
                .node(&id)
                .ok_or_else(|| fault("reader-reachable-node-is-admitted"))?;
            pending.extend(node.body.reference_targets().into_iter().cloned());
        }
        let declarations = reachable
            .iter()
            .filter_map(|id| self.index.node(id))
            .filter(|node| has_form(node, WireTag::Function, WireForm::PureFunction))
            .map(|node| {
                let (parameters, body) = function_parts(&node.body)
                    .ok_or_else(|| fault("reader-pure-function-has-checked-body"))?;
                let parameters = parameters
                    .iter()
                    .map(|parameter| match parameter {
                        CheckedTerm::Reference { target } => Ok(target.clone()),
                        _ => Err(fault("reader-function-parameter-has-checked-identity")),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let body = match body {
                    CheckedTerm::Reference { target } => target.clone(),
                    _ => return Err(fault("reader-function-body-has-checked-identity")),
                };
                let name = node
                    .declaration
                    .as_ref()
                    .and_then(|declaration| declaration.qualified_name.last())
                    .map(|name| name.to_string())
                    .ok_or_else(|| fault("reader-function-name-is-admitted"))?;
                Ok((node.node_id.clone(), name, parameters, body))
            })
            .collect::<Result<Vec<_>, InternalFault>>()?;

        for (node, name, parameters, _) in &declarations {
            let index = self.functions.len();
            self.function_indexes.insert(node.clone(), index);
            self.functions.push(ReaderFunction {
                name: name.clone(),
                body: usize::MAX,
                slots: parameters.len(),
            });
        }
        for (index, (_, _, parameters, body)) in declarations.iter().enumerate() {
            let locals = parameters
                .iter()
                .enumerate()
                .map(|(slot, id)| (id.clone(), slot))
                .collect::<BTreeMap<_, _>>();
            let body = self.project_node(body, Context::Function(index), &locals)?;
            self.functions[index].body = body;
        }
        Ok(())
    }

    fn push(&mut self, node: ReaderProjectedNode) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    fn project_node(
        &mut self,
        id: &CheckedNodeId,
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<usize, BuildError> {
        if let Context::Function(function) = context {
            if let Some(index) = self.memo.get(&(function, id.clone())) {
                return Ok(*index);
            }
        }
        let index = self.project_fresh(id, context, locals)?;
        if let Context::Function(function) = context {
            self.memo.insert((function, id.clone()), index);
        }
        Ok(index)
    }

    fn project_fresh(
        &mut self,
        id: &CheckedNodeId,
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<usize, BuildError> {
        if let Some(slot) = locals.get(id).copied() {
            let value_type = self.value_type(id)?;
            return Ok(self.push(ReaderProjectedNode::leaf(NodeKind::Local(slot), value_type)));
        }
        let node = self.node(id)?;
        let (kind, children, term) = match &node.body {
            CheckedTerm::Literal { .. } => {
                let literal = node
                    .body
                    .literal_read()
                    .and_then(|literal| literal_value(literal.value, literal.type_id, self.index))
                    .ok_or_else(|| BuildError::Unsupported {
                        reason: UnsupportedReason::MissingLiteralProjection,
                        node: id.clone(),
                    })?;
                (NodeKind::Literal(CheckedLiteral(literal)), Vec::new(), None)
            }
            CheckedTerm::Aggregate { members }
                if has_form(node, WireTag::Value, WireForm::UnionValue) =>
            {
                let (kind, children) = self.union_value(node, members, context, locals)?;
                (kind, children, None)
            }
            CheckedTerm::Aggregate { members }
                if has_form(node, WireTag::Value, WireForm::TupleValue) =>
            {
                let value_type = self.value_type(id)?;
                let ValueType::Composite(declaration) = &value_type else {
                    return Err(fault("reader-tuple-value-has-declared-type").into());
                };
                let children = members
                    .iter()
                    .map(|member| self.project_argument(id, member, context, locals))
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    NodeKind::Tuple {
                        declaration: declaration.clone(),
                        arguments: children.clone(),
                    },
                    children,
                    None,
                )
            }
            CheckedTerm::Aggregate { members }
                if has_form(node, WireTag::Value, WireForm::OptionValue) =>
            {
                let [member] = members.as_slice() else {
                    return Err(fault("reader-option-value-has-one-member").into());
                };
                let child = self.project_argument(id, member, context, locals)?;
                (NodeKind::OptionSome(child), vec![child], None)
            }
            CheckedTerm::Aggregate { members }
                if has_form(node, WireTag::Value, WireForm::CollectionValue) =>
            {
                let value_type = self.value_type(id)?;
                let ValueType::Collection(collection_type) = &value_type else {
                    return Err(fault("reader-collection-value-has-collection-type").into());
                };
                let children = members
                    .iter()
                    .map(|member| self.project_argument(id, member, context, locals))
                    .collect::<Result<Vec<_>, _>>()?;
                (
                    NodeKind::Collection {
                        collection_type: (**collection_type).clone(),
                        elements: children.clone(),
                    },
                    children,
                    None,
                )
            }
            CheckedTerm::Aggregate { members } => {
                let (kind, children) = self.record_value(id, members, context, locals)?;
                (kind, children, None)
            }
            CheckedTerm::Reference { target } => {
                if let Some(slot) = locals.get(target).copied() {
                    (NodeKind::Local(slot), Vec::new(), None)
                } else {
                    return self.project_node(target, context, locals);
                }
            }
            CheckedTerm::Application {
                operation,
                arguments,
                ..
            } => {
                let identity = operation.identity();
                let (kind, children) =
                    self.project_application(id, operation, arguments, context, locals)?;
                // FR-383 assumption A1: a `case` (like a `let` or a
                // conditional) is no term and has no row; every other
                // application of the slot's own walk has one.
                let has_location_row = !matches!(
                    identity,
                    Some("quire.op.control.let" | "quire.op.control.if" | "quire.op.control.case")
                );
                let term = (context == Context::Root && has_location_row).then(|| id.clone());
                (kind, children, term)
            }
            _ => {
                return Err(unsupported_operation(
                    id,
                    format!("{}/{}", node.node_tag, node.semantic_form),
                ));
            }
        };
        let value_type = self.value_type(id)?;
        Ok(self.push(ReaderProjectedNode {
            kind,
            value_type,
            location: None,
            children,
            term,
            absorbed: None,
        }))
    }

    fn union_value(
        &mut self,
        node: &CheckedSemanticNodeV2,
        members: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        let union_key = self
            .index
            .graph
            .key(&node.semantic_type)
            .map_err(|_| fault("reader-union-value-has-declaration-key"))?;
        let union_node = self
            .index
            .node(&node.semantic_type)
            .filter(|candidate| has_form(candidate, WireTag::CompositeType, WireForm::Union))
            .ok_or_else(|| fault("reader-union-value-type-is-union"))?;
        let CheckedTerm::Aggregate {
            members: union_members,
        } = &union_node.body
        else {
            return Err(fault("reader-union-declaration-has-checked-members").into());
        };
        let [CheckedTerm::Binding { name, value }] = members else {
            return Err(fault("reader-union-value-has-one-member").into());
        };
        let quire_contract_model::CheckedBindingNameKind::Other(member_name) = name.kind() else {
            return Err(fault("reader-union-value-has-member-name").into());
        };
        let member = member_position(union_members, member_name)
            .ok_or_else(|| fault("reader-union-value-member-is-declared"))?;
        let CheckedTerm::Aggregate { members } = value.as_ref() else {
            return Err(fault("reader-union-value-has-payload-list").into());
        };
        let children = members
            .iter()
            .map(|argument| self.project_argument(&node.node_id, argument, context, locals))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((
            NodeKind::UnionValue {
                declaration: union_key,
                member,
                arguments: children.clone(),
            },
            children,
        ))
    }

    fn record_value(
        &mut self,
        id: &CheckedNodeId,
        members: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        let ValueType::Composite(declaration) = &self.value_type(id)? else {
            return Err(fault("reader-aggregate-has-checked-record-type").into());
        };
        let declaration = declaration.clone();
        let slots = members
            .iter()
            .map(|member| -> Result<RecordSlot<usize>, BuildError> {
                let CheckedTerm::Binding { name, value } = member else {
                    return Err(fault("reader-record-value-has-checked-field-binding").into());
                };
                if !matches!(
                    name.kind(),
                    quire_contract_model::CheckedBindingNameKind::Other(_)
                ) {
                    return Err(fault("reader-record-value-has-declared-field-names").into());
                }
                Ok(RecordSlot::Present(
                    self.project_argument(id, value, context, locals)?,
                ))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let children = slots
            .iter()
            .filter_map(|slot| match slot {
                RecordSlot::Present(child) => Some(*child),
                RecordSlot::Absent | RecordSlot::Null => None,
            })
            .collect();
        Ok((NodeKind::Record { declaration, slots }, children))
    }

    /// `#[string_edge]`: operation names are the admitted wire's fixed
    /// discriminator vocabulary, not source text selected at runtime.
    #[ix_trace_rs::string_edge]
    fn project_application(
        &mut self,
        id: &CheckedNodeId,
        operation: &quire_contract_model::CheckedApplicationOperation,
        arguments: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        let identity = operation.identity();
        match identity {
            Some("quire.op.control.let") => {
                let [CheckedTerm::Binding { name, value }, body] = arguments else {
                    return Err(fault("reader-let-has-binding-and-body").into());
                };
                let binder = self.binder_node(name.as_ref(), body)?;
                let slot = self.next_slot(context, locals);
                let value = self.project_argument(id, value, context, locals)?;
                let mut body_locals = locals.clone();
                body_locals.extend(binder.map(|binder| (binder, slot)));
                let body = self.project_argument(id, body, context, &body_locals)?;
                self.grow_frame(context, slot.saturating_add(1));
                Ok((NodeKind::Let { slot, value, body }, vec![value, body]))
            }
            Some("quire.op.function.call") => {
                let callee = arguments
                    .first()
                    .and_then(reference_target)
                    .ok_or_else(|| fault("reader-call-has-callee"))?;
                let function = self
                    .function_indexes
                    .get(callee)
                    .copied()
                    .ok_or_else(|| fault("reader-call-callee-is-admitted"))?;
                let children = self.project_arguments(id, arguments.get(1..), context, locals)?;
                Ok((
                    NodeKind::Call {
                        function,
                        arguments: children.clone(),
                    },
                    children,
                ))
            }
            Some("quire.op.boolean.and" | "quire.op.boolean.or" | "quire.op.boolean.implies") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-connective-has-two-operands").into());
                };
                let connective = match identity {
                    Some("quire.op.boolean.and") => Connective::And,
                    Some("quire.op.boolean.or") => Connective::Or,
                    _ => Connective::Implies,
                };
                Ok((
                    NodeKind::Connective(connective, ConnectiveOrigin::Authored, *left, *right),
                    children,
                ))
            }
            Some("quire.op.boolean.not") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-not-has-one-operand").into());
                };
                Ok((NodeKind::Not(*operand), children))
            }
            Some("quire.op.control.if") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [condition, then, otherwise] = children.as_slice() else {
                    return Err(fault("reader-conditional-has-three-operands").into());
                };
                Ok((
                    NodeKind::If {
                        condition: *condition,
                        then: *then,
                        otherwise: *otherwise,
                    },
                    children,
                ))
            }
            Some("quire.op.integer.negate") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-integer-negation-has-one-operand").into());
                };
                Ok((NodeKind::Negate(*operand), children))
            }
            Some("quire.op.integer.div" | "quire.op.integer.rem" | "quire.op.integer.mod") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-integer-division-has-two-operands").into());
                };
                let operator = match identity {
                    Some("quire.op.integer.div") => IntegerDivisionOperator::Div,
                    Some("quire.op.integer.rem") => IntegerDivisionOperator::Rem,
                    Some("quire.op.integer.mod") => IntegerDivisionOperator::Mod,
                    _ => return Err(fault("reader-integer-division-identity-is-catalogued").into()),
                };
                let selection = if operator == IntegerDivisionOperator::Mod {
                    None
                } else {
                    let references = operation
                        .law_definitions()
                        .map_err(|_| fault("reader-integer-division-law-is-admitted"))?;
                    let definitions = references
                        .iter()
                        .map(|reference| qsl_semantics::value::DefinitionReference {
                            authority: reference.authority.to_string(),
                            identity: reference.identity.to_string(),
                        })
                        .collect::<Vec<_>>();
                    Some(
                        qsl_semantics::value::admit_integer_division(&definitions, None)
                            .map_err(|_| fault("reader-integer-division-law-is-admitted"))?,
                    )
                };
                Ok((
                    NodeKind::IntegerDivision {
                        operator,
                        selection,
                        left: *left,
                        right: *right,
                    },
                    children,
                ))
            }
            Some("quire.op.integer.add" | "quire.op.integer.sub" | "quire.op.integer.mul") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-integer-arithmetic-has-two-operands").into());
                };
                let operator = match identity {
                    Some("quire.op.integer.add") => Arithmetic::Add,
                    Some("quire.op.integer.sub") => Arithmetic::Subtract,
                    _ => Arithmetic::Multiply,
                };
                Ok((NodeKind::Arithmetic(operator, *left, *right), children))
            }
            Some("quire.op.rational.div") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-rational-division-has-two-operands").into());
                };
                let value_type = self.value_type(id)?;
                let ValueType::Rational(domain) = &value_type else {
                    return Err(fault("reader-rational-division-has-rational-result").into());
                };
                Ok((
                    NodeKind::Divide {
                        left: *left,
                        right: *right,
                        domain: domain.clone(),
                    },
                    children,
                ))
            }
            Some("quire.op.rational.add" | "quire.op.rational.sub" | "quire.op.rational.mul") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-rational-arithmetic-has-two-operands").into());
                };
                let value_type = self.value_type(id)?;
                let ValueType::Rational(domain) = &value_type else {
                    return Err(fault("reader-rational-arithmetic-has-rational-result").into());
                };
                let operator = identity
                    .and_then(arithmetic_operator)
                    .ok_or_else(|| fault("reader-rational-arithmetic-is-catalogued"))?;
                Ok((
                    NodeKind::Rational {
                        operator,
                        left: *left,
                        right: *right,
                        domain: domain.clone(),
                    },
                    children,
                ))
            }
            Some(
                "quire.op.decimal.add"
                | "quire.op.decimal.sub"
                | "quire.op.decimal.mul"
                | "quire.op.decimal.div",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-decimal-arithmetic-has-two-operands").into());
                };
                let value_type = self.value_type(id)?;
                let ValueType::Decimal(target) = &value_type else {
                    return Err(fault("reader-decimal-arithmetic-has-decimal-result").into());
                };
                let operator = identity
                    .and_then(arithmetic_operator)
                    .ok_or_else(|| fault("reader-decimal-arithmetic-is-catalogued"))?;
                Ok((
                    NodeKind::Decimal {
                        operator,
                        left: *left,
                        right: *right,
                        target: target.clone(),
                    },
                    children,
                ))
            }
            Some("quire.op.rational.negate") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-rational-negation-has-one-operand").into());
                };
                let value_type = self.value_type(id)?;
                let ValueType::Rational(domain) = &value_type else {
                    return Err(fault("reader-rational-negation-has-rational-result").into());
                };
                Ok((NodeKind::RationalNegate(*operand, domain.clone()), children))
            }
            Some("quire.op.decimal.negate") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-decimal-negation-has-one-operand").into());
                };
                let value_type = self.value_type(id)?;
                let ValueType::Decimal(target) = &value_type else {
                    return Err(fault("reader-decimal-negation-has-decimal-result").into());
                };
                Ok((NodeKind::DecimalNegate(*operand, target.clone()), children))
            }
            Some("quire.op.option.present") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-presence-test-has-one-operand").into());
                };
                Ok((NodeKind::Present(*operand), children))
            }
            Some("quire.op.option.value") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-option-value-has-one-operand").into());
                };
                Ok((NodeKind::Value(*operand), children))
            }
            Some("quire.op.collection.size") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-collection-size-has-one-operand").into());
                };
                Ok((NodeKind::Size(*operand), children))
            }
            Some("quire.op.collection.contains") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [collection, item] = children.as_slice() else {
                    return Err(fault("reader-collection-containment-has-two-operands").into());
                };
                Ok((NodeKind::Contains(*collection, *item), children))
            }
            Some(
                "quire.op.collection.map"
                | "quire.op.collection.filter"
                | "quire.op.collection.forall"
                | "quire.op.collection.exists"
                | "quire.op.collection.count"
                | "quire.op.collection.sum.integer",
            ) => {
                let [source_term, CheckedTerm::Binding { name, value }] = arguments else {
                    return Err(fault("reader-collection-query-has-source-and-binder").into());
                };
                let binder = self.binder_node(name.as_ref(), value)?;
                let slot = self.next_slot(context, locals);
                let source = self.project_argument(id, source_term, context, locals)?;
                let mut body_locals = locals.clone();
                body_locals.extend(binder.map(|binder| (binder, slot)));
                let body = self.project_argument(id, value, context, &body_locals)?;
                self.grow_frame(context, slot.saturating_add(1));
                let visit = match identity {
                    Some("quire.op.collection.map") => Visit::Map,
                    Some("quire.op.collection.filter") => Visit::Filter,
                    Some("quire.op.collection.forall") => Visit::Forall,
                    Some("quire.op.collection.exists") => Visit::Exists,
                    Some("quire.op.collection.count") => Visit::Count,
                    Some("quire.op.collection.sum.integer") => Visit::Sum,
                    _ => return Err(fault("reader-collection-query-identity-is-catalogued").into()),
                };
                Ok((
                    NodeKind::Query {
                        visit,
                        slot,
                        source,
                        body,
                    },
                    vec![source, body],
                ))
            }
            Some("quire.op.collection.convert") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-collection-conversion-has-one-operand").into());
                };
                let value_type = self.value_type(id)?;
                let ValueType::Collection(target) = &value_type else {
                    return Err(fault("reader-collection-conversion-has-collection-result").into());
                };
                Ok((
                    NodeKind::ConvertCollection {
                        target: (**target).clone(),
                        operand: *operand,
                    },
                    children,
                ))
            }
            Some("quire.op.collection.flatten") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-collection-flatten-has-one-operand").into());
                };
                Ok((NodeKind::Flatten(*operand), children))
            }
            Some(
                "quire.op.collection.sequence.drop_first"
                | "quire.op.collection.ordered_set.drop_first",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [source] = children.as_slice() else {
                    return Err(fault("reader-drop-first-has-one-source").into());
                };
                Ok((NodeKind::DropFirst(*source), children))
            }
            Some("quire.op.collection.remove_one") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [collection, item] = children.as_slice() else {
                    return Err(fault("reader-remove-one-has-collection-and-item").into());
                };
                Ok((NodeKind::RemoveOne(*collection, *item), children))
            }
            Some("quire.op.model.all_instances") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [population] = children.as_slice() else {
                    return Err(fault("reader-all-instances-has-one-population").into());
                };
                Ok((
                    NodeKind::AllInstances {
                        population: *population,
                    },
                    children,
                ))
            }
            Some("quire.op.model.lookup") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [population, reference] = children.as_slice() else {
                    return Err(fault("reader-lookup-has-population-and-reference").into());
                };
                let mode = operation
                    .mode()
                    .map_err(|_| fault("reader-lookup-absence-mode-is-admitted"))?
                    .filter(|mode| mode.kind == OperationModeKind::Absence)
                    .ok_or_else(|| fault("reader-lookup-has-absence-mode"))?;
                let absence = match mode.value {
                    "undefined" => quire_semantic_value::absence::AbsenceMode::Undefined,
                    "empty" => quire_semantic_value::absence::AbsenceMode::Empty,
                    "refused" => quire_semantic_value::absence::AbsenceMode::Refused,
                    _ => return Err(fault("reader-lookup-absence-mode-is-catalogued").into()),
                };
                Ok((
                    NodeKind::Lookup {
                        population: *population,
                        reference: *reference,
                        absence,
                    },
                    children,
                ))
            }
            Some("quire.op.numeric.convert") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-numeric-conversion-has-one-operand").into());
                };
                let operand_id = arguments
                    .first()
                    .and_then(reference_target)
                    .ok_or_else(|| fault("reader-numeric-conversion-has-checked-operand"))?;
                let source = self.value_type(operand_id)?;
                let target = self.value_type(id)?;
                Ok((
                    NodeKind::ConvertScalar(EqualityOperand::converted(source, target), *operand),
                    children,
                ))
            }
            Some(
                "quire.op.quantity.add"
                | "quire.op.quantity.sub"
                | "quire.op.quantity.mul"
                | "quire.op.quantity.div",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-quantity-arithmetic-has-two-operands").into());
                };
                let operator = match identity {
                    Some("quire.op.quantity.add") => ArithmeticOperator::Add,
                    Some("quire.op.quantity.sub") => ArithmeticOperator::Subtract,
                    Some("quire.op.quantity.mul") => ArithmeticOperator::Multiply,
                    _ => ArithmeticOperator::Divide,
                };
                Ok((NodeKind::Quantity(operator, *left, *right), children))
            }
            Some("quire.op.numeric.narrow") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-narrow-has-one-operand").into());
                };
                let ValueType::Int(target) = &self.value_type(id)? else {
                    return Err(fault("reader-narrow-has-exact-integer-target").into());
                };
                Ok((NodeKind::Coerce(*operand, target.clone()), children))
            }
            Some("quire.op.quantity.convert") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-quantity-conversion-has-one-operand").into());
                };
                let operand_id = arguments
                    .first()
                    .and_then(reference_target)
                    .ok_or_else(|| fault("reader-quantity-conversion-has-checked-operand"))?;
                let source = self.value_type(operand_id)?;
                let target = self.value_type(id)?;
                if !matches!(source, ValueType::Quantity(_))
                    || !matches!(target, ValueType::Quantity(_))
                {
                    return Err(fault("reader-quantity-conversion-has-exact-units").into());
                }
                Ok((
                    NodeKind::ConvertScalar(EqualityOperand::converted(source, target), *operand),
                    children,
                ))
            }
            Some("quire.op.control.case") => self.case(id, arguments, context, locals),
            Some(
                "quire.op.integer.lt"
                | "quire.op.integer.le"
                | "quire.op.integer.gt"
                | "quire.op.integer.ge"
                | "quire.op.rational.lt"
                | "quire.op.rational.le"
                | "quire.op.rational.gt"
                | "quire.op.rational.ge"
                | "quire.op.decimal.lt"
                | "quire.op.decimal.le"
                | "quire.op.decimal.gt"
                | "quire.op.decimal.ge"
                | "quire.op.enum.lt"
                | "quire.op.enum.le"
                | "quire.op.enum.gt"
                | "quire.op.enum.ge"
                | "quire.op.text.lt"
                | "quire.op.text.le"
                | "quire.op.text.gt"
                | "quire.op.text.ge"
                | "quire.op.quantity.lt"
                | "quire.op.quantity.le"
                | "quire.op.quantity.gt"
                | "quire.op.quantity.ge",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-order-has-two-operands").into());
                };
                let (kind, operator) = identity
                    .and_then(order_of)
                    .ok_or_else(|| fault("reader-order-family-is-catalogued"))?;
                Ok((NodeKind::Order(operator, kind, *left, *right), children))
            }
            Some("quire.op.record.project") => {
                self.projection(id, operation, arguments, context, locals)
            }
            Some(name) if is_equality_identity(name) => {
                self.equality(id, identity, arguments, context, locals)
            }
            _ => Err(unsupported_operation(
                id,
                identity.unwrap_or("unidentified operation"),
            )),
        }
    }

    fn project_arguments(
        &mut self,
        id: &CheckedNodeId,
        arguments: Option<&[CheckedTerm]>,
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<Vec<usize>, BuildError> {
        arguments
            .ok_or_else(|| fault("reader-application-has-operands"))?
            .iter()
            .map(|argument| self.project_argument(id, argument, context, locals))
            .collect()
    }

    fn equality(
        &mut self,
        id: &CheckedNodeId,
        identity: Option<&str>,
        arguments: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        // The admitted equality decision applies the authored conversion
        // plan to its operand, so a quantity equality reads the conversion's
        // own operand and the conversion is not applied twice.
        let quantity = matches!(
            identity,
            Some("quire.op.quantity.eq" | "quire.op.quantity.ne")
        );
        let children = arguments
            .iter()
            .map(|argument| {
                let conversion = quantity
                    .then(|| self.quantity_conversion(argument))
                    .flatten();
                let Some((conversion, operand)) = conversion else {
                    return self.project_argument(id, argument, context, locals);
                };
                let (conversion, operand) = (conversion.clone(), operand.clone());
                let child = self.project_argument(id, &operand, context, locals)?;
                // The conversion stays a position of the slot's walk (FR-383)
                // with its own row, held by the operand it converts.
                if context == Context::Root {
                    if let Some(node) = self.nodes.get_mut(child) {
                        node.absorbed = Some(conversion);
                    }
                }
                Ok(child)
            })
            .collect::<Result<Vec<_>, BuildError>>()?;
        let decision = self.index.equality_decision(self.admitted(), id)?;
        let equality = CheckedEquality::from_admitted_decision(&decision);
        let operator = if identity.and_then(OperationSuffix::of) == Some(OperationSuffix::Ne) {
            EqualityOperator::NotEqual
        } else {
            EqualityOperator::Equal
        };
        let [left, right] = children.as_slice() else {
            return Err(fault("reader-equality-has-two-operands").into());
        };
        Ok((
            NodeKind::Equality(operator, Box::new(equality), *left, *right),
            children,
        ))
    }

    /// A `case` over a union scrutinee. Its binders take the frame slots
    /// after every slot of the enclosing context; the root context's frame
    /// grows to hold them, as a function's does.
    fn case(
        &mut self,
        id: &CheckedNodeId,
        arguments: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        let scrutinee_term = arguments
            .first()
            .ok_or_else(|| fault("reader-case-has-scrutinee"))?;
        let scrutinee = self.project_argument(id, scrutinee_term, context, locals)?;
        let scrutinee_id = reference_target(scrutinee_term)
            .ok_or_else(|| fault("reader-union-case-scrutinee-has-identity"))?;
        let ValueType::Composite(declaration) = &self.value_type(scrutinee_id)? else {
            return Err(fault("reader-union-case-scrutinee-has-union-type").into());
        };
        let declaration = declaration.clone();
        let union_node = self
            .index
            .ids
            .get(&declaration)
            .and_then(|id| self.index.node(id))
            .filter(|candidate| has_form(candidate, WireTag::CompositeType, WireForm::Union))
            .ok_or_else(|| fault("reader-union-case-declaration-is-admitted"))?;
        let CheckedTerm::Aggregate {
            members: union_members,
        } = &union_node.body
        else {
            return Err(fault("reader-union-declaration-has-checked-members").into());
        };
        let mut arms = Vec::with_capacity(arguments.len().saturating_sub(1));
        let mut children = vec![scrutinee];
        let first_free = match context {
            Context::Root => self.root_slots,
            Context::Function(function) => self
                .functions
                .get(function)
                .map_or(0, |function| function.slots),
        };
        let mut next_slot = locals
            .values()
            .copied()
            .max()
            .map_or(0, |slot| slot.saturating_add(1))
            .max(first_free);
        for argument in arguments.iter().skip(1) {
            let CheckedTerm::Binding { name, value } = argument else {
                return Err(fault("reader-union-case-arm-is-binding").into());
            };
            let quire_contract_model::CheckedBindingNameKind::Other(member_name) = name.kind()
            else {
                return Err(fault("reader-union-case-arm-has-member-name").into());
            };
            let member = member_position(union_members, member_name)
                .ok_or_else(|| fault("reader-union-case-arm-is-declared"))?;
            let CheckedTerm::Aggregate { members } = value.as_ref() else {
                return Err(fault("reader-union-case-arm-has-body").into());
            };
            let [CheckedTerm::Aggregate { members: binders }, body] = members.as_slice() else {
                return Err(fault("reader-union-case-arm-has-binders-and-body").into());
            };
            let mut arm_locals = locals.clone();
            let mut arm_slots = Vec::with_capacity(binders.len());
            for binder in binders {
                let target = reference_target(binder)
                    .ok_or_else(|| fault("reader-union-case-binder-has-identity"))?;
                arm_slots.push(next_slot);
                arm_locals.insert(target.clone(), next_slot);
                next_slot = next_slot
                    .checked_add(1)
                    .ok_or_else(|| fault("reader-union-case-slot-index-fits"))?;
            }
            // Record the grown frame before the arm body, so a nested case
            // in it numbers its binders after these.
            self.grow_frame(context, next_slot);
            let body = self.project_argument(id, body, context, &arm_locals)?;
            children.push(body);
            arms.push(CaseArm {
                member,
                binders: arm_slots,
                body,
            });
        }
        self.grow_frame(context, next_slot);
        Ok((
            NodeKind::Case {
                scrutinee,
                declaration,
                arms,
            },
            children,
        ))
    }

    fn grow_frame(&mut self, context: Context, slots: usize) {
        match context {
            Context::Root => self.root_slots = self.root_slots.max(slots),
            Context::Function(function) => {
                if let Some(function) = self.functions.get_mut(function) {
                    function.slots = function.slots.max(slots);
                }
            }
        }
    }

    fn next_slot(&self, context: Context, locals: &BTreeMap<CheckedNodeId, usize>) -> usize {
        let first_free = match context {
            Context::Root => self.root_slots,
            Context::Function(function) => self
                .functions
                .get(function)
                .map_or(0, |function| function.slots),
        };
        let after_active_locals = locals
            .values()
            .copied()
            .max()
            .map_or(0, |slot| slot.saturating_add(1));
        first_free.max(after_active_locals)
    }

    /// The binder a `let` or a collection query introduces, recovered from
    /// its scope: the one `value`/`parameter` node of `name` the scope reaches
    /// outside every nested scope that rebinds `name` (FR-390 rule 3). A scope
    /// that never reads its binder reaches none, and the binder then takes a
    /// frame slot no node reads. Levels are slot-relative and play no part.
    fn binder_node(
        &self,
        name: &str,
        scope: &CheckedTerm,
    ) -> Result<Option<CheckedNodeId>, BuildError> {
        let mut found = BTreeSet::new();
        let lookup = |id: &CheckedNodeId| self.index.node(id);
        crate::introduced_binders::reach_term(&lookup, scope, &mut |node| {
            if parameter_name(&node.body) == Some(name) {
                found.insert(node.node_id.clone());
            }
        });
        let mut found = found.into_iter();
        match (found.next(), found.next()) {
            (binder, None) => Ok(binder),
            _ => Err(fault("reader-expression-binder-is-admitted").into()),
        }
    }

    /// `record.project`: a field of a record value, or an attribute of a
    /// model object read through `model.deref` (FR-107's charged read).
    fn projection(
        &mut self,
        id: &CheckedNodeId,
        operation: &quire_contract_model::CheckedApplicationOperation,
        arguments: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        let operand_term = arguments
            .first()
            .ok_or_else(|| fault("reader-record-projection-has-operand"))?;
        let operand_id = reference_target(operand_term)
            .ok_or_else(|| fault("reader-record-projection-has-operand"))?;
        let admitted_operation = operation
            .read()
            .map_err(|_| fault("reader-record-projection-has-admitted-field"))?;
        let Some(quire_contract_model::CheckedOperationMember::Field {
            declaration: field_declaration,
            name: field_name,
            read_form,
        }) = admitted_operation.member
        else {
            return Err(fault("reader-record-projection-has-admitted-field").into());
        };
        let selected = self
            .index
            .identities
            .declaration_rows()
            .find(|row| row.structural_node == *field_declaration)
            .map(|row| &row.exact_identity);
        if let Some(quire_contract_model::SelectedExactIdentity::Object(owner)) = selected {
            // A model attribute reads the reference under its `model.deref`.
            let operand = self.node(operand_id)?;
            let reference = match &operand.body {
                CheckedTerm::Application {
                    operation,
                    arguments,
                    ..
                } if operation.identity() == Some("quire.op.model.deref") => arguments
                    .first()
                    .ok_or_else(|| fault("reader-attribute-deref-has-operand"))?
                    .clone(),
                _ => operand_term.clone(),
            };
            let fields = self
                .admitted()
                .model_object_fields(field_declaration)
                .map_err(|_| fault("reader-field-declaration-is-admitted"))?;
            let optional = matches!(
                fields
                    .field(field_name)
                    .ok_or_else(|| fault("reader-field-member-is-admitted"))?
                    .member_type(),
                Some(quire_contract_model::CheckedMemberType::Option(_))
            );
            let child = self.project_argument(id, &reference, context, locals)?;
            return Ok((
                NodeKind::Attribute {
                    reference: child,
                    field: FieldRef::new(owner.clone(), field_name.to_string()),
                    optional,
                    derefed: matches!(
                        read_form,
                        Some(quire_contract_model::CheckedFieldReadForm::ExplicitDeref)
                    ),
                },
                vec![child],
            ));
        }
        let ValueType::Composite(record_key) = &self.value_type(operand_id)? else {
            return Err(fault("reader-record-projection-operand-has-composite-type").into());
        };
        let record_key = record_key.clone();
        let selected_record = match selected {
            Some(quire_contract_model::SelectedExactIdentity::Record(key)) => Some(key),
            _ => None,
        };
        if selected_record != Some(&record_key) {
            // With no selected identity row for the field's declaring node,
            // its checked record declaration key must be the exact operand
            // identity; equal field shape is never evidence.
            let declaration_key = self
                .index
                .graph
                .key(field_declaration)
                .map_err(|_| fault("reader-record-field-node-has-wire-key"))?;
            let declaration = self
                .index
                .node(field_declaration)
                .ok_or_else(|| fault("reader-record-field-node-is-admitted"))?;
            let record_form = (has_form(declaration, WireTag::Model, WireForm::RecordValueType))
                || (has_form(declaration, WireTag::CompositeType, WireForm::Record));
            if !record_form || declaration_key != record_key {
                return Err(fault("reader-record-field-exact-type-matches-operand").into());
            }
        }
        // Selected model records retain field presence independently from
        // member-type projection, so nested option and collection types do not
        // require projecting the record's complete Exact type.
        let selected_presence = selected_record
            .map(|record| {
                let record_id = self
                    .index
                    .ids
                    .get(record)
                    .ok_or_else(|| fault("reader-selected-record-has-admitted-node"))?;
                let fields = self
                    .admitted()
                    .model_record_fields(record_id)
                    .map_err(|_| fault("reader-selected-record-fields-are-admitted"))?;
                let field = fields
                    .field(field_name.as_ref())
                    .ok_or_else(|| fault("reader-selected-record-field-is-admitted"))?;
                Ok::<_, InternalFault>(if field.optional() {
                    quire_exact::Presence::Optional
                } else {
                    quire_exact::Presence::Required
                })
            })
            .transpose()?;
        let presence = match selected_presence {
            Some(presence) => presence,
            None => {
                let from_view = [operand_id, field_declaration]
                    .into_iter()
                    .find_map(|node| {
                        let projection = self
                            .admitted()
                            .project_exact_type(node, &self.index.identities, self.index.work_limit)
                            .ok()?;
                        let view = projection.record_view()?;
                        if view.record_key() != &record_key {
                            return None;
                        }
                        view.fields()
                            .iter()
                            .find(|field| field.name() == field_name.as_ref())
                            .map(|field| field.presence())
                    });
                from_view
                    .or(self.declared_member_presence(&record_key, field_name.as_ref())?)
                    .ok_or_else(|| BuildError::Unsupported {
                        reason: UnsupportedReason::MissingTypeRepresentation,
                        node: id.clone(),
                    })?
            }
        };
        let optional = presence == quire_exact::Presence::Optional;
        let child = self.project_argument(id, operand_term, context, locals)?;
        Ok((
            NodeKind::Field {
                operand: child,
                field: RecordField::new(record_key, field_name.to_string()),
                optional,
            },
            vec![child],
        ))
    }

    /// The presence of record member `field` as the admitted record
    /// declaration `record` declares it.
    fn declared_member_presence(
        &self,
        record: &NodeKey,
        field: &str,
    ) -> Result<Option<quire_exact::Presence>, InternalFault> {
        let declaration = self
            .index
            .ids
            .get(record)
            .and_then(|id| self.index.node(id))
            .ok_or_else(|| fault("reader-record-declaration-is-admitted"))?;
        if !has_form(declaration, WireTag::CompositeType, WireForm::Record) {
            return Ok(None);
        }
        let CheckedTerm::Aggregate { members } = &declaration.body else {
            return Err(fault("reader-record-declaration-has-members"));
        };
        let member_type = members
            .iter()
            .find_map(|member| match member {
                CheckedTerm::Binding { name, value } => {
                    use quire_contract_model::CheckedBindingNameKind as Kind;
                    let named = match name.kind() {
                        Kind::Parameters => "parameters",
                        Kind::Body => "body",
                        Kind::Optional => "optional",
                        Kind::Clause => "clause",
                        Kind::Other(name) => name,
                    };
                    (named == field).then(|| reference_target(value)).flatten()
                }
                _ => None,
            })
            .and_then(|target| self.index.node(target))
            .ok_or_else(|| fault("reader-record-field-member-is-admitted"))?;
        Ok(Some(
            if has_form(member_type, WireTag::CompositeType, WireForm::Option) {
                quire_exact::Presence::Optional
            } else {
                quire_exact::Presence::Required
            },
        ))
    }

    fn project_argument(
        &mut self,
        parent: &CheckedNodeId,
        argument: &CheckedTerm,
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<usize, BuildError> {
        match argument {
            CheckedTerm::Reference { target } => self.project_node(target, context, locals),
            CheckedTerm::Literal { .. } => self.project_inline_literal(parent, argument),
            _ => Err(unsupported_operation(parent, "inline operand form")),
        }
    }

    /// The `quantity.convert` application `argument` names, with its operand.
    fn quantity_conversion<'a>(
        &'a self,
        argument: &'a CheckedTerm,
    ) -> Option<(&'a CheckedNodeId, &'a CheckedTerm)> {
        let id = reference_target(argument)?;
        let conversion = self.index.node(id)?;
        let CheckedTerm::Application {
            operation,
            arguments,
            ..
        } = &conversion.body
        else {
            return None;
        };
        (operation.identity() == Some("quire.op.quantity.convert"))
            .then(|| arguments.first().map(|operand| (id, operand)))
            .flatten()
    }

    fn value_type(&self, id: &CheckedNodeId) -> Result<ValueType, BuildError> {
        node_value_type(&self.index.types(), id).map_err(|error| match error {
            TypeError::Unsupported => BuildError::Unsupported {
                reason: UnsupportedReason::MissingTypeRepresentation,
                node: id.clone(),
            },
            TypeError::Fault(fault) => BuildError::Fault(fault),
        })
    }

    fn project_inline_literal(
        &mut self,
        parent: &CheckedNodeId,
        term: &CheckedTerm,
    ) -> Result<usize, BuildError> {
        let unsupported = || BuildError::Unsupported {
            reason: UnsupportedReason::MissingLiteralProjection,
            node: parent.clone(),
        };
        let literal = term
            .literal_read()
            .ok_or_else(|| fault("reader-inline-literal-is-typed"))?;
        let value =
            literal_value(literal.value, literal.type_id, self.index).ok_or_else(unsupported)?;
        let value_type = match self.admitted().project_exact_type(
            literal.type_id,
            &self.index.identities,
            self.index.work_limit,
        ) {
            Ok(projection) => projection.value_type().clone(),
            Err(error)
                if split_tag_form(&error.form) == Some((WireTag::ScalarType, WireForm::Text)) =>
            {
                ValueType::String
            }
            Err(_) => return Err(unsupported()),
        };
        Ok(self.push(ReaderProjectedNode::leaf(
            NodeKind::Literal(CheckedLiteral(value)),
            value_type,
        )))
    }
}

pub(crate) enum TypeError {
    Unsupported,
    Fault(InternalFault),
}

impl From<InternalFault> for TypeError {
    fn from(fault: InternalFault) -> Self {
        Self::Fault(fault)
    }
}

/// What reading the exact type of an admitted node needs: the package, the
/// selected identity witness, the nodes by id and the structural keys.
pub(crate) struct TypeReader<'a> {
    pub(crate) admitted: &'a CheckedPackageV2,
    pub(crate) identities: &'a quire_contract_model::SelectedExactIdentities,
    pub(crate) nodes: &'a BTreeMap<&'a CheckedNodeId, &'a CheckedSemanticNodeV2>,
    pub(crate) graph: &'a WireGraph,
    pub(crate) work_limit: u64,
}

impl TypeReader<'_> {
    fn node(&self, id: &CheckedNodeId) -> Option<&CheckedSemanticNodeV2> {
        self.nodes.get(id).copied()
    }
}

/// The comparison family and operator a `quire.op.<family>.<operator>` order
/// identity names, `None` for any other identity.
#[ix_trace_rs::string_edge]
fn order_of(identity: &str) -> Option<(OrderedKind, OrderingOperator)> {
    let (family, operator) = identity.strip_prefix("quire.op.")?.split_once('.')?;
    let kind = match family {
        "integer" => OrderedKind::Integers,
        "rational" => OrderedKind::Rationals,
        "decimal" => OrderedKind::Decimals,
        "enum" => OrderedKind::Enums,
        "text" => OrderedKind::Texts,
        "quantity" => OrderedKind::Quantities,
        _ => return None,
    };
    let operator = match operator {
        "lt" => OrderingOperator::Less,
        "le" => OrderingOperator::LessOrEqual,
        "gt" => OrderingOperator::Greater,
        "ge" => OrderingOperator::GreaterOrEqual,
        _ => return None,
    };
    Some((kind, operator))
}

#[ix_trace_rs::string_edge]
fn arithmetic_operator(identity: &str) -> Option<ArithmeticOperator> {
    match identity.rsplit('.').next()? {
        "add" => Some(ArithmeticOperator::Add),
        "sub" => Some(ArithmeticOperator::Subtract),
        "mul" => Some(ArithmeticOperator::Multiply),
        "div" => Some(ArithmeticOperator::Divide),
        _ => None,
    }
}

/// The selected exact type of admitted node `id`, joining declared and
/// structural identities where IR's projection leaves the join to QSL.
pub(crate) fn node_value_type(
    index: &TypeReader<'_>,
    id: &CheckedNodeId,
) -> Result<ValueType, TypeError> {
    use quire_contract_model::ExactTypeProjectionErrorCode as Code;
    use quire_contract_model::SelectedExactIdentity as Identity;
    let error = match index
        .admitted
        .project_exact_type(id, index.identities, index.work_limit)
    {
        Ok(projection) => return Ok(projection.value_type().clone()),
        Err(error) => error,
    };
    if matches!(
        error.code,
        Code::UnsupportedExactType | Code::UnavailableIdentityWitness
    ) && is_collection_bounds(&error.form)
    {
        return collection_node_type(index, id);
    }
    let unresolved = matches!(
        error.code,
        Code::UnsupportedExactType | Code::UnresolvedSelectedDeclaration
    );
    let at = error.at.as_deref();
    let selected = |declaration: &CheckedNodeId| {
        index
            .identities
            .declaration_rows()
            .find(|row| row.structural_node == *declaration)
            .map(|row| &row.exact_identity)
    };
    // A checked declaration node of `form` is itself the exact structural
    // identity when no selected identity row exists; equal shape never is.
    let structural = |declaration: &CheckedNodeId, tag: WireTag, form: WireForm| {
        index
            .node(declaration)
            .filter(|node| has_form(node, tag, form))
            .and_then(|_| index.graph.key(declaration).ok())
    };
    match split_tag_form(&error.form) {
        Some((WireTag::ScalarType, WireForm::Text)) if unresolved => {
            index
                .node(at.ok_or_else(|| fault("reader-string-type-node-is-admitted"))?)
                .filter(|node| has_form(node, WireTag::ScalarType, WireForm::Text))
                .ok_or_else(|| fault("reader-string-type-has-checked-form"))?;
            Ok(ValueType::String)
        }
        Some((WireTag::CompositeType, WireForm::Record)) if unresolved =>
        {
            let declaration = at.ok_or_else(|| fault("reader-record-type-error-has-node"))?;
            let key = match selected(declaration) {
                Some(Identity::Record(key)) => Some(key.clone()),
                _ => structural(declaration, WireTag::CompositeType, WireForm::Record),
            }
            .ok_or_else(|| fault("reader-record-type-has-exact-identity"))?;
            Ok(ValueType::Composite(key))
        }
        Some((WireTag::CompositeType, WireForm::Union))
            if error.code == Code::UnavailableIdentityWitness =>
        {
            let declaration = at.ok_or_else(|| fault("reader-union-type-error-has-node"))?;
            let key = match selected(declaration) {
                Some(Identity::Union(key)) => Some(key.clone()),
                _ => structural(declaration, WireTag::CompositeType, WireForm::Union),
            }
            .ok_or_else(|| fault("reader-union-type-has-exact-identity"))?;
            Ok(ValueType::Composite(key))
        }
        Some((WireTag::ScalarType, WireForm::Unit)) if unresolved => {
            let declaration = at.ok_or_else(|| fault("reader-unit-has-exact-identity"))?;
            let unit = match selected(declaration) {
                Some(Identity::DeclaredUnit(unit)) => Some(unit.clone()),
                _ => structural(declaration, WireTag::ScalarType, WireForm::Unit)
                    .map(quire_exact::UnitId::declared),
            }
            .ok_or_else(|| fault("reader-unit-has-exact-identity"))?;
            Ok(ValueType::Quantity(unit))
        }
        Some((WireTag::ScalarType, WireForm::CompoundUnit)) if unresolved => {
            let unit_node = at.ok_or_else(|| fault("reader-compound-unit-type-has-node"))?;
            let unit = crate::checked_v2::compound_unit(index.admitted, unit_node)
                .ok_or_else(|| fault("reader-compound-unit-has-exact-identity"))?;
            Ok(ValueType::Quantity(unit))
        }
        _ => {
            // A package-local record or unit is typed by its checked
            // semantic-type declaration, the node's exact structural identity.
            let semantic_type = &index
                .node(id)
                .ok_or_else(|| fault("reader-expression-child-is-admitted"))?
                .semantic_type;
            if error.code == Code::UnresolvedSelectedDeclaration {
                let record = match selected(semantic_type) {
                    Some(Identity::Record(key)) => Some(key.clone()),
                    _ => structural(semantic_type, WireTag::Model, WireForm::RecordValueType),
                };
                if let Some(key) = record {
                    return Ok(ValueType::Composite(key));
                }
            }
            if unresolved
                && split_tag_form(&error.form) == Some((WireTag::ScalarType, WireForm::Dimension))
            {
                let unit = match selected(semantic_type) {
                    Some(Identity::DeclaredUnit(unit)) => Some(unit.clone()),
                    _ => structural(semantic_type, WireTag::ScalarType, WireForm::Unit)
                        .map(quire_exact::UnitId::declared),
                };
                if let Some(unit) = unit {
                    return Ok(ValueType::Quantity(unit));
                }
            }
            Err(TypeError::Unsupported)
        }
    }
}

/// `#[string_edge]`: `form` is the admitted `tag/form` wire word an IR type
/// error names.
#[ix_trace_rs::string_edge]
fn is_collection_bounds(form: &str) -> bool {
    form == "bounded_domain/collection_bounds"
}

/// The integer a `min` or `max` member of a bounds node holds.
#[ix_trace_rs::string_edge]
fn bound_member(body: &CheckedTerm, wanted: &str) -> Option<u64> {
    let CheckedTerm::Aggregate { members } = body else {
        return None;
    };
    members.iter().find_map(|member| {
        let CheckedTerm::Binding { name, value } = member else {
            return None;
        };
        (name.as_ref() == wanted).then(|| match value.literal_read()?.value {
            quire_contract_model::CheckedLiteralValue::Integer(bound) => {
                bound.to_string().parse::<u64>().ok()
            }
            _ => None,
        })?
    })
}

/// The exact type of a node whose semantic type is a cardinality-bounded
/// collection. IR's projection reads a collection only as a record member, so
/// the join is QSL's: the bounds node gives the cardinality, the collection
/// type node the kind and its element, and the element is typed by a node of
/// the package that has it as semantic type (IR's own projection of that node).
#[ix_trace_rs::string_edge]
fn collection_node_type(
    index: &TypeReader<'_>,
    id: &CheckedNodeId,
) -> Result<ValueType, TypeError> {
    let bounds = index
        .node(id)
        .and_then(|node| index.node(&node.semantic_type))
        .ok_or_else(|| fault("reader-collection-bounds-node-is-admitted"))?;
    let collection = index
        .node(&bounds.semantic_type)
        .filter(|node| node.node_tag.as_ref() == "composite_type")
        .ok_or_else(|| fault("reader-collection-type-node-is-admitted"))?;
    let kind = match collection.semantic_form.as_ref() {
        "sequence" => quire_exact::CollectionKind::Sequence,
        "set" => quire_exact::CollectionKind::Set,
        "bag" => quire_exact::CollectionKind::Bag,
        "ordered_set" => quire_exact::CollectionKind::OrderedSet,
        _ => return Err(TypeError::Unsupported),
    };
    let (Some(minimum), Some(maximum)) = (
        bound_member(&bounds.body, "min"),
        bound_member(&bounds.body, "max"),
    ) else {
        return Err(fault("reader-collection-bounds-are-integers").into());
    };
    let bound = quire_exact::CardinalityBound::new(minimum, maximum)
        .map_err(|_| fault("reader-collection-bounds-are-ordered"))?;
    let element = match &collection.body {
        CheckedTerm::Aggregate { members } => match members.as_slice() {
            [CheckedTerm::Reference { target }] => target,
            _ => return Err(fault("reader-collection-type-has-one-element").into()),
        },
        _ => return Err(fault("reader-collection-type-has-one-element").into()),
    };
    let witness = index
        .nodes
        .values()
        .find(|node| node.semantic_type == *element && node.node_id != *id)
        .ok_or(TypeError::Unsupported)?;
    let element = node_value_type(index, &witness.node_id)?;
    Ok(ValueType::collection(quire_exact::CollectionType::new(
        kind,
        element,
        Some(bound),
    )))
}

/// `#[string_edge]`: `name` is an admitted binding key in the parameter wire
/// form.
#[ix_trace_rs::string_edge]
pub(crate) fn parameter_name(term: &CheckedTerm) -> Option<&str> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    members.iter().find_map(|member| {
        let CheckedTerm::Binding { name, value } = member else {
            return None;
        };
        (name.as_ref() == "name").then(|| match value.literal_read()?.value {
            quire_contract_model::CheckedLiteralValue::Text(name) => Some(name),
            _ => None,
        })?
    })
}

fn reference_target(term: &CheckedTerm) -> Option<&CheckedNodeId> {
    match term {
        CheckedTerm::Reference { target } => Some(target),
        _ => None,
    }
}

fn member_position(members: &[CheckedTerm], wanted: &str) -> Option<usize> {
    members.iter().position(|candidate| match candidate {
        CheckedTerm::Binding { name, .. } => matches!(
            name.kind(),
            quire_contract_model::CheckedBindingNameKind::Other(name) if name == wanted
        ),
        _ => false,
    })
}

/// A wire location as the evaluator's `Location`; `None` for an ordinal no
/// `usize` holds.
fn location(admitted: &CheckedProtocolLocation) -> Option<Location> {
    let origin = match &admitted.origin {
        quire_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name } => {
            Origin::TypeDeclaration {
                name: name.to_string(),
            }
        }
        quire_contract_model::CheckedProtocolLocationOrigin::ProtocolAttempt {
            protocol,
            attempt,
        } => Origin::ProtocolAttempt {
            protocol: usize::try_from(*protocol).ok()?,
            attempt: usize::try_from(*attempt).ok()?,
        },
    };
    let path = admitted
        .path
        .iter()
        .map(|ordinal| match ordinal {
            quire_contract_model::CheckedKeyOrdinal::Safe(value) => usize::try_from(*value).ok(),
            quire_contract_model::CheckedKeyOrdinal::BeyondSafe(value) => {
                value.parse::<usize>().ok()
            }
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Location::at(origin, &path))
}

fn function_parts(term: &CheckedTerm) -> Option<(&[CheckedTerm], &CheckedTerm)> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    let mut parameters = None;
    let mut body = None;
    for member in members {
        let CheckedTerm::Binding { name, value } = member else {
            continue;
        };
        match name.kind() {
            quire_contract_model::CheckedBindingNameKind::Parameters => {
                let CheckedTerm::Aggregate { members } = value.as_ref() else {
                    return None;
                };
                parameters = Some(members.as_slice());
            }
            quire_contract_model::CheckedBindingNameKind::Body => body = Some(value.as_ref()),
            _ => {}
        }
    }
    Some((parameters?, body?))
}

fn literal_value(
    value: quire_contract_model::CheckedLiteralValue<'_>,
    type_id: &CheckedNodeId,
    index: &WireIndex<'_>,
) -> Option<Value> {
    use quire_contract_model::CheckedLiteralValue as Literal;
    let (admitted, identities, work_limit) = (index.admitted, &index.identities, index.work_limit);
    Some(match value {
        Literal::Boolean(value) => Value::Boolean(value),
        Literal::Integer(value) => Value::Integer(value.to_string().parse().ok()?),
        Literal::Rational {
            numerator,
            denominator,
        } => {
            let magnitude = quire_exact::Rational::new(
                numerator.to_string().parse().ok()?,
                denominator.to_string().parse().ok()?,
            )
            .ok()?;
            let literal_type = project_literal_type(type_id, index)?;
            match &literal_type {
                ValueType::Quantity(unit) => {
                    Value::Quantity(quire_exact::Quantity::new(magnitude, unit.clone()))
                }
                ValueType::Rational(_) => Value::Rational(magnitude),
                _ => return None,
            }
        }
        Literal::String(value) => Value::String(value.into()),
        Literal::Text(value) => {
            let checked_type = index.node(type_id)?;
            if has_form(checked_type, WireTag::ScalarType, WireForm::Text) {
                return Some(Value::String(value.into()));
            }
            let type_projection = admitted.project_exact_type(type_id, identities, work_limit);
            let projection = type_projection.ok()?;
            let ValueType::Text(text_type) = projection.value_type() else {
                return None;
            };
            let payload = TextPayload::from_utf8(value.as_bytes()).ok()?;
            let mut meter = Meter::new(ScalarLimits {
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
            Value::Text(admit_text(&payload, text_type, &mut meter).completed()?)
        }
        Literal::Uuid(value) => Value::Uuid(Uuid::from_canonical_text(value).ok()?),
        Literal::Timestamp(value) => {
            Value::Timestamp(Timestamp::from_canonical_text(&value.to_string()).ok()?)
        }
        _ => return None,
    })
}

fn project_literal_type(type_id: &CheckedNodeId, index: &WireIndex<'_>) -> Option<ValueType> {
    let (admitted, identities, work_limit) = (index.admitted, &index.identities, index.work_limit);
    match admitted.project_exact_type(type_id, identities, work_limit) {
        Ok(projection) => Some(projection.value_type().clone()),
        Err(error) => {
            if !matches!(
                error.code,
                quire_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType
                    | quire_contract_model::ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
            ) || !matches!(
                split_tag_form(&error.form),
                Some((WireTag::ScalarType, WireForm::Unit | WireForm::Dimension))
            )
            {
                return None;
            }
            let declaration = type_id;
            let unit_node = index.node(declaration)?;
            if !has_form(unit_node, WireTag::ScalarType, WireForm::Unit) {
                return None;
            }
            let unit = identities
                .declaration_rows()
                .find(|row| row.structural_node == *declaration)
                .and_then(|row| match &row.exact_identity {
                    quire_contract_model::SelectedExactIdentity::DeclaredUnit(unit) => {
                        Some(unit.clone())
                    }
                    _ => None,
                })
                .or_else(|| {
                    index
                        .key(declaration)
                        .ok()
                        .map(quire_exact::UnitId::declared)
                })?;
            Some(ValueType::Quantity(unit))
        }
    }
}

struct ReaderProjectedNode {
    kind: NodeKind<usize, CheckedLiteral>,
    value_type: ValueType,
    /// Set from the paired row for a slot-walk application; `None` for any
    /// other node, the wire carrying no location of its own for it.
    location: Option<Location>,
    children: Vec<usize>,
    /// The application this slot-walk position is a term of (FR-383).
    term: Option<CheckedNodeId>,
    /// A quantity conversion the walk visits directly above this node,
    /// which the enclosing equality's admitted decision applies itself.
    absorbed: Option<CheckedNodeId>,
}

impl ReaderProjectedNode {
    fn leaf(kind: NodeKind<usize, CheckedLiteral>, value_type: ValueType) -> Self {
        Self {
            kind,
            value_type,
            location: None,
            children: Vec::new(),
            term: None,
            absorbed: None,
        }
    }

    /// The applications this position is visited for, in walk order: an
    /// absorbed conversion, then the node's own term.
    fn terms(&self) -> impl Iterator<Item = &CheckedNodeId> {
        self.absorbed.iter().chain(&self.term)
    }
}

struct ReaderFunction {
    name: String,
    body: usize,
    slots: usize,
}

fn fault(invariant: &'static str) -> InternalFault {
    InternalFault::new("FR-390", invariant)
}

impl UnsupportedView {
    /// A span-named slot whose root holds an application and which no origin
    /// of the package names (FR-533 "Slot naming on the bytes route", rule
    /// 5): `rows` is the number of candidate origins and `visits` the
    /// application's visits in the slot's walk.
    pub(crate) fn missing_origin(
        package: &str,
        prefix: &str,
        application: NodeKey,
        visits: usize,
    ) -> Self {
        Self {
            package: package.to_owned(),
            locus: ViewLocus::node(application.clone()),
            reason: UnsupportedReason::MissingLocationProjection(Box::new(MissingLocation {
                origin: prefix.trim_end().to_owned(),
                application,
                index: 0,
                rows: 0,
                visits,
                defect: ("invalid_package", "invalid-value"),
                pointer: ROWS_POINTER.to_owned(),
            })),
        }
    }
}

#[cfg(test)]
mod order_tests {
    use super::*;
    use crate::bare::IdentityWitnessRefusal;
    use ix_trace_rs::trace;
    use quire_exact::PathSegment;

    fn key(name: &str) -> NodeKey {
        NodeKey::new("pkg", vec![PathSegment::Text(name.to_owned())], 0).expect("a key")
    }

    fn node(package: &str, node: &str, reason: UnsupportedReason) -> UnsupportedView {
        UnsupportedView {
            package: package.to_owned(),
            locus: ViewLocus::node(key(node)),
            reason,
        }
    }

    fn witness(reason: impl Into<UnsupportedReason>) -> UnsupportedView {
        UnsupportedView::witness("pkg", reason.into())
    }

    fn first(views: Vec<UnsupportedView>) -> String {
        let mut views = views.into_iter();
        let mut chosen = views.next();
        for view in views {
            chosen = first_refusal(chosen, Some(view));
        }
        chosen.map(|view| view.reason.name()).unwrap_or_default()
    }

    /// FR-390-AC-67: view creation reports the first unsupported projection
    /// by package identity, then locus (`Package` before `Node`), then node
    /// key, then FR-390's reason order, whatever order they were found in.
    #[trace("FR-390-AC-67", "TC-1070")]
    #[test]
    fn the_first_unsupported_projection_is_by_package_locus_node_and_reason() {
        let type_at_node = node("pkg", "a", UnsupportedReason::MissingTypeRepresentation);
        let literal_at_node = node("pkg", "a", UnsupportedReason::MissingLiteralProjection);
        let foreign = || witness(IdentityWitnessRefusal::Foreign);
        let missing_identity = || witness(IdentityWitnessRefusal::Missing);
        let missing_equality = || witness(EqualityWitnessRefusal::Missing);
        // A Node-locus type refusal and a foreign identity witness: Package first.
        for views in [
            vec![type_at_node.clone(), foreign()],
            vec![foreign(), type_at_node.clone()],
        ] {
            assert_eq!(first(views), "foreign-identity-witness");
        }
        // A Node-locus literal refusal and a missing equality witness.
        for views in [
            vec![literal_at_node.clone(), missing_equality()],
            vec![missing_equality(), literal_at_node.clone()],
        ] {
            assert_eq!(first(views), "missing-equality-witness");
        }
        // Both at the Package locus: the reason order breaks the tie.
        for views in [
            vec![foreign(), missing_equality()],
            vec![missing_equality(), foreign()],
        ] {
            assert_eq!(first(views), "foreign-identity-witness");
        }
        for views in [
            vec![missing_identity(), missing_equality()],
            vec![missing_equality(), missing_identity()],
        ] {
            assert_eq!(first(views), "missing-identity-witness");
        }
        // Ascending package identity precedes locus; ascending node key
        // precedes reason; reason order decides at one node.
        let earlier_package = node("aaa", "z", UnsupportedReason::MissingLiteralProjection);
        assert_eq!(
            first(vec![
                witness(IdentityWitnessRefusal::Missing),
                earlier_package
            ]),
            "missing-literal-projection"
        );
        assert_eq!(
            first(vec![
                node("pkg", "b", UnsupportedReason::MissingTypeRepresentation),
                literal_at_node.clone(),
            ]),
            "missing-literal-projection",
            "the lower node key wins over an earlier reason"
        );
        assert_eq!(
            first(vec![literal_at_node, type_at_node]),
            "missing-type-representation"
        );
    }
}
