---
id: NFR-015
org: agent-ix
title: "Build evaluator-path crates no_std + alloc behind a default-on std feature"
type: NFR
quality_attribute: portability
relationships:
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: references
  - target: ix://agent-ix/quire-spec-language/ADR-030
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: references
  - target: ix://agent-ix/quire-spec-language/NFR-013
    type: references
---
# NFR-015: Build evaluator-path crates no_std + alloc behind a default-on std feature

## Statement

While its `std` feature is disabled, each QSL crate in the per-crate table
below shall build with `core` and `alloc` only, to the tier its row names.

## Scope

This is the build contract for the QSL crates that the AD-001 slices make
`no_std`-capable. It covers the feature, the target, the toolchain, what each
crate may depend on with `std` disabled, the surface each crate keeps with
`std` disabled, and the `make` gate that checks it. With `std` enabled (the
default) each crate builds and behaves as it does with no `std` feature
declared; AC-3 and AC-6 verify that.

The shared evaluation-core leaf of FR-262 lives in its own repository and
never depends on a QSL crate (ADR-011 FB-05). Its own `no_std` gate belongs
to that repository and to RT's consumer acceptance, not to this requirement.
The leaf therefore never reaches `qsl-foundation` through this contract. The
foundation items the leaf uses (for example `Code`, `Category`,
`CatalogCode`, `InternalFault`, the digest records and the setting limits)
move to a `no_std` home below the leaf in the slice that moves the shared
vocabulary (S1, IR-585), which decides that set. Callers name each moved
item at its new path; `qsl-foundation` re-exports none of them (ADR-011
§7.2, as X-11 applies it). Because `qsl-foundation` builds with `core` and
`alloc` only, an item S1 moves needs no `std` rework to leave.

### Feature contract

- The feature is named `std`. Each crate in the table declares
  `default = ["std"]`, so every dependent that takes the crate with default
  features builds unchanged.
- The crate root carries `#![cfg_attr(not(feature = "std"), no_std)]` and
  `extern crate alloc;`. With `std` disabled, shipped source names `core::`
  and `alloc::` paths only, and imports `alloc` items (`String`, `Vec`,
  `Box`, `ToString`, `format!`, `vec!`) explicitly, since the `no_std`
  prelude lacks them. An item that needs `std` is compiled only under
  `#[cfg(feature = "std")]`. Unit-test modules compiled with `std` disabled
  import their `alloc` items the same way, or declare `extern crate std;`
  under `#[cfg(test)]`.
- A dependency that has a `std` feature is declared with
  `default-features = false`, and the crate's `std` feature turns that
  dependency's `std` feature on (`serde/std`, `sha2/std`, `thiserror/std`,
  and a QSL dependency's own `std`). A dependency used only by `std` items
  is `optional = true` and enabled by `std` (`dep:<name>`).
- `--no-default-features` disables `std` and nothing else is needed to
  build. No second feature (`alloc`, `no_std`) exists: `alloc` is always
  required.
- No output changes meaning with `std` disabled. A collection whose
  iteration order reaches any output is ordered (`BTreeMap`, `BTreeSet` or a
  `Vec` in a stated order) in both configurations; the `std` feature never
  switches a map type or an encoding.
- The `std` feature changes no layer edge: ADR-011 §6.1's "Depends on"
  column holds in both configurations. A layer crate that depends on a lower
  QSL layer reaches the closure tier only when that lower layer is itself
  `no_std`-capable.

### Target and toolchain

The `no_std` target is `thumbv7em-none-eabi`, the governed target RT builds
its `std`-disabled runtime for (RT FR-275-AC-9) and the target the shared
leaves `quire-exact`, `quire-semantic-value` and `quire-canonical` build for
in their own gates. Its sysroot has no `std`, so a build that reaches `std`
fails there instead of passing on the host. It has pointer-width atomics, so
`alloc::sync::Arc` is available. The compiler is the one
`rust-toolchain.toml` pins (channel 1.98.1, which lists this target), the
floor RT sets in FR-275-AC-20; this requirement adds no second toolchain.

### Tiers

- **Closure tier.** `cargo build --locked -p <crate> --no-default-features
  --target thumbv7em-none-eabi` succeeds. This holds only when every normal
  dependency in the crate's closure is `no_std`-capable with `std` disabled.
- **Crate tier.** The crate's own shipped source is `no_std` with `std`
  disabled: `cargo check --locked -p <crate> --no-default-features` succeeds
  on the host, and the crate's direct normal dependencies with `std`
  disabled are exactly the row's list. A crate is in this tier while some
  normal dependency in its closure is not `no_std`-capable; the row names
  the condition that moves it to the closure tier.

Each command selects one package with `-p`, never `--workspace`: the root
crate and other members enable `std` on shared dependencies, and Cargo's
feature unification would turn it back on.

### Per-crate rows

| Crate | Slice | Tier | Normal dependencies with `std` disabled | Behind `std` | Closure-tier condition |
| --- | --- | --- | --- | --- | --- |
| `qsl-foundation` | S3 (IR-587) | closure | `quire-exact` (no features; `#![no_std]`), `serde` with `derive` and `alloc`, `sha2`, `thiserror`, each with default features off; the `ix-trace-rs` proc macro, which runs on the host | no item: every `std::` path the crate uses (`cmp`, `fmt`, `str`, `hash`, `marker`, `num`, `iter`, `slice`, `error::Error`, `collections::{BTreeMap, BTreeSet, btree_map}`, `sync::Arc`) has a `core::` or `alloc::` equivalent on this target | holds: every dependency listed is `no_std`-capable |
| `qsl-package` | S5 (IR-589) | crate | `qsl-foundation` with default features off, `qsl-semantics`, `quire-exact`, `quire-semantic-value`, `quire-canonical` (without `std` in the workspace table), `serde`, `thiserror`, and `ix-trace-rs` | `quire-contract-model` (optional, enabled by `std`) and the items listed in "`qsl-package` surface with `std` disabled" | `qsl-semantics` (S4, IR-588), `qsl-forms` and `qsl-cst` each build with `std` disabled, with their own `std`-only dependencies behind their `std` features: `serde_json`, `tempfile` and the FCD crates (`agent-ix-extraction-frontend`, `agent-ix-semantic-ir`) in `qsl-semantics`; `logos` and `serde_json` in `qsl-cst`. `qsl-package`'s `std` feature then also enables `qsl-semantics/std` |

Rows for `qsl-semantics` (S4, IR-588) and `qsl-eval` (S6, IR-590) are
defined by the checked-term boundary of IR-586, which fixes their
evaluator-side scope.

### The checked-IR path

The checked-IR path is the in-memory checked package an evaluator reads:
the checked graph, its dependency closure keyed by `package_id`, the
resolved library selections and the checked abstraction relation, held by
`qsl-package`'s `CheckedPackage`. It is not the `quire.checked-package/v2`
JSON wire.

With `std` disabled, `CheckedPackage::link(graph)` is the only constructor,
and the package it builds has an empty dependency closure and no
selections. The dependency-bearing constructor, `link_with`, recomputes
each dependency's `package_id` by emitting it (`emit_checked`), and
ADR-013 O-02 forbids accepting a `package_id` from a caller, so `link_with`
and everything it alone uses are `std` items. A `no_std` constructor of a
package with dependencies needs a `no_std` reader of the
`quire.checked-package-id/v2` identity preimage, which IR owns; it is
outside this requirement.

With `std` disabled the checked-IR path reads no JSON and computes no
`package_id`. `qsl-package`'s shipped source does not name `serde_json`; it
reaches the crate through `quire-contract-model` (which also brings
`jsonschema`, `serde_path_to_error`, `serde_stacker` and `stacker`, none
`no_std`) and through `qsl-semantics` and `qsl-cst`. The first edge is
behind `qsl-package`'s `std` feature. The others are behind those crates'
`std` features once the row's closure-tier condition holds.

The wire representation is unchanged: QSpec FR-322 defines
`quire.checked-package/v2`, IR's `quire-contract-model` owns its reader,
and QSL's emitter writes it. `package_id` is the SHA-256 of the RFC 8785
(JCS) bytes of the `quire.checked-package-id/v2` identity preimage, encoded
through `quire-canonical`, the one encoder (ADR-013 §2). When a `no_std`
crate must read JSON, it reads through `quire-canonical`'s `read` (the one
shared iterative reader, ADR-030 RU-2, which builds `no_std` + `alloc`),
never through `serde_json` or a second reader, and it encodes identities
through `quire-canonical`'s encoder only.

### `qsl-package` surface with `std` disabled

Every public export of `qsl-package`, by module:

| Module | With `std` disabled | Behind `std` |
| --- | --- | --- |
| `checked` | `CheckedPackage` with `link`, `bind_abstraction`, `bind_abstraction_at`, `abstraction`, `graph`, `shared_graph`, `dependencies`, `dependency_selections`; `ResolvedDependency` | `CheckedPackage::link_with`, `CheckedPackage::emitted_package_id` and the remembered emission (`OnceLock`); `Import`, the input only `link_with` takes; `LinkRefusal`, which only `link_with` returns, with both variants (`ConflictingDefinition`, and `DependencyEmission`, which carries an `EmitRefusal`); `EmittedPackage` |
| `export` | `export_bindings`, `BoundItem`, `ExportRefusal`, `UnboundElement` | none |
| `emit` | none | the whole module: `emit_checked`, `emit_checked_with_cancel`, `emit_package`, `checked_node_id`, `checked_occurrence`, `Emission`, `EmitRefusal`, `OmissionCause`, `OmittedNode`, and the `test-support` items `checked_node_id_of` and `emit_checked_with_fault` |
| `checked_v2` | none | the whole module: `read_import_view`, `supply_closure`, `clause_roots`, `resolve_clause_root`, `AdmittedPackages`, `ClauseRoot`, `ClauseRootUnresolved`, `ImportViewRefusal`, `SourceMapDefect`, `UnsupportedWire`, `V2ReadLimits`, `V2ReadRefusal` |

`bind_abstraction` and `bind_abstraction_at` keep their behaviour; with
`std` enabled they also clear the remembered emission.

### Gate

The `make` target `build-no-std` runs, for each row, its tier's command and
`cargo clippy --locked -p <crate> --no-default-features` with
`-D warnings` (with `--target thumbv7em-none-eabi` for a closure-tier row),
in a target directory of its own (`$CARGO_TARGET_DIR/no-std`, else
`target/no-std`, so it reuses no other lane's artifacts, NFR-013). The
Makefile's `CI_CHECKS` lists it, so `make ci` runs it. It needs no network
and no tool beyond the pinned toolchain.

## Rationale

AD-001 converts the evaluator's QSL dependencies one slice at a time, with
`std` kept on until IR-593 switches the default. A shared contract keeps
every slice's feature, target and gate the same, so a later slice adds a
row, not a new convention. Building for a target with no `std` in its
sysroot is the only check that cannot pass by accident on the host. The
host `cargo check` of the crate tier lets a crate clean its own source
before its dependencies are `no_std`-capable, so S3 and S5 do not wait for
S4. Clippy with `std` disabled catches unused imports and dead `cfg` code
that exist only in that configuration, which the default and all-features
lanes never build.

A default-on `std` feature keeps each slice from changing any dependent: the
workspace, the CLI and every test lane build `std`. Gating the JSON wire
instead of rewriting it keeps one reader (IR's) and one encoder
(`quire-canonical`) for `quire.checked-package/v2`, as ADR-013 §2 and
ADR-030 RU-2 require, and keeps `package_id` computed, never supplied
(ADR-013 O-02).

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
| --- | --- | --- | --- |
| Per-crate rows whose tier command or clippy run fails in `make build-no-std` | 0 | 0 | Test |
| QSpec FR-322 conformance vectors that fail with default features | 0 | 0 | Test |

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| NFR-015-AC-1 | `qsl-foundation` declares a `std` feature with `default = ["std"]`, and `cargo build --locked -p qsl-foundation --no-default-features --target thumbv7em-none-eabi` succeeds on the toolchain `rust-toolchain.toml` pins. | Test (TC-1110) |
| NFR-015-AC-2 | `cargo tree --locked -p qsl-foundation --no-default-features -e normal --depth 1 --target thumbv7em-none-eabi` lists exactly `quire-exact`, `serde`, `sha2`, `thiserror` and the `ix-trace-rs` proc macro, and the full `cargo tree --locked -p qsl-foundation --no-default-features -e normal,features --target thumbv7em-none-eabi` lists no `std` feature of `serde`, `sha2` or `thiserror`. | Test (TC-1110) |
| NFR-015-AC-3 | `cargo test --locked -p qsl-foundation` and `cargo test --locked -p qsl-foundation --no-default-features` both pass on the host, and every workspace test lane passes with `qsl-foundation`'s default features. | Test (TC-1110) |
| NFR-015-AC-4 | `qsl-package` declares a `std` feature with `default = ["std"]` that enables `dep:quire-contract-model` and `qsl-foundation/std`; `cargo check --locked -p qsl-package --no-default-features` succeeds on the host with the crate root `no_std`; and the full `cargo tree --locked -p qsl-package --no-default-features -e normal -i quire-contract-model` finds no path to `quire-contract-model`. | Test (TC-1111) |
| NFR-015-AC-5 | With `std` disabled, every export in the "With `std` disabled" column of the `qsl-package` surface table resolves, and every export in its "Behind `std`" column does not. | Test (TC-1111) |
| NFR-015-AC-6 | With default features, the `quire.checked-package/v2` bytes and `package_id` that `qsl-package` emits match the QSpec FR-322 conformance vectors (`cargo test -p qsl-package --lib checked_v2` against `QSPEC_DIR`), and the FR-091 S1-to-S4 round trip passes. | Test (TC-1111) |
| NFR-015-AC-7 | With `std` disabled, no shipped source of a crate in the per-crate table names `serde_json`, `HashMap` or `HashSet`, and the `arch-lint canonical-encoder` check passes. | Inspection (TC-1111) |
| NFR-015-AC-8 | `make build-no-std` runs AC-1's build for each closure-tier row and AC-4's host check for each crate-tier row, each with `-p`, plus `cargo clippy --no-default-features -D warnings` per row, in its own `no-std` target directory, and `build-no-std` is listed in the Makefile's `CI_CHECKS`; a shipped `std::` path added to a closure-tier crate outside `#[cfg(feature = "std")]` makes the target fail. | Test (TC-1110) |

## Verification

TC-1110 builds `qsl-foundation` for `thumbv7em-none-eabi` with `std` off,
reads its dependency and feature graph, runs its tests in both
configurations and checks that `make build-no-std` is part of `make ci` and
fails on a shipped `std` path. TC-1111 checks `qsl-package` on the host with
`std` off, reads its dependency closure and its whole public surface, and
reruns the FR-322 conformance vectors with default features. Rust tests
carry `#[trace("TC-...", "NFR-015-AC-...")]` attributes (NFR-005).

## Dependencies

- [AD-001](../assurance/AD-001-shared-evaluation-core-interface.md) slices
  S3 (IR-587) and S5 (IR-589).
- [ADR-011](../decisions/ADR-011-stage-dag-and-dependency-architecture.md)
  §6.1 layer rows, FB-05 and §7.2 with X-11.
- ADR-013 §2 (one RFC 8785 encoder) and O-02 (`package_id` is computed,
  never accepted from a caller);
  [ADR-030](../decisions/ADR-030-arbitrary-nesting-depth-no-fixed-caps.md)
  RU-2 (one shared JSON reader in `quire-canonical`).
- [NFR-013](NFR-013-isolate-native-ci-feature-artifacts.md) (lane target
  directories).
- QSpec FR-322 (`quire.checked-package/v2`); RT FR-275-AC-9 and AC-20
  (governed target and 1.98.1 floor).
