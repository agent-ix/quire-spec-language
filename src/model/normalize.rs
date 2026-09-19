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
//! is future work, tracked by the PR that made this decision. This scope
//! decision covers only the *view* this pass grows: the `m + r`
//! `normalize.redefinition-check` charge below still prices every
//! redefinition record and every effective member the spec names,
//! operation and field alike (QSL #145), and the
//! `Σ (c − 1) × f(o)` `normalize.conflict-check` charge below prices a
//! contested operation target (`c >= 2` operation-redefinition records
//! reaching the same target) exactly as it prices a contested field target
//! (QSL #145) — neither charge is itself
//! an operation-redefinition normalization pass. Detecting and *resolving*
//! a contested operation target — deciding which of several competing
//! operation redefiners wins, the way this pass's own field-redefinition
//! dominance search does — is not implemented anywhere in this crate today:
//! `crate::model::conformance`'s own module doc names the same gap for its
//! side of this split. Remaining work: #173.
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
//! every path between them. Two earlier attempts got this wrong: reusing
//! [`ancestor_paths`] itself under a budget that silently reset on every
//! call (QSL #145), and then delegating to
//! [`crate::model::conformance::ancestor_closure`]'s bounded
//! (`MAX_CONFORMANCE_DEPTH`-ceiling) walk, which counts *breadth* (total
//! distinct nodes visited) against a ceiling named for depth — a wide but
//! uncontested ancestry (128+ direct generalizations) could exhaust it on a
//! single owner with nothing to dominate, and, once the `edges.len() < 2`
//! guard below fixed that uncontested case, a wide ancestry with a genuine
//! *contest* (`edges.len() >= 2`) still could (QSL #145). Phase 4 now
//! derives each owner's ancestor set from
//! `owner_ancestor_sets`, built once in `build()` from every type's own
//! phase-3 `type_paths` (`ancestor_key`, one entry per distinct proper
//! ancestor) — data `build()` already produced walking every type exactly
//! once for its own derivation facts, so this needs no second walk and has
//! no breadth ceiling of its own to exceed. Because those paths can be
//! truncated under a tight fact budget, `build()` skips phase 4
//! *resolution* entirely whenever `fact_budget_exceeded(limits,
//! facts_so_far)` is already true once every type's phase 2/3 has run: a
//! truncated path set could otherwise derive a false `derivation-conflict`
//! from an owner ancestry that looks incomplete rather than merely
//! unresolved, where the correct outcome is the `Incomplete`
//! [`charge_all`]'s own replay already reports for the phase 2/3 facts that
//! triggered the truncation, reached in charge order well before phase 4's
//! own charges. The pairwise dominance loop itself is still `O(|edges|^2)`
//! in the worst case, but each contesting redefinition edge's *owner* — not
//! the edge — is what `owner_ancestor_sets` is keyed on, and TC-196 R07
//! documents the case where several edges share one owner; looking up each
//! distinct owner's already-built set (`O(distinct owners)` build-wide,
//! computed once regardless of how many (type, target) groups query it —
//! QSL #145) rather than re-deriving it per group
//! leaves only cheap `O(1)` set lookups inside the pair enumeration. A
//! target group with fewer than two contesting edges has nothing to
//! dominate and needs no ancestor-set lookup at all — `apply_redefinitions`
//! skips it and both `normalize.conflict-check` charges entirely whenever a
//! group's `edges.len() < 2`, matching `value-accounting.md:456`'s own
//! `c >= 2` condition below. Phase 3's own already-computed ancestor paths
//! for `type_key` are still reused verbatim for `apply_redefinitions`'s
//! owner-path bookkeeping rather than recomputed a second time (PR #140
//! F10's "don't walk the identical DFS twice" lesson).
//!
//! Phase 4's charge points price this work exactly as
//! `proposals/quire-v1/definitions/value-accounting.md` states, not a flat
//! one work unit (QSL #145 scope item 2):
//! `normalize.redefinition-check` (`:455`) charges `m + r` once per
//! redefinition record in the *entire bundle* — field or operation alike —
//! ascending by the record's own producer key, where `m` is the number of
//! the record's own owning type's effective members (field and operation
//! together) and `r` is the count of redefinition records already checked
//! before it in that same ascending, bundle-wide sequence. This is computed
//! exactly once, in `build()`, before the per-type phase-4 loop runs at
//! all: the unit the spec prices is the record itself, tested once against
//! its own owning type, never once per (record, effective type reaching
//! it) pair the previous per-type loop recomputed it at, which overcharged
//! any record reachable from more than one effective type and reset `r` at
//! each type instead of counting across the whole bundle.
//! `normalize.conflict-check` (`:456`) charges `Σ (c − 1) × f(o)` per
//! contested target with `c >= 2` redefiners, summed once per redefining
//! edge (not once per *distinct* owner: the memoization above is this
//! rung's own optimization, never a change to what is priced), where `f(o)`
//! is owner `o`'s own count of type derivation facts (qualify plus
//! inherit). `m` and `f(o)` are read directly from `build`'s own phase 2/3
//! results (`member_preimages`/`type_preimages`, extended for `m` with the
//! bundle's own directly-declared and inherited operation members), which
//! is why phase 4 now runs as its own pass only after every type's phase
//! 2/3 has finished, rather than interleaved per type as before: a
//! redefinition's owner can sort after the type currently being processed
//! in `type_keys`' ascending order, and its own member/fact counts must
//! already exist regardless.
//!
//! The redefine facts phase 4 itself derives (`RULE_REDEFINE`, one on the
//! redefining feature and one on the redefined feature per redefinition
//! record reaching a type — `model-complete.md:231`) are themselves
//! derivation facts, exactly like phase 2's qualify facts and phase 3's
//! inherit facts, and `value-accounting.md:453` prices every derivation
//! fact as `normalize.fact` regardless of which phase formed it. `build`
//! counts them in `phase4_fact_count`; `charge_all` charges that many
//! `normalize.fact` charges between `normalize.redefinition-check` and
//! `normalize.conflict-check`, matching `:455`'s "before its first
//! `normalize.fact`" and `:456`'s "after its last `normalize.fact`".
//!
//! No phase-4 refusal is ever returned by `build` itself, whichever shape it
//! takes: an owner ancestry with no unique dominant redefiner
//! (`derivation-conflict`, or `redefinition-target` for R07's same-owner
//! shape), a redefinition target that is not an effective member of its
//! owning type, or a redefining member that is not itself an effective
//! member (both the latter surfacing as `RedefinitionUnreachable`).
//! `value-accounting.md:481` states "checking is exhaustive within a
//! stage," so phase 4's own charges — every `normalize.redefinition-check`,
//! every phase-4 `normalize.fact`, every `normalize.conflict-check` — must
//! all be admitted before the refusal they expose is reported. `build`
//! stores whichever such refusal ranks earliest in charge order, of any of
//! these shapes (`record_phase4_refusal`'s own doc, next to
//! [`Phase4Accounting`]), in `Built::phase4_refusal` and keeps resolving
//! every remaining type and target group, so every later phase-4 charge
//! amount is still computed correctly; `charge_all` charges through the last
//! `normalize.conflict-check` and returns that refusal only once every
//! phase-4 charge has been admitted. An earlier `Incomplete` still wins,
//! matching `:482`'s "a stage that reports a refusal ends checking: no
//! later stage runs or charges" — phase 5's own `normalize.declaration`/
//! `normalize.hash` charges never run once this refusal is pending.
#![allow(
    clippy::large_enum_variant,
    reason = "cold refusal path; ModelRefusalCause carries ProducerKeys inline"
)]
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries ProducerKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Code;
use crate::model::accounting::{
    Charge, ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use crate::model::bundle::{
    Bundle, BundleRecord, FieldMemberRecord, GeneralizationRecord, ModelSelection,
    INTERFACE_VERSION_1_2_0, INTERFACE_VERSION_1_3_0,
};
use crate::model::conformance::generals_by_specific;
use crate::model::key::{
    digest_of, jcs_bytes, EffectiveDeclarationPreimage, EffectiveId, Fact, ProducerKey,
    PRODUCER_DIGEST_DOMAIN, RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE,
};
use crate::value::length_amount;

/// Re-exported from [`crate::model::refusal`] (#141 finding 4): that module
/// is lower-level than this one so [`crate::model::key`] can depend on the
/// cause type without a cycle. This path is unchanged for every caller.
pub use crate::model::refusal::{ModelRefusalCause, OfferedSelection};

/// A refusal FR-150 normalization returns for a real defect (never a
/// resource limit; see [`Incomplete`] for that).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelRefusal {
    /// The stable top-level code.
    pub code: Code,
    /// The FR-150-specific cause tag (#141: a closed enum, not a bare
    /// string literal constructed independently at each call site).
    pub cause: ModelRefusalCause,
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
                    cause: ModelRefusalCause::UnsortedView {
                        at: pair[1].effective_id.clone(),
                    },
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
///
/// #141 P2: this bounds a distinct recursion (the FR-150 generalization
/// ancestor-path walk over a [`Bundle`]) from [`crate::value::MAX_CHECKING_DEPTH`]
/// (Complete-V1 expression-checking recursion, `src/value/expression/check.rs`).
/// Both happen to be 128 because both were
/// chosen as "a safe bound well inside the host stack" for their own
/// recursion, not because one normatively constrains the other; nothing in
/// FR-150 or FR-143 ties a model's generalization depth to an expression's
/// checking depth. They are independent constants that coincide in value,
/// not one limit duplicated.
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
                cause: ModelRefusalCause::GeneralizationDepthExceeded {
                    root: root_key.clone(),
                },
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
                cause: ModelRefusalCause::SpecializationCycle {
                    ancestor: ancestor_key.clone(),
                    via: record.key.clone(),
                },
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
    /// Every phase-4 `normalize.redefinition-check` charge's own exact
    /// `work_units` amount (`m + r`, `value-accounting.md:455`), one entry
    /// per redefinition record in the entire bundle (field and operation
    /// alike), ascending by the record's own producer key — computed once,
    /// bundle-wide, in `build()` itself, never once per (record, effective
    /// type reaching it) pair; replayed by
    /// `charge_all`.
    redefinition_check_work: Vec<u64>,
    /// The count of phase-4 redefine facts (`RULE_REDEFINE`) awaiting their
    /// own `normalize.fact` charge: `quire.model.normalize.redefine/v1`
    /// derives "one fact on (T, redefining feature) and one on (T, redefined
    /// feature)" per redefinition record reaching `T`
    /// (`model-complete.md:231`), for every record reaching `T`, contested
    /// or not — `apply_redefinitions` builds these two [`Fact`]s directly
    /// into the redefining and target members' own
    /// [`EffectiveDeclarationPreimage`]; this is only their count, charged
    /// as that many `normalize.fact` charges (order is never observable —
    /// each carries `LimitKind::DerivationFacts`'s own running total, not
    /// per-fact data — so no per-fact record or sort is kept here).
    /// `charge_all` charges these between `redefinition_check_work` and
    /// `conflict_check_work`: `value-accounting.md:455` puts
    /// `normalize.redefinition-check` "before its first `normalize.fact`"
    /// and `:456` puts `normalize.conflict-check` "after its last
    /// `normalize.fact`", so this field's charges belong strictly between
    /// the other two. Zero whenever phase 4's own resolution loop did not
    /// run at all (a phase-2/3 fact budget already exhausted; see the
    /// module docs), exactly like `conflict_check_work` in that case.
    phase4_fact_count: u64,
    /// Every phase-4 `normalize.conflict-check` charge's own exact
    /// `work_units` amount (`Σ (c − 1) × f(o)`, `value-accounting.md:456`),
    /// one entry per (effective type, redefined member) group with `c >= 2`
    /// redefiners resolved across every type — a group with a single
    /// redefiner needs no entry at all (`value-accounting.md:456`'s own
    /// `c >= 2` condition); replayed by `charge_all`.
    conflict_check_work: Vec<u64>,
    /// Whichever phase-4 refusal — `derivation-conflict`,
    /// `redefinition-target`, or `RedefinitionUnreachable` — ranks earliest
    /// in charge order (`record_phase4_refusal`'s own doc), held here rather
    /// than returned by `build` (see the module docs): `apply_redefinitions`
    /// keeps resolving every remaining type and target group after finding
    /// one, so every later phase-4 charge amount above is still computed
    /// correctly, and `charge_all` reports this refusal only once every
    /// phase-4 charge is admitted.
    phase4_refusal: Option<ModelRefusal>,
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
                    cause: ModelRefusalCause::UnknownOwner {
                        member: member.key.clone(),
                        owner: member.owner.clone(),
                    },
                    detail: format!(
                        "field member {} names owner {}, which is not a declared object type",
                        member.key.identity, member.owner.identity
                    ),
                });
            }
            BundleRecord::Generalization(general) if !index.types.contains(&general.specific) => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownSpecific {
                        generalization: general.key.clone(),
                        specific: general.specific.clone(),
                    },
                    detail: format!(
                        "generalization {} names specific {}, which is not a declared object type",
                        general.key.identity, general.specific.identity
                    ),
                });
            }
            BundleRecord::Generalization(general) if !index.types.contains(&general.general) => {
                return Err(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownGeneral {
                        generalization: general.key.clone(),
                        general: general.general.clone(),
                    },
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
                    cause: ModelRefusalCause::UnknownOwner {
                        member: op.key.clone(),
                        owner: op.owner.clone(),
                    },
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
                            cause: ModelRefusalCause::UnknownValueType {
                                operation: op.key.clone(),
                                parameter: Some(parameter.key.clone()),
                                value_type: parameter.value_type.clone(),
                            },
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
                            cause: ModelRefusalCause::UnknownValueType {
                                operation: op.key.clone(),
                                parameter: None,
                                value_type: result.value_type.clone(),
                            },
                            detail: format!(
                                "operation {} result names value type {}, which is not a declared type",
                                op.key.identity, result.value_type.identity
                            ),
                        });
                    }
                }
                for field in &op.effect.modifies {
                    if !index.field_member_keys.contains(field) {
                        return Err(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: ModelRefusalCause::UnknownFieldWrite {
                                operation: op.key.clone(),
                                field: field.clone(),
                            },
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
                            cause: ModelRefusalCause::UnknownEffectType {
                                operation: op.key.clone(),
                                type_name: target.clone(),
                            },
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
                    cause: ModelRefusalCause::UnknownOwner {
                        member: redefinition.key.clone(),
                        owner: redefinition.owner.clone(),
                    },
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
                            cause: ModelRefusalCause::UnknownMember {
                                record: redefinition.key.clone(),
                                member: member.clone(),
                            },
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
                    cause: ModelRefusalCause::UnknownOwner {
                        member: subsetting.key.clone(),
                        owner: subsetting.owner.clone(),
                    },
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
                            cause: ModelRefusalCause::UnknownMember {
                                record: subsetting.key.clone(),
                                member: member.clone(),
                            },
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
    // Phase 3's own ancestor paths, retained per type (rather than dropped
    // at the end of each iteration below) so phase 4 -- now its own pass,
    // run only after every type's phase 2/3 has finished -- can reuse them
    // without recomputing (PR #140 F10) for a `type_key` whose own
    // iteration already ran.
    let mut type_paths: HashMap<ProducerKey, Vec<AncestorPath>> = HashMap::new();

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

        type_paths.insert(type_key.clone(), paths);
    }

    // Phase 4: every type's own field-redefinition conflicts, run only now
    // that every type's phase 2/3 has finished (see the module docs): a
    // redefinition's owner can sort after `type_key` in `type_keys`'
    // ascending order, and its own effective member count (`m`,
    // `value-accounting.md:455`) and type derivation-fact count (`f(o)`,
    // `:456`) -- both read directly out of `member_preimages`/
    // `type_preimages` below -- must already exist regardless of which
    // type's turn happens to reach it first.
    let mut member_counts_by_owner: HashMap<ProducerKey, u64> = HashMap::new();
    for (owner, _member) in member_preimages.keys() {
        *member_counts_by_owner.entry(owner.clone()).or_insert(0) += 1;
    }
    // `m` counts *every* effective member, not only fields (QSL #145): each
    // type's own directly-declared operation
    // members, plus every operation directly declared on a proper ancestor
    // reached along that type's own phase-3 `type_paths` -- the same
    // "direct at this type, or direct at some ancestor `type_paths` already
    // reaches" shape `member_preimages` above builds for fields, just
    // counted rather than given a full preimage (no phase 5 view entry, no
    // redefinition resolution, exists here for `m` alone).
    let mut operations_by_owner: HashMap<ProducerKey, Vec<ProducerKey>> = HashMap::new();
    for record in &bundle.records {
        if let BundleRecord::OperationMember(operation) = record {
            operations_by_owner
                .entry(operation.owner.clone())
                .or_default()
                .push(operation.key.clone());
        }
    }
    for type_key in &type_keys {
        let mut effective_operations: HashSet<ProducerKey> = HashSet::new();
        if let Some(direct) = operations_by_owner.get(type_key) {
            effective_operations.extend(direct.iter().cloned());
        }
        if let Some(paths) = type_paths.get(type_key) {
            for ancestor in paths {
                if let Some(inherited) = operations_by_owner.get(&ancestor.ancestor_key) {
                    effective_operations.extend(inherited.iter().cloned());
                }
            }
        }
        if !effective_operations.is_empty() {
            *member_counts_by_owner.entry(type_key.clone()).or_insert(0) +=
                length_amount(effective_operations.len());
        }
    }
    let type_fact_counts: HashMap<ProducerKey, u64> = type_preimages
        .iter()
        .map(|(key, preimage)| (key.clone(), length_amount(preimage.derivation.len())))
        .collect();
    // Every owner's own proper-ancestor set, derived from phase 3's own
    // `type_paths` (populated above for every type in the bundle) rather
    // than a fresh, separately-bounded walk (QSL #145): `ancestor_key` is
    // exactly the proper-ancestor identity
    // `crate::model::conformance::ancestor_closure` used to compute with its
    // own `MAX_CONFORMANCE_DEPTH` breadth ceiling, so deduplicating those
    // same keys here needs no walk of its own and has no ceiling to exceed.
    // Computed once, build-wide (QSL #145), not once per (type, target)
    // group.
    let owner_ancestor_sets: HashMap<ProducerKey, HashSet<ProducerKey>> = type_paths
        .iter()
        .map(|(owner, paths)| {
            let ancestors: HashSet<ProducerKey> =
                paths.iter().map(|path| path.ancestor_key.clone()).collect();
            (owner.clone(), ancestors)
        })
        .collect();

    // `normalize.redefinition-check` (`value-accounting.md:455`) charges
    // `m + r` once per redefinition record in the *entire bundle* -- field
    // and operation alike -- ascending by the record's own producer key,
    // `r` the count of records already checked before it in this same
    // bundle-wide sequence. Computed once here, before the per-type phase-4
    // loop below even starts: the record is the spec's own priced unit,
    // tested once against its own owning type, never once per (record,
    // effective type reaching it) pair a per-type loop would recompute it
    // at.
    let mut all_redefinition_records: Vec<_> = bundle
        .records
        .iter()
        .filter_map(|record| match record {
            BundleRecord::Redefinition(redefinition) => Some(redefinition),
            _ => None,
        })
        .collect();
    all_redefinition_records.sort_by(|a, b| a.key.cmp(&b.key));
    let mut redefinition_check_work: Vec<u64> = Vec::new();
    for (r, redefinition) in all_redefinition_records.iter().enumerate() {
        let m = member_counts_by_owner
            .get(&redefinition.owner)
            .copied()
            .unwrap_or(0);
        redefinition_check_work.push(m.saturating_add(length_amount(r)));
    }

    let mut conflict_check_work: Vec<u64> = Vec::new();
    let mut phase4_fact_count: u64 = 0;
    let mut phase4_refusal: Option<ModelRefusal> = None;
    let mut phase4_refusal_rank: Option<(u8, Option<ProducerKey>, ProducerKey)> = None;
    let mut accounting = Phase4Accounting {
        type_fact_counts: &type_fact_counts,
        owner_ancestor_sets: &owner_ancestor_sets,
        conflict_check_work: &mut conflict_check_work,
        phase4_fact_count: &mut phase4_fact_count,
        refusal: &mut phase4_refusal,
        refusal_rank: &mut phase4_refusal_rank,
    };
    // A truncated phase-3 path set (a tight fact budget already exceeded by
    // the time every type's own phase 2/3 above has run) cannot resolve
    // phase-4 dominance honestly: an owner ancestry `owner_ancestor_sets`
    // built from it could look incomplete rather than merely undominated,
    // and wrongly derive a `derivation-conflict` refusal no complete build
    // would report. Skipping phase-4 resolution here is safe exactly
    // because `charge_all`'s own replay of the phase 2/3 facts that
    // triggered the truncation runs, in charge order, before it ever
    // reaches phase 4's own charges below -- it already reports the correct
    // `Incomplete` there, never consulting `redefinition_check_work`/
    // `conflict_check_work` computed from truncated data (QSL #145).
    if !fact_budget_exceeded(limits, facts_so_far) {
        for type_key in &type_keys {
            let paths = type_paths
                .get(type_key)
                .expect("populated in the loop above");
            apply_redefinitions(
                bundle,
                &index,
                type_key,
                paths,
                &mut member_preimages,
                &mut hidden,
                &mut accounting,
            );
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
        redefinition_check_work,
        phase4_fact_count,
        conflict_check_work,
        phase4_refusal,
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

/// Every cross-type input and output `apply_redefinitions` needs beyond its
/// own `type_key`'s local bookkeeping, grouped into one `&mut` borrow
/// (rather than five separate parameters) so the function stays within
/// clippy's `too_many_arguments` ceiling. `type_fact_counts`/
/// `owner_ancestor_sets` are build-wide, read-only lookups (`f(o)`,
/// `value-accounting.md:456`, and each owner's own proper-ancestor set);
/// `conflict_check_work`, `phase4_fact_count` and `refusal` are build-wide
/// accumulators mutated across every `type_key`'s own call.
/// `normalize.redefinition-check`'s own charge sequence — and the `m` it
/// needs — is not built here at all: `build` computes it once, bundle-wide,
/// before any `apply_redefinitions` call.
struct Phase4Accounting<'a> {
    type_fact_counts: &'a HashMap<ProducerKey, u64>,
    owner_ancestor_sets: &'a HashMap<ProducerKey, HashSet<ProducerKey>>,
    conflict_check_work: &'a mut Vec<u64>,
    /// The running count of phase-4 redefine facts awaiting their own
    /// `normalize.fact` charge (see [`Built::phase4_fact_count`]'s own
    /// doc) — field redefinition only, exactly like `member_preimages`/
    /// `hidden`: an operation-member redefinition edge never reaches
    /// [`Fact`] construction here at all (see the module docs), so it
    /// contributes nothing to this count.
    phase4_fact_count: &'a mut u64,
    /// The phase-4 refusal this pass reports (see [`Built::phase4_refusal`]'s
    /// own doc): whichever refusal phase 4 exposes ranks earliest in charge
    /// order, not merely the first one this pass's own per-`type_key` walk
    /// happens to encounter. Set only by `record_phase4_refusal`, which also
    /// maintains `refusal_rank` alongside it.
    refusal: &'a mut Option<ModelRefusal>,
    /// The charge-order rank of whatever refusal `refusal` currently holds
    /// (see `record_phase4_refusal`'s own doc for what a rank is and how
    /// candidates are compared against it).
    refusal_rank: &'a mut Option<(u8, Option<ProducerKey>, ProducerKey)>,
}

/// Charge-order stage for `record_phase4_refusal`'s own rank key: lower
/// values rank earlier. `normalize.redefinition-check` charges every
/// redefinition record before phase 4's own `normalize.fact` charges, which
/// in turn precede every `normalize.conflict-check` charge
/// (`value-accounting.md:455`, `:456`), so a redefinition-check-stage
/// refusal always outranks a conflict-check-stage refusal, regardless of
/// which type or target group this pass's own per-`type_key` walk happens
/// to reach first.
const REDEFINITION_CHECK_STAGE: u8 = 0;
/// See [`REDEFINITION_CHECK_STAGE`]'s own doc.
const CONFLICT_CHECK_STAGE: u8 = 1;

/// Records `refusal` as `accounting`'s own phase-4 refusal only if `(stage,
/// type_key.cloned(), key.clone())` ranks earlier, in charge order, than
/// whatever rank `accounting.refusal_rank` already holds — never merely
/// because `accounting.refusal` is still `None`: `stage` is
/// [`REDEFINITION_CHECK_STAGE`] or [`CONFLICT_CHECK_STAGE`], and
/// `(type_key, key)` is the pair the exposing stage itself ascends by.
/// `normalize.redefinition-check` (`value-accounting.md:455`) ascends by
/// the redefinition record's own producer key alone, so redefinition-check
/// callers pass `type_key: None` and `key` as that record's key.
/// `normalize.conflict-check` (`:456`) charges "per type, in `type_keys`
/// order, and only then by target" (`model-complete.md:160` puts the
/// owning type first in the effective member key), so conflict-check
/// callers pass `type_key: Some(&the contested group's own type_key)` and
/// `key` as the contested target's own key — `Option`, not a placeholder
/// value, because `None` always sorts before `Some(_)`, matching
/// `REDEFINITION_CHECK_STAGE` (`0`) always sorting before
/// `CONFLICT_CHECK_STAGE` (`1`) regardless of any particular type's own
/// identity. A candidate that does not rank earlier changes nothing:
/// `:481`'s "checking is exhaustive within a stage" only requires every
/// phase-4 charge to still run, not that the recorded refusal track the
/// order this pass's own walk happens to visit types and target groups in.
fn record_phase4_refusal(
    accounting: &mut Phase4Accounting<'_>,
    stage: u8,
    type_key: Option<&ProducerKey>,
    key: &ProducerKey,
    refusal: ModelRefusal,
) {
    let rank = (stage, type_key.cloned(), key.clone());
    let replace = match accounting.refusal_rank.as_ref() {
        None => true,
        Some(current) => rank < *current,
    };
    if replace {
        *accounting.refusal_rank = Some(rank);
        *accounting.refusal = Some(refusal);
    }
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
///
/// Also appends this contested target group's own `normalize.conflict-check`
/// charge amount (`value-accounting.md:456`) to `conflict_check_work`,
/// replayed later by `charge_all` — `normalize.redefinition-check`'s own
/// charge sequence is `build`'s own bundle-wide pass, not this function's.
/// `type_fact_counts` supplies `f(o)` for any owner in the bundle, not just
/// `type_key` itself — `build` computes it
/// only after every type's own phase 2/3 has run (see the module docs) so
/// this is always a lookup, never a fresh walk. `owner_ancestor_sets` is
/// `build`'s own build-wide map of every owner's proper-ancestor set,
/// derived from phase 3's own `type_paths` (QSL #145) rather than a second,
/// separately bounded walk, and likewise computed once, build-wide, not
/// once per (type, target) group.
fn apply_redefinitions(
    bundle: &Bundle,
    index: &Index,
    type_key: &ProducerKey,
    paths: &[AncestorPath],
    member_preimages: &mut HashMap<(ProducerKey, ProducerKey), EffectiveDeclarationPreimage>,
    hidden: &mut HashSet<(ProducerKey, ProducerKey)>,
    accounting: &mut Phase4Accounting<'_>,
) {
    let mut owner_paths: HashMap<ProducerKey, Vec<ProducerKey>> = HashMap::new();
    owner_paths.insert(type_key.clone(), Vec::new());
    for ancestor in paths {
        owner_paths
            .entry(ancestor.ancestor_key.clone())
            .or_insert_with(|| ancestor.path.clone());
    }

    // Every redefinition record reachable at `type_key`, gathered flat (not
    // yet grouped by target) and split by member kind.
    // `normalize.redefinition-check`'s own charge sequence does not come
    // from either list: it is `build`'s own bundle-wide pass over every
    // redefinition record, field and operation alike; `all_edges`
    // (field-only) is scoped purely to this type's own conflict
    // *resolution*, exactly as the module docs describe. `operation_edges`
    // feeds only the contention *charge* below (QSL #145): operation-member
    // redefinition is still never resolved here —
    // `crate::model::conformance` resolves it
    // directly (see the module docs) — but a contested operation target
    // (`c >= 2`) still owes `normalize.conflict-check`'s own
    // `value-accounting.md:456` price, exactly as a contested field target
    // does.
    let mut all_edges: Vec<RedefinitionEdge> = Vec::new();
    let mut operation_edges: Vec<RedefinitionEdge> = Vec::new();
    for record in &bundle.records {
        let BundleRecord::Redefinition(redefinition) = record else {
            continue;
        };
        let is_field = index.field_member_keys.contains(&redefinition.redefining)
            && index.field_member_keys.contains(&redefinition.redefined);
        let is_operation = index
            .operation_member_keys
            .contains(&redefinition.redefining)
            && index
                .operation_member_keys
                .contains(&redefinition.redefined);
        if !is_field && !is_operation {
            continue;
        }
        let Some(path) = owner_paths.get(&redefinition.owner) else {
            // This redefinition's owner does not reach `type_key`.
            continue;
        };
        let edge = RedefinitionEdge {
            owner: redefinition.owner.clone(),
            redefining: redefinition.redefining.clone(),
            record_key: redefinition.key.clone(),
            target: redefinition.redefined.clone(),
            path: path.clone(),
        };
        if is_field {
            all_edges.push(edge);
        } else {
            operation_edges.push(edge);
        }
    }

    let mut groups: HashMap<ProducerKey, Vec<RedefinitionEdge>> = HashMap::new();
    for edge in all_edges {
        groups.entry(edge.target.clone()).or_default().push(edge);
    }

    let mut target_keys: Vec<ProducerKey> = groups.keys().cloned().collect();
    target_keys.sort();

    // `value-accounting.md:456`'s own charge order is a single ascending
    // pass "by effective member key" over every contested `(effective type,
    // redefined member)` reached by `c >= 2` records -- field and operation
    // targets interleaved by that one key, never field targets as a block
    // followed by operation targets as a block. Each loop below still
    // resolves (and, for fields, hides/derives) its own kind in its own
    // pass, but neither pushes its charge amount straight to
    // `accounting.conflict_check_work`; both collect `(target key, amount)`
    // here, and charges from both loops are sorted by effective member key
    // before being pushed, once, after both loops
    // (`value-accounting.md:456`).
    let mut conflict_charges: Vec<(ProducerKey, u64)> = Vec::new();

    'targets: for target_key in target_keys {
        let mut edges = groups.remove(&target_key).expect("just listed");
        edges.sort_by(|a, b| {
            a.owner
                .cmp(&b.owner)
                .then_with(|| a.record_key.cmp(&b.record_key))
        });

        // `Option<usize>`, not a bare `usize`: an ambiguous group (no edge
        // dominates every other) has no winner at all, but still owes every
        // charge below and still derives both facts per edge
        // (`model-complete.md:231`) -- the ambiguity is reported as a
        // refusal (`accounting.refusal`, below) once phase 4's own charges
        // finish, per `value-accounting.md:481`'s "checking is exhaustive
        // within a stage" (see the module docs), not by aborting this
        // function early. `None` hides every edge and the target itself:
        // harmless, since a build that ever sets `accounting.refusal`
        // never returns `Completed` (see `charge_all`).
        let winner_index: Option<usize> = if edges.len() < 2 {
            // A single redefiner has nothing to dominate: no ancestor
            // closure is computed at all, and no `normalize.conflict-check`
            // charge either (`value-accounting.md:456`'s own `c >= 2`
            // condition; QSL #145): computing a closure regardless of
            // `edges.len()` is what made a wide
            // (128+ direct generalizations) but uncontested bundle wrongly
            // refuse `conformance-depth`.
            Some(0)
        } else {
            let c = length_amount(edges.len());

            // `value-accounting.md:456`: `work_units += Σ (c − 1) × f(o)`,
            // summed over the `c` redefining owners `o` -- once per edge,
            // repeating a shared owner's own `f(o)` once for every edge it
            // owns exactly as written, not once per *distinct* owner (the
            // `owner_ancestor_sets` lookup below is this rung's own
            // optimization of the *walk*, never a change to what is
            // priced).
            let fact_total: u64 = edges
                .iter()
                .map(|edge| {
                    accounting
                        .type_fact_counts
                        .get(&edge.owner)
                        .copied()
                        .unwrap_or(0)
                })
                .sum();
            conflict_charges.push((
                target_key.clone(),
                fact_total.saturating_mul(c.saturating_sub(1)),
            ));

            // `owner_ancestor_sets` already holds every owner in the bundle's
            // own proper-ancestor set (QSL #145: derived once, build-wide,
            // from phase 3's own `type_paths` rather than a fresh,
            // separately bounded walk here) -- the
            // winner search and its undominated-owner fallback below each
            // compare every edge's owner against every other edge's owner,
            // but TC-196 R07's own documented shape — several redefining
            // members sharing one contending owner — means distinct owners
            // are frequently far fewer than edges, and the same owner
            // recurs across many types and targets within one build; both
            // are plain `O(1)` set lookups against the already-built map.
            let mut winner: Option<usize> = None;
            for i in 0..edges.len() {
                let mut dominates_all = true;
                for j in 0..edges.len() {
                    if i == j {
                        continue;
                    }
                    if !owner_dominates(
                        accounting.owner_ancestor_sets,
                        &edges[i].owner,
                        &edges[j].owner,
                    ) {
                        dominates_all = false;
                        break;
                    }
                }
                if dominates_all {
                    winner = Some(i);
                    break;
                }
            }

            if winner.is_none() {
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
                //
                // `normalize.conflict-check` exposes this ambiguity
                // (`value-accounting.md:456`), so `record_phase4_refusal`
                // below ranks it by `CONFLICT_CHECK_STAGE` and this group's
                // own `(type_key, target_key)` — `:456`'s "per type, in
                // `type_keys` order, and only then by target"
                // (`model-complete.md:160` puts the owning type first in the
                // effective member key) — rather than keeping whichever
                // ambiguity this pass's own per-`type_key` walk happens to
                // reach first (see the module docs and
                // `record_phase4_refusal`'s own doc).
                let mut most_derived: Vec<&RedefinitionEdge> = Vec::new();
                for edge in &edges {
                    let mut dominated_by_another = false;
                    for other in &edges {
                        if other.owner.identity == edge.owner.identity {
                            continue;
                        }
                        if owner_dominates(
                            accounting.owner_ancestor_sets,
                            &other.owner,
                            &edge.owner,
                        ) {
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
                let candidate = if same_owner {
                    let mut redefiners: Vec<String> = most_derived
                        .iter()
                        .map(|edge| edge.redefining.identity.clone())
                        .collect();
                    redefiners.sort();
                    ModelRefusal {
                        code: Code::InvalidModelBinding,
                        cause: ModelRefusalCause::RedefinitionTarget,
                        detail: format!(
                            "{} declares {} redefining members ({}) that all redefine {}, with no single valid target",
                            most_derived[0].owner.identity,
                            most_derived.len(),
                            redefiners.join(", "),
                            target_key.identity
                        ),
                    }
                } else {
                    let edge_paths: Vec<String> = edges
                        .iter()
                        .map(|edge| {
                            let mut path: Vec<String> =
                                edge.path.iter().map(|key| key.identity.clone()).collect();
                            path.push(edge.record_key.identity.clone());
                            path.push(target_key.identity.clone());
                            format!("[{}]", path.join(", "))
                        })
                        .collect();
                    ModelRefusal {
                        code: Code::InvalidModelBinding,
                        cause: ModelRefusalCause::DerivationConflict {
                            type_: type_key.clone(),
                            member: target_key.clone(),
                            redefiners: edges.iter().map(|edge| edge.redefining.clone()).collect(),
                        },
                        detail: format!(
                            "type {} has {} undominated redefinitions of {}: {}",
                            type_key.identity,
                            edges.len(),
                            target_key.identity,
                            edge_paths.join(" and ")
                        ),
                    }
                };
                record_phase4_refusal(
                    accounting,
                    CONFLICT_CHECK_STAGE,
                    Some(type_key),
                    &target_key,
                    candidate,
                );
            }
            winner
        };

        let member_key = (type_key.clone(), target_key.clone());
        if !member_preimages.contains_key(&member_key) {
            // `normalize.redefinition-check` exposes this refusal
            // (`value-accounting.md:455`), so `record_phase4_refusal` below
            // ranks it by `REDEFINITION_CHECK_STAGE` and the least of this
            // group's own redefinition records' producer keys — `:455`'s
            // own "ascending by the record's own producer key" order; every
            // edge in this group shares the same unreachable target, so
            // whichever of them sorts first is the one that order would
            // check first. Held in `accounting.refusal` rather than
            // returned here (see the module docs), so every remaining type
            // and target group is still resolved and every later phase-4
            // charge amount is still computed correctly.
            //
            // This exact record can also be reached, and fail the identical
            // check for the identical reason, at more than one `type_key`
            // whenever a descendant of `least_edge.owner` also inherits it
            // (both compute the same `least_edge.record_key`, so they rank
            // identically) -- naming `least_edge.owner` rather than
            // `type_key` keeps the reported refusal the same regardless of
            // which of those tied candidates this pass happens to keep.
            let least_edge = edges
                .iter()
                .min_by(|a, b| a.record_key.cmp(&b.record_key))
                .expect("a target group always has at least one edge");
            record_phase4_refusal(
                accounting,
                REDEFINITION_CHECK_STAGE,
                None,
                &least_edge.record_key,
                ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::RedefinitionUnreachable {
                        member: target_key.clone(),
                        owner: least_edge.owner.clone(),
                    },
                    detail: format!(
                        "redefinition target {} is not an effective member of {}",
                        target_key.identity, least_edge.owner.identity
                    ),
                },
            );
            continue 'targets;
        }

        for (i, edge) in edges.iter().enumerate() {
            let mut inputs = edge.path.clone();
            inputs.push(edge.record_key.clone());
            inputs.push(edge.target.clone());

            let redefining_key = (type_key.clone(), edge.redefining.clone());
            let entry = match member_preimages.get_mut(&redefining_key) {
                Some(entry) => entry,
                None => {
                    // Same deferred-refusal treatment as the target check
                    // above: this redefining member is not itself an
                    // effective member of `type_key`, which
                    // `normalize.redefinition-check` also exposes — ranked
                    // by this one edge's own `record_key`, since unlike the
                    // target check above this failure is specific to a
                    // single record, not shared by the whole group.
                    //
                    // This edge's `record_key` can likewise be reached, and
                    // fail the identical check for the identical reason, at
                    // more than one `type_key` whenever a descendant of
                    // `edge.owner` also inherits it (both tie at the same
                    // `record_key`) -- naming `edge.owner` rather than
                    // `type_key` keeps the reported refusal the same
                    // regardless of which of those tied candidates this pass
                    // happens to keep, matching the target check above.
                    record_phase4_refusal(
                        accounting,
                        REDEFINITION_CHECK_STAGE,
                        None,
                        &edge.record_key,
                        ModelRefusal {
                            code: Code::DanglingReference,
                            cause: ModelRefusalCause::RedefinitionUnreachable {
                                member: edge.redefining.clone(),
                                owner: edge.owner.clone(),
                            },
                            detail: format!(
                                "redefining member {} is not an effective member of {}",
                                edge.redefining.identity, edge.owner.identity
                            ),
                        },
                    );
                    // `continue 'targets` here leaves this group
                    // half-processed — any earlier edge's own derive facts
                    // in this loop are already counted, and the winning
                    // edge's own target is not hidden — but that is
                    // harmless: the outcome is a refusal regardless, once
                    // every remaining phase-4 charge is admitted (see the
                    // module docs).
                    continue 'targets;
                }
            };
            let ordinal = entry.derivation.len();
            entry.derivation.push(Fact {
                ordinal,
                rule: RULE_REDEFINE,
                inputs: inputs.clone(),
            });
            // The redefining feature's own fact, awaiting its
            // `normalize.fact` charge alongside phase 2/3's
            // (`Built::phase4_fact_count`'s own doc;
            // `model-complete.md:231`'s "one fact on (T, redefining
            // feature)").
            *accounting.phase4_fact_count += 1;
            if winner_index != Some(i) {
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
            // The redefined (target) feature's own fact —
            // `model-complete.md:231`'s "one ... on (T, redefined feature)"
            // — one entry per redefinition record reaching this target, not
            // one per target: a `c >= 2` group counts one here for every
            // contesting edge.
            *accounting.phase4_fact_count += 1;
        }
        hidden.insert(member_key);
    }

    // Operation-member redefinition contention: never resolved here (see
    // the module docs — `crate::model::conformance` decides which operation
    // redefiner wins), but a contested operation target still owes
    // `normalize.conflict-check`'s own `Σ (c − 1) × f(o)` price
    // (`value-accounting.md:456`) whenever `c >= 2`, exactly like a
    // contested field target (QSL #145).
    // No `member_preimages`/`hidden` state is touched here: operation
    // members never enter `member_preimages` (see the module docs), so
    // there is nothing here for this loop to resolve or hide.
    let mut operation_groups: HashMap<ProducerKey, Vec<RedefinitionEdge>> = HashMap::new();
    for edge in operation_edges {
        operation_groups
            .entry(edge.target.clone())
            .or_default()
            .push(edge);
    }
    let mut operation_target_keys: Vec<ProducerKey> = operation_groups.keys().cloned().collect();
    operation_target_keys.sort();
    for target_key in operation_target_keys {
        let edges = operation_groups.remove(&target_key).expect("just listed");
        if edges.len() < 2 {
            // A single redefiner has nothing to contest: no charge, matching
            // `value-accounting.md:456`'s own `c >= 2` condition.
            continue;
        }
        let c = length_amount(edges.len());
        let fact_total: u64 = edges
            .iter()
            .map(|edge| {
                accounting
                    .type_fact_counts
                    .get(&edge.owner)
                    .copied()
                    .unwrap_or(0)
            })
            .sum();
        conflict_charges.push((
            target_key.clone(),
            fact_total.saturating_mul(c.saturating_sub(1)),
        ));
    }

    conflict_charges.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, amount) in conflict_charges {
        accounting.conflict_check_work.push(amount);
    }
}

/// Whether `p_owner` strictly dominates `q_owner`: `p_owner` is a proper
/// descendant of `q_owner` in `closures` (`build`'s own `owner_ancestor_sets`,
/// derived from phase 3's own `type_paths` — QSL #145). An `O(1)` set
/// lookup against an already-computed set, never
/// a fresh graph walk — the `O(|edges|^2)` pair enumeration this function is
/// called from stays cheap because `closures` was already built once,
/// build-wide, before any `apply_redefinitions` call.
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
/// `charge_all`'s own denial: an ordinary metered [`Incomplete`], or
/// `built`'s own [`Built::phase4_refusal`] reported once every phase-4
/// charge (through the last `normalize.conflict-check`) is admitted (see the
/// module docs). `From<Incomplete>` lets `meter.charge(...)?` keep working
/// unchanged throughout `charge_all`.
enum ChargeAllDenial {
    Incomplete(Incomplete),
    Refused(ModelRefusal),
}

impl From<Incomplete> for ChargeAllDenial {
    fn from(incomplete: Incomplete) -> Self {
        ChargeAllDenial::Incomplete(incomplete)
    }
}

fn charge_all(bundle: &Bundle, built: &Built, meter: &mut Meter) -> Result<(), ChargeAllDenial> {
    // #141 F11: only the running position (`index + 1`) is charged, never a
    // key's value or its relative order, so collecting and sorting a
    // `Vec<ProducerKey>` just to throw the order away was dead work.
    for index in 0..bundle.records.len() {
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

    for work in &built.redefinition_check_work {
        meter.charge(Charge::new(ChargePoint::NormalizeRedefinitionCheck).work(*work))?;
    }

    // Phase-4 redefine facts are derivation facts too
    // (`value-accounting.md:453`) and are charged as `normalize.fact` here,
    // continuing the same `fact_count`/`derivation_facts` sequence phase
    // 2/3 already ran -- strictly between `normalize.redefinition-check`
    // ("before its first `normalize.fact`", `:455`) and
    // `normalize.conflict-check` ("after its last `normalize.fact`",
    // `:456`). See `Built::phase4_fact_count`'s own doc for why these carry
    // no `normalize.cycle-check` (that charge is phase-3 type-level facts
    // only) and no per-fact record (order is never observable, only the
    // running `derivation_facts` total).
    for _ in 0..built.phase4_fact_count {
        fact_count += 1;
        meter.charge(
            Charge::new(ChargePoint::NormalizeFact).size(LimitKind::DerivationFacts, fact_count),
        )?;
    }

    for work in &built.conflict_check_work {
        meter.charge(Charge::new(ChargePoint::NormalizeConflictCheck).work(*work))?;
    }

    // `value-accounting.md:481`'s "checking is exhaustive within a stage":
    // every phase-4 charge above (`normalize.redefinition-check`, every
    // phase-4 `normalize.fact`, every `normalize.conflict-check`) is
    // admitted before `built.phase4_refusal` (see the module docs) is
    // reported -- an earlier `Incomplete` already returned via `?` above
    // wins instead. `:482`'s "a stage that reports a refusal ends checking:
    // no later stage runs or charges" -- phase 5's own charges below never
    // run once this refusal is reported.
    if let Some(refusal) = &built.phase4_refusal {
        return Err(ChargeAllDenial::Refused(refusal.clone()));
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
            for field in &record.effect.modifies {
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
            cause: ModelRefusalCause::UnsupportedWire {
                version: version.clone(),
            },
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
                    cause: ModelRefusalCause::WrongModelSelection { key: key.clone() },
                    detail: format!("{} has no producer revision", key.identity),
                });
            }
            if key.digest.domain != PRODUCER_DIGEST_DOMAIN {
                return Err(ModelRefusal {
                    code: Code::StaleDependency,
                    cause: ModelRefusalCause::DigestDomainMismatch {
                        key: key.clone(),
                        domain: key.digest.domain.clone(),
                    },
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
    // #141 F11: same dead work as `charge_all` above — the sorted order was
    // never used, only the running position.
    for index in 0..bundle.records.len() {
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
            cause: ModelRefusalCause::UnsuppliedProducerRecord,
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
        Err(ChargeAllDenial::Incomplete(incomplete)) => NormalizeOutcome::Incomplete(incomplete),
        Err(ChargeAllDenial::Refused(refusal)) => NormalizeOutcome::Refused(refusal),
    };
    (outcome, meter)
}

/// The object universe `bundle` normalizes to, independent of `charge_all`'s
/// bookkeeping (test and caller convenience; recomputes via [`build`], under
/// [`ModelNormalizationLimits::UNLIMITED`]). Under `UNLIMITED` nothing ever
/// runs out, so `build`'s own deferred `Built::phase4_refusal` (see the
/// module docs) is surfaced here directly rather than replayed through
/// `charge_all`.
pub fn object_universe(bundle: &Bundle) -> Result<ObjectUniverse, ModelRefusal> {
    let built = build(bundle, &ModelNormalizationLimits::UNLIMITED)?;
    if let Some(refusal) = built.phase4_refusal {
        return Err(refusal);
    }
    Ok(built.universe)
}
