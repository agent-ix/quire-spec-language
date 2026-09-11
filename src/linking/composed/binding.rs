// SPDX-License-Identifier: AGPL-3.0-only
//! FR-036: compose static definition, model/export and lexical binding stages.
//! Resolved names are inputs to type/family checking, never executable packages.

use std::collections::{BTreeMap, VecDeque};

use super::binding_work::{Dimension, Exhaustion, Limits, Usage, Work};
use super::definitions;
use super::models::{self, ModelBindings, ModelDeclarationReport, ModelInput};
use super::scopes::{self, ScopeDisposition, ScopeReport};
use super::{DeclarationDisposition, DeclarationId, SyntaxNamespace, UnitId};
use crate::Span;

/// Source-owned evidence location in one of the retained component reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// The original namespace record retains the exact name/cycle cause.
    Namespace,
    /// Shared duplicate model/profile alias evidence in this source unit.
    Alias { conflict: usize },
    /// The selected edition definition/closure refused all declarations.
    Edition,
    /// Index into this declaration's definition-profile uses.
    Profile { occurrence: usize },
    /// Index into this declaration's model/export errors.
    Model { occurrence: usize },
    /// Index into this declaration's lexical/structural issues.
    Scope { occurrence: usize },
    /// A typed native dependency refused; the namespace retains its source span.
    Dependency {
        reference: usize,
        target: DeclarationId,
    },
}

/// Binding-stage outcome only. No variant asserts expression/family checking.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// All required names/definitions/exports/scopes resolved for type checking.
    NamesResolved,
    /// A known local or dependency refusal prevents binding.
    Refused,
    /// Required processing is unfinished, including an exhausted earlier stage.
    Unfinished,
}

/// One declaration's causes, whose source identity lives in the retained namespace.
#[derive(Debug)]
pub struct DeclarationBinding {
    declaration: DeclarationId,
    refusals: Vec<Refusal>,
}

/// Two authored model/profile aliases collide within one source unit.
#[derive(Debug)]
pub struct AliasConflict {
    /// Source owner of both original token regions.
    pub unit: UnitId,
    /// Original first alias token.
    pub first: Span,
    /// Original conflicting alias token; all occurrences are retained.
    pub repeated: Span,
}

impl DeclarationBinding {
    /// Namespace-local original declaration identity.
    pub fn declaration(&self) -> DeclarationId {
        self.declaration
    }
    /// Shared evidence references without copying complete model/source artifacts.
    pub fn refusals(&self) -> &[Refusal] {
        &self.refusals
    }
}

/// Immutable report over original inputs. Missing component/record means that
/// work did not reach that stage; it never means successful empty admission.
#[derive(Debug)]
pub struct Report<'a> {
    namespace: &'a SyntaxNamespace,
    definitions: Option<definitions::Report<'a>>,
    models: Option<ModelBindings<'a>>,
    exports: Vec<ModelDeclarationReport<'a>>,
    scopes: Option<ScopeReport>,
    declarations: Vec<DeclarationBinding>,
    aliases: Vec<AliasConflict>,
    exhaustion: Option<Exhaustion>,
    complete: bool,
    limits: Limits,
    usage: Usage,
}

impl<'a> Report<'a> {
    /// Original constructor-private source namespace and dependency evidence.
    pub fn namespace(&self) -> &'a SyntaxNamespace {
        self.namespace
    }
    /// Exact definition selections/rules and partial profile closure evidence.
    pub fn definitions(&self) -> Option<&definitions::Report<'a>> {
        self.definitions.as_ref()
    }
    /// Original admitted models and unit-local import selections.
    pub fn models(&self) -> Option<&ModelBindings<'a>> {
        self.models.as_ref()
    }
    /// Per-declaration model-export occurrences, including located failures.
    pub fn exports(&self) -> &[ModelDeclarationReport<'a>] {
        &self.exports
    }
    /// Declaration-owned binders, anchors and structural protocol references.
    pub fn scopes(&self) -> Option<&ScopeReport> {
        self.scopes.as_ref()
    }
    /// Created result records; missing trailing records remain unfinished.
    pub fn declarations(&self) -> &[DeclarationBinding] {
        &self.declarations
    }
    /// Duplicate local aliases across both model and profile namespaces.
    pub fn alias_conflicts(&self) -> &[AliasConflict] {
        &self.aliases
    }
    /// First unaffordable binding operation, if any.
    pub fn exhaustion(&self) -> Option<&Exhaustion> {
        self.exhaustion.as_ref()
    }
    /// Effective invocation capacities, separate from semantic selections.
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Successful charges, including work in all reached component stages.
    pub fn usage(&self) -> Usage {
        self.usage
    }
    /// All binding passes finished; known semantic refusals may remain.
    pub fn complete(&self) -> bool {
        self.complete
    }
    /// A missing or unfinished declaration is never silently successful.
    pub fn disposition(&self, id: DeclarationId) -> Option<Disposition> {
        self.namespace.declaration(id)?;
        let refusal = self.namespace.disposition(id) == Some(DeclarationDisposition::Refused)
            || self
                .declarations
                .get(id.index())
                .is_some_and(|entry| !entry.refusals.is_empty())
            || self.definitions.as_ref().is_some_and(|report| {
                report.edition_refusal.is_some()
                    || report
                        .declarations
                        .get(id.index())
                        .is_some_and(|entry| entry.uses.iter().any(|usage| usage.refusal.is_some()))
            })
            || self.exports.get(id.index()).is_some_and(|entry| {
                entry.refusals.iter().any(|error| {
                    !matches!(error.kind, models::ModelErrorKind::ResourceExhausted(_))
                })
            })
            || self
                .scopes
                .as_ref()
                .and_then(|report| report.declaration(id))
                .is_some_and(|entry| !entry.issues.is_empty());
        Some(if refusal {
            Disposition::Refused
        } else if self.complete {
            Disposition::NamesResolved
        } else {
            Disposition::Unfinished
        })
    }
}

/// Bind exact supplied static dependencies without accepting runtime observations.
///
/// Component accounting rules apply in stage order: source-result records,
/// definitions, models/exports, scopes, then dependent-refusal propagation. Every
/// result/cause costs one Binding; native dependency insertion and each propagation
/// visit cost one Edge. Any exhaustion stops subsequent work. A retry has fresh
/// counters and leaves the prior report and source/model inputs unchanged.
pub fn bind<'a>(
    namespace: &'a SyntaxNamespace,
    definitions: &'a definitions::Inventory<'a>,
    models: &'a [ModelInput<'a>],
    limits: Limits,
) -> Report<'a> {
    let mut work = Work::new(limits);
    let mut report = Report {
        namespace,
        definitions: None,
        models: None,
        exports: Vec::new(),
        scopes: None,
        declarations: Vec::new(),
        aliases: Vec::new(),
        exhaustion: None,
        complete: false,
        limits: work.limits(),
        usage: Usage::default(),
    };
    match run(&mut report, definitions, models, &mut work) {
        Ok(complete) => report.complete = complete,
        Err(exhaustion) => report.exhaustion = Some(exhaustion),
    }
    report.usage = work.usage();
    report
}

fn run<'a>(
    report: &mut Report<'a>,
    definitions: &'a definitions::Inventory<'a>,
    models: &'a [ModelInput<'a>],
    work: &mut Work,
) -> Result<bool, Exhaustion> {
    for (index, _) in report.namespace.declarations().iter().enumerate() {
        work.charge(Dimension::Bindings, 1)?;
        let id = DeclarationId(index);
        let mut entry = DeclarationBinding {
            declaration: id,
            refusals: Vec::new(),
        };
        if report.namespace.disposition(id) == Some(DeclarationDisposition::Refused) {
            work.charge(Dimension::Bindings, 1)?;
            entry.refusals.push(Refusal::Namespace);
        }
        report.declarations.push(entry);
    }
    // An unfinished namespace cannot publish source dependency-closed bindings.
    if !report.namespace.dependencies_complete() {
        return Ok(false);
    }
    for (unit_index, unit) in report.namespace.units().iter().enumerate() {
        let unit_id = UnitId(unit_index);
        let mut aliases = BTreeMap::<&str, Span>::new();
        for import in unit.profiles().iter().chain(unit.models()) {
            work.charge(Dimension::References, 1)?;
            if let Some(first) = aliases.get(import.alias.value.as_str()) {
                work.charge(Dimension::Bindings, 1)?;
                let conflict = report.aliases.len();
                report.aliases.push(AliasConflict {
                    unit: unit_id,
                    first: *first,
                    repeated: import.alias.span,
                });
                for entry in &mut report.declarations {
                    work.charge(Dimension::References, 1)?;
                    if report
                        .namespace
                        .declaration(entry.declaration)
                        .expect("namespace declaration")
                        .unit()
                        == unit_id
                    {
                        work.charge(Dimension::Bindings, 1)?;
                        entry.refusals.push(Refusal::Alias { conflict });
                    }
                }
            } else {
                work.charge(Dimension::Bindings, 1)?;
                aliases.insert(&import.alias.value, import.alias.span);
            }
        }
    }
    report.definitions = Some(definitions::resolve(report.namespace, definitions, work));
    let defined = report
        .definitions
        .as_ref()
        .expect("definition stage retained");
    if let Some(exhaustion) = defined.exhaustion {
        return Err(exhaustion);
    }
    for entry in &mut report.declarations {
        if defined.edition_refusal.is_some() {
            work.charge(Dimension::Bindings, 1)?;
            entry.refusals.push(Refusal::Edition);
        }
        for (index, occurrence) in defined.declarations[entry.declaration.index()]
            .uses
            .iter()
            .enumerate()
        {
            if occurrence.refusal.is_some() {
                work.charge(Dimension::Bindings, 1)?;
                entry.refusals.push(Refusal::Profile { occurrence: index });
            }
        }
    }
    report.models = Some(models::bind_models(report.namespace, models, work));
    let bound_models = report.models.as_ref().expect("model catalog retained");
    if let Some(exhaustion) = bound_models.exhaustion() {
        return Err(*exhaustion);
    }
    for entry in &mut report.declarations {
        let output = bound_models
            .resolve_declaration(report.namespace, entry.declaration, work)
            .expect("enumerated namespace declaration");
        let exhaustion = output.exhaustion().copied();
        report.exports.push(output);
        if let Some(exhaustion) = exhaustion {
            return Err(exhaustion);
        }
        for index in 0..report
            .exports
            .last()
            .expect("export record retained")
            .refusals
            .len()
        {
            work.charge(Dimension::Bindings, 1)?;
            entry.refusals.push(Refusal::Model { occurrence: index });
        }
    }
    report.scopes = Some(scopes::resolve(report.namespace, bound_models, work));
    let scoped = report.scopes.as_ref().expect("scope evidence retained");
    if let Some(exhaustion) = scoped.exhaustion {
        return Err(exhaustion);
    }
    for entry in &mut report.declarations {
        let output = scoped
            .declaration(entry.declaration)
            .expect("completed scope traversal");
        for index in 0..output.issues.len() {
            work.charge(Dimension::Bindings, 1)?;
            entry.refusals.push(Refusal::Scope { occurrence: index });
        }
        debug_assert_ne!(
            scoped.disposition(entry.declaration),
            ScopeDisposition::Unfinished
        );
    }
    propagate(report, work)?;
    Ok(true)
}

fn propagate(report: &mut Report<'_>, work: &mut Work) -> Result<(), Exhaustion> {
    let mut incoming = vec![Vec::new(); report.declarations.len()];
    for (index, entry) in report.namespace.declarations().iter().enumerate() {
        for (reference, occurrence) in entry.references().iter().enumerate() {
            work.charge(Dimension::References, 1)?;
            if let Some(target) = occurrence.target {
                work.charge(Dimension::Edges, 1)?;
                incoming[target.index()].push((index, reference));
            }
        }
    }
    let mut queue: VecDeque<usize> = report
        .declarations
        .iter()
        .enumerate()
        .filter_map(|(index, entry)| (!entry.refusals.is_empty()).then_some(index))
        .collect();
    while let Some(target) = queue.pop_front() {
        for &(dependent, reference) in &incoming[target] {
            work.charge(Dimension::Edges, 1)?;
            if report.declarations[dependent].refusals.is_empty() {
                work.charge(Dimension::Bindings, 1)?;
                report.declarations[dependent]
                    .refusals
                    .push(Refusal::Dependency {
                        reference,
                        target: DeclarationId(target),
                    });
                queue.push_back(dependent);
            }
        }
    }
    Ok(())
}
