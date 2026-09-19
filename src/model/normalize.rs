// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150 model normalization: phases 1 (decode), 2 (qualify), 3 (inherit),
//! 4 (`quire.model.normalize.redefine/v1`, explicit field redefinition) and
//! 5 (canonicalize).
//!
//! Phase 4 here covers exactly TC-195 N06's shape: **field** members' own
//! explicit `redefines` property (`model-complete.md`:159/160), resolved by
//! the same proper-descendant dominance FR-151 dispatch later reuses (a
//! redefining owner that is a proper descendant of every other contesting owner wins
//! outright; two or more undominated owners refuse `derivation-conflict`).
//! `quire.model.normalize.subset/v1` (explicit subsetting) derives no
//! replacement member — FR-150 says so explicitly ("subsetting never
//! conflicts with redefinition because it derives no replacement") — so a
//! field member's own `subsets` property (`model-complete.md`:159) carries
//! no normalization derivation at all; it is checked directly by
//! `crate::model::conformance` (the static `subsetting-type` axis) and, at
//! runtime, by FR-153's `binding.subset-value` check (`subsetting-violation`)
//! — FR-153 territory, not yet implemented anywhere in this crate (see
//! `crate::model::conformance`'s own module doc).
//! **Operation-member** redefinition (TC-196 R02–R08) also builds no phase
//! here: `crate::model::conformance` constructs its own
//! [`EffectiveDeclarationPreimage`] directly over the domain package's
//! [`crate::model::domain_package::OperationMemberRecord`]'s own `redefines`
//! property for FR-151's conformance checking, so this pass — which exists to
//! grow [`EffectiveView`] itself — has nothing to add for operations. This
//! split is a scope decision recorded here, not a silent gap: extending
//! this pass to also normalize operation-member redefinition into the view
//! is future work, tracked by the PR that made this decision. This scope
//! decision covers only the *view* this pass grows: the `m + r`
//! `normalize.redefinition-check` charge below still prices every
//! redefining member and every effective member the spec names,
//! operation and field alike (QSL #145), and the
//! `Σ (c − 1) × f(o)` `normalize.conflict-check` charge below prices a
//! contested operation target (`c >= 2` redefining operation members
//! reaching the same target) exactly as it prices a contested field target
//! (QSL #145) — neither charge is itself
//! an operation-redefinition normalization pass. Detecting and *resolving*
//! a contested operation target — deciding which of several competing
//! operation redefiners wins, the way this pass's own field-redefinition
//! dominance search does — is not implemented anywhere in this crate today:
//! `crate::model::conformance`'s own module doc names the same gap for its
//! side of this split. Remaining work: #173.
//!
//! This engine takes a [`DomainPackage`] value the caller constructs; it holds no
//! ambient registry. Every identity is SHA-256 over RFC 8785 JCS bytes of a
//! preimage built here, replayed exactly for the same [`DomainPackage`] and
//! [`ModelNormalizationLimits`] — the same inputs always retrace the same
//! derivation and the same bytes.
//!
//! [`build`] enumerates ancestor paths and derivation facts under a fact
//! budget derived from `limits` itself (PR #140 F1): a diamond
//! generalization graph produces an ancestor-path count exponential in
//! depth, so an adversarial domain package enumerated without bound before any
//! charge is consulted can exhaust memory long before [`charge_all`] gets a
//! chance to deny anything — `ModelNormalizationLimits` protects nothing if
//! it is only consulted after the fact. `remaining_fact_budget` and
//! `fact_budget_exceeded` both only ever *overestimate* remaining capacity
//! (never underestimate it), so a domain package that legitimately completes under
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
//! least key. [`ancestor_paths`] charges every closing extension it finds
//! (`value-accounting.md:491`: "a closing extension is charged even when
//! its cycle's edge set was already reported") and keeps walking past it —
//! this frame's remaining siblings, every other frame still on its stack,
//! and, in `build`'s own per-type loop, every remaining type — rather than
//! stopping at the first one, so R01's own six-`normalize.cycle-check`
//! accounting across both types' walks is reproduced exactly (QSL #193).
//! `build` deduplicates the refusals themselves by edge set, keeping only
//! each distinct cycle's own first (charge-order-earliest) closing
//! extension, per `model-complete.md`:292-295's "each cycle is refused
//! exactly once" — collected in `Built::phase3_refusals` and reported only
//! once every phase 2/3 charge has admitted (`:505-511`'s "every refusal
//! that work exposes is reported, in charge order, when its stage ends").
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
//! redefining member in the *entire domain package* — field or operation alike —
//! ascending by the member's own key (there is no separate redefinition-record
//! key to sort by under QSpec's inline shape), where `m` is the number of
//! the member's own owning type's effective members (field and operation
//! together) and `r` is the count of redefining members already checked
//! before it in that same ascending, domain-package-wide sequence. This is computed
//! exactly once, in `build()`, before the per-type phase-4 loop runs at
//! all: the unit the spec prices is the redefining member itself, tested once against
//! its own owning type, never once per (member, effective type reaching
//! it) pair the previous per-type loop recomputed it at, which overcharged
//! any member reachable from more than one effective type and reset `r` at
//! each type instead of counting across the whole domain package.
//! `normalize.conflict-check` (`:456`) charges `Σ (c − 1) × f(o)` per
//! contested target with `c >= 2` redefiners, summed once per redefining
//! edge (not once per *distinct* owner: the memoization above is this
//! rung's own optimization, never a change to what is priced), where `f(o)`
//! is owner `o`'s own count of type derivation facts (qualify plus
//! inherit). `m` and `f(o)` are read directly from `build`'s own phase 2/3
//! results (`member_preimages`/`type_preimages`, extended for `m` with the
//! domain package's own directly-declared and inherited operation members), which
//! is why phase 4 now runs as its own pass only after every type's phase
//! 2/3 has finished, rather than interleaved per type as before: a
//! redefinition's owner can sort after the type currently being processed
//! in `type_keys`' ascending order, and its own member/fact counts must
//! already exist regardless.
//!
//! The redefine facts phase 4 itself derives (`RULE_REDEFINE`, one on the
//! redefining feature and one on the redefined feature per redefining
//! member reaching a type — `model-complete.md:231`) are themselves
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
//! shape), or a redefinition target that is not a member inherited by its
//! owning type (`invalid_model_binding`/`redefinition-target`, QSL #184 —
//! the target key resolves to a real declaration elsewhere in the domain
//! package, just not one of that owner's own effective members, so this is
//! not a dangling reference). `value-accounting.md:481` states "checking is
//! exhaustive within a stage," so phase 4's own charges — every
//! `normalize.redefinition-check`, every phase-4 `normalize.fact`, every
//! `normalize.conflict-check` — must all be admitted before the refusals
//! they expose are reported. `build` collects every such refusal, of any of
//! these shapes, tagged with its own [`Phase4Rank`]
//! (`record_phase4_refusal`'s own doc, next to [`Phase4Accounting`]), and
//! keeps resolving every remaining type and target group regardless, so
//! every later phase-4 charge amount is still computed correctly; once every
//! call has returned, `build` sorts the whole collection into charge order
//! and stores it in `Built::phase4_refusals`. `charge_all` charges through
//! the last `normalize.conflict-check` and returns every one of these
//! refusals, together, in that charge order, only once every phase-4 charge
//! has been admitted (`:505-511`'s "every refusal that work exposes is
//! reported, in charge order, when its stage ends"; QSL #195). An earlier
//! `Incomplete` still wins, matching `:482`'s "a stage that reports a
//! refusal ends checking: no later stage runs or charges" — phase 5's own
//! `normalize.declaration`/`normalize.hash` charges never run once these
//! refusals are pending.
#![allow(
    clippy::large_enum_variant,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline"
)]
#![allow(
    clippy::result_large_err,
    reason = "cold refusal path; ModelRefusalCause carries DeclarationKeys inline, matching state::evaluation's typed-failure precedent"
)]

use std::collections::{HashMap, HashSet};

use crate::diagnostic::Code;
use crate::model::accounting::{
    Charge, ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use crate::model::conformance::generals_by_specific;
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, FieldMemberRecord,
};
use crate::model::key::{
    digest_of, jcs_bytes, DeclarationKey, EffectiveDeclarationPreimage, EffectiveId, Fact,
    RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE,
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

/// A non-empty [`ModelRefusal`] list (L1 finding, PR #228 review):
/// `value-accounting.md:505-511`'s "every refusal that work exposes is
/// reported, in charge order, when its stage ends" means a real defect
/// always exposes at least one refusal -- checked by the compiler at
/// construction (a `first` refusal plus the `rest`, not a bare
/// `Vec<ModelRefusal>` a caller could pass empty), not merely documented as
/// the previous `Vec<ModelRefusal>` shape left it. Supports every read a
/// charge-order assertion needs -- `Index<usize>`, `iter`/`IntoIterator`
/// (both by value and by reference), and `len` -- and compares equal to a
/// plain `Vec<ModelRefusal>` of the same refusals in the same order (in
/// either direction), so existing charge-order assertions read and compare
/// it exactly as before. Not `Deref<Target = [ModelRefusal]>`: `first` and
/// `rest` are two separate fields, not one contiguous allocation, so there
/// is no real `&[ModelRefusal]` to hand back without first materializing an
/// owned copy.
#[derive(Clone, Eq, PartialEq)]
pub struct Refusals {
    first: ModelRefusal,
    rest: Vec<ModelRefusal>,
}

impl Refusals {
    /// Splits a non-empty `Vec` into this type's own `first`/`rest` shape --
    /// the shape every crate-internal producer already builds. Panics on an
    /// empty `Vec`: every real producer already has at least one refusal in
    /// hand (this type's own doc).
    pub fn from_vec(refusals: Vec<ModelRefusal>) -> Self {
        let mut iter = refusals.into_iter();
        let first = iter
            .next()
            .expect("Refusals::from_vec called with an empty Vec");
        Self {
            first,
            rest: iter.collect(),
        }
    }

    /// The number of refusals (always at least one).
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    /// Every refusal, in original order.
    pub fn iter(&self) -> impl Iterator<Item = &ModelRefusal> {
        std::iter::once(&self.first).chain(self.rest.iter())
    }
}

impl std::fmt::Debug for Refusals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

impl PartialEq<Vec<ModelRefusal>> for Refusals {
    fn eq(&self, other: &Vec<ModelRefusal>) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl PartialEq<Refusals> for Vec<ModelRefusal> {
    fn eq(&self, other: &Refusals) -> bool {
        other == self
    }
}

impl IntoIterator for Refusals {
    type Item = ModelRefusal;
    type IntoIter =
        std::iter::Chain<std::iter::Once<ModelRefusal>, std::vec::IntoIter<ModelRefusal>>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.first).chain(self.rest)
    }
}

impl<'a> IntoIterator for &'a Refusals {
    type Item = &'a ModelRefusal;
    type IntoIter =
        std::iter::Chain<std::iter::Once<&'a ModelRefusal>, std::slice::Iter<'a, ModelRefusal>>;
    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(&self.first).chain(self.rest.iter())
    }
}

impl std::ops::Index<usize> for Refusals {
    type Output = ModelRefusal;
    fn index(&self, index: usize) -> &ModelRefusal {
        if index == 0 {
            &self.first
        } else {
            &self.rest[index - 1]
        }
    }
}

/// The outcome of one normalization attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizeOutcome {
    /// Normalization completed within the given limits.
    Completed(EffectiveView),
    /// A real defect refused normalization outright; no effective view.
    /// `value-accounting.md:505-511`: "every refusal that work exposes is
    /// reported, in charge order, when its stage ends" -- every refusal a
    /// stage exposes is reported together, never only the first
    /// ([`Refusals`], L1 finding, PR #228 review).
    Refused(Refusals),
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
/// [`DomainPackageRef`] admits, sorted ascending by effective identity.
///
/// Both fields are private; [`normalize`] is this type's only constructor, so
/// a caller cannot hand-build or alter a view (its `declarations` cannot be
/// substituted for some other domain package's while keeping a matching
/// `model_selection` header) and admission never needs to re-normalize a
/// caller-supplied view to trust it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveView {
    /// The model selection this view was normalized under.
    model_selection: DomainPackageRef,
    /// Every admitted declaration, ascending by [`EffectiveId`].
    declarations: Vec<ViewEntry>,
}

impl EffectiveView {
    /// The model selection this view was normalized under.
    pub fn model_selection(&self) -> &DomainPackageRef {
        &self.model_selection
    }

    /// Every admitted declaration, ascending by [`EffectiveId`].
    pub fn declarations(&self) -> &[ViewEntry] {
        &self.declarations
    }

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
    pub model_selection: DomainPackageRef,
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
/// ancestor-path walk over a [`DomainPackage`]) from [`crate::value::MAX_CHECKING_DEPTH`]
/// (Complete-V1 expression-checking recursion, `src/value/expression/check.rs`).
/// Both happen to be 128 because both were
/// chosen as "a safe bound well inside the host stack" for their own
/// recursion, not because one normatively constrains the other; nothing in
/// FR-150 or FR-143 ties a model's generalization depth to an expression's
/// checking depth. They are independent constants that coincide in value,
/// not one limit duplicated.
pub const MAX_GENERALIZATION_DEPTH: usize = 128;

struct Index {
    /// Every declared object type, keyed by its own [`DeclarationKey`]
    /// (`package`/`node`). Under #131's flat key shape, two `ObjectType`
    /// records that share one key are no longer distinct declarations: this
    /// set's own `.insert()` would silently keep only the first, but
    /// `build`'s caller never trusts that -- `validate_references`'s
    /// collision check re-walks `domain_package.records` right after
    /// `Index::build` returns and refuses
    /// `invalid_model_binding`/`conflicting-binding` before this `Index` is
    /// put to any real use, so a caller that observes a completed
    /// normalization never sees a collapsed pair here. Also serves phase 4's
    /// and this module's other passes' "is this a declared type"
    /// dangling-reference checks — a second, redundant `known_types` set
    /// would only ever duplicate this one.
    types: std::collections::BTreeSet<DeclarationKey>,
    fields_by_owner: HashMap<DeclarationKey, Vec<FieldMemberRecord>>,
    generals_by_specific: HashMap<DeclarationKey, Vec<DeclarationKey>>,
    /// Every type whose own `supertypes[]` (`model-complete.md`:155) is
    /// non-empty, i.e. not a root.
    non_root: std::collections::HashSet<DeclarationKey>,
    /// Every declared scalar type's own full key.
    known_scalars: std::collections::HashSet<DeclarationKey>,
    /// Every field member's own key.
    field_member_keys: std::collections::HashSet<DeclarationKey>,
    /// Every operation member's own key.
    operation_member_keys: std::collections::HashSet<DeclarationKey>,
}

impl Index {
    fn build(domain_package: &DomainPackage) -> Self {
        let mut types = std::collections::BTreeSet::new();
        let mut fields_by_owner: HashMap<DeclarationKey, Vec<_>> = HashMap::new();
        // Built once by the one shared function every bounded proper-descendant
        // walk in `crate::model` uses (`crate::model::conformance`'s own doc),
        // rather than a second, independent accumulation of the identical
        // `specific -> generalization records` map.
        let generals_by_specific = generals_by_specific(domain_package);
        let mut non_root = std::collections::HashSet::new();
        let mut known_scalars = std::collections::HashSet::new();
        let mut field_member_keys = std::collections::HashSet::new();
        let mut operation_member_keys = std::collections::HashSet::new();
        for record in &domain_package.records {
            match record {
                DomainPackageRecord::ObjectType(t) => {
                    types.insert(t.key.clone());
                    // A type is `non_root` when its own `supertypes[]`
                    // (`model-complete.md`:155) is non-empty, i.e. it is the
                    // `specific` end of at least one generalization edge.
                    if !t.supertypes.is_empty() {
                        non_root.insert(t.key.clone());
                    }
                }
                DomainPackageRecord::FieldMember(m) => {
                    field_member_keys.insert(m.key.clone());
                    fields_by_owner
                        .entry(m.owner.clone())
                        .or_default()
                        .push(m.clone());
                }
                DomainPackageRecord::ScalarType(s) => {
                    known_scalars.insert(s.key.clone());
                }
                DomainPackageRecord::OperationMember(o) => {
                    operation_member_keys.insert(o.key.clone());
                }
                // FR-152 systems-model records (crate::model::systems) and
                // FR-153 population declarations (crate::model::population)
                // are not FR-150 normalization inputs: they neither declare
                // a type nor derive an effective declaration here.
                DomainPackageRecord::Component(_)
                | DomainPackageRecord::Endpoint(_)
                | DomainPackageRecord::Relationship(_)
                | DomainPackageRecord::Population(_) => {}
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

    fn sorted_type_keys(&self) -> Vec<DeclarationKey> {
        self.types.iter().cloned().collect()
    }

    fn sorted_direct_members(&self, owner: &DeclarationKey) -> Vec<FieldMemberRecord> {
        let mut members = self.fields_by_owner.get(owner).cloned().unwrap_or_default();
        members.sort_by(|a, b| a.key.cmp(&b.key));
        members
    }

    /// `specific`'s own declared `supertypes[]`, ascending by the general
    /// type's own key (there is no separate generalization-record key to
    /// sort by under QSpec's inline shape).
    fn sorted_generals(&self, specific: &DeclarationKey) -> Vec<DeclarationKey> {
        let mut generals = self
            .generals_by_specific
            .get(specific)
            .cloned()
            .unwrap_or_default();
        generals.sort();
        generals
    }
}

/// One path from a type to a strict ancestor: the ordered chain of
/// supertype-record keys taken, and the ancestor's own original
/// producer key (PR #140 F2: the full key, not a display identity string).
struct AncestorPath {
    path: Vec<DeclarationKey>,
    ancestor_key: DeclarationKey,
}

/// A conservative upper bound on how many additional phase-2/3 facts this
/// build could ever admit before [`charge_all`] denies a `normalize.fact` or
/// `normalize.cycle-check` charge, derived from `limits` and the facts
/// already produced (PR #140 F1). Passed into [`ancestor_paths`] so an
/// adversarial diamond-generalization domain-package's path count is bounded by the
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

/// A conservative upper bound on how many additional closing-cycle
/// extensions this build could still admit before [`charge_all`] denies a
/// `normalize.cycle-check` charge (H2 finding, PR #228 review). Unlike
/// [`remaining_fact_budget`], this is `work_units` room only, never
/// intersected with `derivation_facts` room: a closing extension charges no
/// `normalize.fact` (`value-accounting.md:513`), so it never consumes
/// `derivation_facts` at all, only `work_units` (its own
/// `normalize.cycle-check` charge). `facts_so_far` is reused here as the
/// same conservative (undercounting, so always-overestimating) proxy for
/// `work_units` already consumed that [`remaining_fact_budget`]'s own
/// `work_room` term uses.
fn remaining_cycle_budget(limits: &ModelNormalizationLimits, facts_so_far: u64) -> usize {
    let work_room = limits.work_units.saturating_sub(facts_so_far);
    usize::try_from(work_room.saturating_add(1)).unwrap_or(usize::MAX)
}

/// Whether at least `limits.derivation_facts` or `limits.work_units` facts
/// have already been produced, i.e. continuing to generate more can no
/// longer change the outcome: [`charge_all`]'s real replay will already
/// deny at or before this point. See [`remaining_fact_budget`]'s doc for why
/// this never fires early on a run that would otherwise complete.
fn fact_budget_exceeded(limits: &ModelNormalizationLimits, facts_so_far: u64) -> bool {
    facts_so_far > limits.derivation_facts || facts_so_far > limits.work_units
}

/// One generalization cycle a `root_key`'s own ancestor walk closes: the
/// closing extension's own [`ModelRefusal`] (`SpecializationCycle`, keyed by
/// its rotated edge listing) and the path length charged to it
/// (`normalize.cycle-check`'s own `work_units += L`,
/// `value-accounting.md:491`). `edge_set` is the same rotated node listing
/// the refusal's own `detail` reports, kept structured here so refusals
/// found by different types' own walks that close the identical cycle can
/// be recognized as the same cycle and deduplicated
/// (`model-complete.md`:292-295: "each cycle is refused ... exactly once,
/// keyed by the set of its supertype edges").
struct ClosingCycle {
    refusal: ModelRefusal,
    /// The extended path that closes this cycle — the same shape as
    /// [`AncestorPath::path`], reused as this closing extension's own
    /// [`PendingFact::inputs`] (`closes_cycle: true`) so it sorts into
    /// `normalize.cycle-check`/`normalize.fact` charge order
    /// (`value-accounting.md:490`) exactly like an ordinary ancestor-path
    /// fact from the same type, and its length is `normalize.cycle-check`'s
    /// own `work_units += L` (`:491`).
    path: Vec<DeclarationKey>,
    edge_set: Vec<DeclarationKey>,
}

/// One [`ClosingCycle`] collected build-wide across `build`'s own per-type
/// loop, tagged with the type whose walk found it (L3 finding, PR #228
/// review: previously an anonymous 4-tuple). Sorted by `(type_key, path)` --
/// the same charge-order key its own [`PendingFact`] sorts by
/// (`sort_facts`) -- then deduplicated by `edge_set` only after every type
/// has been walked (`model-complete.md`:292-295's "each cycle is refused ...
/// exactly once"; see the module docs).
struct CycleCandidate {
    type_key: DeclarationKey,
    path: Vec<DeclarationKey>,
    refusal: ModelRefusal,
    edge_set: Vec<DeclarationKey>,
}

/// [`ancestor_paths`]'s own result: every ordinary ancestor path (`paths`,
/// unchanged meaning) plus every generalization cycle this same walk closed
/// (`closing_cycles`) — one entry per closing extension, not deduplicated
/// here (`build` deduplicates by `edge_set` across every type's own walk;
/// see the module docs). The walk never stops at its first closing
/// extension: `value-accounting.md:491`'s "a closing extension is charged
/// even when its cycle's edge set was already reported" requires every
/// closing extension's own `normalize.cycle-check` charge to still occur,
/// which needs every remaining branch, and every remaining type in `build`'s
/// own per-type loop, still walked rather than abandoned at the first cycle
/// found.
struct AncestorWalk {
    paths: Vec<AncestorPath>,
    closing_cycles: Vec<ClosingCycle>,
}

/// Every ancestor path of `root_key`, in DFS pre-order over ascending-key
/// direct generalizations, capped at two independent entry counts:
/// `fact_budget` for `paths` and `cycle_budget` for `closing_cycles` (H2
/// finding, PR #228 review). Separate caps, not one shared cap counted
/// against both: a closing extension charges no `normalize.fact`
/// (`value-accounting.md:513`), so it never consumes `derivation_facts`
/// room, only `work_units` — capping it against a budget that also
/// subtracts `derivation_facts` (as `paths`' own cap correctly does)
/// undercounts its true remaining room whenever `derivation_facts` is the
/// tighter limit, truncating the walk before `charge_all`'s own replay
/// would and returning `Refused` where the spec gives `Incomplete`. The
/// walk keeps exploring siblings (not new depth) once `paths` reaches its
/// own cap, so closing extensions past that point are still discovered up
/// to `cycle_budget`. Explicit stack, not native recursion: domain package
/// data is caller-supplied and may describe a cycle.
fn ancestor_paths(
    root_key: &DeclarationKey,
    index: &Index,
    fact_budget: usize,
    cycle_budget: usize,
) -> Result<AncestorWalk, ModelRefusal> {
    struct Frame {
        directs: Vec<DeclarationKey>,
        next: usize,
        path: Vec<DeclarationKey>,
        visited: Vec<DeclarationKey>,
    }

    let mut stack = vec![Frame {
        directs: index.sorted_generals(root_key),
        next: 0,
        path: Vec::new(),
        visited: vec![root_key.clone()],
    }];
    let mut out = Vec::new();
    let mut closing_cycles: Vec<ClosingCycle> = Vec::new();
    loop {
        if out.len() >= fact_budget && closing_cycles.len() >= cycle_budget {
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
                    root_key.node
                ),
            });
        }
        // The type whose own `supertypes[]` this frame walks: a refusal
        // cites this owning node's own key.
        let specific_key = frame
            .visited
            .last()
            .cloned()
            .expect("every Frame is seeded with root_key and only ever grows visited");
        let ancestor_key = frame.directs[frame.next].clone();
        frame.next += 1;
        let mut new_path = frame.path.clone();
        new_path.push(ancestor_key.clone());
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
            // TC-196 R01: a closing extension is charged its own
            // `normalize.cycle-check` even when its cycle's edge set was
            // already reported, and enumeration continues past it -- to
            // this frame's remaining siblings, to every other frame still on
            // `stack`, and (in `build`'s own per-type loop) to every
            // remaining type -- so every closing extension anywhere in the
            // domain package is charged, not only the first one this walk
            // finds.
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
            let listing: Vec<&str> = chain.iter().map(|key| key.node.as_str()).collect();
            let refusal = ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::SpecializationCycle {
                    ancestor: ancestor_key.clone(),
                    via: specific_key.clone(),
                },
                detail: format!(
                    "{} generalizes back to itself via {}, through the cycle [{}]",
                    ancestor_key.node,
                    specific_key.node,
                    listing.join(", ")
                ),
            };
            if closing_cycles.len() < cycle_budget {
                closing_cycles.push(ClosingCycle {
                    refusal,
                    path: new_path,
                    edge_set: chain,
                });
            }
            continue;
        }
        if out.len() >= fact_budget {
            // `paths`' own cap is reached: this candidate is dropped (it
            // would exceed `derivation_facts`/`work_units` room and
            // `charge_all`'s real replay will refuse it anyway), and no new
            // frame is pushed for it -- the walk does not grow new depth
            // past this point, but `frame.next` already advanced above, so
            // it still visits this frame's remaining siblings (and every
            // other frame already on `stack`), which may still close cycles
            // within `cycle_budget`.
            continue;
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
    Ok(AncestorWalk {
        paths: out,
        closing_cycles,
    })
}

/// One fact awaiting replayed accounting, tagged with what it charges.
#[derive(Clone)]
struct PendingFact {
    owner_key: Option<DeclarationKey>,
    declared_key: DeclarationKey,
    inputs: Vec<DeclarationKey>,
    /// `Some(path_len)` for a phase-3 type-level fact (charges
    /// `normalize.cycle-check` first); `None` otherwise.
    cycle_check_len: Option<usize>,
    /// Whether this entry is a closing extension (`value-accounting.md:491`:
    /// "a closing extension is charged even when its cycle's edge set was
    /// already reported"; `model-complete.md`:513: "whose `normalize.fact`
    /// is not charged"): its own `normalize.cycle-check` still charges, but
    /// it derives no fact, so `charge_all` skips its `normalize.fact`
    /// charge and it is never counted toward `derivation_facts`. `false`
    /// for every ordinary phase 2/3 fact.
    closes_cycle: bool,
}

/// One declaration awaiting replayed `normalize.declaration`/`normalize.hash`,
/// already in final charge order (see `build`'s declaration assembly).
struct PendingDeclaration {
    jcs_len: u64,
}

struct Built {
    /// Every `model-complete.md`:81 refusal FR-154's per-node checks expose,
    /// in node order, then in FR-154's table order within one node
    /// (`check_node`). `model-complete.md`:82 describes a reference naming
    /// a refused node as not reported again as its own
    /// `missing_declaration`/`missing-name`, but no code here performs that
    /// cross-node dedup (H1 finding, PR #228 review): `check_node` checks
    /// each node against `index`/`seen_keys` independently, with no view of
    /// which other nodes already refused, so a reference naming a refused
    /// node is reported exactly like any other dangling reference. Non-empty
    /// only when intake itself
    /// refuses: `charge_all` reports these, once every `normalize.record`
    /// charge is admitted, and phase 2 onward never runs
    /// (`model-complete.md`:80: "Intake reports every such refusal in node
    /// order, and then no later phase runs"). Every other field below is a
    /// placeholder (empty/default) when this is non-empty — phase 2 onward
    /// never ran to populate them.
    intake_refusals: Vec<ModelRefusal>,
    phase2_facts: Vec<PendingFact>,
    phase3_facts: Vec<PendingFact>,
    /// Every distinct generalization cycle phase 3 exposes, ascending by
    /// the closing extension's own charge-order position (owning type, then
    /// path), deduplicated by edge set
    /// (`model-complete.md`:292-295: "each cycle is refused ... exactly
    /// once, keyed by the set of its supertype edges"). Non-empty only when
    /// phase 3 refuses; `charge_all` reports these once every phase 2/3
    /// charge (`normalize.record`, `normalize.fact`, `normalize.cycle-check`)
    /// is admitted, before phase 4 ever charges
    /// (`value-accounting.md:509`: "a stage that reports a refusal ends
    /// checking: no later stage runs or charges").
    phase3_refusals: Vec<ModelRefusal>,
    declarations: Vec<PendingDeclaration>,
    view: EffectiveView,
    universe: ObjectUniverse,
    /// Every phase-4 `normalize.redefinition-check` charge's own exact
    /// `work_units` amount (`m + r`, `value-accounting.md:455`), one entry
    /// per redefining member in the entire domain package (field and operation
    /// alike), ascending by the member's own key — computed once,
    /// domain-package-wide, in `build()` itself, never once per (member, effective
    /// type reaching it) pair; replayed by
    /// `charge_all`.
    redefinition_check_work: Vec<u64>,
    /// The count of phase-4 redefine facts (`RULE_REDEFINE`) awaiting their
    /// own `normalize.fact` charge: `quire.model.normalize.redefine/v1`
    /// derives "one fact on (T, redefining feature) and one on (T, redefined
    /// feature)" per redefining member reaching `T`
    /// (`model-complete.md:231`), for every member reaching `T`, contested
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
    /// Every phase-4 refusal — `derivation-conflict` or `redefinition-target`
    /// — `apply_redefinitions` exposes across every type and target group,
    /// ascending by charge order (`record_phase4_refusal`'s own doc), held
    /// here rather than returned by `build` (see the module docs):
    /// `apply_redefinitions` keeps resolving every remaining type and target
    /// group regardless of what it has already found, so every later
    /// phase-4 charge amount above is still computed correctly, and
    /// `charge_all` reports every refusal here, together, only once every
    /// phase-4 charge is admitted (`value-accounting.md:505-511`: "every
    /// refusal that work exposes is reported, in charge order, when its
    /// stage ends").
    phase4_refusals: Vec<ModelRefusal>,
}

/// Refuses a [`DomainPackageRecord`] that names a type key absent from the domain package's
/// own `ObjectType` records, so a dangling `owner`, `specific` or `general`
/// reference is a typed refusal rather than a panic or a silently dropped
/// record — `domain_package` is caller-supplied, not validated on the way in
/// except for this function's own empty-component and colliding-key checks
/// above, which run first. Membership is checked by the record's whole
/// [`DeclarationKey`] (`package`/`node`, #131): a reference matching some
/// declared type's `node` but naming a different `package` is exactly as
/// dangling as one matching nothing.
fn validate_selection(domain_package: &DomainPackage) -> Result<(), ModelRefusal> {
    // FR-321: "Missing required properties, duplicate keys, out-of-domain
    // values and non-canonical encodings refuse before consumption." An
    // empty `identity`/`version` string is schema `minLength`-invalid (out
    // of domain) under FR-321's own `ModelSelectionArtifact` shape.
    // `value-accounting.md:507-508`: "a refusal decided by an intake
    // admission check before a stage's first charge is reported first" --
    // this selection check precedes every `normalize.record` charge
    // (`model-complete.md`:72: intake reads each IR node only "then",
    // after admitting the package and its selection), so it refuses
    // immediately, with no charge at all, unlike the per-node checks
    // `validate_references` runs once node reading has begun.
    if domain_package.model_selection.identity.is_empty()
        || domain_package.model_selection.version.is_empty()
    {
        return Err(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::MalformedDeclaration,
            detail: format!(
                "domain package selection has an empty identity or version: {:?}",
                domain_package.model_selection
            ),
        });
    }
    Ok(())
}

/// Every FR-154 per-node check for one already-key-validated node, in
/// FR-154's own table order, the object id check first
/// (`model-complete.md`:81). Returns every failing check for this node, in
/// that table order -- not only the first -- `model-complete.md`:80-82:
/// "Intake reports every such refusal in node order... Within one node,
/// every failing check reports in FR-154's table order." (H1 finding, PR
/// #228 review; the previous single-`Option` shape stopped at this node's
/// own first failing check, silently dropping every later one). The
/// `seen_keys` collision check always runs, after every other check for
/// this node, regardless of whether an earlier check on this same node
/// already failed -- a node's own dangling references do not exempt it from
/// also being reported as a duplicate key.
fn check_node<'a>(
    record: &'a DomainPackageRecord,
    index: &Index,
    seen_keys: &mut std::collections::HashSet<&'a DeclarationKey>,
) -> Vec<ModelRefusal> {
    let key = record.key();
    // `model-complete.md`:81: within a node, the malformed-declaration
    // check is that node's own first check, ahead of any
    // dangling-reference or collision check for that same node -- an
    // empty `package`/`node` is schema `minLength`-invalid (FR-321,
    // out of domain) so it refuses before this node's own key is ever
    // looked up in `index` or compared against `seen_keys`, and no other
    // check for this node can run: every later check assumes a
    // well-formed key to look up or insert.
    if key.package.is_empty() || key.node.is_empty() {
        return vec![ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::MalformedDeclaration,
            detail: format!("declaration key has an empty package or node: {key:?}"),
        }];
    }
    let mut refusals = Vec::new();
    match record {
        DomainPackageRecord::ObjectType(t) => {
            // `specific` is always this same record's own `t.key`,
            // already inserted into `index.types` from this identical
            // record under QSpec's inline `supertypes[]` shape
            // (`model-complete.md`:155) -- so an "unknown specific" is
            // structurally unreachable and there is no matching
            // `ModelRefusalCause` variant for it.
            for general in &t.supertypes {
                if !index.types.contains(general) {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownGeneral {
                            supertype: t.key.clone(),
                            general: general.clone(),
                        },
                        detail: format!(
                            "object type {} names supertype {}, which is not a declared object type",
                            t.key.node, general.node
                        ),
                    });
                }
            }
        }
        DomainPackageRecord::FieldMember(member) => {
            if !index.types.contains(&member.owner) {
                refusals.push(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownOwner {
                        member: member.key.clone(),
                        owner: member.owner.clone(),
                    },
                    detail: format!(
                        "field member {} names owner {}, which is not a declared object type",
                        member.key.node, member.owner.node
                    ),
                });
            }
            if let Some(redefines) = &member.redefines {
                if !index.field_member_keys.contains(redefines)
                    && !index.operation_member_keys.contains(redefines)
                {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownMember {
                            record: member.key.clone(),
                            member: redefines.clone(),
                        },
                        detail: format!(
                            "field member {} redefines {}, which is not a declared field or operation member",
                            member.key.node, redefines.node
                        ),
                    });
                }
            }
            for subsetted in &member.subsets {
                if !index.field_member_keys.contains(subsetted)
                    && !index.operation_member_keys.contains(subsetted)
                {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownMember {
                            record: member.key.clone(),
                            member: subsetted.clone(),
                        },
                        detail: format!(
                            "field member {} subsets {}, which is not a declared field or operation member",
                            member.key.node, subsetted.node
                        ),
                    });
                }
            }
        }
        DomainPackageRecord::ScalarType(_) => {}
        DomainPackageRecord::OperationMember(op) => {
            if !index.types.contains(&op.owner) {
                refusals.push(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownOwner {
                        member: op.key.clone(),
                        owner: op.owner.clone(),
                    },
                    detail: format!(
                        "operation member {} names owner {}, which is not a declared object type",
                        op.key.node, op.owner.node
                    ),
                });
            }
            for parameter in &op.parameters {
                if !index.types.contains(&parameter.value_type)
                    && !index.known_scalars.contains(&parameter.value_type)
                {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownValueType {
                            operation: op.key.clone(),
                            parameter: Some(parameter.key.clone()),
                            value_type: parameter.value_type.clone(),
                        },
                        detail: format!(
                            "operation {} parameter {} names value type {}, which is not a declared type",
                            op.key.node,
                            parameter.key.node,
                            parameter.value_type.node
                        ),
                    });
                }
            }
            if let Some(result) = &op.result {
                if !index.types.contains(&result.value_type)
                    && !index.known_scalars.contains(&result.value_type)
                {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownValueType {
                            operation: op.key.clone(),
                            parameter: None,
                            value_type: result.value_type.clone(),
                        },
                        detail: format!(
                            "operation {} result names value type {}, which is not a declared type",
                            op.key.node, result.value_type.node
                        ),
                    });
                }
            }
            for field in &op.effect.modifies {
                if !index.field_member_keys.contains(field) {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownFieldWrite {
                            operation: op.key.clone(),
                            field: field.clone(),
                        },
                        detail: format!(
                            "operation {} effect writes {}, which is not a declared field member",
                            op.key.node, field.node
                        ),
                    });
                }
            }
            for target in op.effect.creates.iter().chain(&op.effect.deletes) {
                if !index.types.contains(target) {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownEffectType {
                            operation: op.key.clone(),
                            type_name: target.clone(),
                        },
                        detail: format!(
                            "operation {} effect names type {}, which is not a declared object type",
                            op.key.node, target.node
                        ),
                    });
                }
            }
            if let Some(redefines) = &op.redefines {
                if !index.field_member_keys.contains(redefines)
                    && !index.operation_member_keys.contains(redefines)
                {
                    refusals.push(ModelRefusal {
                        code: Code::DanglingReference,
                        cause: ModelRefusalCause::UnknownMember {
                            record: op.key.clone(),
                            member: redefines.clone(),
                        },
                        detail: format!(
                            "operation member {} redefines {}, which is not a declared field or operation member",
                            op.key.node, redefines.node
                        ),
                    });
                }
            }
        }
        // FR-152 systems-model records validate their own references
        // independently (crate::model::systems); FR-150's phase 1 does
        // not concern itself with them.
        DomainPackageRecord::Component(_)
        | DomainPackageRecord::Endpoint(_)
        | DomainPackageRecord::Relationship(_) => {}
        // model-complete.md's "Populations" row: each member type names
        // a declared object type; a missing one refuses
        // `missing_declaration`/`missing-name`.
        DomainPackageRecord::Population(population) => {
            for type_name in &population.member_types {
                if !index.types.contains(type_name) {
                    refusals.push(ModelRefusal {
                        code: Code::MissingDeclaration,
                        cause: ModelRefusalCause::UnknownPopulationMemberType {
                            population: population.key.clone(),
                            type_name: type_name.clone(),
                        },
                        detail: format!(
                            "population {} names member type {}, which is not a declared object type",
                            population.key.node, type_name.node
                        ),
                    });
                }
            }
        }
    }
    // Always runs, even when an earlier check above already pushed a
    // refusal for this node (H1 finding, PR #228 review): a node whose own
    // reference checks already fail is not exempted from also being a
    // duplicate key, and skipping this insert on an earlier failure left a
    // later node with the same key believing it saw that key for the first
    // time, silently losing `conflicting-binding` for the later node.
    if !seen_keys.insert(key) {
        refusals.push(ModelRefusal {
            code: Code::InvalidModelBinding,
            cause: ModelRefusalCause::ConflictingBinding { key: key.clone() },
            detail: format!(
                "{} is declared by more than one record in this domain package",
                key.node
            ),
        });
    }
    refusals
}

/// Every FR-154 refusal the domain package's own nodes expose, in node
/// order (`model-complete.md`:73: "Nodes are read ascending by declaration
/// key"), then in FR-154's table order within that node (`check_node`'s own
/// doc: every failing check for that node, not only the first). Never stops
/// at the first failing node -- TC-195 N08's own two refusals, reported
/// together in node order, are exactly this shape
/// (`model-complete.md`:80). `domain_package` is caller-supplied, not
/// validated on the way in except for this function's own empty-component
/// and colliding-key checks, which run first per node. Membership is
/// checked by the record's whole [`DeclarationKey`] (`package`/`node`,
/// #131): a reference matching some declared type's `node` but naming a
/// different `package` is exactly as dangling as one matching nothing.
fn validate_references(domain_package: &DomainPackage, index: &Index) -> Vec<ModelRefusal> {
    // `model-complete.md`:73: "Nodes are read ascending by declaration key."
    // Every following check -- the per-node malformed-key check, the
    // dangling-reference checks, and the collision check -- runs over this
    // sorted order, not `domain_package.records`' own input order, so two
    // domain packages that differ only in record order refuse identically.
    let mut records: Vec<&DomainPackageRecord> = domain_package.records.iter().collect();
    records.sort_by(|a, b| a.key().cmp(b.key()));
    // FR-154's own row order: node (declaration) order first, then table
    // order within one node -- `missing-name` (row 4) before
    // `conflicting-binding` (row 6). Folded into one pass over the sorted
    // `records`, not several: a standalone collision pre-pass over every
    // record before any dangling-reference check ran would report a *later*
    // node's duplicate key ahead of an *earlier* node's own dangling
    // reference, applying FR-154's table order globally instead of within
    // each node -- the wrong axis. `DeclarationKey` is `package`/`node`
    // only, so two records sharing one key collide for real and must
    // refuse here, not silently let the later record replace or shadow the
    // earlier one in `Index`'s by-key maps/sets.
    let mut seen_keys: std::collections::HashSet<&DeclarationKey> =
        std::collections::HashSet::new();
    let mut refusals = Vec::new();
    for record in records {
        refusals.extend(check_node(record, index, &mut seen_keys));
    }
    refusals
}

/// Pass one: build the complete normalization, refusing outright on a real
/// defect and bounding ancestor-path enumeration against `limits` (PR #140
/// F1) so a diamond-generalization domain-package cannot force unbounded work before
/// [`charge_all`] gets to deny anything.
fn build(
    domain_package: &DomainPackage,
    limits: &ModelNormalizationLimits,
) -> Result<Built, ModelRefusal> {
    validate_selection(domain_package)?;
    let index = Index::build(domain_package);
    // QSL #199: every `normalize.record` charge (one per IR node, in node
    // order) runs before any intake refusal is reported -- `validate_references`
    // itself never charges anything; it is `charge_all` that charges the
    // whole `domain_package.records` sequence and only then consults this
    // result (see `Built::intake_refusals`'s own doc).
    let intake_refusals = validate_references(domain_package, &index);
    if !intake_refusals.is_empty() {
        // `model-complete.md`:80: "Intake reports every such refusal in
        // node order, and then no later phase runs" -- phase 2 onward never
        // ran, so every other field below is its own placeholder.
        return Ok(Built {
            intake_refusals,
            phase2_facts: Vec::new(),
            phase3_facts: Vec::new(),
            phase3_refusals: Vec::new(),
            declarations: Vec::new(),
            view: EffectiveView {
                model_selection: domain_package.model_selection.clone(),
                declarations: Vec::new(),
            },
            universe: ObjectUniverse {
                model_selection: domain_package.model_selection.clone(),
                root_types: Vec::new(),
            },
            redefinition_check_work: Vec::new(),
            phase4_fact_count: 0,
            conflict_check_work: Vec::new(),
            phase4_refusals: Vec::new(),
        });
    }
    let type_keys = index.sorted_type_keys();

    let mut phase2_facts = Vec::new();
    let mut phase3_facts = Vec::new();
    let mut type_preimages: HashMap<DeclarationKey, EffectiveDeclarationPreimage> = HashMap::new();
    let mut type_effective_ids: HashMap<DeclarationKey, EffectiveId> = HashMap::new();
    let mut type_jcs_lens: HashMap<DeclarationKey, u64> = HashMap::new();
    let mut member_preimages: HashMap<
        (DeclarationKey, DeclarationKey),
        EffectiveDeclarationPreimage,
    > = HashMap::new();
    let mut hidden: std::collections::HashSet<(DeclarationKey, DeclarationKey)> =
        std::collections::HashSet::new();
    let mut facts_so_far: u64 = 0;
    // Phase 3's own ancestor paths, retained per type (rather than dropped
    // at the end of each iteration below) so phase 4 -- now its own pass,
    // run only after every type's phase 2/3 has finished -- can reuse them
    // without recomputing (PR #140 F10) for a `type_key` whose own
    // iteration already ran.
    let mut type_paths: HashMap<DeclarationKey, Vec<AncestorPath>> = HashMap::new();

    // QSL #193: every distinct generalization cycle any type's own walk
    // closes, tagged with the same `(type_key, path)` charge-order key its
    // own `PendingFact` sorts by (`sort_facts`), collected build-wide across
    // every type in the loop below and deduplicated by edge set only after
    // the loop finishes (`model-complete.md`:292-295's "each cycle is
    // refused ... exactly once"; see the module docs).
    let mut cycle_candidates: Vec<CycleCandidate> = Vec::new();

    for type_key in &type_keys {
        phase2_facts.push(PendingFact {
            owner_key: None,
            declared_key: type_key.clone(),
            inputs: vec![type_key.clone()],
            cycle_check_len: None,
            closes_cycle: false,
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
        let fact_budget = remaining_fact_budget(limits, facts_so_far);
        let cycle_budget = remaining_cycle_budget(limits, facts_so_far);
        let walk = ancestor_paths(type_key, &index, fact_budget, cycle_budget)?;
        let paths = walk.paths;
        for ancestor in &paths {
            let inputs = ancestor.path.clone();
            phase3_facts.push(PendingFact {
                owner_key: None,
                declared_key: type_key.clone(),
                inputs: inputs.clone(),
                cycle_check_len: Some(ancestor.path.len()),
                closes_cycle: false,
            });
            facts_so_far += 1;
            derivation.push(Fact {
                ordinal: derivation.len(),
                rule: RULE_INHERIT,
                inputs,
            });
        }
        // Every closing extension this type's own walk found is charged its
        // own `normalize.cycle-check` (QSL #193) but derives no fact
        // (`closes_cycle: true`); its refusal is only a *candidate* until
        // every type has been walked and duplicates across types have been
        // removed, below.
        for closing in walk.closing_cycles {
            phase3_facts.push(PendingFact {
                owner_key: None,
                declared_key: type_key.clone(),
                inputs: closing.path.clone(),
                cycle_check_len: Some(closing.path.len()),
                closes_cycle: true,
            });
            cycle_candidates.push(CycleCandidate {
                type_key: type_key.clone(),
                path: closing.path,
                refusal: closing.refusal,
                edge_set: closing.edge_set,
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
                closes_cycle: false,
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
                    closes_cycle: false,
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

    // QSL #193: deduplicate cycle refusal candidates by edge set, keeping
    // each distinct cycle's own first (charge-order-earliest) closing
    // extension (`model-complete.md`:292-295), sorted the same way
    // `sort_facts` orders their own `PendingFact` entries (type key, then
    // path) so `phase3_refusals`' own order matches the charge order
    // `charge_all` reports them in.
    cycle_candidates.sort_by(|a, b| {
        a.type_key
            .cmp(&b.type_key)
            .then_with(|| a.path.cmp(&b.path))
    });
    let mut seen_edge_sets: HashSet<Vec<DeclarationKey>> = HashSet::new();
    let mut phase3_refusals: Vec<ModelRefusal> = Vec::new();
    for candidate in cycle_candidates {
        if seen_edge_sets.insert(candidate.edge_set) {
            phase3_refusals.push(candidate.refusal);
        }
    }

    // Phase 4: every type's own field-redefinition conflicts, run only now
    // that every type's phase 2/3 has finished (see the module docs): a
    // redefinition's owner can sort after `type_key` in `type_keys`'
    // ascending order, and its own effective member count (`m`,
    // `value-accounting.md:455`) and type derivation-fact count (`f(o)`,
    // `:456`) -- both read directly out of `member_preimages`/
    // `type_preimages` below -- must already exist regardless of which
    // type's turn happens to reach it first.
    let mut member_counts_by_owner: HashMap<DeclarationKey, u64> = HashMap::new();
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
    let mut operations_by_owner: HashMap<DeclarationKey, Vec<DeclarationKey>> = HashMap::new();
    for record in &domain_package.records {
        if let DomainPackageRecord::OperationMember(operation) = record {
            operations_by_owner
                .entry(operation.owner.clone())
                .or_default()
                .push(operation.key.clone());
        }
    }
    for type_key in &type_keys {
        let mut effective_operations: HashSet<DeclarationKey> = HashSet::new();
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
    let type_fact_counts: HashMap<DeclarationKey, u64> = type_preimages
        .iter()
        .map(|(key, preimage)| (key.clone(), length_amount(preimage.derivation.len())))
        .collect();
    // Every owner's own proper-ancestor set, derived from phase 3's own
    // `type_paths` (populated above for every type in the domain package) rather
    // than a fresh, separately-bounded walk (QSL #145): `ancestor_key` is
    // exactly the proper-ancestor identity
    // `crate::model::conformance::ancestor_closure` used to compute with its
    // own `MAX_CONFORMANCE_DEPTH` breadth ceiling, so deduplicating those
    // same keys here needs no walk of its own and has no ceiling to exceed.
    // Computed once, build-wide (QSL #145), not once per (type, target)
    // group.
    let owner_ancestor_sets: HashMap<DeclarationKey, HashSet<DeclarationKey>> = type_paths
        .iter()
        .map(|(owner, paths)| {
            let ancestors: HashSet<DeclarationKey> =
                paths.iter().map(|path| path.ancestor_key.clone()).collect();
            (owner.clone(), ancestors)
        })
        .collect();

    // `normalize.redefinition-check` (`value-accounting.md:455`) charges
    // `m + r` once per redefining member in the *entire domain package* --
    // field and operation alike -- ascending by the member's own key
    // (there is no separate redefinition-record key to sort by under
    // QSpec's inline `redefines` property), `r` the count of members
    // already checked before it in this same domain-package-wide sequence.
    // Computed once here, before the per-type phase-4 loop below even
    // starts: each redefining member is the spec's own priced unit, tested
    // once against its own owning type, never once per (member, effective
    // type reaching it) pair a per-type loop would recompute it at.
    let mut all_redefining_members: Vec<(&DeclarationKey, &DeclarationKey)> = domain_package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::FieldMember(member) if member.redefines.is_some() => {
                Some((&member.key, &member.owner))
            }
            DomainPackageRecord::OperationMember(operation) if operation.redefines.is_some() => {
                Some((&operation.key, &operation.owner))
            }
            _ => None,
        })
        .collect();
    all_redefining_members.sort_by(|a, b| a.0.cmp(b.0));
    let mut redefinition_check_work: Vec<u64> = Vec::new();
    for (r, (_key, owner)) in all_redefining_members.iter().enumerate() {
        let m = member_counts_by_owner.get(*owner).copied().unwrap_or(0);
        redefinition_check_work.push(m.saturating_add(length_amount(r)));
    }

    let mut conflict_charges: Vec<(EffectiveId, DeclarationKey, u64)> = Vec::new();
    let mut phase4_fact_count: u64 = 0;
    let mut phase4_refusal_candidates: Vec<(Phase4Rank, ModelRefusal)> = Vec::new();
    let mut accounting = Phase4Accounting {
        type_fact_counts: &type_fact_counts,
        owner_ancestor_sets: &owner_ancestor_sets,
        type_effective_ids: &type_effective_ids,
        conflict_charges: &mut conflict_charges,
        phase4_fact_count: &mut phase4_fact_count,
        refusals: &mut phase4_refusal_candidates,
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
    // `conflict_charges` computed from truncated data (QSL #145). Likewise,
    // a non-empty `phase3_refusals` (QSL #193) means phase 3 itself already
    // exposed a refusal that `charge_all` reports before phase 4 ever
    // charges (`value-accounting.md:509`'s "a stage that reports a refusal
    // ends checking: no later stage runs or charges"), so phase 4's own
    // resolution is skipped here too.
    if !fact_budget_exceeded(limits, facts_so_far) && phase3_refusals.is_empty() {
        for type_key in &type_keys {
            let paths = type_paths
                .get(type_key)
                .expect("populated in the loop above");
            apply_redefinitions(
                domain_package,
                &index,
                type_key,
                paths,
                &mut member_preimages,
                &mut hidden,
                &mut accounting,
            );
        }
    }
    conflict_charges.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let conflict_check_work: Vec<u64> = conflict_charges
        .into_iter()
        .map(|(_, _, amount)| amount)
        .collect();
    phase4_refusal_candidates.sort_by(|a, b| a.0.cmp(&b.0));
    // A `normalize.redefinition-check` refusal's rank is the redefining
    // member's own key alone (`Phase4Rank::RedefinitionCheck`), matching
    // `redefinition_check_work`'s own domain-package-wide, once-per-record
    // charge above: every descendant type that independently reaches the
    // same broken redefining member (through generalization) derives the
    // identical rank and an identical refusal, so a dedup by rank here
    // keeps each redefinition-check refusal reported exactly once, the same
    // cardinality as its own charge. A `normalize.conflict-check` refusal's
    // rank also carries the resolving type's own effective identity, so two
    // different types' own conflicts never collapse into each other here.
    phase4_refusal_candidates.dedup_by(|a, b| a.0 == b.0);
    let phase4_refusals: Vec<ModelRefusal> = phase4_refusal_candidates
        .into_iter()
        .map(|(_, refusal)| refusal)
        .collect();

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
    // QSL #195: `normalize.declaration` charges "effective types, then
    // effective members, each ascending by effective member key"
    // (`value-accounting.md:494`) -- `(owner effective type identity,
    // original declaration key)` (`model-complete.md`:206), not the owner's
    // producer `DeclarationKey`. `type_effective_ids` is fully populated for
    // every type by the loop above, so this lookup never falls back.
    let mut member_keys: Vec<(DeclarationKey, DeclarationKey)> =
        member_preimages.keys().cloned().collect();
    member_keys.sort_by(|(owner_a, decl_a), (owner_b, decl_b)| {
        type_effective_ids[owner_a]
            .cmp(&type_effective_ids[owner_b])
            .then_with(|| decl_a.cmp(decl_b))
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
        model_selection: domain_package.model_selection.clone(),
        declarations: entries,
    };
    let universe = ObjectUniverse {
        model_selection: domain_package.model_selection.clone(),
        root_types,
    };

    Ok(Built {
        intake_refusals: Vec::new(),
        phase2_facts,
        phase3_facts,
        phase3_refusals,
        declarations,
        view,
        universe,
        redefinition_check_work,
        phase4_fact_count,
        conflict_check_work,
        phase4_refusals,
    })
}

/// One redefining member's contest for its `redefines` target, reachable
/// at `type_key` along `path` (the ancestor-generalization keys from
/// `type_key` to `owner`, empty when `owner` is `type_key` itself).
/// `redefining` is both this edge's identity and the node F1 requires every
/// refusal to cite -- there is no separate redefinition-record key under
/// QSpec's inline shape (`model-complete.md`:159/160/270/271).
struct RedefinitionEdge {
    owner: DeclarationKey,
    redefining: DeclarationKey,
    target: DeclarationKey,
    path: Vec<DeclarationKey>,
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
/// needs — is not built here at all: `build` computes it once,
/// domain-package-wide, before any `apply_redefinitions` call.
struct Phase4Accounting<'a> {
    type_fact_counts: &'a HashMap<DeclarationKey, u64>,
    owner_ancestor_sets: &'a HashMap<DeclarationKey, HashSet<DeclarationKey>>,
    /// Every type's own effective identity, populated by `build`'s phase
    /// 2/3 loop before any `apply_redefinitions` call — QSL #195: the
    /// effective member key (`model-complete.md`:206) orders both
    /// `conflict_charges` below and `build`'s own member-declaration
    /// sequence by an owner's *effective* identity, not its producer
    /// `DeclarationKey`.
    type_effective_ids: &'a HashMap<DeclarationKey, EffectiveId>,
    /// Every phase-4 `normalize.conflict-check` charge's own exact
    /// `work_units` amount, tagged with its charge-order key: `(owner
    /// effective type identity, target key)` — `value-accounting.md:456`'s
    /// "ascending by effective member key" — collected build-wide, across
    /// every `type_key`'s own call to this function, and sorted into final
    /// charge order once, in `build`, only after every call has returned
    /// (QSL #195; see the module docs).
    conflict_charges: &'a mut Vec<(EffectiveId, DeclarationKey, u64)>,
    /// The running count of phase-4 redefine facts awaiting their own
    /// `normalize.fact` charge (see [`Built::phase4_fact_count`]'s own
    /// doc) — field redefinition only, exactly like `member_preimages`/
    /// `hidden`: an operation-member redefinition edge never reaches
    /// [`Fact`] construction here at all (see the module docs), so it
    /// contributes nothing to this count.
    phase4_fact_count: &'a mut u64,
    /// Every phase-4 refusal this pass exposes, across every `type_key`'s
    /// own call, each tagged with its own [`Phase4Rank`] — every one, not
    /// only the earliest (`value-accounting.md:505-511`: "every refusal
    /// that work exposes is reported, in charge order, when its stage
    /// ends"; QSL #195). Sorted into final charge order once, in `build`,
    /// only after every call has returned. Set only by
    /// `record_phase4_refusal`.
    refusals: &'a mut Vec<(Phase4Rank, ModelRefusal)>,
}

/// Charge-order rank of one phase-4 refusal candidate: lower sorts earlier.
/// `normalize.redefinition-check` charges every redefining member before
/// phase 4's own `normalize.fact` charges, which in turn precede every
/// `normalize.conflict-check` charge (`value-accounting.md:455`, `:456`), so
/// every `RedefinitionCheck` variant sorts ahead of every `ConflictCheck`
/// variant (`#[derive(Ord)]` on an enum ranks earlier-declared variants
/// first). `RedefinitionCheck` carries the redefining member's own key alone
/// (`:455`: "ascending by declaration key"); `ConflictCheck` carries the
/// contested group's own effective member key — `(owner effective type
/// identity, target key)` (`:456`: "ascending by effective member key",
/// `model-complete.md`:206) — not the owning type's producer `DeclarationKey`
/// (QSL #195).
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Phase4Rank {
    RedefinitionCheck(DeclarationKey),
    ConflictCheck(EffectiveId, DeclarationKey),
}

/// Appends `(rank, refusal)` to `accounting.refusals`. Every phase-4 refusal
/// any call exposes is kept, not only whichever ranks earliest — `build`
/// sorts the whole collection by `Phase4Rank` once every `type_key` has been
/// processed, then reports every entry, in that charge order, as
/// `Built::phase4_refusals` (QSL #195; see the module docs).
fn record_phase4_refusal(
    accounting: &mut Phase4Accounting<'_>,
    rank: Phase4Rank,
    refusal: ModelRefusal,
) {
    accounting.refusals.push((rank, refusal));
}

/// Phase 4 (TC-195 N06): field redefinition only — operation-member
/// redefinition is out of scope here (see the module docs); `conformance`
/// resolves that case directly against its own [`EffectiveDeclarationPreimage`]
/// values instead of this pass's exposure bookkeeping.
///
/// For every field member's own `redefines` property reachable at `type_key` (declared on
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
/// charge sequence is `build`'s own domain-package-wide pass, not this function's.
/// `type_fact_counts` supplies `f(o)` for any owner in the domain package, not just
/// `type_key` itself — `build` computes it
/// only after every type's own phase 2/3 has run (see the module docs) so
/// this is always a lookup, never a fresh walk. `owner_ancestor_sets` is
/// `build`'s own build-wide map of every owner's proper-ancestor set,
/// derived from phase 3's own `type_paths` (QSL #145) rather than a second,
/// separately bounded walk, and likewise computed once, build-wide, not
/// once per (type, target) group.
fn apply_redefinitions(
    domain_package: &DomainPackage,
    index: &Index,
    type_key: &DeclarationKey,
    paths: &[AncestorPath],
    member_preimages: &mut HashMap<(DeclarationKey, DeclarationKey), EffectiveDeclarationPreimage>,
    hidden: &mut HashSet<(DeclarationKey, DeclarationKey)>,
    accounting: &mut Phase4Accounting<'_>,
) {
    // QSL #195: every `normalize.conflict-check` charge and refusal this
    // call exposes is tagged by `type_key`'s own *effective* identity, not
    // its producer `DeclarationKey` — `build` populates `type_effective_ids`
    // for every type before any `apply_redefinitions` call (see the module
    // docs).
    let owner_effective_id = accounting
        .type_effective_ids
        .get(type_key)
        .cloned()
        .expect("build populates type_effective_ids for every type before phase 4 runs");

    let mut owner_paths: HashMap<DeclarationKey, Vec<DeclarationKey>> = HashMap::new();
    owner_paths.insert(type_key.clone(), Vec::new());
    for ancestor in paths {
        owner_paths
            .entry(ancestor.ancestor_key.clone())
            .or_insert_with(|| ancestor.path.clone());
    }

    // Every redefining member reachable at `type_key`, gathered flat (not
    // yet grouped by target) and split by member kind, from each field's or
    // operation's own inline `redefines` property (`model-complete.md`:159/
    // 160/270/271).
    // `normalize.redefinition-check`'s own charge sequence does not come
    // from either list: it is `build`'s own domain-package-wide pass over every
    // redefining member, field and operation alike; `all_edges`
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
    for record in &domain_package.records {
        let (owner, redefining, redefines, is_field, is_operation) = match record {
            DomainPackageRecord::FieldMember(member) => match &member.redefines {
                Some(redefines) => (&member.owner, &member.key, redefines, true, false),
                None => continue,
            },
            DomainPackageRecord::OperationMember(operation) => match &operation.redefines {
                Some(redefines) => (&operation.owner, &operation.key, redefines, false, true),
                None => continue,
            },
            _ => continue,
        };
        let is_field = is_field
            && index.field_member_keys.contains(redefining)
            && index.field_member_keys.contains(redefines);
        let is_operation = is_operation
            && index.operation_member_keys.contains(redefining)
            && index.operation_member_keys.contains(redefines);
        if !is_field && !is_operation {
            continue;
        }
        let Some(path) = owner_paths.get(owner) else {
            // This redefining member's owner does not reach `type_key`.
            continue;
        };
        let edge = RedefinitionEdge {
            owner: owner.clone(),
            redefining: redefining.clone(),
            target: redefines.clone(),
            path: path.clone(),
        };
        if is_field {
            all_edges.push(edge);
        } else {
            operation_edges.push(edge);
        }
    }

    let mut groups: HashMap<DeclarationKey, Vec<RedefinitionEdge>> = HashMap::new();
    for edge in all_edges {
        groups.entry(edge.target.clone()).or_default().push(edge);
    }

    let mut target_keys: Vec<DeclarationKey> = groups.keys().cloned().collect();
    target_keys.sort();

    // `value-accounting.md:456`'s own charge order is a single ascending
    // pass "by effective member key" over every contested `(effective type,
    // redefined member)` reached by `c >= 2` records -- field and operation
    // targets interleaved by that one key, never field targets as a block
    // followed by operation targets as a block. Each loop below still
    // resolves (and, for fields, hides/derives) its own kind in its own
    // pass, but neither sorts its own charge amount locally (QSL #195):
    // both push straight to `accounting.conflict_charges`, tagged with this
    // call's own `owner_effective_id`, and `build` sorts the whole
    // build-wide collection by effective member key once, after every
    // `type_key`'s own call has returned.

    'targets: for target_key in target_keys {
        let mut edges = groups.remove(&target_key).expect("just listed");
        edges.sort_by(|a, b| {
            a.owner
                .cmp(&b.owner)
                .then_with(|| a.redefining.cmp(&b.redefining))
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
            // (128+ direct generalizations) but uncontested domain package wrongly
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
            accounting.conflict_charges.push((
                owner_effective_id.clone(),
                target_key.clone(),
                fact_total.saturating_mul(c.saturating_sub(1)),
            ));

            // `owner_ancestor_sets` already holds every owner in the domain package's
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
                // redefining member's own declaration key and the one
                // contended target.
                //
                // The `redefinition-target` shape below (`same_owner`) is
                // exposed by `normalize.redefinition-check`
                // (`value-accounting.md:492`), ranked ascending by
                // redefining-member key, and charges *before* every
                // `normalize.conflict-check` refusal — the same charge
                // point and rank shape `record_phase4_refusal` already uses
                // for the unreachable-target `redefinition-target` refusal
                // below (M1 finding, PR #228 review). The genuine
                // `derivation-conflict` shape (`!same_owner`) stays on
                // `normalize.conflict-check` (`:493`), ranked
                // `Phase4Rank::ConflictCheck(owner_effective_id, target_key)`
                // — `:493`'s "ascending by effective member key"
                // (`model-complete.md:206`: owner effective type identity,
                // then original declaration key) — using this group's own
                // resolving `type_key`'s effective identity, genuinely
                // distinct per resolving type (QSL #195; see the module
                // docs and `record_phase4_refusal`'s own doc).
                let mut most_derived: Vec<&RedefinitionEdge> = Vec::new();
                for edge in &edges {
                    let mut dominated_by_another = false;
                    for other in &edges {
                        if other.owner == edge.owner {
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
                        .all(|edge| edge.owner == most_derived[0].owner);
                // Unlike the `derivation-conflict` candidate below, whose own
                // detail names `type_key` and so is genuinely distinct per
                // resolving type, this `redefinition-target` candidate's own
                // detail names only `most_derived[0].owner` -- content that
                // does not depend on `type_key` at all. Every descendant of
                // `most_derived[0].owner` that also inherits `target_key`
                // through it independently re-derives the identical
                // candidate (the same reasoning as `redefinition_check_work`
                // /the dedup below `apply_redefinitions`'s own per-type_key
                // loop, QSL #195), so this candidate is ranked by a key
                // drawn from its own redefining members -- content-stable
                // across every resolving `type_key` -- not by any effective
                // identity tied to `type_key`.
                if same_owner {
                    let mut redefiners: Vec<DeclarationKey> = most_derived
                        .iter()
                        .map(|edge| edge.redefining.clone())
                        .collect();
                    redefiners.sort();
                    // `value-accounting.md:492`'s "ascending by
                    // redefining-member key" ranks this group by one of its
                    // own redefining members' keys; the second-checked one
                    // (ascending order) is used rather than the least, which
                    // the unreachable-target `redefinition-target` shape
                    // below already uses for its own rank (M1 finding, PR
                    // #228 review) -- keeping the two `redefinition-target`
                    // shapes distinguishable by rank when they tie on every
                    // other component.
                    let rank_key = redefiners
                        .get(1)
                        .cloned()
                        .unwrap_or_else(|| redefiners[0].clone());
                    let redefiner_names: Vec<&str> =
                        redefiners.iter().map(|key| key.node.as_str()).collect();
                    record_phase4_refusal(
                        accounting,
                        Phase4Rank::RedefinitionCheck(rank_key),
                        ModelRefusal {
                            code: Code::InvalidModelBinding,
                            cause: ModelRefusalCause::RedefinitionTarget {
                                redefiners: redefiners.clone(),
                                target: target_key.clone(),
                            },
                            detail: format!(
                                "{} declares {} redefining members ({}) that all redefine {}, with no single valid target",
                                most_derived[0].owner.node,
                                redefiners.len(),
                                redefiner_names.join(", "),
                                target_key.node
                            ),
                        },
                    );
                } else {
                    let edge_paths: Vec<String> = edges
                        .iter()
                        .map(|edge| {
                            let mut path: Vec<String> =
                                edge.path.iter().map(|key| key.node.clone()).collect();
                            path.push(edge.redefining.node.clone());
                            path.push(target_key.node.clone());
                            format!("[{}]", path.join(", "))
                        })
                        .collect();
                    record_phase4_refusal(
                        accounting,
                        Phase4Rank::ConflictCheck(owner_effective_id.clone(), target_key.clone()),
                        ModelRefusal {
                            code: Code::InvalidModelBinding,
                            cause: ModelRefusalCause::DerivationConflict {
                                type_: type_key.clone(),
                                member: target_key.clone(),
                                redefiners: edges
                                    .iter()
                                    .map(|edge| edge.redefining.clone())
                                    .collect(),
                            },
                            detail: format!(
                                "type {} has {} undominated redefinitions of {}: {}",
                                type_key.node,
                                edges.len(),
                                target_key.node,
                                edge_paths.join(" and ")
                            ),
                        },
                    );
                }
            }
            winner
        };

        let member_key = (type_key.clone(), target_key.clone());
        if !member_preimages.contains_key(&member_key) {
            // `normalize.redefinition-check` exposes this refusal
            // (`value-accounting.md:455`), so `record_phase4_refusal` below
            // ranks it by the least of this group's own redefining members'
            // own keys — `:455`'s own "ascending by the member's own key"
            // order (there is no separate redefinition-record key under
            // QSpec's inline shape); every edge in this group shares the
            // same unreachable target, so whichever of them sorts first is
            // the one that order would check first. Collected into
            // `accounting.refusals` rather than returned here (see the
            // module docs), so every remaining type and target group is
            // still resolved and every later phase-4 charge amount is still
            // computed correctly.
            //
            // This exact member can also be reached, and fail the identical
            // check for the identical reason, at more than one `type_key`
            // whenever a descendant of `least_edge.owner` also inherits it
            // (both compute the same `least_edge.redefining`, so they rank
            // identically) -- naming `least_edge.owner` rather than
            // `type_key` keeps the reported refusal the same regardless of
            // which of those tied candidates this pass happens to keep.
            //
            // QSL #184: the spec's own name for this refusal is
            // `invalid_model_binding`/`redefinition-target` ("target is not
            // a member inherited by its owning type"), not
            // `dangling_reference`/`RedefinitionUnreachable` -- the target
            // key does resolve to a real declaration somewhere in the
            // domain package (it is simply not one of `least_edge.owner`'s
            // own effective members), so it is not a dangling reference.
            let least_edge = edges
                .iter()
                .min_by(|a, b| a.redefining.cmp(&b.redefining))
                .expect("a target group always has at least one edge");
            // `model-complete.md:298-300`: this refusal lists every
            // redefining member that names the unreachable target, not only
            // `least_edge` (M2 finding, PR #228 review) -- `least_edge`
            // still decides the refusal's own rank (unchanged).
            let mut redefiners: Vec<DeclarationKey> =
                edges.iter().map(|edge| edge.redefining.clone()).collect();
            redefiners.sort();
            let redefiner_names: Vec<&str> =
                redefiners.iter().map(|key| key.node.as_str()).collect();
            record_phase4_refusal(
                accounting,
                Phase4Rank::RedefinitionCheck(least_edge.redefining.clone()),
                ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: ModelRefusalCause::RedefinitionTarget {
                        redefiners: redefiners.clone(),
                        target: target_key.clone(),
                    },
                    detail: format!(
                        "{} redefine{} {}, which is not a member {} inherits",
                        redefiner_names.join(", "),
                        if redefiners.len() == 1 { "s" } else { "" },
                        target_key.node,
                        least_edge.owner.node
                    ),
                },
            );
            continue 'targets;
        }

        for (i, edge) in edges.iter().enumerate() {
            let mut inputs = edge.path.clone();
            inputs.push(edge.redefining.clone());
            inputs.push(edge.target.clone());

            let redefining_key = (type_key.clone(), edge.redefining.clone());
            // Always present: `owner_paths.get(owner)` above already
            // restricted every edge in this group to an `owner` that is
            // either `type_key` itself or one of its ancestors in `paths`,
            // and phase 3's own per-type loop inserts a `member_preimages`
            // entry for every direct member of `type_key` and of every
            // ancestor along `paths` -- `edge.redefining` is one such direct
            // member of `edge.owner`, so `(type_key, edge.redefining)` was
            // already inserted before this phase-4 pass ever runs.
            let entry = member_preimages.get_mut(&redefining_key).expect(
                "a redefining member whose owner reaches type_key is always one of type_key's own effective members, inserted by phase 3 above",
            );
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
            // — one entry per redefining member reaching this target, not
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
    let mut operation_groups: HashMap<DeclarationKey, Vec<RedefinitionEdge>> = HashMap::new();
    for edge in operation_edges {
        operation_groups
            .entry(edge.target.clone())
            .or_default()
            .push(edge);
    }
    let mut operation_target_keys: Vec<DeclarationKey> = operation_groups.keys().cloned().collect();
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
        accounting.conflict_charges.push((
            owner_effective_id.clone(),
            target_key.clone(),
            fact_total.saturating_mul(c.saturating_sub(1)),
        ));
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
    closures: &HashMap<DeclarationKey, HashSet<DeclarationKey>>,
    p_owner: &DeclarationKey,
    q_owner: &DeclarationKey,
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
/// `charge_all`'s own denial: an ordinary metered [`Incomplete`], or one of
/// `built`'s own refusal-vec fields (`intake_refusals`, `phase3_refusals`,
/// `phase4_refusals`), each reported only once every charge its own stage
/// admits (see the module docs). `From<Incomplete>` lets
/// `meter.charge(...)?` keep working unchanged throughout `charge_all`.
enum ChargeAllDenial {
    Incomplete(Incomplete),
    Refused(Vec<ModelRefusal>),
}

impl From<Incomplete> for ChargeAllDenial {
    fn from(incomplete: Incomplete) -> Self {
        ChargeAllDenial::Incomplete(incomplete)
    }
}

fn charge_all(
    domain_package: &DomainPackage,
    built: &mut Built,
    meter: &mut Meter,
) -> Result<(), ChargeAllDenial> {
    // #141 F11: only the running position (`index + 1`) is charged, never a
    // key's value or its relative order, so collecting and sorting a
    // `Vec<DeclarationKey>` just to throw the order away was dead work.
    //
    // `normalize.record` charges once per IR node (`value-accounting.md`:489).
    // Under QSpec's inline shape (`model-complete.md`:155/159/160,
    // :270/271) a supertype, redefinition or subsetting relationship is a
    // property of the declaration record that owns it -- `ObjectTypeRecord`,
    // `FieldMemberRecord`, `OperationMemberRecord` -- never a record of its
    // own, so there is no separate variant left to filter out here:
    // `domain_package.records.len()` already is the exact IR node count.
    for index in 0..domain_package.records.len() {
        meter.charge(
            Charge::new(ChargePoint::NormalizeRecord)
                .size(LimitKind::DeclarationRecords, length_amount(index + 1)),
        )?;
    }

    // QSL #199: every `normalize.record` charge above is admitted before
    // this is ever consulted -- `Built::intake_refusals`'s own doc.
    if !built.intake_refusals.is_empty() {
        return Err(ChargeAllDenial::Refused(std::mem::take(
            &mut built.intake_refusals,
        )));
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
        // QSL #193: a closing extension is charged its own
        // `normalize.cycle-check` above but derives no fact
        // (`value-accounting.md:491`; `PendingFact::closes_cycle`'s own
        // doc), so it is skipped here rather than also charged
        // `normalize.fact` and counted toward `derivation_facts`.
        if fact.closes_cycle {
            continue;
        }
        fact_count += 1;
        meter.charge(
            Charge::new(ChargePoint::NormalizeFact).size(LimitKind::DerivationFacts, fact_count),
        )?;
    }

    // QSL #193: every phase 2/3 charge above (`normalize.record`,
    // `normalize.fact`, `normalize.cycle-check`) is admitted before this is
    // ever consulted -- `Built::phase3_refusals`' own doc.
    // `value-accounting.md:509`: "a stage that reports a refusal ends
    // checking: no later stage runs or charges" -- phase 4's own charges
    // below never run once this is reported.
    if !built.phase3_refusals.is_empty() {
        return Err(ChargeAllDenial::Refused(std::mem::take(
            &mut built.phase3_refusals,
        )));
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
    // admitted before `built.phase4_refusals` (see the module docs) is
    // reported -- an earlier `Incomplete` already returned via `?` above
    // wins instead. `:482`'s "a stage that reports a refusal ends checking:
    // no later stage runs or charges" -- phase 5's own charges below never
    // run once these refusals are reported.
    if !built.phase4_refusals.is_empty() {
        return Err(ChargeAllDenial::Refused(std::mem::take(
            &mut built.phase4_refusals,
        )));
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

/// Normalize `domain_package` under `limits`: FR-150 phases 1, 2, 3, 4 and 5.
///
/// This entry point takes an already-parsed [`DomainPackage`]: the FR-154
/// byte-level intake checks (digest domain, package digest and
/// ModelSelection identity/version against admitted package bytes) run
/// before a caller builds one, and are not repeated here (Remaining work:
/// #131 wires a real Semantic IR 2.0.0 intake in front of this entry point).
/// `validate_references`'s own phase-1 checks -- FR-321's empty-component
/// (`minLength`) and FR-154's colliding-key checks -- are schema-shape
/// checks over the already-parsed [`DomainPackage`] itself, not byte-level
/// digest checks against admitted package bytes, so they run here rather
/// than waiting on that future intake.
pub fn normalize(
    domain_package: &DomainPackage,
    limits: ModelNormalizationLimits,
) -> NormalizeOutcome {
    let (outcome, _meter) = normalize_with_meter(domain_package, limits);
    outcome
}

/// A meter constructed to run `domain_package` under [`ModelNormalizationLimits::UNLIMITED`]
/// and expose its admitted-charge sequence, for tests that assert the exact
/// charge order alongside [`normalize`]'s result.
pub fn normalize_with_meter(
    domain_package: &DomainPackage,
    limits: ModelNormalizationLimits,
) -> (NormalizeOutcome, Meter) {
    let mut meter = Meter::new(limits);
    let mut built = match build(domain_package, &limits) {
        Ok(built) => built,
        // `validate_selection`'s own immediate refusal (see its own doc):
        // decided before any charge at all, so it is always the sole entry.
        Err(refusal) => {
            return (
                NormalizeOutcome::Refused(Refusals::from_vec(vec![refusal])),
                meter,
            )
        }
    };
    let outcome = match charge_all(domain_package, &mut built, &mut meter) {
        Ok(()) => NormalizeOutcome::Completed(built.view),
        Err(ChargeAllDenial::Incomplete(incomplete)) => NormalizeOutcome::Incomplete(incomplete),
        Err(ChargeAllDenial::Refused(refusal)) => {
            NormalizeOutcome::Refused(Refusals::from_vec(refusal))
        }
    };
    (outcome, meter)
}

/// The object universe `domain_package` normalizes to, independent of `charge_all`'s
/// bookkeeping (test and caller convenience; recomputes via [`build`], under
/// [`ModelNormalizationLimits::UNLIMITED`]). Under `UNLIMITED` nothing ever
/// runs out, so `build`'s own deferred refusal-vec fields (`intake_refusals`,
/// `phase3_refusals`, `phase4_refusals` — see the module docs) are surfaced
/// here directly rather than replayed through `charge_all`, checked in that
/// same stage order; this convenience wrapper returns whichever stage's
/// full refusal bundle is non-empty first, the same charge-ordered bundle
/// [`NormalizeOutcome::Refused`] carries (L2 finding, PR #228 review: the
/// previous single-`ModelRefusal` shape dropped every refusal after the
/// first).
pub fn object_universe(
    domain_package: &DomainPackage,
) -> Result<ObjectUniverse, Vec<ModelRefusal>> {
    let built = build(domain_package, &ModelNormalizationLimits::UNLIMITED)
        .map_err(|refusal| vec![refusal])?;
    if !built.intake_refusals.is_empty() {
        return Err(built.intake_refusals);
    }
    if !built.phase3_refusals.is_empty() {
        return Err(built.phase3_refusals);
    }
    if !built.phase4_refusals.is_empty() {
        return Err(built.phase4_refusals);
    }
    Ok(built.universe)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// A synthetic type-level [`ViewEntry`] whose own effective identity is
    /// `byte` repeated 32 times, for a hand-built [`EffectiveView`] that
    /// exercises [`EffectiveView::validate_order`] directly. Unit-test-only:
    /// `EffectiveView`'s fields are private outside this module, so a
    /// deliberately unsorted view can only be built here, in-module, never
    /// by an integration test simulating an external caller.
    fn entry(byte: u8) -> ViewEntry {
        ViewEntry {
            effective_id: EffectiveId::from_digest_bytes([byte; 32]),
            preimage: EffectiveDeclarationPreimage {
                owner_effective_type: None,
                original: DeclarationKey::fixture("model.A"),
                derivation: Vec::new(),
            },
            visible: true,
        }
    }

    /// TC-195 N10: a view whose declarations are not ascending by effective
    /// identity refuses `invalid_model_binding`/`unsorted-view`, naming the
    /// first out-of-order entry's own effective identity.
    #[trace("TC-195")]
    #[test]
    fn n10_unsorted_view_refuses_by_the_semantic_check() {
        let view = EffectiveView {
            model_selection: DomainPackageRef::fixture("test/orders"),
            declarations: vec![entry(2), entry(1)],
        };
        let refusal = view
            .validate_order()
            .expect_err("a view whose declarations are no longer ascending must be refused");
        assert_eq!(
            refusal.cause,
            ModelRefusalCause::UnsortedView {
                at: view.declarations()[1].effective_id.clone(),
            }
        );
        assert_eq!(refusal.code, Code::InvalidModelBinding);
    }
}
