---
id: SR-12301
title: "Code review of quire-spec-language PR #1180: refuse unit names shared with records or dimensions (QSL-950)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@c4f570dd90fa9fc0a390463dd768347eeb2f0c80; PR #1180 diff against origin/main: qsl-semantics/src/check/assemble.rs; qsl-semantics/src/check/assemble/tests.rs; qsl-replay/tests/it/declaration_name_collisions.rs; qsl-replay/tests/it/main.rs"
review_set: subset
---
# Code review of quire-spec-language PR #1180

## Summary

Ticket: QSL-950. Code review with the rust-review lane folded in. Scope is
the PR diff only.

The change adds `Unit::quantity_name_collisions`. It refuses each unit
whose name matches a dimension or a record of the same source unit, with
the existing `AssemblyCause::AmbiguousTypeName`
(`ambiguous_declaration`/`ambiguous-name`, which is a catalog pair in QSpec
FR-272). The error sits at the unit's name and lists every candidate span.
It runs in E3 assembly, before emission.

Measured at c4f570dd9 with a local probe that was not committed. Each
source went through `compile_admitted` and then
`CheckedPackageV2::read`:

- Unit/record and unit/dimension refuse at E3, used or unused, in both
  orders, with the right spans.
- Unit/unit refuses with the existing `DuplicateQuantityName`.
- These sources compile and are admitted by the strict reader, so there
  are no false positives:
  - distinct names (`Reading`);
  - case-distinct names (`M`);
  - a unit `m` beside a field `m`, a parameter `m`, an enum case `m` or a
    union variant `m`;
  - a unit beside a type alias of the same name.
- Unit/tuple, unit/union, unit/enum and unit/function compile, and the
  strict reader then refuses them with `AmbiguousDeclaration`/`AmbiguousName`
  at `/semantic_graph/nodes/N/declaration/qualified_name`. So do
  dimension/record, dimension/enum and dimension/function. These are
  measured with each declaration used by a function. See FND-001.

Oracle: with the `errors.extend(unit.quantity_name_collisions())` call
replaced by a discard, `unit_record_and_unit_dimension_collisions_are_typed_before_emission`
fails. The probe then shows unit/record compiling and strict-admitted when
unused. So the new test does detect the check being removed.

Gates run at c4f570dd9:

- `cargo test -p qsl-semantics --lib check::assemble`: 72 passed.
- `cargo test -p qsl-replay --test it`: 88 passed, including QSL-946's
  `quantity_type_source`.
- `cargo test -p qsl-package --lib emit::tests::admission_corpus`: 3 passed.
- `cargo clippy -p qsl-semantics -p qsl-replay --all-targets -D warnings`:
  clean.
- `cargo fmt --check`: clean.
- `cargo run --locked -p xtask -- string-edge`: clean.
- `make arch-lint-api-surface-qsl`: PASS.
- `make check-spec-validation`: rc 0.

Rust-review lane:

- No panics or unsafe on production paths.
- `BTreeMap` keeps the error order deterministic.
- The candidate spans are sorted by position.
- The per-unit candidate `clone()` is bounded by the declaration count.
- No string-edge violations.

## Verdict

NOT MERGEABLE. FND-001 is a major: the same defect class QSL-950 reports
(QSL accepts, the strict reader refuses `AmbiguousDeclaration`) is still
reachable through seven other declaration pairs. The fix is local:

- Compare every unit name and every dimension name against every emitted
  declaration name. That is every `unit.declared` entry that emits (record,
  tuple, union, enum) plus function names.
- Add used-declaration tests for each pair.

FND-002 and FND-003 are lows.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `quantity_name_collisions` compares unit names only with dimensions and `Members::Record` composites. The strict reader keys every declared node by `declaration.qualified_name` in one map (quire-contract-model v2 `mod.rs`, `holders`). Measured at c4f570dd9 through `compile_admitted` and then `CheckedPackageV2::read`, these sources compile and the reader refuses them with `AmbiguousDeclaration`/`AmbiguousName`: unit `m` with `tuple m`, `union m`, `enum m` or `function m`, and dimension `m` with `record m`, `enum m` or `function m`, each used by a function. That is QSL-950's defect: QSL accepts and the strict reader refuses. The fix covers one of nine pairs that share the cause. | qsl-semantics/src/check/assemble.rs:733-762 |
| FND-002 | low | The collision reuses `AssemblyCause::AmbiguousTypeName`. That variant's doc says "A qualified type name names more than one declaration of the unit", but no type-name lookup is ambiguous here, because unit and type lookups stay separate. Either update the variant doc to cover emitted-name collisions or use a dedicated cause. The catalog code and cause would be the same. | qsl-semantics/src/check/assemble.rs:184-196 |
| FND-003 | low | The no-false-positive control covers only distinct and case-distinct record names. No test pins that a unit named like a field, a parameter, an enum case or a union variant still compiles and is strict-admitted. All of these were measured passing. A wider check written for FND-001 makes this regression more likely. | qsl-replay/tests/it/declaration_name_collisions.rs:79-111 |
