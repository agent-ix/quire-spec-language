// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531: the evaluation entry over a checked package read from its bytes
//! alone.
//!
//! A [`BarePackage`] borrows the [`CheckedPackageV2`] that Contract IR's
//! `read_checked_package` returned. It has no other constructor, so no value
//! of the source side (`CheckedPackage`, `Emission`, `AdmittedPackageRead`)
//! can reach the evaluation code that takes it. The identity rows a protocol
//! slot call needs come from the handoff directory's `identities.json`
//! through [`ReadIdentityRows`], and the dependency closure from the direct
//! dependencies the reader retained in the package
//! ([`BarePackage::dependencies`]).

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    CheckedNodeId, CheckedPackageV2, ExactTypeProjection, ExactTypeProjectionError,
    NativeJoinBudget, SelectedExactDeclarationForm, SelectedExactDeclarationRow,
    SelectedExactFieldRow, SelectedExactIdentities, SelectedExactIdentity, SelectedSourceRecordRow,
    SelectedSourceUnionRow,
};
use quire_exact::{EffectiveId, MemberId, MemberOwner, NodeKey, OriginalDeclarationKey, UnitId};
use serde::{Deserialize, Serialize};

use crate::checked_v2::wire_graph;

mod carried;
mod observation;
use carried::{SourceRecordJson, SourceUnionJson};

/// The format of `identities.json` (FR-531 "The identity file").
pub const IDENTITIES_FORMAT: &str = "quire.checked-handoff.identities/3";

/// A checked package as a bytes-only consumer holds it (FR-531).
///
/// `BarePackage::new` is the one constructor; nothing builds one from bytes,
/// from a decoded graph or from a term (FR-531-AC-2):
///
/// ```compile_fail,E0599
/// let _ = qsl_package::BarePackage::from_bytes(b"{}");
/// ```
///
/// ```compile_fail,E0599
/// fn forge(graph: quire_contract_model::CheckedPackageV2) {
///     let _ = qsl_package::BarePackage::from_graph(&graph);
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn forge(term: &quire_contract_model::CheckedTerm) {
///     let _ = qsl_package::BarePackage::from_term(term);
/// }
/// ```
///
/// The package joins its supply's identity rows once, at
/// [`Self::with_supply`], under the caller's one native budget (FR-531,
/// FR-358). It owns the joined identities and every later selection,
/// world and projection call borrows them, so it is not `Copy` and not
/// `Clone`: an unbudgeted copy of the joined identities cannot be made.
#[derive(Debug)]
pub struct BarePackage<'p> {
    package: &'p CheckedPackageV2,
    supply: Option<BareSupply<'p>>,
    /// The supply's join, held with the witness defect it found: a missing,
    /// foreign or unknown witness is refused by the stage that reads the
    /// identities, not when the supply is attached (FR-390).
    joined: Option<Result<SelectedExactIdentities, IdentityWitnessRefusal>>,
    projections: BTreeMap<CheckedNodeId, Result<ExactTypeProjection, ExactTypeProjectionError>>,
    equality: crate::reader_protocol_projection::EqualityDecisions,
    /// The source-owned units' definitions the supply's equality rows select
    /// (FR-533), empty when the rows select none.
    pub(crate) units: quire_semantic_value::quantity::UnitTable,
    /// The identity an observation this package admits carries (FR-533
    /// "Observation admission"); one per value, never copied.
    owner: std::sync::Arc<()>,
    /// The selected object declarations the bare environment declares, or
    /// the refusal of their projection, retained for admission (FR-533).
    objects: Result<Vec<quire_semantic_value::declaration::ObjectTypeDeclaration>, ()>,
    /// The population catalog: the declaration and member types of each
    /// selected population node, or the refusal of the read.
    populations: Result<BTreeMap<NodeKey, PopulationTypes>, ()>,
}

/// The declaration and member types of one selected population.
pub(crate) type PopulationTypes = (quire_checked::key::DeclarationKey, Vec<EffectiveId>);

/// What a protocol-slot call reads beside the package (FR-531).
#[derive(Clone, Copy, Debug)]
pub struct BareSupply<'p> {
    /// The selected model identity rows of the package.
    pub identities: &'p ReadIdentityRows,
    /// The producer's equality rows, read from `equality.json`; `None` for
    /// a package whose slots hold no equality application (FR-531).
    pub equality: Option<&'p crate::equality_file::ReadEqualityRows>,
}

impl<'p> BarePackage<'p> {
    /// The bare package over `package`, with no supply.
    pub fn new(package: &'p CheckedPackageV2) -> Self {
        Self {
            package,
            supply: None,
            joined: None,
            projections: BTreeMap::new(),
            equality: BTreeMap::new(),
            units: quire_semantic_value::quantity::UnitTable::default(),
            owner: std::sync::Arc::new(()),
            objects: Ok(Vec::new()),
            populations: Ok(BTreeMap::new()),
        }
    }

    /// The identity of this value as an observation owner.
    pub(crate) fn owner(&self) -> std::sync::Arc<()> {
        std::sync::Arc::clone(&self.owner)
    }

    /// The model identities the package selects, ascending (FR-533).
    pub fn model_selections(&self) -> Vec<qsl_semantics::model::domain_package::DomainPackageRef> {
        self.package
            .selected_model_packages()
            .map(
                |identity| qsl_semantics::model::domain_package::DomainPackageRef {
                    identity: identity.to_owned(),
                },
            )
            .collect()
    }

    /// The declaration and member types of the selected population `node`,
    /// from the package's population catalog; `None` for a node no selected population has.
    pub fn population_types(
        &self,
        node: &NodeKey,
    ) -> Option<(&quire_checked::key::DeclarationKey, &[EffectiveId])> {
        let (declaration, members) = self.populations.as_ref().ok()?.get(node)?;
        Some((declaration, members.as_slice()))
    }

    /// The selected object declarations, or `Err` when their projection or
    /// the catalog was refused (FR-533 "Observation admission", step 3).
    pub(crate) fn object_declarations(
        &self,
    ) -> Result<&[quire_semantic_value::declaration::ObjectTypeDeclaration], ()> {
        self.populations.as_ref().map_err(|_| ())?;
        self.objects.as_deref().map_err(|_| ())
    }

    /// `identities` extended with the source-owned units the equality rows
    /// select (FR-533): IR joins the rows, then the units' definitions are
    /// read from the package's nodes. Rows IR does not join, or definitions
    /// that are no unit graph, are an unknown identity witness; an exhausted
    /// native allowance is the caller's, never a defect of the rows.
    fn join_source_units(
        &self,
        identities: SelectedExactIdentities,
        equality: Option<&crate::equality_file::ReadEqualityRows>,
        budget: &mut NativeJoinBudget,
    ) -> Result<
        (
            Result<SelectedExactIdentities, IdentityWitnessRefusal>,
            quire_semantic_value::quantity::UnitTable,
        ),
        crate::NativeJoinFailure,
    > {
        let none = quire_semantic_value::quantity::UnitTable::default();
        // Another package's rows are the equality stage's refusal to report.
        let Some(equality) = equality.filter(|equality| {
            equality.package_name() == self.package.name() && !equality.source_units().is_empty()
        }) else {
            return Ok((Ok(identities), none));
        };
        let extended = match identities.with_admitted_type_rows(
            self.package,
            &[],
            equality.source_units(),
            budget,
        ) {
            Ok(extended) => extended,
            Err(denial) if qsl_semantics::model::intake::is_native_budget_denial(&denial) => {
                return Err(crate::NativeJoinFailure::from(denial).with_budget(budget))
            }
            Err(_) => return Ok((Err(IdentityWitnessRefusal::Unknown), none)),
        };
        match crate::bare_units::read_unit_table(self.package, equality.source_units(), budget) {
            Ok(units) => Ok((Ok(extended), units)),
            Err(crate::bare_units::UnitDefinitionRefusal::Read(
                quire_contract_model::nominal_reads::NominalReadCode::LimitReached,
            )) => {
                // A native read that reports its allowance exhausted records the
                // unpaid charge; without one the denial is no allowance of this
                // pass, and the witness is refused rather than given a setting.
                let setting = match budget.first_unpaid().map(|charge| charge.kind) {
                    Some(quire_contract_model::NativeJoinChargeKind::Work) => {
                        qsl_foundation::Setting::I2WorkUnits
                    }
                    Some(quire_contract_model::NativeJoinChargeKind::Storage) => {
                        qsl_foundation::Setting::I2NativeJoinStorage
                    }
                    None => return Ok((Err(IdentityWitnessRefusal::Unknown), none)),
                };
                Err(crate::NativeJoinFailure::UnitDefinitions { setting }.with_budget(budget))
            }
            Err(_) => Ok((Err(IdentityWitnessRefusal::Unknown), none)),
        }
    }

    /// The bare package with `supply`, its identity rows joined once and
    /// charged to `budget`, the caller's one finite native work and storage
    /// allowance for the pass (FR-531, FR-358). A missing, foreign or
    /// structurally unknown witness is not refused here: it stays deferred to
    /// the stage that reads the identities, in FR-390's order.
    ///
    /// # Errors
    ///
    /// [`crate::NativeJoinFailure`] retains IR's `ExhaustedWork` or
    /// `ExhaustedStorage` for identity/type projection, or its `WorkLimit` or
    /// `StorageLimit` for equality admission. No package is returned when
    /// `budget` runs out; the denial is never an unknown witness, a fault or
    /// an empty identity set. Resource denials carry a
    /// [`crate::NativeJoinFailure::Limit`] diagnostic with the original owner
    /// error, caller-configured ceiling, setting and cumulative attempted charge.
    pub fn with_supply(
        mut self,
        supply: BareSupply<'p>,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, crate::NativeJoinFailure> {
        self.joined = match supply.identities.selected(self.package, budget) {
            Ok(identities) => {
                let (joined, units) =
                    self.join_source_units(identities, supply.equality, budget)?;
                self.units = units;
                Some(joined)
            }
            Err(IdentityJoinFailure::Witness(defect)) => Some(Err(defect)),
            Err(IdentityJoinFailure::Exhausted(denial)) => {
                return Err(crate::NativeJoinFailure::from(*denial).with_budget(budget))
            }
        };
        self.supply = Some(supply);
        if let Some(Ok(identities)) = &self.joined {
            for node in &self.package.graph().nodes {
                let projection = self
                    .package
                    .project_exact_type(&node.node_id, identities, budget);
                if let Err(denial) = &projection {
                    if qsl_semantics::model::intake::is_native_budget_denial(denial) {
                        return Err(
                            crate::NativeJoinFailure::from(denial.clone()).with_budget(budget)
                        );
                    }
                }
                self.projections.insert(node.node_id.clone(), projection);
            }
        }
        if self.identities().is_some() {
            self.equality = crate::reader_protocol_projection::admit_equality_decisions(
                self.package,
                crate::reader_protocol_projection::ProjectionSource::Bytes {
                    equality: self.supply.as_ref().and_then(|supply| supply.equality),
                },
                budget,
            )
            .map_err(|failure| failure.with_budget(budget))?;
            self.read_selected_objects(budget)?;
            self.read_population_catalog(budget)?;
        }
        Ok(self)
    }

    /// The selected objects, projected once under `budget` (FR-533
    /// "Observation admission"). A budget denial is the caller's; any other
    /// refusal is retained for admission.
    fn read_selected_objects(
        &mut self,
        budget: &mut NativeJoinBudget,
    ) -> Result<(), crate::NativeJoinFailure> {
        let Some(Ok(identities)) = &self.joined else {
            return Ok(());
        };
        let source_units = self
            .supply
            .as_ref()
            .and_then(|supply| supply.equality)
            .map(|equality| equality.source_units())
            .unwrap_or(&[]);
        match self
            .package
            .project_selected_type_declarations(identities, &[], source_units, budget)
        {
            Ok(declarations) => {
                self.objects = Ok(declarations
                    .objects()
                    .iter()
                    .map(|object| {
                        let attributes = object
                            .declared_fields()
                            .iter()
                            .map(|field| {
                                quire_semantic_value::declaration::FieldDeclaration::new(
                                    field.name(),
                                    field.value_type().clone(),
                                    field.presence(),
                                )
                            })
                            .collect();
                        quire_semantic_value::declaration::ObjectTypeDeclaration::new(
                            object.effective_id().clone(),
                            object.original_identity(),
                            attributes,
                        )
                        .with_supertypes(object.supertypes().to_vec())
                    })
                    .collect());
                Ok(())
            }
            Err(refusal) => {
                let refusal = quire_contract_model::ExactTypeProjectionError {
                    code: refusal.code,
                    requested: None,
                    at: refusal.at,
                    form: refusal.form,
                    work: refusal.work,
                    denied: refusal.denied,
                    configured_bound: None,
                };
                if qsl_semantics::model::intake::is_native_budget_denial(&refusal) {
                    return Err(crate::NativeJoinFailure::from(refusal).with_budget(budget));
                }
                self.objects = Err(());
                Ok(())
            }
        }
    }

    /// The population catalog, read once with the budget's unconsumed work
    /// as its work allowance and its unconsumed storage as its entries
    /// allowance; its charges are not added to the budget (FR-533).
    fn read_population_catalog(
        &mut self,
        budget: &NativeJoinBudget,
    ) -> Result<(), crate::NativeJoinFailure> {
        use quire_contract_model::{
            CheckedModelPopulationCatalogError as CatalogError,
            CheckedModelPopulationCatalogLimitKind as Kind,
            CheckedModelPopulationCatalogLimits as Limits,
        };
        let limits = Limits {
            work: budget.work_limit().saturating_sub(budget.consumed().work),
            entries: budget
                .storage_limit()
                .saturating_sub(budget.consumed().storage),
        };
        let catalog = match self.package.selected_model_population_catalog(limits) {
            Ok(catalog) => catalog,
            Err(CatalogError::Limit {
                kind,
                limit,
                consumed,
                ..
            }) => {
                let setting = match kind {
                    Kind::Work => qsl_foundation::Setting::I2WorkUnits,
                    Kind::Entries => qsl_foundation::Setting::I2NativeJoinStorage,
                };
                return Err(crate::NativeJoinFailure::PopulationCatalog {
                    setting,
                    limit: qsl_foundation::diagnostic::LimitExceeded::new(
                        setting,
                        limit,
                        u128::from(consumed),
                    ),
                });
            }
            Err(_) => {
                self.populations = Err(());
                return Ok(());
            }
        };
        let mut populations = BTreeMap::new();
        for root in catalog.roots() {
            for population in root.populations() {
                let declaration = quire_checked::key::DeclarationKey {
                    package: root.model_identity().to_owned(),
                    node: population.original_identity().to_owned(),
                };
                let Ok(node) = qsl_semantics::model::key::population_node_key(&declaration) else {
                    self.populations = Err(());
                    return Ok(());
                };
                let members = population
                    .members()
                    .iter()
                    .map(|member| member.normalized_type_identity().clone())
                    .collect();
                populations.insert(node, (declaration, members));
            }
        }
        self.populations = Ok(populations);
        Ok(())
    }

    pub(crate) fn equality_decisions(
        &self,
    ) -> &crate::reader_protocol_projection::EqualityDecisions {
        &self.equality
    }

    /// Owner-projected Exact types retained under the supply entry's native budget.
    /// Unsupported types retain their original owner refusal for the reading stage.
    pub fn exact_type_projections(
        &self,
    ) -> &BTreeMap<CheckedNodeId, Result<ExactTypeProjection, ExactTypeProjectionError>> {
        &self.projections
    }

    /// The supply with the identities it joined, or the witness refusal
    /// FR-531-AC-5 names.
    pub(crate) fn witness(
        &self,
    ) -> Result<(&BareSupply<'p>, &SelectedExactIdentities), crate::BareSelectionFailure> {
        let supply = self
            .supply
            .as_ref()
            .ok_or(crate::BareSelectionFailure::Identity(
                IdentityWitnessRefusal::Missing,
            ))?;
        match &self.joined {
            Some(Ok(identities)) => Ok((supply, identities)),
            Some(Err(defect)) => Err(crate::BareSelectionFailure::Identity(*defect)),
            None => Err(crate::BareSelectionFailure::Identity(
                IdentityWitnessRefusal::Missing,
            )),
        }
    }

    /// The joined identities, when the supply's witness was admitted.
    pub fn identities(&self) -> Option<&SelectedExactIdentities> {
        self.joined.as_ref().and_then(|joined| joined.as_ref().ok())
    }

    /// The package.
    pub fn package(&self) -> &'p CheckedPackageV2 {
        self.package
    }

    /// The supply, when one was given.
    pub fn supply(&self) -> Option<&BareSupply<'p>> {
        self.supply.as_ref()
    }

    /// The package's dependency closure, read from the direct dependencies
    /// each package retains from its read evidence (IR-1325): every
    /// dependency once, nearest first, in lock-selection order. A package
    /// two parents retain is one retained value and is listed once; two
    /// distinct values that share a name are both listed. The root package is
    /// not listed.
    pub fn dependencies(&self) -> Vec<&'p CheckedPackageV2> {
        let mut closure: Vec<&'p CheckedPackageV2> = Vec::new();
        let mut queue: std::collections::VecDeque<&'p CheckedPackageV2> =
            self.package.direct_dependencies().collect();
        while let Some(next) = queue.pop_front() {
            if closure.iter().any(|held| std::ptr::eq(*held, next)) {
                continue;
            }
            closure.push(next);
            queue.extend(next.direct_dependencies());
        }
        closure
    }
}

/// The selected model identity rows of one package, read from its
/// `identities.json` (FR-390, FR-531).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReadIdentityRows {
    package: String,
    declarations: Vec<SelectedExactDeclarationRow>,
    fields: Vec<SelectedExactFieldRow>,
    source_records: Vec<SelectedSourceRecordRow>,
    source_unions: Vec<SelectedSourceUnionRow>,
}

/// Why an identity file was not read: a closed set, naming what it found.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityFileRefusal {
    /// Not JSON, not this shape, or an unknown or repeated member.
    #[error("the identity file is malformed at {member}")]
    Malformed {
        /// The member the parse stopped at.
        member: IdentityMember,
    },
    /// Another `format` string.
    #[error("the identity file has format `{0}`")]
    Format(String),
    /// A row the package does not join.
    #[error("the identity file row {index} of {member} is not a node of the package")]
    Unjoined {
        /// `declarations` or `fields`.
        member: IdentityMember,
        /// The first row that does not join.
        index: usize,
    },
    /// IR refused the original producer rows at its typed structural locus.
    #[error("the identity file has an invalid native witness: {0:?}")]
    Native(Box<quire_contract_model::ExactTypeProjectionError>),
}

/// The member of an identity file a refusal names (FR-531 "The identity file").
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityMember {
    /// The file as a whole: not JSON, or not an object.
    #[error("the file")]
    File,
    /// The `format` member.
    #[error("`format`")]
    Format,
    /// A well-shaped but unsupported format string.
    #[error("unsupported format `{0}`")]
    Version(String),
    /// The `declarations` member.
    #[error("`declarations`")]
    Declarations,
    /// The `fields` member.
    #[error("`fields`")]
    Fields,
    /// The required final-emission source-record rows.
    #[error("`source_records`")]
    SourceRecords,
    /// The required final-emission source-union rows.
    #[error("`source_unions`")]
    SourceUnions,
    /// A declaration row's `form`.
    #[error("`form`")]
    Form,
    /// A declaration row's `exact_identity`.
    #[error("`exact_identity`")]
    ExactIdentity,
    /// A field row's `member_id`.
    #[error("`member_id`")]
    MemberId,
    /// A member the file does not define.
    #[error("unknown member `{0}`")]
    Unknown(String),
    /// A member written twice.
    #[error("repeated member `{0}`")]
    Repeated(&'static str),
}

#[derive(Serialize)]
struct File {
    format: String,
    declarations: Vec<DeclarationJson>,
    fields: Vec<FieldJson>,
    source_records: Vec<SourceRecordJson>,
    source_unions: Vec<SourceUnionJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct DeclarationJson {
    structural_node: CheckedNodeId,
    selected_document: String,
    form: String,
    original_identity: String,
    exact_identity: IdentityJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct IdentityJson {
    kind: String,
    value: serde_json::Value,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FieldJson {
    record_node: CheckedNodeId,
    selected_document: String,
    record_original_identity: String,
    field_original_identity: String,
    member_id: MemberJson,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct MemberJson {
    owner: OwnerJson,
    member: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OwnerJson {
    kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    node: Option<CheckedNodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    effective: Option<EffectiveJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EffectiveJson {
    chain: Vec<LinkJson>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LinkJson {
    package: String,
    node: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UnitJson {
    kind: String,
    node: CheckedNodeId,
}

struct TopVisitor<'a> {
    problem: &'a std::cell::Cell<Option<IdentityMember>>,
}

impl TopVisitor<'_> {
    fn fail<E: serde::de::Error>(&self, member: IdentityMember) -> E {
        self.problem.set(Some(member));
        E::custom("malformed identity file")
    }
}

impl<'de> serde::de::Visitor<'de> for TopVisitor<'_> {
    type Value = File;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an identity file object")
    }

    #[ix_trace_rs::string_edge]
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<File, A::Error> {
        let mut format: Option<String> = None;
        let mut declarations: Option<Vec<DeclarationJson>> = None;
        let mut fields: Option<Vec<FieldJson>> = None;
        let mut source_records: Option<Vec<SourceRecordJson>> = None;
        let mut source_unions: Option<Vec<SourceUnionJson>> = None;
        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "format" => {
                    if format.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("format")));
                    }
                    let value: String = map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::Format));
                    })?;
                    if !is_identities_format(&value) {
                        return Err(self.fail(IdentityMember::Version(value)));
                    }
                    format = Some(value);
                }
                "declarations" => {
                    if declarations.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("declarations")));
                    }
                    declarations = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::Declarations));
                    })?);
                }
                "fields" => {
                    if fields.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("fields")));
                    }
                    fields = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::Fields));
                    })?);
                }
                "source_records" => {
                    if source_records.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_records")));
                    }
                    source_records = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceRecords));
                    })?);
                }
                "source_unions" => {
                    if source_unions.is_some() {
                        return Err(self.fail(IdentityMember::Repeated("source_unions")));
                    }
                    source_unions = Some(map.next_value().inspect_err(|_: &A::Error| {
                        self.problem.set(Some(IdentityMember::SourceUnions));
                    })?);
                }
                other => return Err(self.fail(IdentityMember::Unknown(other.to_owned()))),
            }
        }
        match (format, declarations, fields, source_records, source_unions) {
            (
                Some(format),
                Some(declarations),
                Some(fields),
                Some(source_records),
                Some(source_unions),
            ) => Ok(File {
                format,
                declarations,
                fields,
                source_records,
                source_unions,
            }),
            (None, _, _, _, _) => Err(self.fail(IdentityMember::Format)),
            (_, None, _, _, _) => Err(self.fail(IdentityMember::Declarations)),
            (_, _, None, _, _) => Err(self.fail(IdentityMember::Fields)),
            (_, _, _, None, _) => Err(self.fail(IdentityMember::SourceRecords)),
            (_, _, _, _, None) => Err(self.fail(IdentityMember::SourceUnions)),
        }
    }
}

fn parse_file(bytes: &[u8]) -> Result<File, IdentityMember> {
    let problem = std::cell::Cell::new(None);
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    serde::Deserializer::deserialize_map(&mut deserializer, TopVisitor { problem: &problem })
        .and_then(|file| deserializer.end().map(|()| file))
        .map_err(|_| problem.take().unwrap_or(IdentityMember::File))
}

fn form_spelling(form: SelectedExactDeclarationForm) -> &'static str {
    match form {
        SelectedExactDeclarationForm::RecordValueType => "record_value_type",
        SelectedExactDeclarationForm::ObjectType => "object_type",
        SelectedExactDeclarationForm::Union => "union",
        SelectedExactDeclarationForm::DeclaredUnit => "declared_unit",
    }
}

/// Whether `format` spells the identities file format this reader admits.
#[ix_trace_rs::string_edge]
fn is_identities_format(format: &str) -> bool {
    format == IDENTITIES_FORMAT
}

/// Whether `kind` spells a declared unit identity.
#[ix_trace_rs::string_edge]
fn is_declared_unit_kind(kind: &str) -> bool {
    kind == "declared"
}

#[ix_trace_rs::string_edge]
fn form_of(spelling: &str) -> Option<SelectedExactDeclarationForm> {
    Some(match spelling {
        "record_value_type" => SelectedExactDeclarationForm::RecordValueType,
        "object_type" => SelectedExactDeclarationForm::ObjectType,
        "union" => SelectedExactDeclarationForm::Union,
        "declared_unit" => SelectedExactDeclarationForm::DeclaredUnit,
        _ => return None,
    })
}

/// The package's nodes by structural key, for writing a key as the node
/// reference the wire spells it with.
fn node_references(package: &CheckedPackageV2) -> Option<BTreeMap<NodeKey, CheckedNodeId>> {
    let graph = wire_graph(package).ok()?;
    package
        .graph()
        .nodes
        .iter()
        .map(|node| Some((graph.key(&node.node_id).ok()?, node.node_id.clone())))
        .collect()
}

/// Why identity rows are no witness for a slot call (FR-531-AC-5).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityWitnessRefusal {
    /// The bare package has no supply.
    #[error("the bare package holds no identity rows")]
    Missing,
    /// The rows name no identity the package admits.
    #[error("the identity rows name no admitted identity")]
    Unknown,
    /// The rows were produced for another package.
    #[error("the identity rows belong to another package")]
    Foreign,
}

/// Why the join of identity rows gave no identities: a defect of the witness
/// or the caller's native budget running out (FR-358).
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum IdentityJoinFailure {
    /// The rows are no witness for the package.
    #[error(transparent)]
    Witness(#[from] IdentityWitnessRefusal),
    /// The caller's native work or storage allowance was consumed.
    #[error("the native join budget was consumed: {0}")]
    Exhausted(Box<ExactTypeProjectionError>),
}

impl ReadIdentityRows {
    /// The identity witness these rows give `package`, joined by IR's
    /// `SelectedExactIdentities` projection and charged to `budget`.
    ///
    /// # Errors
    ///
    /// [`IdentityWitnessRefusal::Foreign`] for rows of another package,
    /// [`IdentityWitnessRefusal::Unknown`] for rows IR does not join, and
    /// [`IdentityJoinFailure::Exhausted`] with IR's refusal when `budget`
    /// runs out, which says nothing about the rows.
    pub fn selected(
        &self,
        package: &CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<SelectedExactIdentities, IdentityJoinFailure> {
        if self.package != package.name() {
            return Err(IdentityWitnessRefusal::Foreign.into());
        }
        self.join(package, budget).map_err(|refusal| {
            if qsl_semantics::model::intake::is_native_budget_denial(&refusal) {
                IdentityJoinFailure::Exhausted(Box::new(refusal))
            } else {
                IdentityWitnessRefusal::Unknown.into()
            }
        })
    }

    fn join(
        &self,
        package: &CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<SelectedExactIdentities, ExactTypeProjectionError> {
        SelectedExactIdentities::from_stage_rows(
            package,
            self.declarations.iter().cloned(),
            self.fields.iter().cloned(),
            budget,
        )?
        .with_source_record_rows(package, self.source_records.iter().cloned(), budget)?
        .with_source_union_rows(package, self.source_unions.iter().cloned(), budget)
    }

    /// The package the rows were produced for.
    pub fn package_name(&self) -> &str {
        &self.package
    }

    /// The selected declaration rows, in file order.
    pub fn declarations(&self) -> &[SelectedExactDeclarationRow] {
        &self.declarations
    }

    /// The selected record-field rows, in file order.
    pub fn fields(&self) -> &[SelectedExactFieldRow] {
        &self.fields
    }

    /// Original source-record producer rows, with final graph references.
    pub fn source_records(&self) -> &[SelectedSourceRecordRow] {
        &self.source_records
    }

    /// Original source-union producer rows, in declaration and payload order.
    pub fn source_unions(&self) -> &[SelectedSourceUnionRow] {
        &self.source_unions
    }

    /// Read `identities_json`, joining its rows to `package`.
    ///
    /// The rows grant no authority: they are trusted as the package file is,
    /// as an artifact of the producer the consumer chose to read. Only what
    /// the package can answer is verified: that each node a row names is a
    /// node of the package and that each declaration's form matches its
    /// identity. A refusal returns no rows. The validating join charges
    /// `budget`, the caller's one native allowance for the pass (FR-358).
    ///
    /// # Errors
    ///
    /// [`IdentityFileRefusal`]; `Native` carries IR's budget denial when
    /// `budget` runs out.
    pub fn read(
        identities_json: &[u8],
        package: &CheckedPackageV2,
        budget: &mut NativeJoinBudget,
    ) -> Result<Self, IdentityFileRefusal> {
        let malformed = |member: IdentityMember| IdentityFileRefusal::Malformed { member };
        let file = parse_file(identities_json).map_err(|member| match member {
            IdentityMember::Version(format) => IdentityFileRefusal::Format(format),
            other => malformed(other),
        })?;
        if !is_identities_format(&file.format) {
            return Err(IdentityFileRefusal::Format(file.format));
        }
        let graph = wire_graph(package).map_err(|_| malformed(IdentityMember::File))?;
        let held: BTreeSet<&CheckedNodeId> = package
            .graph()
            .nodes
            .iter()
            .map(|node| &node.node_id)
            .collect();
        let key = |id: &CheckedNodeId, member: IdentityMember, index: usize| {
            if held.contains(id) {
                graph
                    .key(id)
                    .map_err(|_| IdentityFileRefusal::Unjoined { member, index })
            } else {
                Err(IdentityFileRefusal::Unjoined { member, index })
            }
        };
        let mut declarations = Vec::new();
        for (index, row) in file.declarations.iter().enumerate() {
            let unjoined = IdentityFileRefusal::Unjoined {
                member: IdentityMember::Declarations,
                index,
            };
            if !held.contains(&row.structural_node) {
                return Err(unjoined);
            }
            let form = form_of(&row.form).ok_or_else(|| malformed(IdentityMember::Form))?;
            let identity = match (row.exact_identity.kind.as_str(), form) {
                ("record", SelectedExactDeclarationForm::RecordValueType) => {
                    SelectedExactIdentity::Record(node_value(
                        &row.exact_identity.value,
                        &key,
                        IdentityMember::Declarations,
                        index,
                    )?)
                }
                ("union", SelectedExactDeclarationForm::Union) => {
                    SelectedExactIdentity::Union(node_value(
                        &row.exact_identity.value,
                        &key,
                        IdentityMember::Declarations,
                        index,
                    )?)
                }
                ("object", SelectedExactDeclarationForm::ObjectType) => {
                    let effective: EffectiveJson =
                        serde_json::from_value(row.exact_identity.value.clone())
                            .map_err(|_| malformed(IdentityMember::ExactIdentity))?;
                    SelectedExactIdentity::Object(effective_id(&effective, &malformed)?)
                }
                ("declared_unit", SelectedExactDeclarationForm::DeclaredUnit) => {
                    let unit: UnitJson = serde_json::from_value(row.exact_identity.value.clone())
                        .map_err(|_| malformed(IdentityMember::ExactIdentity))?;
                    if !is_declared_unit_kind(&unit.kind) {
                        return Err(malformed(IdentityMember::ExactIdentity));
                    }
                    SelectedExactIdentity::DeclaredUnit(UnitId::declared(key(
                        &unit.node,
                        IdentityMember::Declarations,
                        index,
                    )?))
                }
                ("record" | "union" | "object" | "declared_unit", _) => return Err(unjoined),
                _ => return Err(malformed(IdentityMember::ExactIdentity)),
            };
            declarations.push(SelectedExactDeclarationRow::new(
                row.structural_node.clone(),
                row.selected_document.as_str(),
                form,
                row.original_identity.as_str(),
                identity,
            ));
        }
        let mut fields = Vec::new();
        for (index, row) in file.fields.iter().enumerate() {
            if !held.contains(&row.record_node) {
                return Err(IdentityFileRefusal::Unjoined {
                    member: IdentityMember::Fields,
                    index,
                });
            }
            let owner = match (
                row.member_id.owner.kind.as_str(),
                &row.member_id.owner.node,
                &row.member_id.owner.effective,
            ) {
                ("node", Some(node), None) => {
                    MemberOwner::Node(key(node, IdentityMember::Fields, index)?)
                }
                ("effective", None, Some(effective)) => {
                    MemberOwner::Effective(effective_id(effective, &malformed)?)
                }
                _ => return Err(malformed(IdentityMember::MemberId)),
            };
            let member_id = MemberId::new(owner, row.member_id.member.as_str())
                .map_err(|_| malformed(IdentityMember::MemberId))?;
            fields.push(SelectedExactFieldRow::new(
                row.record_node.clone(),
                row.selected_document.as_str(),
                row.record_original_identity.as_str(),
                row.field_original_identity.as_str(),
                member_id,
            ));
        }
        let source_records = file
            .source_records
            .into_iter()
            .map(SourceRecordJson::row)
            .collect::<Result<Vec<_>, _>>()?;
        let rows = Self {
            package: package.name().to_owned(),
            declarations,
            fields,
            source_records,
            source_unions: file
                .source_unions
                .into_iter()
                .map(SourceUnionJson::row)
                .collect::<Result<_, _>>()?,
        };
        rows.join(package, budget)
            .map_err(|error| IdentityFileRefusal::Native(Box::new(error)))?;
        Ok(rows)
    }

    /// The rows of the in-process unit, for a package it was emitted as.
    pub fn of_rows(
        package: &CheckedPackageV2,
        declarations: Vec<SelectedExactDeclarationRow>,
        fields: Vec<SelectedExactFieldRow>,
        source_records: Vec<SelectedSourceRecordRow>,
        source_unions: Vec<SelectedSourceUnionRow>,
    ) -> Self {
        Self {
            package: package.name().to_owned(),
            declarations,
            fields,
            source_records,
            source_unions,
        }
    }

    /// The identity file of these rows (FR-531 "The identity file"): one JSON
    /// object, pretty-printed with two-space indent, members in the
    /// documented order, UTF-8, ending in one newline, byte-identical for
    /// equal inputs.
    ///
    /// # Errors
    ///
    /// [`IdentityFileRefusal::Malformed`] for a compound unit, which is no
    /// selected declaration, and for a key the package does not hold.
    pub fn to_bytes(&self, package: &CheckedPackageV2) -> Result<Vec<u8>, IdentityFileRefusal> {
        let malformed = |member: IdentityMember| IdentityFileRefusal::Malformed { member };
        let references = node_references(package).ok_or_else(|| malformed(IdentityMember::File))?;
        let reference = |key: &NodeKey| {
            references
                .get(key)
                .cloned()
                .ok_or_else(|| malformed(IdentityMember::ExactIdentity))
        };
        let effective = |id: &EffectiveId| EffectiveJson {
            chain: id
                .owner_chain()
                .iter()
                .map(|link| LinkJson {
                    package: link.package().to_owned(),
                    node: link.node().to_owned(),
                })
                .collect(),
        };
        let mut declarations = Vec::new();
        for row in &self.declarations {
            let (kind, value) = match &row.exact_identity {
                SelectedExactIdentity::Record(key) => (
                    "record",
                    serde_json::to_value(reference(key)?)
                        .map_err(|_| malformed(IdentityMember::File))?,
                ),
                SelectedExactIdentity::Union(key) => (
                    "union",
                    serde_json::to_value(reference(key)?)
                        .map_err(|_| malformed(IdentityMember::File))?,
                ),
                SelectedExactIdentity::Object(id) => (
                    "object",
                    serde_json::to_value(effective(id))
                        .map_err(|_| malformed(IdentityMember::File))?,
                ),
                SelectedExactIdentity::DeclaredUnit(unit) => {
                    let key = unit
                        .declared_key()
                        .ok_or_else(|| malformed(IdentityMember::ExactIdentity))?;
                    (
                        "declared_unit",
                        serde_json::to_value(UnitJson {
                            kind: "declared".to_owned(),
                            node: reference(key)?,
                        })
                        .map_err(|_| malformed(IdentityMember::File))?,
                    )
                }
            };
            declarations.push(DeclarationJson {
                structural_node: row.structural_node.clone(),
                selected_document: row.selected_document.to_string(),
                form: form_spelling(row.form).to_owned(),
                original_identity: row.original_identity.to_string(),
                exact_identity: IdentityJson {
                    kind: kind.to_owned(),
                    value,
                },
            });
        }
        let mut fields = Vec::new();
        for row in &self.fields {
            let owner = match row.member_id.owner() {
                MemberOwner::Node(key) => OwnerJson {
                    kind: "node".to_owned(),
                    node: Some(reference(key)?),
                    effective: None,
                },
                MemberOwner::Effective(id) => OwnerJson {
                    kind: "effective".to_owned(),
                    node: None,
                    effective: Some(effective(id)),
                },
            };
            fields.push(FieldJson {
                record_node: row.record_node.clone(),
                selected_document: row.selected_document.to_string(),
                record_original_identity: row.record_original_identity.to_string(),
                field_original_identity: row.field_original_identity.to_string(),
                member_id: MemberJson {
                    owner,
                    member: row.member_id.member().to_owned(),
                },
            });
        }
        let file = File {
            format: IDENTITIES_FORMAT.to_owned(),
            declarations,
            fields,
            source_records: self
                .source_records
                .iter()
                .map(SourceRecordJson::of)
                .collect::<Result<_, _>>()?,
            source_unions: self
                .source_unions
                .iter()
                .map(SourceUnionJson::of)
                .collect::<Result<_, _>>()?,
        };
        let mut bytes =
            serde_json::to_vec_pretty(&file).map_err(|_| malformed(IdentityMember::File))?;
        bytes.push(b'\n');
        Ok(bytes)
    }
}

fn node_value(
    value: &serde_json::Value,
    key: &impl Fn(&CheckedNodeId, IdentityMember, usize) -> Result<NodeKey, IdentityFileRefusal>,
    member: IdentityMember,
    index: usize,
) -> Result<NodeKey, IdentityFileRefusal> {
    let id: CheckedNodeId =
        serde_json::from_value(value.clone()).map_err(|_| IdentityFileRefusal::Malformed {
            member: IdentityMember::ExactIdentity,
        })?;
    key(&id, member, index)
}

fn effective_id(
    effective: &EffectiveJson,
    malformed: &impl Fn(IdentityMember) -> IdentityFileRefusal,
) -> Result<EffectiveId, IdentityFileRefusal> {
    let mut chain = effective.chain.iter();
    let first = chain
        .next()
        .ok_or_else(|| malformed(IdentityMember::ExactIdentity))?;
    let mut id = EffectiveId::new(
        None,
        OriginalDeclarationKey::new(&first.package, &first.node),
    );
    for link in chain {
        id = EffectiveId::new(
            Some(id),
            OriginalDeclarationKey::new(&link.package, &link.node),
        );
    }
    Ok(id)
}
