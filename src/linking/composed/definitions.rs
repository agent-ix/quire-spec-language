// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036: exact supplied definition/rule closure and unit-local profile aliases.
//!
//! The supported interpretation registry is compiler-owned. Supplying arbitrary
//! Markdown with a known name cannot grant that interpretation. Every selected
//! artifact and normative rule must be present in this invocation's inventory.

use std::collections::{BTreeMap, BTreeSet};

use super::binding_work::{Dimension, Exhaustion, Work};
use super::definition_source::RegisteredDefinition;
use super::{DeclarationId, SyntaxNamespace, UnitId};
use crate::syntax::composed::{ControlKind, DeclarationKind, ProtocolRequirement};
use crate::{ByteDigest, Span, Spanned};

/// Immutable definition selection. This digest covers original bytes only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    /// Exact registered definition identity, distinct from an alias or file path.
    pub identity: String,
    /// Exact definition revision, distinct from a language edition.
    pub revision: String,
    /// SHA-256 of the supplied definition artifact's raw bytes.
    pub digest: ByteDigest,
}

/// One explicitly supplied definition; metadata grants no interpretation.
#[derive(Clone, Debug)]
pub struct Artifact<'a> {
    /// Selection asserted by the input and checked against the artifact.
    pub selection: Selection,
    /// Unmodified normative document bytes.
    pub bytes: &'a [u8],
}

/// One normative rule selected by the compiler-supported definition snapshot.
#[derive(Clone, Copy, Debug)]
pub struct RuleInput<'a> {
    /// Exact rule path in the selected baseline; never used for file discovery.
    pub path: &'a str,
    /// Independently supplied raw-byte digest.
    pub digest: ByteDigest,
    /// Original supplied rule content.
    pub bytes: &'a [u8],
}

/// Closed invocation input. Installed resources do not fill omitted entries.
#[derive(Debug)]
pub struct Inventory<'a> {
    /// Definition selected for the language/edition already checked by parsing.
    pub edition: Selection,
    /// Supplied definition entries, including unused and conflicting entries.
    pub definitions: &'a [Artifact<'a>],
    /// Supplied normative rule entries.
    pub rules: &'a [RuleInput<'a>],
}

/// A typed reason an exact definition closure cannot be used.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Cause {
    /// No supplied artifact has this identity and revision.
    MissingDefinition(Selection),
    /// Multiple supplied entries claim one immutable identity/revision.
    AmbiguousDefinition {
        selection: Selection,
        entries: Vec<usize>,
    },
    /// Asserted digest does not match supplied content.
    DefinitionDigest { entry: usize },
    /// Native selection differs from the selected supplied artifact.
    SelectionMismatch { expected: Selection, entry: usize },
    /// The compiler has no interpretation for these exact bytes and labels.
    UnsupportedDefinition { entry: usize },
    /// A required normative rule was omitted.
    MissingRule { path: &'static str },
    /// Multiple supplied rule entries claim the same baseline path.
    AmbiguousRule {
        path: &'static str,
        entries: Vec<usize>,
    },
    /// Rule content or its declared digest differs from the supported baseline.
    RuleMismatch { path: &'static str, entry: usize },
    /// Exact selection is known but is not the composed edition definition.
    WrongEdition(Selection),
    /// An inherited requirement selected two meanings for one identity.
    IncompatibleRequirement { first: Selection, second: Selection },
    /// A semantic definition path repeats an active dependency.
    DefinitionCycle { definition: Selection },
    /// The authored profile alias is absent in this source unit.
    MissingAlias,
    /// More than one authored profile import has this alias.
    AmbiguousAlias { imports: Vec<usize> },
    /// The authored digest is not canonical SHA-256 text.
    InvalidDigest { span: Span },
    /// The exact root does not admit this declaration/obligation kind.
    WrongProfileKind {
        selected: RegisteredDefinition,
        required: UseKind,
    },
}

/// Declaration or nested clause requiring its own explicit root profile.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UseKind {
    /// Reusable Boolean declaration.
    Predicate,
    /// Invariant or operation pre/postcondition.
    State,
    /// Top-level temporal obligation with an explicit clock interpretation.
    Temporal,
    /// Protocol/choreography declaration.
    Protocol,
    /// Await or compensation requiring a concrete temporal interpretation.
    TimedObligation,
    /// A protocol check consuming a state profile.
    StateCheck,
}

/// Source occurrence and exact resolved closure, or its typed refusal.
#[derive(Debug)]
pub struct ProfileUse {
    /// Original unit that owns the local alias and token region.
    pub unit: UnitId,
    /// Original alias token (never a global profile identity).
    pub alias: Spanned<String>,
    /// Required interpretation at this occurrence.
    pub kind: UseKind,
    /// Root-first closure, retaining each selected interpretation exactly once.
    pub closure: Vec<RegisteredDefinition>,
    /// The first failed selection in this occurrence's closure.
    pub refusal: Option<Cause>,
    /// Whether all required definitions and rules were checked.
    pub complete: bool,
}

/// Every declaration's root and explicitly selected nested obligations.
#[derive(Debug)]
pub struct DeclarationProfiles {
    /// Namespace-local declaration identity.
    pub declaration: DeclarationId,
    /// Root first, then nested obligations in authored traversal order.
    pub uses: Vec<ProfileUse>,
    /// Whether all profile occurrences were collected and resolved.
    pub complete: bool,
}

/// Partial evidence survives budget exhaustion; it cannot construct a checker input.
#[derive(Debug)]
pub struct Report<'a> {
    /// Unchanged inputs, including unknown/unselected definitions.
    pub inventory: &'a Inventory<'a>,
    /// Checked edition closure, empty while unavailable.
    pub edition: Vec<RegisteredDefinition>,
    /// Edition failure applies to all declarations.
    pub edition_refusal: Option<Cause>,
    /// Per-declaration results in source namespace order.
    pub declarations: Vec<DeclarationProfiles>,
    /// First unaffordable operation; no later work ran in this stage.
    pub exhaustion: Option<Exhaustion>,
    /// Whether the stage finished, including all known refusals.
    pub complete: bool,
}

struct Catalog<'a> {
    input: &'a Inventory<'a>,
    definitions: BTreeMap<(&'a str, &'a str), Vec<usize>>,
    rules: BTreeMap<&'a str, Vec<usize>>,
    registered: Vec<Option<RegisteredDefinition>>,
    invalid_digest: BTreeSet<usize>,
}

/// Resolve the supplied edition and every authored profile occurrence.
///
/// Charging: every supplied definition is charged once in Definitions, every
/// rule once in Bindings, and every input byte/metadata occurrence once in Bytes.
/// Index probes, alias candidates and control/requirement inspections each cost
/// one Reference. Each selected closure charges every definition visit and rule
/// lookup as a Reference and each dependency traversal as one Edge; repeated
/// roots/shared dependencies are charged again per authored use. No cache waives
/// those charges. Report/occurrence records cost one Binding before allocation.
pub fn resolve<'a>(
    namespace: &SyntaxNamespace,
    inventory: &'a Inventory<'a>,
    work: &mut Work,
) -> Report<'a> {
    let mut report = Report {
        inventory,
        edition: Vec::new(),
        edition_refusal: None,
        declarations: Vec::new(),
        exhaustion: None,
        complete: false,
    };
    if let Err(exhaustion) = run(namespace, &mut report, work) {
        report.exhaustion = Some(exhaustion);
    } else {
        report.complete = true;
    }
    report
}

fn run(
    namespace: &SyntaxNamespace,
    report: &mut Report<'_>,
    work: &mut Work,
) -> Result<(), Exhaustion> {
    let catalog = Catalog::new(report.inventory, work)?;
    match catalog.closure(&report.inventory.edition, work)? {
        Ok(closure) if closure.first() == Some(&RegisteredDefinition::Edition) => {
            report.edition = closure;
        }
        Ok(_) => {
            report.edition_refusal = Some(Cause::WrongEdition(report.inventory.edition.clone()))
        }
        Err(cause) => report.edition_refusal = Some(cause),
    }
    for (index, entry) in namespace.declarations().iter().enumerate() {
        work.charge(Dimension::Bindings, 1)?;
        let id = DeclarationId(index);
        report.declarations.push(DeclarationProfiles {
            declaration: id,
            uses: Vec::new(),
            complete: false,
        });
        let output = report
            .declarations
            .last_mut()
            .expect("profile record inserted");
        let declaration = namespace.syntax(id).expect("closed namespace");
        let kind = match &declaration.kind {
            DeclarationKind::Predicate { .. } => UseKind::Predicate,
            DeclarationKind::State { .. } => UseKind::State,
            DeclarationKind::Temporal { .. } => UseKind::Temporal,
            DeclarationKind::Protocol(_) => UseKind::Protocol,
        };
        profile(
            namespace,
            &catalog,
            entry.unit(),
            &declaration.profile,
            kind,
            output,
            work,
        )?;
        if let DeclarationKind::Protocol(protocol) = &declaration.kind {
            for requirement in &protocol.requirements {
                work.charge(Dimension::References, 1)?;
                if let ProtocolRequirement::Compensation(compensation) = requirement {
                    profile(
                        namespace,
                        &catalog,
                        entry.unit(),
                        &compensation.profile,
                        UseKind::TimedObligation,
                        output,
                        work,
                    )?;
                }
            }
            // Visit only the owning declaration's contiguous arena region.
            let unit = namespace.unit(entry.unit()).expect("closed namespace unit");
            let controls = super::arena::owned(
                unit.controls(),
                declaration.span,
                |control| control.span,
                work,
            )?;
            for control in controls {
                work.charge(Dimension::References, 1)?;
                match &control.kind {
                    ControlKind::Await { profile: alias, .. } => profile(
                        namespace,
                        &catalog,
                        entry.unit(),
                        alias,
                        UseKind::TimedObligation,
                        output,
                        work,
                    )?,
                    ControlKind::Check { profile: alias, .. } => profile(
                        namespace,
                        &catalog,
                        entry.unit(),
                        alias,
                        UseKind::StateCheck,
                        output,
                        work,
                    )?,
                    ControlKind::Sequence(_)
                    | ControlKind::Choice { .. }
                    | ControlKind::Parallel { .. }
                    | ControlKind::Repeat { .. }
                    | ControlKind::Event(_)
                    | ControlKind::Commit { .. } => {}
                }
            }
        }
        output.complete = true;
    }
    Ok(())
}

fn profile(
    namespace: &SyntaxNamespace,
    catalog: &Catalog<'_>,
    unit: UnitId,
    alias: &Spanned<String>,
    kind: UseKind,
    output: &mut DeclarationProfiles,
    work: &mut Work,
) -> Result<(), Exhaustion> {
    work.charge(Dimension::Bindings, 1)?;
    output.uses.push(ProfileUse {
        unit,
        alias: alias.clone(),
        kind,
        closure: Vec::new(),
        refusal: None,
        complete: false,
    });
    let result = output.uses.last_mut().expect("profile occurrence inserted");
    let imports = namespace.unit(unit).expect("source unit").profiles();
    let mut candidates = Vec::new();
    for (index, import) in imports.iter().enumerate() {
        work.charge(Dimension::References, 1)?;
        if import.alias.value == alias.value {
            candidates.push(index);
        }
    }
    let import = match candidates.as_slice() {
        [] => {
            result.refusal = Some(Cause::MissingAlias);
            result.complete = true;
            return Ok(());
        }
        [index] => &imports[*index],
        _ => {
            result.refusal = Some(Cause::AmbiguousAlias {
                imports: candidates,
            });
            result.complete = true;
            return Ok(());
        }
    };
    let digest = match import.digest.value.parse() {
        Ok(digest) => digest,
        Err(_) => {
            result.refusal = Some(Cause::InvalidDigest {
                span: import.digest.span,
            });
            result.complete = true;
            return Ok(());
        }
    };
    let selection = Selection {
        identity: import.package.value.clone(),
        revision: import.version.value.clone(),
        digest,
    };
    match catalog.closure(&selection, work)? {
        Ok(closure) => {
            let root = closure[0];
            if !admits(root, kind) {
                result.refusal = Some(Cause::WrongProfileKind {
                    selected: root,
                    required: kind,
                });
            }
            result.closure = closure;
        }
        Err(cause) => result.refusal = Some(cause),
    }
    result.complete = true;
    Ok(())
}

fn admits(profile: RegisteredDefinition, kind: UseKind) -> bool {
    use RegisteredDefinition as R;
    match profile {
        R::StateCore => matches!(kind, UseKind::State | UseKind::StateCheck),
        R::StateQueries | R::StateGraph => matches!(
            kind,
            UseKind::Predicate | UseKind::State | UseKind::StateCheck
        ),
        R::EventPosition | R::FixedSample | R::TimestampedWindow => {
            matches!(kind, UseKind::Temporal | UseKind::TimedObligation)
        }
        R::Protocol => kind == UseKind::Protocol,
        R::Edition
        | R::TemporalFacet
        | R::ObservationBinding
        | R::Progress
        | R::Range
        | R::Package
        | R::Diagnostics => false,
    }
}

impl<'a> Catalog<'a> {
    fn new(input: &'a Inventory<'a>, work: &mut Work) -> Result<Self, Exhaustion> {
        for text in [&input.edition.identity, &input.edition.revision] {
            work.charge(Dimension::Bytes, text.len())?;
        }
        let mut catalog = Self {
            input,
            definitions: BTreeMap::new(),
            rules: BTreeMap::new(),
            registered: Vec::new(),
            invalid_digest: BTreeSet::new(),
        };
        for (index, artifact) in input.definitions.iter().enumerate() {
            work.charge(Dimension::Definitions, 1)?;
            for bytes in [
                artifact.selection.identity.as_bytes(),
                artifact.selection.revision.as_bytes(),
                artifact.bytes,
            ] {
                work.charge(Dimension::Bytes, bytes.len())?;
            }
            catalog
                .definitions
                .entry((&artifact.selection.identity, &artifact.selection.revision))
                .or_default()
                .push(index);
            if ByteDigest::of(artifact.bytes) != artifact.selection.digest {
                catalog.invalid_digest.insert(index);
            }
            let mut registered = None;
            for candidate in RegisteredDefinition::all() {
                work.charge(Dimension::References, 1)?;
                if candidate.identity() == artifact.selection.identity
                    && candidate.revision() == artifact.selection.revision
                    && candidate.bytes() == artifact.bytes
                {
                    registered = Some(*candidate);
                    break;
                }
            }
            catalog.registered.push(registered);
        }
        for (index, rule) in input.rules.iter().enumerate() {
            work.charge(Dimension::Bindings, 1)?;
            work.charge(Dimension::Bytes, rule.path.len())?;
            work.charge(Dimension::Bytes, rule.bytes.len())?;
            catalog.rules.entry(rule.path).or_default().push(index);
        }
        Ok(catalog)
    }

    fn select(
        &self,
        selection: &Selection,
        work: &mut Work,
    ) -> Result<Result<RegisteredDefinition, Cause>, Exhaustion> {
        work.charge(Dimension::References, 1)?;
        let entries = self
            .definitions
            .get(&(selection.identity.as_str(), selection.revision.as_str()));
        let index = match entries.map(Vec::as_slice) {
            None | Some([]) => return Ok(Err(Cause::MissingDefinition(selection.clone()))),
            Some([index]) => *index,
            Some(entries) => {
                return Ok(Err(Cause::AmbiguousDefinition {
                    selection: selection.clone(),
                    entries: entries.to_vec(),
                }))
            }
        };
        let supplied = &self.input.definitions[index];
        if self.invalid_digest.contains(&index) {
            return Ok(Err(Cause::DefinitionDigest { entry: index }));
        }
        if supplied.selection != *selection {
            return Ok(Err(Cause::SelectionMismatch {
                expected: selection.clone(),
                entry: index,
            }));
        }
        Ok(self.registered[index].ok_or(Cause::UnsupportedDefinition { entry: index }))
    }

    fn closure(
        &self,
        selection: &Selection,
        work: &mut Work,
    ) -> Result<Result<Vec<RegisteredDefinition>, Cause>, Exhaustion> {
        let root = match self.select(selection, work)? {
            Ok(root) => root,
            Err(cause) => return Ok(Err(cause)),
        };
        let mut result = Vec::new();
        let mut seen = BTreeSet::new();
        let mut active = BTreeSet::new();
        let mut identities = BTreeMap::<&str, RegisteredDefinition>::new();
        let mut stack = vec![(root, false)];
        // The current closed registry is acyclic with one revision per identity.
        // Retain these defensive refusals for explicit future registry changes;
        // caller-supplied metadata cannot manufacture either condition today.
        while let Some((definition, leaving)) = stack.pop() {
            work.charge(Dimension::References, 1)?;
            if leaving {
                active.remove(&definition);
                continue;
            }
            if active.contains(&definition) {
                return Ok(Err(Cause::DefinitionCycle {
                    definition: definition.selection(),
                }));
            }
            if !seen.insert(definition) {
                continue;
            }
            if let Some(previous) = identities.insert(definition.identity(), definition) {
                if previous != definition {
                    return Ok(Err(Cause::IncompatibleRequirement {
                        first: previous.selection(),
                        second: definition.selection(),
                    }));
                }
            }
            active.insert(definition);
            result.push(definition);
            stack.push((definition, true));
            for rule in definition.rules() {
                work.charge(Dimension::References, 1)?;
                let entries = self.rules.get(rule.path);
                let index = match entries.map(Vec::as_slice) {
                    None | Some([]) => return Ok(Err(Cause::MissingRule { path: rule.path })),
                    Some([index]) => *index,
                    Some(entries) => {
                        return Ok(Err(Cause::AmbiguousRule {
                            path: rule.path,
                            entries: entries.to_vec(),
                        }))
                    }
                };
                let supplied = &self.input.rules[index];
                if supplied.bytes != rule.bytes || supplied.digest != ByteDigest::of(supplied.bytes)
                {
                    return Ok(Err(Cause::RuleMismatch {
                        path: rule.path,
                        entry: index,
                    }));
                }
            }
            for dependency in definition.requirements().iter().rev() {
                work.charge(Dimension::Edges, 1)?;
                // Exact dependency selections come from compiler-owned registry
                // metadata, never arbitrary annotations supplied with a document.
                match self.select(&dependency.selection(), work)? {
                    Ok(selected) => stack.push((selected, false)),
                    Err(cause) => return Ok(Err(cause)),
                }
            }
        }
        Ok(Ok(result))
    }
}
