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
own module: verified at import level, file by file, over `refusal.rs`,
`ir.rs`, `termination.rs` and `facts.rs` — every `use` resolves to a
K-designated sibling, an F module, `quire_exact`, `std`, or intra-`expression`.
`check.rs:15,20` is the sole exception (`super::super::enumeration::{
EnumDeclaration, EnumValue}` and `super::super::quantity::{check_comparable,
result_unit, UnitOperation}`); this is a declarable interim shape, not a
blocker (see Behavior, "The one declared interim edge").

**This requirement does not amend ADR-011.** PR #271 is already amending the
same rows; this requirement cites the ADR and the ruling as written.

## Inputs

- `value::expression`'s current seven submodules (`check`, `evaluate`,
  `facts`, `ir`, `mod` itself, `refusal`, `termination`), as they exist after
  M-3a: `syntax` has already moved to `forms`, and the seven remaining
  submodules are this requirement's entire input surface.
- ADR-011 §4's stated pattern for an S3/S4 output type: "The constructors of
  S3 and S4 output types are private to their stage modules, so no other
  module can build a checked value," and the parallel example already
  decided for the S4 type in `package`: `package` keeps its fields and
  constructor private and exposes read-only accessors; `value::expression`
  names the type through one `pub use` and implements `call` over those
  accessors, so `value::expression::CheckedPackage::call` is the contract,
  not a second type. §7.3's M-5 row uses the identical phrase for this
  requirement's own public API ("the S6a `CheckedPackage::call` entry"),
  so this requirement applies §4's already-decided pattern to
  `CheckedPackage`, `CheckedExpression` and the crate-internal
  `CheckedFunction`, rather than deciding a new one.
- Every current consumer of a type this requirement relocates:
  `value::mod.rs`'s aggregate `pub use expression::{...}` list, and
  `value::outcome.rs`'s `use super::expression::WrongSnapshotCause;` (see
  Behavior, "The K-designated `outcome.rs` edge").

## Outputs

- A new top-level `check` module (`src/check/`, declared in `src/lib.rs`
  beside `forms`, `model` and `package`) at the layer-3 position ADR-011
  §6.1 assigns it, holding: `check.rs`, `facts.rs`, `ir.rs`, `termination.rs`
  (each moved as a whole module, unchanged in file boundary), the check-cause
  content of `refusal.rs` (`CheckCause`, `CheckRefusal`, `Obligation`,
  `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`,
  `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`,
  `ProvedInterval`, `WrongSnapshotCause`), and the checking behavior and
  checked-output types currently in `value::expression::mod.rs`:
  `PackageDeclarations::check`, `CheckedPackage::check_expression`,
  `CheckedPackage::check_postcondition_expression`,
  `CheckedPackage::check_clause_expression`, and the `CheckedPackage`,
  `CheckedExpression` and `CheckedFunction` type definitions themselves, with
  their constructors private to `check` (ADR-011 §4).
- `value::expression` (layer 5) retaining exactly: `evaluate.rs` unchanged;
  `InputRefusal` (moved from its current location in `mod.rs` to sit beside
  `CheckedPackage::call`'s admission code, still in `value::expression`);
  `CheckedPackage::call`, `CheckedPackage::evaluate` and the argument-admission
  logic (today's `CheckedPackage::validate`), rewritten to operate over
  `check`'s public accessors for `CheckedPackage`/`CheckedExpression` rather
  than over private fields; and one `pub use check::{CheckedPackage,
  CheckedExpression, ...}` per ADR-011 §4's stated pattern, so
  `value::expression::CheckedPackage::call` remains the S6a entry the M-5 row
  names, with `CheckedPackage` itself defined exactly once, in `check`.
- Zero definitions of any moved item remaining under `value::expression`
  after the move: no duplicate, no stub, no re-export whose target is a
  second definition rather than the one in `check`.
- `value::mod.rs`'s aggregate re-export continuing to resolve
  `crate::value::{CheckCause, CheckRefusal, Obligation, ...}` by naming
  `crate::check` instead of `crate::expression`'s (now-removed) `refusal`/
  `check` submodules — the same "value's own aggregation path continues, only
  its source module changes" pattern FR-067-AC-9 already establishes for the
  `forms` move, not a second definition and not a compatibility shim (see
  Constraints, FR-068-CON-4).

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

### `CheckedPackage`, `CheckedExpression` and `CheckedFunction` move with their checking methods

ADR-011 §4 already decides the pattern this requirement applies: an S3 or S4
output type's constructor is private to its stage module, and a consuming
layer reaches it only through public accessors, naming the type through one
`pub use` — the M-5 row's own public-API phrase, "the S6a `CheckedPackage::call`
entry," mirrors §4's own worked example ("`value::expression::CheckedPackage::call`
is the contract, not a second type") precisely. `CheckedPackage`,
`CheckedExpression` and the crate-internal `CheckedFunction` SHALL be defined
in `check`, with their constructors private to `check`. `value::expression`
SHALL construct no value of any of these three types directly; it SHALL
reach `CheckedPackage`'s and `CheckedExpression`'s fields only through public
accessors `check` exposes, and SHALL implement `CheckedPackage::call`,
`CheckedPackage::evaluate` and the argument-admission logic over those
accessors. This requirement does not itemize the accessor surface `check`
exposes beyond what evaluation actually calls (name, body, slots, parameters,
scope, dispatch tables): ADR-011 §4 leaves `package`'s own accessor list
equally unitemized, and this requirement follows the same precedent rather
than inventing a stricter one.

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

### The one declared interim edge: `check` → `value::enumeration`/`value::quantity`

`check.rs` imports `EnumDeclaration`, `EnumValue` from `value::enumeration`
and `check_comparable`, `result_unit`, `UnitOperation` from `value::quantity`
(today `super::super::enumeration`/`super::super::quantity`, relative to
`value::expression`'s nesting; after the move, `crate::value::enumeration`/
`crate::value::quantity`, since `check` is no longer nested under `value`).
M-2 (QSL-7), which relabels `enumeration`, `quantity` and their siblings as
layer-3 `semantic_value`, has not landed and is out of this requirement's
scope (Order: M-5 before M-2). Owner ruling on QSL-139, 2026-09-20: this one
edge is not a blocker, because ADR-011 §6.1's intra-layer-3 order is
`semantic_value < model < library < check core < family checkers`, so an
edge from `check` to modules bound for `semantic_value` runs
later-depends-on-earlier, which the allow-list permits; it is a declarable
interim shape naming QSL-165, not the forbidden reverse edge QSL-165 itself
exists to close. This requirement SHALL preserve exactly this one edge, with
its import path updated to the crate-absolute form, and SHALL introduce no
other edge from `check` into any other `value::` submodule (`model`,
`library`, `package`, or any other sibling not yet relabeled by M-2): an
implementation that widens the edge — for example by importing a `value::`
prelude, or by reaching into `model::checked_dispatch` early — exceeds this
requirement's scope even though M-2 will eventually make some such edges
legal.

### Out of scope: M-2's items stay in `model`, unmoved

`model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` are ADR-011 §6.2's
M-2 items, moving to `check` only when M-2 lands (after this requirement, per
the corrected order). This requirement SHALL NOT move, copy, or duplicate
either one into `check`; both SHALL remain defined in `model`, unchanged,
after this requirement's implementation. `model` itself does not move below
`check` under this requirement; that relabeling is M-2's.

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
| FR-068-CON-1 | This requirement's implementation moves `check.rs`, `facts.rs`, `ir.rs`, `termination.rs`, `refusal.rs`'s check-cause content, and `value::expression::mod.rs`'s checking methods and checked-output types into a new `check` module in the same change that removes them from `value::expression`; no change under this requirement leaves a moved item defined in both modules at once, and no change leaves `value::expression` still declaring `mod check;`, `mod facts;`, `mod ir;` or `mod termination;`. | Process | Test |
| FR-068-CON-2 | This requirement's scope is exactly ADR-011 §7.3 M-5: the checking/evaluation split of `value::expression`. It does not implement M-2 (moving `model::checked_dispatch` or `model::conformance::check_field_refinement_obligation` into `check`, or relabeling `enumeration`/`quantity`/siblings as `semantic_value`), M-4 (the S4 v2 emitter in `package`), or QSL-165 (removing the interim `check` → `value::enumeration`/`value::quantity` edge). A change under this requirement that performs any of these adjacent moves exceeds this requirement's scope even where the ADR eventually requires them. | Design | Inspection |
| FR-068-CON-3 | The move relocates every moved type without changing its shape: no variant, field, or method signature on `CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`, `WrongSnapshotCause`, `CheckedPackage`, `CheckedExpression`, `CheckedFunction`, `PackageDeclarations`, `CheckingLimits`, `DepthAboveMaximum`, `DispatchOperation`, `EnumBinding` or `MAX_CHECKING_DEPTH` differs between its pre-move and post-move definition, except where this requirement's own Behavior section requires a constructor to become private to `check` (a visibility change on the constructor alone, not a shape change on the type). | Design | Test |
| FR-068-CON-4 | `value::mod.rs` (and any other current re-exporter) may continue to resolve `crate::value::{CheckCause, CheckRefusal, InputRefusal, ...}` after this requirement's implementation, by re-exporting from `crate::check` (for the check-cause types) or from `crate::value::expression` (for `InputRefusal`, `Evaluation`, `LocatedLoss`, `ValueLoss`) instead of from `crate::value::expression`'s now-removed `refusal`/`check` submodules. This is `value`'s own aggregation path continuing to a new source module, the same pattern FR-067-AC-9 already establishes for the `forms` move, and is not a compatibility shim: no old defining location (`value::expression::refusal`, `value::expression::check`) remains reachable after the move. | Design | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-068-AC-1 | After this requirement's implementation, `check.rs`, `facts.rs`, `ir.rs` and `termination.rs` are defined exactly once, under `check`, and `value::expression` no longer declares `mod check;`, `mod facts;`, `mod ir;` or `mod termination;`. A module-tree scan of the compiled crate confirms both halves: presence under `check`, and absence of the four module declarations under `value::expression`. This criterion fails on a copy that adds the four modules under `check` while leaving `value::expression`'s original `mod` declarations in place, not only on their outright absence from the tree. | Test (TC-170) |
| FR-068-AC-2 | `CheckedPackage`, `CheckedExpression` and `CheckedFunction` are defined exactly once, in `check`, with private constructors; `value::expression` does not construct a value of any of the three types directly. A `compile_fail` test outside `check` (mirroring the S4/`package` pattern ADR-011 §4 already verifies by the same method) fails to compile when it attempts to build one of these three types by its fields rather than through `check`'s own checking entry points, demonstrating the constructor is actually private, not merely undocumented. | Test (TC-171) |
| FR-068-AC-3 | The compiled crate's real `use` lines — not a table in the ADR or a prose claim — show that no module under `check` imports from `value::expression` (neither `evaluate`, `Machine`, `Callable`, `Evaluation`, `InputRefusal`, `LocatedLoss` nor `ValueLoss`), and that `check` and the pre-existing `checking` module remain two distinct modules with no content moved between them. A source scan of every `use` statement in every file under `src/check/` fails this criterion if any resolves into `value::expression`'s evaluation-only symbols, or into `checking`, or if `src/checking/` gained or lost a file as part of this change. | Test (TC-172) |
| FR-068-AC-4 | After this requirement's implementation, `check` defines every check-cause type this requirement names (`CheckCause`, `CheckRefusal`, `Obligation`, `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`, `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`, `ProvedInterval`, `WrongSnapshotCause`) and does not define `InputRefusal`; `value::expression` defines `InputRefusal`, located beside `CheckedPackage::call`'s and `CheckedPackage::evaluate`'s admission code, and does not define any of the twelve check-cause types. A type-definition scan over both modules confirms the twelve-versus-one split exactly, failing if any check-cause type is left behind in `value::expression`, if `InputRefusal` is moved into `check`, or if any name is defined in both. | Test (TC-173) |
| FR-068-AC-5 | Given a domain package whose functions and dispatch tables `PackageDeclarations::check` (now in `check`) admits without refusal, calling the resulting `CheckedPackage::call` (still in `value::expression`) with valid arguments through an unchanged object environment and meter produces the same `Evaluation` result the pre-move code produced for the identical inputs; given a package `PackageDeclarations::check` refuses, the same `CheckRefusal` set is produced before and after the move. A before/after regression test run against existing checking and evaluation fixtures, comparing results field for field, fails on a split whose module-shape criteria (AC-1 through AC-4) pass but whose accessor plumbing silently changed which field feeds `call`, `evaluate` or argument validation. | Test (TC-174) |
| FR-068-AC-6 | `check.rs`'s only cross-module edge into a `value::` submodule other than through `check`'s own siblings is its import of `EnumDeclaration`, `EnumValue` from `value::enumeration` and `check_comparable`, `result_unit`, `UnitOperation` from `value::quantity`; no other file under `check` imports from any `value::` submodule other than `quire_exact` re-exports already crossing K. A source scan of every `use` statement under `src/check/` against this exact allow-list fails if a new edge into `model`, `library`, `package`, or any other `value::` submodule appears, or if the two named imports gain additional items beyond `EnumDeclaration`, `EnumValue`, `check_comparable`, `result_unit` and `UnitOperation`. | Test (TC-175) |
| FR-068-AC-7 | After this requirement's implementation, `model::checked_dispatch` and `model::conformance::check_field_refinement_obligation` remain defined in `model`, unchanged, and are absent from `check`. A type/function-definition scan over `check` confirms neither symbol appears there; this criterion fails on an implementation that moves either one into `check` ahead of M-2, even if every other criterion in this requirement passes. | Test (TC-175) |
| FR-068-AC-8 | `value::outcome.rs`'s import of `WrongSnapshotCause` resolves to `crate::check::WrongSnapshotCause` after this requirement's implementation, and the crate compiles with this one import path updated and no other change to `value::outcome.rs`. This criterion is satisfied by the path update alone; it does not require, and a correct implementation does not attempt, removing the K→3 direction of this edge (ADR-011 §6.1's X-1 obligation, QSL-131's scope). | Test (TC-173) |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §4 (the private-constructor/public-accessor pattern this requirement
  applies to `CheckedPackage`/`CheckedExpression`/`CheckedFunction`, already
  decided for `package`'s S4 type), §6.1 (the layer-3/layer-5 allow-list and
  the intra-layer-3 order `semantic_value < model < library < check core <
  family checkers` that makes the interim `check` → `enumeration`/`quantity`
  edge legal), §6.2 (the module-table rows for `value::expression::check`,
  `facts`, `ir`, `termination`, `refusal`, `evaluate`, and the `outcome.rs`
  K-edge X-1 must cut), §7.3 M-5 (this requirement's ADR row).
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

## Status

Specified under QSL-139 (ADR-011 §7.3 M-5), split out of QSL-25 (#214) by
owner ruling, 2026-09-20, the same pattern FR-067's Status section records
for M-3a's split from the same parent ticket. Both prerequisites — X-1 and
M-3a (QSL-138) — have landed. Not yet implemented.

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
  behind, tracked elsewhere (the ADR names QSL-131), or whether the ADR's
  "X-1 must cut" language is already stale here in a way ADR-011 itself does
  not record. This requirement does not decide which; a change that also
  cuts this edge's direction, rather than only its path, would go beyond
  what FR-068-AC-8 requires and should be confirmed against QSL-131's actual
  scope first, not folded into this requirement silently.
