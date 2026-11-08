// SPDX-License-Identifier: AGPL-3.0-or-later
//! The front end of the planner (QSL FR-529, FR-530): build a
//! [`PlannerDeclaration`] from the checked package that carries the
//! declaration.
//!
//! The S3 slot types ([`GuaranteeSlot`], [`CompositionSlot`]) become
//! [`DerivedSlot`]s whose [`SlotKey`]s are exactly the item-shaped members
//! `qsl-route` writes into the premise's request item, so the `identity` of
//! a request slot is what [`read_premise_item`](super::read_premise_item)
//! reads back from the item.
//!
//! The request item's `subject` member is the producer's, not the checked
//! package's, so the key spells it from the subject identity text the slot
//! carries ([`subject_text`]); a producer that writes the premise's request
//! item writes the same member.

use qsl_foundation::source::provenance::{OccurrenceKey, SourceRegion};
use qsl_foundation::{Code, Span};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    Capability, CheckedComposition, CheckedImplementationGuarantee, ClauseConstraint, ClauseRef,
    ComponentGuaranteeKey, CompositionRule, CompositionSlot, CompositionSlotKey,
    Correspondence as SemCorrespondence, ExtentClass as SemExtent, GuaranteeSlot, ItemPlacement,
    SlotBacking as SemBacking, SlotKey as SemSlotKey, SlotRole as SemRole, Supplies,
};
use quire_exact::{EffectiveId, NodeKey, Origin, OriginalDeclarationKey, Role};
use quire_semantic_value::fairness::{
    FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation, PropertyForm,
};
use serde_json::{json, Value};

use super::declaration::{
    ClauseFacts, ClauseForm, DerivedSlot, EnvSupplies, EnvironmentMember, ItemRole,
    PlannerDeclaration, SchedulingPremise, SlotBacking, SlotRole,
};
use super::json::canonical_text;
use super::key::{
    ClaimKind, Correspondence, CorrespondenceRole, ExtentClass, FairnessKey, ItemKey, KeyWireError,
    LocalKey, OperationRef, PremiseKey, SlotKey, TemporalItemKind, TemporalKey,
};

/// Why a checked package yields no planner declaration for a node.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DeclarationRefusal {
    /// The node is no implementation guarantee or composition of the
    /// package.
    #[error("the node is no implementation guarantee or composition of the package")]
    NoDeclaration,
    /// The declaration's item is refused: an unbound model element
    /// (`missing_declaration`/`missing-name`) or a dependency cycle with no
    /// declaring profile (`invalid_model_binding`/`malformed-declaration`).
    /// No other item of the package is affected.
    #[error("the item is refused: {code:?}/{cause}")]
    ItemRefused {
        /// The refusal's code.
        code: Code,
        /// The refusal's cause.
        cause: &'static str,
    },
    /// The composition's rule is not settled by the planner; its item is a
    /// request item routed to a composition-capable provider (QSpec
    /// FR-491 "Derived item and slots").
    #[error("the rule `{0}` is not settled by the planner")]
    NotPlannerLocal(&'static str),
    /// The guarantee's abstraction relation is not bound in the package.
    #[error("the guarantee's abstraction relation is not bound in the package")]
    NoRelation,
    /// A slot carries a capability kind no premise item has.
    #[error("a slot carries a capability kind no premise item has")]
    NotAPremiseKind,
    /// A source span lies outside the source it names.
    #[error("a source span is not a region of the package source")]
    SourceRegion,
    /// A key member could not be written.
    #[error(transparent)]
    Key(#[from] KeyWireError),
}

/// The canonical text of the `subject` member a premise's request item
/// carries for the subject `identity` (the S3 slot key's subject text),
/// checked under the peer guarantees `assumptions` (a component-guarantee
/// slot's FR-176 assumption set, part of its slot key). A subject with no
/// assumptions carries no `assumptions` member.
pub fn subject_text(identity: &str, assumptions: &[NodeKey]) -> Result<String, KeyWireError> {
    let mut subject = json!({
        "identity": identity,
        "initial_states": [],
        "universes": [],
        "proof_bounds": [],
    });
    if !assumptions.is_empty() {
        let nodes: Result<Vec<Value>, KeyWireError> = assumptions
            .iter()
            .map(|node| Ok(qsl_foundation::node_wire::node_key_to_json(node)?))
            .collect();
        subject["assumptions"] = Value::Array(nodes?);
    }
    Ok(canonical_text(&subject))
}

impl PlannerDeclaration {
    /// The planner declaration of the implementation guarantee or the
    /// `assume-guarantee` composition at `node` in `package`. The package
    /// identity text is the checked package's own (`CheckedGraph::package_name`),
    /// never a caller's string.
    ///
    /// # Errors
    ///
    /// [`DeclarationRefusal`] when `node` names no such declaration, the
    /// declaration's item is refused, or its rule is not planner-local.
    pub fn from_package(
        package: &CheckedPackage,
        node: &NodeKey,
    ) -> Result<Self, DeclarationRefusal> {
        let graph = package.graph();
        let identity = graph.package_name();
        let regions = Regions {
            source: graph.source(),
        };
        if let Some(guarantee) = graph
            .implementation_guarantees()
            .iter()
            .find(|guarantee| guarantee.node() == node)
        {
            return guarantee_declaration(package, guarantee, identity, &regions);
        }
        if let Some(composition) = graph
            .compositions()
            .iter()
            .find(|composition| composition.node() == node)
        {
            return composition_declaration(composition, identity, &regions);
        }
        Err(DeclarationRefusal::NoDeclaration)
    }
}

/// Turns S3 spans into regions of the package source.
struct Regions<'a> {
    source: &'a qsl_foundation::source::provenance::RawSourceRef,
}

impl Regions<'_> {
    fn region(&self, span: Span) -> Result<SourceRegion, DeclarationRefusal> {
        let (start, end) = (
            u64::try_from(span.start).map_err(|_| DeclarationRefusal::SourceRegion)?,
            u64::try_from(span.end).map_err(|_| DeclarationRefusal::SourceRegion)?,
        );
        SourceRegion::new(self.source.clone(), start, end)
            .map_err(|_| DeclarationRefusal::SourceRegion)
    }
}

fn slot_role(role: SemRole) -> Option<SlotRole> {
    Some(match role {
        SemRole::ModelClaim => SlotRole::ModelClaim,
        SemRole::Initialization => SlotRole::Initialization,
        SemRole::TransitionSimulation => SlotRole::TransitionSimulation,
        SemRole::Frame => SlotRole::Frame,
        SemRole::Environment => SlotRole::Environment,
        SemRole::NoDivergence => SlotRole::NoDivergence,
        SemRole::FairnessCorrespondence => SlotRole::FairnessCorrespondence,
        SemRole::ComponentGuarantee => SlotRole::ComponentGuarantee,
        SemRole::Rely => SlotRole::Rely,
        SemRole::LivenessDependency => SlotRole::LivenessDependency,
        SemRole::Fairness => SlotRole::Fairness,
        SemRole::FixedPoint => SlotRole::FixedPoint,
        SemRole::Guarantee | SemRole::Composition => return None,
    })
}

fn correspondence_role(role: SemRole) -> Option<CorrespondenceRole> {
    Some(match role {
        SemRole::Initialization => CorrespondenceRole::Initialization,
        SemRole::TransitionSimulation => CorrespondenceRole::TransitionSimulation,
        SemRole::Frame => CorrespondenceRole::Frame,
        SemRole::NoDivergence => CorrespondenceRole::NoDivergence,
        _ => return None,
    })
}

fn operation_ref(context: &NodeKey, operation: &str) -> OperationRef {
    OperationRef {
        context: context.clone(),
        operation: operation.to_owned(),
    }
}

fn sorted_fairness(constraints: &[FairnessConstraint]) -> Result<Vec<FairnessKey>, KeyWireError> {
    let mut keys = constraints
        .iter()
        .map(FairnessKey::of)
        .collect::<Result<Vec<_>, _>>()?;
    keys.sort();
    Ok(keys)
}

/// The text of a constraint as a local slot key member.
fn constraint_member(constraint: &FairnessConstraint) -> Result<String, KeyWireError> {
    Ok(canonical_text(&FairnessKey::of(constraint)?.to_value()?))
}

/// The item key of an S3 item-backed slot key, checked under `assumptions`.
fn item_key(key: &SemSlotKey, assumptions: &[NodeKey]) -> Result<ItemKey, DeclarationRefusal> {
    let kind = match key.kind {
        Capability::OperationContract => ClaimKind::OperationContract,
        Capability::TemporalSatisfaction => ClaimKind::TemporalSatisfaction,
        Capability::Composition => ClaimKind::Composition,
        _ => return Err(DeclarationRefusal::NotAPremiseKind),
    };
    let temporal = if kind == ClaimKind::TemporalSatisfaction {
        Some(TemporalKey {
            subject: subject_text(&key.subject, assumptions)?,
            item_kind: TemporalItemKind::Formula,
            fairness: sorted_fairness(&key.fairness)?,
            stutter_closure: key.stutter_closure,
        })
    } else {
        None
    };
    let correspondence = key
        .correspondence
        .as_ref()
        .and_then(|c: &SemCorrespondence| {
            Some(Correspondence {
                role: correspondence_role(c.role)?,
                operation: c
                    .operation
                    .as_ref()
                    .map(|(context, name)| operation_ref(context, name.as_str())),
            })
        });
    Ok(ItemKey {
        claim: OccurrenceKey::new(
            key.claim.clone(),
            Origin::new(Role::new("claim"), u64::from(key.ordinal)),
        ),
        kind,
        temporal,
        correspondence,
        premises: Vec::new(),
        extent: match key.extent {
            SemExtent::Bounded => ExtentClass::Bounded,
            SemExtent::Unbounded => ExtentClass::Unbounded,
        },
    })
}

fn local(role: SlotRole, members: Vec<String>) -> SlotKey {
    SlotKey::Local(LocalKey { role, members })
}

/// The effective identity a supplied scheduling constraint's operation is
/// spelled with: the clause's own constraint on the same operation when it
/// has one, else the context node's.
fn supplied_constraint(
    clause: &[ClauseConstraint],
    kind: FairnessKind,
    granularity: FairnessGranularity,
    context: &NodeKey,
    operation: &str,
) -> FairnessConstraint {
    let declaring = clause
        .iter()
        .find(|entry| entry.context == *context && entry.operation.as_str() == operation)
        .map(|entry| entry.constraint.operation().declaring_type())
        .unwrap_or_else(|| {
            EffectiveId::new(
                None,
                OriginalDeclarationKey::new(context.package(), context.to_string()),
            )
        });
    FairnessConstraint::new(
        kind,
        FairnessOperation::new(declaring, operation.to_owned()),
        granularity,
    )
}

fn guarantee_declaration(
    package: &CheckedPackage,
    guarantee: &CheckedImplementationGuarantee,
    identity: &str,
    regions: &Regions<'_>,
) -> Result<PlannerDeclaration, DeclarationRefusal> {
    if let Some(refusal) = guarantee.item_refusal() {
        return Err(DeclarationRefusal::ItemRefused {
            code: refusal.code(),
            cause: refusal.cause(),
        });
    }
    let relation = package
        .abstraction()
        .ok_or(DeclarationRefusal::NoRelation)?;
    let clause = guarantee.clause();
    let environment = guarantee
        .environment()
        .iter()
        .map(|member| EnvironmentMember {
            name: member.name.as_str().to_owned(),
            supplier: member.supplier.clone(),
            supplies: match &member.supplies {
                Supplies::Fairness {
                    kind,
                    granularity,
                    context,
                    operation,
                } => EnvSupplies::Fairness(SchedulingPremise {
                    constraint: supplied_constraint(
                        &clause.fairness,
                        *kind,
                        *granularity,
                        context,
                        operation.as_str(),
                    ),
                    operation: operation_ref(context, operation.as_str()),
                }),
                Supplies::Assumption(node) => EnvSupplies::Assumption(node.clone()),
            },
        })
        .collect();
    let slots = guarantee
        .slots(identity)
        .iter()
        .map(|slot| guarantee_slot(slot, &clause.fairness, regions))
        .collect::<Result<Vec<_>, _>>()?;
    let spans = guarantee.spans();
    Ok(PlannerDeclaration {
        role: ItemRole::Guarantee,
        node: guarantee.node().clone(),
        package: identity.to_owned(),
        rule: None,
        clause: Some(ClauseFacts {
            node: clause.node.clone(),
            form: if clause.form == PropertyForm::Liveness {
                ClauseForm::Liveness
            } else {
                ClauseForm::Safety
            },
            fairness: clause
                .fairness
                .iter()
                .map(|entry| entry.constraint.clone())
                .collect(),
            fragment_operator: guarantee.fragment_operator(),
            clause_text_span: Some(regions.region(spans.clause)?),
        }),
        relation_sources: guarantee
            .rust_paths(relation)
            .iter()
            .map(|path| path.as_str().to_owned())
            .collect(),
        relation_node: Some(guarantee.relation().clone()),
        environment,
        slots,
        supplies: Vec::new(),
    })
}

fn guarantee_slot(
    slot: &GuaranteeSlot,
    clause: &[ClauseConstraint],
    regions: &Regions<'_>,
) -> Result<DerivedSlot, DeclarationRefusal> {
    let role = slot_role(slot.role).ok_or(DeclarationRefusal::NotAPremiseKind)?;
    // A fairness-correspondence slot names its constraint; its operation is
    // the one the clause's constraint is over.
    let operation = slot
        .operation
        .as_ref()
        .map(|(context, name)| operation_ref(context, name.as_str()))
        .or_else(|| {
            let constraint = slot.constraint.as_ref()?;
            let entry = clause
                .iter()
                .find(|entry| entry.constraint == *constraint)?;
            Some(operation_ref(&entry.context, entry.operation.as_str()))
        });
    let constraint_text = || {
        slot.constraint
            .as_ref()
            .map(constraint_member)
            .transpose()
            .map(Option::unwrap_or_default)
    };
    let operation_text = operation
        .as_ref()
        .map(|operation| operation.operation.clone())
        .unwrap_or_default();
    let (key, backing) = match (slot.backing, role) {
        (SemBacking::Item, _) => {
            let key = slot
                .key
                .as_ref()
                .ok_or(DeclarationRefusal::NotAPremiseKind)?;
            (SlotKey::Item(item_key(key, &[])?), SlotBacking::Item)
        }
        (SemBacking::Environment, _) => {
            let name = slot
                .environment
                .as_ref()
                .map(|name| name.as_str().to_owned())
                .unwrap_or_default();
            (
                local(role, vec![name.clone()]),
                SlotBacking::Environment { member: Some(name) },
            )
        }
        (SemBacking::PlannerLocal, _) => (
            local(role, vec![operation_text, constraint_text()?]),
            SlotBacking::PlannerLocal,
        ),
        (SemBacking::Unfilled, SlotRole::Environment) => (
            local(role, vec![constraint_text()?]),
            SlotBacking::Environment { member: None },
        ),
        (SemBacking::Unfilled, _) => (local(role, vec![operation_text]), SlotBacking::Unfilled),
    };
    Ok(DerivedSlot {
        role,
        key,
        backing,
        constraint: slot.constraint.clone(),
        operation,
        premise_subject: None,
        peer_clause: None,
        cycle: None,
        source: Some(regions.region(slot.source)?),
    })
}

fn composition_declaration(
    composition: &CheckedComposition,
    identity: &str,
    regions: &Regions<'_>,
) -> Result<PlannerDeclaration, DeclarationRefusal> {
    if composition.rule() != CompositionRule::AssumeGuarantee {
        return Err(DeclarationRefusal::NotPlannerLocal(
            composition.rule().as_str(),
        ));
    }
    if let Some(refusal) = composition.item_refusal() {
        return Err(DeclarationRefusal::ItemRefused {
            code: refusal.code(),
            cause: refusal.cause(),
        });
    }
    let item = composition.item();
    debug_assert_eq!(item.placement, ItemPlacement::PlannerLocal);
    let clause_constraints: Vec<ClauseConstraint> = composition
        .clauses()
        .iter()
        .flat_map(|facts| facts.fairness.iter().cloned())
        .collect();
    let environment = composition
        .environment()
        .iter()
        .map(|member| EnvironmentMember {
            name: member.name.as_str().to_owned(),
            supplier: member.supplier.clone(),
            supplies: match &member.supplies {
                Supplies::Fairness {
                    kind,
                    granularity,
                    context,
                    operation,
                } => EnvSupplies::Fairness(SchedulingPremise {
                    constraint: supplied_constraint(
                        &clause_constraints,
                        *kind,
                        *granularity,
                        context,
                        operation.as_str(),
                    ),
                    operation: operation_ref(context, operation.as_str()),
                }),
                Supplies::Assumption(node) => EnvSupplies::Assumption(node.clone()),
            },
        })
        .collect();
    let supplies = composition
        .supplies()
        .iter()
        .map(|supply| {
            supplied_constraint(
                &clause_constraints,
                supply.kind,
                supply.granularity,
                &supply.context,
                supply.operation.as_str(),
            )
        })
        .collect();
    let context = CompositionContext {
        composition,
        regions,
    };
    let slots = item
        .premises
        .iter()
        .map(|slot| context.slot(slot))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PlannerDeclaration {
        role: ItemRole::Composition,
        node: composition.node().clone(),
        package: identity.to_owned(),
        rule: Some(composition.rule().as_str().to_owned()),
        clause: None,
        relation_sources: Vec::new(),
        relation_node: None,
        environment,
        slots,
        supplies,
    })
}

struct CompositionContext<'a> {
    composition: &'a CheckedComposition,
    regions: &'a Regions<'a>,
}

impl CompositionContext<'_> {
    /// The subject identity text of the component `reference` names.
    fn peer_subject(&self, reference: &ClauseRef) -> Result<String, DeclarationRefusal> {
        let component = self
            .composition
            .components()
            .iter()
            .find(|component| component.name == reference.component)
            .ok_or(DeclarationRefusal::NoDeclaration)?;
        Ok(subject_text(&component.subject.to_string(), &[])?)
    }

    /// The index of the accepted cycle whose guarantees include `reference`.
    fn cycle_of(&self, reference: &ClauseRef) -> Option<usize> {
        self.composition
            .cycles()
            .iter()
            .position(|cycle| cycle.guarantees.contains(reference))
    }

    fn guarantee_item(&self, key: &ComponentGuaranteeKey) -> Result<ItemKey, DeclarationRefusal> {
        item_key(&key.key, &key.assumptions)
    }

    fn slot(&self, slot: &CompositionSlot) -> Result<DerivedSlot, DeclarationRefusal> {
        let role = slot_role(slot.role).ok_or(DeclarationRefusal::NotAPremiseKind)?;
        let guarantee_text = slot
            .guarantee
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        let mut derived = DerivedSlot {
            role,
            key: local(role, Vec::new()),
            backing: SlotBacking::Pool,
            constraint: None,
            operation: None,
            premise_subject: None,
            peer_clause: None,
            cycle: None,
            source: Some(self.regions.region(slot.source)?),
        };
        match (slot.role, &slot.key) {
            (SemRole::ComponentGuarantee, Some(CompositionSlotKey::ComponentGuarantee(key))) => {
                derived.key = SlotKey::Item(self.guarantee_item(key)?);
                derived.backing = SlotBacking::Item;
            }
            (SemRole::FixedPoint, Some(CompositionSlotKey::FixedPoint { node, premises, .. })) => {
                let mut keys = Vec::new();
                for premise in premises {
                    keys.push(PremiseKey {
                        role: SlotRole::ComponentGuarantee,
                        key: canonical_text(&self.guarantee_item(premise)?.to_wire()?),
                    });
                }
                derived.key = SlotKey::Item(ItemKey {
                    claim: OccurrenceKey::new(node.clone(), Origin::new(Role::new("claim"), 0)),
                    kind: ClaimKind::Composition,
                    temporal: None,
                    correspondence: None,
                    premises: keys,
                    extent: ExtentClass::Unbounded,
                });
                derived.backing = SlotBacking::Item;
                derived.cycle = self.composition.cycles().iter().position(|cycle| {
                    premises.len() == cycle.guarantees.len()
                        && premises
                            .iter()
                            .zip(&cycle.guarantees)
                            .all(|(key, reference)| key.key.node == reference.node)
                });
            }
            (SemRole::Rely | SemRole::LivenessDependency, _) => {
                let (Some(guarantee), Some(on)) = (&slot.guarantee, &slot.on) else {
                    return Err(DeclarationRefusal::NotAPremiseKind);
                };
                derived.key = local(role, vec![guarantee.to_string(), on.to_string()]);
                derived.premise_subject = Some(self.peer_subject(on)?);
                derived.peer_clause = Some(on.node.clone());
                derived.cycle = self.cycle_of(on);
            }
            (SemRole::Fairness, _) => {
                let constraint = slot
                    .constraint
                    .clone()
                    .ok_or(DeclarationRefusal::NotAPremiseKind)?;
                derived.key = local(role, vec![guarantee_text, constraint_member(&constraint)?]);
                derived.constraint = Some(constraint);
            }
            (SemRole::Environment, _) => {
                let name = slot
                    .environment
                    .as_ref()
                    .map(|name| name.as_str().to_owned())
                    .unwrap_or_default();
                derived.key = local(role, vec![name.clone()]);
                derived.backing = SlotBacking::Environment { member: Some(name) };
            }
            _ => return Err(DeclarationRefusal::NotAPremiseKind),
        }
        Ok(derived)
    }
}
