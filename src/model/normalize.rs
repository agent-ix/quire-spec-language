// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 model normalization: phases 1 (decode), 2 (qualify), 3 (inherit),
//! 4 (`quire.model.normalize.redefine/v1`, explicit field redefinition) and
//! 5 (canonicalize).
//!
//! Phase 4 here covers exactly TC-195 N06's shape: explicit
//! [`crate::model::bundle::RedefinitionRecord`]s over **field** members,
//! resolved by the same proper-descendant dominance FR-151 dispatch later
//! reuses (a redefining owner that is a proper descendant of every other
//! contesting owner wins outright; two or more undominated owners refuse
//! `derivation-conflict`). `quire.model.normalize.subset/v1` (explicit
//! subsetting) derives no replacement member — FR-150 says so explicitly
//! ("subsetting never conflicts with redefinition because it derives no
//! replacement") — so [`crate::model::bundle::SubsettingRecord`] carries no
//! normalization derivation at all; it is checked directly by
//! `crate::model::conformance` (the static `subsetting-type` axis) and, at
//! runtime, by FR-153's `binding.subset-value` check (`subsetting-violation`)
//! — FR-153 territory, not yet implemented anywhere in this crate (see
//! `crate::model::conformance`'s own module doc).
//! **Operation-member** redefinition (TC-196 R02–R08) also builds no phase
//! here: `crate::model::conformance` constructs its own
//! [`EffectiveDeclarationPreimage`] directly over the bundle's
//! [`crate::model::bundle::OperationMemberRecord`]/`RedefinitionRecord`
//! values for FR-151's conformance checking, so this pass — which exists to
//! grow [`EffectiveView`] itself — has nothing to add for operations. This
//! split is a scope decision recorded here, not a silent gap: extending
//! this pass to also normalize operation-member redefinition into the view
//! is future work, tracked by the PR that made this decision.
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
//!
//! A closing cycle edge (TC-196 R01) refuses `specialization-cycle` naming
//! every contributing declaration in the cycle, rotated to start at its
//! least key, from the one type whose own walk finds it first
//! ([`ancestor_paths`]'s own `?`-propagated `Err`) — this rung does not also
//! continue walking every remaining type to reproduce R01's exact
//! six-`normalize.cycle-check`-charge, deduplicated-across-both-walks
//! accounting; only the refusal's own shape is reproduced, a recorded scope
//! choice like phase 4's own dominance-check reuse above.
//!
//! Phase 4's own dominance check (deciding which of several redefiners of
//! the same target wins) asks a different question than phase 3's own
//! [`ancestor_paths`]: only *reachability* between two specific owners, never
//! every path between them, and it has no derivation facts of its own to
//! charge against `limits`. An earlier version reused [`ancestor_paths`]
//! anyway, under a budget (`remaining_fact_budget(limits, 0)`) that read as
//! this run's real remaining capacity but silently reset to "everything
//! remaining" on every call — so `apply_redefinitions`'s `O(|edges|^2)` loop
//! of pairwise dominance tests over it could look individually bounded
//! while doing unbounded aggregate work across many calls (PR #144 review
//! finding #4, QSL #145). It now delegates to
//! [`crate::model::conformance::type_conforms`], the identical bounded
//! (`MAX_CONFORMANCE_DEPTH`-ceiling) graph walk
//! [`crate::model::dispatch`]'s own dominance check already uses — a fixed,
//! honest per-call ceiling rather than a budget with no legitimate claim on
//! `build`'s shared, cumulative fact accounting. The pairwise loop itself is
//! still `O(|edges|^2)` in the worst case, but each contesting redefinition
//! edge's *owner* — not the edge — is what the walk is keyed on, and TC-196
//! R07 documents the case where several edges share one owner; `apply_redefinitions`
//! precomputes each *distinct* owner's ancestor closure once
//! ([`conformance::ancestor_closure`](crate::model::conformance::ancestor_closure))
//! rather than re-walking the graph once per edge pair, so the walk cost is
//! `O(distinct owners)`, not `O(|edges|^2)`, and only cheap `O(1)` set
//! lookups remain inside the pair enumeration. Phase 3's own already-computed
//! ancestor paths for `type_key` are still reused verbatim for
//! `apply_redefinitions`'s owner-path bookkeeping rather than recomputed a
//! second time (PR #140 F10's "don't walk the identical DFS twice" lesson).

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Code;
use crate::model::accounting::{
    Charge, ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use crate::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection,
    INTERFACE_VERSION_1_2_0, INTERFACE_VERSION_1_3_0,
};
use crate::model::conformance::{ancestor_closure, generals_by_specific};
use crate::model::key::{
    digest_of, jcs_bytes, EffectiveDeclarationPreimage, EffectiveId, Fact, ProducerKey,
    PRODUCER_DIGEST_DOMAIN, RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE,
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

/// One entry of an [`EffectiveView`]: `{effective_id, preimage}`, plus this
/// pass's own `visible` bit.
///
/// `visible` is never part of `preimage`'s identity bytes — the effective
/// view's serialized shape and every `quire.model.effective-view/v1`
/// identity this rung already ships (TC-195 N01/N02) are unchanged by phase
/// 4. A phase-4 redefinition contest can still mark an entry hidden: the
/// member is a real effective declaration FR-150 requires the view to
/// retain for provenance, but it lost the redefinition contest for its
/// (owner, original) key, so it is not the declaration a name resolves to.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ViewEntry {
    /// This declaration's computed identity.
    pub effective_id: EffectiveId,
    /// The preimage that identity was computed from.
    pub preimage: EffectiveDeclarationPreimage,
    /// Whether this declaration is the active one for its `(owner,
    /// original)` key. `false` only for a member phase 4 dominated away by
    /// a redefinition elsewhere reaching the same owner.
    pub visible: bool,
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
    /// must both survive, never merge. Also serves phase 4's and this
    /// module's other passes' "is this a declared type" dangling-reference
    /// checks — a second, redundant `known_types` set would only ever
    /// duplicate this one.
    types: std::collections::BTreeSet<ProducerKey>,
    fields_by_owner: HashMap<ProducerKey, Vec<FieldMemberRecord>>,
    generals_by_specific: HashMap<ProducerKey, Vec<GeneralizationRecord>>,
    /// Every type that is some record's `specific`, i.e. not a root.
    non_root: std::collections::HashSet<ProducerKey>,
    /// Every declared scalar type's own full key.
    known_scalars: std::collections::HashSet<ProducerKey>,
    /// Every field member's own key.
    field_member_keys: std::collections::HashSet<ProducerKey>,
    /// Every operation member's own key.
    operation_member_keys: std::collections::HashSet<ProducerKey>,
}

impl Index {
    fn build(bundle: &Bundle) -> Self {
        let mut types = std::collections::BTreeSet::new();
        let mut fields_by_owner: HashMap<ProducerKey, Vec<_>> = HashMap::new();
        // Built once by the one shared function every bounded proper-descendant
        // walk in `crate::model` uses (`crate::model::conformance`'s own doc),
        // rather than a second, independent accumulation of the identical
        // `specific -> generalization records` map.
        let generals_by_specific = generals_by_specific(bundle);
        let mut non_root = std::collections::HashSet::new();
        let mut known_scalars = std::collections::HashSet::new();
        let mut field_member_keys = std::collections::HashSet::new();
        let mut operation_member_keys = std::collections::HashSet::new();
        for record in &bundle.records {
            match record {
                BundleRecord::ObjectType(t) => {
                    types.insert(t.key.clone());
                }
                BundleRecord::FieldMember(m) => {
                    field_member_keys.insert(m.key.clone());
                    fields_by_owner
                        .entry(m.owner.clone())
                        .or_default()
                        .push(m.clone());
                }
                BundleRecord::Generalization(g) => {
                    non_root.insert(g.specific.clone());
                }
                BundleRecord::ScalarType(s) => {
                    known_scalars.insert(s.key.clone());
                }
                BundleRecord::OperationMember(o) => {
                    operation_member_keys.insert(o.key.clone());
                }
                // Redefinition bookkeeping is scanned per type directly from
                // `bundle.records` by `apply_redefinitions`, so it needs no
                // index bucket here.
                BundleRecord::Redefinition(_) => {}
                // Subsetting derives no normalization fact (see module
                // docs); it needs no bookkeeping here at all.
                BundleRecord::Subsetting(_) => {}
                // FR-152 systems-model records (crate::model::systems) are
                // not FR-150 normalization inputs: they neither declare a
                // type nor derive an effective declaration here.
                BundleRecord::Component(_)
                | BundleRecord::Endpoint(_)
                | BundleRecord::Relationship(_) => {}
            }
        }
        Self {
            types,
            fields_by_owner,
            generals_by_specific,
            non_root,
            known_scalars,
            field_member_keys,
            operation_member_keys,
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
            // Every contributing declaration in the cycle itself, not the
            // whole path from the walk's root: `frame.visited` is that whole
            // path, so it is first trimmed to start at `ancestor_key`'s own
            // first occurrence (everything before that is how the walk
            // *reached* the cycle, not part of it), then rotated to start at
            // its least key so the same cycle reports identically regardless
            // of which type's own walk closes it first (TC-196 R01: "listing
            // [A, B], rotated to start at the least key A"). E.g. A -> C,
            // C -> B, B -> C lists `[model.B, model.C]`, not
            // `[model.A, model.C, model.B]`.
            // Scope decision: this rung stops at the first cycle a type's
            // own walk finds (line ~723's `?`) rather than continuing to
            // walk every remaining type and deduplicating repeated closures,
            // so it does not reproduce R01's exact six-`normalize.cycle-check`
            // charge count across both types' walks — only the refusal's own
            // code/cause/contributing-declarations shape.
            let mut chain = frame.visited.clone();
            if let Some(start) = chain.iter().position(|key| key == &ancestor_key) {
                chain.drain(..start);
            }
            if let Some(least) = chain
                .iter()
                .enumerate()
                .min_by_key(|(_, key)| *key)
                .map(|(index, _)| index)
            {
                chain.rotate_left(least);
            }
            let listing: Vec<&str> = chain.iter().map(|key| key.identity.as_str()).collect();
            return Err(ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: "specialization-cycle",
                detail: format!(
                    "{} generalizes back to itself via {}, through the cycle [{}]",
                    ancestor_key.identity,
                    record.key.identity,
                    listing.join(", ")
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
    /// Total phase-4 redefinition edges examined across every type, in
    /// `apply_redefinitions` call order; replayed by `charge_all` as
    /// `normalize.redefinition-check` charges.
    redefinition_edges: u64,
    /// Total phase-4 target groups (contested `redefined` keys) resolved
    /// across every type; replayed by `charge_all` as
    /// `normalize.conflict-check` charges.
    conflict_groups: u64,
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
            BundleRecord::ScalarType(_) => {}
            BundleRecord::OperationMember(op) if !index.types.contains(&op.owner) => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: "unknown-owner",
                    detail: format!(
                        "operation member {} names owner {}, which is not a declared object type",
                        op.key.identity, op.owner.identity
                    ),
                });
            }
            BundleRecord::OperationMember(op) => {
                for parameter in &op.parameters {
                    if !index.types.contains(&parameter.value_type)
                        && !index.known_scalars.contains(&parameter.value_type)
                    {
                        return Err(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: "unknown-value-type",
                            detail: format!(
                                "operation {} parameter {} names value type {}, which is not a declared type",
                                op.key.identity,
                                parameter.key.identity,
                                parameter.value_type.identity
                            ),
                        });
                    }
                }
                if let Some(result) = &op.result {
                    if !index.types.contains(&result.value_type)
                        && !index.known_scalars.contains(&result.value_type)
                    {
                        return Err(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: "unknown-value-type",
                            detail: format!(
                                "operation {} result names value type {}, which is not a declared type",
                                op.key.identity, result.value_type.identity
                            ),
                        });
                    }
                }
                for field in &op.effect.field_writes {
                    if !index.field_member_keys.contains(field) {
                        return Err(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: "unknown-field-write",
                            detail: format!(
                                "operation {} effect writes {}, which is not a declared field member",
                                op.key.identity, field.identity
                            ),
                        });
                    }
                }
                for target in op.effect.creates.iter().chain(&op.effect.deletes) {
                    if !index.types.contains(target) {
                        return Err(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: "unknown-effect-type",
                            detail: format!(
                                "operation {} effect names type {}, which is not a declared object type",
                                op.key.identity, target.identity
                            ),
                        });
                    }
                }
            }
            BundleRecord::Redefinition(redefinition)
                if !index.types.contains(&redefinition.owner) =>
            {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: "unknown-owner",
                    detail: format!(
                        "redefinition {} names owner {}, which is not a declared object type",
                        redefinition.key.identity, redefinition.owner.identity
                    ),
                });
            }
            BundleRecord::Redefinition(redefinition) => {
                for member in [&redefinition.redefining, &redefinition.redefined] {
                    if !index.field_member_keys.contains(member)
                        && !index.operation_member_keys.contains(member)
                    {
                        return Err(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: "unknown-member",
                            detail: format!(
                                "redefinition {} names {}, which is not a declared field or operation member",
                                redefinition.key.identity, member.identity
                            ),
                        });
                    }
                }
            }
            BundleRecord::Subsetting(subsetting) if !index.types.contains(&subsetting.owner) => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: "unknown-owner",
                    detail: format!(
                        "subsetting {} names owner {}, which is not a declared object type",
                        subsetting.key.identity, subsetting.owner.identity
                    ),
                });
            }
            BundleRecord::Subsetting(subsetting) => {
                for member in [&subsetting.subsetting, &subsetting.subsetted] {
                    if !index.field_member_keys.contains(member)
                        && !index.operation_member_keys.contains(member)
                    {
                        return Err(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: "unknown-member",
                            detail: format!(
                                "subsetting {} names {}, which is not a declared field or operation member",
                                subsetting.key.identity, member.identity
                            ),
                        });
                    }
                }
            }
            BundleRecord::FieldMember(_) | BundleRecord::Generalization(_) => {}
            // FR-152 systems-model records validate their own references
            // independently (crate::model::systems); FR-150's phase 1 does
            // not concern itself with them.
            BundleRecord::Component(_)
            | BundleRecord::Endpoint(_)
            | BundleRecord::Relationship(_) => {}
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
    let mut hidden: std::collections::HashSet<(ProducerKey, ProducerKey)> =
        std::collections::HashSet::new();
    let mut facts_so_far: u64 = 0;
    let mut redefinition_edges: u64 = 0;
    let mut conflict_groups: u64 = 0;

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
        // twice per type), and again for phase 4's own owner-path
        // bookkeeping just below (the same F10 lesson applied there too).
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

        let (type_edges, type_groups) = apply_redefinitions(
            bundle,
            &index,
            type_key,
            &paths,
            &mut member_preimages,
            &mut hidden,
        )?;
        redefinition_edges += type_edges;
        conflict_groups += type_groups;
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
            visible: true,
        });
    }
    let mut member_keys: Vec<(ProducerKey, ProducerKey)> =
        member_preimages.keys().cloned().collect();
    member_keys.sort_by(|(owner_a, decl_a), (owner_b, decl_b)| {
        owner_a.cmp(owner_b).then_with(|| decl_a.cmp(decl_b))
    });
    for key in member_keys {
        let visible = !hidden.contains(&key);
        let preimage = member_preimages.remove(&key).expect("built above");
        let (effective_id, jcs_len) = preimage.identity_and_jcs_len();
        declarations.push(PendingDeclaration { jcs_len });
        entries.push(ViewEntry {
            effective_id,
            preimage,
            visible,
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
        redefinition_edges,
        conflict_groups,
    })
}

/// One redefinition record's contest for its `redefined` target, reachable
/// at `type_key` along `path` (the ancestor-generalization keys from
/// `type_key` to `owner`, empty when `owner` is `type_key` itself).
struct RedefinitionEdge {
    owner: ProducerKey,
    redefining: ProducerKey,
    record_key: ProducerKey,
    target: ProducerKey,
    path: Vec<ProducerKey>,
}

/// Phase 4 (TC-195 N06): field redefinition only — operation-member
/// redefinition is out of scope here (see the module docs); `conformance`
/// resolves that case directly against its own [`EffectiveDeclarationPreimage`]
/// values instead of this pass's exposure bookkeeping.
///
/// For every field redefinition record reachable at `type_key` (declared on
/// `type_key` itself or a generalization ancestor, per `paths` — already
/// computed by `build` for this same `type_key`, not recomputed here), groups
/// the competing redefiners of the same `redefined` target and resolves a
/// unique winner: the one whose owner is a proper descendant of every other
/// competing owner. A target with no competing redefiner needs no
/// resolution. Two or more competing redefiners with no owner dominating
/// every other is a typed `derivation-conflict` refusal naming every
/// competing path — never an arbitrary pick. The winner gains one redefine
/// fact citing its own edge; every other contender (the target itself, and
/// every losing redefiner) gains its own edge's redefine fact and is marked
/// hidden, retained in the view for provenance but not the declaration its
/// `(owner, original)` key resolves to.
fn apply_redefinitions(
    bundle: &Bundle,
    index: &Index,
    type_key: &ProducerKey,
    paths: &[AncestorPath],
    member_preimages: &mut HashMap<(ProducerKey, ProducerKey), EffectiveDeclarationPreimage>,
    hidden: &mut std::collections::HashSet<(ProducerKey, ProducerKey)>,
) -> Result<(u64, u64), ModelRefusal> {
    let mut owner_paths: HashMap<ProducerKey, Vec<ProducerKey>> = HashMap::new();
    owner_paths.insert(type_key.clone(), Vec::new());
    for ancestor in paths {
        owner_paths
            .entry(ancestor.ancestor_key.clone())
            .or_insert_with(|| ancestor.path.clone());
    }

    let mut groups: HashMap<ProducerKey, Vec<RedefinitionEdge>> = HashMap::new();
    for record in &bundle.records {
        let BundleRecord::Redefinition(redefinition) = record else {
            continue;
        };
        if !index.field_member_keys.contains(&redefinition.redefining)
            || !index.field_member_keys.contains(&redefinition.redefined)
        {
            // Operation-member redefinition: out of scope for this pass.
            continue;
        }
        let Some(path) = owner_paths.get(&redefinition.owner) else {
            // This redefinition's owner does not reach `type_key`.
            continue;
        };
        groups
            .entry(redefinition.redefined.clone())
            .or_default()
            .push(RedefinitionEdge {
                owner: redefinition.owner.clone(),
                redefining: redefinition.redefining.clone(),
                record_key: redefinition.key.clone(),
                target: redefinition.redefined.clone(),
                path: path.clone(),
            });
    }

    let mut target_keys: Vec<ProducerKey> = groups.keys().cloned().collect();
    target_keys.sort();

    let mut edge_total: u64 = 0;
    let mut group_total: u64 = 0;

    for target_key in target_keys {
        let mut edges = groups.remove(&target_key).expect("just listed");
        group_total += 1;
        edge_total += edges.len() as u64;
        edges.sort_by(|a, b| {
            a.owner
                .cmp(&b.owner)
                .then_with(|| a.record_key.cmp(&b.record_key))
        });

        // Precomputed once per *distinct* owner among `edges` (QSL #145 / PR
        // #144 review finding #4): the winner search and its undominated-owner
        // fallback below each compare every edge's owner against every other
        // edge's owner, but TC-196 R07's own documented shape — several
        // redefining members sharing one contending owner — means distinct
        // owners are frequently far fewer than edges. See the module docs.
        let closures = owner_dominance_closures(
            &index.generals_by_specific,
            edges.iter().map(|edge| edge.owner.clone()),
        )?;

        let mut winner: Option<usize> = None;
        for i in 0..edges.len() {
            let mut dominates_all = true;
            for j in 0..edges.len() {
                if i == j {
                    continue;
                }
                if !owner_dominates(&closures, &edges[i].owner, &edges[j].owner) {
                    dominates_all = false;
                    break;
                }
            }
            if dominates_all {
                winner = Some(i);
                break;
            }
        }

        let Some(winner_index) = winner else {
            // TC-196 R07's second shape: among the undominated edges, the
            // most-derived owners (those no *other* edge's owner properly
            // descends from) are all the identical owner — contending
            // redefiners of one inherited target (e.g. `B/z` and `B/z2`
            // both `redefines: A/x`) — rather than distinct sibling
            // lineages neither of which dominates the other (a genuine
            // diamond, `derivation-conflict` below).
            //
            // Restricted to the most-derived owners, not every edge's
            // owner: a less-derived owner's edge (e.g. `C/w` where
            // `B <= C`) has already lost to any more-derived owner's edge
            // (`B`'s) in the winner search above exactly as it would if
            // `B` had only one redefiner, so it takes no part in deciding
            // whether the *remaining* ambiguity is "one owner, several
            // redefiners" or "two genuinely different lineages."
            //
            // A caller cannot resolve either shape by an arbitrary pick,
            // but they are different ambiguities with different FR-272
            // causes: this one refuses `redefinition-target`, naming every
            // redefining member's own declaration key (never its
            // redefinition record's key) and the one contended target.
            let mut most_derived: Vec<&RedefinitionEdge> = Vec::new();
            for edge in &edges {
                let mut dominated_by_another = false;
                for other in &edges {
                    if other.owner.identity == edge.owner.identity {
                        continue;
                    }
                    if owner_dominates(&closures, &other.owner, &edge.owner) {
                        dominated_by_another = true;
                        break;
                    }
                }
                if !dominated_by_another {
                    most_derived.push(edge);
                }
            }
            let same_owner = !most_derived.is_empty()
                && most_derived
                    .iter()
                    .all(|edge| edge.owner.identity == most_derived[0].owner.identity);
            if same_owner {
                let mut redefiners: Vec<String> = most_derived
                    .iter()
                    .map(|edge| edge.redefining.identity.clone())
                    .collect();
                redefiners.sort();
                return Err(ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: "redefinition-target",
                    detail: format!(
                        "{} declares {} redefining members ({}) that all redefine {}, with no single valid target",
                        most_derived[0].owner.identity,
                        most_derived.len(),
                        redefiners.join(", "),
                        target_key.identity
                    ),
                });
            }

            let paths: Vec<String> = edges
                .iter()
                .map(|edge| {
                    let mut path: Vec<String> =
                        edge.path.iter().map(|key| key.identity.clone()).collect();
                    path.push(edge.record_key.identity.clone());
                    path.push(target_key.identity.clone());
                    format!("[{}]", path.join(", "))
                })
                .collect();
            return Err(ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: "derivation-conflict",
                detail: format!(
                    "type {} has {} undominated redefinitions of {}: {}",
                    type_key.identity,
                    edges.len(),
                    target_key.identity,
                    paths.join(" and ")
                ),
            });
        };

        let member_key = (type_key.clone(), target_key.clone());
        if !member_preimages.contains_key(&member_key) {
            return Err(ModelRefusal {
                code: Code::DanglingReference,
                cause: "redefinition-unreachable",
                detail: format!(
                    "redefinition target {} is not an effective member of {}",
                    target_key.identity, type_key.identity
                ),
            });
        }

        for (i, edge) in edges.iter().enumerate() {
            let mut inputs = edge.path.clone();
            inputs.push(edge.record_key.clone());
            inputs.push(edge.target.clone());

            let redefining_key = (type_key.clone(), edge.redefining.clone());
            let entry = member_preimages
                .get_mut(&redefining_key)
                .ok_or_else(|| ModelRefusal {
                    code: Code::DanglingReference,
                    cause: "redefinition-unreachable",
                    detail: format!(
                        "redefining member {} is not an effective member of {}",
                        edge.redefining.identity, type_key.identity
                    ),
                })?;
            let ordinal = entry.derivation.len();
            entry.derivation.push(Fact {
                ordinal,
                rule: RULE_REDEFINE,
                inputs: inputs.clone(),
            });
            if i != winner_index {
                hidden.insert(redefining_key);
            }

            let target_entry = member_preimages
                .get_mut(&member_key)
                .expect("checked reachable above");
            let ordinal = target_entry.derivation.len();
            target_entry.derivation.push(Fact {
                ordinal,
                rule: RULE_REDEFINE,
                inputs,
            });
        }
        hidden.insert(member_key);
    }

    Ok((edge_total, group_total))
}

/// Precomputes each distinct owner among `owners`' own proper-ancestor
/// closure via [`ancestor_closure`], once per owner rather than once per
/// contesting-edge pair (QSL #145 / PR #144 review finding #4; see the
/// module docs). Delegates to [`crate::model::conformance`]'s shared,
/// bounded (`MAX_CONFORMANCE_DEPTH`-ceiling) graph walk — the same
/// primitive [`crate::model::conformance::type_conforms`] and
/// [`crate::model::dispatch`]'s own
/// dominance check use — rather than [`ancestor_paths`], which exists to
/// build derivation-fact *paths* for the effective view under `build`'s own
/// cumulative fact budget; a pure boolean dominance query has no derivation
/// facts to charge against that budget and no legitimate claim on it.
fn owner_dominance_closures(
    generals_by_specific: &HashMap<ProducerKey, Vec<GeneralizationRecord>>,
    owners: impl Iterator<Item = ProducerKey>,
) -> Result<HashMap<ProducerKey, HashSet<ProducerKey>>, ModelRefusal> {
    let mut distinct: Vec<ProducerKey> = owners.collect();
    distinct.sort();
    distinct.dedup();
    let mut closures = HashMap::with_capacity(distinct.len());
    for owner in distinct {
        let closure = ancestor_closure(generals_by_specific, &owner)?;
        closures.insert(owner, closure);
    }
    Ok(closures)
}

/// Whether `p_owner` strictly dominates `q_owner`: `p_owner` is a proper
/// descendant of `q_owner` in `closures` (see [`owner_dominance_closures`]).
/// An `O(1)` set lookup against an already-computed closure, never a fresh
/// graph walk — the `O(|edges|^2)` pair enumeration this function is called
/// from stays cheap because the expensive part already ran once per
/// distinct owner.
fn owner_dominates(
    closures: &HashMap<ProducerKey, HashSet<ProducerKey>>,
    p_owner: &ProducerKey,
    q_owner: &ProducerKey,
) -> bool {
    p_owner != q_owner
        && closures
            .get(p_owner)
            .is_some_and(|ancestors| ancestors.contains(q_owner))
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

    for _ in 0..built.redefinition_edges {
        meter.charge(Charge::new(ChargePoint::NormalizeRedefinitionCheck))?;
    }
    for _ in 0..built.conflict_groups {
        meter.charge(Charge::new(ChargePoint::NormalizeConflictCheck))?;
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
        BundleRecord::ScalarType(record) => vec![&record.key],
        BundleRecord::OperationMember(record) => {
            let mut keys = vec![&record.key, &record.owner];
            for parameter in &record.parameters {
                keys.push(&parameter.key);
                keys.push(&parameter.value_type);
            }
            if let Some(result) = &record.result {
                keys.push(&result.value_type);
            }
            for field in &record.effect.field_writes {
                keys.push(field);
            }
            for target in record.effect.creates.iter().chain(&record.effect.deletes) {
                keys.push(target);
            }
            keys
        }
        BundleRecord::Redefinition(record) => {
            vec![
                &record.key,
                &record.owner,
                &record.redefining,
                &record.redefined,
            ]
        }
        BundleRecord::Subsetting(record) => {
            vec![
                &record.key,
                &record.owner,
                &record.subsetting,
                &record.subsetted,
            ]
        }
        BundleRecord::Component(record) => {
            vec![&record.key, &record.owning_type, &record.value_type]
        }
        BundleRecord::Endpoint(record) => {
            vec![&record.key, &record.owning_component, &record.value_type]
        }
        BundleRecord::Relationship(record) => {
            vec![
                &record.key,
                &record.source.type_identity,
                &record.target.type_identity,
            ]
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

/// Normalize `bundle` under `limits`: FR-150 phases 1, 2, 3, 4 and 5.
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
