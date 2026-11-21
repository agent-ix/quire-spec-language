// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reader-owned protocol expression nodes projected from admitted I2 terms
//! (FR-390 "Composed roots from the reader").
//!
//! The whole package is projected once, when its reader-backed view is
//! created: every slot of every protocol node, each a tree of positions in
//! FR-383's slot-walk order, its applications paired with the admitted
//! `protocol_locations` rows of the slot's own origin. A node reads its
//! location from that row or, for a leaf, `let`, conditional or case, from its
//! own admitted node, which the wire does not carry yet (FSpec
//! FR-322-AC-310), so it reads none. Nothing here takes a location from the
//! source package.

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::node::{
    Arithmetic, CaseArm, CheckedLiteral, Connective, ConnectiveOrigin, IntegerDivisionOperator,
    NodeKind, OrderedKind, RecordField, RecordSlot, Visit,
};
use filament_contract_model::{
    CheckedBodyLocation, CheckedBodyLocationOrigin, CheckedNodeId, CheckedPackageV2,
    CheckedProtocolLocation, CheckedProtocolSemanticLocation, CheckedSemanticNodeV2, CheckedTerm,
    OperationModeKind,
};
use filament_exact::{
    admit_text, ArithmeticOperator, Meter, NodeKey, OrderingOperator, ScalarLimits, TextPayload,
    Timestamp, Uuid, Value, ValueType,
};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::declaration::{
    CheckedEquality, EqualityOperand, EqualityOperator, FieldRef, TypeEnvironment,
};
use filament_semantic_value::location::{
    ExpressionLocation as Location, ExpressionOwner as Origin,
};
use fsl_semantics::check::wire_id::WireGraph;

use crate::equality_file::{EqualityWitnessRefusal, ReadEqualityRows};
use crate::introduced_binders::is_sum_identity;
use crate::wire_form::{has_form, has_tag, split_tag_form, OperationSuffix, WireForm, WireTag};

/// One projected protocol slot: owned evaluator nodes, the admitted function
/// table its calls reach, and the frame its locals and case binders use.
/// Wire terms and producer rows stay inside fsl-package.
pub struct ReaderProtocolProjection {
    nodes: Vec<ReaderProjectedNode>,
    functions: Vec<ReaderFunction>,
    root: usize,
    selected_function: Option<usize>,
    selected_function_node: Option<NodeKey>,
    frame_slots: usize,
    locals: BTreeMap<NodeKey, usize>,
}

impl ReaderProtocolProjection {
    /// Resolve runtime representations against the closed declarations. A
    /// native-authenticated reference needs no object population environment.
    pub(crate) fn has_unrepresented_types(&self, types: &TypeEnvironment) -> bool {
        self.nodes
            .iter()
            .any(|node| types.runtime_type(&node.value_type).is_none())
    }

    /// Index of the selected root node.
    pub fn root(&self) -> usize {
        self.root
    }

    /// The exact node kind at an arena index.
    pub fn kind(&self, index: usize) -> Option<&NodeKind<usize, CheckedLiteral>> {
        self.nodes.get(index).map(|node| &node.kind)
    }

    /// The declaration key of the alias the source wrote as the type argument
    /// of the application at `index`, when it wrote one.
    pub fn type_argument(&self, index: usize) -> Option<&str> {
        self.nodes.get(index)?.type_argument.as_deref()
    }

    /// The selected exact type of an arena node.
    pub fn value_type(&self, index: usize) -> Option<&ValueType> {
        self.nodes.get(index).map(|node| &node.value_type)
    }

    /// The admitted location of an arena node: its paired
    /// `protocol_locations` row for a slot-walk application, `None` where
    /// the admitted wire carries none for it (FR-390, FSpec FR-322-AC-310).
    pub fn location(&self, index: usize) -> Option<&Location> {
        self.nodes
            .get(index)
            .and_then(|node| node.location.as_ref())
    }

    /// The occurrence key of the node at an arena index when it has just one `expression`
    /// occurrence; `None` otherwise.
    pub fn occurrence(
        &self,
        index: usize,
    ) -> Option<&fsl_foundation::source::provenance::OccurrenceKey> {
        self.nodes
            .get(index)
            .and_then(|node| node.occurrence.as_ref())
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
            node: &function.key,
            body: function.body,
            slots: function.slots,
            parameters: &function.parameters,
            result_type: function.result_type.as_ref()?,
        })
    }

    /// The selected entry function when this projection was built for a
    /// top-level function call. Protocol-slot projections have no selected
    /// entry function.
    pub fn selected_function(&self) -> Option<ReaderProtocolFunction<'_>> {
        self.function(self.selected_function?)
    }

    /// Structural key of the selected top-level function, when present.
    pub fn selected_function_node(&self) -> Option<&NodeKey> {
        self.selected_function_node.as_ref()
    }

    /// The number of frame slots the slot's root context uses: its binders
    /// and the binders of every `case` it holds outside a function.
    pub fn frame_slots(&self) -> usize {
        self.frame_slots
    }

    /// Locate every node of the root context under `owner`, by its
    /// child-index path from the root: a clause term is checked as its own
    /// expression rooted at the term, owned by its clause site (FR-337), so
    /// this is the location the source route's checked term gives each node.
    /// A node reached twice keeps the location of its first walk position.
    fn locate_under(&mut self, owner: Origin) {
        self.locate_from(self.root, owner);
    }

    /// Locate each term of every function body from the admitted
    /// `body_locations` rows of its function (FSpec FR-322): visit `k` of an
    /// application in the body's walk takes the application's row `k` of the
    /// function, in (occurrence, `term_ordinal`) order. A node the wire
    /// carries no row for keeps no location, and the evaluator faults rather
    /// than invent one (FR-390).
    fn locate_function_bodies(&mut self, admitted: &CheckedPackageV2) -> Result<(), InternalFault> {
        let Some(data) = admitted.checked_term_data() else {
            return Ok(());
        };
        let has_rows: BTreeSet<CheckedNodeId> = data
            .body_locations
            .iter()
            .map(|row| {
                let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &row.location.origin;
                function.clone()
            })
            .collect();
        let mut rows: BTreeMap<CheckedNodeId, BTreeMap<CheckedNodeId, Vec<Location>>> =
            BTreeMap::new();
        for projected in &self.functions {
            let function = &projected.node;
            if !has_rows.contains(function) {
                continue;
            }
            let work_limit = body_location_allowance(admitted, function)
                .ok_or_else(|| fault("body-location-reconstruction-size-fits"))?;
            rows.insert(
                function.clone(),
                reconstructed_body_locations(admitted, function, work_limit)?,
            );
        }
        let mut located = Vec::new();
        for function in &self.functions {
            let Some(applications) = rows.get(&function.node) else {
                continue;
            };
            let mut seen: BTreeMap<&CheckedNodeId, usize> = BTreeMap::new();
            for position in walk(&self.nodes, function.body) {
                let Some(node) = self.nodes.get(position) else {
                    continue;
                };
                for application in node.terms() {
                    let visit = seen.entry(application).or_default();
                    let row = applications
                        .get(application)
                        .and_then(|group| group.get(*visit));
                    *visit += 1;
                    if node.term.as_ref() == Some(application) {
                        let location =
                            row.ok_or_else(|| fault("projected-body-term-has-captured-location"))?;
                        located.push((position, location.clone()));
                    }
                }
            }
        }
        for (position, location) in located {
            if let Some(node) = self.nodes.get_mut(position) {
                node.location = Some(location);
            }
        }
        // A `let`, conditional or `case` is no term and has no row (FSpec
        // FR-322), so one the author wrote stays unlocated: an evaluator never
        // invents a location. The checker locates the `let` it synthesizes
        // for an IEEE ordering at its body's position, so that node takes its
        // last child's location, children first.
        for function in &self.functions {
            if !has_rows.contains(&function.node) {
                continue;
            }
            for position in walk(&self.nodes, function.body).into_iter().rev() {
                let Some(node) = self.nodes.get(position) else {
                    continue;
                };
                if node.location.is_some()
                    || !matches!(
                        &node.kind,
                        NodeKind::Let { name, .. } if crate::introduced_binders::is_ieee_operand(name)
                    )
                {
                    continue;
                }
                let inherited = node
                    .children
                    .last()
                    .and_then(|child| self.nodes.get(*child))
                    .and_then(|child| child.location.clone());
                if let Some(node) = self.nodes.get_mut(position) {
                    node.location = inherited;
                }
            }
        }
        Ok(())
    }

    fn locate_from(&mut self, root: usize, owner: Origin) {
        let mut pending = vec![(root, Location::root(owner))];
        while let Some((index, location)) = pending.pop() {
            let Some(node) = self.nodes.get_mut(index) else {
                continue;
            };
            if node.location.is_some() {
                continue;
            }
            let children = node.children.clone();
            let coerced = matches!(node.kind, NodeKind::Coerce(..));
            node.location = Some(location.clone());
            if coerced {
                // The checker locates an implicit coercion at its operand's
                // position, so the operand shares the coercion's location.
                pending.extend(children.into_iter().map(|child| (child, location.clone())));
                continue;
            }
            pending.extend(
                children
                    .into_iter()
                    .enumerate()
                    .rev()
                    .map(|(position, child)| (child, location.child(position))),
            );
        }
    }

    /// The location of every node of the root context a `pre(..)` reads
    /// (FR-104): each node below a `Pre` node, which observes the pre
    /// snapshot. Empty for a projection with no location.
    pub fn pre_read_locations(&self) -> Vec<Location> {
        let mut found = Vec::new();
        let mut pending = vec![(self.root, false)];
        let mut seen = BTreeSet::new();
        while let Some((index, under_pre)) = pending.pop() {
            if !seen.insert((index, under_pre)) {
                continue;
            }
            let Some(node) = self.nodes.get(index) else {
                continue;
            };
            if under_pre {
                found.extend(node.location.clone());
            }
            let below = under_pre || matches!(node.kind, NodeKind::Pre(_));
            pending.extend(node.children.iter().map(|child| (*child, below)));
        }
        found
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
    /// Structural identity of the admitted function declaration.
    pub node: &'a NodeKey,
    /// Function body node index.
    pub body: usize,
    /// Number of checked parameter slots.
    pub slots: usize,
    /// The checked parameters, in declaration order.
    pub parameters: &'a [ReaderProtocolParameter],
    /// The checked result type.
    pub result_type: &'a ValueType,
}

/// One checked parameter of a reader-projected pure function.
pub struct ReaderProtocolParameter {
    /// Parameter name admitted from its declaration.
    pub name: String,
    /// Structural key of the parameter declaration.
    pub node: NodeKey,
    /// Parameter type admitted from its checked type node.
    pub value_type: ValueType,
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
    Exhausted(crate::NativeJoinFailure),
}

/// One slot to project: its root, origin name and binders in local-slot order.
pub(crate) struct SlotRequest {
    pub(crate) root: NodeKey,
    pub(crate) origin: String,
    pub(crate) binders: Vec<NodeKey>,
    /// The checker binders in scope that no local slot holds: a term that reads
    /// only some of its clause's binders is given slots for those alone, while
    /// the checker numbers an introduced binder's level after every binder of
    /// the clause.
    pub(crate) level_offset: usize,
}

/// Project every slot of `admitted`, over the selected `identities` the view
/// entry joined once under the pass's native budget, and pair each projected slot's walk with
/// its origin's `protocol_locations` rows (FR-390 view creation).
pub(crate) fn project_package(
    admitted: &CheckedPackageV2,
    emission: &crate::Emission,
    package: &str,
    requests: Vec<SlotRequest>,
    identities: filament_contract_model::SelectedExactIdentities,
    dependencies: &[(&CheckedPackageV2, &crate::Emission)],
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<ReaderProtocolSlots, PackageRefusal> {
    use crate::checked_v2::EvaluationWitnessSupplier;
    let mut index = source_index(admitted, emission, identities, budget)?;
    for (admitted, emission) in dependencies {
        let supplied = crate::checked_v2::SourceWitnessSupplier::new(emission)
            .supply(admitted, budget)
            .map_err(|failure| match failure {
                crate::checked_v2::WitnessSupplyFailure::Exhausted(native) => {
                    PackageRefusal::Exhausted((*native).into())
                }
                crate::checked_v2::WitnessSupplyFailure::Rows(rows) => {
                    PackageRefusal::Exhausted(rows.into())
                }
                crate::checked_v2::WitnessSupplyFailure::Fault => {
                    PackageRefusal::Fault(fault("dependency-emission-witness-is-admitted"))
                }
            })?;
        let identities = supplied
            .identities
            .ok_or_else(|| {
                PackageRefusal::Fault(fault("dependency-emission-holds-identity-witness"))
            })?
            .validate_and_rebind(admitted, budget)
            .map_err(|error| PackageRefusal::Exhausted(error.into()))?;
        index
            .dependencies
            .push(source_index(admitted, emission, identities, budget)?);
    }
    project_index(&index, admitted, package, requests)
}

fn source_index<'wire>(
    admitted: &'wire CheckedPackageV2,
    emission: &crate::Emission,
    identities: filament_contract_model::SelectedExactIdentities,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<WireIndex<'wire>, PackageRefusal> {
    let mut projections = BTreeMap::new();
    for node in &admitted.resolved_graph().nodes {
        let projection = admitted.project_exact_type(&node.node_id, &identities, budget);
        if let Err(denial) = &projection {
            if fsl_semantics::model::intake::is_native_budget_denial(denial) {
                return Err(PackageRefusal::Exhausted(denial.clone().into()));
            }
        }
        projections.insert(node.node_id.clone(), projection);
    }
    let equality =
        admit_equality_decisions(admitted, ProjectionSource::Source { emission }, budget)
            .map_err(PackageRefusal::Exhausted)?;
    WireIndex::new(
        std::borrow::Cow::Owned(equality),
        std::borrow::Cow::Owned(identities),
        admitted,
        std::borrow::Cow::Owned(projections),
    )
    .map_err(PackageRefusal::Fault)
}

fn project_index(
    index: &WireIndex<'_>,
    admitted: &CheckedPackageV2,
    package: &str,
    requests: Vec<SlotRequest>,
) -> Result<ReaderProtocolSlots, PackageRefusal> {
    let rows = OriginRows::new(admitted);
    let mut slots = BTreeMap::new();
    let mut refusal: Option<UnsupportedView> = None;
    for request in requests {
        let slot = match index.project_slot(&request) {
            Ok(mut projection) => {
                let found = rows.pair(&mut projection, &request.origin, package, index)?;
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
            Err(BuildError::Suspended) => {
                return Err(PackageRefusal::Fault(self::fault(
                    "reader-projection-resolves-its-suspensions",
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
        "filament.op.string.eq"
            | "filament.op.string.ne"
            | "filament.op.uuid.eq"
            | "filament.op.uuid.ne"
            | "filament.op.timestamp.eq"
            | "filament.op.timestamp.ne"
            | "filament.op.boolean.eq"
            | "filament.op.boolean.ne"
            | "filament.op.integer.eq"
            | "filament.op.integer.ne"
            | "filament.op.rational.eq"
            | "filament.op.rational.ne"
            | "filament.op.decimal.eq"
            | "filament.op.decimal.ne"
            | "filament.op.quantity.eq"
            | "filament.op.quantity.ne"
            | "filament.op.structural.eq"
            | "filament.op.structural.ne"
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
    /// The node being projected reads a child [`ReaderBuilder::project_tree`]
    /// has not projected yet; the walk projects the child first and then the
    /// node again. It never leaves the walk.
    Suspended,
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
type ExactProjections = BTreeMap<
    CheckedNodeId,
    Result<
        filament_contract_model::ExactTypeProjection,
        filament_contract_model::ExactTypeProjectionError,
    >,
>;

/// Equality decisions admitted once under the caller's shared native allowance.
pub(crate) type EqualityDecisions = BTreeMap<
    CheckedNodeId,
    Result<filament_semantic_value::declaration::AdmittedEqualityDecision, EqualityWitnessRefusal>,
>;

pub(crate) fn admit_equality_decisions(
    admitted: &CheckedPackageV2,
    source: ProjectionSource<'_>,
    budget: &mut filament_contract_model::NativeJoinBudget,
) -> Result<EqualityDecisions, crate::NativeJoinFailure> {
    let mut decisions = BTreeMap::new();
    let rows = match &source {
        ProjectionSource::Source { emission } => Some(emission.equality_rows(budget)?),
        _ => None,
    };
    let Some(data) = admitted.checked_term_data() else {
        return Ok(decisions);
    };
    for decision in &data.equality_decisions {
        let result = match &source {
            ProjectionSource::Source { emission } => {
                let result = emission.admitted_equality_from(
                    rows.as_ref().expect("source rows were retained"),
                    admitted,
                    &decision.application,
                    &decision.occurrence,
                    budget,
                );
                match result {
                    Ok(value) => Ok(value),
                    Err(error)
                        if matches!(
                            error.code,
                            filament_contract_model::AdmittedEqualityReadErrorCode::WorkLimit
                                | filament_contract_model::AdmittedEqualityReadErrorCode::StorageLimit
                        ) =>
                    {
                        return Err(crate::NativeJoinFailure::Equality(Box::new(error)))
                    }
                    Err(_) => Err(EqualityWitnessRefusal::Unknown),
                }
            }
            ProjectionSource::Bytes { equality: None } => Err(EqualityWitnessRefusal::Missing),
            ProjectionSource::Bytes {
                equality: Some(rows),
            } => match rows.decision(admitted, &decision.application, budget) {
                Ok(value) => Ok(value),
                Err(crate::equality_file::EqualityDecisionFailure::Witness(error)) => Err(error),
                Err(crate::equality_file::EqualityDecisionFailure::Exhausted(error)) => {
                    return Err(crate::NativeJoinFailure::Equality(error))
                }
            },
        };
        decisions.insert(decision.application.clone(), result);
    }
    Ok(decisions)
}

struct WireIndex<'wire> {
    dependencies: Vec<WireIndex<'wire>>,
    populations: Option<&'wire crate::bare::BarePackage<'wire>>,
    equality: std::borrow::Cow<'wire, EqualityDecisions>,
    projections: std::borrow::Cow<'wire, ExactProjections>,
    admitted: &'wire CheckedPackageV2,
    nodes: BTreeMap<&'wire CheckedNodeId, &'wire CheckedSemanticNodeV2>,
    ids: BTreeMap<NodeKey, &'wire CheckedNodeId>,
    graph: WireGraph,
    identities: std::borrow::Cow<'wire, filament_contract_model::SelectedExactIdentities>,
    /// Whether the walk reads names and term structure for printing an atom,
    /// which needs no exact type of a declaration the identity rows select
    /// (FR-931 "The lowering input").
    print: bool,
}

/// What the projection reads beside the admitted package.
pub(crate) enum ProjectionSource<'wire> {
    /// The in-process route: the emission's equality rows. The identities
    /// are the source graph's, joined once by the view entry.
    Source { emission: &'wire crate::Emission },
    /// The bytes route: where the handoff carried one, the equality rows
    /// (FR-531). The identity witness the supply gave is borrowed.
    Bytes {
        equality: Option<&'wire ReadEqualityRows>,
    },
}

impl<'wire> WireIndex<'wire> {
    fn new(
        equality: std::borrow::Cow<'wire, EqualityDecisions>,
        identities: std::borrow::Cow<'wire, filament_contract_model::SelectedExactIdentities>,
        admitted: &'wire CheckedPackageV2,
        projections: std::borrow::Cow<'wire, ExactProjections>,
    ) -> Result<Self, InternalFault> {
        let graph = crate::checked_v2::wire_graph(admitted)
            .map_err(|_| fault("reader-protocol-graph-has-structural-keys"))?;
        let mut nodes = BTreeMap::new();
        let mut ids = BTreeMap::new();
        for node in &admitted.resolved_graph().nodes {
            nodes.insert(&node.node_id, node);
            let key = graph
                .key(&node.node_id)
                .map_err(|_| fault("reader-protocol-graph-has-structural-keys"))?;
            ids.insert(key, &node.node_id);
        }
        Ok(Self {
            dependencies: Vec::new(),
            populations: None,
            equality,
            projections,
            admitted,
            nodes,
            ids,
            graph,
            identities,
            print: false,
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
    ) -> Result<filament_semantic_value::declaration::AdmittedEqualityDecision, BuildError> {
        let _ = admitted;
        self.equality
            .get(id)
            .cloned()
            .unwrap_or(Err(EqualityWitnessRefusal::Unknown))
            .map_err(BuildError::Equality)
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

    /// The type a print walk gives node `id` when its exact type is not
    /// selected: the record, tuple or union declaration its own type node is,
    /// or the model object type it references, each named by the structural
    /// key of that declaration node alone.
    fn declared_type(&self, id: &CheckedNodeId) -> Option<ValueType> {
        let node = self.node(id)?;
        let declaration = if self.is_declared_type(node) {
            node
        } else {
            self.node(&node.semantic_type)?
        };
        let key = self.key(&declaration.node_id).ok()?;
        if has_tag(declaration, WireTag::CompositeType)
            && (has_form(declaration, WireTag::CompositeType, WireForm::Record)
                || has_form(declaration, WireTag::CompositeType, WireForm::Union)
                || has_form(declaration, WireTag::CompositeType, WireForm::Tuple))
        {
            return Some(ValueType::Composite(key));
        }
        let object = if has_form(declaration, WireTag::CompositeType, WireForm::Reference) {
            let CheckedTerm::Aggregate { members } = &declaration.body else {
                return None;
            };
            let target = reference_target(members.first()?)?;
            self.key(target).ok()?
        } else {
            key
        };
        let object_node = self.ids.get(&object)?;
        self.admitted
            .model_object_fields(object_node)
            .ok()
            .map(|_| ValueType::Reference(print_effective_id(&object)))
    }

    /// Whether `node` is itself a type declaration node.
    fn is_declared_type(&self, node: &CheckedSemanticNodeV2) -> bool {
        has_tag(node, WireTag::CompositeType)
            || self.admitted.model_object_fields(&node.node_id).is_ok()
    }

    fn types(&self) -> TypeReader<'_> {
        TypeReader {
            admitted: self.admitted,
            identities: &self.identities,
            nodes: &self.nodes,
            graph: &self.graph,
            projections: &self.projections,
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
            root_index: self,
            owner: 0,
            index: self,
            nodes: Vec::new(),
            functions: Vec::new(),
            function_indexes: BTreeMap::new(),
            binders: BTreeMap::new(),
            root_slots: request.binders.len(),
            level_offset: request.level_offset,
            active: None,
            suspended: Vec::new(),
            ordinal: 0,
        };
        builder.read_functions(root)?;
        let root_index = builder.project_tree(NodeRequest {
            owner: 0,
            id: root.clone(),
            context: Context::Root,
            locals: std::rc::Rc::new(locals),
        })?;
        let mut projection = ReaderProtocolProjection {
            nodes: builder.nodes,
            functions: builder.functions,
            root: root_index,
            selected_function: None,
            selected_function_node: None,
            frame_slots: builder.root_slots,
            locals: local_keys,
        };
        projection.locate_function_bodies(self.admitted)?;
        for dependency in &self.dependencies {
            projection.locate_function_bodies(dependency.admitted)?;
        }
        Ok(projection)
    }

    fn project_function(
        &self,
        root: &CheckedNodeId,
    ) -> Result<ReaderProtocolProjection, BuildError> {
        let selected_function_node = Some(self.key(root).map_err(BuildError::Fault)?);
        let mut builder = ReaderBuilder {
            root_index: self,
            owner: 0,
            index: self,
            nodes: Vec::new(),
            functions: Vec::new(),
            function_indexes: BTreeMap::new(),
            binders: BTreeMap::new(),
            root_slots: 0,
            level_offset: 0,
            active: None,
            suspended: Vec::new(),
            ordinal: 0,
        };
        builder.read_functions(root)?;
        let selected = *builder
            .function_indexes
            .get(&(0, root.clone()))
            .ok_or_else(|| fault("reader-selected-function-is-in-function-table"))?;
        let function = builder
            .functions
            .get(selected)
            .ok_or_else(|| fault("reader-selected-function-index-is-retained"))?;
        let root = function.body;
        let frame_slots = function.slots;
        let mut projection = ReaderProtocolProjection {
            nodes: builder.nodes,
            functions: builder.functions,
            root,
            selected_function: Some(selected),
            selected_function_node,
            frame_slots,
            locals: BTreeMap::new(),
        };
        projection.locate_function_bodies(self.admitted)?;
        for dependency in &self.dependencies {
            projection.locate_function_bodies(dependency.admitted)?;
        }
        Ok(projection)
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
            let filament_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name } =
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
    /// A function parameter's type has no native run value form.
    FunctionParameterType {
        position: usize,
        view: Box<UnsupportedView>,
    },
    /// A function result's type has no native run value form.
    FunctionResultType {
        function: String,
        view: Box<UnsupportedView>,
    },
    /// The equality rows do not give an application its decision.
    Equality(EqualityWitnessRefusal),
    /// The admitted package broke a projection invariant.
    Fault(InternalFault),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FunctionTypePosition {
    Parameter(usize),
    Result,
}

fn function_type_position(
    reason: &UnsupportedReason,
    parameter_position: Option<usize>,
    body_is_node: bool,
) -> Option<FunctionTypePosition> {
    if !matches!(reason, UnsupportedReason::MissingTypeRepresentation) {
        return None;
    }
    parameter_position
        .map(FunctionTypePosition::Parameter)
        .or_else(|| body_is_node.then_some(FunctionTypePosition::Result))
}

/// The slot walks of a package read from its bytes alone (FR-533 "Slot
/// naming on the bytes route"): the projection of FR-390 over the package's
/// own nodes, the identity witness and the equality rows.
pub(crate) struct BareWalker<'wire> {
    index: WireIndex<'wire>,
}

impl<'wire> BareWalker<'wire> {
    /// Read named population operands from the already admitted catalog.
    pub(crate) fn with_populations(
        mut self,
        package: &'wire crate::bare::BarePackage<'wire>,
    ) -> Self {
        self.index.populations = Some(package);
        self
    }

    /// A walk that reads a term for printing: a node whose exact type the
    /// identity rows do not select is typed by its own declaration node.
    pub(crate) fn printing(mut self) -> Self {
        self.index.print = true;
        self
    }

    pub(crate) fn new(
        admitted: &'wire CheckedPackageV2,
        identities: &'wire filament_contract_model::SelectedExactIdentities,
        equality: &'wire EqualityDecisions,
        projections: &'wire ExactProjections,
        dependencies: &'wire [crate::BarePackage<'wire>],
    ) -> Result<Self, InternalFault> {
        let mut index = WireIndex::new(
            std::borrow::Cow::Borrowed(equality),
            std::borrow::Cow::Borrowed(identities),
            admitted,
            std::borrow::Cow::Borrowed(projections),
        )?;
        for dependency in dependencies {
            let identities = dependency
                .identities()
                .ok_or_else(|| fault("dependency-identity-witness-is-admitted"))?;
            index.dependencies.push(WireIndex::new(
                std::borrow::Cow::Borrowed(dependency.equality_decisions()),
                std::borrow::Cow::Borrowed(identities),
                dependency.package(),
                std::borrow::Cow::Borrowed(dependency.exact_type_projections()),
            )?);
        }
        Ok(Self { index })
    }

    /// The pure function `function` selects, when the package holds it.
    fn selected_function(&self, function: FunctionSelector<'_>) -> Option<&CheckedSemanticNodeV2> {
        match function {
            FunctionSelector::Name(name) => self.index.nodes.values().copied().find(|node| {
                has_form(node, WireTag::Function, WireForm::PureFunction)
                    && node.declaration.as_ref().is_some_and(|declaration| {
                        declaration.qualified_name.len() == 1
                            && declaration.qualified_name[0].as_ref() == name
                    })
            }),
            FunctionSelector::Node(key) => self
                .index
                .ids
                .get(key)
                .and_then(|id| self.index.node(id))
                .filter(|node| has_form(node, WireTag::Function, WireForm::PureFunction)),
        }
    }

    /// Read the checked result type of a selected pure function without
    /// projecting its body. This lets callers reject an unrepresentable
    /// result before a body operation can produce an earlier refusal.
    pub(crate) fn function_result_type(
        &self,
        function: FunctionSelector<'_>,
    ) -> Result<Option<ReaderFunctionResultType>, WalkFailure> {
        let Some(selected) = self.selected_function(function) else {
            return Ok(None);
        };
        let function = function_label(selected)
            .ok_or_else(|| WalkFailure::Fault(fault("reader-function-name-is-admitted")))?;
        let (_, body) = function_parts(&selected.body)
            .ok_or_else(|| WalkFailure::Fault(fault("reader-pure-function-has-checked-body")))?;
        let CheckedTerm::Reference { target } = body else {
            return Err(WalkFailure::Fault(fault(
                "reader-function-body-has-checked-identity",
            )));
        };
        let result_node = self.index.key(target).map_err(WalkFailure::Fault)?;
        let value_type =
            node_value_type(&self.index.types(), target).map_err(|error| match error {
                TypeError::Unsupported => WalkFailure::FunctionResultType {
                    function,
                    view: Box::new(UnsupportedView {
                        package: self.index.admitted.name().to_owned(),
                        locus: ViewLocus::node(result_node),
                        reason: UnsupportedReason::MissingTypeRepresentation,
                    }),
                },
                TypeError::Fault(fault) => WalkFailure::Fault(fault),
            })?;
        Ok(Some(ReaderFunctionResultType {
            function_node: self
                .index
                .key(&selected.node_id)
                .map_err(WalkFailure::Fault)?,
            value_type,
        }))
    }

    /// Project the named pure function and every function its body reaches.
    /// The function's body has no protocol-origin location rows, so this
    /// route retains the admitted node types and evaluator operations only.
    pub(crate) fn project_function(
        &self,
        function: FunctionSelector<'_>,
    ) -> Result<Option<ReaderProtocolProjection>, WalkFailure> {
        let Some(selected) = self.selected_function(function) else {
            return Ok(None);
        };
        let failure = match self.index.project_function(&selected.node_id) {
            Ok(projection) => return Ok(Some(projection)),
            Err(BuildError::Fault(fault)) => return Err(WalkFailure::Fault(fault)),
            Err(failure) => failure,
        };
        if let Some(refusal) = self.index.equality_refusal(&selected.node_id) {
            return Err(WalkFailure::Equality(refusal));
        }
        match failure {
            BuildError::Equality(refusal) => Err(WalkFailure::Equality(refusal)),
            BuildError::Unsupported { reason, node } => {
                let view = UnsupportedView {
                    package: self.index.admitted.name().to_owned(),
                    locus: ViewLocus::node(self.index.key(&node).map_err(WalkFailure::Fault)?),
                    reason,
                };
                let (parameters, body) = function_parts(&selected.body).ok_or_else(|| {
                    WalkFailure::Fault(fault("reader-pure-function-has-checked-body"))
                })?;
                let parameter_position = parameters.iter().position(|parameter| {
                    matches!(parameter, CheckedTerm::Reference { target } if target == &node)
                });
                let body_is_node =
                    matches!(body, CheckedTerm::Reference { target } if target == &node);
                match function_type_position(&view.reason, parameter_position, body_is_node) {
                    Some(FunctionTypePosition::Parameter(position)) => {
                        Err(WalkFailure::FunctionParameterType {
                            position,
                            view: Box::new(view),
                        })
                    }
                    Some(FunctionTypePosition::Result) => {
                        let function = function_label(selected).ok_or_else(|| {
                            WalkFailure::Fault(fault("reader-function-name-is-admitted"))
                        })?;
                        Err(WalkFailure::FunctionResultType {
                            function,
                            view: Box::new(view),
                        })
                    }
                    None => Err(WalkFailure::Unsupported(view)),
                }
            }
            BuildError::Fault(fault) => Err(WalkFailure::Fault(fault)),
            BuildError::Suspended => Err(WalkFailure::Fault(fault(
                "reader-function-projection-resolves-its-suspensions",
            ))),
        }
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
            BuildError::Suspended => Err(WalkFailure::Fault(fault(
                "reader-projection-resolves-its-suspensions",
            ))),
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
            level_offset: 0,
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
            level_offset: 0,
        };
        let mut projection = self.slot(&request)?;
        let rows = OriginRows::new(self.index.admitted);
        match rows.pair(&mut projection, &request.origin, package, &self.index) {
            Ok(None) => Ok(projection),
            Ok(Some(view)) => Err(WalkFailure::Unsupported(view)),
            Err(PackageRefusal::Unsupported(view)) => Err(WalkFailure::Unsupported(view)),
            Err(PackageRefusal::Fault(fault)) => Err(WalkFailure::Fault(fault)),
            Err(PackageRefusal::Exhausted(_)) => Err(WalkFailure::Fault(fault(
                "location-pairing-does-not-project-types",
            ))),
        }
    }

    /// The projection of the clause term rooted at `root` over `binders`
    /// (FR-940 "Package forms of the source-checked evaluators"): a temporal
    /// clause's `holds` condition or activation guard. The package writes no
    /// `protocol_locations` row for a clause term (FR-383 rows are written for
    /// protocol slots only), so each node is located under `owner`, the
    /// term's clause site.
    pub(crate) fn project_term(
        &self,
        root: &NodeKey,
        binders: &[NodeKey],
        owner: Origin,
    ) -> Result<ReaderProtocolProjection, WalkFailure> {
        self.project_term_at(root, binders, 0, owner)
    }

    /// [`Self::project_term`] for a term whose scope holds `level_offset`
    /// checker binders beyond `binders`.
    pub(crate) fn project_term_at(
        &self,
        root: &NodeKey,
        binders: &[NodeKey],
        level_offset: usize,
        owner: Origin,
    ) -> Result<ReaderProtocolProjection, WalkFailure> {
        let mut projection = self.slot(&SlotRequest {
            root: root.clone(),
            origin: String::new(),
            binders: binders.to_vec(),
            level_offset,
        })?;
        projection.locate_under(owner);
        projection
            .locate_function_bodies(self.index.admitted)
            .map_err(WalkFailure::Fault)?;
        Ok(projection)
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

pub(crate) struct ReaderFunctionResultType {
    pub(crate) function_node: NodeKey,
    pub(crate) value_type: ValueType,
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
    root_index: &'index WireIndex<'wire>,
    owner: usize,
    index: &'index WireIndex<'wire>,
    nodes: Vec<ReaderProjectedNode>,
    functions: Vec<ReaderFunction>,
    function_indexes: BTreeMap<(usize, CheckedNodeId), usize>,
    /// The binder of each `let` or query scope read so far, which a node
    /// projected again does not look up again.
    binders: BTreeMap<(usize, CheckedNodeId), Option<CheckedNodeId>>,
    root_slots: usize,
    /// [`SlotRequest::level_offset`].
    level_offset: usize,
    /// The node [`Self::project_tree`] is projecting now.
    active: Option<Frame>,
    /// The children the active node's projection asked for and did not
    /// find, by the ordinal of the lookup that asked.
    suspended: Vec<(usize, NodeRequest)>,
    /// The number of child lookups the active attempt has made.
    ordinal: usize,
}

/// A node to project: where it lives and the locals its subtree reads.
struct NodeRequest {
    owner: usize,
    id: CheckedNodeId,
    context: Context,
    locals: std::rc::Rc<BTreeMap<CheckedNodeId, usize>>,
}

/// A child's finished projection, kept by the node that asked for it.
struct Finished {
    index: usize,
    /// The context's frame size once the child's subtree was projected.
    slots: usize,
}

/// One node on [`ReaderBuilder::project_tree`]'s heap stack.
struct Frame {
    request: std::rc::Rc<NodeRequest>,
    /// The context's frame size when the node was first entered.
    entered: usize,
    /// The node's children projected so far, by the ordinal of the lookup
    /// that asked for each: a node's projection asks for the same children in
    /// the same order every time it is attempted.
    finished: BTreeMap<usize, Finished>,
    /// The children still to project before the node is attempted again,
    /// next last.
    queue: Vec<(usize, NodeRequest)>,
    /// The ordinal of the child being projected.
    waiting: usize,
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

    fn select_owner(&mut self, owner: usize) -> Result<(), BuildError> {
        self.index = if owner == 0 {
            self.root_index
        } else {
            self.root_index
                .dependencies
                .get(owner - 1)
                .ok_or_else(|| fault("reader-function-owner-is-retained"))?
        };
        self.owner = owner;
        Ok(())
    }

    fn callee(&self, term: &CheckedTerm) -> Result<(usize, CheckedNodeId), BuildError> {
        match term {
            CheckedTerm::Reference { target } => Ok((self.owner, target.clone())),
            CheckedTerm::DependencyReference { package, node } => {
                let (position, owner) = self
                    .root_index
                    .dependencies
                    .iter()
                    .enumerate()
                    .find(|(_, owner)| owner.admitted.name() == package.as_ref())
                    .ok_or_else(|| fault("reader-selected-library-is-retained"))?;
                let key = fsl_semantics::check::wire_id::node_key(node)
                    .map_err(|_| fault("reader-library-function-has-structural-key"))?;
                let target = owner
                    .ids
                    .get(&key)
                    .ok_or_else(|| fault("reader-library-function-is-admitted"))?;
                Ok((position + 1, (*target).clone()))
            }
            _ => Err(fault("reader-call-has-callee").into()),
        }
    }

    fn read_functions(&mut self, root: &CheckedNodeId) -> Result<(), BuildError> {
        let mut reachable = BTreeSet::new();
        let mut pending = vec![(0, root.clone())];
        let mut declarations = Vec::new();
        while let Some((owner, id)) = pending.pop() {
            if !reachable.insert((owner, id.clone())) {
                continue;
            }
            self.select_owner(owner)?;
            let node = self
                .index
                .node(&id)
                .ok_or_else(|| fault("reader-reachable-node-is-admitted"))?;
            pending.extend(
                node.body
                    .reference_targets()
                    .into_iter()
                    .cloned()
                    .map(|id| (owner, id)),
            );
            if let CheckedTerm::Application {
                operation,
                arguments,
                ..
            } = &node.body
            {
                if operation.identity() == Some("filament.op.function.call") {
                    let callee = arguments
                        .first()
                        .ok_or_else(|| fault("reader-call-has-callee"))?;
                    pending.push(self.callee(callee)?);
                }
            }
            if has_form(node, WireTag::Function, WireForm::PureFunction) {
                declarations.push((owner, id));
            }
        }
        // Assign every function index before walking any body, including local
        // recursion and a dependency reached from several imported functions.
        for (owner, id) in &declarations {
            self.select_owner(*owner)?;
            let node = self
                .index
                .node(id)
                .ok_or_else(|| fault("reader-function-is-admitted"))?;
            let (parameters, body) = function_parts(&node.body)
                .ok_or_else(|| fault("reader-pure-function-has-checked-body"))?;
            let mut parameter_ids = Vec::new();
            let mut projected = Vec::new();
            for parameter in parameters {
                let CheckedTerm::Reference { target } = parameter else {
                    return Err(fault("reader-function-parameter-has-checked-identity").into());
                };
                let parameter = self
                    .index
                    .node(target)
                    .ok_or_else(|| fault("reader-function-parameter-is-admitted"))?;
                let name = parameter_name(&parameter.body)
                    .ok_or_else(|| fault("reader-function-parameter-name-is-admitted"))?;
                projected.push(ReaderProtocolParameter {
                    name: name.to_owned(),
                    node: self.index.key(target)?,
                    value_type: self.value_type(target)?,
                });
                parameter_ids.push(target.clone());
            }
            let CheckedTerm::Reference { .. } = body else {
                return Err(fault("reader-function-body-has-checked-identity").into());
            };
            let index = self.functions.len();
            self.function_indexes.insert((*owner, id.clone()), index);
            self.functions.push(ReaderFunction {
                node: id.clone(),
                name: function_label(node)
                    .ok_or_else(|| fault("reader-function-name-is-admitted"))?,
                key: self.index.key(id)?,
                body: usize::MAX,
                slots: parameter_ids.len(),
                parameters: projected,
                result_type: None,
            });
        }
        for (index, (owner, id)) in declarations.iter().enumerate() {
            self.select_owner(*owner)?;
            let node = self
                .index
                .node(id)
                .ok_or_else(|| fault("reader-function-is-admitted"))?;
            let (parameters, body) = function_parts(&node.body)
                .ok_or_else(|| fault("reader-pure-function-has-checked-body"))?;
            let CheckedTerm::Reference { target: body } = body else {
                return Err(fault("reader-function-body-has-checked-identity").into());
            };
            let locals = parameters
                .iter()
                .enumerate()
                .map(|(slot, parameter)| {
                    let CheckedTerm::Reference { target } = parameter else {
                        return Err(fault("reader-function-parameter-has-checked-identity"));
                    };
                    Ok((target.clone(), slot))
                })
                .collect::<Result<BTreeMap<_, _>, _>>()?;
            let body = self.project_tree(NodeRequest {
                owner: *owner,
                id: body.clone(),
                context: Context::Function(index),
                locals: std::rc::Rc::new(locals),
            })?;
            self.functions[index].body = body;
            self.functions[index].result_type = Some(
                self.nodes
                    .get(body)
                    .ok_or_else(|| fault("reader-function-body-is-projected"))?
                    .value_type
                    .clone(),
            );
        }
        self.select_owner(0)?;
        Ok(())
    }

    /// The occurrence key of `node` (ADR-013 O-07) when it has exactly one
    /// `expression` occurrence. A node shared by structurally equal
    /// expressions has several, and the wire pairs none with a position of
    /// this walk, so such a node names none.
    fn sole_expression_occurrence(
        &self,
        id: &CheckedNodeId,
        node: &CheckedSemanticNodeV2,
    ) -> Option<fsl_foundation::source::provenance::OccurrenceKey> {
        let mut expression = node.occurrences.iter().filter(|occurrence| {
            occurrence.role == filament_contract_model::CheckedOccurrenceRole::Expression
        });
        let (Some(only), None) = (expression.next(), expression.next()) else {
            return None;
        };
        Some(fsl_foundation::source::provenance::OccurrenceKey::new(
            self.index.key(id).ok()?,
            filament_exact::Origin::new(
                filament_exact::Role::new(crate::checked_v2::role_spelling(&only.role)),
                only.ordinal,
            ),
        ))
    }

    fn push(&mut self, node: ReaderProjectedNode) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    /// The frame size of `context`.
    fn slots(&self, context: Context) -> usize {
        match context {
            Context::Root => self.root_slots,
            Context::Function(function) => self
                .functions
                .get(function)
                .map_or(0, |function| function.slots),
        }
    }

    /// Set the frame size of `context`.
    fn set_slots(&mut self, context: Context, slots: usize) {
        match context {
            Context::Root => self.root_slots = slots,
            Context::Function(function) => {
                if let Some(function) = self.functions.get_mut(function) {
                    function.slots = slots;
                }
            }
        }
    }

    /// The projection of `id`, as the node being projected reads it. A child
    /// the walk has projected for this node is returned; any other is
    /// requested ([`BuildError::Suspended`]) and the node is projected again
    /// once it exists. A function body shares a node reached twice.
    fn project_node(
        &mut self,
        id: &CheckedNodeId,
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<usize, BuildError> {
        let ordinal = self.ordinal;
        self.ordinal = ordinal.saturating_add(1);
        let finished = self
            .active
            .as_ref()
            .and_then(|frame| frame.finished.get(&ordinal))
            .map(|child| (child.index, child.slots));
        if let Some((index, slots)) = finished {
            // The frame size the child's subtree left, as the node saw it.
            self.set_slots(context, self.slots(context).max(slots));
            return Ok(index);
        }
        self.suspended = vec![(
            ordinal,
            NodeRequest {
                owner: self.owner,
                id: id.clone(),
                context,
                locals: std::rc::Rc::new(locals.clone()),
            },
        )];
        Err(BuildError::Suspended)
    }

    /// The projections of `terms`, in order. Every child of `terms` the walk
    /// has not projected is requested at once, so a node of any width is
    /// attempted twice at most for these children.
    fn project_terms<'a>(
        &mut self,
        parent: &CheckedNodeId,
        terms: impl Iterator<Item = &'a CheckedTerm> + Clone,
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<Vec<usize>, BuildError> {
        let mut ordinal = self.ordinal;
        let mut missing = Vec::new();
        let mut shared = None;
        for term in terms.clone() {
            let CheckedTerm::Reference { target } = term else {
                continue;
            };
            let known = self
                .active
                .as_ref()
                .is_some_and(|frame| frame.finished.contains_key(&ordinal));
            if !known {
                let locals = shared.get_or_insert_with(|| std::rc::Rc::new(locals.clone()));
                missing.push((
                    ordinal,
                    NodeRequest {
                        owner: self.owner,
                        id: target.clone(),
                        context,
                        locals: std::rc::Rc::clone(locals),
                    },
                ));
            }
            ordinal = ordinal.saturating_add(1);
        }
        if !missing.is_empty() {
            self.ordinal = ordinal;
            self.suspended = missing;
            return Err(BuildError::Suspended);
        }
        terms
            .map(|term| self.project_argument(parent, term, context, locals))
            .collect()
    }

    /// Project `root` and its subtree on a heap stack, one frame per node
    /// being projected, however deep the subtree nests (ADR-030 D-1): a node
    /// that reads an unprojected child leaves the stack top, the child is
    /// projected, and the node is projected again with the child at hand.
    /// A projection attempt changes nothing but the projected nodes it adds,
    /// which are dropped when it suspends, and the context's frame size,
    /// which each attempt restarts from the size at the node's entry and
    /// raises by each child's as it reads the child, so every node sees the
    /// sizes the nested projection of its children would give it. The heap
    /// stack holds one frame per node on the path from `root`, and each
    /// frame one entry per child.
    fn project_tree(&mut self, root: NodeRequest) -> Result<usize, BuildError> {
        let mut parents: Vec<Frame> = Vec::new();
        let mut frame = self.enter(root);
        loop {
            let request = std::rc::Rc::clone(&frame.request);
            self.select_owner(request.owner)?;
            self.set_slots(request.context, frame.entered);
            self.ordinal = 0;
            let added = self.nodes.len();
            self.active = Some(frame);
            let result = self.project_fresh(&request.id, request.context, &request.locals);
            frame = self
                .active
                .take()
                .ok_or_else(|| fault("reader-projection-frame-is-active"))?;
            let finished = match result {
                Ok(index) => Finished {
                    index,
                    slots: self.slots(request.context),
                },
                Err(BuildError::Suspended) => {
                    self.nodes.truncate(added);
                    frame.queue = std::mem::take(&mut self.suspended);
                    frame.queue.reverse();
                    parents.push(frame);
                    frame = match self.next_child(&mut parents)? {
                        Some(next) => next,
                        None => parents
                            .pop()
                            .ok_or_else(|| fault("reader-projection-parent-is-waiting"))?,
                    };
                    continue;
                }
                Err(failure) => return Err(failure),
            };
            // The node is projected: its parent takes it, then asks for the
            // children still queued, then is attempted again.
            let Some(mut parent) = parents.pop() else {
                return Ok(finished.index);
            };
            parent.finished.insert(parent.waiting, finished);
            parents.push(parent);
            frame = match self.next_child(&mut parents)? {
                Some(next) => next,
                None => parents
                    .pop()
                    .ok_or_else(|| fault("reader-projection-parent-is-waiting"))?,
            };
        }
    }

    /// The frame of the next child the node on top of `parents` waits for,
    /// entered; a child a function body shares and that is projected by now
    /// is taken at once, as the nested projection would. `None` once the node
    /// waits for no child.
    fn next_child(&mut self, parents: &mut [Frame]) -> Result<Option<Frame>, BuildError> {
        let Some(parent) = parents.last_mut() else {
            return Ok(None);
        };
        let Some((ordinal, wanted)) = parent.queue.pop() else {
            return Ok(None);
        };
        parent.waiting = ordinal;
        Ok(Some(self.enter(wanted)))
    }

    /// A frame for `request`, entered at the context's current frame size.
    fn enter(&self, request: NodeRequest) -> Frame {
        Frame {
            entered: self.slots(request.context),
            request: std::rc::Rc::new(request),
            finished: BTreeMap::new(),
            queue: Vec::new(),
            waiting: 0,
        }
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
        if let Some(package) = self.index.populations {
            let key = self.index.key(id).map_err(BuildError::Fault)?;
            if let Some((population, members)) = package.population_types(&key) {
                return Ok(self.push(ReaderProjectedNode::leaf(
                    NodeKind::NamedPopulation {
                        population: population.clone(),
                        members: members.to_vec(),
                    },
                    // The all_instances/lookup result carries the bound;
                    // the population reference itself is bound-independent.
                    ValueType::Population(None),
                )));
            }
        }
        let node = self.node(id)?;
        let (kind, children, term) = match &node.body {
            CheckedTerm::Literal { .. }
                if matches!(
                    node.body.literal_read().map(|literal| literal.value),
                    Some(filament_contract_model::CheckedLiteralValue::Reference(_))
                ) =>
            {
                let Some(filament_contract_model::CheckedLiteralValue::Reference(identity)) =
                    node.body.literal_read().map(|literal| literal.value)
                else {
                    return Err(fault("reader-reference-literal-has-identity").into());
                };
                let ValueType::Reference(ref object_type) = self.value_type(id)? else {
                    return Err(fault("reader-reference-literal-has-object-type").into());
                };
                let bytes = identity.as_bytes();
                let canonical = !bytes.is_empty()
                    && bytes.len() % 2 == 0
                    && bytes
                        .iter()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte));
                let identity = canonical
                    .then(|| {
                        bytes
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|pair| {
                                u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()
                            })
                            .collect::<Option<Vec<_>>>()
                    })
                    .flatten()
                    .ok_or_else(|| BuildError::Unsupported {
                        reason: UnsupportedReason::MissingLiteralProjection,
                        node: id.clone(),
                    })?;
                (
                    NodeKind::ReferenceLiteral {
                        object_type: object_type.clone(),
                        identity,
                    },
                    Vec::new(),
                    None,
                )
            }
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
                let children = self.project_terms(id, members.iter(), context, locals)?;
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
                let children = self.project_terms(id, members.iter(), context, locals)?;
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
                    Some(
                        "filament.op.control.let"
                            | "filament.op.control.if"
                            | "filament.op.control.case"
                    )
                );
                let term = has_location_row.then(|| id.clone());
                (kind, children, term)
            }
            _ => {
                return Err(unsupported_operation(
                    id,
                    format!("{}/{}", node.node_tag, node.semantic_form),
                ));
            }
        };
        let type_argument = match &node.body {
            CheckedTerm::Application { operation, .. } => {
                operation.read().ok().and_then(|read| match read.member {
                    Some(filament_contract_model::CheckedOperationMember::TypeArgument {
                        alias: Some(alias),
                        ..
                    }) => Some(alias.node.clone()),
                    _ => None,
                })
            }
            _ => None,
        };
        let value_type = self.value_type(id)?;
        Ok(self.push(ReaderProjectedNode {
            kind,
            value_type,
            location: None,
            children,
            term,
            absorbed: None,
            type_argument,
            occurrence: self.sole_expression_occurrence(id, node),
        }))
    }

    fn union_value(
        &mut self,
        node: &CheckedSemanticNodeV2,
        members: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        // A sealed member can only be interpreted against the original
        // union declaration carried by the selected type projection.
        let ValueType::Composite(union_key) = &self.value_type(&node.semantic_type)? else {
            return Err(fault("reader-union-value-has-declaration-key").into());
        };
        let union_key = union_key.clone();
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
        let [CheckedTerm::Binding { name, value, .. }] = members else {
            return Err(fault("reader-union-value-has-one-member").into());
        };
        let filament_contract_model::CheckedBindingNameKind::Other(member_name) = name.kind()
        else {
            return Err(fault("reader-union-value-has-member-name").into());
        };
        let member = member_position(union_members, member_name)
            .ok_or_else(|| fault("reader-union-value-member-is-declared"))?;
        let CheckedTerm::Aggregate { members } = value.as_ref() else {
            return Err(fault("reader-union-value-has-payload-list").into());
        };
        let children = self.project_terms(&node.node_id, members.iter(), context, locals)?;
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
        let mut values = Vec::with_capacity(members.len());
        for member in members {
            let CheckedTerm::Binding { name, value, .. } = member else {
                return Err(fault("reader-record-value-has-checked-field-binding").into());
            };
            if !matches!(
                name.kind(),
                filament_contract_model::CheckedBindingNameKind::Other(_)
            ) {
                return Err(fault("reader-record-value-has-declared-field-names").into());
            }
            values.push(value.as_ref());
        }
        let slots = self
            .project_terms(id, values.iter().copied(), context, locals)?
            .into_iter()
            .map(RecordSlot::Present)
            .collect::<Vec<_>>();
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
        operation: &filament_contract_model::CheckedApplicationOperation,
        arguments: &[CheckedTerm],
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<(NodeKind<usize, CheckedLiteral>, Vec<usize>), BuildError> {
        let identity = operation.identity();
        match identity {
            Some("filament.op.control.let") => {
                let [CheckedTerm::Binding { name, value, .. }, body] = arguments else {
                    return Err(fault("reader-let-has-binding-and-body").into());
                };
                let filament_contract_model::CheckedBindingNameKind::Other(_) = name.kind() else {
                    return Err(fault("reader-let-binder-has-name").into());
                };
                let slot = self.next_slot(context, locals);
                let binder = self.binder_of(id, name.as_ref(), body, slot, context)?;
                let value = self.project_argument(id, value, context, locals)?;
                let mut body_locals = locals.clone();
                body_locals.extend(binder.map(|binder| (binder, slot)));
                let body = self.project_argument(id, body, context, &body_locals)?;
                self.grow_frame(context, slot.saturating_add(1));
                Ok((
                    NodeKind::Let {
                        slot,
                        name: name.as_ref().to_owned(),
                        value,
                        body,
                    },
                    vec![value, body],
                ))
            }
            Some("filament.op.function.call") => {
                let callee = arguments
                    .first()
                    .ok_or_else(|| fault("reader-call-has-callee"))?;
                let callee = self.callee(callee)?;
                let function = self
                    .function_indexes
                    .get(&callee)
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
            Some(
                "filament.op.boolean.and"
                | "filament.op.boolean.or"
                | "filament.op.boolean.implies",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-connective-has-two-operands").into());
                };
                let connective = match identity {
                    Some("filament.op.boolean.and") => Connective::And,
                    Some("filament.op.boolean.or") => Connective::Or,
                    _ => Connective::Implies,
                };
                Ok((
                    NodeKind::Connective(connective, ConnectiveOrigin::Authored, *left, *right),
                    children,
                ))
            }
            Some("filament.op.boolean.not") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-not-has-one-operand").into());
                };
                Ok((NodeKind::Not(*operand), children))
            }
            Some("filament.op.control.if") => {
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
            Some("filament.op.integer.negate") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-integer-negation-has-one-operand").into());
                };
                Ok((NodeKind::Negate(*operand), children))
            }
            Some(
                "filament.op.integer.div" | "filament.op.integer.rem" | "filament.op.integer.mod",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-integer-division-has-two-operands").into());
                };
                let operator = match identity {
                    Some("filament.op.integer.div") => IntegerDivisionOperator::Div,
                    Some("filament.op.integer.rem") => IntegerDivisionOperator::Rem,
                    Some("filament.op.integer.mod") => IntegerDivisionOperator::Mod,
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
                        .map(|reference| fsl_semantics::value::DefinitionReference {
                            authority: reference.authority.to_string(),
                            identity: reference.identity.to_string(),
                        })
                        .collect::<Vec<_>>();
                    Some(
                        fsl_semantics::value::admit_integer_division(&definitions, None)
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
            Some(
                "filament.op.integer.add" | "filament.op.integer.sub" | "filament.op.integer.mul",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-integer-arithmetic-has-two-operands").into());
                };
                let operator = match identity {
                    Some("filament.op.integer.add") => Arithmetic::Add,
                    Some("filament.op.integer.sub") => Arithmetic::Subtract,
                    _ => Arithmetic::Multiply,
                };
                Ok((NodeKind::Arithmetic(operator, *left, *right), children))
            }
            Some("filament.op.rational.div") => {
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
            Some(
                "filament.op.rational.add"
                | "filament.op.rational.sub"
                | "filament.op.rational.mul",
            ) => {
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
                "filament.op.decimal.add"
                | "filament.op.decimal.sub"
                | "filament.op.decimal.mul"
                | "filament.op.decimal.div",
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
            Some("filament.op.rational.negate") => {
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
            Some("filament.op.decimal.negate") => {
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
            Some("filament.op.state.pre") => {
                // FR-104: `pre(e)` reads `e` at the pre observation; its
                // value is the operand's own.
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-pre-read-has-one-operand").into());
                };
                Ok((NodeKind::Pre(*operand), children))
            }
            Some("filament.op.option.present") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-presence-test-has-one-operand").into());
                };
                Ok((NodeKind::Present(*operand), children))
            }
            Some("filament.op.option.value") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-option-value-has-one-operand").into());
                };
                Ok((NodeKind::Value(*operand), children))
            }
            Some("filament.op.collection.sequence") => {
                let value_type = self.value_type(id)?;
                let ValueType::Collection(collection_type) = &value_type else {
                    return Err(fault("reader-sequence-has-collection-type").into());
                };
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                Ok((
                    NodeKind::Collection {
                        collection_type: (**collection_type).clone(),
                        elements: children.clone(),
                    },
                    children,
                ))
            }
            Some("filament.op.collection.size") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-collection-size-has-one-operand").into());
                };
                Ok((NodeKind::Size(*operand), children))
            }
            Some("filament.op.collection.contains") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [collection, item] = children.as_slice() else {
                    return Err(fault("reader-collection-containment-has-two-operands").into());
                };
                Ok((NodeKind::Contains(*collection, *item), children))
            }
            Some(query)
                if is_sum_identity(query)
                    || matches!(
                        query,
                        "filament.op.collection.map"
                            | "filament.op.collection.filter"
                            | "filament.op.collection.forall"
                            | "filament.op.collection.exists"
                            | "filament.op.collection.count"
                    ) =>
            {
                let [source_term, CheckedTerm::Binding { name, value, .. }] = arguments else {
                    return Err(fault("reader-collection-query-has-source-and-binder").into());
                };
                let filament_contract_model::CheckedBindingNameKind::Other(_) = name.kind() else {
                    return Err(fault("reader-collection-query-binder-has-name").into());
                };
                let slot = self.next_slot(context, locals);
                let binder = self.binder_of(id, name.as_ref(), value, slot, context)?;
                let source = self.project_argument(id, source_term, context, locals)?;
                let mut body_locals = locals.clone();
                body_locals.extend(binder.map(|binder| (binder, slot)));
                let body = self.project_argument(id, value, context, &body_locals)?;
                self.grow_frame(context, slot.saturating_add(1));
                let visit = match identity {
                    Some("filament.op.collection.map") => Visit::Map,
                    Some("filament.op.collection.filter") => Visit::Filter,
                    Some("filament.op.collection.forall") => Visit::Forall,
                    Some("filament.op.collection.exists") => Visit::Exists,
                    Some("filament.op.collection.count") => Visit::Count,
                    Some(sum) if is_sum_identity(sum) => Visit::Sum,
                    _ => return Err(fault("reader-collection-query-identity-is-catalogued").into()),
                };
                Ok((
                    NodeKind::Query {
                        visit,
                        slot,
                        name: name.as_ref().to_owned(),
                        source,
                        body,
                    },
                    vec![source, body],
                ))
            }
            Some("filament.op.collection.seq") => {
                // FR-322 `seq`: two bounds outside the binder, the body inside.
                let [low_term, high_term, CheckedTerm::Binding { name, value, .. }] = arguments
                else {
                    return Err(fault("reader-seq-has-two-bounds-and-binder").into());
                };
                let slot = self.next_slot(context, locals);
                let binder = self.binder_of(id, name.as_ref(), value, slot, context)?;
                let low = self.project_argument(id, low_term, context, locals)?;
                let high = self.project_argument(id, high_term, context, locals)?;
                let mut body_locals = locals.clone();
                body_locals.extend(binder.map(|binder| (binder, slot)));
                let body = self.project_argument(id, value, context, &body_locals)?;
                self.grow_frame(context, slot.saturating_add(1));
                Ok((
                    NodeKind::Seq {
                        slot,
                        name: name.as_ref().to_owned(),
                        low,
                        high,
                        body,
                    },
                    vec![low, high, body],
                ))
            }
            Some("filament.op.collection.convert") => {
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
            Some("filament.op.collection.flatten") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-collection-flatten-has-one-operand").into());
                };
                Ok((NodeKind::Flatten(*operand), children))
            }
            Some(
                "filament.op.collection.sequence.drop_first"
                | "filament.op.collection.ordered_set.drop_first",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [source] = children.as_slice() else {
                    return Err(fault("reader-drop-first-has-one-source").into());
                };
                Ok((NodeKind::DropFirst(*source), children))
            }
            Some("filament.op.collection.remove_one") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [collection, item] = children.as_slice() else {
                    return Err(fault("reader-remove-one-has-collection-and-item").into());
                };
                Ok((NodeKind::RemoveOne(*collection, *item), children))
            }
            Some("filament.op.model.all_instances") => {
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
            Some("filament.op.model.lookup") => {
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
                    "undefined" => filament_semantic_value::absence::AbsenceMode::Undefined,
                    "empty" => filament_semantic_value::absence::AbsenceMode::Empty,
                    "refused" => filament_semantic_value::absence::AbsenceMode::Refused,
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
            Some(
                "filament.op.numeric.convert"
                | "filament.op.ieee.from_exact"
                | "filament.op.ieee.to_float32"
                | "filament.op.ieee.to_float64",
            ) => {
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
                "filament.op.quantity.add"
                | "filament.op.quantity.sub"
                | "filament.op.quantity.mul"
                | "filament.op.quantity.div",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-quantity-arithmetic-has-two-operands").into());
                };
                let operator = match identity {
                    Some("filament.op.quantity.add") => ArithmeticOperator::Add,
                    Some("filament.op.quantity.sub") => ArithmeticOperator::Subtract,
                    Some("filament.op.quantity.mul") => ArithmeticOperator::Multiply,
                    _ => ArithmeticOperator::Divide,
                };
                Ok((NodeKind::Quantity(operator, *left, *right), children))
            }
            Some("filament.op.quantity.pow") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [base, exponent] = children.as_slice() else {
                    return Err(fault("reader-quantity-power-has-two-operands").into());
                };
                Ok((NodeKind::QuantityPower(*base, *exponent), children))
            }
            Some("filament.op.numeric.narrow") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-narrow-has-one-operand").into());
                };
                let ValueType::Int(target) = &self.value_type(id)? else {
                    return Err(fault("reader-narrow-has-exact-integer-target").into());
                };
                Ok((NodeKind::Coerce(*operand, target.clone()), children))
            }
            Some("filament.op.quantity.convert") => {
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
            Some("filament.op.control.case") => self.case(id, arguments, context, locals),
            Some(
                "filament.op.integer.lt"
                | "filament.op.integer.le"
                | "filament.op.integer.gt"
                | "filament.op.integer.ge"
                | "filament.op.rational.lt"
                | "filament.op.rational.le"
                | "filament.op.rational.gt"
                | "filament.op.rational.ge"
                | "filament.op.decimal.lt"
                | "filament.op.decimal.le"
                | "filament.op.decimal.gt"
                | "filament.op.decimal.ge"
                | "filament.op.enum.lt"
                | "filament.op.enum.le"
                | "filament.op.enum.gt"
                | "filament.op.enum.ge"
                | "filament.op.text.lt"
                | "filament.op.text.le"
                | "filament.op.text.gt"
                | "filament.op.text.ge"
                | "filament.op.quantity.lt"
                | "filament.op.quantity.le"
                | "filament.op.quantity.gt"
                | "filament.op.quantity.ge",
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
            Some(
                "filament.op.ieee.numeric_equal"
                | "filament.op.ieee.total_order"
                | "filament.op.ieee.bit_identical",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-ieee-comparison-has-two-operands").into());
                };
                let kind = match identity {
                    Some("filament.op.ieee.numeric_equal") => {
                        NodeKind::IeeeNumericEquality(*left, *right)
                    }
                    Some("filament.op.ieee.total_order") => {
                        NodeKind::IeeeTotalOrder(OrderedKind::Float, *left, *right)
                    }
                    _ => NodeKind::IeeeBitIdentical(*left, *right),
                };
                Ok((kind, children))
            }
            Some("filament.op.ieee.float32.negate" | "filament.op.ieee.float64.negate") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-ieee-negation-has-one-operand").into());
                };
                self.ieee_width_matches(id, identity)?;
                Ok((NodeKind::Negate(*operand), children))
            }
            Some(
                "filament.op.ieee.float32.add"
                | "filament.op.ieee.float32.sub"
                | "filament.op.ieee.float32.mul"
                | "filament.op.ieee.float32.div"
                | "filament.op.ieee.float64.add"
                | "filament.op.ieee.float64.sub"
                | "filament.op.ieee.float64.mul"
                | "filament.op.ieee.float64.div",
            ) => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [left, right] = children.as_slice() else {
                    return Err(fault("reader-ieee-arithmetic-has-two-operands").into());
                };
                let operator = identity
                    .and_then(arithmetic_operator)
                    .ok_or_else(|| fault("reader-ieee-arithmetic-is-catalogued"))?;
                self.ieee_width_matches(id, identity)?;
                let rounding = ieee_rounding(operation)?;
                Ok((NodeKind::Ieee(operator, rounding, *left, *right), children))
            }
            Some("filament.op.ieee.float32.sqrt" | "filament.op.ieee.float64.sqrt") => {
                let children = self.project_arguments(id, Some(arguments), context, locals)?;
                let [operand] = children.as_slice() else {
                    return Err(fault("reader-ieee-square-root-has-one-operand").into());
                };
                self.ieee_width_matches(id, identity)?;
                let rounding = ieee_rounding(operation)?;
                Ok((NodeKind::IeeeSquareRoot(rounding, *operand), children))
            }
            Some("filament.op.record.project") => {
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

    /// A width-specific IEEE identity (`float32.*` or `float64.*`) names the
    /// width of the float its application yields; a mismatch is an admitted
    /// package the source lowering never writes.
    fn ieee_width_matches(
        &self,
        id: &CheckedNodeId,
        identity: Option<&str>,
    ) -> Result<(), BuildError> {
        let named = match identity.and_then(|identity| identity.strip_prefix("filament.op.ieee.")) {
            Some(rest) if rest.starts_with("float32.") => filament_exact::IeeeWidth::Binary32,
            Some(rest) if rest.starts_with("float64.") => filament_exact::IeeeWidth::Binary64,
            _ => return Err(fault("reader-ieee-identity-names-a-width").into()),
        };
        match self.value_type(id)? {
            ValueType::Float(float) if float.width() == named => Ok(()),
            _ => Err(fault("reader-ieee-width-matches-its-operation").into()),
        }
    }

    fn project_arguments(
        &mut self,
        id: &CheckedNodeId,
        arguments: Option<&[CheckedTerm]>,
        context: Context,
        locals: &BTreeMap<CheckedNodeId, usize>,
    ) -> Result<Vec<usize>, BuildError> {
        let arguments = arguments.ok_or_else(|| fault("reader-application-has-operands"))?;
        self.project_terms(id, arguments.iter(), context, locals)
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
            Some("filament.op.quantity.eq" | "filament.op.quantity.ne")
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
            .node(scrutinee_id)
            .and_then(|scrutinee| self.index.node(&scrutinee.semantic_type))
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
            let CheckedTerm::Binding { name, value, .. } = argument else {
                return Err(fault("reader-union-case-arm-is-binding").into());
            };
            let filament_contract_model::CheckedBindingNameKind::Other(member_name) = name.kind()
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
            let mut arm_names = Vec::with_capacity(binders.len());
            for binder in binders {
                let target = reference_target(binder)
                    .ok_or_else(|| fault("reader-union-case-binder-has-identity"))?;
                // FR-400: the declared name is the parameter node's `name`.
                let declared = self
                    .index
                    .node(target)
                    .and_then(|node| parameter_name(&node.body))
                    .ok_or_else(|| fault("reader-union-case-binder-has-name"))?;
                arm_names.push(declared.to_owned());
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
                names: arm_names,
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
    fn binder_of(
        &mut self,
        id: &CheckedNodeId,
        name: &str,
        scope: &CheckedTerm,
        slot: usize,
        context: Context,
    ) -> Result<Option<CheckedNodeId>, BuildError> {
        if let Some(binder) = self.binders.get(&(self.owner, id.clone())) {
            return Ok(binder.clone());
        }
        let binder = self.binder_node(name, scope, slot, context)?;
        self.binders
            .insert((self.owner, id.clone()), binder.clone());
        Ok(binder)
    }

    /// [`Self::binder_of`] without the memo.
    fn binder_node(
        &self,
        name: &str,
        scope: &CheckedTerm,
        slot: usize,
        context: Context,
    ) -> Result<Option<CheckedNodeId>, BuildError> {
        let mut found = BTreeSet::new();
        let lookup = |id: &CheckedNodeId| self.index.node(id);
        crate::introduced_binders::reach_term(&lookup, scope, Some(name), &mut |node| {
            if parameter_name(&node.body) == Some(name) {
                found.insert(node.node_id.clone());
            }
        });
        if crate::introduced_binders::is_ieee_operand(name) {
            // The checker names every operand binder of an IEEE ordering
            // lowering `#ieee_operand` and nests them, so a scope reads the
            // binders of its enclosing lets by that one name. A let binds the
            // operand whose level is the let's own slot (FR-092).
            if found.is_empty() {
                return Ok(None);
            }
            // A function body numbers its levels from its own parameters.
            let offset = if context == Context::Root {
                self.level_offset
            } else {
                0
            };
            let slot = u64::try_from(slot.saturating_add(offset))
                .map_err(|_| fault("reader-let-slot-is-in-range"))?;
            return found
                .into_iter()
                .find(|node| {
                    self.index.node(node).and_then(|n| parameter_level(&n.body)) == Some(slot)
                })
                .map(Some)
                .ok_or_else(|| fault("reader-ieee-operand-binder-is-at-the-let-slot").into());
        }
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
        operation: &filament_contract_model::CheckedApplicationOperation,
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
        let Some(filament_contract_model::CheckedOperationMember::Field {
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
        let object_owner = match selected {
            Some(filament_contract_model::SelectedExactIdentity::Object(owner)) => {
                Some(owner.clone())
            }
            None if self.index.print
                && self
                    .admitted()
                    .model_object_fields(field_declaration)
                    .is_ok() =>
            {
                Some(print_effective_id(&self.index.key(field_declaration)?))
            }
            _ => None,
        };
        if let Some(owner) = object_owner {
            // A model attribute reads the reference under its `model.deref`.
            let operand = self.node(operand_id)?;
            let explicit_deref = matches!(
                read_form,
                Some(filament_contract_model::CheckedFieldReadForm::ExplicitDeref)
            ) && matches!(
                &operand.body,
                CheckedTerm::Application { operation, .. }
                    if operation.identity() == Some("filament.op.model.deref")
            );
            let reference = match &operand.body {
                CheckedTerm::Application {
                    operation,
                    arguments,
                    ..
                } if operation.identity() == Some("filament.op.model.deref") => arguments
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
                Some(filament_contract_model::CheckedMemberType::Option(_))
            );
            let child = self.project_argument(id, &reference, context, locals)?;
            // The explicit `deref` stays a position of the slot's walk (FR-383)
            // with its own row, held by the reference it reads.
            if explicit_deref && context == Context::Root {
                if let Some(node) = self.nodes.get_mut(child) {
                    node.absorbed = Some(operand_id.clone());
                }
            }
            return Ok((
                NodeKind::Attribute {
                    reference: child,
                    field: FieldRef::new(owner.clone(), field_name.to_string()),
                    optional,
                    derefed: matches!(
                        read_form,
                        Some(filament_contract_model::CheckedFieldReadForm::ExplicitDeref)
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
            Some(filament_contract_model::SelectedExactIdentity::Record(key)) => Some(key),
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
                    filament_exact::Presence::Optional
                } else {
                    filament_exact::Presence::Required
                })
            })
            .transpose()?;
        let presence = match selected_presence {
            Some(presence) => presence,
            None => {
                let from_view = [operand_id, field_declaration]
                    .into_iter()
                    .find_map(|node| {
                        let projection = self.index.projections.get(node)?.as_ref().ok()?;
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
        let optional = presence == filament_exact::Presence::Optional;
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
    ) -> Result<Option<filament_exact::Presence>, InternalFault> {
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
                CheckedTerm::Binding { name, value, .. } => {
                    use filament_contract_model::CheckedBindingNameKind as Kind;
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
                filament_exact::Presence::Optional
            } else {
                filament_exact::Presence::Required
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
        (operation.identity() == Some("filament.op.quantity.convert"))
            .then(|| arguments.first().map(|operand| (id, operand)))
            .flatten()
    }

    fn value_type(&self, id: &CheckedNodeId) -> Result<ValueType, BuildError> {
        match node_value_type(&self.index.types(), id) {
            Ok(value_type) => Ok(value_type),
            Err(TypeError::Unsupported) => self
                .index
                .print
                .then(|| self.index.declared_type(id))
                .flatten()
                .ok_or_else(|| BuildError::Unsupported {
                    reason: UnsupportedReason::MissingTypeRepresentation,
                    node: id.clone(),
                }),
            Err(TypeError::Fault(fault)) => Err(BuildError::Fault(fault)),
        }
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
        let value_type = match self
            .index
            .projections
            .get(literal.type_id)
            .ok_or_else(unsupported)?
            .as_ref()
        {
            Ok(projection) => projection.value_type().clone(),
            Err(error)
                if error.form.as_deref().and_then(split_tag_form)
                    == Some((WireTag::ScalarType, WireForm::Text)) =>
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
    pub(crate) identities: &'a filament_contract_model::SelectedExactIdentities,
    pub(crate) nodes: &'a BTreeMap<&'a CheckedNodeId, &'a CheckedSemanticNodeV2>,
    pub(crate) graph: &'a WireGraph,
    pub(crate) projections: &'a ExactProjections,
}

impl TypeReader<'_> {
    fn node(&self, id: &CheckedNodeId) -> Option<&CheckedSemanticNodeV2> {
        self.nodes.get(id).copied()
    }
}

/// The comparison family and operator a `filament.op.<family>.<operator>` order
/// identity names, `None` for any other identity.
#[ix_trace_rs::string_edge]
fn order_of(identity: &str) -> Option<(OrderedKind, OrderingOperator)> {
    let (family, operator) = identity.strip_prefix("filament.op.")?.split_once('.')?;
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
/// structural identities where IR's projection leaves the join to FSL.
#[ix_trace_rs::string_edge]
pub(crate) fn node_value_type(
    index: &TypeReader<'_>,
    id: &CheckedNodeId,
) -> Result<ValueType, TypeError> {
    use filament_contract_model::ExactTypeProjectionErrorCode as Code;
    use filament_contract_model::SelectedExactIdentity as Identity;
    let error = match index
        .projections
        .get(id)
        .ok_or(TypeError::Unsupported)?
        .as_ref()
    {
        Ok(projection) => return Ok(projection.value_type().clone()),
        Err(error) => error,
    };
    if matches!(
        error.code,
        Code::UnsupportedExactType | Code::UnavailableIdentityWitness
    ) && error.form.as_deref().is_some_and(is_collection_bounds)
    {
        return collection_node_type(index, id);
    }
    if index
        .node(id)
        .and_then(|node| index.node(&node.semantic_type))
        .is_some_and(|node| {
            matches!(
                index.admitted.node_kind(&node.node_id),
                Some(filament_contract_model::CheckedNodeKind::CompositeType(
                    filament_contract_model::CompositeTypeForm::Sequence
                        | filament_contract_model::CompositeTypeForm::Set
                        | filament_contract_model::CompositeTypeForm::Bag
                        | filament_contract_model::CompositeTypeForm::OrderedSet
                ))
            )
        })
    {
        return collection_node_type(index, id);
    }
    let unresolved = matches!(
        error.code,
        Code::UnsupportedExactType | Code::UnresolvedSelectedDeclaration
    );
    // IR names the first failing type node inside the requested type. A
    // scalar failure types the requested node only when that node is the
    // scalar itself or has it as its semantic type; a record whose member
    // names the scalar is not that scalar (a `Recovery` whose `receipts` are
    // quantities of a source-owned unit is no quantity).
    let own = index.node(id).map(|node| &node.semantic_type);
    let at = error.at.as_deref();
    let scalar_at = at.filter(|failed| *failed == id || own == Some(failed));
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
    if error.code == Code::UnsupportedExactType
        && error.form.as_deref() == Some("bounded_domain/rational_range")
    {
        let declaration = scalar_at.ok_or(TypeError::Unsupported)?;
        return rational_range_type(index.node(declaration).ok_or(TypeError::Unsupported)?);
    }
    if unresolved && error.form.as_deref() == Some("bounded_domain/float_rounding") {
        // Only the requested node's own float type is typed here; a float
        // nested in an option, collection, tuple, record or map is refused
        // like the sibling scalar forms.
        let declaration = scalar_at.ok_or(TypeError::Unsupported)?;
        return Ok(crate::checked_v2::float_rounding_type(
            index.admitted,
            declaration,
        )?);
    }
    match error.form.as_deref().and_then(split_tag_form) {
        Some((WireTag::ScalarType, WireForm::Text)) if unresolved => {
            index
                .node(scalar_at.ok_or(TypeError::Unsupported)?)
                .filter(|node| has_form(node, WireTag::ScalarType, WireForm::Text))
                .ok_or_else(|| fault("reader-string-type-has-checked-form"))?;
            Ok(ValueType::String)
        }
        Some((WireTag::CompositeType, WireForm::Record)) if unresolved => {
            let declaration = at.ok_or_else(|| fault("reader-record-type-error-has-node"))?;
            let key = match selected(declaration) {
                Some(Identity::Record(key)) => Some(key.clone()),
                _ => structural(declaration, WireTag::CompositeType, WireForm::Record),
            }
            .ok_or_else(|| fault("reader-record-type-has-exact-identity"))?;
            Ok(ValueType::Composite(key))
        }
        Some((WireTag::ScalarType, WireForm::Unit)) if unresolved => {
            let declaration = scalar_at.ok_or(TypeError::Unsupported)?;
            let unit = match selected(declaration) {
                Some(Identity::DeclaredUnit(unit)) => Some(unit.clone()),
                _ => structural(declaration, WireTag::ScalarType, WireForm::Unit)
                    .map(filament_exact::UnitId::declared),
            }
            .ok_or_else(|| fault("reader-unit-has-exact-identity"))?;
            Ok(ValueType::Quantity(unit))
        }
        Some((WireTag::ScalarType, WireForm::CompoundUnit)) if unresolved => {
            let unit_node = scalar_at.ok_or(TypeError::Unsupported)?;
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
                && error.form.as_deref().and_then(split_tag_form)
                    == Some((WireTag::ScalarType, WireForm::Dimension))
            {
                let unit = match selected(semantic_type) {
                    Some(Identity::DeclaredUnit(unit)) => Some(unit.clone()),
                    _ => structural(semantic_type, WireTag::ScalarType, WireForm::Unit)
                        .map(filament_exact::UnitId::declared),
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

/// The `Rational[nmin, nmax; dmin, dmax]` a `rational_range` node holds, its
/// four bounds exact integers.
#[ix_trace_rs::string_edge]
fn rational_range_type(node: &CheckedSemanticNodeV2) -> Result<ValueType, TypeError> {
    let bound = |wanted: &str| -> Option<filament_exact::Integer> {
        let CheckedTerm::Aggregate { members } = &node.body else {
            return None;
        };
        members.iter().find_map(|member| {
            let CheckedTerm::Binding { name, value, .. } = member else {
                return None;
            };
            (name.as_ref() == wanted).then(|| match value.literal_read()?.value {
                filament_contract_model::CheckedLiteralValue::Integer(bound) => {
                    bound.to_string().parse().ok()
                }
                _ => None,
            })?
        })
    };
    let interval = |lower: &str, upper: &str| {
        filament_exact::IntegerInterval::new(bound(lower)?, bound(upper)?).ok()
    };
    let numerator = interval("numerator_min", "numerator_max").ok_or(TypeError::Unsupported)?;
    let denominator =
        interval("denominator_min", "denominator_max").ok_or(TypeError::Unsupported)?;
    filament_exact::RationalDomain::new(numerator, denominator)
        .map(ValueType::Rational)
        .map_err(|_| TypeError::Unsupported)
}

/// The integer a `min` or `max` member of a bounds node holds.
#[ix_trace_rs::string_edge]
fn bound_member(body: &CheckedTerm, wanted: &str) -> Option<u64> {
    let CheckedTerm::Aggregate { members } = body else {
        return None;
    };
    members.iter().find_map(|member| {
        let CheckedTerm::Binding { name, value, .. } = member else {
            return None;
        };
        (name.as_ref() == wanted).then(|| match value.literal_read()?.value {
            filament_contract_model::CheckedLiteralValue::Integer(bound) => {
                bound.to_string().parse::<u64>().ok()
            }
            _ => None,
        })?
    })
}

/// The exact type of a node whose semantic type is a collection, optionally
/// cardinality-bounded. IR's projection reads a collection only as a record member, so
/// the join is FSL's: the bounds node gives the cardinality, the collection
/// type node the kind and its element, and the element is typed by a node of
/// the package that has it as semantic type (IR's own projection of that node).
#[ix_trace_rs::string_edge]
fn collection_node_type(
    index: &TypeReader<'_>,
    id: &CheckedNodeId,
) -> Result<ValueType, TypeError> {
    let semantic_type = index
        .node(id)
        .and_then(|node| index.node(&node.semantic_type))
        .ok_or_else(|| fault("reader-collection-type-node-is-admitted"))?;
    let (collection, bound) = if semantic_type.node_tag.as_ref() == "bounded_domain"
        && semantic_type.semantic_form.as_ref() == "collection_bounds"
    {
        let (Some(minimum), Some(maximum)) = (
            bound_member(&semantic_type.body, "min"),
            bound_member(&semantic_type.body, "max"),
        ) else {
            return Err(fault("reader-collection-bounds-are-integers").into());
        };
        let bound = filament_exact::CardinalityBound::new(minimum, maximum)
            .map_err(|_| fault("reader-collection-bounds-are-ordered"))?;
        let collection = index
            .node(&semantic_type.semantic_type)
            .filter(|node| node.node_tag.as_ref() == "composite_type")
            .ok_or_else(|| fault("reader-collection-type-node-is-admitted"))?;
        (collection, Some(bound))
    } else {
        (semantic_type, None)
    };
    let kind = match collection.semantic_form.as_ref() {
        "sequence" => filament_exact::CollectionKind::Sequence,
        "set" => filament_exact::CollectionKind::Set,
        "bag" => filament_exact::CollectionKind::Bag,
        "ordered_set" => filament_exact::CollectionKind::OrderedSet,
        _ => return Err(TypeError::Unsupported),
    };
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
    Ok(ValueType::collection(filament_exact::CollectionType::new(
        kind, element, bound,
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
        let CheckedTerm::Binding { name, value, .. } = member else {
            return None;
        };
        (name.as_ref() == "name").then(|| match value.literal_read()?.value {
            filament_contract_model::CheckedLiteralValue::Text(name) => Some(name),
            _ => None,
        })?
    })
}

/// The `level` binding of a parameter node's body.
///
/// `#[string_edge]`: `name` is an admitted binding key in the parameter wire
/// form.
#[ix_trace_rs::string_edge]
fn parameter_level(term: &CheckedTerm) -> Option<u64> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    members.iter().find_map(|member| {
        let CheckedTerm::Binding { name, value, .. } = member else {
            return None;
        };
        if name.as_ref() != "level" {
            return None;
        }
        match value.literal_read()?.value {
            filament_contract_model::CheckedLiteralValue::Integer(level) => {
                level.to_string().parse().ok()
            }
            _ => None,
        }
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
            filament_contract_model::CheckedBindingNameKind::Other(name) if name == wanted
        ),
        _ => false,
    })
}

/// A wire location as the evaluator's `Location`; `None` for an ordinal no
/// `usize` holds.
fn location(admitted: &CheckedProtocolLocation) -> Option<Location> {
    let origin = match &admitted.origin {
        filament_contract_model::CheckedProtocolLocationOrigin::TypeDeclaration { name } => {
            Origin::TypeDeclaration {
                name: name.to_string(),
            }
        }
        filament_contract_model::CheckedProtocolLocationOrigin::ProtocolAttempt {
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
            filament_contract_model::CheckedKeyOrdinal::Safe(value) => usize::try_from(*value).ok(),
            filament_contract_model::CheckedKeyOrdinal::BeyondSafe(value) => {
                value.parse::<usize>().ok()
            }
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Location::at(origin, &path))
}

/// Reconstruct through the admitted lending cursor. An exhausted cursor or an
/// unrepresentable coordinate fails the whole function before any projection
/// node receives a location.
fn reconstructed_body_locations(
    admitted: &CheckedPackageV2,
    function: &CheckedNodeId,
    work_limit: u64,
) -> Result<BTreeMap<CheckedNodeId, Vec<Location>>, InternalFault> {
    let mut rows: BTreeMap<CheckedNodeId, Vec<Location>> = BTreeMap::new();
    let mut cursor = admitted
        .body_locations(function, work_limit)
        .map_err(|_| fault("admitted-body-locations-reconstruct"))?;
    let mut previous = Location::root(Origin::Expression);
    while let Some(view) = cursor
        .next_location()
        .map_err(|_| fault("admitted-body-locations-reconstruct"))?
    {
        let location = body_location(&view.row.location, &previous)
            .ok_or_else(|| fault("body-location-coordinates-fit"))?;
        previous = location.clone();
        rows.entry(view.row.application.clone())
            .or_default()
            .push(location);
    }
    Ok(rows)
}

/// This projection predates a caller-owned work meter. Bound reconstruction
/// from the admitted DAG instead: count lending-cursor tasks with memoized
/// subtree costs, retaining multiplicity at shared references. No full paths
/// are rebuilt to establish the allowance. Overflow or a cycle is a fault.
fn body_location_allowance(admitted: &CheckedPackageV2, function: &CheckedNodeId) -> Option<u64> {
    let nodes: BTreeMap<_, _> = admitted
        .resolved_graph()
        .nodes
        .iter()
        .map(|node| (&node.node_id, node))
        .collect();
    let function = nodes.get(function)?;
    let CheckedTerm::Aggregate { members } = &function.body else {
        return None;
    };
    let start = members.iter().find_map(|term| match term {
        CheckedTerm::Binding { name, value, .. }
            if name.kind() == filament_contract_model::CheckedBindingNameKind::Body =>
        {
            Some(value.as_ref())
        }
        _ => None,
    })?;
    let mut pending = Vec::new();
    if let CheckedTerm::Reference { target } = start {
        if target.package.as_ref() == admitted.name() {
            pending.push((target, false));
        }
    }
    let root = pending.first().map(|(id, _)| *id);
    let mut costs: BTreeMap<&CheckedNodeId, Option<u64>> = BTreeMap::new();
    while let Some((id, finish)) = pending.pop() {
        if !finish && costs.contains_key(id) {
            costs.get(id).copied().flatten()?;
            continue;
        }
        let node = nodes.get(id)?;
        let mut terms = vec![&node.body];
        let mut own = 1_u64; // One Node task, then every inline Term task.
        let mut children = Vec::new();
        while let Some(term) = terms.pop() {
            own = own.checked_add(1)?;
            match term {
                CheckedTerm::Reference { target } => {
                    if target.package.as_ref() != admitted.name() {
                        continue;
                    }
                    if let Some(child) = nodes.get(target) {
                        if has_tag(child, WireTag::Expression)
                            || has_form(child, WireTag::Value, WireForm::RecordValue)
                            || has_form(child, WireTag::Value, WireForm::TupleValue)
                        {
                            children.push(target);
                        }
                    }
                }
                CheckedTerm::Application { arguments, .. } => terms.extend(arguments),
                CheckedTerm::Aggregate { members } => terms.extend(members),
                CheckedTerm::Binding { value, .. } => terms.push(value),
                _ => {}
            }
        }
        if finish {
            let total = children.into_iter().try_fold(own, |total, child| {
                total.checked_add(costs.get(child).copied().flatten()?)
            })?;
            costs.insert(id, Some(total));
        } else {
            costs.insert(id, None);
            pending.push((id, true));
            pending.extend(children.into_iter().rev().map(|child| (child, false)));
        }
    }
    let tasks = root.map_or(Some(0), |root| costs.get(root).copied().flatten())?;
    let rows = &admitted.checked_term_data()?.body_locations;
    let mut encoded = 0_u64;
    for row in rows {
        encoded = encoded.checked_add(row.location.up)?;
        encoded = encoded.checked_add(u64::try_from(row.location.down.len()).ok()?)?;
        for ordinal in &row.location.down {
            if let filament_contract_model::CheckedKeyOrdinal::BeyondSafe(text) = ordinal {
                encoded = encoded.checked_add(u64::try_from(text.len()).ok()?)?;
            }
        }
    }
    // Each task pays at most three traversal/push charges. Doubling pending
    // capacity costs less than two tasks per task; six leaves slack for the
    // first allocation. Three encoded units cover deltas and doubled path
    // capacity. Setup pays nodes, all rows twice, and matching rows once.
    tasks
        .checked_mul(6)?
        .checked_add(u64::try_from(nodes.len()).ok()?)?
        .checked_add(u64::try_from(rows.len()).ok()?.checked_mul(3)?)?
        .checked_add(encoded.checked_mul(3)?)?
        .checked_add(16)
}

/// The checker's location of a function-body term, as its row captured it.
fn body_location(admitted: &CheckedBodyLocation, previous: &Location) -> Option<Location> {
    let CheckedBodyLocationOrigin::FunctionBody { name, index, .. } = &admitted.origin;
    let origin = Origin::Body {
        function: name.to_string(),
        index: usize::try_from(*index).ok()?,
    };
    let mut location = previous.clone();
    for _ in 0..admitted.up {
        location = location.parent()?;
    }
    location.origin = origin;
    for ordinal in &admitted.down {
        let index = match ordinal {
            filament_contract_model::CheckedKeyOrdinal::Safe(value) => usize::try_from(*value).ok(),
            filament_contract_model::CheckedKeyOrdinal::BeyondSafe(value) => {
                value.parse::<usize>().ok()
            }
        }?;
        location = location.child(index);
    }
    Some(location)
}

/// The name of a function node: its declared name, or empty for a clause
/// function, which carries a model owner and no declaration. `None` for any
/// other function without a declared name.
fn function_label(node: &CheckedSemanticNodeV2) -> Option<String> {
    match node.declaration.as_ref() {
        Some(declaration) => declaration.qualified_name.last().map(ToString::to_string),
        None => node.owner.is_some().then(String::new),
    }
}

/// How a pure function is selected from a package's nodes.
#[derive(Clone, Copy)]
pub(crate) enum FunctionSelector<'a> {
    /// By its declared, unqualified name.
    Name(&'a str),
    /// By its node's structural key: the only way to select a clause function,
    /// which has no declared name.
    Node(&'a NodeKey),
}

impl FunctionSelector<'_> {
    /// The name a refusal gives the function: the selecting name, or empty
    /// for a clause function.
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::Node(_) => "",
        }
    }
}

fn function_parts(term: &CheckedTerm) -> Option<(&[CheckedTerm], &CheckedTerm)> {
    let CheckedTerm::Aggregate { members } = term else {
        return None;
    };
    let mut parameters = None;
    let mut body = None;
    for member in members {
        let CheckedTerm::Binding { name, value, .. } = member else {
            continue;
        };
        match name.kind() {
            filament_contract_model::CheckedBindingNameKind::Parameters => {
                let CheckedTerm::Aggregate { members } = value.as_ref() else {
                    return None;
                };
                parameters = Some(members.as_slice());
            }
            filament_contract_model::CheckedBindingNameKind::Body => body = Some(value.as_ref()),
            _ => {}
        }
    }
    Some((parameters?, body?))
}

fn literal_value(
    value: filament_contract_model::CheckedLiteralValue<'_>,
    type_id: &CheckedNodeId,
    index: &WireIndex<'_>,
) -> Option<Value> {
    use filament_contract_model::CheckedLiteralValue as Literal;
    Some(match value {
        Literal::Boolean(value) => Value::Boolean(value),
        Literal::Integer(value) => Value::Integer(value.to_string().parse().ok()?),
        Literal::Rational {
            numerator,
            denominator,
        } => {
            let magnitude = filament_exact::Rational::new(
                numerator.to_string().parse().ok()?,
                denominator.to_string().parse().ok()?,
            )
            .ok()?;
            let literal_type = project_literal_type(type_id, index)?;
            match &literal_type {
                ValueType::Quantity(unit) => {
                    Value::Quantity(filament_exact::Quantity::new(magnitude, unit.clone()))
                }
                ValueType::Rational(_) => Value::Rational(magnitude),
                _ => return None,
            }
        }
        Literal::Float32Bits(bits) => Value::Float(filament_exact::IeeeValue::binary32(bits)),
        Literal::Float64Bits(bits) => Value::Float(filament_exact::IeeeValue::binary64(bits)),
        Literal::String(value) => Value::String(value.into()),
        Literal::Text(value) => {
            let checked_type = index.node(type_id)?;
            if has_form(checked_type, WireTag::ScalarType, WireForm::Text) {
                return Some(Value::String(value.into()));
            }
            let type_projection = index.projections.get(type_id)?.as_ref();
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
    match index.projections.get(type_id)?.as_ref() {
        Ok(projection) => Some(projection.value_type().clone()),
        Err(error) => {
            if !matches!(
                error.code,
                filament_contract_model::ExactTypeProjectionErrorCode::UnsupportedExactType
                    | filament_contract_model::ExactTypeProjectionErrorCode::UnresolvedSelectedDeclaration
            ) || !matches!(
                error.form.as_deref().and_then(split_tag_form),
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
            let unit = index
                .identities
                .declaration_rows()
                .find(|row| row.structural_node == *declaration)
                .and_then(|row| match &row.exact_identity {
                    filament_contract_model::SelectedExactIdentity::DeclaredUnit(unit) => {
                        Some(unit.clone())
                    }
                    _ => None,
                })
                .or_else(|| {
                    index
                        .key(declaration)
                        .ok()
                        .map(filament_exact::UnitId::declared)
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
    /// The occurrence of the node at this position, when it has just one.
    occurrence: Option<fsl_foundation::source::provenance::OccurrenceKey>,
    /// The application this slot-walk position is a term of (FR-383).
    term: Option<CheckedNodeId>,
    /// A quantity conversion the walk visits directly above this node,
    /// which the enclosing equality's admitted decision applies itself.
    absorbed: Option<CheckedNodeId>,
    /// The declaration key of the alias the source wrote as this aggregate's
    /// type argument (FSpec FR-322 "Named type arguments").
    type_argument: Option<Box<str>>,
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
            type_argument: None,
            occurrence: None,
        }
    }

    /// The applications this position is visited for, in walk order: an
    /// absorbed conversion, then the node's own term.
    fn terms(&self) -> impl Iterator<Item = &CheckedNodeId> {
        self.absorbed.iter().chain(&self.term)
    }
}

struct ReaderFunction {
    node: CheckedNodeId,
    name: String,
    key: NodeKey,
    body: usize,
    slots: usize,
    parameters: Vec<ReaderProtocolParameter>,
    result_type: Option<ValueType>,
}

fn fault(invariant: &'static str) -> InternalFault {
    InternalFault::new("FR-390", invariant)
}

/// The rounding mode an IEEE operation application declares.
fn ieee_rounding(
    operation: &filament_contract_model::CheckedApplicationOperation,
) -> Result<filament_exact::RoundingMode, BuildError> {
    let mode = operation
        .mode()
        .map_err(|_| fault("reader-ieee-rounding-mode-is-admitted"))?
        .filter(|mode| mode.kind == OperationModeKind::Rounding)
        .ok_or_else(|| fault("reader-ieee-operation-has-rounding-mode"))?;
    filament_exact::RoundingMode::from_code(mode.value)
        .ok_or_else(|| fault("reader-ieee-rounding-mode-is-catalogued").into())
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

/// The effective identity a print walk gives the model object type whose
/// declaration node is `key`: the original declaration key alone.
fn print_effective_id(key: &NodeKey) -> filament_exact::EffectiveId {
    let node = key
        .view()
        .path()
        .filter_map(|segment| match segment {
            filament_exact::PathSegmentView::Text(text) => Some(text),
            filament_exact::PathSegmentView::Node(_) => None,
        })
        .last()
        .unwrap_or_default();
    filament_exact::EffectiveId::new(
        None,
        filament_exact::OriginalDeclarationKey::new(key.package(), node),
    )
}

#[cfg(test)]
mod order_tests {
    use super::*;
    use crate::bare::IdentityWitnessRefusal;
    use filament_exact::PathSegment;
    use ix_trace_rs::trace;

    fn key(name: &str) -> NodeKey {
        NodeKey::new("pkg", vec![PathSegment::Text(name.to_owned())], 0).expect("a key")
    }

    /// Representation admission preserves opaque references and still checks
    /// every later nested composite in the same projected arena.
    #[trace("TC-1333", "FR-533-AC-20")]
    #[test]
    fn a_reference_does_not_mask_an_unrepresented_composite_in_the_same_projection() {
        let types = TypeEnvironment::new(Vec::new(), Vec::new()).expect("an empty closed world");
        let reference = ValueType::Reference(filament_exact::EffectiveId::new(
            None,
            filament_exact::OriginalDeclarationKey::new("model", "Node"),
        ));
        let nested = |value_type| {
            ValueType::option(ValueType::map(
                filament_exact::MapType::new(ValueType::String, value_type, None)
                    .expect("a leaf key"),
            ))
        };
        let mut projection = ReaderProtocolProjection {
            nodes: vec![ReaderProjectedNode::leaf(
                NodeKind::Local(0),
                nested(reference),
            )],
            functions: Vec::new(),
            root: 0,
            selected_function: None,
            selected_function_node: None,
            frame_slots: 2,
            locals: BTreeMap::new(),
        };
        assert!(
            !projection.has_unrepresented_types(&types),
            "a reference's native identity does not require an object declaration in the bare value environment"
        );
        projection.nodes.push(ReaderProjectedNode::leaf(
            NodeKind::Local(1),
            nested(ValueType::Composite(key("missing"))),
        ));
        assert!(
            projection.has_unrepresented_types(&types),
            "accepting the earlier reference must not hide a later unrepresented composite under wrappers"
        );
    }

    /// Real emitted bytes exercise the consumer's lending-cursor boundary;
    /// exhaustion returns a typed fault instead of a partly located function.
    #[trace("FR-531-AC-39", "FSpec-FR-322-AC-510")]
    #[test]
    fn compact_body_projection_reconstructs_paths_and_refuses_exhaustion() {
        let (_, checked) = crate::emit::tests::package_from_text(
            "function f using v(x: Integer): Integer pure { (x + 1) * (x - 2) }\n",
        );
        let emission =
            crate::emit::emit_checked(&checked, crate::checked::EmitLimits::new(1 << 20)).unwrap();
        let package = match filament_contract_model::read_checked_package(
            emission.package().bytes(),
            crate::checked_v2::V2ReadLimits::fixture().for_ir(),
            emission.evidence(),
        ) {
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => panic!("the authentic emitted body must read: {other:?}"),
        };
        let rows = &package.checked_term_data().unwrap().body_locations;
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &rows[0].location.origin;
        let allowance =
            body_location_allowance(&package, function).expect("a finite DAG allowance");
        let locations = reconstructed_body_locations(&package, function, allowance).unwrap();
        let mut paths: Vec<_> = locations.values().flatten().map(Location::path).collect();
        paths.sort();
        assert_eq!(paths, vec![vec![], vec![0], vec![1]]);
        assert!(
            reconstructed_body_locations(&package, function, 0).is_err(),
            "an unpaid cursor must fail the whole projection boundary"
        );
        let mut cursor = package.body_locations(function, allowance).unwrap();
        while cursor.next_location().unwrap().is_some() {}
        let consumed = cursor.work_consumed();
        assert!(consumed <= allowance);
        assert!(
            reconstructed_body_locations(&package, function, consumed - 1).is_err(),
            "late exhaustion must discard every reconstructed partial row"
        );
        assert_eq!(
            reconstructed_body_locations(&package, function, consumed)
                .unwrap()
                .len(),
            locations.len()
        );
    }

    #[trace("FR-531-AC-39", "FSpec-FR-322-AC-510")]
    #[test]
    fn compact_body_projection_refuses_a_captured_coordinate_that_cannot_fit() {
        let (_, checked) = crate::emit::tests::package_from_text(
            "function f using v(x: Integer): Integer pure { x + 1 }\n",
        );
        let emission =
            crate::emit::emit_checked(&checked, crate::checked::EmitLimits::new(1 << 20)).unwrap();
        let mut wire: serde_json::Value =
            serde_json::from_slice(emission.package().bytes()).unwrap();
        wire["checked_term_data"]["body_locations"][0]["location"]["down"] =
            serde_json::json!(["18446744073709551616"]);
        let package = match filament_contract_model::read_checked_package(
            &serde_json::to_vec(&wire).unwrap(),
            crate::checked_v2::V2ReadLimits::fixture().for_ir(),
            emission.evidence(),
        ) {
            filament_contract_model::CheckedPackageDispatchResult::AdmittedV2(package) => package,
            other => {
                panic!("the reader retains arbitrary producer-selected coordinates: {other:?}")
            }
        };
        let row = &package.checked_term_data().unwrap().body_locations[0];
        let CheckedBodyLocationOrigin::FunctionBody { function, .. } = &row.location.origin;
        let allowance = body_location_allowance(&package, function).unwrap();
        assert!(
            reconstructed_body_locations(&package, function, allowance).is_err(),
            "native usize projection must not drop or truncate a captured coordinate"
        );
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

    /// Only a missing type representation can be attributed to a function's
    /// parameter or result type; an unsupported body operation stays generic.
    #[trace("FR-945", "TC-1333")]
    #[test]
    fn unsupported_root_operation_is_not_a_function_result_type() {
        let operation = UnsupportedReason::MissingOperationProjection {
            operation: "future-operation".to_owned(),
        };
        assert_eq!(
            function_type_position(&operation, None, true),
            None,
            "a body-root operation refusal must not carry function-result details"
        );
        assert_eq!(
            function_type_position(&UnsupportedReason::MissingTypeRepresentation, None, true),
            Some(FunctionTypePosition::Result)
        );
        assert_eq!(
            function_type_position(&operation, Some(0), false),
            None,
            "an operation refusal must not be attributed to a parameter type"
        );
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
