---
id: SR-12296
title: "Code review of quire-spec-language PR #1174: QSL-947 admitted unit table through the replay facade (FR-398)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@7f746bf97; PR #1174, only the commit new against origin/qsl-946-impl (git diff origin/qsl-946-impl...HEAD, commit 7f746bf97): qsl-replay/src/admitted.rs, qsl-replay/src/lib.rs, qsl-replay/tests/admitted_unit_facade.rs, qsl-replay/tests/admitted_unit_root_only.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-398
    type: reviews
---
# SR-12296: Code review of quire-spec-language PR #1174: QSL-947 admitted unit table through the replay facade (FR-398)

## Summary

Ticket: QSL-947. This is the code review, with the rust-review lane folded in. The scope is commit 7f746bf97 only. The merge of qsl-946-impl (#1173, QSL-946) and the #1128 commits it carries are gated separately.

Run at 7f746bf97: the six `admitted_*` suites in qsl-replay (composite facade and root-only, context facade, root-only, unit facade, unit root-only) pass with 15 tests. I did not run fmt, clippy or `make ci`; the coder reports fmt, clippy, string-edge, arch-lint-api-surface-qsl and spec validation as run.

What I measured:

- **(a) `id()` is the checker's id.** `unit_ids_are_the_spines_distinct_and_stable` takes its oracle from the spine's own checked graph, without the unit table: the `UnitId` on each `Quantity<u>` parameter of `f` in the checked signature, and the semantic `Unit`/`Dimension` nodes. A source mutant that sets every unit's `id` to the table's first id fails two facade tests (`unit_ids_are_the_spines_distinct_and_stable` and `the_unit_table_holds_every_admitted_unit_by_id`). Replay reads a quantity argument as `WitnessValueType::Canonical` (execute.rs:1322), typed by the same checked signature and `scope().types()`. So a replayed quantity value carries the same id. I traced this and did not run it (see SR-12297 FND-003).
- **(b) The root-only consumer.** `admitted_unit_root_only.rs` names only `qsl_replay` root items, `quire_exact` and `quire_semantic_value`, and mints no key. A test mutant that converts to `m.quantity_unit()` in place of `km.quantity_unit()` fails at the `5/1000` assertion. The oracle `rational(5, 1000)` is a literal, not something the facade produced.
- **(e) The conversion relation.** I probed it with a throwaway test, since deleted. 2 km converts to exactly 2,000,000 mm (km = 1000 m and mm = m/1000, a chain through the root). The affine 0 C converts to 5463/20 K (273.15). The derived `dimension Speed = Length / Time` is listed with `is_base() == false`. `mps : Speed` does not convert to `m`. `QuantityUnit` carries the normalized dimension, the root and the exact path, and `convert_quantity` is the kernel's own function. So `converts_to` plus `quantity_unit()` plus `unit_table()` is the relation CG needs. Cycles are refused at assembly (`QuantityCycle`), so the table never holds one.
- **(d) Refusal order.** Foreign comes before WrongKind, and WrongKind before Unknown. Unit keys are scoped to their package: the same `kg` source under another package identity has a different node key. So an own unit id can never also be a foreign one, and the order never hides an own unit.
- **(g) Gap 5: an importer re-declares a dependency's unit.** An importer of `test/mass` that declares the same `dimension Mass; unit kg` gets its own `kg` with a different key. The library's `kg` id still refuses `Foreign`, naming the library. There is no collision. An importer also cannot name a dependency's unit in source at all: `Quantity<d.kg>` is a parse error, and `Quantity<d::kg>` and `Quantity<kg>` are `UnresolvedTypeName`. `Foreign` therefore fires only for an id read from a dependency's own emitted package.
- **(f) Declared but unused units.** An unused `unit unused : Length` is in `units()`, `unit_declaration` finds it, and `composite_declaration` refuses it `WrongKind`. `enum_declaration` refuses any unit, used or not, as `Unknown` (FND-001).

## Verdict

No major findings. The table is the checker's own (`scope().types().units()`), its ids are bound to the checked signature by an independent oracle, and the conversion path works for chains, scales and offsets. The findings below are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The refusal class for a unit id depends on which lookup is asked. `composite_declaration` reads `nodes`, which now includes every declared unit and dimension, so it refuses a unit `WrongKind`. `enum_declaration` still refuses the same id `Unknown`, used or unused, because the enum path does not consult `nodes`. FR-398 says only that the record and tuple lookup changed, so the code matches the spec. But a consumer that branches on `WrongKind` versus `Unknown` gets different answers for the same id. Either route the enum lookup through `refusal()` as well, or state the asymmetry in the `WrongKind` rustdoc. | qsl-replay/src/admitted.rs:314, qsl-replay/src/admitted.rs:252-257 |
| FND-002 | low | `context_of` calls `unit_tables(dependency.graph())` for each dependency in the closure. That clones the dependency's whole `UnitTable` and builds full `AdmittedUnit`/`AdmittedDimension` values (qualified names included), but only the keys are kept. It is wasted work that grows with closure size, and a malformed name in a dependency's table becomes an `InternalFault` for the importer. Collect the keys directly from `table.ids()` and the dimension walk. | qsl-replay/src/admitted.rs:644-652 |
| FND-003 | low | `AdmittedDimension` exposes only `is_base()`, not a derived dimension's terms and exponents. A consumer can see that `Speed` is derived but cannot read `Length / Time` from the facade. It can only get that through `QuantityUnit`'s normalized dimension, and only for a dimension some unit names. This is acceptable if CG only needs `converts_to`. Otherwise expose the terms. | qsl-replay/src/admitted.rs:142-167 |

## Dispositions

| FND | outcome | sha |
| --- | --- | --- |
| FND-001 | fixed: enum lookup keeps never refusing WrongKind by FR-394; asymmetry stated in the WrongKind rustdoc and FR-398 and tested (enum lookup of an unused unit is Unknown) | 66a821ff2 |
| FND-002 | fixed: dependencies now collect only unit and dimension keys (unit_keys); no unit table clone or name building | 66a821ff2 |
| FND-003 | fixed: AdmittedDimension::terms() exposes a derived dimension's parts and exponents; tested on Speed | 66a821ff2 |
