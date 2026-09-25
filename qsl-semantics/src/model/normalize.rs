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
//! **Operation-member** redefinition (TC-196 R02–R08) builds no *view* entry
//! here: `crate::model::conformance` constructs its own
//! [`EffectiveDeclarationPreimage`] directly over the domain package's
//! [`crate::model::domain_package::OperationMemberRecord`]'s own `redefines`
//! property for FR-151's conformance checking, so this pass — which exists to
//! grow [`EffectiveView`] itself — has no member to add for operations. This
//! split is a scope decision recorded here, not a silent gap. It covers only
//! the *view* this pass grows, though: the `m + r`
//! `normalize.redefinition-check` charge below still prices every
//! redefining member and every effective member the spec names,
//! operation and field alike (QSL #145), and a contested operation target
//! (`c >= 2` redefining operation members reaching the same target) is
//! detected and resolved by the identical proper-descendant dominance search
//! this pass's own field-redefinition contention uses (#173): a winner if
//! one redefiner's owner dominates every other, otherwise a typed
//! `redefinition-target`/`derivation-conflict` refusal, naming every
//! competing redefiner exactly as the field case does, and priced by the
//! same `Σ (c − 1) × f(o)` `normalize.conflict-check` charge (QSL #145).
//! Resolving a contested operation target this way builds no
//! [`EffectiveView`] member entry and touches no `member_preimages`/`hidden`
//! state: the view stays field-only, as above; only the ambiguity check
//! itself is shared.
//!
//! This engine takes a [`DomainPackage`] value the caller constructs; it holds no
//! ambient registry. Every identity is SHA-256 over RFC 8785 JCS bytes of a
//! preimage built here, replayed exactly for the same [`DomainPackage`] and
//! [`ModelNormalizationLimits`] — the same inputs always retrace the same
//! derivation and the same bytes.
//!
//! `build` charges the meter inside each phase as it works, in
//! `value-accounting.md`'s charge order, and stops at the first denied
//! charge (QSL-216). The limits therefore bound the work normalization
//! does, not only what it admits: a diamond generalization graph, whose
//! ancestor-path count is exponential in depth, is walked only as far as
//! `derivation_facts` and `work_units` admit (PR #140 F1), and no member
//! identity is hashed past `effective_declarations`. Charging in charge
//! order needs no replay: ancestor paths are walked in DFS pre-order over
//! ascending keys, which is their ascending charge order; every
//! `normalize.fact` charge differs only in the running count it sizes, so
//! each phase charges its facts as it derives them; phase 4 charges its
//! facts and conflict checks before resolving any contest; and phase 5
//! charges each member's and the view's `normalize.hash` from a length
//! counted from parts, before encoding (an effective type's identity is the
//! one hash made earlier; see `build`). Facts derived along one ancestor
//! path share it ([`FactInputs`]), so memory grows with facts plus total
//! path length. `ancestor_steps` is read, not
//! charged: a path one step past it refuses `AncestorSteps` where the walk
//! reaches it, unless an earlier charge was already denied.
//!
//! A closing cycle edge (TC-196 R01) refuses `specialization-cycle` naming
//! every contributing declaration in the cycle, rotated to start at its
//! least key. `ancestor_paths` charges every closing extension it finds
//! (`value-accounting.md:491`: "a closing extension is charged even when
//! its cycle's edge set was already reported") and keeps walking past it —
//! this frame's remaining siblings, every other frame still on its stack,
//! and, in `build`'s own per-type loop, every remaining type — rather than
//! stopping at the first one, so R01's own six-`normalize.cycle-check`
//! accounting across both types' walks is reproduced exactly (QSL #193).
//! `build` deduplicates the refusals themselves by edge set, keeping only
//! each distinct cycle's own first (charge-order-earliest) closing
//! extension, per `model-complete.md`:292-295's "each cycle is refused
//! exactly once" — and reported only
//! once every phase 2/3 charge has admitted (`:505-511`'s "every refusal
//! that work exposes is reported, in charge order, when its stage ends").
//!
//! Phase 4's own dominance check (deciding which of several redefiners of
//! the same target wins) asks a different question than phase 3's own
//! `ancestor_paths`: only *reachability* between two specific owners, never
//! every path between them. Two earlier attempts got this wrong: reusing
//! `ancestor_paths` itself under a budget that silently reset on every
//! call (QSL #145), and then delegating to
//! `ancestor_closure`'s bounded
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
//! no breadth ceiling of its own to exceed. Phase 4 runs only once every
//! phase 2/3 charge is admitted, so those paths are complete: a denied
//! phase 2/3 charge has already stopped `build`. The pairwise dominance
//! loop itself is still `O(|edges|^2)`
//! in the worst case, but each contesting redefinition edge's *owner* — not
//! the edge — is what `owner_ancestor_sets` is keyed on, and TC-196 R07
//! documents the case where several edges share one owner; looking up each
//! distinct owner's already-built set (`O(distinct owners)` build-wide,
//! computed once regardless of how many (type, target) groups query it —
//! QSL #145) rather than re-deriving it per group
//! leaves only cheap `O(1)` set lookups inside the pair enumeration. A
//! target group with fewer than two contesting edges has nothing to
//! dominate and needs no ancestor-set lookup at all — `plan_redefinitions`
//! skips it and both `normalize.conflict-check` charges entirely whenever a
//! group's `edges.len() < 2`, matching `value-accounting.md:456`'s own
//! `c >= 2` condition below. Phase 3's own already-computed ancestor paths
//! for `type_key` are still reused verbatim for `plan_redefinitions`'s
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
//! fact as `normalize.fact` regardless of which phase formed it.
//! `plan_redefinitions` counts them in `phase4_fact_count` and prices each
//! contested group's `normalize.conflict-check`, and `build` charges each
//! type's facts as soon as its plan is made, after every
//! `normalize.redefinition-check` and before any `normalize.conflict-check`,
//! matching `:455`'s "before its first `normalize.fact`" and `:456`'s "after
//! its last `normalize.fact`". Only once every phase-4 charge is admitted
//! does `resolve_redefinitions` run any dominance contest or build any
//! redefine fact (QSL-216).
//!
//! No phase-4 refusal is reported as soon as it is found, whichever shape it
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
//! these shapes, tagged with its own `Phase4Rank`
//! (`record_phase4_refusal`'s own doc, next to `Phase4Accounting`), and
//! keeps resolving every remaining type and target group regardless, so
//! every later phase-4 charge amount is still computed correctly; once every
//! call has returned, `build` sorts the whole collection into charge order,
//! charges through the last `normalize.conflict-check` and returns every
//! one of these refusals, together, in that charge order, only once every
//! phase-4 charge has been admitted (`:505-511`'s "every refusal that work exposes is
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

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use crate::model::accounting::{
    Charge, ChargePoint, Incomplete, LimitKind, Meter, ModelNormalizationLimits,
};
use crate::model::domain_package::DomainPackageRefWire;
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, PopulationRecord,
};
use crate::model::index::{DeclIdx, ModelIndex, RecordIndex, Redefiner};
use crate::model::key::{
    canonical_len, sha256_and_len, DeclarationKey, EffectiveDeclarationPreimage,
    EffectiveDeclarationWire, EffectiveId, EffectiveIdWire, Fact, FactInputs, KeyPath, RuleRefWire,
    RULE_INHERIT, RULE_QUALIFY, RULE_REDEFINE,
};
use qsl_foundation::diagnostic::Code;
use quire_exact::length_amount;
use serde::Serialize;

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

impl ModelRefusal {
    /// ADR-013 O-17 (FR-090-AC-8): this refusal's catalog code, its cause's
    /// own [`ModelRefusalCause::catalog_code`]. Reads the cause alone, never
    /// the native-v1 `code` (ADR-013 R-09).
    pub fn catalog_code(&self) -> qsl_foundation::diagnostic::CatalogCode {
        self.cause.catalog_code()
    }
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
    /// Builds this type directly from its own non-empty shape: a `first`
    /// refusal a caller already has in hand, plus the `rest` in original
    /// order. The only public production constructor (M2 finding, PR #228
    /// round 2 review) -- unlike the previous sole `from_vec`, this cannot be
    /// called with an empty collection at all, so there is nothing left to
    /// panic over.
    pub fn new(first: ModelRefusal, rest: Vec<ModelRefusal>) -> Self {
        Self { first, rest }
    }

    /// Splits a non-empty `Vec` into this type's own `first`/`rest` shape --
    /// the shape every crate-internal producer already builds. Panics on an
    /// empty `Vec`: every real producer already has at least one refusal in
    /// hand (this type's own doc). Test-only (M2 finding, PR #228 round 2
    /// review): a fixture builds its expected refusals as a plain `Vec` far
    /// more often than as a `first`/`rest` pair, and a test's own panic on an
    /// empty fixture is a fixture bug worth failing loudly on, not a
    /// production caller's concern -- production code builds a `Refusals`
    /// through [`Self::new`] or [`TryFrom`] instead, neither of which can
    /// panic. Follows [`DeclarationKey::fixture`]'s identical
    /// `test`/`test-support` gate.
    #[cfg(any(test, feature = "test-support"))]
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

    /// The first refusal by value, discarding the rest -- for a caller whose
    /// own outcome shape (like FR-153 admission's `AdmissionOutcome::Refused`)
    /// surfaces only one refusal (M2 finding, PR #228 round 2 review): a
    /// guaranteed-present read with no `.expect()` of its own, since
    /// `Refusals` is non-empty by construction.
    pub fn into_first(self) -> ModelRefusal {
        self.first
    }
}

/// [`Refusals::new`]'s own empty-collection counterpart: `TryFrom<Vec<_>>`
/// fails, rather than panics, on an empty `Vec` (M2 finding, PR #228 round 2
/// review) -- for a caller that already holds a `Vec` and does not know,
/// until it checks, whether it is empty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmptyRefusals;

impl TryFrom<Vec<ModelRefusal>> for Refusals {
    type Error = EmptyRefusals;

    fn try_from(refusals: Vec<ModelRefusal>) -> Result<Self, EmptyRefusals> {
        let mut iter = refusals.into_iter();
        let first = iter.next().ok_or(EmptyRefusals)?;
        Ok(Self {
            first,
            rest: iter.collect(),
        })
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
/// [`DomainPackage`] admits, sorted ascending by effective identity, together
/// with the domain package it was normalized from.
///
/// Every field is private and [`normalize`]/[`normalize_shared`] are this
/// type's only constructors, so a view always carries the exact
/// [`DomainPackage`] its declarations, type identities and object universes
/// were computed from. A caller cannot pair a view with some other package:
/// population admission reads the package from the view itself (QSL-204), so
/// there is no second package whose correspondence to the view would need
/// checking or re-normalizing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectiveView {
    /// The domain package this view was normalized from, together with the
    /// [`ModelIndex`] this normalization built over it (QSL-202). Shared,
    /// never copied, by every [`crate::model::population::PopulationBinding`]
    /// admitted against this view.
    index: Arc<ModelIndex>,
    /// Everything else normalization computed from the package.
    body: ViewBody,
}

/// What normalization computes for an [`EffectiveView`], apart from the
/// package itself. `build` produces this alone, so a normalization that is
/// refused or runs out of limits never copies or retains the package.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ViewBody {
    /// Every admitted declaration, ascending by [`EffectiveId`].
    declarations: Vec<ViewEntry>,
    /// Every top-level declaration's effective identity, keyed by its
    /// original producer [`DeclarationKey`], and the reverse: exactly the
    /// `declarations` entries with no owner effective type. Derived from
    /// `declarations` at construction, so it adds no identity of its own.
    /// Shared, never copied, by every population binding admitted against
    /// the view.
    types: Arc<TypeCatalog>,
    /// The object universes this normalization produced, one per connected
    /// component of the object-type supertype graph (ADR-013 §8 OQ-E),
    /// ascending by each universe's own first root type identity -- the
    /// exact `normalize.hash` charge order (`value-accounting.md`:495).
    /// Non-empty by type: a domain package with no declared object type
    /// still gets one universe, with empty `root_types`. Not part of the
    /// view's own preimage (`wire`), so it adds no identity of its own.
    universes: ObjectUniverses,
    /// Every declared object type's position in `universes`: its own
    /// connected component's universe.
    universe_index_by_type: BTreeMap<DeclarationKey, usize>,
    /// Every `Population` record of the package, keyed by its own
    /// declaration key.
    populations: BTreeMap<DeclarationKey, PopulationEntry>,
    /// The limits this view was normalized under (QSL-222).
    limits: ModelNormalizationLimits,
}

/// Every declared type's effective identity, keyed both ways: by its
/// original producer [`DeclarationKey`], and by the [`EffectiveId`] a checked
/// `Reference<T>` carries for `T` (ADR-013 O-05). Built once per
/// normalization.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TypeCatalog {
    by_key: BTreeMap<DeclarationKey, EffectiveId>,
    by_identity: BTreeMap<EffectiveId, DeclarationKey>,
}

impl TypeCatalog {
    fn new(by_key: BTreeMap<DeclarationKey, EffectiveId>) -> Self {
        let by_identity = by_key
            .iter()
            .map(|(key, identity)| (*identity, key.clone()))
            .collect();
        Self {
            by_key,
            by_identity,
        }
    }

    /// Every declared type's effective identity, by producer key.
    pub(crate) fn by_key(&self) -> &BTreeMap<DeclarationKey, EffectiveId> {
        &self.by_key
    }

    /// The producer key of the declared type whose effective identity is
    /// `identity`.
    pub(crate) fn key_of(&self, identity: &EffectiveId) -> Option<&DeclarationKey> {
        self.by_identity.get(identity)
    }
}

impl ViewBody {
    /// The RFC 8785 length of [`Self::wire`] under `model_selection`, given
    /// `declarations_len`, the encoded length of its `declarations` elements
    /// (each [`view_entry_len`], comma-separated): the view with no
    /// declarations, plus those elements. Nothing but the header is encoded.
    fn canonical_len_from_parts(
        &self,
        model_selection: &DomainPackageRef,
        declarations_len: u64,
    ) -> u64 {
        canonical_len(&Self::wire_header(model_selection)).saturating_add(declarations_len)
    }

    /// [`Self::wire`] with no declarations.
    fn wire_header(model_selection: &DomainPackageRef) -> EffectiveViewWire<'_> {
        EffectiveViewWire {
            version: crate::model::key::EFFECTIVE_VIEW_DOMAIN,
            model_selection: model_selection.wire(),
            rules: RuleRefWire {
                identity: crate::model::key::RULES_IDENTITY,
                revision: crate::model::key::RULES_REVISION,
            },
            declarations: Vec::new(),
        }
    }

    /// The view's `quire.model.effective-view/v1` preimage form under
    /// `model_selection`: `{version, model_selection, rules, declarations}`,
    /// each declaration `{effective_id, preimage}` -- the package's own
    /// header and every declaration, never the universes, populations or
    /// `visible` bits.
    fn wire<'a>(&'a self, model_selection: &'a DomainPackageRef) -> EffectiveViewWire<'a> {
        EffectiveViewWire {
            declarations: self
                .declarations
                .iter()
                .map(|entry| ViewEntryWire {
                    effective_id: EffectiveIdWire::from(&entry.effective_id),
                    preimage: entry.preimage.wire(),
                })
                .collect(),
            ..Self::wire_header(model_selection)
        }
    }
}

/// One `Population` record of a view's package, with the position in the
/// view's universes of the universe its bindings are admitted into
/// (ADR-013 §8 OQ-E: its first member type's component, or the first
/// universe when it declares no member type).
#[derive(Clone, Debug, Eq, PartialEq)]
struct PopulationEntry {
    record: PopulationRecord,
    universe_index: usize,
}

/// A population declaration as an [`EffectiveView`] resolves it for
/// admission: the package's own record and the object universe its bindings
/// are admitted into.
#[derive(Clone, Copy, Debug)]
pub struct ViewPopulation<'a> {
    /// The package's own `Population` record.
    pub record: &'a PopulationRecord,
    /// The object universe this population's bindings are admitted into.
    pub universe: &'a ObjectUniverse,
}

/// A non-empty, ordered list of object universes: the first, then the rest.
#[derive(Clone, Debug, Eq, PartialEq)]
struct ObjectUniverses {
    first: ObjectUniverse,
    rest: Vec<ObjectUniverse>,
}

impl ObjectUniverses {
    /// The universe at `index` in ascending order, if any.
    fn get(&self, index: usize) -> Option<&ObjectUniverse> {
        match index.checked_sub(1) {
            None => Some(&self.first),
            Some(rest_index) => self.rest.get(rest_index),
        }
    }

    fn iter(&self) -> impl Iterator<Item = &ObjectUniverse> {
        std::iter::once(&self.first).chain(&self.rest)
    }
}

impl EffectiveView {
    /// The domain package this view was normalized from.
    pub fn domain_package(&self) -> &DomainPackage {
        self.index.package()
    }

    /// The shared [`ModelIndex`] over [`EffectiveView::domain_package`],
    /// built once by the normalization that produced this view.
    pub fn model_index(&self) -> &ModelIndex {
        &self.index
    }

    /// The shared handle to [`EffectiveView::model_index`], for a binding
    /// that keeps the index alive beyond this view.
    pub(crate) fn shared_model_index(&self) -> &Arc<ModelIndex> {
        &self.index
    }

    /// The shared handle to this view's type catalog, for a binding that
    /// reads it beyond this view.
    pub(crate) fn shared_type_catalog(&self) -> &Arc<TypeCatalog> {
        &self.body.types
    }

    /// The model selection this view was normalized under.
    pub fn model_selection(&self) -> &DomainPackageRef {
        &self.domain_package().model_selection
    }

    /// The limits this view was normalized under: the caller's, as given,
    /// or [`ModelNormalizationLimits::default`]'s finite ceilings (QSL-222).
    pub fn effective_limits(&self) -> &ModelNormalizationLimits {
        &self.body.limits
    }

    /// Every admitted declaration, ascending by [`EffectiveId`].
    pub fn declarations(&self) -> &[ViewEntry] {
        &self.body.declarations
    }

    /// Every top-level declaration's effective identity, keyed by its
    /// original producer [`DeclarationKey`]; member declarations (those with
    /// an owner effective type) are excluded. This is the one
    /// `DeclarationKey` -> [`EffectiveId`] correspondence for declared types:
    /// FR-143's reference type component (ADR-013 O-05) and FR-153's
    /// population type catalog both read it. Normalization computes it once.
    pub fn type_identities(&self) -> &BTreeMap<DeclarationKey, EffectiveId> {
        self.body.types.by_key()
    }

    /// Every object universe this view's normalization produced, one per
    /// connected component of the object-type supertype graph (ADR-013 §8
    /// OQ-E), ascending by each universe's own first root type identity.
    /// Always yields at least one universe.
    pub fn object_universes(&self) -> impl Iterator<Item = &ObjectUniverse> {
        self.body.universes.iter()
    }

    /// `type_key`'s own object universe (ADR-013 §8 OQ-E): the universe of
    /// the connected component of the object-type supertype graph that
    /// contains `type_key`. `None` when `type_key` names no declared object
    /// type of the normalized domain package.
    pub fn object_universe_of(&self, type_key: &DeclarationKey) -> Option<&ObjectUniverse> {
        self.body
            .universe_index_by_type
            .get(type_key)
            .and_then(|index| self.body.universes.get(*index))
    }

    /// The first object universe, ascending by first root type identity:
    /// the sole universe of a domain package whose object-type supertype
    /// graph has exactly one connected component. A domain package with
    /// more than one component still yields a single universe here rather
    /// than refusing; callers that need a *specific* type's universe call
    /// [`EffectiveView::object_universe_of`], and callers that need every
    /// universe call [`EffectiveView::object_universes`].
    pub fn object_universe(&self) -> &ObjectUniverse {
        &self.body.universes.first
    }

    /// The `Population` record of this view's domain package declared under
    /// `key`, with the object universe its bindings are admitted into, or
    /// `None` when the package declares no population under `key`.
    pub fn population(&self, key: &DeclarationKey) -> Option<ViewPopulation<'_>> {
        let entry = self.body.populations.get(key)?;
        Some(ViewPopulation {
            record: &entry.record,
            universe: self.body.universes.get(entry.universe_index)?,
        })
    }

    fn wire(&self) -> EffectiveViewWire<'_> {
        self.body.wire(self.model_selection())
    }

    /// This view's `quire.model.effective-view/v1` identity, encoded by
    /// `quire-canonical` (ADR-013 §2, ADR-013:113: one RFC 8785
    /// implementation).
    pub fn identity(&self) -> EffectiveId {
        EffectiveId::from_digest(sha256_and_len(&self.wire()).0)
    }

    /// The length of this view's RFC 8785 bytes (for `normalize.hash`
    /// accounting, and for tests asserting an exact hashed-byte length
    /// against a ground-truth vector), counted by the encoder.
    pub fn canonical_len(&self) -> u64 {
        canonical_len(&self.wire())
    }

    /// Whether `declarations` is correctly sorted ascending by effective
    /// identity — TC-195 N10's `unsorted-view` mutation, "refused by the
    /// semantic check" over an already-constructed view (PR #140 F5).
    pub fn validate_order(&self) -> Result<(), ModelRefusal> {
        for pair in self.body.declarations.windows(2) {
            if pair[0].effective_id > pair[1].effective_id {
                return Err(ModelRefusal {
                    code: Code::InvalidModelBinding,
                    cause: ModelRefusalCause::UnsortedView {
                        at: pair[1].effective_id,
                    },
                    detail: format!(
                        "view declarations are not sorted ascending by effective identity at {}",
                        pair[1].effective_id
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
    /// Every root effective type (no generalization ancestor) of this
    /// universe's own connected component of the object-type supertype
    /// graph (ADR-013 §8 OQ-E), ascending by [`EffectiveId`] -- never every
    /// root type in the domain package.
    pub root_types: Vec<EffectiveId>,
}

impl ObjectUniverse {
    /// This universe's `quire.model.object-universe/v1` preimage form:
    /// `{version, model_selection, root_types}`.
    fn wire(&self) -> ObjectUniverseWire<'_> {
        ObjectUniverseWire {
            version: crate::model::key::OBJECT_UNIVERSE_DOMAIN,
            model_selection: self.model_selection.wire(),
            root_types: self.root_types.iter().map(EffectiveIdWire::from).collect(),
        }
    }

    /// This universe's `quire.model.object-universe/v1` identity (ADR-013
    /// §8 OQ-C ruling: a `UniverseId`, not an `EffectiveId` -- the two
    /// digests use the same SHA-256 computation over the same preimage
    /// shape, differing only in which kernel newtype carries the result).
    /// Encoded by `quire-canonical` (ADR-013 §2, ADR-013:113).
    pub fn identity(&self) -> quire_exact::UniverseId {
        quire_exact::UniverseId::from_digest(sha256_and_len(&self.wire()).0)
    }

    /// The length of this universe's RFC 8785 bytes (for `normalize.hash`
    /// accounting, and for tests asserting an exact hashed-byte length
    /// against a ground-truth vector), counted by the encoder.
    pub fn canonical_len(&self) -> u64 {
        canonical_len(&self.wire())
    }
}

#[derive(Serialize)]
struct EffectiveViewWire<'a> {
    version: &'static str,
    model_selection: DomainPackageRefWire<'a>,
    rules: RuleRefWire,
    declarations: Vec<ViewEntryWire<'a>>,
}

#[derive(Serialize)]
struct ViewEntryWire<'a> {
    effective_id: EffectiveIdWire,
    preimage: EffectiveDeclarationWire<'a>,
}

#[derive(Serialize)]
struct ObjectUniverseWire<'a> {
    version: &'static str,
    model_selection: DomainPackageRefWire<'a>,
    root_types: Vec<EffectiveIdWire>,
}

/// Every declared object type's connected-component id in the undirected
/// supertype graph restricted to object types (ADR-013 §8 OQ-E:
/// `model-complete.md`'s "Object universe" -- "the connected component of
/// `T` in the supertype graph restricted to object types"). `supertypes[]`
/// edges are read in both directions: `type_keys` and every key a
/// [`ModelIndex::generalization_edges`] edge names are already guaranteed
/// declared object types by the time this runs (a dangling supertype
/// reference is an intake refusal reported before `build` ever reaches this
/// call, and the index only ever collects `ObjectTypeRecord.supertypes[]`
/// entries). A type with no supertype and no
/// subtype is its own singleton component. Component ids are assigned in
/// `type_keys`' own ascending order and carry no meaning beyond grouping --
/// callers needing a stable, spec-meaningful order sort the resulting
/// universes by their own first root type identity instead (`build`'s own
/// call site does exactly that).
fn connected_components(
    type_keys: &[DeclarationKey],
    index: &RecordIndex,
) -> HashMap<DeclarationKey, usize> {
    let mut adjacency: HashMap<&DeclarationKey, Vec<&DeclarationKey>> = HashMap::new();
    for key in type_keys {
        adjacency.entry(key).or_default();
    }
    for (specific, general) in index.generalization_edges() {
        adjacency.entry(specific).or_default().push(general);
        adjacency.entry(general).or_default().push(specific);
    }
    let mut component_of: HashMap<DeclarationKey, usize> = HashMap::new();
    let mut next_component = 0usize;
    for start in type_keys {
        if component_of.contains_key(start) {
            continue;
        }
        let mut stack = vec![start];
        component_of.insert(start.clone(), next_component);
        while let Some(node) = stack.pop() {
            if let Some(neighbors) = adjacency.get(node) {
                for &neighbor in neighbors {
                    if !component_of.contains_key(neighbor) {
                        component_of.insert(neighbor.clone(), next_component);
                        stack.push(neighbor);
                    }
                }
            }
        }
        next_component += 1;
    }
    component_of
}

/// One path from a type to a strict ancestor: the ordered chain of
/// supertype-record keys taken, and the ancestor's own original
/// producer key (PR #140 F2: the full key, not a display identity string).
struct AncestorPath {
    path: KeyPath,
    ancestor_key: DeclarationKey,
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
    /// phase-3 charge-order key, so it sorts into
    /// `normalize.cycle-check`/`normalize.fact` charge order
    /// (`value-accounting.md:490`) exactly like an ordinary ancestor-path
    /// fact from the same type, and its length is `normalize.cycle-check`'s
    /// own `work_units += L` (`:491`).
    path: KeyPath,
    edge_set: Vec<DeclarationKey>,
}

/// One [`ClosingCycle`] collected build-wide across `build`'s own per-type
/// loop, tagged with the type whose walk found it (L3 finding, PR #228
/// review: previously an anonymous 4-tuple). Sorted by `(type_key, path)` --
/// the same charge-order key `ancestor_paths` charges its path in --
/// then deduplicated by `edge_set` only after every type
/// has been walked (`model-complete.md`:292-295's "each cycle is refused ...
/// exactly once"; see the module docs).
struct CycleCandidate {
    type_key: DeclarationKey,
    path: KeyPath,
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
/// direct generalizations, charged as the walk finds it (QSL-216).
///
/// DFS pre-order over ascending keys is ascending path order, which is
/// phase 3's `normalize.cycle-check`/`normalize.fact` charge order for one
/// type (`value-accounting.md:490`). Each path is therefore charged its
/// `normalize.cycle-check` (`work_units += L`) and its `normalize.fact`
/// before the walk extends it, and the first denied charge stops the walk:
/// a diamond lattice with exponentially many paths is walked only as far as
/// the limits admit. A closing extension is charged its
/// `normalize.cycle-check` only (`value-accounting.md:491`; it derives no
/// fact, `:513`), and the walk keeps going past it -- to this frame's
/// remaining siblings, every frame still on the stack and, in `build`'s own
/// per-type loop, every remaining type -- so every closing extension in the
/// domain package is charged, not only the first. Explicit stack, not native
/// recursion: domain package data is caller-supplied and may describe a
/// cycle.
///
/// `max_steps` is the caller's
/// [`crate::model::accounting::ModelNormalizationLimits::ancestor_steps`],
/// used as given: an ancestor path of `n` generalization steps is admitted
/// at `max_steps == n`, and extending any path one step further refuses
/// [`ModelRefusalCause::AncestorSteps`] naming `root_key` and the bound.
fn ancestor_paths(
    root_key: &DeclarationKey,
    index: &RecordIndex,
    max_steps: u64,
    charges: &mut Charges<'_>,
) -> Result<AncestorWalk, Denial> {
    struct Frame {
        directs: Vec<DeclarationKey>,
        next: usize,
        path: KeyPath,
        visited: Vec<DeclarationKey>,
    }

    let mut stack = vec![Frame {
        directs: index.sorted_generals(root_key),
        next: 0,
        path: KeyPath::default(),
        visited: vec![root_key.clone()],
    }];
    let mut out = Vec::new();
    let mut closing_cycles: Vec<ClosingCycle> = Vec::new();
    loop {
        let stack_len = stack.len();
        let Some(frame) = stack.last_mut() else { break };
        if frame.next >= frame.directs.len() {
            stack.pop();
            continue;
        }
        work_step();
        // Extending this frame's path gives a path of `stack_len`
        // generalization steps (the root frame's own path is empty).
        if u64::try_from(stack_len).unwrap_or(u64::MAX) > max_steps {
            return Err(Denial::from(ModelRefusal {
                code: Code::ResourceExhausted,
                cause: ModelRefusalCause::AncestorSteps {
                    from: root_key.clone(),
                    limit: max_steps,
                },
                detail: format!(
                    "ancestor path from {} exceeded the ancestor_steps limit of {max_steps}",
                    root_key.node
                ),
            }));
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
        let new_path = frame.path.extended(&ancestor_key);

        charges.cycle_check(new_path.len())?;
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
            closing_cycles.push(ClosingCycle {
                refusal,
                path: new_path,
                edge_set: chain,
            });
            continue;
        }
        charges.fact()?;
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
    index: &RecordIndex,
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
            // already an object type of `index` from this identical
            // record under QSpec's inline `supertypes[]` shape
            // (`model-complete.md`:155) -- so an "unknown specific" is
            // structurally unreachable and there is no matching
            // `ModelRefusalCause` variant for it.
            for general in &t.supertypes {
                if !index.is_object_type(general) {
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
            if !index.is_object_type(&member.owner) && !index.is_record_value_type(&member.owner) {
                refusals.push(ModelRefusal {
                    code: Code::DanglingReference,
                    cause: ModelRefusalCause::UnknownOwner {
                        member: member.key.clone(),
                        owner: member.owner.clone(),
                    },
                    detail: format!(
                        "field member {} names owner {}, which is not a declared object type or record value type",
                        member.key.node, member.owner.node
                    ),
                });
            }
            if let Some(redefines) = &member.redefines {
                if !index.is_field_member(redefines) && !index.is_operation_member(redefines) {
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
                if !index.is_field_member(subsetted) && !index.is_operation_member(subsetted) {
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
        DomainPackageRecord::RecordValueType(_) | DomainPackageRecord::ScalarType(_) => {}
        DomainPackageRecord::OperationMember(op) => {
            if !index.is_object_type(&op.owner) {
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
            // A native value type ([`ValueTypeRef::Native`]) names no
            // node of any package, so it has nothing to dangle against
            // here; only a package value type is checked.
            for parameter in &op.parameters {
                if let Some(value_type) = parameter.value_type.as_package() {
                    if !index.is_object_type(value_type) && !index.is_scalar_type(value_type) {
                        refusals.push(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: ModelRefusalCause::UnknownValueType {
                                operation: op.key.clone(),
                                parameter: Some(parameter.key.clone()),
                                value_type: parameter.value_type.clone(),
                            },
                            detail: format!(
                                "operation {} parameter {} names value type {}, which is not a declared type",
                                op.key.node, parameter.key.node, parameter.value_type
                            ),
                        });
                    }
                }
            }
            if let Some(result) = &op.result {
                if let Some(value_type) = result.value_type.as_package() {
                    if !index.is_object_type(value_type) && !index.is_scalar_type(value_type) {
                        refusals.push(ModelRefusal {
                            code: Code::DanglingReference,
                            cause: ModelRefusalCause::UnknownValueType {
                                operation: op.key.clone(),
                                parameter: None,
                                value_type: result.value_type.clone(),
                            },
                            detail: format!(
                                "operation {} result names value type {}, which is not a declared type",
                                op.key.node, result.value_type
                            ),
                        });
                    }
                }
            }
            for field in &op.effect.modifies {
                if !index.is_field_member(field) {
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
                if !index.is_object_type(target) {
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
                if !index.is_field_member(redefines) && !index.is_operation_member(redefines) {
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
        | DomainPackageRecord::Relationship(_)
        | DomainPackageRecord::Allocation(_) => {}
        // model-complete.md's "Populations" row: each member type names
        // a declared object type; a missing one refuses
        // `missing_declaration`/`missing-name`.
        DomainPackageRecord::Population(population) => {
            for type_name in &population.member_types {
                // FR-208-AC-9: a record value type is a declared type of
                // another meaning, not a missing one.
                if index.is_record_value_type(type_name) {
                    refusals.push(ModelRefusal {
                        code: Code::InvalidModelBinding,
                        cause: ModelRefusalCause::MalformedDeclaration,
                        detail: format!(
                            "population {} names member type {}, a record value type, not an object type",
                            population.key.node, type_name.node
                        ),
                    });
                } else if !index.is_object_type(type_name) {
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
fn validate_references(domain_package: &DomainPackage, index: &RecordIndex) -> Vec<ModelRefusal> {
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

#[cfg(test)]
thread_local! {
    /// How many times [`build`] has run on this thread. Observation only: it
    /// changes no result. Unit tests read it through [`build_calls`] to
    /// count how often a caller normalizes a package (QSL-204).
    static BUILD_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// How many units of per-declaration work [`build`] has done on this
    /// thread: ancestor-walk steps, inherited-member facts and member
    /// identity hashes. Observation only; unit tests read it through
    /// [`work_steps`] to show a refused normalization's work is bounded by
    /// the limit, not by the package (QSL-216).
    static WORK_STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times [`build`] has run on the current thread (test-only).
#[cfg(test)]
pub(crate) fn build_calls() -> usize {
    BUILD_CALLS.with(std::cell::Cell::get)
}

/// How many units of per-declaration work [`build`] has done on the
/// current thread (test-only; see `WORK_STEPS`).
#[cfg(test)]
fn work_steps() -> u64 {
    WORK_STEPS.with(std::cell::Cell::get)
}

/// Counts one unit of per-declaration work in a test build; a no-op
/// otherwise.
#[inline]
fn work_step() {
    #[cfg(test)]
    WORK_STEPS.with(|steps| steps.set(steps.get() + 1));
}

/// Normalize `domain_package`, charging `meter` inside each phase as the
/// work happens (QSL-216; see the module docs). Every charge is made in
/// `value-accounting.md`'s charge order before the work it prices grows any
/// further, and the first denied charge stops normalization.
fn build(
    domain_package: &DomainPackage,
    meter: &mut Meter,
) -> Result<(ViewBody, RecordIndex), Denial> {
    #[cfg(test)]
    BUILD_CALLS.with(|calls| calls.set(calls.get() + 1));
    validate_selection(domain_package)?;
    // `normalize.record` charges once per IR node, in node order
    // (`value-accounting.md`:489), before the index over those nodes is
    // built. Under QSpec's inline shape (`model-complete.md`:155/159/160,
    // :270/271) a supertype, redefinition or subsetting relationship is a
    // property of the declaration record that owns it, never a record of
    // its own, so `records.len()` is the exact IR node count. Only the
    // running position is charged (#141 F11).
    for position in 0..domain_package.records.len() {
        meter.charge(
            Charge::new(ChargePoint::NormalizeRecord)
                .size(LimitKind::DeclarationRecords, length_amount(position + 1)),
        )?;
    }
    let index = RecordIndex::build(domain_package);
    // QSL #199: every `normalize.record` charge above is admitted before any
    // intake refusal is reported. `model-complete.md`:80: "Intake reports
    // every such refusal in node order, and then no later phase runs".
    if let Ok(refusals) = Refusals::try_from(validate_references(domain_package, &index)) {
        return Err(Denial::Refused(refusals));
    }
    let limits = *meter.limits();
    let mut charges = Charges::new(meter);
    let type_keys: Vec<DeclarationKey> = index.object_types().cloned().collect();

    // Phase 2: one qualify fact per type and one per directly declared field
    // member. Every phase-2 fact sorts before every phase-3 fact, and every
    // `normalize.fact` charge differs only in the running count it sizes, so
    // charging them here, in any order, admits the same sequence as charging
    // them sorted.
    let mut direct_fields = Vec::with_capacity(type_keys.len());
    for type_key in &type_keys {
        charges.fact()?;
        let fields = index.sorted_direct_fields(&domain_package.records, type_key);
        for _ in &fields {
            charges.fact()?;
        }
        direct_fields.push(fields);
    }

    // Phase 3, type level: every type's ancestor paths, charged as each walk
    // finds them. Types are walked ascending by key, and each walk yields its
    // paths ascending, which is `value-accounting.md:490`'s charge order for
    // type-level facts (owner-less facts sort before member facts).
    let mut type_preimages: HashMap<DeclarationKey, EffectiveDeclarationPreimage> = HashMap::new();
    let mut type_effective_ids: HashMap<DeclarationKey, EffectiveId> = HashMap::new();
    let mut type_hashed_bytes: HashMap<DeclarationKey, u64> = HashMap::new();
    // Phase 3's own ancestor paths, retained per type so the member pass
    // below and phase 4 reuse them without walking again (PR #140 F10).
    let mut type_paths: HashMap<DeclarationKey, Vec<AncestorPath>> = HashMap::new();
    // QSL #193: every generalization cycle any type's own walk closes,
    // tagged with its charge-order key `(type_key, path)`, deduplicated by
    // edge set only once every type is walked (`model-complete.md`:292-295's
    // "each cycle is refused ... exactly once"; see the module docs).
    let mut cycle_candidates: Vec<CycleCandidate> = Vec::new();
    for type_key in &type_keys {
        let walk = ancestor_paths(type_key, &index, limits.ancestor_steps, &mut charges)?;
        let mut derivation = vec![Fact {
            ordinal: 0,
            rule: RULE_QUALIFY,
            inputs: vec![type_key.clone()].into(),
        }];
        for ancestor in &walk.paths {
            derivation.push(Fact {
                ordinal: derivation.len(),
                rule: RULE_INHERIT,
                inputs: FactInputs::new(ancestor.path.clone(), Vec::new()),
            });
        }
        for closing in walk.closing_cycles {
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
        // One encoding serves both the identity (needed next, as this type's
        // members' owner, and for phase 4's charge order) and the RFC 8785
        // length phase 5 charges (PR #140 F10). This is the one hash made
        // before its `normalize.hash` charge: that charge sits in phase 5,
        // after every phase-4 charge ordered by these identities. Its
        // encoding holds this type's paths, whose lengths the
        // `normalize.cycle-check` charges above already admitted as work.
        let (effective_id, hashed) = preimage.identity_and_canonical_len();
        type_effective_ids.insert(type_key.clone(), effective_id);
        type_hashed_bytes.insert(type_key.clone(), hashed);
        type_preimages.insert(type_key.clone(), preimage);
        type_paths.insert(type_key.clone(), walk.paths);
    }

    // Phase 3, member level: each type's directly declared members (their
    // qualify facts were charged in phase 2) and the members it inherits
    // along its ancestor paths, one `normalize.fact` each, charged before the
    // fact is built.
    let mut member_preimages: HashMap<
        (DeclarationKey, DeclarationKey),
        EffectiveDeclarationPreimage,
    > = HashMap::new();
    for (type_key, fields) in type_keys.iter().zip(&direct_fields) {
        let owner_effective_id = type_effective_ids[type_key];
        for member in fields {
            member_preimages.insert(
                (type_key.clone(), member.key.clone()),
                EffectiveDeclarationPreimage {
                    owner_effective_type: Some(owner_effective_id),
                    original: member.key.clone(),
                    derivation: vec![Fact {
                        ordinal: 0,
                        rule: RULE_QUALIFY,
                        inputs: vec![member.key.clone()].into(),
                    }],
                },
            );
        }
        let Some(paths) = type_paths.get(type_key) else {
            continue;
        };
        for ancestor in paths {
            for member in
                index.sorted_direct_fields(&domain_package.records, &ancestor.ancestor_key)
            {
                work_step();
                charges.fact()?;
                // The fact shares the ancestor path rather than copying it,
                // so memory grows with facts plus total path length, not
                // facts times path length (QSL-216).
                let inputs = FactInputs::new(ancestor.path.clone(), vec![member.key.clone()]);
                let entry = member_preimages
                    .entry((type_key.clone(), member.key.clone()))
                    .or_insert_with(|| EffectiveDeclarationPreimage {
                        owner_effective_type: Some(owner_effective_id),
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
    drop(direct_fields);

    // QSL #193: keep each distinct cycle's own first (charge-order-earliest)
    // closing extension (`model-complete.md`:292-295), reported once every
    // phase 2/3 charge is admitted; `value-accounting.md:509`: "a stage that
    // reports a refusal ends checking: no later stage runs or charges".
    cycle_candidates.sort_by(|a, b| {
        a.type_key
            .cmp(&b.type_key)
            .then_with(|| a.path.keys().cmp(b.path.keys()))
    });
    let mut seen_edge_sets: HashSet<Vec<DeclarationKey>> = HashSet::new();
    let mut phase3_refusals: Vec<ModelRefusal> = Vec::new();
    for candidate in cycle_candidates {
        if seen_edge_sets.insert(candidate.edge_set) {
            phase3_refusals.push(candidate.refusal);
        }
    }
    if let Ok(refusals) = Refusals::try_from(phase3_refusals) {
        return Err(Denial::Refused(refusals));
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
    // type's own directly-declared operation members, plus every operation
    // directly declared on a proper ancestor reached along that type's own
    // phase-3 `type_paths`.
    for type_key in &type_keys {
        let mut effective_operations: HashSet<DeclIdx> = HashSet::new();
        effective_operations.extend(index.direct_operations(type_key));
        if let Some(paths) = type_paths.get(type_key) {
            for ancestor in paths {
                effective_operations.extend(index.direct_operations(&ancestor.ancestor_key));
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
    // `type_paths` rather than a fresh, separately-bounded walk (QSL #145),
    // computed once, build-wide, not once per (type, target) group.
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
    // field and operation alike -- ascending by the member's own key (there
    // is no separate redefinition-record key under QSpec's inline
    // `redefines` property), `r` the count of members already checked
    // before it in this same domain-package-wide sequence. Each redefining
    // member is the spec's own priced unit, tested once against its own
    // owning type, never once per (member, effective type reaching it) pair.
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
    for (r, (_key, owner)) in all_redefining_members.iter().enumerate() {
        let m = member_counts_by_owner.get(*owner).copied().unwrap_or(0);
        charges.charge(
            Charge::new(ChargePoint::NormalizeRedefinitionCheck)
                .work(m.saturating_add(length_amount(r))),
        )?;
    }

    let mut hidden: HashSet<(DeclarationKey, DeclarationKey)> = HashSet::new();
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
    // Phase-4 redefine facts are derivation facts too
    // (`value-accounting.md:453`), charged as `normalize.fact`, continuing
    // phase 2/3's running count, strictly between every
    // `normalize.redefinition-check` ("before its first `normalize.fact`",
    // `:455`) and every `normalize.conflict-check` ("after its last
    // `normalize.fact`", `:456`). Their charges differ only in the running
    // count, so each type's are charged as soon as that type's plan counts
    // them. Nothing is resolved until every phase-4 charge is admitted
    // (QSL-216): planning prices each type's groups, the charges run in
    // charge order, and only then does any dominance contest run or any
    // redefine fact get built.
    let mut phase4_facts_charged: u64 = 0;
    let mut plans = Vec::with_capacity(type_keys.len());
    for type_key in &type_keys {
        let Some(paths) = type_paths.get(type_key) else {
            continue;
        };
        let plan = plan_redefinitions(&index, type_key, paths, &member_preimages, &mut accounting);
        while phase4_facts_charged < *accounting.phase4_fact_count {
            charges.fact()?;
            phase4_facts_charged += 1;
        }
        plans.push((type_key, plan));
    }
    accounting
        .conflict_charges
        .sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    for (_, _, work) in accounting.conflict_charges.iter() {
        charges.charge(Charge::new(ChargePoint::NormalizeConflictCheck).work(*work))?;
    }
    for (type_key, plan) in plans {
        resolve_redefinitions(
            type_key,
            plan,
            &mut member_preimages,
            &mut hidden,
            &mut accounting,
        );
    }
    phase4_refusal_candidates.sort_by(|a, b| a.0.cmp(&b.0));
    // A `normalize.redefinition-check` refusal's rank is the redefining
    // member's own key alone (`Phase4Rank::RedefinitionCheck`), matching its
    // domain-package-wide, once-per-record charge above: every descendant
    // type that independently reaches the same broken redefining member
    // derives the identical rank and an identical refusal, so a dedup by
    // rank keeps each reported exactly once. A `normalize.conflict-check`
    // refusal's rank also carries the resolving type's own effective
    // identity, so two different types' own conflicts never collapse here.
    phase4_refusal_candidates.dedup_by(|a, b| a.0 == b.0);
    // `value-accounting.md:481`'s "checking is exhaustive within a stage":
    // every phase-4 charge above is admitted before these refusals are
    // reported, and `:482`'s "a stage that reports a refusal ends checking"
    // keeps phase 5 from charging.
    if let Ok(refusals) = Refusals::try_from(
        phase4_refusal_candidates
            .into_iter()
            .map(|(_, refusal)| refusal)
            .collect::<Vec<_>>(),
    ) {
        return Err(Denial::Refused(refusals));
    }

    // Phase 5: `normalize.declaration` then `normalize.hash` per effective
    // declaration -- "effective types, then effective members, each
    // ascending by effective member key" (`value-accounting.md:494`, QSL
    // #195): `(owner effective type identity, original declaration key)`
    // (`model-complete.md`:206). A member's `normalize.hash` is charged from
    // its length counted from its parts, before it is encoded and hashed
    // (QSL-216). `declarations_len` counts the view's `declarations`
    // elements as they are admitted, so the view's own `normalize.hash` is
    // charged without encoding the view.
    let mut entries: Vec<ViewEntry> = Vec::new();
    let mut declarations_len: u64 = 0;
    let mut add_entry_len = |effective_id: &EffectiveId, preimage_len: u64| {
        let separator = u64::from(declarations_len > 0);
        declarations_len = declarations_len
            .saturating_add(separator)
            .saturating_add(view_entry_len(effective_id, preimage_len));
    };
    for type_key in &type_keys {
        let preimage = type_preimages.remove(type_key).expect("built above");
        charges.declaration()?;
        let hashed_bytes = type_hashed_bytes[type_key];
        charges.hash(hashed_bytes)?;
        add_entry_len(&type_effective_ids[type_key], hashed_bytes);
        entries.push(ViewEntry {
            effective_id: type_effective_ids[type_key],
            preimage,
            visible: true,
        });
    }
    let mut member_keys: Vec<(DeclarationKey, DeclarationKey)> =
        member_preimages.keys().cloned().collect();
    member_keys.sort_by(|(owner_a, decl_a), (owner_b, decl_b)| {
        type_effective_ids[owner_a]
            .cmp(&type_effective_ids[owner_b])
            .then_with(|| decl_a.cmp(decl_b))
    });
    for key in member_keys {
        charges.declaration()?;
        work_step();
        let visible = !hidden.contains(&key);
        let preimage = member_preimages.remove(&key).expect("built above");
        let hashed_bytes = preimage.canonical_len_from_parts();
        charges.hash(hashed_bytes)?;
        let (effective_id, encoded_bytes) = preimage.identity_and_canonical_len();
        debug_assert_eq!(
            encoded_bytes, hashed_bytes,
            "a preimage's length from its parts"
        );
        add_entry_len(&effective_id, hashed_bytes);
        entries.push(ViewEntry {
            effective_id,
            preimage,
            visible,
        });
    }
    entries.sort_by_key(|a| a.effective_id);

    // ADR-013 §8 OQ-E: one universe per connected component of the
    // object-type supertype graph, never one universe over every root type
    // in the domain package (`model-complete.md`'s "Object universe").
    let component_of = connected_components(&type_keys, &index);
    let mut members_by_component: HashMap<usize, Vec<DeclarationKey>> = HashMap::new();
    for key in &type_keys {
        members_by_component
            .entry(component_of[key])
            .or_default()
            .push(key.clone());
    }
    let mut components: Vec<(Vec<DeclarationKey>, ObjectUniverse)> = members_by_component
        .into_values()
        .map(|members| {
            let mut root_types: Vec<EffectiveId> = members
                .iter()
                .filter(|key| !index.is_non_root(key))
                .map(|key| type_effective_ids[key])
                .collect();
            root_types.sort();
            let universe = ObjectUniverse {
                model_selection: domain_package.model_selection.clone(),
                root_types,
            };
            (members, universe)
        })
        .collect();
    // `value-accounting.md`:495: "each object universe preimage ascending
    // by its first root type identity". A component with no root type at
    // all (a pure cycle) arises only on a package phase 3 already refused
    // above, so `None` sorting first is never observed.
    components.sort_by_key(|(_, universe)| universe.root_types.first().copied());
    let mut universe_index_by_type: BTreeMap<DeclarationKey, usize> = BTreeMap::new();
    for (component_index, (members, _)) in components.iter().enumerate() {
        for member in members {
            universe_index_by_type.insert(member.clone(), component_index);
        }
    }
    let mut ordered = components.into_iter().map(|(_, universe)| universe);
    // A domain package with no declared object type at all still produces
    // exactly one (empty) universe.
    let first = ordered.next().unwrap_or_else(|| ObjectUniverse {
        model_selection: domain_package.model_selection.clone(),
        root_types: Vec::new(),
    });
    let universes = ObjectUniverses {
        first,
        rest: ordered.collect(),
    };
    // `value-accounting.md`:495: "each object universe preimage ascending
    // by its first root type identity, then the effective view preimage".
    for universe in universes.iter() {
        charges.hash(universe.canonical_len())?;
    }

    // Each population's bindings are admitted into its first member type's
    // universe, or the first universe when it declares no member type.
    // Intake has already refused a member type that is not a declared
    // object type (`UnknownPopulationMemberType`), so every member type
    // here has a universe.
    let populations = domain_package
        .records
        .iter()
        .filter_map(|record| match record {
            DomainPackageRecord::Population(population) => Some(population),
            _ => None,
        })
        .map(|population| {
            let universe_index = population
                .member_types
                .first()
                .map_or(0, |type_key| universe_index_by_type[type_key]);
            (
                population.key.clone(),
                PopulationEntry {
                    record: population.clone(),
                    universe_index,
                },
            )
        })
        .collect();

    let type_identities = type_keys
        .iter()
        .map(|key| (key.clone(), type_effective_ids[key]))
        .collect();
    let view = ViewBody {
        declarations: entries,
        types: Arc::new(TypeCatalog::new(type_identities)),
        universes,
        universe_index_by_type,
        populations,
        limits,
    };
    charges
        .hash(view.canonical_len_from_parts(&domain_package.model_selection, declarations_len))?;
    Ok((view, index))
}

/// The RFC 8785 length of one effective-view `declarations` element,
/// `{"effective_id":<id>,"preimage":<preimage>}`, from its identity and its
/// preimage's length: the element's fixed frame, plus the two values.
fn view_entry_len(effective_id: &EffectiveId, preimage_len: u64) -> u64 {
    const FRAME: &str = r#"{"effective_id":,"preimage":}"#;
    length_amount(FRAME.len())
        .saturating_add(canonical_len(&EffectiveIdWire::from(effective_id)))
        .saturating_add(preimage_len)
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
    path: KeyPath,
}

/// Every cross-type input and output `plan_redefinitions` needs beyond its
/// own `type_key`'s local bookkeeping, grouped into one `&mut` borrow
/// (rather than five separate parameters) so the function stays within
/// clippy's `too_many_arguments` ceiling. `type_fact_counts`/
/// `owner_ancestor_sets` are build-wide, read-only lookups (`f(o)`,
/// `value-accounting.md:456`, and each owner's own proper-ancestor set);
/// `conflict_charges`, `phase4_fact_count` and `refusal` are build-wide
/// accumulators mutated across every `type_key`'s own call.
/// `normalize.redefinition-check`'s own charge sequence — and the `m` it
/// needs — is not built here at all: `build` computes it once,
/// domain-package-wide, before any `plan_redefinitions` call.
struct Phase4Accounting<'a> {
    type_fact_counts: &'a HashMap<DeclarationKey, u64>,
    owner_ancestor_sets: &'a HashMap<DeclarationKey, HashSet<DeclarationKey>>,
    /// Every type's own effective identity, populated by `build`'s phase
    /// 2/3 loop before any `plan_redefinitions` call — QSL #195: the
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
    /// `normalize.fact` charge (`build` charges each type's after this
    /// function returns) — field redefinition only, exactly like `member_preimages`/
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
/// one `Refusals` (QSL #195; see the module docs).
fn record_phase4_refusal(
    accounting: &mut Phase4Accounting<'_>,
    rank: Phase4Rank,
    refusal: ModelRefusal,
) {
    accounting.refusals.push((rank, refusal));
}

/// Phase 4 (TC-195 N06): grows [`EffectiveView`] for field redefinition
/// only — operation-member redefinition contributes no view entry here (see
/// the module docs); `conformance` resolves each operation redefinition's
/// own conformance axes directly against its own
/// [`EffectiveDeclarationPreimage`] values instead of this pass's exposure
/// bookkeeping. This function still detects and resolves *contention* — two
/// or more redefiners of the same target — for operation members as well as
/// fields (#173), sharing [`resolve_redefinition_contest`]'s dominance
/// search; see this function's own operation-edges loop below.
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
/// charge amount (`value-accounting.md:456`) to `conflict_charges`,
/// charged later by `build` — `normalize.redefinition-check`'s own
/// charge sequence is `build`'s own domain-package-wide pass, not this function's.
/// `type_fact_counts` supplies `f(o)` for any owner in the domain package, not just
/// `type_key` itself — `build` computes it
/// only after every type's own phase 2/3 has run (see the module docs) so
/// this is always a lookup, never a fresh walk. `owner_ancestor_sets` is
/// `build`'s own build-wide map of every owner's proper-ancestor set,
/// derived from phase 3's own `type_paths` (QSL #145) rather than a second,
/// separately bounded walk, and likewise computed once, build-wide, not
/// once per (type, target) group.
fn plan_redefinitions(
    index: &RecordIndex,
    type_key: &DeclarationKey,
    paths: &[AncestorPath],
    member_preimages: &HashMap<(DeclarationKey, DeclarationKey), EffectiveDeclarationPreimage>,
    accounting: &mut Phase4Accounting<'_>,
) -> TypeRedefinitions {
    // QSL #195: every `normalize.conflict-check` charge and refusal this
    // call exposes is tagged by `type_key`'s own *effective* identity, not
    // its producer `DeclarationKey` — `build` populates `type_effective_ids`
    // for every type before any `plan_redefinitions` call (see the module
    // docs).
    let owner_effective_id = accounting
        .type_effective_ids
        .get(type_key)
        .cloned()
        .expect("build populates type_effective_ids for every type before phase 4 runs");

    let mut owner_paths: HashMap<DeclarationKey, KeyPath> = HashMap::new();
    owner_paths.insert(type_key.clone(), KeyPath::default());
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
    // (field-only) feeds this type's own field conflict *resolution and
    // view growth*, exactly as the module docs describe. `operation_edges`
    // feeds the contention *charge* below (QSL #145) and, like `all_edges`,
    // its own dominance-search *resolution* (#173) — but never
    // `member_preimages`/`hidden`/view growth: `crate::model::conformance`
    // still checks each operation redefinition's own conformance axes
    // directly (see the module docs), and a contested operation target
    // (`c >= 2`) still owes `normalize.conflict-check`'s own
    // `value-accounting.md:456` price, exactly as a contested field target
    // does.
    //
    // Read from the index's redefiners grouped by owner (QSL-202), for the
    // owners that reach `type_key` only, in the records' own order: the
    // same edges, in the same order, as a scan of every record keeping those
    // whose owner reaches `type_key`.
    let mut reaching: Vec<(&DeclarationKey, &KeyPath, Redefiner)> = owner_paths
        .iter()
        .flat_map(|(owner, path)| {
            index
                .redefiners_of_owner(owner)
                .iter()
                .map(move |redefiner| (owner, path, *redefiner))
        })
        .collect();
    reaching.sort_unstable_by_key(|(_, _, redefiner)| redefiner.record);
    let mut all_edges: Vec<RedefinitionEdge> = Vec::new();
    let mut operation_edges: Vec<RedefinitionEdge> = Vec::new();
    for (owner, path, redefiner) in reaching {
        let redefining = index.key(redefiner.member);
        let redefines = index.key(redefiner.target);
        let is_field = redefiner.is_field
            && index.is_field_member(redefining)
            && index.is_field_member(redefines);
        let is_operation = !redefiner.is_field
            && index.is_operation_member(redefining)
            && index.is_operation_member(redefines);
        if !is_field && !is_operation {
            continue;
        }
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

    let mut field_groups = Vec::with_capacity(target_keys.len());
    for target_key in target_keys {
        let mut edges = groups.remove(&target_key).expect("just listed");
        edges.sort_by(|a, b| {
            a.owner
                .cmp(&b.owner)
                .then_with(|| a.redefining.cmp(&b.redefining))
        });
        // An unreachable target has no member for a redefinition to resolve
        // *to* (H1 finding, PR #228 round 2 review): it derives no fact, and
        // `resolve_redefinitions` refuses every edge in its group.
        let reachable = member_preimages.contains_key(&(type_key.clone(), target_key.clone()));
        if edges.len() >= 2 {
            // `value-accounting.md:456`: `work_units += Σ (c − 1) × f(o)`,
            // summed over the `c` redefining owners `o` -- once per edge,
            // repeating a shared owner's own `f(o)` for every edge it owns.
            // It fires whenever `c >= 2`, reachable or not (PR #228 round 2
            // review): the package still declared `c` redefinitions of this
            // target. A single redefiner has nothing to contest and owes no
            // charge (QSL #145).
            push_conflict_charge(accounting, owner_effective_id, &target_key, &edges);
        }
        if reachable {
            // `model-complete.md:231`: one redefine fact on the redefining
            // feature and one on the redefined feature per edge, contested
            // or not, winner or not.
            *accounting.phase4_fact_count += 2 * length_amount(edges.len());
        }
        field_groups.push(TargetGroup {
            target: target_key,
            edges,
            reachable,
        });
    }

    let mut operation_groups: HashMap<DeclarationKey, Vec<RedefinitionEdge>> = HashMap::new();
    for edge in operation_edges {
        operation_groups
            .entry(edge.target.clone())
            .or_default()
            .push(edge);
    }
    let mut operation_target_keys: Vec<DeclarationKey> = operation_groups.keys().cloned().collect();
    operation_target_keys.sort();
    let mut contested_operations = Vec::new();
    for target_key in operation_target_keys {
        let edges = operation_groups.remove(&target_key).expect("just listed");
        // Operation redefinitions derive no fact and grow no view entry (see
        // the module docs); a contested target (`c >= 2`) still owes the same
        // `normalize.conflict-check` price as a field target (QSL #145, #173).
        if edges.len() < 2 {
            continue;
        }
        push_conflict_charge(accounting, owner_effective_id, &target_key, &edges);
        contested_operations.push(TargetGroup {
            target: target_key,
            edges,
            reachable: true,
        });
    }

    TypeRedefinitions {
        owner_effective_id,
        field_groups,
        contested_operations,
    }
}

/// Appends one contested target group's `normalize.conflict-check` amount,
/// `Σ (c − 1) × f(o)` (`value-accounting.md:456`), tagged with its charge
/// order key `(owner effective type identity, target)`.
fn push_conflict_charge(
    accounting: &mut Phase4Accounting<'_>,
    owner_effective_id: EffectiveId,
    target_key: &DeclarationKey,
    edges: &[RedefinitionEdge],
) {
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
        owner_effective_id,
        target_key.clone(),
        fact_total.saturating_mul(c.saturating_sub(1)),
    ));
}

/// One target group of one type: every redefining member reaching the type
/// that redefines `target`, sorted by (owner, redefining member).
struct TargetGroup {
    target: DeclarationKey,
    edges: Vec<RedefinitionEdge>,
    /// Whether `target` is one of the type's effective members. Always
    /// `true` for an operation group.
    reachable: bool,
}

/// One type's phase-4 work, planned and priced by [`plan_redefinitions`]
/// before any of it is charged, then resolved by [`resolve_redefinitions`]
/// once every phase-4 charge is admitted (QSL-216).
struct TypeRedefinitions {
    owner_effective_id: EffectiveId,
    /// Field target groups, ascending by target key.
    field_groups: Vec<TargetGroup>,
    /// Operation target groups with two or more redefiners, ascending by
    /// target key.
    contested_operations: Vec<TargetGroup>,
}

/// Resolves one type's planned phase-4 work, once every phase-4 charge is
/// admitted (QSL-216): each field group's dominance contest, its redefine
/// facts and hidden members, and each contested operation group's contest.
/// Every refusal goes to `accounting.refusals`, ranked for charge order, and
/// every remaining group is still resolved (see the module docs).
fn resolve_redefinitions(
    type_key: &DeclarationKey,
    plan: TypeRedefinitions,
    member_preimages: &mut HashMap<(DeclarationKey, DeclarationKey), EffectiveDeclarationPreimage>,
    hidden: &mut HashSet<(DeclarationKey, DeclarationKey)>,
    accounting: &mut Phase4Accounting<'_>,
) {
    let TypeRedefinitions {
        owner_effective_id,
        field_groups,
        contested_operations,
    } = plan;
    'targets: for group in field_groups {
        let TargetGroup {
            target: target_key,
            edges,
            reachable,
        } = group;
        let member_key = (type_key.clone(), target_key.clone());

        // `Option<usize>`, not a bare `usize`: an ambiguous group (no edge
        // dominates every other) has no winner at all, but still owes every
        // charge below and still derives both facts per edge
        // (`model-complete.md:231`) -- the ambiguity is reported as a
        // refusal (`accounting.refusal`, below) once phase 4's own charges
        // finish, per `value-accounting.md:481`'s "checking is exhaustive
        // within a stage" (see the module docs), not by aborting this
        // function early. `None` hides every edge and the target itself:
        // harmless, since a build that ever sets `accounting.refusal`
        // never returns `Completed` (see `build`). An unreachable
        // target (`!reachable`, above) never reaches the hiding loop below
        // at all -- it refuses and `continue`s instead.
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
            if !reachable {
                None
            } else {
                // `owner_ancestor_sets` already holds every owner in the domain package's
                // own proper-ancestor set (QSL #145: derived once, build-wide,
                // from phase 3's own `type_paths` rather than a fresh,
                // separately bounded walk here); `resolve_redefinition_contest`'s
                // own winner search and undominated-owner fallback each compare
                // every edge's owner against every other edge's owner, but
                // TC-196 R07's own documented shape — several redefining members
                // sharing one contending owner — means distinct owners are
                // frequently far fewer than edges, and the same owner recurs
                // across many types and targets within one build; both are plain
                // `O(1)` set lookups against the already-built map.
                //
                // `resolve_redefinition_contest` now returns its own
                // `Phase4Rank` bundled with its refusal (M1 finding, PR #228
                // round 2 review): the two `Err` shapes it builds -- a
                // same-owner `RedefinitionTarget` on
                // `normalize.redefinition-check` and a genuine
                // `DerivationConflict` on `normalize.conflict-check`
                // (`value-accounting.md:492`/`:493`) -- are ranked at the one
                // place that already knows which of them it built, so this
                // match has nothing left to re-derive.
                match resolve_redefinition_contest(
                    accounting.owner_ancestor_sets,
                    &owner_effective_id,
                    type_key,
                    &target_key,
                    &edges,
                ) {
                    Ok(index) => Some(index),
                    Err((rank, refusal)) => {
                        record_phase4_refusal(accounting, rank, refusal);
                        None
                    }
                }
            }
        };

        if !reachable {
            // `model-complete.md:298-300`: one `RedefinitionTarget` refusal
            // per *owner* redefining this unreachable target (H1 + L2
            // findings, PR #228 round 2 review), not one refusal spanning
            // every owner in this target group -- grouping by target alone
            // conflated distinct owners' own redefiners under the rank-only
            // dedup elsewhere in this module
            // (`phase4_refusal_candidates.dedup_by`), dropping some owners'
            // refusals outright and misattributing others' redefiners to the
            // wrong owner. Grouping by `(owner, target)` here keeps each
            // owner's own claim distinct regardless of which type names it,
            // and regardless of which other owners also redefine this same
            // target.
            //
            // `normalize.redefinition-check` exposes this refusal
            // (`value-accounting.md:455`), so `record_phase4_refusal` below
            // ranks it by the least of each owner's own redefining members'
            // own keys — `:455`'s own "ascending by the member's own key"
            // order (there is no separate redefinition-record key under
            // QSpec's inline shape). Collected into `accounting.refusals`
            // rather than returned here (see the module docs), so every
            // remaining type and target group is still resolved and every
            // later phase-4 charge amount is still computed correctly.
            //
            // This exact member can also be reached, and fail the identical
            // check for the identical reason, at more than one `type_key`
            // whenever a descendant of an owner here also inherits it (both
            // compute the same owner and redefiners, so they rank
            // identically) -- naming the owner rather than `type_key` keeps
            // the reported refusal the same regardless of which of those
            // tied candidates this pass happens to keep.
            //
            // QSL #184: the spec's own name for this refusal is
            // `invalid_model_binding`/`redefinition-target` ("target is not
            // a member inherited by its owning type"), not
            // `dangling_reference`/`RedefinitionUnreachable` -- the target
            // key does resolve to a real declaration somewhere in the
            // domain package (it is simply not one of the owner's own
            // effective members), so it is not a dangling reference.
            let mut owners: Vec<DeclarationKey> =
                edges.iter().map(|edge| edge.owner.clone()).collect();
            owners.sort();
            owners.dedup();
            for owner in owners {
                let mut redefiners: Vec<DeclarationKey> = edges
                    .iter()
                    .filter(|edge| edge.owner == owner)
                    .map(|edge| edge.redefining.clone())
                    .collect();
                redefiners.sort();
                let least = redefiners[0].clone();
                let redefiner_names: Vec<&str> =
                    redefiners.iter().map(|key| key.node.as_str()).collect();
                record_phase4_refusal(
                    accounting,
                    Phase4Rank::RedefinitionCheck(least),
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
                            owner.node
                        ),
                    },
                );
            }
            continue 'targets;
        }

        for (i, edge) in edges.iter().enumerate() {
            let inputs = FactInputs::new(
                edge.path.clone(),
                vec![edge.redefining.clone(), edge.target.clone()],
            );

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
        }
        hidden.insert(member_key);
    }

    // Operation-member redefinition contention (#173): applies the identical
    // `resolve_redefinition_contest` dominance rule field targets get above —
    // a winner if one edge's owner dominates every other, otherwise a typed
    // `redefinition-target`/`derivation-conflict` refusal — but touches no
    // `member_preimages`/`hidden` state: operation members never enter
    // `member_preimages` (see the module docs), so there is nothing here for
    // a winner to resolve or hide; a resolved group's winner needs no further
    // bookkeeping in this pass, and a contested target still owes
    // `normalize.conflict-check`'s own `Σ (c − 1) × f(o)` price
    // (`value-accounting.md:456`) whenever `c >= 2`, exactly like a
    // contested field target (QSL #145), whether or not it resolves.
    for TargetGroup {
        target: target_key,
        edges,
        ..
    } in contested_operations
    {
        if let Err((rank, refusal)) = resolve_redefinition_contest(
            accounting.owner_ancestor_sets,
            &owner_effective_id,
            type_key,
            &target_key,
            &edges,
        ) {
            // `resolve_redefinition_contest` bundles its own `Phase4Rank`
            // with its refusal (M1 finding, PR #228 round 2 review) -- see
            // the field-edges loop's identical call above -- so there is no
            // cause to re-match here.
            record_phase4_refusal(accounting, rank, refusal);
        }
    }
}

/// Whether `p_owner` strictly dominates `q_owner`: `p_owner` is a proper
/// descendant of `q_owner` in `closures` (`build`'s own `owner_ancestor_sets`,
/// derived from phase 3's own `type_paths` — QSL #145). An `O(1)` set
/// lookup against an already-computed set, never
/// a fresh graph walk — the `O(|edges|^2)` pair enumeration this function is
/// called from stays cheap because `closures` was already built once,
/// build-wide, before any `plan_redefinitions` call.
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

/// The dominance rule `plan_redefinitions` applies to a contested
/// `target_key`'s `edges` (`c >= 2`, already checked by every caller):
/// `Ok(i)` names the unique edge whose owner strictly dominates every other
/// edge's owner. `Err` when no edge does, carrying the FR-272 refusal to
/// report -- shared between field and operation redefiners (#173 applies the
/// identical dominance rule to both) since the rule itself does not depend
/// on member kind, only on the contending edges' own owners.
///
/// `Err` distinguishes TC-196 R07's two ambiguity shapes. Restricted to the
/// most-derived owners, not every edge's owner: a less-derived owner's edge
/// (e.g. `C/w` where `B <= C`) has already lost to any more-derived owner's
/// edge (`B`'s) in the winner search above exactly as it would if `B` had
/// only one redefiner, so it takes no part in deciding whether the
/// *remaining* ambiguity is "one owner, several redefiners" or "two
/// genuinely different lineages":
///
/// - `redefinition-target` when every most-derived owner among `edges`
///   (those no *other* edge's owner properly descends from) is the identical
///   owner -- several redefining members of one type contending for the same
///   inherited target (e.g. `B/z` and `B/z2` both `redefines: A/x`), naming
///   every contending redefiner and the one contended target.
/// - `derivation-conflict` when the most-derived owners are not all the same
///   owner -- a genuine diamond of distinct sibling lineages, neither
///   dominating the other, naming every contending edge's own path.
///
/// A caller cannot resolve either shape by an arbitrary pick.
///
/// Sorted here, once, by `(owner, redefining)` (M1, #204 round 1) so the
/// refusal payload built below (`DerivationConflict`'s own `redefiners`/
/// `detail`, and `RedefinitionTarget`'s `detail`) is deterministic
/// regardless of a caller's own incoming order: the field-target loop above
/// already sorts its own `edges` the identical way before calling, for its
/// *other* (`member_preimages`/`hidden`) needs downstream, so this sort is a
/// stable no-op there and `Ok(i)` still indexes that caller's own slice the
/// same way; the operation-target loop does not pre-sort, and never reads
/// `Ok(i)` (operation members have no winner bookkeeping of their own — see
/// that loop's comment), so sorting only here, once, covers both callers.
///
/// `Err`'s own [`Phase4Rank`] travels with its refusal (MEDIUM finding, PR
/// #228 round 2 review): both callers used to re-derive it from the returned
/// cause with an identical `match`, one of them behind a wildcard arm that
/// silently absorbed any cause this function does not actually return.
/// Computing the rank here, the one place that already knows which of the
/// two shapes it built, makes each caller's own match exhaustive over a
/// `(Phase4Rank, ModelRefusal)` pair with nothing left to wildcard.
fn resolve_redefinition_contest(
    owner_ancestor_sets: &HashMap<DeclarationKey, HashSet<DeclarationKey>>,
    owner_effective_id: &EffectiveId,
    type_key: &DeclarationKey,
    target_key: &DeclarationKey,
    edges: &[RedefinitionEdge],
) -> Result<usize, (Phase4Rank, ModelRefusal)> {
    let mut edges: Vec<&RedefinitionEdge> = edges.iter().collect();
    edges.sort_by(|a, b| {
        a.owner
            .cmp(&b.owner)
            .then_with(|| a.redefining.cmp(&b.redefining))
    });
    let edges = edges;
    // L2, #204 round 1: `iter().position` in place of a raw `0..len()`
    // index loop; `std::ptr::eq` stands in for the original `i == j`
    // self-skip, comparing each edge's own identity (its slot in this
    // group), not its value -- two structurally identical edges at
    // different slots must still each get their own turn as `candidate`.
    if let Some(winner) = edges.iter().position(|candidate| {
        edges.iter().all(|other| {
            std::ptr::eq(*candidate, *other)
                || owner_dominates(owner_ancestor_sets, &candidate.owner, &other.owner)
        })
    }) {
        return Ok(winner);
    }

    let mut most_derived: Vec<&RedefinitionEdge> = Vec::new();
    for edge in edges.iter().copied() {
        let dominated_by_another = edges.iter().any(|other| {
            other.owner != edge.owner
                && owner_dominates(owner_ancestor_sets, &other.owner, &edge.owner)
        });
        if !dominated_by_another {
            most_derived.push(edge);
        }
    }
    let same_owner = !most_derived.is_empty()
        && most_derived
            .iter()
            .all(|edge| edge.owner == most_derived[0].owner);

    Err(if same_owner {
        let mut redefiners: Vec<String> = most_derived
            .iter()
            .map(|edge| edge.redefining.node.clone())
            .collect();
        redefiners.sort();
        let mut redefiner_keys: Vec<DeclarationKey> = most_derived
            .iter()
            .map(|edge| edge.redefining.clone())
            .collect();
        redefiner_keys.sort();
        // `redefiner_keys.get(1)` is always `Some`, never the fallback a
        // previous round's `.unwrap_or_else(|| redefiners[0].clone())`
        // covered at both call sites (dead code, MEDIUM finding, PR #228
        // round 2 review): `same_owner` is only reachable once the winner
        // search above has already failed, and a finite poset with a unique
        // maximal element also has a unique maximum (that element would
        // dominate every other edge, so the winner search would have
        // returned `Ok` instead) -- so `most_derived`, and therefore
        // `redefiner_keys`, always has at least two elements here.
        let rank_key = redefiner_keys
            .get(1)
            .cloned()
            .expect("a same-owner ambiguity's most_derived set always has at least two elements");
        // `value-accounting.md:492`'s "ascending by redefining-member key"
        // ranks this group by one of its own redefining members' keys; the
        // second-checked one (ascending order) is used rather than the
        // least, which the unreachable-target `RedefinitionTarget` shape
        // uses for its own rank -- keeping the two `redefinition-target`
        // shapes distinguishable by rank when they tie on every other
        // component.
        let rank = Phase4Rank::RedefinitionCheck(rank_key);
        (
            rank,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::RedefinitionTarget {
                    redefiners: redefiner_keys,
                    target: target_key.clone(),
                },
                detail: format!(
                    "{} declares {} redefining members ({}) that all redefine {}, with no single valid target",
                    most_derived[0].owner.node,
                    most_derived.len(),
                    redefiners.join(", "),
                    target_key.node
                ),
            },
        )
    } else {
        let edge_paths: Vec<String> = edges
            .iter()
            .map(|edge| {
                let mut path: Vec<String> = edge
                    .path
                    .keys()
                    .iter()
                    .map(|key| key.node.clone())
                    .collect();
                path.push(edge.redefining.node.clone());
                path.push(target_key.node.clone());
                format!("[{}]", path.join(", "))
            })
            .collect();
        // `:493`'s "ascending by effective member key"
        // (`model-complete.md:206`: owner effective type identity, then
        // original declaration key) -- this group's own resolving type's
        // effective identity, genuinely distinct per resolving type (QSL
        // #195).
        let rank = Phase4Rank::ConflictCheck(*owner_effective_id, target_key.clone());
        (
            rank,
            ModelRefusal {
                code: Code::InvalidModelBinding,
                cause: ModelRefusalCause::DerivationConflict {
                    type_: type_key.clone(),
                    member: target_key.clone(),
                    redefiners: edges.iter().map(|edge| edge.redefining.clone()).collect(),
                },
                detail: format!(
                    "type {} has {} undominated redefinitions of {}: {}",
                    type_key.node,
                    edges.len(),
                    target_key.node,
                    edge_paths.join(" and ")
                ),
            },
        )
    })
}

/// Why [`build`] stopped before completing: a counted limit ran out, or a
/// stage exposed its refusals, each reported only once every charge its own
/// stage admits (see the module docs). `From<Incomplete>` lets
/// `charges.fact()?` and friends propagate a denial directly.
enum Denial {
    Incomplete(Incomplete),
    Refused(Refusals),
}

impl From<Incomplete> for Denial {
    fn from(incomplete: Incomplete) -> Self {
        Denial::Incomplete(incomplete)
    }
}

impl From<ModelRefusal> for Denial {
    fn from(refusal: ModelRefusal) -> Self {
        Denial::Refused(Refusals::new(refusal, Vec::new()))
    }
}

/// The meter one normalization charges as it works, with the running counts
/// its high-water charges size: `normalize.fact`'s `derivation_facts` and
/// `normalize.declaration`'s `effective_declarations`.
struct Charges<'m> {
    meter: &'m mut Meter,
    facts: u64,
    declarations: u64,
}

impl<'m> Charges<'m> {
    fn new(meter: &'m mut Meter) -> Self {
        Self {
            meter,
            facts: 0,
            declarations: 0,
        }
    }

    fn charge(&mut self, charge: Charge) -> Result<(), Incomplete> {
        self.meter.charge(charge)
    }

    /// One `normalize.fact`: one more derivation fact.
    fn fact(&mut self) -> Result<(), Incomplete> {
        let next = self.facts.saturating_add(1);
        self.charge(
            Charge::new(ChargePoint::NormalizeFact).size(LimitKind::DerivationFacts, next),
        )?;
        self.facts = next;
        Ok(())
    }

    /// One `normalize.cycle-check` over an ancestor path of `path_len`
    /// steps (`work_units += L`, `value-accounting.md:491`).
    fn cycle_check(&mut self, path_len: usize) -> Result<(), Incomplete> {
        self.charge(Charge::new(ChargePoint::NormalizeCycleCheck).work(length_amount(path_len)))
    }

    /// One `normalize.declaration`: one more effective declaration.
    fn declaration(&mut self) -> Result<(), Incomplete> {
        let next = self.declarations.saturating_add(1);
        self.charge(
            Charge::new(ChargePoint::NormalizeDeclaration)
                .size(LimitKind::EffectiveDeclarations, next),
        )?;
        self.declarations = next;
        Ok(())
    }

    /// One `normalize.hash` over `bytes` of RFC 8785 preimage.
    fn hash(&mut self, bytes: u64) -> Result<(), Incomplete> {
        self.charge(Charge::new(ChargePoint::NormalizeHash).size(LimitKind::HashedBytes, bytes))
    }
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
///
/// A completed view owns a copy of `domain_package`; only a completed
/// normalization makes that copy. A caller that already holds the package in
/// an [`Arc`] calls [`normalize_shared`] instead, which makes none.
pub fn normalize(
    domain_package: &DomainPackage,
    limits: ModelNormalizationLimits,
) -> NormalizeOutcome {
    let (outcome, _meter) = normalize_with_meter(domain_package, limits);
    outcome
}

/// [`normalize`] over a package the caller already shares: a completed view
/// holds `domain_package` itself, with no copy.
pub fn normalize_shared(
    domain_package: Arc<DomainPackage>,
    limits: ModelNormalizationLimits,
) -> NormalizeOutcome {
    let (body, _meter) = normalize_body(&domain_package, limits);
    into_outcome(body, || domain_package)
}

/// [`normalize`], also returning the meter that ran `domain_package` under
/// `limits`, for tests that assert the charges alongside the result.
///
/// Normalization charges the meter inside each phase as it works (QSL-216):
/// the first denied charge stops it, so the counted limits bound the work
/// done, not only what normalization admits.
pub fn normalize_with_meter(
    domain_package: &DomainPackage,
    limits: ModelNormalizationLimits,
) -> (NormalizeOutcome, Meter) {
    let (body, meter) = normalize_body(domain_package, limits);
    (
        into_outcome(body, || Arc::new(domain_package.clone())),
        meter,
    )
}

/// [`build`] under a fresh meter: the completed view's body and the record
/// index it was built through, or the denial that stopped it.
fn normalize_body(
    domain_package: &DomainPackage,
    limits: ModelNormalizationLimits,
) -> (Result<(ViewBody, RecordIndex), Denial>, Meter) {
    let mut meter = Meter::new(limits);
    let body = build(domain_package, &mut meter);
    (body, meter)
}

/// The outcome of a normalization whose completed body is paired with the
/// package `package` supplies, called only when normalization completed.
fn into_outcome(
    body: Result<(ViewBody, RecordIndex), Denial>,
    package: impl FnOnce() -> Arc<DomainPackage>,
) -> NormalizeOutcome {
    match body {
        Ok((body, records)) => NormalizeOutcome::Completed(EffectiveView {
            index: Arc::new(ModelIndex::from_parts(package(), records)),
            body,
        }),
        Err(Denial::Incomplete(incomplete)) => NormalizeOutcome::Incomplete(incomplete),
        // `Denial::Refused` already carries `Refusals` (L1
        // finding, PR #228 round 2 review): every raise site builds it
        // directly from an already-checked non-empty source, so there is
        // nothing left to convert, and nothing left to panic over, here.
        Err(Denial::Refused(refusals)) => NormalizeOutcome::Refused(refusals),
    }
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
            effective_id: EffectiveId::from_digest([byte; 32]),
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
            index: Arc::new(ModelIndex::build(DomainPackage::new(
                DomainPackageRef::fixture("test/orders"),
                Vec::new(),
            ))),
            body: ViewBody {
                declarations: vec![entry(2), entry(1)],
                types: Arc::default(),
                universes: ObjectUniverses {
                    first: ObjectUniverse {
                        model_selection: DomainPackageRef::fixture("test/orders"),
                        root_types: Vec::new(),
                    },
                    rest: Vec::new(),
                },
                universe_index_by_type: BTreeMap::new(),
                populations: BTreeMap::new(),
                limits: ModelNormalizationLimits::UNLIMITED,
            },
        };
        let refusal = view
            .validate_order()
            .expect_err("a view whose declarations are no longer ascending must be refused");
        assert_eq!(
            refusal.cause,
            ModelRefusalCause::UnsortedView {
                at: view.declarations()[1].effective_id,
            }
        );
        assert_eq!(refusal.code, Code::InvalidModelBinding);
    }

    /// QSL-204 review (PR #387, LOW-1): `normalize_shared` puts the caller's
    /// own `Arc` into the completed view instead of copying the package, and
    /// yields the same view as the borrowing `normalize`.
    #[test]
    #[trace("TC-195")]
    fn normalize_shared_keeps_the_callers_package_without_copying() {
        let domain_package = Arc::new(DomainPackage::new(
            DomainPackageRef::fixture("test/orders"),
            vec![DomainPackageRecord::ObjectType(
                crate::model::domain_package::ObjectTypeRecord {
                    key: DeclarationKey::fixture("model.A"),
                    interface_features: None,
                    abstract_type: false,
                    supertypes: Vec::new(),
                },
            )],
        ));
        let NormalizeOutcome::Completed(shared) = normalize_shared(
            Arc::clone(&domain_package),
            ModelNormalizationLimits::UNLIMITED,
        ) else {
            panic!("the one-type package normalizes");
        };
        assert!(std::ptr::eq(shared.domain_package(), &*domain_package));
        let NormalizeOutcome::Completed(borrowed) =
            normalize(&domain_package, ModelNormalizationLimits::UNLIMITED)
        else {
            panic!("the one-type package normalizes");
        };
        assert!(!std::ptr::eq(borrowed.domain_package(), &*domain_package));
        assert_eq!(shared, borrowed);
    }

    fn object_type(name: &str, supertypes: &[String]) -> DomainPackageRecord {
        DomainPackageRecord::ObjectType(crate::model::domain_package::ObjectTypeRecord {
            key: DeclarationKey::fixture(name),
            interface_features: None,
            abstract_type: false,
            supertypes: supertypes.iter().map(DeclarationKey::fixture).collect(),
        })
    }

    fn field(name: &str, owner: &str) -> DomainPackageRecord {
        DomainPackageRecord::FieldMember(crate::model::domain_package::FieldMemberRecord {
            key: DeclarationKey::fixture(name),
            owner: DeclarationKey::fixture(owner),
            value_type: crate::model::domain_package::ValueTypeRef::Package(
                DeclarationKey::fixture(owner),
            ),
            multiplicity: crate::model::domain_package::Multiplicity {
                lower: 0,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
            subsets: Vec::new(),
            redefines: None,
        })
    }

    fn package(records: Vec<DomainPackageRecord>) -> DomainPackage {
        DomainPackage::new(DomainPackageRef::fixture("test/bounded"), records)
    }

    /// `A` declaring `fields` fields, and `B` generalizing `A`, so `B`
    /// inherits every one of them.
    fn wide(fields: usize) -> DomainPackage {
        let mut records = vec![
            object_type("model.A", &[]),
            object_type("model.B", &["model.A".to_owned()]),
        ];
        records.extend((0..fields).map(|i| field(&format!("model.A.f{i:05}"), "model.A")));
        package(records)
    }

    /// `depth` levels of two types each, every type generalizing both types
    /// of the level above: the bottom type has `2^depth` ancestor paths.
    fn diamond_lattice(depth: usize) -> DomainPackage {
        let name = |level: usize, side: char| format!("model.L{level:03}{side}");
        let mut records = vec![
            object_type(&name(0, 'a'), &[]),
            object_type(&name(0, 'b'), &[]),
        ];
        for level in 1..=depth {
            let parents = [name(level - 1, 'a'), name(level - 1, 'b')];
            records.push(object_type(&name(level, 'a'), &parents));
            records.push(object_type(&name(level, 'b'), &parents));
        }
        package(records)
    }

    /// Normalizes `domain_package` under `limits` and counts the
    /// per-declaration work that normalization did on this thread.
    fn incomplete_with_steps(
        domain_package: &DomainPackage,
        limits: ModelNormalizationLimits,
    ) -> (Incomplete, u64) {
        let before = work_steps();
        let outcome = normalize(domain_package, limits);
        let steps = work_steps() - before;
        match outcome {
            NormalizeOutcome::Incomplete(incomplete) => (incomplete, steps),
            other => panic!("expected Incomplete, got {other:?}"),
        }
    }

    /// QSL-216 AC 1: a package past a normalization limit is refused after
    /// work bounded by that limit, not by the package. Hashing member
    /// identities stops at `effective_declarations`, inheriting members stops
    /// at `derivation_facts`, and walking a diamond lattice's exponentially
    /// many ancestor paths stops at `derivation_facts` too. Growing each
    /// package tenfold (the lattice by 2^8 paths) leaves the refusal's work
    /// flat.
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn a_refused_normalization_does_work_bounded_by_the_limit_not_the_package() {
        let declarations = ModelNormalizationLimits {
            effective_declarations: 6,
            ..ModelNormalizationLimits::UNLIMITED
        };
        let (small, small_steps) = incomplete_with_steps(&wide(100), declarations);
        let (large, large_steps) = incomplete_with_steps(&wide(1_000), declarations);
        assert_eq!(small.limit_kind, LimitKind::EffectiveDeclarations);
        assert_eq!(small.charge_point, ChargePoint::NormalizeDeclaration);
        assert_eq!(small, large);
        // One walk step (B to A) and the n inherited members are admitted
        // under unlimited facts; the member hashes are what the limit
        // bounds: two type declarations and four member declarations are
        // admitted, so four member identities are hashed, never n.
        assert_eq!(small_steps, 1 + 100 + 4);
        assert_eq!(large_steps, 1 + 1_000 + 4);

        // Phase 2 charges 2 + n facts. Of three more, one admits B's path to
        // A and two admit inherited members; the third inherited member is
        // denied. One walk step and three member steps, whatever n is.
        let facts = |fields: u64| ModelNormalizationLimits {
            derivation_facts: 2 + fields + 3,
            ..ModelNormalizationLimits::UNLIMITED
        };
        let (small, small_steps) = incomplete_with_steps(&wide(100), facts(100));
        let (large, large_steps) = incomplete_with_steps(&wide(1_000), facts(1_000));
        assert_eq!(small.limit_kind, LimitKind::DerivationFacts);
        assert_eq!(large.limit_kind, LimitKind::DerivationFacts);
        assert_eq!((small_steps, large_steps), (4, 4));

        let lattice = ModelNormalizationLimits {
            derivation_facts: 200,
            ..ModelNormalizationLimits::UNLIMITED
        };
        let (shallow, shallow_steps) = incomplete_with_steps(&diamond_lattice(10), lattice);
        let (deep, deep_steps) = incomplete_with_steps(&diamond_lattice(18), lattice);
        assert_eq!(shallow.limit_kind, LimitKind::DerivationFacts);
        assert_eq!(deep.limit_kind, LimitKind::DerivationFacts);
        // Every walk step admits a path or is the one denied, so the walk
        // takes at most `derivation_facts` steps whatever the lattice's
        // 2^depth path count.
        assert!(shallow_steps <= 200, "{shallow_steps} steps");
        assert!(deep_steps <= 200, "{deep_steps} steps");
    }

    /// A chain `C000 <- C001 <- ... <- C{depth}` with `fields` fields
    /// declared on the root `C000`: type `Ci` inherits every field along a
    /// path of `i` steps.
    fn deep_chain_with_wide_root(depth: usize, fields: usize) -> DomainPackage {
        let name = |level: usize| format!("model.C{level:03}");
        let mut records = vec![object_type(&name(0), &[])];
        records.extend((1..=depth).map(|level| object_type(&name(level), &[name(level - 1)])));
        records.extend((0..fields).map(|i| field(&format!("model.C000.f{i:04}"), &name(0))));
        package(records)
    }

    /// QSL-216 review (HIGH 2): a type's inherited facts share its ancestor
    /// paths rather than copying them, so memory grows with facts plus total
    /// path length, not facts times path length. At the default limits, a
    /// 120-deep chain whose root declares 20 fields normalizes, and every
    /// inherited member fact of every chain type holds the very path its
    /// type's own inherit fact holds. (A 200-deep chain over 2,000 root
    /// fields refuses on `hashed_bytes` at the defaults, peaking at about
    /// 650 MB where copied paths reached about 6 GB; that run takes about a
    /// minute in a debug build, so it is measured, not run here.)
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn inherited_facts_share_their_ancestor_path_at_the_defaults() {
        let depth = 120;
        let fields = 20;
        let NormalizeOutcome::Completed(view) = normalize(
            &deep_chain_with_wide_root(depth, fields),
            ModelNormalizationLimits::default(),
        ) else {
            panic!("a 120-deep chain over 20 root fields normalizes at the defaults");
        };
        let root = DeclarationKey::fixture("model.C000");
        let type_paths: HashMap<EffectiveId, &FactInputs> = view
            .declarations()
            .iter()
            .filter(|entry| entry.preimage.owner_effective_type.is_none())
            .filter_map(|entry| {
                let to_root = entry.preimage.derivation.iter().find(|fact| {
                    fact.rule == RULE_INHERIT && fact.inputs.iter().last() == Some(&root)
                })?;
                Some((entry.effective_id, &to_root.inputs))
            })
            .collect();
        assert_eq!(type_paths.len(), depth);
        let mut shared = 0;
        for entry in view.declarations() {
            let Some(owner) = entry.preimage.owner_effective_type else {
                continue;
            };
            for fact in entry
                .preimage
                .derivation
                .iter()
                .filter(|fact| fact.rule == RULE_INHERIT)
            {
                assert!(fact.inputs.shares_path_with(type_paths[&owner]));
                shared += 1;
            }
        }
        assert_eq!(shared, depth * fields);
    }

    /// QSL-216 review (LOW 5): `ancestor_steps` is read, not charged, so its
    /// refusal and a counted limit are ordered by charge order. Every
    /// `normalize.record` charge precedes the ancestor walk: one record short
    /// of a 10-deep chain's 11 records is incomplete on
    /// `declaration_records`, even though the walk would pass
    /// `ancestor_steps` 5. With records unlimited, the walk reaches that
    /// ceiling and refuses `AncestorSteps`.
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn a_counted_limit_denied_first_wins_over_ancestor_steps() {
        let chain = deep_chain_with_wide_root(10, 0);
        let short = ModelNormalizationLimits {
            declaration_records: 10,
            ancestor_steps: 5,
            ..ModelNormalizationLimits::UNLIMITED
        };
        match normalize(&chain, short) {
            NormalizeOutcome::Incomplete(incomplete) => {
                assert_eq!(incomplete.limit_kind, LimitKind::DeclarationRecords);
                assert_eq!(incomplete.charge_point, ChargePoint::NormalizeRecord);
                assert_eq!((incomplete.limit, incomplete.next_charge), (10, 11));
            }
            other => panic!("expected Incomplete(DeclarationRecords), got {other:?}"),
        }
        let records_unlimited = ModelNormalizationLimits {
            declaration_records: u64::MAX,
            ..short
        };
        match normalize(&chain, records_unlimited) {
            NormalizeOutcome::Refused(refusals) => {
                assert_eq!(refusals.len(), 1);
                assert!(matches!(
                    refusals[0].cause,
                    ModelRefusalCause::AncestorSteps { limit: 5, .. }
                ));
            }
            other => panic!("expected Refused(AncestorSteps), got {other:?}"),
        }
    }

    /// QSL-222: the default limits are NFR-012's finite ceilings, and a
    /// completed view records the limits it was normalized under: the
    /// defaults, or a caller's limits as given.
    #[trace("TC-434", "NFR-012")]
    #[test]
    fn the_default_limits_are_finite_and_recorded_with_the_view() {
        assert_eq!(
            ModelNormalizationLimits::default(),
            ModelNormalizationLimits {
                declaration_records: 100_000,
                derivation_facts: 1_600_000,
                effective_declarations: 1_600_000,
                dispatch_candidates: 1_600_000,
                hashed_bytes: 268_435_456,
                work_units: 16_777_216,
                ancestor_steps: 100_000,
                family_steps: 100_000,
            }
        );
        let NormalizeOutcome::Completed(view) =
            normalize(&wide(3), ModelNormalizationLimits::default())
        else {
            panic!("a small package normalizes at the defaults");
        };
        assert_eq!(
            view.effective_limits(),
            &ModelNormalizationLimits::default()
        );
        let raised = ModelNormalizationLimits {
            work_units: u64::MAX,
            ..ModelNormalizationLimits::default()
        };
        let NormalizeOutcome::Completed(view) = normalize(&wide(3), raised) else {
            panic!("a small package normalizes under raised limits");
        };
        assert_eq!(view.effective_limits(), &raised);
    }
}
