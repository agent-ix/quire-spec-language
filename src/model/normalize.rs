// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 model normalization: phases 1 (decode), 2 (qualify), 3 (inherit)
//! and 5 (canonicalize).
//!
//! Phase 4 (`quire.model.normalize.subset/v1` and
//! `quire.model.normalize.redefine/v1`: explicit subsetting, redefinition
//! and their conflict/redefinition-target checks, TC-195 N06) is not
//! implemented in this rung: [`crate::model::bundle`] carries no subsetting
//! or redefinition records, and `ChargePoint` has no
//! `normalize.redefinition-check`/`normalize.conflict-check`. This is a
//! scope decision reported in the delivering PR, not a silent gap: a bundle
//! with a redefinition or subsetting record is simply not representable
//! yet, so nothing here can silently approximate one.
//!
//! This engine takes a [`Bundle`] value the caller constructs; it holds no
//! ambient registry. Every identity is SHA-256 over RFC 8785 JCS bytes of a
//! preimage built here, replayed exactly for the same [`Bundle`] and
//! [`ModelNormalizationLimits`] — the same inputs always retrace the same
//! derivation and the same bytes.
//!
//! Construction and accounting are deliberately two passes. Pass one (this
//! module's `build`) computes the complete normalization unconditionally,
//! refusing outright on a real defect (an unsupported interface version, a
//! generalization cycle, or a path deeper than [`MAX_GENERALIZATION_DEPTH`]).
//! Pass two (`charge_all`) replays the exact `ModelNormalizationLimitsV1`
//! charge sequence FR-150 defines over the already-built result and reports
//! [`crate::model::accounting::Incomplete`] at the first unavailable
//! counter, exposing no effective view for that run — matching TC-195 N01's
//! boundary vectors exactly. This keeps the accounting replay simple and
//! correct for the bundle sizes this rung admits, at the cost of not
//! bounding pass one's own memory use against an adversarial bundle; that
//! tradeoff is recorded here rather than left implicit.

use std::collections::HashMap;

use crate::diagnostic::Code;
use crate::model::accounting::{
    Charge, ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use crate::model::bundle::{Bundle, BundleRecord, ModelSelection, INTERFACE_VERSION_1_3_0};
use crate::model::key::{
    self, digest_of, jcs_bytes, EffectiveDeclarationPreimage, EffectiveId, Fact, ProducerKey,
    PRODUCER_DIGEST_DOMAIN, RULE_INHERIT, RULE_QUALIFY,
};

/// A refusal FR-150 normalization returns for a real defect (never a
/// resource limit; see [`Incomplete`] for that).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRefusal {
    /// The stable top-level code.
    pub code: Code,
    /// The FR-150-specific cause tag.
    pub cause: &'static str,
    /// A human-readable detail naming the offending declaration(s).
    pub detail: String,
}

/// The outcome of one normalization attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizeOutcome {
    /// Normalization completed within the given limits.
    Completed(EffectiveView),
    /// A real defect refused normalization outright; no effective view.
    Refused(ModelRefusal),
    /// A `ModelNormalizationLimitsV1` counter was exhausted; no effective view.
    Incomplete(Incomplete),
}

/// One entry of an [`EffectiveView`]: `{effective_id, preimage}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewEntry {
    /// This declaration's computed identity.
    pub effective_id: EffectiveId,
    /// The preimage that identity was computed from.
    pub preimage: EffectiveDeclarationPreimage,
}

/// A `quire.model.effective-view/v1` result: every effective declaration a
/// [`ModelSelection`] admits, sorted ascending by effective identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveView {
    /// The model selection this view was normalized under.
    pub model_selection: ModelSelection,
    /// Every admitted declaration, ascending by [`EffectiveId`].
    pub declarations: Vec<ViewEntry>,
}

impl EffectiveView {
    fn to_json(&self) -> serde_json::Value {
        use serde_json::{Map, Value};
        let mut object = Map::new();
        object.insert(
            "version".to_owned(),
            Value::String(key::EFFECTIVE_VIEW_DOMAIN.to_owned()),
        );
        object.insert("model_selection".to_owned(), self.model_selection.to_json());
        object.insert(
            "rules".to_owned(),
            Value::Object({
                let mut rules = Map::new();
                rules.insert(
                    "identity".to_owned(),
                    Value::String(key::RULES_IDENTITY.to_owned()),
                );
                rules.insert(
                    "revision".to_owned(),
                    Value::String(key::RULES_REVISION.to_owned()),
                );
                rules
            }),
        );
        object.insert(
            "declarations".to_owned(),
            Value::Array(
                self.declarations
                    .iter()
                    .map(|entry| {
                        let mut e = Map::new();
                        e.insert("effective_id".to_owned(), entry.effective_id.to_json());
                        e.insert("preimage".to_owned(), preimage_to_json(&entry.preimage));
                        Value::Object(e)
                    })
                    .collect(),
            ),
        );
        Value::Object(object)
    }

    /// This view's `quire.model.effective-view/v1` identity.
    pub fn identity(&self) -> EffectiveId {
        digest_of(&self.to_json())
    }

    fn jcs_len(&self) -> u64 {
        jcs_bytes(&self.to_json()).len() as u64
    }
}

/// A `quire.model.object-universe/v1` result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectUniverse {
    /// The model selection this universe was normalized under.
    pub model_selection: ModelSelection,
    /// Every root effective type (no generalization ancestor), ascending by
    /// [`EffectiveId`].
    pub root_types: Vec<EffectiveId>,
}

impl ObjectUniverse {
    fn to_json(&self) -> serde_json::Value {
        use serde_json::{Map, Value};
        let mut object = Map::new();
        object.insert(
            "version".to_owned(),
            Value::String(key::OBJECT_UNIVERSE_DOMAIN.to_owned()),
        );
        object.insert("model_selection".to_owned(), self.model_selection.to_json());
        object.insert(
            "root_types".to_owned(),
            Value::Array(self.root_types.iter().map(EffectiveId::to_json).collect()),
        );
        Value::Object(object)
    }

    /// This universe's `quire.model.object-universe/v1` identity.
    pub fn identity(&self) -> EffectiveId {
        digest_of(&self.to_json())
    }

    fn jcs_len(&self) -> u64 {
        jcs_bytes(&self.to_json()).len() as u64
    }
}

fn preimage_to_json(preimage: &EffectiveDeclarationPreimage) -> serde_json::Value {
    // `EffectiveDeclarationPreimage::to_json` is crate-private in `key.rs`;
    // its own `identity`/`jcs_bytes` methods already round-trip the exact
    // bytes we need, so reuse `identity`'s hashed form is avoided here and
    // we instead ask for the bytes directly.
    serde_json::from_slice(&preimage.jcs_bytes()).expect("preimage JCS bytes are valid JSON")
}

/// A generalization or field-member producer record is not visited more
/// than this many times in one ancestor path before normalization refuses
/// rather than recurse without bound.
pub const MAX_GENERALIZATION_DEPTH: usize = 128;

struct Index {
    types: HashMap<String, ProducerKey>,
    fields_by_owner: HashMap<String, Vec<crate::model::bundle::FieldMemberRecord>>,
    generals_by_specific: HashMap<String, Vec<crate::model::bundle::GeneralizationRecord>>,
    /// Every type that is some record's `specific`, i.e. not a root.
    non_root: std::collections::HashSet<String>,
}

impl Index {
    fn build(bundle: &Bundle) -> Self {
        let mut types = HashMap::new();
        let mut fields_by_owner: HashMap<String, Vec<_>> = HashMap::new();
        let mut generals_by_specific: HashMap<String, Vec<_>> = HashMap::new();
        let mut non_root = std::collections::HashSet::new();
        for record in &bundle.records {
            match record {
                BundleRecord::ObjectType(t) => {
                    types.insert(t.key.identity.clone(), t.key.clone());
                }
                BundleRecord::FieldMember(m) => {
                    fields_by_owner
                        .entry(m.owner.identity.clone())
                        .or_default()
                        .push(m.clone());
                }
                BundleRecord::Generalization(g) => {
                    non_root.insert(g.specific.identity.clone());
                    generals_by_specific
                        .entry(g.specific.identity.clone())
                        .or_default()
                        .push(g.clone());
                }
            }
        }
        Self {
            types,
            fields_by_owner,
            generals_by_specific,
            non_root,
        }
    }

    fn sorted_type_keys(&self) -> Vec<ProducerKey> {
        let mut keys: Vec<ProducerKey> = self.types.values().cloned().collect();
        keys.sort();
        keys
    }

    fn sorted_direct_members(
        &self,
        owner_identity: &str,
    ) -> Vec<crate::model::bundle::FieldMemberRecord> {
        let mut members = self
            .fields_by_owner
            .get(owner_identity)
            .cloned()
            .unwrap_or_default();
        members.sort_by(|a, b| a.key.cmp(&b.key));
        members
    }

    fn sorted_generals(
        &self,
        specific_identity: &str,
    ) -> Vec<crate::model::bundle::GeneralizationRecord> {
        let mut generals = self
            .generals_by_specific
            .get(specific_identity)
            .cloned()
            .unwrap_or_default();
        generals.sort_by(|a, b| a.key.cmp(&b.key));
        generals
    }
}

/// One path from a type to a strict ancestor: the ordered chain of
/// generalization-record keys taken, and the ancestor's own identity.
struct AncestorPath {
    path: Vec<ProducerKey>,
    ancestor_identity: String,
}

/// Every ancestor path of `root_identity`, in DFS pre-order over
/// ascending-key direct generalizations. Explicit stack, not native
/// recursion: bundle data is caller-supplied and may describe a cycle.
fn ancestor_paths(root_identity: &str, index: &Index) -> Result<Vec<AncestorPath>, ModelRefusal> {
    struct Frame {
        directs: Vec<crate::model::bundle::GeneralizationRecord>,
        next: usize,
        path: Vec<ProducerKey>,
        visited: Vec<String>,
    }

    let mut stack = vec![Frame {
        directs: index.sorted_generals(root_identity),
        next: 0,
        path: Vec::new(),
        visited: vec![root_identity.to_owned()],
    }];
    let mut out = Vec::new();
    loop {
        let stack_len = stack.len();
        let Some(frame) = stack.last_mut() else { break };
        if frame.next >= frame.directs.len() {
            stack.pop();
            continue;
        }
        if stack_len >= MAX_GENERALIZATION_DEPTH {
            return Err(ModelRefusal {
                code: Code::ResourceExhausted,
                cause: "generalization-depth-exceeded",
                detail: format!(
                    "ancestor path from {root_identity} exceeds {MAX_GENERALIZATION_DEPTH} generalization records"
                ),
            });
        }
        let record = frame.directs[frame.next].clone();
        frame.next += 1;
        let mut new_path = frame.path.clone();
        new_path.push(record.key.clone());
        let ancestor_identity = record.general.identity.clone();
        if frame.visited.contains(&ancestor_identity) {
            return Err(ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: "specialization-cycle",
                detail: format!(
                    "{ancestor_identity} generalizes back to itself via {}",
                    record.key.identity
                ),
            });
        }
        out.push(AncestorPath {
            path: new_path.clone(),
            ancestor_identity: ancestor_identity.clone(),
        });
        let mut new_visited = frame.visited.clone();
        new_visited.push(ancestor_identity.clone());
        stack.push(Frame {
            directs: index.sorted_generals(&ancestor_identity),
            next: 0,
            path: new_path,
            visited: new_visited,
        });
    }
    Ok(out)
}

/// One fact awaiting replayed accounting, tagged with what it charges.
#[derive(Clone)]
struct PendingFact {
    owner_key: Option<ProducerKey>,
    declared_key: ProducerKey,
    inputs: Vec<ProducerKey>,
    /// `Some(path_len)` for a phase-3 type-level fact (charges
    /// `normalize.cycle-check` first); `None` otherwise.
    cycle_check_len: Option<usize>,
}

/// One declaration awaiting replayed `normalize.declaration`/`normalize.hash`,
/// already in final charge order (see `build`'s declaration assembly).
struct PendingDeclaration {
    jcs_len: u64,
}

struct Built {
    phase2_facts: Vec<PendingFact>,
    phase3_facts: Vec<PendingFact>,
    declarations: Vec<PendingDeclaration>,
    view: EffectiveView,
    universe: ObjectUniverse,
}

/// Pass one: build the complete normalization unconditionally, refusing
/// outright on a real defect. See the module docs for why this is
/// unconstrained by [`ModelNormalizationLimits`].
fn build(bundle: &Bundle) -> Result<Built, ModelRefusal> {
    let index = Index::build(bundle);
    let type_keys = index.sorted_type_keys();

    let mut phase2_facts = Vec::new();
    let mut phase3_facts = Vec::new();
    let mut type_preimages: HashMap<String, EffectiveDeclarationPreimage> = HashMap::new();
    let mut type_effective_ids: HashMap<String, EffectiveId> = HashMap::new();

    for type_key in &type_keys {
        phase2_facts.push(PendingFact {
            owner_key: None,
            declared_key: type_key.clone(),
            inputs: vec![type_key.clone()],
            cycle_check_len: None,
        });
        let mut derivation = vec![Fact {
            ordinal: 0,
            rule: RULE_QUALIFY,
            inputs: vec![type_key.clone()],
        }];

        let paths = ancestor_paths(&type_key.identity, &index)?;
        for ancestor in &paths {
            let ancestor_key = index
                .types
                .get(&ancestor.ancestor_identity)
                .expect("every generalization general is a declared object type")
                .clone();
            let mut inputs = ancestor.path.clone();
            inputs.push(ancestor_key);
            phase3_facts.push(PendingFact {
                owner_key: None,
                declared_key: type_key.clone(),
                inputs: inputs.clone(),
                cycle_check_len: Some(ancestor.path.len()),
            });
            derivation.push(Fact {
                ordinal: derivation.len(),
                rule: RULE_INHERIT,
                inputs,
            });
        }

        let preimage = EffectiveDeclarationPreimage {
            owner_effective_type: None,
            original: type_key.clone(),
            derivation,
        };
        type_effective_ids.insert(type_key.identity.clone(), preimage.identity());
        type_preimages.insert(type_key.identity.clone(), preimage);
    }

    let mut member_preimages: HashMap<(String, String), EffectiveDeclarationPreimage> =
        HashMap::new();

    for type_key in &type_keys {
        let owner_effective_id = type_effective_ids[&type_key.identity].clone();

        // Phase 2: members declared directly on this type.
        for member in index.sorted_direct_members(&type_key.identity) {
            phase2_facts.push(PendingFact {
                owner_key: Some(type_key.clone()),
                declared_key: member.key.clone(),
                inputs: vec![member.key.clone()],
                cycle_check_len: None,
            });
            member_preimages.insert(
                (type_key.identity.clone(), member.key.identity.clone()),
                EffectiveDeclarationPreimage {
                    owner_effective_type: Some(owner_effective_id.clone()),
                    original: member.key.clone(),
                    derivation: vec![Fact {
                        ordinal: 0,
                        rule: RULE_QUALIFY,
                        inputs: vec![member.key.clone()],
                    }],
                },
            );
        }

        // Phase 3: members inherited along this type's ancestor paths, from
        // each ancestor's own directly-declared members.
        let paths = ancestor_paths(&type_key.identity, &index)?;
        for ancestor in &paths {
            for member in index.sorted_direct_members(&ancestor.ancestor_identity) {
                let mut inputs = ancestor.path.clone();
                inputs.push(member.key.clone());
                phase3_facts.push(PendingFact {
                    owner_key: Some(type_key.clone()),
                    declared_key: member.key.clone(),
                    inputs: inputs.clone(),
                    cycle_check_len: None,
                });
                let entry = member_preimages
                    .entry((type_key.identity.clone(), member.key.identity.clone()))
                    .or_insert_with(|| EffectiveDeclarationPreimage {
                        owner_effective_type: Some(owner_effective_id.clone()),
                        original: member.key.clone(),
                        derivation: Vec::new(),
                    });
                let ordinal = entry.derivation.len();
                entry.derivation.push(Fact {
                    ordinal,
                    rule: RULE_INHERIT,
                    inputs,
                });
            }
        }
    }

    // Phase 5 (identities only; charging is replayed separately).
    let mut declarations: Vec<PendingDeclaration> = Vec::new();
    let mut entries: Vec<ViewEntry> = Vec::new();
    for type_key in &type_keys {
        let preimage = type_preimages
            .remove(&type_key.identity)
            .expect("built above");
        let effective_id = type_effective_ids[&type_key.identity].clone();
        declarations.push(PendingDeclaration {
            jcs_len: preimage.jcs_bytes().len() as u64,
        });
        entries.push(ViewEntry {
            effective_id,
            preimage,
        });
    }
    let mut member_keys: Vec<(String, String)> = member_preimages.keys().cloned().collect();
    member_keys.sort_by(|(owner_a, decl_a), (owner_b, decl_b)| {
        index.types[owner_a]
            .cmp(&index.types[owner_b])
            .then_with(|| decl_a.cmp(decl_b))
    });
    for (owner_identity, decl_identity) in member_keys {
        let preimage = member_preimages
            .remove(&(owner_identity.clone(), decl_identity))
            .expect("built above");
        let effective_id = preimage.identity();
        declarations.push(PendingDeclaration {
            jcs_len: preimage.jcs_bytes().len() as u64,
        });
        entries.push(ViewEntry {
            effective_id,
            preimage,
        });
    }
    entries.sort_by(|a, b| a.effective_id.cmp(&b.effective_id));

    let mut root_types: Vec<EffectiveId> = type_keys
        .iter()
        .filter(|key| !index.non_root.contains(&key.identity))
        .map(|key| type_effective_ids[&key.identity].clone())
        .collect();
    root_types.sort();

    let view = EffectiveView {
        model_selection: bundle.model_selection.clone(),
        declarations: entries,
    };
    let universe = ObjectUniverse {
        model_selection: bundle.model_selection.clone(),
        root_types,
    };

    Ok(Built {
        phase2_facts,
        phase3_facts,
        declarations,
        view,
        universe,
    })
}

fn sort_facts(facts: &mut [PendingFact]) {
    facts.sort_by(|a, b| {
        a.owner_key
            .cmp(&b.owner_key)
            .then_with(|| a.declared_key.cmp(&b.declared_key))
            .then_with(|| a.inputs.cmp(&b.inputs))
    });
}

/// Pass two: replay the exact `ModelNormalizationLimitsV1` charge sequence
/// over an already-built [`Built`] result.
fn charge_all(bundle: &Bundle, built: &Built, meter: &mut Meter) -> Result<(), Incomplete> {
    let mut record_keys: Vec<ProducerKey> = bundle
        .records
        .iter()
        .map(BundleRecord::key)
        .cloned()
        .collect();
    record_keys.sort();
    for (index, _) in record_keys.iter().enumerate() {
        meter.charge(
            Charge::new(ChargePoint::NormalizeRecord)
                .size(LimitKind::ProducerRecords, (index + 1) as u64),
        )?;
    }

    let mut fact_count: u64 = 0;
    let mut phase2_owned: Vec<PendingFact> = built.phase2_facts.clone();
    sort_facts(&mut phase2_owned);
    for _fact in &phase2_owned {
        fact_count += 1;
        meter.charge(
            Charge::new(ChargePoint::NormalizeFact).size(LimitKind::DerivationFacts, fact_count),
        )?;
    }

    let mut phase3_owned: Vec<PendingFact> = built.phase3_facts.clone();
    sort_facts(&mut phase3_owned);
    for fact in &phase3_owned {
        if let Some(path_len) = fact.cycle_check_len {
            meter.charge(Charge::new(ChargePoint::NormalizeCycleCheck).work(path_len as u64))?;
        }
        fact_count += 1;
        meter.charge(
            Charge::new(ChargePoint::NormalizeFact).size(LimitKind::DerivationFacts, fact_count),
        )?;
    }

    let mut decl_count: u64 = 0;
    for declaration in &built.declarations {
        decl_count += 1;
        meter.charge(
            Charge::new(ChargePoint::NormalizeDeclaration)
                .size(LimitKind::EffectiveDeclarations, decl_count),
        )?;
        meter.charge(
            Charge::new(ChargePoint::NormalizeHash)
                .size(LimitKind::HashedBytes, declaration.jcs_len),
        )?;
    }

    meter.charge(
        Charge::new(ChargePoint::NormalizeHash)
            .size(LimitKind::HashedBytes, built.universe.jcs_len()),
    )?;
    meter.charge(
        Charge::new(ChargePoint::NormalizeHash).size(LimitKind::HashedBytes, built.view.jcs_len()),
    )?;
    Ok(())
}

/// Every producer key `record` itself declares or refers to.
fn referenced_keys(record: &BundleRecord) -> Vec<&ProducerKey> {
    match record {
        BundleRecord::ObjectType(record) => vec![&record.key],
        BundleRecord::FieldMember(record) => vec![&record.key, &record.owner, &record.value_type],
        BundleRecord::Generalization(record) => {
            vec![&record.key, &record.specific, &record.general]
        }
    }
}

/// Phase-1 decode checks common to every entry point: the claimed producer
/// interface version (TC-195 N08) and every referenced key's digest domain
/// (TC-195 N05). Both refuse before any charge; no effective view is exposed.
fn decode_check(bundle: &Bundle) -> Result<(), ModelRefusal> {
    if bundle.model_selection.contract_version.interface_version != INTERFACE_VERSION_1_3_0 {
        return Err(ModelRefusal {
            code: Code::UnknownWire,
            cause: "unsupported-wire",
            detail: format!(
                "producer interface {} is not supported; only {INTERFACE_VERSION_1_3_0} is implemented",
                bundle.model_selection.contract_version.interface_version
            ),
        });
    }
    for record in &bundle.records {
        for key in referenced_keys(record) {
            if key.digest.domain != PRODUCER_DIGEST_DOMAIN {
                return Err(ModelRefusal {
                    code: Code::StaleDependency,
                    cause: "digest-domain-mismatch",
                    detail: format!(
                        "{} digest domain is {}; expected {PRODUCER_DIGEST_DOMAIN}",
                        key.identity, key.digest.domain
                    ),
                });
            }
        }
    }
    Ok(())
}

/// Normalize `bundle` under `limits`: FR-150 phases 1, 2, 3 and 5.
pub fn normalize(bundle: &Bundle, limits: ModelNormalizationLimits) -> NormalizeOutcome {
    if let Err(refusal) = decode_check(bundle) {
        return NormalizeOutcome::Refused(refusal);
    }
    let built = match build(bundle) {
        Ok(built) => built,
        Err(refusal) => return NormalizeOutcome::Refused(refusal),
    };
    let mut meter = Meter::new(limits);
    match charge_all(bundle, &built, &mut meter) {
        Ok(()) => NormalizeOutcome::Completed(built.view),
        Err(incomplete) => NormalizeOutcome::Incomplete(incomplete),
    }
}

/// A meter constructed to run `bundle` under [`ModelNormalizationLimits::UNLIMITED`]
/// and expose its admitted-charge sequence, for tests that assert the exact
/// charge order alongside [`normalize`]'s result.
pub fn normalize_with_meter(
    bundle: &Bundle,
    limits: ModelNormalizationLimits,
) -> (NormalizeOutcome, Meter) {
    if let Err(refusal) = decode_check(bundle) {
        return (NormalizeOutcome::Refused(refusal), Meter::new(limits));
    }
    let built = match build(bundle) {
        Ok(built) => built,
        Err(refusal) => return (NormalizeOutcome::Refused(refusal), Meter::new(limits)),
    };
    let mut meter = Meter::new(limits);
    let outcome = match charge_all(bundle, &built, &mut meter) {
        Ok(()) => NormalizeOutcome::Completed(built.view),
        Err(incomplete) => NormalizeOutcome::Incomplete(incomplete),
    };
    (outcome, meter)
}

/// The object universe `bundle` normalizes to, independent of `charge_all`'s
/// bookkeeping (test and caller convenience; recomputes via [`build`]).
pub fn object_universe(bundle: &Bundle) -> Result<ObjectUniverse, ModelRefusal> {
    build(bundle).map(|built| built.universe)
}
