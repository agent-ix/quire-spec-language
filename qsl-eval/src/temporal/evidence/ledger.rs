// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-524, FR-526: the caller-held ledger of retained instances and latest
//! results, in the way FR-502's `ProgressLedger` keeps progress.
//!
//! The ledger holds flat entries and no result bytes. Only the evaluator
//! writes an entry: the entry types have no public constructor, and an
//! operation returns an updated ledger and leaves the one it was called on
//! as it was.

use std::collections::BTreeMap;

use quire_exact::ValueType;

use super::identity::{InputIdentity, ResultIdentity};
use super::trigger::{CapturedValue, Payload, Receipt, SemanticTriggerIdentity};

/// The obligation an instance or a result belongs to: the checked package
/// identity and the clause occurrence (FR-524 "The evidence ledger"). Two
/// clauses of one body and different names share a declaration node and are
/// two occurrences.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObligationKey {
    package: Box<str>,
    clause: Box<[u8]>,
    occurrence: u64,
}

impl ObligationKey {
    /// The obligation of `clause`, the encoded identity of the clause's
    /// declaration node, in `package`.
    pub fn new(
        package: impl Into<Box<str>>,
        clause: impl Into<Box<[u8]>>,
        occurrence: u64,
    ) -> Self {
        Self {
            package: package.into(),
            clause: clause.into(),
            occurrence,
        }
    }

    /// The package identity.
    pub fn package(&self) -> &str {
        &self.package
    }

    /// The declaration's occurrence ordinal.
    pub const fn occurrence(&self) -> u64 {
        self.occurrence
    }

    /// The encoded clause identity.
    pub fn clause(&self) -> &[u8] {
        &self.clause
    }
}

/// Where a capture's initializer sits: the clause and its index in authored
/// order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceBinding {
    /// The clause's declared name.
    pub clause: Box<str>,
    /// The initializer's index in authored order.
    pub index: usize,
}

/// One established capture (FR-524 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureEntry {
    name: Box<str>,
    declared_type: ValueType,
    value: CapturedValue,
    source: SourceBinding,
    anchor: Box<str>,
}

impl CaptureEntry {
    pub(super) fn new(
        name: &str,
        declared_type: ValueType,
        value: CapturedValue,
        source: SourceBinding,
        anchor: &str,
    ) -> Self {
        Self {
            name: name.into(),
            declared_type,
            value,
            source,
            anchor: anchor.into(),
        }
    }

    /// The declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The declared type.
    pub fn declared_type(&self) -> &ValueType {
        &self.declared_type
    }

    /// The value, as established.
    pub fn value(&self) -> &CapturedValue {
        &self.value
    }

    /// The initializer the value came from.
    pub fn source(&self) -> &SourceBinding {
        &self.source
    }

    /// The identity of the position the value was read at.
    pub fn anchor(&self) -> &str {
        &self.anchor
    }
}

/// The captures of one instance, in authored order. It is built once, when
/// the instance is created, and has no mutator: a later delivery or
/// observation cannot change it (FR-524).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CaptureEnvironment {
    entries: Vec<CaptureEntry>,
}

impl CaptureEnvironment {
    pub(super) fn new(entries: Vec<CaptureEntry>) -> Self {
        Self { entries }
    }

    /// The captures, in authored order.
    pub fn entries(&self) -> &[CaptureEntry] {
        &self.entries
    }

    /// The capture named `name`.
    pub fn get(&self, name: &str) -> Option<&CaptureEntry> {
        self.entries.iter().find(|entry| entry.name() == name)
    }
}

/// An instance the ledger retains (FR-524 "The evidence ledger").
///
/// Its fields are private and it has no public constructor, so a caller can
/// read an entry and cannot make or change one:
///
/// ```compile_fail,E0451
/// use qsl_eval::temporal::evidence::RetainedInstance;
/// fn forge() -> RetainedInstance {
///     RetainedInstance { evaluations: 0 }
/// }
/// ```
///
/// ```compile_fail,E0599
/// use qsl_eval::temporal::evidence::RetainedInstance;
/// fn forge() -> RetainedInstance {
///     RetainedInstance::new()
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RetainedInstance {
    payload: Payload,
    receipts: Vec<Receipt>,
    environment: CaptureEnvironment,
    evaluations: u64,
}

impl RetainedInstance {
    pub(super) fn created(
        payload: Payload,
        receipt: Option<Receipt>,
        env: CaptureEnvironment,
    ) -> Self {
        Self {
            payload,
            receipts: receipt.into_iter().collect(),
            environment: env,
            evaluations: 1,
        }
    }

    pub(super) fn with_receipt(&self, receipt: &Receipt) -> Self {
        let mut next = self.clone();
        next.receipts.push(receipt.clone());
        next
    }

    /// The payload the first delivery carried.
    pub fn payload(&self) -> &Payload {
        &self.payload
    }

    /// The receipts of every delivery of the instance, in arrival order.
    pub fn receipts(&self) -> &[Receipt] {
        &self.receipts
    }

    /// The captures, as established when the instance was created.
    pub fn environment(&self) -> &CaptureEnvironment {
        &self.environment
    }

    /// How many times the instance's initializers were evaluated: one, for
    /// the life of the instance.
    pub fn evaluations(&self) -> u64 {
        self.evaluations
    }
}

/// The latest result of one obligation instance (FR-526 "Semantic authority
/// and boundary"): flat identities, no chain and no result bytes.
///
/// ```compile_fail,E0451
/// use qsl_eval::temporal::evidence::ResultEntry;
/// fn forge(entry: ResultEntry) -> u64 {
///     ResultEntry { lineage: 7, ..entry }.lineage()
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResultEntry {
    result_identity: ResultIdentity,
    input_identity: InputIdentity,
    correspondence: Box<[u8]>,
    lineage: u64,
}

impl ResultEntry {
    pub(super) fn new(
        result_identity: ResultIdentity,
        input_identity: InputIdentity,
        correspondence: Box<[u8]>,
        lineage: u64,
    ) -> Self {
        Self {
            result_identity,
            input_identity,
            correspondence,
            lineage,
        }
    }

    /// The result's identity.
    pub fn result_identity(&self) -> &ResultIdentity {
        &self.result_identity
    }

    /// The identity over the result's evidence members.
    pub fn input_identity(&self) -> &InputIdentity {
        &self.input_identity
    }

    /// The encoded correspondence the result was computed under.
    pub fn correspondence(&self) -> &[u8] {
        &self.correspondence
    }

    /// The result's lineage number.
    pub fn lineage(&self) -> u64 {
        self.lineage
    }
}

/// The key of one obligation instance: the obligation and the trigger
/// identity (`None` for a call that assessed no instance).
type InstanceKey = (ObligationKey, Option<SemanticTriggerIdentity>);

/// The caller-held ledger of retained instances (FR-524) and latest results
/// (FR-526).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EvidenceLedger {
    instances: BTreeMap<InstanceKey, RetainedInstance>,
    results: BTreeMap<InstanceKey, ResultEntry>,
}

impl EvidenceLedger {
    /// An empty ledger.
    pub fn new() -> Self {
        Self::default()
    }

    /// The instance retained for `trigger` of `obligation`.
    pub fn instance(
        &self,
        obligation: &ObligationKey,
        trigger: &SemanticTriggerIdentity,
    ) -> Option<&RetainedInstance> {
        self.instances
            .get(&(obligation.clone(), Some(trigger.clone())))
    }

    /// Every instance retained for `obligation`, in identity order.
    pub fn instances_of<'a>(
        &'a self,
        obligation: &'a ObligationKey,
    ) -> impl Iterator<Item = (&'a SemanticTriggerIdentity, &'a RetainedInstance)> + 'a {
        self.instances
            .iter()
            .filter(move |((key, _), _)| key == obligation)
            .filter_map(|((_, trigger), entry)| trigger.as_ref().map(|trigger| (trigger, entry)))
    }

    /// The latest result of the obligation instance.
    pub fn result(
        &self,
        obligation: &ObligationKey,
        trigger: Option<&SemanticTriggerIdentity>,
    ) -> Option<&ResultEntry> {
        self.results.get(&(obligation.clone(), trigger.cloned()))
    }

    /// How many instances the ledger retains.
    pub fn instance_count(&self) -> usize {
        self.instances.len()
    }

    /// How many obligation instances have a latest result.
    pub fn result_count(&self) -> usize {
        self.results.len()
    }

    pub(super) fn with_instance(
        mut self,
        obligation: &ObligationKey,
        trigger: &SemanticTriggerIdentity,
        entry: RetainedInstance,
    ) -> Self {
        self.instances
            .insert((obligation.clone(), Some(trigger.clone())), entry);
        self
    }

    pub(super) fn with_result(
        mut self,
        obligation: &ObligationKey,
        trigger: Option<&SemanticTriggerIdentity>,
        entry: ResultEntry,
    ) -> Self {
        self.results
            .insert((obligation.clone(), trigger.cloned()), entry);
        self
    }
}
