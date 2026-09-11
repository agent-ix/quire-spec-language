// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: source/proof correspondence and exact selected semantic resources.
use super::{
    layout,
    types::{text, ValueBuilder},
    Selections,
};
use crate::checking::composed::proofs::ProofReport;
use crate::linking::composed::models::{ModelInput, ModelTarget};
use crate::linking::composed::{definition_source::RegisteredDefinition as R, DeclarationId};
use crate::protocol_artifact::{
    self as artifact, wire as w, work::Work, ByteDigest, Dimension, Error, Invalid, Unsupported,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Metadata<'a> {
    pub dependencies: Vec<artifact::SuppliedDependency<'a>>,
    pub registered: Vec<R>,
    pub definitions: Vec<w::Definition>,
    pub declaration_indices: BTreeMap<DeclarationId, u32>,
}
impl Metadata<'_> {
    pub fn definition(&self, registered: R, work: &mut Work) -> Result<u32, Error> {
        for (index, selected) in self.registered.iter().enumerate() {
            work.visit()?;
            if *selected == registered {
                return layout::index(index);
            }
        }
        Err(Error::Unsupported(Unsupported::Definition))
    }
    pub fn dependency(&self, reference: &w::ArtifactRef, work: &mut Work) -> Result<u32, Error> {
        dependency(&self.dependencies, reference, work)
    }
    pub fn definition_dependency(&self, registered: R, work: &mut Work) -> Result<u32, Error> {
        Ok(self.definitions[self.definition(registered, work)? as usize].artifact)
    }
}
pub(super) fn copy_reference(
    value: &w::ArtifactRef,
    work: &mut Work,
) -> Result<w::ArtifactRef, Error> {
    artifact::intake::reference(value)?;
    let bytes = value.ref_version.len()
        + value.authority.len()
        + value.identity.len()
        + value.revision.namespace.len()
        + value.revision.value.len()
        + value.wire.identity.len()
        + value.wire.version.len();
    work.bytes(bytes)?;
    work.charge(Dimension::Entries, 3)?;
    Ok(value.clone())
}
fn dependency(
    dependencies: &[artifact::SuppliedDependency<'_>],
    reference: &w::ArtifactRef,
    work: &mut Work,
) -> Result<u32, Error> {
    for (i, d) in dependencies.iter().enumerate() {
        work.visit()?;
        if artifact::intake::key(d.artifact) == artifact::intake::key(reference) {
            artifact::intake::same_reference(d.artifact, reference, work)?;
            return layout::index(i);
        }
    }
    Err(Error::Invalid(Invalid::Dependency))
}
fn matching_bytes(
    dependencies: &[artifact::SuppliedDependency<'_>],
    bytes: &[u8],
    work: &mut Work,
) -> Result<u32, Error> {
    let mut result = None;
    for (i, d) in dependencies.iter().enumerate() {
        work.visit()?;
        if d.artifact.kind != w::ArtifactKind::Source || d.bytes.len() != bytes.len() {
            continue;
        }
        work.bytes(bytes.len().saturating_mul(2))?;
        if d.bytes == bytes {
            if result.is_some() {
                return Err(Error::Invalid(Invalid::Duplicate));
            }
            result = Some(layout::index(i)?);
        }
    }
    result.ok_or(Error::Unsupported(Unsupported::Definition))
}
fn definitions(
    dependencies: &[artifact::SuppliedDependency<'_>],
    revision_namespace: &str,
    work: &mut Work,
) -> Result<(Vec<R>, Vec<w::Definition>), Error> {
    let mut selected = Vec::new();
    for &registered in R::all() {
        match matching_bytes(dependencies, registered.bytes(), work) {
            Ok(at) => {
                work.charge(Dimension::Entries, 1)?;
                selected.push((registered, at));
            }
            Err(Error::Unsupported(Unsupported::Definition)) => {}
            Err(e) => return Err(e),
        }
    }
    selected.sort_by_key(|(r, _)| (r.identity(), revision_namespace, r.revision()));
    work.charge(Dimension::Definitions, selected.len())?;
    let mut definitions = Vec::new();
    for &(registered, at) in &selected {
        let mut requires = Vec::new();
        for required in registered.requirements() {
            work.visit()?;
            let index = selected
                .iter()
                .position(|(r, _)| r == required)
                .ok_or(Error::Unsupported(Unsupported::Definition))?;
            work.charge(Dimension::Entries, 1)?;
            requires.push(layout::index(index)?);
        }
        requires.sort_unstable();
        let mut rules = Vec::new();
        for rule in registered.rules() {
            work.charge(Dimension::Entries, 1)?;
            rules.push(matching_bytes(dependencies, rule.bytes, work)?);
        }
        rules.sort_unstable();
        work.charge(Dimension::Entries, 1)?;
        definitions.push(w::Definition {
            identity: text(registered.identity(), work)?,
            revision: w::Revision {
                namespace: text(revision_namespace, work)?,
                value: text(registered.revision(), work)?,
            },
            artifact: at,
            rules,
            requires,
        });
    }
    work.charge(Dimension::Entries, selected.len())?;
    Ok((selected.into_iter().map(|(r, _)| r).collect(), definitions))
}
pub(super) fn lower(
    proofs: &ProofReport<'_, '_, '_>,
    selections: &Selections<'_>,
    work: &mut Work,
) -> Result<w::Package, Error> {
    let binding = proofs.types().binding();
    let namespace = binding.namespace();
    artifact::intake::name(selections.requirement_revision_namespace)?;
    work.charge(Dimension::Dependencies, selections.dependencies.len())?;
    work.charge(Dimension::Entries, selections.dependencies.len())?;
    let mut dependencies = selections.dependencies.to_vec();
    dependencies.sort_by_key(|d| artifact::intake::key(d.artifact));
    for pair in dependencies.windows(2) {
        work.visit()?;
        if artifact::intake::key(pair[0].artifact) >= artifact::intake::key(pair[1].artifact) {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
    }
    let mut wire_dependencies = Vec::new();
    for d in &dependencies {
        work.visit()?;
        artifact::intake::reference(d.artifact)?;
        work.bytes(d.bytes.len())?;
        if ByteDigest::of(d.bytes) != d.artifact.digest {
            return Err(Error::Invalid(Invalid::Seal));
        }
        let mut requires = Vec::new();
        for r in d.requires {
            work.charge(Dimension::Entries, 1)?;
            requires.push(dependency(&dependencies, r, work)?);
        }
        requires.sort_unstable();
        artifact::intake::sorted_indices(&requires, work)?;
        work.charge(Dimension::Entries, 1)?;
        wire_dependencies.push(w::Dependency {
            artifact: copy_reference(d.artifact, work)?,
            requires,
        });
    }
    for r in [
        selections.contract,
        selections.baseline,
        &selections.producer.binary,
    ] {
        dependency(&dependencies, r, work)?;
    }
    artifact::intake::name(selections.definition_revision_namespace)?;
    let (registered, definitions) = definitions(
        &dependencies,
        selections.definition_revision_namespace,
        work,
    )?;
    let mut meta = Metadata {
        dependencies,
        registered,
        definitions,
        declaration_indices: BTreeMap::new(),
    };
    let package_definition = meta.definition(R::Package, work)?;
    meta.definition(R::Protocol, work)?;
    meta.definition(R::Edition, work)?;
    let bound_definitions = binding
        .definitions()
        .ok_or(Error::Invalid(Invalid::Definition))?;
    for d in &bound_definitions.declarations {
        for use_ in &d.uses {
            for &r in &use_.closure {
                work.visit()?;
                meta.definition(r, work)?;
            }
        }
    }
    work.charge(Dimension::Sources, selections.sources.len())?;
    if selections.sources.len() != namespace.units().len() {
        return Err(Error::Invalid(Invalid::Inventory));
    }
    work.charge(
        Dimension::Entries,
        selections.sources.len().saturating_mul(2),
    )?;
    let mut sources = selections.sources.to_vec();
    sources.sort_by_key(|s| artifact::intake::key(s.artifact));
    let mut source_indices = vec![None; namespace.units().len()];
    let mut wire_sources = Vec::new();
    for (i, selected) in sources.iter().enumerate() {
        work.visit()?;
        if i > 0
            && artifact::intake::key(sources[i - 1].artifact)
                >= artifact::intake::key(selected.artifact)
        {
            return Err(Error::Invalid(Invalid::Duplicate));
        }
        let source = selected.source.source();
        if selected.artifact.kind != w::ArtifactKind::Source
            || selected.artifact.digest != source.digest()
        {
            return Err(Error::Invalid(Invalid::Selection));
        }
        work.charge(Dimension::SourceBytes, source.text().len())?;
        artifact::intake::name(selected.revision_namespace)?;
        let mut found = None;
        for (u, unit) in namespace.units().iter().enumerate() {
            work.visit()?;
            if unit.source().identity() == source.identity() {
                work.bytes(
                    unit.source()
                        .text()
                        .len()
                        .saturating_add(source.text().len()),
                )?;
                if unit.source().text() != source.text() || unit.source().path() != source.path() {
                    return Err(Error::Invalid(Invalid::Selection));
                }
                if found.is_some() || source_indices[u].replace(layout::index(i)?).is_some() {
                    return Err(Error::Invalid(Invalid::Duplicate));
                }
                found = Some(u);
            }
        }
        if found.is_none() {
            return Err(Error::Invalid(Invalid::Inventory));
        }
        work.charge(Dimension::Entries, 1)?;
        wire_sources.push(w::Source {
            artifact: copy_reference(selected.artifact, work)?,
            native: w::NativeSource {
                identity: text(&source.identity().identity, work)?,
                revision: text(&source.identity().revision, work)?,
            },
            path: text(source.path(), work)?,
            formal: w::Formal {
                document: text(selected.source.identity().document().as_str(), work)?,
                revision: w::Revision {
                    namespace: text(selected.revision_namespace, work)?,
                    value: decimal(selected.source.identity().revision().get(), work)?,
                },
            },
            text: text(source.text(), work)?,
        });
    }
    work.charge(Dimension::Entries, source_indices.len())?;
    let checked_sources = source_indices
        .iter()
        .map(|s| s.ok_or(Error::Invalid(Invalid::Inventory)))
        .collect::<Result<Vec<_>, _>>()?;
    super::families::check(proofs, &checked_sources, work)?;
    for proof in proofs.declarations() {
        work.visit()?;
        let authored = proof
            .authored_binding()
            .ok_or(Error::Invalid(Invalid::Owner))?;
        let source = &sources[checked_sources[proof.unit().index()] as usize];
        if proofs.bindings()[authored.source].source.identity() != source.source.identity() {
            return Err(Error::Invalid(Invalid::Owner));
        }
    }

    work.charge(Dimension::Declarations, proofs.declarations().len())?;
    work.charge(
        Dimension::Entries,
        proofs.declarations().len().saturating_mul(2),
    )?;
    let mut keyed = Vec::new();
    for d in proofs.declarations() {
        work.visit()?;
        let entry = namespace
            .declaration(d.declaration())
            .ok_or(Error::Invalid(Invalid::Owner))?;
        let syntax = namespace
            .syntax(d.declaration())
            .ok_or(Error::Invalid(Invalid::Owner))?;
        keyed.push((
            (
                source_indices[entry.unit().index()],
                syntax.span.start,
                syntax.span.end,
            ),
            d.declaration(),
        ));
    }
    keyed.sort_by_key(|(key, _)| *key);
    let order = keyed.into_iter().map(|(_, id)| id).collect::<Vec<_>>();
    for (i, &d) in order.iter().enumerate() {
        meta.declaration_indices.insert(d, layout::index(i)?);
    }
    let model_binding = binding.models().ok_or(Error::Invalid(Invalid::Model))?;
    work.charge(Dimension::Models, selections.models.len())?;
    work.charge(
        Dimension::Entries,
        selections.models.len().saturating_mul(2),
    )?;
    let mut models = Vec::new();
    for model in selections.models {
        let mut found = false;
        for input in model_binding.inputs() {
            work.visit()?;
            if matches!(input,ModelInput::Native(actual) if std::ptr::eq(*actual,model.model)) {
                found = true;
            }
        }
        if !found {
            return Err(Error::Invalid(Invalid::Model));
        }
        models.push((meta.dependency(model.artifact, work)?, *model));
    }
    models.sort_by_key(|(index, _)| *index);
    let indices = models.iter().map(|(i, _)| *i).collect::<Vec<_>>();
    let selected_models = models.into_iter().map(|(_, m)| m).collect::<Vec<_>>();
    let mut builder = ValueBuilder::new(&selected_models, &indices, work)?;
    let mut declarations = Vec::new();
    let mut families = BTreeSet::new();
    let mut features = BTreeSet::from(["quire.protocol.bindings/1", "quire.protocol.numeric/1"]);
    for id in order {
        work.visit()?;
        let typed = proofs
            .types()
            .declaration(id)
            .ok_or(Error::Invalid(Invalid::Type))?;
        let scope = binding
            .scopes()
            .and_then(|s| s.declaration(id))
            .ok_or(Error::Invalid(Invalid::Scope))?;
        let syntax = namespace.syntax(id).ok_or(Error::Invalid(Invalid::Owner))?;
        let unit = namespace
            .unit(typed.unit())
            .ok_or(Error::Invalid(Invalid::Owner))?;
        let layout = layout::build(
            namespace,
            typed,
            scope,
            meta.declaration_indices[&id],
            source_indices[typed.unit().index()].ok_or(Error::Invalid(Invalid::Inventory))?,
            &meta.declaration_indices,
            work,
        )?;
        work.locus = Some(layout.locus(syntax.span)?);
        let profiles = bound_definitions
            .declarations
            .get(id.index())
            .filter(|d| d.declaration == id)
            .ok_or(Error::Invalid(Invalid::Profile))?;
        let mut profile_indices = Vec::new();
        for use_ in &profiles.uses {
            work.charge(Dimension::Entries, 1)?;
            profile_indices.push(
                meta.definition(
                    *use_
                        .closure
                        .first()
                        .ok_or(Error::Invalid(Invalid::Profile))?,
                    work,
                )?,
            );
        }
        let profile = *profile_indices
            .first()
            .ok_or(Error::Invalid(Invalid::Profile))?;
        let mut binders = super::runtime::binders(typed, scope, &layout, &mut builder, work)?;
        let entry = namespace
            .declaration(id)
            .ok_or(Error::Invalid(Invalid::Owner))?;
        let exports = binding
            .exports()
            .get(id.index())
            .filter(|report| report.declaration == id)
            .ok_or(Error::Invalid(Invalid::Model))?;
        let context = super::context::Declaration {
            typed,
            unit,
            syntax,
            scope,
            exports,
            references: entry.references(),
            profile_uses: &profiles.uses,
            profiles: &profile_indices,
            layout: &layout,
        };
        let (anchors, bindings, mut body) =
            super::runtime::body(&context, &meta, &binders, &mut builder, work)?;
        // Initializers reference values in the same original occurrence layout.
        super::runtime::initializers(syntax, scope, &layout, &mut binders)?;
        let values = builder.lower_values(typed, unit, scope, &layout, &profile_indices, work)?;
        if let w::Body::Predicate { result, .. } = &mut body {
            *result = builder.ty(&crate::checking::NativeType::Boolean, work)?;
        }
        let temporal = super::families::temporal(unit, &layout, work)?;
        let family = match &body {
            w::Body::Predicate { .. } => "declaration.predicate",
            w::Body::State { .. } => "family.state",
            w::Body::Temporal { .. } => "family.temporal",
            w::Body::Protocol { controls, .. } => {
                if !controls.is_empty() {
                    features.insert("quire.protocol.control/1");
                }
                "family.protocol"
            }
        };
        families.insert(family);
        if !values.is_empty() {
            features.insert("quire.protocol.values/1");
        }
        if !temporal.is_empty() {
            features.insert("quire.protocol.temporal/1");
        }
        let clause = proofs
            .clause_binding(id)
            .ok_or(Error::Invalid(Invalid::Owner))?;
        let execution = execution(&clause.execution_point, &context, &builder, work)?;
        let mut requires = BTreeSet::new();
        for reference in namespace
            .declaration(id)
            .ok_or(Error::Invalid(Invalid::Owner))?
            .references()
        {
            work.visit()?;
            let target = reference
                .target
                .and_then(|d| meta.declaration_indices.get(&d))
                .ok_or(Error::Invalid(Invalid::Dependency))?;
            work.charge(Dimension::Entries, 1)?;
            requires.insert(*target);
        }
        work.charge(Dimension::Entries, 1)?;
        declarations.push(w::Declaration {
            name: text(&syntax.name.value, work)?,
            locus: layout.locus(syntax.span)?,
            requirement: w::Requirement {
                package: text(clause.requirement.package().as_str(), work)?,
                identity: text(clause.requirement.requirement().as_str(), work)?,
                revision: w::Revision {
                    namespace: text(selections.requirement_revision_namespace, work)?,
                    value: decimal(clause.requirement.revision().get(), work)?,
                },
            },
            clause: text(clause.clause.as_str(), work)?,
            execution,
            profile,
            requires: requires.into_iter().collect(),
            scopes: layout.scopes,
            anchors,
            binders,
            values,
            temporal,
            bindings,
            body,
        });
    }
    let (types, models) = builder.finish();
    let first = namespace
        .units()
        .first()
        .ok_or(Error::Invalid(Invalid::Inventory))?;
    let language = w::Language {
        identity: text(&first.language().value, work)?,
        edition: text(&first.edition().value, work)?,
    };
    let package = w::Package {
        wire: text(artifact::WIRE, work)?,
        media: text(artifact::MEDIA, work)?,
        schema: text(artifact::SCHEMA, work)?,
        package_type: text(artifact::PACKAGE_TYPE, work)?,
        encoding: text(artifact::ENCODING, work)?,
        numeric: text(artifact::NUMERIC_PROFILE, work)?,
        contract: copy_reference(selections.contract, work)?,
        producer: w::Producer {
            implementation: text(&selections.producer.implementation, work)?,
            revision: w::Revision {
                namespace: text(&selections.producer.revision.namespace, work)?,
                value: text(&selections.producer.revision.value, work)?,
            },
            binary: copy_reference(&selections.producer.binary, work)?,
        },
        baseline: copy_reference(selections.baseline, work)?,
        language,
        package_definition,
        features: w::Features {
            declarations: families
                .into_iter()
                .map(|s| text(s, work))
                .collect::<Result<_, _>>()?,
            required: features
                .into_iter()
                .map(|s| text(s, work))
                .collect::<Result<_, _>>()?,
            optional: Vec::new(),
        },
        sources: wire_sources,
        dependencies: wire_dependencies,
        definitions: meta.definitions,
        models,
        types,
        declarations,
    };
    // Existing admitted-model adapter checks the actual export/locus/type authority.
    artifact::models::validate(&package, selections.models, selections.dependencies, work)?;
    Ok(package)
}
fn decimal(value: u64, work: &mut Work) -> Result<String, Error> {
    work.bytes(20)?;
    Ok(value.to_string())
}
fn execution(
    point: &quire_contract_ir::ExecutionPoint,
    context: &super::context::Declaration<'_, '_>,
    builder: &ValueBuilder<'_>,
    work: &mut Work,
) -> Result<w::Execution, Error> {
    use quire_contract_ir::ExecutionPoint as E;
    match point {
        E::Initialization { name } => Ok(w::Execution::Initialization {
            name: text(name.as_str(), work)?,
        }),
        E::Handler { name } => Ok(w::Execution::Handler {
            name: text(name.as_str(), work)?,
        }),
        E::Pre { operation } => Ok(w::Execution::Pre {
            operation: execution_operation(operation, context, builder, work)?,
        }),
        E::Post { operation } => Ok(w::Execution::Post {
            operation: execution_operation(operation, context, builder, work)?,
        }),
    }
}
fn execution_operation(
    operation: &quire_contract_ir::AnchorName,
    context: &super::context::Declaration<'_, '_>,
    builder: &ValueBuilder<'_>,
    work: &mut Work,
) -> Result<w::ExportRef, Error> {
    for occurrence in &context.exports.occurrences {
        work.visit()?;
        if let ModelTarget::Operation(op) = &occurrence.target {
            if &op.role().anchor == operation {
                return builder.export(
                    op.model(),
                    w::ExportKind::Operation,
                    op.role().context.as_str(),
                    Some(op.role().name.as_str()),
                    work,
                );
            }
        }
    }
    Err(Error::Invalid(Invalid::Model))
}
