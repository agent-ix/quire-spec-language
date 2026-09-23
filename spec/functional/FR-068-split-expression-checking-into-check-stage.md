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
— have landed: the `forms` core exists (the `qsl-forms` crate) and `LoweredSourceGraph` is absent from
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
the sole non-K exception; none of these is a blocker (see Behavior,
"`check`'s import rule: modules by layer").

**Amendment (PR #282 review, finding F4): `family.rs` is part of this
requirement's move surface.** The verification above covered `check.rs`,
`facts.rs`, `ir.rs` and `termination.rs` only. `check.rs`'s own
`Typer::call`, however, already called `family.rs`'s
`mint_call_identity`/`DEFAULT_PACKAGE_IDENTITY` directly before this
requirement. Leaving `family.rs` unnamed would strand it in
`value::expression`, reopening the `check` → `value::expression` reverse
edge FR-068-AC-3 forbids. This amendment adds `family.rs`'s
identity-minting half (`mint_call_identity`, `mint_declaration_identity`,
`Preimage` and its `encode_*` functions, `OccurrenceMap`, and
`ValueFunctionFamily`'s `FamilyContract::check` half) to Outputs' move
surface, alongside the four files already named.

**Amendment (layer-rule ruling, 2026-09-22): `check`'s imports are bounded
by module and layer, not by item.** FR-068-AC-6 formerly carried a tier (b)
closed item list — exactly seven items across `value::enumeration`,
`value::quantity` and `value::text` that `check` could import — and FR-087
added a tier (c) list of exactly two `library` items. Both item lists are
retired. `check` is bounded by a module-level layer rule instead (Behavior,
"`check`'s import rule: modules by layer"; FR-068-AC-6). The reasons:

1. **The item list capped edges ADR-011 §6.1 already permits.** Layer 3 is
   ordered `semantic_value < model < library < check core`, and layer 3
   depends on 2, F and K. Beyond that layer rule, the list caught only
   `check` using one more item from a module it was already allowed to
   use, which is not a design violation.
2. **It went red on legitimate refactors.** #339, #343 and #344 each had to
   widen the list or route around it. A control that fails in normal
   development is miscalibrated.
3. **It made code worse.** #343 added a constructor only to keep an import
   out of the gate's sight.
4. **It was M-5's ratchet, and M-5 is done.** The property that
   still matters is that `check` imports nothing from a later layer. The
   layer rule keeps that property.

The PR #282 review ruling that kept `check` from importing
`value::text::TextType` is superseded. `value::text` is a permitted K-copy
module, and the rule does not bound which of its items `check` imports.

The layer rule is an interim, source-scan gate. Once `qsl-semantics` is its own
crate (ADR-011 §6.1 crate map, §7.2), Cargo's dependency graph enforces
direction and this gate is deleted.

**The *inbound* direction was not checked by the QSL-139 ruling, and it matters
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

### `check`'s import rule: modules by layer

`check`'s shipped imports SHALL resolve only into the modules this section
permits. The rule names modules, never items: any item of a permitted
module may be imported. The permitted list is closed, so an import into a
module not on it fails even when the module is not on the forbidden list.

`check` MAY import:

- K: `quire_exact`.
- F: `qsl_foundation`.
- Layer 2: `forms`.
- Layer 3 before `check` core, per ADR-011 §6.1's order `semantic_value <
  model < library < check core`:
  - the `value` submodules ADR-011 §6.2 maps to 3 `semantic_value`:
    `value::definition`, `value::enumeration`, `value::unit`,
    `value::quantity`, `value::key`, `value::reference`,
    `value::containment` and `value::semantic_node`;
  - `value::declaration`, the declared-type registry (`TypeEnvironment`,
    `ObjectTypeDeclaration`), which ADR-011 §6.1 keeps out of the kernel and
    in layer 3; the module is created by #344;
  - `model`, and `value::model_query`, which §6.2 maps to 3 `model`;
  - `library`.
- Layer 3 `check` core itself: `check` and its descendants, and `family`
  (`FamilyContract` and `FamilyOutcome`, the family checker trait and
  outcome type ADR-011 §6.1 lists in `check` core; ADR-012 §2 defines the
  contract).
- The `value` K-copy modules, each only while it exists: `value::collection`,
  `value::composite`, `value::decimal`,
  `value::equality`, `value::ieee`,
  `value::numeric`, `value::rational` and `value::text`.
  This is a named module list that only shrinks. A module leaves it in the
  change that deletes that module's QSL copy, and no module is added to it.
  These are the §6.2 kernel-row modules still present under `src/value/`.
  `value::outcome` left this list under QSL-131 O2, which deleted the
  module.

`check` MUST NOT import any later layer:

- layer 4: `checked_package` and `package`;
- layer 5: `value::expression`;
- R: `route`;
- layer 6: `replay`;
- `lowering`.

A `value` submodule this section does not name (today `value::member`) is
unlisted, and an import into it fails. Adding it needs a layer placement in
ADR-011 §6.2 and an edit to this list.

**Scope: shipped code.** The rule covers every `use` line and every
`crate::`-rooted path outside `#[cfg(test)]` items under `src/check/`. A
fully-qualified inline path is covered as well as a `use` line, because an
inline path into a later layer is the same edge. This includes a `use`
inside a function body, a path written inside a macro invocation's
arguments, and a later path through a module a `use` binds (`use
crate::value;` followed by `value::Presence`). A `use` that binds a module
by name (`use crate::checked_package;`) is classified on that module. A
`super::` or `self::` path is resolved relative to its file before it is
classified. The rule
classifies this crate's own modules and the workspace crates `quire_exact`
and `qsl_foundation`; `std` and third-party crates (`serde_json`, `sha2`,
`ix_trace_rs`) are governed by the Cargo manifest, not by this rule. Test code is excluded
because the Cargo dependency graph that replaces this gate governs shipped
code only; `check`'s own tests link a `CheckedPackage` from `checked_package`
to test the S3-to-S4 handoff. FR-068-AC-3's `value::expression` prohibition
is separate and still covers test code.

**Every `value` import names its submodule.** `check` SHALL write each
`value` import as `crate::value::<submodule>::Name`, never through `value`'s
flat aggregate `crate::value::Name`. This requirement is kept because it
still guards something. `value/mod.rs` is not one module in one layer: it
re-exports K copies, `semantic_value` items and layer-5 `value::expression`
items alike. A flat import names no layer, so the rule cannot be applied to
it without resolving the aggregate. It would also keep compiling after a
K-copy module is deleted and its name re-pointed at `quire_exact`, so
`check` would keep depending on the aggregate. That is the dependency the
shrinking K-copy list exists to remove. The qualified form makes each
import's module readable from its own line, and turns each K-copy deletion
into a compile error at every importing site.

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
the `check` → `enumeration`/`quantity` edge. This requirement's own
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
| FR-068-CON-2 | This requirement's scope is exactly ADR-011 §7.3 M-5: the checking/evaluation split of `value::expression`. It does not implement M-2 (moving `model::checked_dispatch` or `model::conformance::check_field_refinement_obligation` into `check`, or relabeling `enumeration`/`quantity`/`text`/siblings as `semantic_value`), M-4 (the S4 v2 emitter in `package`), or QSL-165 (relocating the `value` K copies and `semantic_value` modules `check` imports from). A change under this requirement that performs any of these adjacent moves exceeds this requirement's scope even where the ADR eventually requires them. **Scoping note (FR-074, ADR-011 §7.3 M-2, QSL-7, 2026-09-21): this constraint bound FR-068's *own* implementation only, not the ecosystem's ability to do M-2 later under its own requirement.** FR-074 has since implemented M-2 (moving both named items into `check`); this constraint's "does not implement M-2" clause remains an accurate historical statement about FR-068's own change, and was never a prohibition on a later, separately-specified requirement doing that work. | Design | Inspection |
| FR-068-CON-3 | The move relocates every moved type without changing its shape: no variant, field, or method signature on `CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`, `WrongSnapshotCause`, `CheckedPackage`, `CheckedExpression`, `CheckedFunction`, `PackageDeclarations`, `CheckingLimits`, `DepthAboveMaximum`, `DispatchOperation`, `EnumBinding`, `MAX_CHECKING_DEPTH` or `CheckMode` differs between its pre-move and post-move definition, except where this requirement's own Behavior section requires a constructor to become private to `check` (a visibility change on the constructor alone, not a shape change on the type). | Design | Test (TC-170) |
| FR-068-CON-4 | `value::mod.rs` (and any other current re-exporter **other than `model/checked_dispatch.rs` and `model/conformance.rs`, which FR-068-CON-5 governs instead**) may continue to resolve `crate::value::{CheckCause, CheckRefusal, InputRefusal, ...}` after this requirement's implementation, by re-exporting from `crate::check` (for the check-cause types) or from `crate::value::expression` (for `InputRefusal`, `Evaluation`, `LocatedLoss`, `ValueLoss`) instead of from `crate::value::expression`'s now-removed `refusal`/`check` submodules. This is `value`'s own aggregation path continuing to a new source module, the same pattern FR-067-AC-9 already establishes for the `forms` move, and is not a compatibility shim: no old defining location (`value::expression::refusal`, `value::expression::check`) remains reachable after the move. | Design | Test (TC-173) |
| FR-068-CON-5 | **RETIRED by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** `model/checked_dispatch.rs` and `model/conformance.rs` import their thirteen relocated names (see Behavior, "The interim `model` → `check` edge") directly from `crate::check`, never through `crate::value`'s aggregate re-export, so the interim `model` → `check` edge stays visible to a textual scan of `model`'s own `use` lines. This is the one place FR-068-CON-4's aggregation-continues allowance does NOT apply, precisely because applying it here would hide a genuinely forbidden-until-M-2 reverse edge behind `value`'s indirection. This constraint described the interim edge's own shape while it existed; FR-074 closed the edge by moving both files' relocated code into `check` itself, so there is no longer a `model/checked_dispatch.rs` or a `model/conformance.rs` importing from `crate::check` at all (FR-074-AC-3: the edge is bounded to zero files and zero names, not thirteen). This constraint is false by design from FR-074 onward and is retired, not amended: see FR-074-AC-3 and TC-262, its closer. | Design | Test (TC-176), superseded by TC-262 |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-068-AC-1 | After this requirement's implementation, `check.rs`, `facts.rs`, `ir.rs` and `termination.rs` are defined exactly once, under `check`, and `value::expression` no longer declares `mod check;`, `mod facts;`, `mod ir;` or `mod termination;`. A module-tree scan of the compiled crate confirms both halves: presence under `check`, and absence of the four module declarations under `value::expression`. This criterion fails on a copy that adds the four modules under `check` while leaving `value::expression`'s original `mod` declarations in place, not only on their outright absence from the tree. | Test (TC-170) |
| FR-068-AC-2 | `CheckedPackage`, `CheckedExpression` and `CheckedFunction` are defined exactly once, in `check`, and at their `check`-module definition site none of their fields, and neither struct's constructor, carries any visibility qualifier — not `pub`, not `pub(crate)`, not `pub(super)` or `pub(in path)`; the only `pub` surface these three types expose is `check`'s own named accessor methods. This criterion is verified by Inspection, not by a compiled negative test: `compile_fail` is a doctest-only attribute in this crate (every existing `compile_fail` site — `src/package.rs:188`, `src/temporal/mapping.rs:125` and the rest — is a doctest), there is no `trybuild`/`compiletest` dependency to run a compile_fail check from inside the crate instead, and a doctest links the crate externally — the same external vantage `tests/` has, from which these three types' fields are already unconstructible today (module-private, not even `pub(crate)`, at `value/expression/mod.rs:58-79` on the pre-move baseline) for a reason unrelated to this move; a doctest-based before/after comparison would therefore pass identically pre- and post-move and could not demonstrate the flip a Rust-internal (descendant-to-sibling) visibility change produces. A direct visibility scan of the three types' declarations, together with confirming `value::expression`'s `CheckedPackage::call`, `CheckedPackage::evaluate` and the argument-admission logic reach these types' state only through `check`'s named accessor methods — never through a re-exported field or a module-path trick — is this criterion's actual verification method. **Amended by FR-087 (owner ruling on QSL-158, 2026-09-21): `CheckedPackage` is relocated out of `check` into layer-4 `package`** (ADR-013 T-1; FR-087-AC-1 restates this criterion's privacy claim for `CheckedPackage` at its new site). This criterion continues to hold, and is not superseded, for `CheckedExpression` and `CheckedFunction`, which stay in `check` unchanged; it no longer applies to `CheckedPackage`, which this requirement's own text placed here as a real disagreement with ADR-011 §4 that the ruling resolved in §4/T-1's favor once `CheckedGraph` (`check`'s own S3 output, absent when this requirement was implemented) existed for `check` to return instead (see FR-087 Description, item 2). | Inspection (TC-171); superseded in part by FR-087-AC-1 for `CheckedPackage` |
| FR-068-AC-3 | The compiled crate's real `use` lines — not a table in the ADR or a prose claim — show that no module under `check` imports anything at all from `value::expression`, and that `check` and the pre-existing `checking` module (`src/checking.rs` and `src/checking/`) remain two distinct modules with no content moved between them. A source scan of every `use` statement in every file under `src/check/`, resolved at the post-macro-expansion level, fails this criterion if any import resolves into any part of `value::expression` — `Machine`, `Callable`, `Evaluation`, `InputRefusal`, `LocatedLoss` and `ValueLoss` are illustrative examples of such an import, not an exhaustive deny-list to match textually against — or if `src/checking.rs` or any file under `src/checking/` gained, lost, or changed content as part of this change. `CheckMode` is not one of these examples: this requirement's Outputs allocate it to `check` (see Outputs and FR-068-CON-3), so a `check` file defining `CheckMode` is the required shape, not a violation of this criterion; only an import of `CheckMode` from `value::expression` — meaning `check` failed to bring its own definition — would trip this criterion. | Test (TC-172) |
| FR-068-AC-4 | After this requirement's implementation, `check` defines every check-cause type this requirement names (`CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`, `WrongSnapshotCause`) and does not define `InputRefusal`; `value::expression` defines `InputRefusal`, located beside `CheckedPackage::call`'s and `CheckedPackage::evaluate`'s admission code, and does not define any of the twelve check-cause types. A type-definition scan over both modules confirms the twelve-versus-one split exactly, failing if any check-cause type is left behind in `value::expression`, if `InputRefusal` is moved into `check`, or if any name is defined in both. | Test (TC-173) |
| FR-068-AC-5 | Given a domain package with at least one pair of functions that differ from each other in parameter count, slot count and dispatch-table membership, whose functions and dispatch tables `PackageDeclarations::check` (now in `check`) admits without refusal, calling each function's `CheckedPackage::call` (still in `value::expression`) with valid arguments through an unchanged object environment and meter produces the same `Evaluation` result — including the correct function's own `slots` count reflected in the result — the pre-move code produced for the identical inputs; given a package `PackageDeclarations::check` refuses, the same `CheckRefusal` set is produced before and after the move. The discriminating fixture (two functions differing in shape, not one) is required because this criterion's own named wrong implementation — an accessor returning a different function's `slots` — is unobservable with only one function or with functions of identical shape; a before/after regression test run against such a fixture, comparing results field for field, fails on a split whose module-shape criteria (AC-1 through AC-4) pass but whose accessor plumbing silently swapped which function's `slots`, `body`, or dispatch table feeds `call`, `evaluate` or argument validation. | Test (TC-174) |
| FR-068-AC-6 | Every shipped (non-`#[cfg(test)]`) import under `src/check/` into this crate's own modules or the workspace crates `quire_exact` and `qsl_foundation`, whether a `use` line or a `crate::`-rooted inline path, resolves into a module `check` MAY import (Behavior, "`check`'s import rule: modules by layer"): `quire_exact`; `qsl_foundation`; `forms`; `check` and `family`; `model` and `value::model_query`; `library`; the §6.2 `semantic_value` modules `value::definition`, `value::enumeration`, `value::unit`, `value::quantity`, `value::key`, `value::reference`, `value::containment` and `value::semantic_node`, and `value::declaration`; and the `value` K-copy modules `value::collection`, `value::composite`, `value::decimal`, `value::equality`, `value::ieee`, `value::numeric`, `value::rational` and `value::text`, each only while it exists (`value::outcome` left this list under QSL-131 O2, which deleted the module). No shipped import resolves into `checked_package`, `package`, `value::expression`, `route`, `replay` or `lowering`. The bound is per module: any item of a permitted module may be imported. Every `value` import names its submodule (`crate::value::<submodule>::Name`), never `value`'s flat aggregate. The K-copy list only shrinks: a module leaves it in the change that deletes that module's QSL copy. A scan of `src/check/` fails, naming file, line and resolved module, if a shipped import resolves outside the permitted list, into a forbidden module, or through `value`'s flat aggregate. The gate is interim: it is deleted once `qsl-semantics` is its own crate and Cargo's dependency graph enforces direction. **Amended by the layer-rule ruling (2026-09-22)**; the reasons are in Description, "Amendment (layer-rule ruling, 2026-09-22)". | Test (TC-175) |
| FR-068-AC-7 | **RETIRED by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** After this requirement's implementation, `model::checked_dispatch` and `model::conformance::check_field_refinement_obligation` remain defined in `model`, unchanged, and are absent from `check`. A type/function-definition scan over `check` confirms neither symbol appears there; this criterion fails on an implementation that moves either one into `check` ahead of M-2, even if every other criterion in this requirement passes. This criterion asserted FR-068's own scope boundary (M-5 before M-2). FR-074 is M-2: both symbols are now defined in `check` and absent from `model`, the exact inverse of what this criterion required. This criterion is false by design from FR-074 onward and is retired, not amended: see FR-074-AC-1/FR-074-AC-2 and TC-261, its closer. | Test (TC-175), superseded by TC-261 |
| FR-068-AC-8 | **RETIRED by QSL-131 O2 (2026-09-23), which deletes `value/outcome.rs`.** `value::outcome.rs`'s import of `WrongSnapshotCause` resolves to `crate::check::WrongSnapshotCause` after this requirement's implementation, and the crate compiles with this one import path updated and no other change to `value::outcome.rs`. This criterion was satisfied by the path update alone; it did not require, and a correct implementation did not attempt, removing the K→3 direction of this edge (ADR-011 §6.1's X-1 obligation, QSL-131's scope). `value/outcome.rs` no longer exists: QSL-131 O2 deletes the module and repoints every caller onto `quire_exact::{Outcome, Refusal, Undefined}` directly (TC-390), so the file and the one import path this criterion named are both gone, not relocated. The criterion's claim was true when made and is now permanently unfalsifiable rather than false; it is retired, not amended, since there is no successor file or edge for it to describe. | Test (TC-173), retired |
| FR-068-AC-9 | **RETIRED by FR-074 (ADR-011 §7.3 M-2, QSL-7, 2026-09-21).** The resolved import graph shows `model` → `check` bounded to exactly two files and exactly thirteen names: `model/checked_dispatch.rs` importing `DispatchCandidate`, `DispatchOperation`, `DispatchTable`, `PackageDeclarations` directly from `crate::check`, and `model/conformance.rs` importing `established_field_fact`, `Connective`, `Established`, `Location`, `Node`, `NodeKind`, `OrderedKind`, `Origin`, `ProvedInterval` directly from `crate::check` — neither file routed through `crate::value`'s aggregate re-export for these thirteen names. This criterion fails if a third `model` file gains a `check` import, if either named file's import list grows beyond its own name count (`checked_dispatch.rs`: four; `conformance.rs`: nine), or if either file reaches these names indirectly through `crate::value` instead of directly, since the indirect form would satisfy a textual `use`-line scan of `model` while hiding the edge from it — this criterion requires the resolved-level check to also hold, not only the textual one. FR-074 closed the edge this criterion bounded: `model/checked_dispatch.rs` and `model/conformance.rs` no longer exist as import sites for `crate::check` at all (the code that needed the import moved to `check` itself), so the resolved `model` → `check` edge is now bounded to zero files and zero names, not two and thirteen. This criterion is false by design from FR-074 onward and is retired, not amended: see FR-074-AC-3 and TC-262, its closer. | Test (TC-176), superseded by TC-262 |
| FR-068-AC-10 | `value::expression`'s own re-export of the relocated checked-output types is exactly `pub use crate::check::{CheckedPackage, CheckedExpression};` — a closed, two-name list naming `check` by its crate-absolute path (since `check` is a top-level sibling of `value`, not a name in scope by a bare `check::` path from inside `value::expression`), never a glob (`pub use crate::check::*;`) and never a third name. A source scan of `value::expression`'s `pub use crate::check::{...}` line fails this criterion if it names anything other than exactly `CheckedPackage` and `CheckedExpression`, if it is a glob import, or if it does not use the `crate::check` path. **Amended by FR-087 (owner ruling on QSL-158, 2026-09-21): the two-name re-export splits into two single-name re-exports from two different crate-absolute paths, because `CheckedPackage` no longer lives in `check`.** After FR-087, `value::expression` names `CheckedExpression` through `pub use crate::check::{CheckedExpression};` (this criterion, narrowed to one name) and `CheckedPackage` through `pub use crate::checked_package::CheckedPackage;` (layer-4 `package`'s interim module until X-7, ADR-011 §6.2; a new re-export, FR-087-AC-9's own closed-list assertion, not a third name added to this criterion's `crate::check` line). This criterion no longer governs `CheckedPackage`'s re-export path. | Test (TC-172); narrowed to `CheckedExpression` by FR-087, which adds FR-087-AC-9 for `CheckedPackage`'s own re-export |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §4 (the private-constructor/public-accessor mechanism this requirement
  applies to `CheckedPackage`/`CheckedExpression`/`CheckedFunction`; §4's own
  text names a different owning layer for this pattern than this requirement
  does — see Behavior — and that contradiction is recorded, not amended,
  here), §6.1 (the layer-3/layer-5 allow-list and the intra-layer-3 order
  `semantic_value < model < library < check core < family checkers` that
  makes `check` → `semantic_value`/`model`/`library` edges legal and the
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

**Amended by PR #282 review (2026-09-20).** `family.rs` was missing from
the move surface despite `check.rs`'s pre-existing dependency on it
(Description, Inputs, Outputs; see F4). PR #282 also wrote `check`'s
`value` imports in crate-absolute, submodule-qualified form, per F2.

**FR-068-AC-6 is implemented as a module-level layer rule** (Description,
"Amendment (layer-rule ruling, 2026-09-22)"; Behavior, "`check`'s import
rule: modules by layer"). `xtask::import_graph::check_layer_edges` parses
every file under `src/check/` with `syn` and classifies, outside
`#[cfg(test)]` items: every `use` item at any depth, including a function
body's; every `crate::`/`super::`/`self::`-rooted inline path, including
one inside a macro invocation's arguments; and every later path through a
module a `use` binds. The test `real_check_layer_edges_have_no_violation`
(TC-175) runs it over the real tree in `make ci` and finds no violation.
The `crate::checked_package` paths under `src/check/` are in doc comments
or `#[cfg(test)]` code.

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
`package`), and takes `library` (FR-087's own new layer-3 module, ordered
before `check` core) off FR-068-AC-6's forbidden list. This closes the
`CheckedPackage`-placement half of QSL-167;
its separate §6.2 refusal-row finding is untouched and stays open. TC-171,
TC-172 and TC-175 (this requirement's own tests) are amended in place by
FR-087's corresponding criteria rather than superseded wholesale, since
each test's claim about `CheckedExpression`/`CheckedFunction` and the K/M-2
boundaries continues to hold unchanged.

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
