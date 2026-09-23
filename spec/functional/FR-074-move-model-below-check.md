---
id: FR-074
title: "Move model below check: relocate checked_dispatch and the field-refinement obligation"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-009
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: traces_to
---
# FR-074: Move model below check: relocate checked_dispatch and the field-refinement obligation

## Description

ADR-011 §7.3 M-2 requires `model` to sit below `check` in §6.1's layer-3
order (`semantic_value < model < library < check core < family checker
modules`). Two pieces of `model` code make `model` depend on `check` today:
`model::checked_dispatch` (the whole file) and
`model::conformance::check_field_refinement_obligation` (a contiguous block,
with its own exclusive helpers). FR-068 (M-5, QSL-139) split
`value::expression`'s checking half into the new `check` module but could
not close this dependency itself — the ADR row's own order (M-5 before M-2)
put moving `model` code out of M-5's scope — so FR-068 left it as a
declared, bounded interim `model` → `check` edge (FR-068-AC-9, FR-068-CON-5)
naming exactly two files and thirteen names, forbidden by §6.1's
intra-layer-3 order in every other case, until this requirement closes it.

This requirement is that closure. It moves `model::checked_dispatch` and
`model::conformance::check_field_refinement_obligation` (together with the
latter's seven exclusive helper functions and one exclusive const —
`clause_location`, `self_field_node`, `field_domain_type`,
`presence_condition`, `comparison_condition`, `established_facts`,
`format_interval` and `CLAUSE_SELF_SLOT`) into `check`, widening
`ConformanceIndex` (the struct, its `build` constructor, and its `fields`,
`operations` and `scalars` members) and `missing_member` to `pub(crate)` —
`AxisFailure` and `ConformanceOutcome` were already `pub` and are unchanged
— so the relocated code can call back into them from `check`, which now
sits above `model` in the layer-3 order: that
direction is the one §6.1 permits (later depends on earlier), the reverse of
the forbidden edge FR-068 declared and this requirement removes.

**Scope is exactly ADR-011 §7.3's M-2 row.** This requirement does not touch
`model::accounting` or the kernel `Meter`/`Incomplete`/`LimitKind`
consolidation (QSL-166's and QSL-164's scope, both explicitly outside M-2 per
ADR-011:657-660 and the §7.3 QSL-166 row), and it does not create
`semantic_value` from `value`'s non-kernel submodules (QSL-165, split out of
M-2's original scope on 2026-09-20 and blocked on QSL-131). After this
requirement and QSL-165 both land, `model` depends on `semantic_value`, F and
K only (§6.1); this requirement alone gets `model` only as far as removing
its one `check` dependency, since `semantic_value` does not exist yet.

## Inputs

- `qsl-semantics/src/model/checked_dispatch.rs`, the whole file (1149 lines pre-move),
  including its own interim-edge comment and `use crate::check::{...}` line
  FR-068-CON-5 required (`:109-117`).
- `qsl-semantics/src/model/conformance.rs:763-1094`: the contiguous
  `check_field_refinement_obligation` obligation block (`CLAUSE_SELF_SLOT`,
  `clause_location`, `self_field_node`, `field_domain_type`,
  `presence_condition`, `comparison_condition`, `established_facts`,
  `format_interval`, `check_field_refinement_obligation`), and the file's own
  interim-edge comment and `use crate::check::{...}` line FR-068-CON-5
  required (`:89-100`).
- `model::conformance`'s private `ConformanceIndex` struct and `build`
  constructor, and private `missing_member` function: the state and helper
  the relocated obligation reads back into once it moves.
- `check/mod.rs`'s own "The interim `model` → `check` edge" narrative
  section (FR-068's Behavior), describing the edge this requirement closes.

## Outputs

- `model::checked_dispatch` relocated to `check` as a whole file, in the
  same private-`mod`-plus-explicit-re-export style `check/mod.rs` already
  uses for its M-5 submodules: the file's own public items
  (`checked_dispatch_operation`, `object_type_supertypes`,
  `DispatchBridgeRefusal`, `DispatchRoot`, `MissingClauseField`,
  `OperationClauses`) are re-exported from `check`'s own `mod.rs`, keeping
  the crate's public API for these items reachable at an equivalent path so
  `tests/dispatch_calls.rs` continues to exercise them.
- `model::conformance::check_field_refinement_obligation`, with its seven
  exclusive helpers and one exclusive const, relocated to a new `check`
  submodule, re-exported the same way, so `tests/model_conformance.rs`
  continues to exercise it.
- `model::conformance::ConformanceIndex` (struct and `build`),
  `model::conformance::AxisFailure`, `model::conformance::ConformanceOutcome`
  and `model::conformance::missing_member` widened to `pub(crate)` — no other
  `model::conformance` item widened, since the compiler does not require it.
- Both interim-edge declarations FR-068-CON-5 required
  (`checked_dispatch.rs:109-117`'s comment and `use crate::check::{...}`
  line; `conformance.rs:89-100`'s comment and `use crate::check::{...}`
  line) deleted, since the code they described no longer exists once the
  code that needed them has moved to the module it needed.
- `check/mod.rs`'s "The interim `model` → `check` edge" narrative section
  rewritten to record that the edge is closed and name what closed it.
- Zero definitions of `model::checked_dispatch`,
  `check_field_refinement_obligation`, or any of its seven exclusive helpers
  and one exclusive const, remaining under `model` after the move.
- Zero `use crate::check` edges remaining anywhere under `qsl-semantics/src/model/`: `grep
  -rn "use crate::check" qsl-semantics/src/model/` finds nothing.

## Behavior

### The interim `model` → `check` edge is closed, not merely narrowed

Before this requirement, `model::checked_dispatch.rs` and
`model::conformance.rs` were the two files FR-068-CON-5 permitted to import
directly from `crate::check`, and no other `model` file could gain such an
edge (FR-068-AC-9). After this requirement, no file under `model` imports
from `check` at all: the two files that used to are the two files that move.
This requirement SHALL NOT leave any `model` file — the two relocated files'
former locations or any other — importing from `crate::check`; the resolved
import graph (`xtask`'s `model_check_edges`, not only a textual scan) SHALL
be empty over the whole `qsl-semantics/src/model/` tree.

### `check` depending on `model` is the permitted direction

ADR-011 §6.1 orders `model` before `check` in the layer-3 sequence
(`semantic_value < model < library < check core < family checker modules`):
later depends on earlier. The relocated `checked_dispatch_operation` and
`check_field_refinement_obligation` continue to read model-layer state —
`DomainPackage`, `DomainPackageRecord`, `DeclarationKey`, `EffectiveView`,
`ModelRefusal`/`ModelRefusalCause`, and (for the refinement obligation)
`model::conformance`'s own `ConformanceIndex` — after they move to `check`.
This is `check` (layer-3-later) depending on `model` (layer-3-earlier), the
forward direction §6.1's order permits, not a reverse edge: the property
this requirement establishes is not "the relocated code stops reading
`model` state" (it does not, and could not, without reimplementing the
domain-package traversal it depends on) but "the dependency's direction
matches the layer order," which moving the *reading* code to the
*later*-ordered module achieves without changing what it reads.

### Widening is exactly six items, no more

`ConformanceIndex` (the struct itself, its `build` associated function, and
its `fields`, `operations` and `scalars` members) and `missing_member` are
the only `model::conformance` items this requirement widens to
`pub(crate)` — six items in total. `AxisFailure` and `ConformanceOutcome`
were already `pub` before this requirement and are unchanged: the
relocated `check_field_refinement_obligation` reads them, but reading a
`pub` item widens nothing. `ConformanceIndex`'s own `generals_by_specific`
and `member_owner` members stay private — nothing outside `model` reads
them. No other `model::conformance` item (`walk_ancestors`, `charge_axis`,
`MAX_CONFORMANCE_DEPTH`, `generals_by_specific`, `type_conforms`,
`value_type_conforms`, `multiplicity_conforms`) is widened: the relocated
`check_field_refinement_obligation` does not call any of them, so widening
them would be speculative, not compiler-required.

### `checked_dispatch.rs`'s and the field-refinement module's other imports are untouched in substance

FR-068-CON-5 named exactly which imports of these two files were the
declared interim edge (four names for `checked_dispatch.rs`, nine for
`conformance.rs`) and which were not (`checked_dispatch.rs`'s
`BinaryOperator`, `DeclaredClauseKind`, `Expression`, `FieldInitializer`,
`FunctionDeclaration`, `NodeKey`, `ValueType`; `conformance.rs`'s
`OrderingOperator`, `Value`, `ValueType`). This requirement's move changes
*where* those non-interim imports are written (from `crate::value::{...}`'s
flat form, valid inside `model`, to the crate-absolute, submodule-qualified
form `check`'s own files already use — `crate::forms::{...}`,
`crate::value::composite::{...}` (QSL-131 V5b later deleted `value::composite`
and repointed the import directly onto `quire_exact`), `quire_exact::NodeKey`,
`quire_exact::OrderingOperator` (named `crate::value::numeric::OrderingOperator`
at the time of this move; QSL-131 O3 later deleted `value::numeric` and
repointed the import directly onto `quire_exact`) — per the existing
FR-068-AC-6 tier rule for `check`'s own shipped `value` imports, which now
applies to these files for the first time because they are now under
`qsl-semantics/src/check/`) but
SHALL NOT change *what* they import: the same items, resolving to the same
definitions, before and after the move.

## Constraints

| ID | Constraint | Type | Validation |
| --- | --- | --- | --- |
| FR-074-CON-1 | This requirement's scope is exactly ADR-011 §7.3 M-2: relocating `model::checked_dispatch` and `model::conformance::check_field_refinement_obligation` (with its seven helpers and one const) to `check`, and the four-item `pub(crate)` widening this move requires. It does not implement QSL-165 (creating `semantic_value`), QSL-166 (the `model::accounting`/kernel accounting consolidation) or QSL-164. A change under this requirement that performs any of these adjacent moves exceeds its scope even where the ADR eventually requires them. | Design | Inspection |
| FR-074-CON-2 | The move relocates every moved item without changing its shape: no variant, field, signature or body differs between its pre-move and post-move definition, except for the six named items' visibility (`ConformanceIndex`, `ConformanceIndex::build`, and `ConformanceIndex`'s `fields`, `operations` and `scalars` members, plus `missing_member`, widen from private to `pub(crate)`; `AxisFailure` and `ConformanceOutcome` are already `pub` and are unchanged) and the import-path rewrites FR-074's Behavior section describes (flat `crate::value::{...}` to submodule-qualified form, and `crate::check::{...}` to intra-`check` `super::`-relative form for the names the interim edge covered). | Design | Test (TC-261) |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-074-AC-1 | After this requirement's implementation, `checked_dispatch_operation` is defined exactly once, under `check`, and is absent from `model`. A definition scan over the whole compiled crate confirms both halves: presence under `check`, and absence under `model`. This criterion fails on an implementation that adds a copy under `check` while leaving the original under `model` in place, not only on the original's outright absence. | Test (TC-261) |
| FR-074-AC-2 | After this requirement's implementation, `check_field_refinement_obligation` is defined exactly once, under `check`, and is absent from `model`; its seven exclusive helpers (`clause_location`, `self_field_node`, `field_domain_type`, `presence_condition`, `comparison_condition`, `established_facts`, `format_interval`) and one exclusive const (`CLAUSE_SELF_SLOT`) move with it as one unit, verified by inspection of the diff (none of the eight items remains defined, in whole or in part, under `model/conformance.rs`). | Test (TC-261); Inspection (helper set) |
| FR-074-AC-3 | The resolved import graph shows zero `model` → `check` edges anywhere under `qsl-semantics/src/model/`: `xtask`'s `model_check_edges`, run against the real tree, returns an empty result, and a textual scan (`grep -rn "use crate::check" qsl-semantics/src/model/`) also finds nothing. This criterion fails if any `model` file — including, but not limited to, the two files this requirement moves code out of — still imports from `crate::check` after the move. | Test (TC-262) |
| FR-074-AC-4 | `model::conformance::ConformanceIndex` (the struct, its `build` associated function, and its `fields`, `operations` and `scalars` members) and `model::conformance::missing_member` are `pub(crate)`, and `AxisFailure`/`ConformanceOutcome` remain `pub`; no other `model::conformance` item (including `ConformanceIndex`'s own `generals_by_specific` and `member_owner` members, `walk_ancestors`, `charge_axis`, `MAX_CONFORMANCE_DEPTH`, `type_conforms`, `value_type_conforms`, `multiplicity_conforms`) changes visibility. Verified by inspecting the diff against the pre-move baseline for every visibility-qualifier change in `model/conformance.rs`. | Inspection |

## Dependencies

- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 (the layer-3 order this requirement satisfies for `model`/`check`),
  §7.3 M-2 (this requirement's ADR row).
- [FR-068](FR-068-split-expression-checking-into-check-stage.md) is this
  requirement's direct predecessor (M-5 before M-2, owner ruling on
  QSL-139, 2026-09-20): FR-068 created `check` and declared the interim
  edge (FR-068-AC-9, FR-068-CON-5) this requirement closes, and FR-068-AC-6's
  submodule-qualified import-path rule now applies to the two files this
  requirement relocates into `check` for the first time.
- [US-009](../usecase/US-009-trust-a-single-check-authority-unreachable-from-evaluation.md).
- Linear QSL-7 (ticket for this requirement); GitHub agent-ix/quire-spec-language#241.

## Status

Specified under QSL-7 (ADR-011 §7.3 M-2), the direct successor to FR-068
(QSL-139, ADR-011 §7.3 M-5) in the corrected order (X-1 → M-3a → M-5 → M-2).
This requirement did not exist before this ticket: M-2 was, until now, an ADR
row with no owning FR — this requirement fills that gap.
