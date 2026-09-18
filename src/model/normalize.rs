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
//! [`build`] enumerates ancestor paths and derivation facts under a fact
//! budget derived from `limits` itself (PR #140 F1): a diamond
//! generalization graph produces an ancestor-path count exponential in
//! depth, so an adversarial bundle enumerated without bound before any
//! charge is consulted can exhaust memory long before [`charge_all`] gets a
//! chance to deny anything — `ModelNormalizationLimits` protects nothing if
//! it is only consulted after the fact. `remaining_fact_budget` and
//! `fact_budget_exceeded` both only ever *overestimate* remaining capacity
//! (never underestimate it), so a bundle that legitimately completes under
//! `limits` is never truncated: enumeration only stops once continuing is
//! certainly futile, and it always generates at least one fact past that
//! point so [`charge_all`]'s real, exact replay is the one that reports the
//! [`Incomplete`] — `build` itself never guesses at `limit`/`consumed`/
//! `next_charge`. In the (adversarial, meter-already-saturated) case where a
//! type's own budget is already exhausted before its ancestor paths are
//! walked, that type's cycle/depth checks stop early too: this is
//! deliberate, not a missed check — the real charge sequence would have
//! denied at or before this type regardless of what a deeper walk would have
//! found, so the reported outcome is still the correct `Incomplete`, matching
//! FR-150's "exhaustion ends checking" rule.

use std::collections::HashMap;

use crate::diagnostic::Code;
use crate::model::accounting::{
    Charge, ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use crate::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection,
    INTERFACE_VERSION_1_2_0, INTERFACE_VERSION_1_3_0,
};
use crate::model::key::{
    digest_of, jcs_bytes, EffectiveDeclarationPreimage, EffectiveId, Fact, ProducerKey,
    PRODUCER_DIGEST_DOMAIN, RULE_INHERIT, RULE_QUALIFY,
};
use crate::value::length_amount;

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
    /// A producer interface `1.2.0` bundle: exactly one
    /// `unsupplied-producer-record` refusal per missing FR-150 capability
    /// item, in the fixed TC-195 N08 order; no effective view.
    UnsupportedCapabilities(Vec<ModelRefusal>),
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
            Value::String(crate::model::key::EFFECTIVE_VIEW_DOMAIN.to_owned()),
        );
        object.insert("model_selection".to_owned(), self.model_selection.to_json());
        object.insert(
            "rules".to_owned(),
            Value::Object({
                let mut rules = Map::new();
                rules.insert(
                    "identity".to_owned(),
                    Value::String(crate::model::key::RULES_IDENTITY.to_owned()),
                );
                rules.insert(
                    "revision".to_owned(),
                    Value::String(crate::model::key::RULES_REVISION.to_owned()),
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
                        e.insert("preimage".to_owned(), entry.preimage.to_json());
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

    /// The exact JCS bytes of this view (for `normalize.hash` accounting,
    /// and for tests asserting an exact hashed-byte length against a
    /// vendored ground-truth vector).
    pub fn jcs_bytes(&self) -> Vec<u8> {
        jcs_bytes(&self.to_json())
    }

    /// Whether `declarations` is correctly sorted ascending by effective
    /// identity — TC-195 N10's `unsorted-view` mutation, "refused by the
    /// semantic check" over an already-constructed view (PR #140 F5).
    pub fn validate_order(&self) -> Result<(), ModelRefusal> {
        for pair in self.declarations.windows(2) {
            if pair[0].effective_id > pair[1].effective_id {
                return Err(ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: "unsorted-view",
                    detail: format!(
                        "view declarations are not sorted ascending by effective identity at {}",
                        pair[1].effective_id.hex()
                    ),
                });
            }
        }
        Ok(())
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
            Value::String(crate::model::key::OBJECT_UNIVERSE_DOMAIN.to_owned()),
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

    /// The exact JCS bytes of this universe (for `normalize.hash` accounting,
    /// and for tests asserting an exact hashed-byte length against a
    /// vendored ground-truth vector).
    pub fn jcs_bytes(&self) -> Vec<u8> {
        jcs_bytes(&self.to_json())
    }
}

/// A generalization or field-member producer record is not visited more
/// than this many times in one ancestor path before normalization refuses
/// rather than recurse without bound.
pub const MAX_GENERALIZATION_DEPTH: usize = 128;

struct Index {
    /// Every declared object type, keyed by its own full producer key (PR
    /// #140 F2): two `ObjectType` records that share a display identity but
    /// differ in revision or digest are distinct original declarations and
    /// must both survive, never merge.
    types: std::collections::BTreeSet<ProducerKey>,
    fields_by_owner: HashMap<ProducerKey, Vec<FieldMemberRecord>>,
    generals_by_specific: HashMap<ProducerKey, Vec<GeneralizationRecord>>,
    /// Every type that is some record's `specific`, i.e. not a root.
    non_root: std::collections::HashSet<ProducerKey>,
}

impl Index {
    fn build(bundle: &Bundle) -> Self {
        let mut types = std::collections::BTreeSet::new();
        let mut fields_by_owner: HashMap<ProducerKey, Vec<_>> = HashMap::new();
        let mut generals_by_specific: HashMap<ProducerKey, Vec<_>> = HashMap::new();
        let mut non_root = std::collections::HashSet::new();
        for record in &bundle.records {
            match record {
                BundleRecord::ObjectType(t) => {
                    types.insert(t.key.clone());
                }
                BundleRecord::FieldMember(m) => {
                    fields_by_owner
                        .entry(m.owner.clone())
                        .or_default()
                        .push(m.clone());
                }
                BundleRecord::Generalization(g) => {
                    non_root.insert(g.specific.clone());
                    generals_by_specific
                        .entry(g.specific.clone())
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
        self.types.iter().cloned().collect()
    }

    fn sorted_direct_members(&self, owner: &ProducerKey) -> Vec<FieldMemberRecord> {
        let mut members = self.fields_by_owner.get(owner).cloned().unwrap_or_default();
        members.sort_by(|a, b| a.key.cmp(&b.key));
        members
    }

    fn sorted_generals(&self, specific: &ProducerKey) -> Vec<GeneralizationRecord> {
        let mut generals = self
            .generals_by_specific
            .get(specific)
            .cloned()
            .unwrap_or_default();
        generals.sort_by(|a, b| a.key.cmp(&b.key));
        generals
    }
}

/// One path from a type to a strict ancestor: the ordered chain of
/// generalization-record keys taken, and the ancestor's own original
/// producer key (PR #140 F2: the full key, not a display identity string).
struct AncestorPath {
    path: Vec<ProducerKey>,
    ancestor_key: ProducerKey,
}

/// A conservative upper bound on how many additional phase-2/3 facts this
/// build could ever admit before [`charge_all`] denies a `normalize.fact` or
/// `normalize.cycle-check` charge, derived from `limits` and the facts
/// already produced (PR #140 F1). Passed into [`ancestor_paths`] so an
/// adversarial diamond-generalization bundle's path count is bounded by the
/// meter's own configuration *during* enumeration, not only checked after
/// full materialization.
///
/// Both terms only ever overestimate true remaining capacity, so a
/// legitimately completable run is never truncated: `derivation_room` is
/// exact (every fact costs exactly one `derivation_facts` unit);
/// `work_room` credits every fact with only the one `work_units` it is
/// guaranteed to cost, though a `normalize.cycle-check` fact may cost more —
/// undercounting consumption always overestimates remaining room.
fn remaining_fact_budget(limits: &ModelNormalizationLimits, facts_so_far: u64) -> usize {
    let derivation_room = limits.derivation_facts.saturating_sub(facts_so_far);
    let work_room = limits.work_units.saturating_sub(facts_so_far);
    let room = derivation_room.min(work_room);
    usize::try_from(room.saturating_add(1)).unwrap_or(usize::MAX)
}

/// Whether at least `limits.derivation_facts` or `limits.work_units` facts
/// have already been produced, i.e. continuing to generate more can no
/// longer change the outcome: [`charge_all`]'s real replay will already
/// deny at or before this point. See [`remaining_fact_budget`]'s doc for why
/// this never fires early on a run that would otherwise complete.
fn fact_budget_exceeded(limits: &ModelNormalizationLimits, facts_so_far: u64) -> bool {
    facts_so_far > limits.derivation_facts || facts_so_far > limits.work_units
}

/// Every ancestor path of `root_key`, in DFS pre-order over ascending-key
/// direct generalizations, capped at `budget` entries. Explicit stack, not
/// native recursion: bundle data is caller-supplied and may describe a
/// cycle.
fn ancestor_paths(
    root_key: &ProducerKey,
    index: &Index,
    budget: usize,
) -> Result<Vec<AncestorPath>, ModelRefusal> {
    struct Frame {
        directs: Vec<GeneralizationRecord>,
        next: usize,
        path: Vec<ProducerKey>,
        visited: Vec<ProducerKey>,
    }

    let mut stack = vec![Frame {
        directs: index.sorted_generals(root_key),
        next: 0,
        path: Vec::new(),
        visited: vec![root_key.clone()],
    }];
    let mut out = Vec::new();
    loop {
        if out.len() >= budget {
            break;
        }
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
                    "ancestor path from {} exceeds {MAX_GENERALIZATION_DEPTH} generalization records",
                    root_key.identity
                ),
            });
        }
        let record = frame.directs[frame.next].clone();
        frame.next += 1;
        let mut new_path = frame.path.clone();
        new_path.push(record.key.clone());
        let ancestor_key = record.general.clone();
        if frame.visited.contains(&ancestor_key) {
            return Err(ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: "specialization-cycle",
                detail: format!(
                    "{} generalizes back to itself via {}",
                    ancestor_key.identity, record.key.identity
                ),
            });
        }
        out.push(AncestorPath {
            path: new_path.clone(),
            ancestor_key: ancestor_key.clone(),
        });
        let mut new_visited = frame.visited.clone();
        new_visited.push(ancestor_key.clone());
        stack.push(Frame {
            directs: index.sorted_generals(&ancestor_key),
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

/// Refuses a [`BundleRecord`] that names a type key absent from the bundle's
/// own `ObjectType` records, so a dangling `owner`, `specific` or `general`
/// reference is a typed refusal rather than a panic or a silently dropped
/// record — `bundle` is caller-supplied, not validated on the way in.
/// Membership is checked by the record's *whole* producer key (PR #140 F2):
/// a reference matching some declared type's display identity but not its
/// exact revision/digest is exactly as dangling as one matching nothing.
fn validate_references(bundle: &Bundle, index: &Index) -> Result<(), ModelRefusal> {
    for record in &bundle.records {
        match record {
            BundleRecord::ObjectType(_) => {}
            BundleRecord::FieldMember(member) if !index.types.contains(&member.owner) => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: "unknown-owner",
                    detail: format!(
                        "field member {} names owner {}, which is not a declared object type",
                        member.key.identity, member.owner.identity
                    ),
                });
            }
            BundleRecord::Generalization(general) if !index.types.contains(&general.specific) => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: "unknown-specific",
                    detail: format!(
                        "generalization {} names specific {}, which is not a declared object type",
                        general.key.identity, general.specific.identity
                    ),
                });
            }
            BundleRecord::Generalization(general) if !index.types.contains(&general.general) => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: "unknown-general",
                    detail: format!(
                        "generalization {} names general {}, which is not a declared object type",
                        general.key.identity, general.general.identity
                    ),
                });
            }
            BundleRecord::FieldMember(_) | BundleRecord::Generalization(_) => {}
        }
    }
    Ok(())
}

/// Pass one: build the complete normalization, refusing outright on a real
/// defect and bounding ancestor-path enumeration against `limits` (PR #140
/// F1) so a diamond-generalization bundle cannot force unbounded work before
/// [`charge_all`] gets to deny anything.
fn build(bundle: &Bundle, limits: &ModelNormalizationLimits) -> Result<Built, ModelRefusal> {
    let index = Index::build(bundle);
    validate_references(bundle, &index)?;
    let type_keys = index.sorted_type_keys();

    let mut phase2_facts = Vec::new();
    let mut phase3_facts = Vec::new();
    let mut type_preimages: HashMap<ProducerKey, EffectiveDeclarationPreimage> = HashMap::new();
    let mut type_effective_ids: HashMap<ProducerKey, EffectiveId> = HashMap::new();
    let mut type_jcs_lens: HashMap<ProducerKey, u64> = HashMap::new();
    let mut member_preimages: HashMap<(ProducerKey, ProducerKey), EffectiveDeclarationPreimage> =
        HashMap::new();
    let mut facts_so_far: u64 = 0;

    for type_key in &type_keys {
        phase2_facts.push(PendingFact {
            owner_key: None,
            declared_key: type_key.clone(),
            inputs: vec![type_key.clone()],
            cycle_check_len: None,
        });
        facts_so_far += 1;
        let mut derivation = vec![Fact {
            ordinal: 0,
            rule: RULE_QUALIFY,
            inputs: vec![type_key.clone()],
        }];

        // Ancestor paths are computed once per type and reused for both the
        // type's own derivation and its inherited members below (PR #140
        // F10: the original two-loop shape recomputed this identical DFS
        // twice per type).
        let budget = remaining_fact_budget(limits, facts_so_far);
        let paths = ancestor_paths(type_key, &index, budget)?;
        for ancestor in &paths {
            let mut inputs = ancestor.path.clone();
            inputs.push(ancestor.ancestor_key.clone());
            phase3_facts.push(PendingFact {
                owner_key: None,
                declared_key: type_key.clone(),
                inputs: inputs.clone(),
                cycle_check_len: Some(ancestor.path.len()),
            });
            facts_so_far += 1;
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
        // One `to_json` build serves both the identity (needed immediately,
        // as this type's members' owner) and the JCS length phase 5 needs
        // later (PR #140 F10).
        let (owner_effective_id, type_jcs_len) = preimage.identity_and_jcs_len();
        type_effective_ids.insert(type_key.clone(), owner_effective_id.clone());
        type_jcs_lens.insert(type_key.clone(), type_jcs_len);
        type_preimages.insert(type_key.clone(), preimage);

        // Phase 2: members declared directly on this type.
        for member in index.sorted_direct_members(type_key) {
            phase2_facts.push(PendingFact {
                owner_key: Some(type_key.clone()),
                declared_key: member.key.clone(),
                inputs: vec![member.key.clone()],
                cycle_check_len: None,
            });
            member_preimages.insert(
                (type_key.clone(), member.key.clone()),
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

        // Phase 3: members inherited along this type's ancestor paths
        // (reusing `paths` computed above), from each ancestor's own
        // directly-declared members. Bounded the same way (PR #140 F1): an
        // ancestor with many members multiplies path count, so this loop
        // stops the moment continuing cannot change the outcome.
        'inherited_members: for ancestor in &paths {
            for member in index.sorted_direct_members(&ancestor.ancestor_key) {
                let mut inputs = ancestor.path.clone();
                inputs.push(member.key.clone());
                phase3_facts.push(PendingFact {
                    owner_key: Some(type_key.clone()),
                    declared_key: member.key.clone(),
                    inputs: inputs.clone(),
                    cycle_check_len: None,
                });
                facts_so_far += 1;
                let entry = member_preimages
                    .entry((type_key.clone(), member.key.clone()))
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
                if fact_budget_exceeded(limits, facts_so_far) {
                    break 'inherited_members;
                }
            }
        }
    }

    // Phase 5 (identities only; charging is replayed separately).
    let mut declarations: Vec<PendingDeclaration> = Vec::new();
    let mut entries: Vec<ViewEntry> = Vec::new();
    for type_key in &type_keys {
        let preimage = type_preimages.remove(type_key).expect("built above");
        let effective_id = type_effective_ids[type_key].clone();
        declarations.push(PendingDeclaration {
            jcs_len: type_jcs_lens[type_key],
        });
        entries.push(ViewEntry {
            effective_id,
            preimage,
        });
    }
    let mut member_keys: Vec<(ProducerKey, ProducerKey)> =
        member_preimages.keys().cloned().collect();
    member_keys.sort_by(|(owner_a, decl_a), (owner_b, decl_b)| {
        owner_a.cmp(owner_b).then_with(|| decl_a.cmp(decl_b))
    });
    for key in member_keys {
        let preimage = member_preimages.remove(&key).expect("built above");
        let (effective_id, jcs_len) = preimage.identity_and_jcs_len();
        declarations.push(PendingDeclaration { jcs_len });
        entries.push(ViewEntry {
            effective_id,
            preimage,
        });
    }
    entries.sort_by(|a, b| a.effective_id.cmp(&b.effective_id));

    let mut root_types: Vec<EffectiveId> = type_keys
        .iter()
        .filter(|key| !index.non_root.contains(*key))
        .map(|key| type_effective_ids[key].clone())
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
                .size(LimitKind::ProducerRecords, length_amount(index + 1)),
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
            meter.charge(
                Charge::new(ChargePoint::NormalizeCycleCheck).work(length_amount(path_len)),
            )?;
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

    meter.charge(Charge::new(ChargePoint::NormalizeHash).size(
        LimitKind::HashedBytes,
        length_amount(built.universe.jcs_bytes().len()),
    ))?;
    meter.charge(Charge::new(ChargePoint::NormalizeHash).size(
        LimitKind::HashedBytes,
        length_amount(built.view.jcs_bytes().len()),
    ))?;
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

/// Whether `revision` is absent: FR-150 admits `Revision` as a required
/// struct (there is no wire-optional variant in this typed `Bundle`), so
/// "absent" is the caller supplying an empty namespace or value rather than
/// a real revision label (PR #140 F6).
fn revision_is_absent(revision: &crate::model::key::Revision) -> bool {
    revision.namespace.is_empty() || revision.value.is_empty()
}

/// Phase-1 decode checks common to every entry point: the claimed producer
/// interface version (TC-195 N08), every referenced key's absent revision
/// (TC-195 N04, PR #140 F6) and digest domain (TC-195 N05). All refuse
/// before any charge; no effective view is exposed.
fn decode_check(bundle: &Bundle) -> Result<(), ModelRefusal> {
    let version = &bundle.model_selection.contract_version.interface_version;
    if version != INTERFACE_VERSION_1_3_0 && version != INTERFACE_VERSION_1_2_0 {
        return Err(ModelRefusal {
            code: Code::UnknownWire,
            cause: "unsupported-wire",
            detail: format!(
                "producer interface {version} is not supported; only {INTERFACE_VERSION_1_3_0} is normalized and {INTERFACE_VERSION_1_2_0} is refused"
            ),
        });
    }
    for record in &bundle.records {
        for key in referenced_keys(record) {
            if revision_is_absent(&key.revision) {
                return Err(ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: "wrong-model-selection",
                    detail: format!("{} has no producer revision", key.identity),
                });
            }
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

/// The fixed TC-195 N08 order of FR-150 capability items a producer
/// interface `1.2.0` bundle cannot supply, given the object types and field
/// members `bundle` declares (a `1.2.0` bundle carries no generalization,
/// subsetting or redefinition record at all, so those never appear as
/// *bundle content* — they are refused as capabilities the interface itself
/// cannot carry, one item per declared type or member).
fn unsupplied_capability_items(bundle: &Bundle) -> Vec<(&'static str, String)> {
    let index = Index::build(bundle);
    let type_keys = index.sorted_type_keys();
    let mut member_keys: Vec<ProducerKey> = bundle
        .records
        .iter()
        .filter_map(|record| match record {
            BundleRecord::FieldMember(member) => Some(member.key.clone()),
            _ => None,
        })
        .collect();
    member_keys.sort();

    let bundle_identity = bundle.model_selection.export.identity.clone();
    let mut items: Vec<(&'static str, String)> = vec![
        ("subtype-closure", bundle_identity.clone()),
        ("subsetting-closure", bundle_identity.clone()),
        ("redefinition-closure", bundle_identity),
    ];
    for type_key in &type_keys {
        items.push(("generalization", type_key.identity.clone()));
    }
    for member_key in &member_keys {
        items.push(("redefinition", member_key.identity.clone()));
    }
    for member_key in &member_keys {
        items.push(("subsetting", member_key.identity.clone()));
    }
    for type_key in &type_keys {
        items.push(("interface-signature", type_key.identity.clone()));
    }
    for member_key in &member_keys {
        items.push(("typed-multiplicity", member_key.identity.clone()));
    }
    items
}

/// A producer interface `1.2.0` bundle (TC-195 N08, FR-150-AC-7): charges
/// one `normalize.record` per record and one `normalize.unsupplied-item` per
/// missing capability item, then returns exactly one
/// `invalid_model_binding`/`unsupplied-producer-record` refusal per item, in
/// the fixed order (PR #140 F4). No effective view is ever exposed.
fn unsupported_capability_refusals(
    bundle: &Bundle,
    meter: &mut Meter,
) -> Result<Vec<ModelRefusal>, Incomplete> {
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
                .size(LimitKind::ProducerRecords, length_amount(index + 1)),
        )?;
    }

    let items = unsupplied_capability_items(bundle);
    let supplied = &bundle.model_selection.contract_version.interface_version;
    let mut refusals = Vec::with_capacity(items.len());
    for (name, subject) in items {
        meter.charge(Charge::new(ChargePoint::NormalizeUnsuppliedItem))?;
        refusals.push(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: "unsupplied-producer-record",
            detail: format!(
                "{name} for {subject} requires producer interface {INTERFACE_VERSION_1_3_0}; supplied {supplied}"
            ),
        });
    }
    Ok(refusals)
}

/// Normalize `bundle` under `limits`: FR-150 phases 1, 2, 3 and 5.
pub fn normalize(bundle: &Bundle, limits: ModelNormalizationLimits) -> NormalizeOutcome {
    let (outcome, _meter) = normalize_with_meter(bundle, limits);
    outcome
}

/// A meter constructed to run `bundle` under [`ModelNormalizationLimits::UNLIMITED`]
/// and expose its admitted-charge sequence, for tests that assert the exact
/// charge order alongside [`normalize`]'s result.
pub fn normalize_with_meter(
    bundle: &Bundle,
    limits: ModelNormalizationLimits,
) -> (NormalizeOutcome, Meter) {
    let mut meter = Meter::new(limits);
    if let Err(refusal) = decode_check(bundle) {
        return (NormalizeOutcome::Refused(refusal), meter);
    }
    if bundle.model_selection.contract_version.interface_version == INTERFACE_VERSION_1_2_0 {
        let outcome = match unsupported_capability_refusals(bundle, &mut meter) {
            Ok(refusals) => NormalizeOutcome::UnsupportedCapabilities(refusals),
            Err(incomplete) => NormalizeOutcome::Incomplete(incomplete),
        };
        return (outcome, meter);
    }
    let built = match build(bundle, &limits) {
        Ok(built) => built,
        Err(refusal) => return (NormalizeOutcome::Refused(refusal), meter),
    };
    let outcome = match charge_all(bundle, &built, &mut meter) {
        Ok(()) => NormalizeOutcome::Completed(built.view),
        Err(incomplete) => NormalizeOutcome::Incomplete(incomplete),
    };
    (outcome, meter)
}

/// The object universe `bundle` normalizes to, independent of `charge_all`'s
/// bookkeeping (test and caller convenience; recomputes via [`build`], under
/// [`ModelNormalizationLimits::UNLIMITED`]).
pub fn object_universe(bundle: &Bundle) -> Result<ObjectUniverse, ModelRefusal> {
    build(bundle, &ModelNormalizationLimits::UNLIMITED).map(|built| built.universe)
}
