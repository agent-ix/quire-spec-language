---
id: FR-068
title: "Split value::expression: checking moves to layer-3 check, evaluation stays at layer 5"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-009
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-067
    type: traces_to
---
# FR-068: Split value::expression: checking moves to layer-3 check, evaluation stays at layer 5

## Description

ADR-011 §7.3 M-5 requires QSL to split `value::expression`: the checking half
moves to a new top-level layer-3 `check` module (S3), and the evaluation half
stays in layer-5 `value::expression` (S6a). ADR-011 §6.2's module table names
the move exactly: `value::expression::check`, `facts`, `termination` and `ir`
move to `check` (`ir` is the checked expression output); the "check causes"
content of `value::expression::refusal` moves to `check`, while `InputRefusal`
(defined in `value::expression::mod.rs`, not in `refusal.rs`) stays at layer 5,
relocated beside `CheckedPackage`'s argument-admission code; and
`value::expression::evaluate` and `CheckedPackage::call` stay at layer 5,
unchanged in stage. The split is the whole point of this requirement: both
halves SHALL NOT end up in both places, and the requirement's own acceptance
criteria are written to fail on a copy that leaves the original in place, not
only on the original's absence.

**Order (owner ruling on QSL-139, 2026-09-20): X-1 → M-3a → M-5 (this
requirement) → M-2 (QSL-7).** M-5 comes before M-2, not after; the ADR row's
"after M-2 and M-3" is withdrawn by the ruling and is corrected in ADR-011 by
PR #271, which this requirement does not duplicate or race. Both prerequisites
— X-1 (the `quire-exact` extraction) and M-3a (QSL-138, the S2 `forms` core)
— have landed: `src/forms/` exists and `LoweredSourceGraph` is absent from
`src/` repo-wide. This requirement does not depend on `semantic_value`, M-2's
own module, in the *outbound* direction: verified at import level, file by
file, over `refusal.rs`, `ir.rs`, `termination.rs` and `facts.rs` — every
`use` resolves to a K-designated sibling, an F module, `quire_exact`, `std`,
or intra-`expression`. `check.rs` additionally imports nine further
K-designated `value::` siblings (`collection`, `comparison`, `composite`,
`decimal`, `equality`, `ieee`, `node`, `numeric`, `rational`) that X-1 has not
yet physically relocated into `quire-exact`, and `check.rs:15,20`
(`super::super::enumeration::{EnumDeclaration, EnumValue}` and
`super::super::quantity::{check_comparable, result_unit, UnitOperation}`) is
the sole non-K exception; none of these is a blocker (see Behavior, "The
layer-3 sibling imports `check.rs` keeps").

**Amendment (PR #282 review, findings F3/F4): `family.rs` is part of this
requirement's move surface, and tier 2 widens to seven items across three
modules.** The verification above covered `check.rs`, `facts.rs`, `ir.rs`
and `termination.rs` only. `check.rs`'s own `Typer::call`, however, already
called `family.rs`'s `mint_call_identity`/`DEFAULT_PACKAGE_IDENTITY`
directly before this requirement, and `family.rs`'s identity-minting content
(`encode_value_type`'s exhaustive match over `ValueType`) additionally
imports `QuantityUnit` from `value::quantity` and `TextProfile` from
`value::text` — real, pre-existing dependencies an implementation confirmed
while meeting this requirement's original five-item tier 2 and ten-module
forbidden list, neither of which named these two. Leaving `family.rs`
unnamed would either strand it in `value::expression` (reopening the exact
`check` → `value::expression` reverse edge FR-068-AC-3 forbids, since
`check.rs` depends on it) or force a silent, undocumented tier-2 violation.
This amendment adds `family.rs`'s identity-minting half (`mint_call_identity`,
`mint_declaration_identity`, `Preimage` and its `encode_*` functions,
`OccurrenceMap`, and `ValueFunctionFamily`'s `FamilyContract::check` half) to
Outputs' move surface, alongside the four files already named, and widens
tier 2 to seven items across three modules (see Behavior, "The layer-3
sibling imports `check.rs` keeps," and Acceptance Criteria, FR-068-AC-6).

**Scope clarification (owner ruling, PR #282 review, post-rebase): tier 2
bounds `check`'s shipped dependency graph, not every import in its source
text, and does not widen for `TextType`.** `family.rs`'s own pre-existing
golden-digest identity test (`mint_declaration_identity_matches_a_checked_in_digest`),
moved to `check` verbatim by the amendment above, constructs its fixture
over `ValueType::Text(TextType::new(...))` inside its own `#[cfg(test)] mod
tests` block — a real, pre-existing dependency on `TextType` from
`value::text`, but a `#[cfg(test)]`-only one. A first pass widened tier 2 to
admit it and was reverted: FR-068-AC-6's bound exists to constrain the
layering of the *shipped* crate, and a test-only import is not part of that
graph; admitting it into the allow-list to make one test pass would have
permanently licensed production code to import it too, with this
criterion's own verification method unable to ever catch that widening back
— the same shape as an unjustified raised size limit, not a genuine bound.
Tier 2 stays seven items across three modules; FR-068-AC-6's verification
excludes `#[cfg(test)]`-gated imports from its scan on this stated basis
(see Behavior, "The layer-3 sibling imports `check.rs` keeps," and
Acceptance Criteria, FR-068-AC-6).

**The *inbound* direction was not checked by the ruling above, and it matters
just as much: `model` imports symbols this requirement relocates.** The
ruling verified only what the moved code imports, not who imports the moved
code. `model::checked_dispatch.rs` and `model::conformance.rs` today reach
`DispatchCandidate`, `DispatchOperation`, `DispatchTable`, `PackageDeclarations`
(`checked_dispatch.rs:109-112`) and `established_field_fact`, `Connective`,
`Established`, `Location`, `Node`, `NodeKind`, `OrderedKind`, `Origin`,
`ProvedInterval` (`conformance.rs:89-92`) — every one of these thirteen names
is relocated to `check` by this requirement. Before the move this dependency
sat inside `value::expression`, a module not yet cleanly assigned to one
layer; after the move it becomes a legible, checkable `model` → `check`
reverse edge, forbidden by §6.1's intra-layer-3 order (`semantic_value <
model < library < check core`) and exactly the edge M-2 (QSL-7) exists to
close by moving `model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` into `check` itself.
Until M-2 lands, this requirement declares the edge explicitly rather than
letting the move create it silently (see Behavior, "The interim `model` →
`check` edge").

**This requirement does not amend ADR-011.** PR #271 is already amending the
same rows; this requirement cites the ADR and the ruling as written.
Separately, §4's own text is inconsistent with where this requirement places
`CheckedPackage`/`CheckedExpression` (see Behavior, "`CheckedPackage`,
`CheckedExpression` and `CheckedFunction` move with their checking methods");
that inconsistency is recorded, not resolved, here.

## Inputs

- `value::expression`'s current eight submodules (`check`, `evaluate`,
  `facts`, `family`, `ir`, `mod` itself, `refusal`, `termination`), as they
  exist after M-3a: `syntax` has already moved to `forms`, and the eight
  remaining submodules are this requirement's entire input surface.
  **Amendment (PR #282 review, F4): `family.rs` was omitted from this list in
  the original text (seven submodules, `family.rs` unnamed) despite
  `check.rs`'s pre-existing dependency on it; see Description's amendment
  paragraph above.**
- ADR-011 §4's stated *mechanism* for an S3/S4 output type: "The constructors
  of S3 and S4 output types are private to their stage modules, so no other
  module can build a checked value," worked out for the S4 type in `package`:
  `package` keeps its fields and constructor private and exposes read-only
  accessors; `value::expression` names the type through one `pub use` and
  implements `call` over those accessors, so
  `value::expression::CheckedPackage::call` is the contract, not a second
  type. §4 decides this mechanism only, not which layer owns `CheckedPackage`:
  §4's own text names `package` (layer 4) as the S4 type's owner, while this
  requirement places `CheckedPackage`/`CheckedExpression`/`CheckedFunction` in
  `check` (layer 3) instead. This requirement applies §4's mechanism (private
  constructor, public accessors, one `pub use`) to these three types without
  treating §4 as having already decided their owning layer — see Behavior,
  "`CheckedPackage`, `CheckedExpression` and `CheckedFunction` move with their
  checking methods," for why layer 3 is this requirement's placement and why
  that makes §4's own text inconsistent with §6.1.
- Every current consumer of a type this requirement relocates:
  `value::mod.rs`'s aggregate `pub use expression::{...}` list (including its
  separate `pub(crate) use expression::{established_field_fact, Connective,
  Established, Node, NodeKind, OrderedKind, ...}` block at `value/mod.rs:131-132`);
  `value::outcome.rs`'s `use super::expression::WrongSnapshotCause;` (see
  Behavior, "The K-designated `outcome.rs` edge"); and, found only on
  independent re-verification of the ruling's outbound-only check,
  `model::checked_dispatch.rs`'s and `model::conformance.rs`'s direct imports
  of thirteen relocated names between them (see Behavior, "The interim
  `model` → `check` edge").

## Outputs

- A new top-level `check` module (`src/check/`, declared in `src/lib.rs`
  beside `forms`, `model` and `package`) at the layer-3 position ADR-011
  §6.1 assigns it, holding: `check.rs`, `facts.rs`, `ir.rs`, `termination.rs`
  (each moved as a whole module, unchanged in file boundary); `family.rs`'s
  checking-only half (**amendment, PR #282 review F4**: `mint_call_identity`,
  `mint_declaration_identity`, `Preimage` and its `encode_*` functions,
  `DEFAULT_PACKAGE_IDENTITY`, `SCALAR_LIMITS_UNLIMITED`, `OccurrenceMap`, and
  `ValueFunctionFamily`'s `crate::family::FamilyContract` implementation) —
  `family.rs` splits along the same checking/evaluation boundary as
  `value::expression` itself, since `check.rs`'s `Typer::call` already
  depended on this content and `value::expression::family` keeps
  `QualifiedName`, S4 linking, the v2 emit/decode codec, and
  `ValueFunctionFamily`'s `crate::family::ReferenceEvaluation` implementation
  (the evaluation half) at layer 5; the check-cause
  content of `refusal.rs` (`CheckCause`, `CheckRefusal`, `Obligation`,
  `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`,
  `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`,
  `ProvedInterval`, `WrongSnapshotCause`), and the checking behavior and
  checked-output types currently in `value::expression::mod.rs`:
  `PackageDeclarations::check`, `CheckedPackage::check_expression`,
  `CheckedPackage::check_postcondition_expression`,
  `CheckedPackage::check_clause_expression`, the `CheckedPackage`,
  `CheckedExpression` and `CheckedFunction` type definitions themselves, with
  their constructors private to `check` (ADR-011 §4), and `CheckMode`
  (`value/expression/mod.rs:48`), moving to `check` alongside them: `CheckMode`
  is used only as a parameter type of the three `check_*_expression` methods
  this requirement relocates (`value/expression/mod.rs:461,498,522,542`, all
  inside those three methods' bodies or signatures) and nowhere else in the
  crate, so it has no remaining reason to stay in `value::expression` once
  its only callers move; leaving it behind would force `check` to import it
  back from `value::expression`, the reverse edge FR-068-AC-3 forbids.
- `value::expression` (layer 5) retaining exactly: `evaluate.rs` unchanged;
  `InputRefusal` (moved from its current location in `mod.rs` to sit beside
  `CheckedPackage::call`'s admission code, still in `value::expression`);
  `CheckedPackage::call`, `CheckedPackage::evaluate` and the argument-admission
  logic (today's `CheckedPackage::validate`), rewritten to operate over
  `check`'s public accessors for `CheckedPackage`/`CheckedExpression` rather
  than over private fields; and exactly one closed-list re-export —
  `pub use crate::check::{CheckedPackage, CheckedExpression};` (no third
  name, and never a glob `pub use crate::check::*;`) — per ADR-011 §4's
  stated mechanism, so
  `value::expression::CheckedPackage::call` remains the S6a entry the M-5 row
  names, with `CheckedPackage` itself defined exactly once, in `check`, and
  the crate's own import paths still show which two layers own what (see
  Acceptance Criteria, FR-068-AC-10).
- Zero definitions of any moved item remaining under `value::expression`
  after the move: no duplicate, no stub, no re-export whose target is a
  second definition rather than the one in `check`.
- `value::mod.rs`'s aggregate re-export continuing to resolve
  `crate::value::{CheckCause, CheckRefusal, Obligation, ...}` by naming
  `crate::check` instead of `crate::expression`'s (now-removed) `refusal`/
  `check` submodules — the same "value's own aggregation path continues, only
  its source module changes" pattern FR-067-AC-9 already establishes for the
  `forms` move, not a second definition and not a compatibility shim (see
  Constraints, FR-068-CON-4). This continuation covers `value::mod.rs`'s own
  re-export surface only. It does NOT cover `model::checked_dispatch.rs` or
  `model::conformance.rs`: those two files SHALL import their thirteen
  relocated names directly from `crate::check`, not through `crate::value`'s
  aggregator, so the interim `model` → `check` edge stays visible in a
  textual scan of `model`'s own `use` lines rather than hiding behind
  `value`'s indirection (see Behavior, "The interim `model` → `check` edge,"
  and Constraints, FR-068-CON-5).

## Behavior

### Checking moves to `check`; evaluation stays in `value::expression`

After this requirement's implementation, `check.rs`, `facts.rs`, `ir.rs` and
`termination.rs` SHALL each be defined exactly once, under the new top-level
`check` module, and SHALL NOT remain defined, in whole or in part, under
`value::expression`. `evaluate.rs` SHALL remain defined exactly once, under
`value::expression`, unchanged in stage. Every checking-only method currently
on `PackageDeclarations` and `CheckedPackage` in `value::expression::mod.rs`
(`PackageDeclarations::check`, `CheckedPackage::check_expression`,
`CheckedPackage::check_postcondition_expression`,
`CheckedPackage::check_clause_expression`) SHALL be defined exactly once,
under `check`, and SHALL NOT remain defined under `value::expression`.

### `check` is a new module, distinct from the existing `checking` module

`src/lib.rs` already declares `pub mod checking;`: the SEAM-1/SEAM-2 native
and composed checking module ADR-011 §6.2's seam table retires per lane under
M-6, untouched by this requirement. `check` (no `-ing`) is a different,
newly-created module this requirement adds. This requirement SHALL NOT alter,
merge into, or relocate any content into or out of the existing `checking`
module; the two names are easy to conflate, and an implementation that
writes the S3 checking content into `checking` instead of a newly-declared
`check`, or that merges the two, does not satisfy this requirement.

### `CheckedPackage`, `CheckedExpression` and `CheckedFunction` move with their checking methods — and this contradicts §4's text

ADR-011 §4 decides a *mechanism* for an S3 or S4 output type: its
constructor is private to its stage module, and a consuming layer reaches it
only through public accessors, naming the type through one `pub use`. §4's
own worked example, though, names layer-4 `package` as the *owner* of the S4
type, with `value::expression::CheckedPackage::call` merely naming it
through a `pub use` — i.e. §4's text places `CheckedPackage` in `package`,
not in `check`. This requirement places `CheckedPackage`, `CheckedExpression`
and the crate-internal `CheckedFunction` in `check` (layer 3) instead, and
that is a real disagreement with §4's text, not this requirement's
paraphrase of something §4 already settled.

The placement in `check` is deliberate: `PackageDeclarations::check` (in
`check`) returns `CheckedPackage`, so if the type lived in layer-5
`value::expression` instead, `check`'s own checking entry point would import
its return type from a higher layer — a `check` → `value::expression`
reverse edge forbidden by §6.1. If the type lived in layer-4 `package` as
§4's text names, `check` → `package` would be exactly the same class of
reverse edge (§6.1 orders `package` above `check`). Layer 3 `check` is the
only placement of the three that creates neither reverse edge, which is why
this requirement places it there. **§4's text is accordingly inconsistent
with §6.1 for this specific type, and this requirement resolves the
inconsistency by following §6.1's layer ordering over §4's worked example,
recording the contradiction rather than silently picking a side.** This
requirement does not amend ADR-011 §4 to say so (see Description); the
contradiction is stated here so a later reader does not get two different
owning layers from §4 and from this requirement with no note that anyone
noticed the conflict. **QSL-167 already tracks this as one of two recorded
ADR-011 defects (§4 versus §6.1 on `CheckedPackage`'s owning layer is the
second); this requirement cites QSL-167 as that defect's tracking ticket
rather than opening a new one or re-deciding the question here.**

`CheckedPackage`, `CheckedExpression` and the crate-internal `CheckedFunction`
SHALL be defined in `check`, with their constructors private to `check`.
`value::expression` SHALL construct no value of any of these three types
directly; it SHALL reach `CheckedPackage`'s and `CheckedExpression`'s fields
only through public accessors `check` exposes, and SHALL implement
`CheckedPackage::call`, `CheckedPackage::evaluate` and the argument-admission
logic over those accessors. This requirement does not itemize the accessor
surface `check` exposes beyond what evaluation actually calls (name, body,
slots, parameters, scope, dispatch tables): ADR-011 §4 leaves `package`'s own
accessor list equally unitemized, and this requirement follows the same
precedent rather than inventing a stricter one.

### The refusal split: check causes move; `InputRefusal` stays

`value::expression::refusal` today holds only check-cause content
(`CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`,
`CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`,
`InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`,
`WrongSnapshotCause`); `InputRefusal` is defined in `mod.rs`, not in
`refusal.rs`. ADR-011 §6.2's module-table row for `value::expression::refusal`
("split ... check causes move to check; `InputRefusal` moves to argument
admission in `CheckedPackage::call`, before S6a") describes the *concern*
split, not a split inside one file: every name `refusal.rs` defines today
SHALL move to `check`, unchanged in shape, and `InputRefusal` SHALL stay
defined in `value::expression`, relocated to sit beside the argument-admission
code (`CheckedPackage::call`'s and `CheckedPackage::evaluate`'s admission
step, today's `validate`) rather than at the top of `mod.rs`. After this
requirement's implementation, `check` SHALL define every check-cause type
and SHALL NOT define `InputRefusal`; `value::expression` SHALL define
`InputRefusal` and SHALL NOT define any check-cause type.

### The layer-3 sibling imports `check.rs` keeps: three tiers, not one

`check.rs` imports from ten `value::` siblings in total, not only
`enumeration`/`quantity`. This requirement sorts them into three tiers, and
only the second is the "declared interim edge" earlier drafts of this
requirement named alone:

1. **K-designated siblings, permitted because X-1 has not relocated them.**
   `collection` (`CardinalityBound`, `CollectionType`), `comparison`
   (`IllTypedCause`), `composite` (`CompositeShape`, `TypeEnvironment`,
   `Value`, `ValueType`), `decimal` (`DecimalType`), `equality`
   (`admits_equality_conversion`, `EqualityOperand`, `EqualityOperator`),
   `ieee` (`AdmittedIeeeProfile`), `node` (`NodeKey`), `numeric`
   (`ArithmeticOperator`, `OrderingOperator`) and `rational` (`Rational`) are
   all K-designated per §6.2's kernel-submodule row (bound for
   `quire-exact`), but X-1's own Compatibility column states it "leaves the
   QSL `value` kernel copies in place" — these modules still physically live
   under `value::` today, not in `quire-exact`. An import from `check` into
   one of these nine is therefore a `check` → K-designated-but-not-yet-moved
   edge, not a `check` → `semantic_value`/`model`/`library` edge; it is
   permitted for the same reason `check.rs`'s existing `quire_exact::{
   CollectionKind, Integer}` import is permitted (K is always available to
   layer 3), and closing it is X-1's remaining scope, not this requirement's.
2. **The declared non-K interim edge: `enumeration`/`quantity`/`text`.**
   `check.rs` imports `EnumDeclaration`, `EnumValue` from `value::enumeration`
   and `check_comparable`, `result_unit`, `UnitOperation` from
   `value::quantity` (today `super::super::enumeration`/
   `super::super::quantity`, relative to `value::expression`'s nesting; after
   the move, `crate::value::enumeration`/`crate::value::quantity`, since
   `check` is no longer nested under `value`). **Amendment (PR #282 review,
   F3): `family.rs`'s identity-minting content (moved to `check` by F4's
   amendment above) additionally needs `QuantityUnit` from `value::quantity`
   and `TextProfile` from `value::text` for `encode_value_type`'s exhaustive
   match over `ValueType::Quantity`/`ValueType::Text` — both real,
   pre-existing dependencies (already present in the pre-move, unsplit
   `family.rs`) that a conforming implementation confirmed while meeting this
   tier's original five-item, two-module bound. Tier 2 is corrected to seven
   items across three modules: `EnumDeclaration`, `EnumValue` from
   `value::enumeration`; `check_comparable`, `result_unit`, `UnitOperation`,
   `QuantityUnit` from `value::quantity`; `TextProfile` from `value::text`.**
   **Scope clarification (owner ruling, PR #282 review, post-rebase): this
   tier bounds `check`'s shipped dependency graph, and `TextType` stays out
   of it by design.** `family.rs`'s own moved golden-digest identity test
   (`mint_declaration_identity_matches_a_checked_in_digest`) builds its
   fixture, inside its own `#[cfg(test)] mod tests` block, over
   `ValueType::Text(TextType::new(...))` — a real, pre-existing dependency on
   `TextType` from `value::text`, but a `#[cfg(test)]`-only one. Widening
   tier 2 to admit it was tried and reverted: this tier constrains `check`'s
   dependency on `value` as a property of the *shipped* crate, and a
   test-only import is not part of that graph; admitting it here would have
   permanently licensed production code to import it too, with no remaining
   way for this criterion's own verification to catch that widening back.
   Tier 2 therefore stays exactly the seven items above, and FR-068-AC-6's
   verification is scoped to exclude `#[cfg(test)]`-gated imports on this
   stated basis.
   M-2 (QSL-7), which relabels `enumeration`, `quantity`, `text` and their
   siblings as layer-3 `semantic_value`, has not landed and is out of this
   requirement's scope (Order: M-5 before M-2). Owner ruling on QSL-139,
   2026-09-20: this tier is not a blocker, because ADR-011 §6.1's
   intra-layer-3 order is `semantic_value < model < library < check core <
   family checkers`, so an edge from `check` to modules bound for
   `semantic_value` runs later-depends-on-earlier, which the allow-list
   permits; it is a declarable interim shape naming QSL-165, not the
   forbidden reverse edge QSL-165 itself exists to close.
3. **Everything else is forbidden.** `definition`, `unit`, `key`, `reference`
   and `containment` — every other `semantic_value`-bound sibling per §6.2's
   module table besides `enumeration`/`quantity`/`text` — are NOT part of
   tier 2: M-2 has not yet relabeled them, and this requirement does not
   extend tier 2's allowance to them by analogy. `model_query`,
   `package_identity` and `package` (the layer-3/4 modules those
   three files feed) are likewise forbidden — eight forbidden modules in
   total. **Amendment (PR #282 review, F3): the original text's forbidden
   list, both here and in FR-068-AC-6, named exactly ten modules and did
   not include `text`, while the tier structure's own "exactly two tiers
   and no third" framing implied `text` should have been tier-3-forbidden
   by elimination — a real disagreement between the rule and its own
   enumerated fail condition. `text` is now explicitly part of tier 2
   (point 2 above), resolving the disagreement by inclusion rather than by
   adding `text` to this forbidden list, which would have contradicted
   permitting it.** **Amended by FR-074 (ADR-011 §7.3 M-2, QSL-7,
   2026-09-21) and by FR-087 (owner ruling on QSL-158, 2026-09-21): `model`
   and `library` are removed from this forbidden list**, for the same
   §6.1 layer-order reason FR-068-AC-6 records (both sit before `check`
   core; FR-087 additionally gives `check` a closed tier (c) import of
   exactly `ImportView` and `LibraryLock`, read-only, from `library`). The
   forbidden list was ten modules, is now eight. An implementation that
   widens `check`'s imports into any of the eight forbidden modules — for
   example by importing a `value::` prelude, reaching into
   `value::definition` because it looks similar to `value::enumeration`,
   or reaching into `library::VerifiedPackage`/`PackageNodeKey` — exceeds
   this requirement's scope.

This requirement SHALL preserve exactly tier 1 and tier 2, with every import
path updated to its crate-absolute, submodule-qualified form (**amendment,
PR #282 review F2**: `crate::value::<submodule>::Name`, not the flat
`crate::value::Name` aggregate FR-068-CON-4 permits every *other*
`value`-re-exporting consumer to use — a flat import carries no tier
information, so it would leave AC-6's own stated verification method (“a
source scan of every `use` statement under `src/check/` against this
two-tier allow-list”) unable to read the tier off the import text at all;
`value`'s affected submodules (`collection`, `comparison`, `composite`,
`decimal`, `enumeration`, `equality`, `ieee`, `node`, `numeric`, `quantity`,
`rational`, `text`) are `pub(crate) mod`, not private, so `check` — a
top-level sibling of `value`, not a descendant — can name them this way),
and SHALL introduce no tier-3 edge.

### Out of scope: M-2's items stay in `model`, unmoved (RETIRED — closed by FR-074)

**Retired by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** This section
described *this requirement's own* scope boundary while M-2 was still future
work: `model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` are ADR-011 §6.2's
M-2 items, and this requirement (FR-068/M-5) SHALL NOT move, copy, or
duplicate either one into `check` ahead of M-2. That boundary held for the
duration of this requirement's own implementation and is not amended
retroactively — FR-068 itself did not move either item. FR-074 is the
requirement that moves them, in the corrected order (M-5 before M-2) this
section already described; after FR-074's implementation, both items are
defined in `check`, not `model`, and `model` itself sits below `check` in
ADR-011 §6.1's layer order. This section's assertion that both "SHALL remain
defined in `model`, unchanged" is therefore false by design from FR-074
onward and is retired, not silently left standing: see FR-074 for the
requirement that supersedes it, and FR-068-AC-7 (retired the same way,
immediately below) for the corresponding acceptance criterion.

### The interim `model` → `check` edge (`checked_dispatch.rs`, `conformance.rs`)

The owner ruling on QSL-139 verified only the *outbound* direction — what
the moved code imports — and confirmed no dependency on `semantic_value`.
It did not check the *inbound* direction: who imports the code this
requirement moves. Two files in `model` do, today, through `crate::value`:

- `model/checked_dispatch.rs:109-112` imports `DispatchCandidate`,
  `DispatchOperation`, `DispatchTable` and `PackageDeclarations` — all four
  relocated to `check` by this requirement.
- `model/conformance.rs:89-92` imports `established_field_fact`,
  `Connective`, `Established`, `Location`, `Node`, `NodeKind`,
  `OrderedKind`, `Origin` and `ProvedInterval` — all nine relocated to
  `check` by this requirement.

Before this move, this dependency existed inside `value::expression`, a
module not yet cleanly assigned to one §6.1 layer, so it was not a legible,
checkable layer violation. After this move, with `check` cleanly at layer 3,
the identical dependency becomes a `model` → `check` edge, and §6.1's
intra-layer-3 order (`semantic_value < model < library < check core`) makes
it a reverse edge: later-depends-on-earlier runs the wrong way here, unlike
tier 2's `check` → `enumeration`/`quantity` edge. This requirement's own
move is what makes the edge concrete and testable, even though the
underlying symbol reliance predates it. M-2 (QSL-7) closes it: once
`model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` move into `check`
itself, the dependency becomes intra-`check`, not a cross-layer edge at all.

Until M-2 lands, this requirement declares the edge explicitly rather than
letting the move create it unnoticed: `model/checked_dispatch.rs` and
`model/conformance.rs` SHALL import exactly the thirteen names above,
directly from `crate::check` (not through `crate::value`'s aggregate
re-export — see Outputs and FR-068-CON-5, since routing them through
`value`'s continued aggregation, the pattern FR-068-CON-4 otherwise permits,
would make this specific edge invisible to a textual scan of `model`'s own
`use` lines while it stayed real at resolution). No other symbol either
file imports is affected: `checked_dispatch.rs`'s `BinaryOperator`,
`DeclaredClauseKind`, `Expression`, `FieldInitializer`, `FunctionDeclaration`,
`NodeKey` and `ValueType`, and `conformance.rs`'s `OrderingOperator`,
`Value` and `ValueType`, are untouched by this move and SHALL remain
imported exactly as they are today. No third file under `model` SHALL gain a
new `check` import as part of this requirement: the resolved import graph
(not a textual scan alone, since `crate::value`'s continued aggregation
could otherwise mask a new edge the same way it would have masked this one)
SHALL show `model` → `check` bounded to exactly these two files and these
exact thirteen names.

### The K-designated `outcome.rs` edge: path update only, not a fix

`src/value/outcome.rs` — a `value` kernel submodule the module table
assigns to K (`quire-exact`) — imports `WrongSnapshotCause` from
`super::expression` today. ADR-011 §6.1 names this exact edge
(`value/outcome.rs:10,13`) among those "X-1 must cut" so that K stays a leaf;
X-1 is reported landed, yet this one edge is still present on disk (see
Open Questions). This requirement relocates `WrongSnapshotCause`'s defining
module from `value::expression` to `check`; `value::outcome.rs`'s import
SHALL be updated to name the new module (`crate::check::WrongSnapshotCause`)
so the crate keeps compiling. This requirement SHALL NOT otherwise change
`value::outcome.rs`, and SHALL NOT attempt to cut the K→3 direction of that
edge: removing the edge's direction, as distinct from updating its path, is
ADR-011 §6.1's X-1 obligation and QSL-131's tracked scope, not this
requirement's.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-068-CON-1 | This requirement's implementation moves `check.rs`, `facts.rs`, `ir.rs`, `termination.rs`, `refusal.rs`'s check-cause content, and `value::expression::mod.rs`'s checking methods and checked-output types into a new `check` module in the same change that removes them from `value::expression`; no change under this requirement leaves a moved item defined in both modules at once, and no change leaves `value::expression` still declaring `mod check;`, `mod facts;`, `mod ir;` or `mod termination;`. | Process | Test (TC-170) |
| FR-068-CON-2 | This requirement's scope is exactly ADR-011 §7.3 M-5: the checking/evaluation split of `value::expression`. It does not implement M-2 (moving `model::checked_dispatch` or `model::conformance::check_field_refinement_obligation` into `check`, or relabeling `enumeration`/`quantity`/`text`/siblings as `semantic_value`), M-4 (the S4 v2 emitter in `package`), or QSL-165 (removing tier 1's K-designated edges or tier 2's `check` → `value::enumeration`/`value::quantity`/`value::text` edge). A change under this requirement that performs any of these adjacent moves exceeds this requirement's scope even where the ADR eventually requires them. **Scoping note (FR-074, ADR-011 §7.3 M-2, QSL-7, 2026-09-21): this constraint bound FR-068's *own* implementation only, not the ecosystem's ability to do M-2 later under its own requirement.** FR-074 has since implemented M-2 (moving both named items into `check`); this constraint's "does not implement M-2" clause remains an accurate historical statement about FR-068's own change, and was never a prohibition on a later, separately-specified requirement doing that work. | Design | Inspection |
| FR-068-CON-3 | The move relocates every moved type without changing its shape: no variant, field, or method signature on `CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`, `WrongSnapshotCause`, `CheckedPackage`, `CheckedExpression`, `CheckedFunction`, `PackageDeclarations`, `CheckingLimits`, `DepthAboveMaximum`, `DispatchOperation`, `EnumBinding`, `MAX_CHECKING_DEPTH` or `CheckMode` differs between its pre-move and post-move definition, except where this requirement's own Behavior section requires a constructor to become private to `check` (a visibility change on the constructor alone, not a shape change on the type). | Design | Test (TC-170) |
| FR-068-CON-4 | `value::mod.rs` (and any other current re-exporter **other than `model/checked_dispatch.rs` and `model/conformance.rs`, which FR-068-CON-5 governs instead**) may continue to resolve `crate::value::{CheckCause, CheckRefusal, InputRefusal, ...}` after this requirement's implementation, by re-exporting from `crate::check` (for the check-cause types) or from `crate::value::expression` (for `InputRefusal`, `Evaluation`, `LocatedLoss`, `ValueLoss`) instead of from `crate::value::expression`'s now-removed `refusal`/`check` submodules. This is `value`'s own aggregation path continuing to a new source module, the same pattern FR-067-AC-9 already establishes for the `forms` move, and is not a compatibility shim: no old defining location (`value::expression::refusal`, `value::expression::check`) remains reachable after the move. | Design | Test (TC-173) |
| FR-068-CON-5 | **RETIRED by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** `model/checked_dispatch.rs` and `model/conformance.rs` import their thirteen relocated names (see Behavior, "The interim `model` → `check` edge") directly from `crate::check`, never through `crate::value`'s aggregate re-export, so the interim `model` → `check` edge stays visible to a textual scan of `model`'s own `use` lines. This is the one place FR-068-CON-4's aggregation-continues allowance does NOT apply, precisely because applying it here would hide a genuinely forbidden-until-M-2 reverse edge behind `value`'s indirection. This constraint described the interim edge's own shape while it existed; FR-074 closed the edge by moving both files' relocated code into `check` itself, so there is no longer a `model/checked_dispatch.rs` or a `model/conformance.rs` importing from `crate::check` at all (FR-074-AC-3: the edge is bounded to zero files and zero names, not thirteen). This constraint is false by design from FR-074 onward and is retired, not amended: see FR-074-AC-3 and TC-262, its closer. | Design | Test (TC-176), superseded by TC-262 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-068-AC-1 | After this requirement's implementation, `check.rs`, `facts.rs`, `ir.rs` and `termination.rs` are defined exactly once, under `check`, and `value::expression` no longer declares `mod check;`, `mod facts;`, `mod ir;` or `mod termination;`. A module-tree scan of the compiled crate confirms both halves: presence under `check`, and absence of the four module declarations under `value::expression`. This criterion fails on a copy that adds the four modules under `check` while leaving `value::expression`'s original `mod` declarations in place, not only on their outright absence from the tree. | Test (TC-170) |
| FR-068-AC-2 | `CheckedPackage`, `CheckedExpression` and `CheckedFunction` are defined exactly once, in `check`, and at their `check`-module definition site none of their fields, and neither struct's constructor, carries any visibility qualifier — not `pub`, not `pub(crate)`, not `pub(super)` or `pub(in path)`; the only `pub` surface these three types expose is `check`'s own named accessor methods. This criterion is verified by Inspection, not by a compiled negative test: `compile_fail` is a doctest-only attribute in this crate (every one of the 17 existing `compile_fail` sites — `src/package.rs:188`, `src/temporal/mapping.rs:125`, `src/forms/dispatch.rs:38`, `src/forms/syntax.rs:709`, and the rest — is a doctest), there is no `trybuild`/`compiletest` dependency to run a compile_fail check from inside the crate instead, and a doctest links the crate externally — the same external vantage `tests/` has, from which these three types' fields are already unconstructible today (module-private, not even `pub(crate)`, at `value/expression/mod.rs:58-79` on the pre-move baseline) for a reason unrelated to this move; a doctest-based before/after comparison would therefore pass identically pre- and post-move and could not demonstrate the flip a Rust-internal (descendant-to-sibling) visibility change produces. A direct visibility scan of the three types' declarations, together with confirming `value::expression`'s `CheckedPackage::call`, `CheckedPackage::evaluate` and the argument-admission logic reach these types' state only through `check`'s named accessor methods — never through a re-exported field or a module-path trick — is this criterion's actual verification method. **Amended by FR-087 (owner ruling on QSL-158, 2026-09-21): `CheckedPackage` is relocated out of `check` into layer-4 `package`** (ADR-013 T-1; FR-087-AC-1 restates this criterion's privacy claim for `CheckedPackage` at its new site). This criterion continues to hold, and is not superseded, for `CheckedExpression` and `CheckedFunction`, which stay in `check` unchanged; it no longer applies to `CheckedPackage`, which this requirement's own text placed here as a real disagreement with ADR-011 §4 that the ruling resolved in §4/T-1's favor once `CheckedGraph` (`check`'s own S3 output, absent when this requirement was implemented) existed for `check` to return instead (see FR-087 Description, item 2). | Inspection (TC-171); superseded in part by FR-087-AC-1 for `CheckedPackage` |
| FR-068-AC-3 | The compiled crate's real `use` lines — not a table in the ADR or a prose claim — show that no module under `check` imports anything at all from `value::expression`, and that `check` and the pre-existing `checking` module (`src/checking.rs` and `src/checking/`) remain two distinct modules with no content moved between them. A source scan of every `use` statement in every file under `src/check/`, resolved at the post-macro-expansion level, fails this criterion if any import resolves into any part of `value::expression` — `Machine`, `Callable`, `Evaluation`, `InputRefusal`, `LocatedLoss` and `ValueLoss` are illustrative examples of such an import, not an exhaustive deny-list to match textually against — or if `src/checking.rs` or any file under `src/checking/` gained, lost, or changed content as part of this change. `CheckMode` is not one of these examples: this requirement's Outputs allocate it to `check` (see Outputs and FR-068-CON-3), so a `check` file defining `CheckMode` is the required shape, not a violation of this criterion; only an import of `CheckMode` from `value::expression` — meaning `check` failed to bring its own definition — would trip this criterion. | Test (TC-172) |
| FR-068-AC-4 | After this requirement's implementation, `check` defines every check-cause type this requirement names (`CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`, `WrongSnapshotCause`) and does not define `InputRefusal`; `value::expression` defines `InputRefusal`, located beside `CheckedPackage::call`'s and `CheckedPackage::evaluate`'s admission code, and does not define any of the twelve check-cause types. A type-definition scan over both modules confirms the twelve-versus-one split exactly, failing if any check-cause type is left behind in `value::expression`, if `InputRefusal` is moved into `check`, or if any name is defined in both. | Test (TC-173) |
| FR-068-AC-5 | Given a domain package with at least one pair of functions that differ from each other in parameter count, slot count and dispatch-table membership, whose functions and dispatch tables `PackageDeclarations::check` (now in `check`) admits without refusal, calling each function's `CheckedPackage::call` (still in `value::expression`) with valid arguments through an unchanged object environment and meter produces the same `Evaluation` result — including the correct function's own `slots` count reflected in the result — the pre-move code produced for the identical inputs; given a package `PackageDeclarations::check` refuses, the same `CheckRefusal` set is produced before and after the move. The discriminating fixture (two functions differing in shape, not one) is required because this criterion's own named wrong implementation — an accessor returning a different function's `slots` — is unobservable with only one function or with functions of identical shape; a before/after regression test run against such a fixture, comparing results field for field, fails on a split whose module-shape criteria (AC-1 through AC-4) pass but whose accessor plumbing silently swapped which function's `slots`, `body`, or dispatch table feeds `call`, `evaluate` or argument validation. | Test (TC-174) |
| FR-068-AC-6 | Every file under `check` (`check.rs`, `facts.rs`, `family.rs`'s checking-only half, `ir.rs`, `termination.rs`, `refusal.rs`'s check-cause portion, and `check`'s own `mod.rs` — the file holding the relocated `CheckedPackage`, `CheckedExpression`, `CheckedFunction` and `CheckMode` type definitions and `PackageDeclarations::check`'s and the three `check_*_expression` methods' relocated bodies, per Outputs)'s cross-module **shipped** (non-`#[cfg(test)]`) imports into `value::` sort into exactly two permitted tiers and no third: (a) the nine K-designated siblings X-1 has not yet relocated (`collection`, `comparison`, `composite`, `decimal`, `equality`, `ieee`, `node`, `numeric`, `rational`), unbounded in which items they import since X-1, not this requirement, owns closing them; and (b) exactly `EnumDeclaration`, `EnumValue` from `value::enumeration`, `check_comparable`, `result_unit`, `UnitOperation`, `QuantityUnit` from `value::quantity`, and `TextProfile` from `value::text`, bounded to precisely these seven items across three modules (**amended, PR #282 review F3**: the original text bounded tier (b) to five items across two modules and did not admit `value::text` at all; `family.rs`'s pre-existing, unavoidable `QuantityUnit`/`TextProfile` dependencies, real under both the pre-move and post-move tree, are why this criterion could not pass as originally written against any conforming implementation). This criterion governs `check`'s *shipped* dependency graph only (**scope clarification, owner ruling, PR #282 review, post-rebase**): `check::family.rs`'s own `#[cfg(test)]`-gated `TextType` import (`value::text`) is real but not part of that graph, and is excluded from this criterion's scan on that stated basis rather than admitted into tier (b) — admitting a test-only import into a bound on shipped code would permanently license production code to the same import with no way for this criterion to ever catch that widening back. Every import under tier (a) and (b) is written in crate-absolute, submodule-qualified form (`crate::value::<submodule>::Name`), never through `value`'s flat aggregate re-export, so the tier each import belongs to is legible directly from its own `use` line. No file under `check` imports from `definition`, `unit`, `key`, `reference`, `containment`, `model_query`, `package_identity` or `package` — eight forbidden modules. A source scan of every shipped `use` statement under `src/check/` against this two-tier allow-list fails if a tier-(b) import gains an item beyond the seven named, if any import resolves into any of the eight named forbidden modules, or if any tier-(a)/(b) import is written through `value`'s flat aggregate rather than its owning submodule. **Amended by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21) and by FR-087 (owner ruling on QSL-158, 2026-09-21): `model` and `library` both move off this forbidden list.** The original text forbade `check` from importing `crate::model` or `crate::library` at all, alongside `crate::package`; ADR-011 §6.1's own layer-3 order (`semantic_value < model < library < check core < family checker modules`) places both `model` and `library` *before* `check` core, so a `check` import from either is the later-depends-on-earlier direction the order always permitted — a direct contradiction between this criterion's original forbidden list and §6.1 that FR-068 did not notice because M-5 (this requirement) moved no code that imported from either module. FR-074 (M-2) gives `check` real, legal imports from `model`: `check::checked_dispatch` and `check`'s field-refinement-obligation submodule import `crate::model::{domain_package, key, normalize, ...}` types, and `check_field_refinement_obligation` also imports back into `crate::model::conformance`'s own `ConformanceIndex`/`AxisFailure`/`ConformanceOutcome`/`missing_member`. FR-087 gives `check` a real, legal, closed tier (c) import from `library` — exactly `ImportView` and `LibraryLock` (read-only) — for E3 name resolution: ADR-011 `:253` (the E3 edge row) lists "library lock" beside the import views as an E3 input, and E3's own name resolution (`resolve_name`, moving into `check` per FR-087's owner ruling) reads the relocated `LibraryLock`'s selections to bind a qualified reference's `a::Name` qualifier — `check` reads `LibraryLock` only through its existing read-only accessors, never constructing, mutating, or holding one of its private fields; `check` importing any other `library` item (`VerifiedPackage`, `PackageNodeKey`, or the §4 binding itself) remains outside this criterion's allow-list and fails it. Neither removal is a new violation; the forbidden list is corrected to agree with §6.1 rather than left standing against it. `package` stays forbidden: §6.1 orders `package` (layer 4) above `check` (layer 3), so `check` importing from `package` remains the forbidden reverse direction, unaffected by either amendment. | Test (TC-175, tiers (a)/(b) and the eight-module forbidden list); tier (c) — the `check` → `library::{ImportView, LibraryLock}` edge itself and its two-item bound — verified by FR-087-AC-9/TC-256, not by TC-175 |
| FR-068-AC-7 | **RETIRED by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** After this requirement's implementation, `model::checked_dispatch` and `model::conformance::check_field_refinement_obligation` remain defined in `model`, unchanged, and are absent from `check`. A type/function-definition scan over `check` confirms neither symbol appears there; this criterion fails on an implementation that moves either one into `check` ahead of M-2, even if every other criterion in this requirement passes. This criterion asserted FR-068's own scope boundary (M-5 before M-2). FR-074 is M-2: both symbols are now defined in `check` and absent from `model`, the exact inverse of what this criterion required. This criterion is false by design from FR-074 onward and is retired, not amended: see FR-074-AC-1/FR-074-AC-2 and TC-261, its closer. | Test (TC-175), superseded by TC-261 |
| FR-068-AC-8 | `value::outcome.rs`'s import of `WrongSnapshotCause` resolves to `crate::check::WrongSnapshotCause` after this requirement's implementation, and the crate compiles with this one import path updated and no other change to `value::outcome.rs`. This criterion is satisfied by the path update alone; it does not require, and a correct implementation does not attempt, removing the K→3 direction of this edge (ADR-011 §6.1's X-1 obligation, QSL-131's scope). | Test (TC-173) |
| FR-068-AC-9 | **RETIRED by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** The resolved import graph shows `model` → `check` bounded to exactly two files and exactly thirteen names: `model/checked_dispatch.rs` importing `DispatchCandidate`, `DispatchOperation`, `DispatchTable`, `PackageDeclarations` directly from `crate::check`, and `model/conformance.rs` importing `established_field_fact`, `Connective`, `Established`, `Location`, `Node`, `NodeKind`, `OrderedKind`, `Origin`, `ProvedInterval` directly from `crate::check` — neither file routed through `crate::value`'s aggregate re-export for these thirteen names. This criterion fails if a third `model` file gains a `check` import, if either named file's import list grows beyond its own name count (`checked_dispatch.rs`: four; `conformance.rs`: nine), or if either file reaches these names indirectly through `crate::value` instead of directly, since the indirect form would satisfy a textual `use`-line scan of `model` while hiding the edge from it — this criterion requires the resolved-level check to also hold, not only the textual one. FR-074 closed the edge this criterion bounded: `model/checked_dispatch.rs` and `model/conformance.rs` no longer exist as import sites for `crate::check` at all (the code that needed the import moved to `check` itself), so the resolved `model` → `check` edge is now bounded to zero files and zero names, not two and thirteen. This criterion is false by design from FR-074 onward and is retired, not amended: see FR-074-AC-3 and TC-262, its closer. | Test (TC-176), superseded by TC-262 |
| FR-068-AC-10 | `value::expression`'s own re-export of the relocated checked-output types is exactly `pub use crate::check::{CheckedPackage, CheckedExpression};` — a closed, two-name list naming `check` by its crate-absolute path (since `check` is a top-level sibling of `value`, not a name in scope by a bare `check::` path from inside `value::expression`), never a glob (`pub use crate::check::*;`) and never a third name. A source scan of `value::expression`'s `pub use crate::check::{...}` line fails this criterion if it names anything other than exactly `CheckedPackage` and `CheckedExpression`, if it is a glob import, or if it does not use the `crate::check` path. **Amended by FR-087 (owner ruling on QSL-158, 2026-09-21): the two-name re-export splits into two single-name re-exports from two different crate-absolute paths, because `CheckedPackage` no longer lives in `check`.** After FR-087, `value::expression` names `CheckedExpression` through `pub use crate::check::{CheckedExpression};` (this criterion, narrowed to one name) and `CheckedPackage` through `pub use crate::package::{CheckedPackage};` (a new re-export, FR-087-AC-9's own closed-list assertion, not a third name added to this criterion's `crate::check` line). This criterion no longer governs `CheckedPackage`'s re-export path. | Test (TC-172); narrowed to `CheckedExpression` by FR-087, which adds FR-087-AC-9 for `CheckedPackage`'s own re-export |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §4 (the private-constructor/public-accessor mechanism this requirement
  applies to `CheckedPackage`/`CheckedExpression`/`CheckedFunction`; §4's own
  text names a different owning layer for this pattern than this requirement
  does — see Behavior — and that contradiction is recorded, not amended,
  here), §6.1 (the layer-3/layer-5 allow-list and the intra-layer-3 order
  `semantic_value < model < library < check core < family checkers` that
  makes tier 2's `check` → `enumeration`/`quantity` edge legal and the
  interim `model` → `check` edge a reverse edge), §6.2 (the module-table
  rows for `value::expression::check`, `facts`, `ir`, `termination`,
  `refusal`, `evaluate`, and the `outcome.rs` K-edge X-1 must cut), §7.3 M-5
  (this requirement's ADR row).
- [FR-067](FR-067-add-s2-forms-and-retire-seam-5.md) is this requirement's
  direct predecessor in the corrected order (X-1 → M-3a → M-5 → M-2) and the
  precedent this requirement follows for "a re-export naming a new source
  module is not a second definition" (FR-067-AC-9, applied here in
  FR-068-CON-4).
- [US-009](../usecase/US-009-trust-a-single-check-authority-unreachable-from-evaluation.md).
- Linear QSL-139, owner ruling 2026-09-20 (order correction, import-level
  verification of no `semantic_value` dependency, the QSL-165 interim-edge
  disposition, and "do not amend ADR-011 from this ticket" — PR #271 owns
  that amendment).
- Linear QSL-167 tracks two recorded ADR-011 defects, one of which is §4
  versus §6.1 on `CheckedPackage`'s owning layer (see Behavior,
  "`CheckedPackage`, `CheckedExpression` and `CheckedFunction` move with
  their checking methods"); this requirement cites QSL-167 rather than
  re-deciding or duplicating that defect's disposition.

## Status

Specified under QSL-139 (ADR-011 §7.3 M-5), split out of QSL-25 (#214) by
owner ruling, 2026-09-20, the same pattern FR-067's Status section records
for M-3a's split from the same parent ticket. Both prerequisites — X-1 and
M-3a (QSL-138) — have landed.

**Amended by PR #282 review (2026-09-20).** Implementation (PR #282)
surfaced two gaps the original text did not anticipate, both fixed in this
document rather than worked around in code: `family.rs` was missing from
the move surface despite `check.rs`'s pre-existing dependency on it
(Description, Inputs, Outputs; see F4), and tier 2's five-item,
two-module bound could not pass against any conforming implementation
because `family.rs`'s identity-minting content also needs `QuantityUnit`
(`value::quantity`) and `TextProfile` (`value::text`), neither named
(Behavior, "The layer-3 sibling imports `check.rs` keeps"; AC-6; see F3).
Tier 2 is now seven items across three modules. PR #282 also widened
`check`'s tier-1/tier-2 imports to crate-absolute, submodule-qualified
paths (`crate::value::<submodule>::Name`) rather than `value`'s flat
aggregate, per F2 — required by this document as originally written
(FR-068:280-283, "crate-absolute form") but not implemented that way in
PR #282's first round.

**Scope clarified, post-rebase re-verification (2026-09-20).** Running the
F2/F3 fixes through the resolved import-graph checker for real (not a
textual scan) surfaced a further real dependency: `family.rs`'s own moved
golden-digest test fixture, inside its own `#[cfg(test)] mod tests` block,
depends on `TextType` from `value::text`. A first pass widened tier 2 to
eight items to admit it; on review, this was reverted as the wrong fix
(owner ruling): FR-068-AC-6 bounds `check`'s *shipped* dependency graph, and
a `#[cfg(test)]`-only import is not part of it, so admitting it would have
permanently licensed production code to the same import with no way for
this criterion's own verification to ever catch that widening back. Tier 2
stays seven items across three modules; FR-068-AC-6's verification method is
now stated explicitly as scoped to shipped (non-`#[cfg(test)]`) imports, and
`xtask::import_graph::value_import_edges` implements that scoping directly
(Behavior, "The layer-3 sibling imports `check.rs` keeps"; AC-6).

**Amended by FR-087, owner ruling on QSL-158 (2026-09-21): `CheckedPackage`
relocates from `check` to layer-4 `package`.** This requirement's own
Behavior section ("`CheckedPackage`, `CheckedExpression` and
`CheckedFunction` move with their checking methods") recorded a real
disagreement between ADR-011 §4 (which names `package` as the S4 type's
owner) and §6.1 (whose layer ordering the placement in `check` was chosen
to avoid violating), and opened QSL-167 to track it. The owner ruling on
QSL-158 (2026-09-21) settled that disagreement in §4's favor: the `check` →
`package` reverse edge this requirement avoided was an artifact of
`CheckedGraph` — `check`'s own S3 output type — not existing yet at the
time of this requirement's implementation, not a property the architecture
requires. FR-087 builds `CheckedGraph`, retargets `PackageDeclarations::check`
and its three checking methods to it, and relocates `CheckedPackage` to
`package` (FR-087 Description, item 2; Behavior, "`check` produces
`CheckedGraph`; `package` constructs `CheckedPackage`"). This supersedes
FR-068-AC-2 for `CheckedPackage` only (unchanged for `CheckedExpression`
and `CheckedFunction`), narrows FR-068-AC-10 to `CheckedExpression`'s
re-export alone (FR-087-AC-9 covers `CheckedPackage`'s new re-export from
`package`), and amends FR-068-AC-6's forbidden list (ten modules
originally; eight after this amendment and FR-074's own removal of `model`
together) to move `library` (FR-087's own new layer-3 module, ordered
before `check` core) to a bounded permitted tier (c): exactly `ImportView`
and `LibraryLock` (read-only), for E3's own name resolution. This closes the
`CheckedPackage`-placement half of QSL-167;
its separate §6.2 refusal-row finding is untouched and stays open. TC-171,
TC-172 and TC-175 (this requirement's own tests) are amended in place by
FR-087's corresponding criteria rather than superseded wholesale, since
each test's claim about `CheckedExpression`/`CheckedFunction`, tier 1/tier
2, and the K/M-2 boundaries continues to hold unchanged.

## Open Questions

- **The `value::outcome.rs` → `WrongSnapshotCause` edge is still live on
  disk, despite X-1 being reported landed.** ADR-011 §6.1 lists this exact
  edge (`value/outcome.rs:10,13`) among those X-1 "must cut" so K stays a
  leaf, and frames its removal as "`WrongSnapshotCause` leaves `Refusal`."
  The ruling on QSL-139 reports X-1 as one of M-5's two landed prerequisites,
  but the edge itself — the K-designated module importing a non-kernel,
  layer-3/5 type — is unchanged in the baseline this requirement starts
  from. This requirement's own Behavior section ("The K-designated
  `outcome.rs` edge") resolves the narrow question the move forces (update
  the import path so the crate compiles) without resolving the wider one:
  whether removing the edge's *direction* is still open work X-1 left
  behind, or whether the ADR's "X-1 must cut" language is already stale here
  in a way ADR-011 itself does not record. **QSL-131 is this edge's
  direction-removal owner** (ADR-011 §7.3's X-1 row names it as the
  successor to "the rest" of X-1's deferred cuts); this requirement does not
  decide whether QSL-131's scope already covers this specific line or needs
  amending to. A change that also cuts this edge's direction, rather than
  only its path, would go beyond what FR-068-AC-8 requires and should be
  confirmed against QSL-131's actual scope first, not folded into this
  requirement silently.
