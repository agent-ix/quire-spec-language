---
id: SR-15530
title: "integrity review of STD-299 no_std build contract (QSL PR #1223)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-spec-language@44f497b8ced379b560d10c2707c97b643ca609e4; spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md, spec/test-cases/TC-1110-qsl-foundation-builds-no-std-on-the-governed-target.md, spec/test-cases/TC-1111-qsl-package-gates-the-checked-package-wire-behind-std.md, spec/assurance/AD-001-shared-evaluation-core-interface.md, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md, spec/functional/FR-262-evaluate-values-and-calls-at-any-depth.md, spec/spec.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/NFR-015
    type: reviews
  - target: ix://agent-ix/quire-spec-language/AD-001
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
# SR-15530: integrity review of STD-299 no_std build contract (QSL PR #1223)

## Summary

Ticket: STD-299. Diff `origin/main...44f497b8c` (main 85f6f62a0), 7 files, spec only. The source
claims were checked against QSL main.

These claims hold:

- `qsl-foundation`'s normal dependencies are `ix-trace-rs` (a `proc-macro = true` crate),
  `quire-exact`, `serde`, `sha2` and `thiserror`. `quire-exact` at the locked rev 1c6cbcb is
  `#![no_std]` and its only feature is `test-support`.
- `qsl-foundation`'s `std::` paths are all in the listed set, and it names no `HashMap`, `io`,
  `fs`, `env`, `time`, `thread` or float math. The reviewer measured the build on a scratch copy:
  1. Applying the feature contract (`default = ["std"]`, the three dependencies with default
     features off, the `cfg_attr` root) and running `cargo build --locked -p qsl-foundation
     --no-default-features --target thumbv7em-none-eabi` built every dependency. The 231 errors
     were all in the crate itself, and all were `std::` paths or prelude names (`String`, `Vec`,
     `Box`, `format!`, `vec!`).
  2. Mechanically rewriting the paths to `core::`/`alloc::` and importing those names made the
     library build clean.
  3. `cargo tree ... -e normal,features` then listed no `std` feature anywhere in the closure.
- `rust-toolchain.toml` pins 1.98.1 and lists `thumbv7em-none-eabi`. RT FR-275-AC-9 and AC-20 say
  what NFR-015 cites.
- `quire-contract-model` (IR bcf7e35) depends on `serde_json` with default features, `jsonschema`,
  `serde_path_to_error`, `serde_stacker` and `stacker`.
- `qsl-package`'s shipped source names no `serde_json`: `emit/extent_agreement.rs` is
  `#[cfg(test)]`. `CheckedPackage` holds a `std::sync::OnceLock`.
- `quire_canonical::read` exists, and `quire-canonical` builds `no_std` + `alloc` without `std`.
- The `HashMap` at `qsl-semantics/src/model/population.rs:1497` is only used for `get`.
- `CI_CHECKS` and `arch-lint-canonical-encoder` exist in the Makefile.

The author's disagreement with terra:QSL:code: `qsl-foundation` re-exports none of the items S1
moves. The reviewer supports the author. ADR-011 X-11's row ("none: no re-export from
`qsl-semantics` or any other crate (§7.2)") and §6.1's SV text ("No QSL crate re-exports them") set
the precedent for moving items down to a shared leaf, and a re-export would be a compatibility
layer. §7.2's own text speaks only of the root crate, so citing X-11 next to §7.2 would be more
exact.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | major: AD-001's Risks bullet says `logos` "reaches the evaluator closure only through `qsl-cst` and `qsl-forms` (the parser), which are not on the evaluator path and stay `std`". That is false. `qsl-semantics` has a normal dependency on `qsl-forms`, which has a normal dependency on `qsl-cst`, which needs `logos` and `serde_json`. ADR-011 §6.1 gives layer 3 the edge "2, F, SV, K", and the new ADR-011 bullet says "No `std` feature changes a layer edge". Together these mean `qsl-semantics`, and so `qsl-package`'s closure tier, needs `qsl-forms` and `qsl-cst` to be `no_std` as well. NFR-015 lines 105-111 name only `qsl-semantics`, and `qsl-semantics` also has normal dependencies on `tempfile` and the FCD crates. <br>Fix: correct the AD-001 verdict. Then either state the full closure-tier condition (layers 2 and 1, `logos` without `std`, `serde_json` gated in `qsl-cst`), or bring the question of cutting the 3-to-2 edge under `std` to the owner. Cutting it contradicts the new bullet. | spec/assurance/AD-001-shared-evaluation-core-interface.md:237-246, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:105-111, spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:886-896 |
| FND-002 | high | major: the checked-IR path is defined as including "its dependency closure keyed by `package_id`", and it "holds identities the `std` side already computed". A `std`-off build has no `std` side. On main, `link_with` calls `emit_checked` through `recomputed_package_id`. `LinkRefusal::DependencyEmission` carries an `EmitRefusal`, and `EmittedPackage::new` takes IR's `CheckedPackageIdentityPreimageRefV2`. All three are `std` items under this contract, so with `std` off the only constructor left is `CheckedPackage::link(graph)`, which has an empty closure. A caller-supplied `package_id` is ruled out by ADR-013 O-02. The spec should say which of these holds: <br>- with `std` off, `CheckedPackage` has no dependency constructor, and the closure is empty until a named follow-up; or <br>- a `no_std` closure constructor exists that recomputes `package_id` through `quire-canonical` without IR's preimage types. <br>It should also say what `LinkRefusal` looks like with `std` off. Without this, a coder has to guess. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:114-119, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:103 |
| FND-003 | medium | minor: NFR-015-AC-5 splits only part of `qsl-package`'s public surface. These exports fall on neither side: <br>- `checked_v2`: `clause_roots`, `resolve_clause_root`, `AdmittedPackages`, `ClauseRoot`, `ClauseRootUnresolved`, `ImportViewRefusal`, `SourceMapDefect`, `UnsupportedWire` <br>- `emit`: `checked_node_id`, `checked_occurrence`, `OmissionCause`, `OmittedNode` <br>- `checked`: `EmittedPackage`, `Import`, `LinkRefusal`, `link`, `link_with`, `bind_abstraction`, `bind_abstraction_at` <br>- `export`: `export_bindings`, `BoundItem`, `ExportRefusal`, `UnboundElement` <br>State the split by module (`emit` and `checked_v2` are `std`; `checked` and `export` are `no_std` except named items), and have TC-1111 step 4 check the whole list. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:178, spec/test-cases/TC-1111-qsl-package-gates-the-checked-package-wire-behind-std.md:26-30 |
| FND-004 | low | minor: the new ADR-011 bullet says `make build-no-std` "checks it for `thumbv7em-none-eabi`". NFR-015's crate tier is a host `cargo check`, so `qsl-package` is not built for that target. Say "runs each row's tier command". | spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md:894-895 |
| FND-005 | low | minor: NFR-015 has status and plan wording in places where it should state behaviour or conditions: <br>- "already `core` + `alloc` clean" (line 46) <br>- "nothing in the current source" (row, line 102) <br>- "(`CheckedPackage` today, in `qsl-package`)" (line 117) <br>- "names no `serde_json` today" (line 122) <br>- "the second leaves with S4" <br>- "its row moves to the closure tier" <br>- "join this table when ..." <br>Rephrase each as a condition: what holds with `std` off, and which row condition changes the tier. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:46, spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:102-130 |

## Verdict

Not mergeable as is. The two majors are FND-001, a false dependency verdict in AD-001 that also
makes the closure-tier condition incomplete, and FND-002, which leaves undefined how a closure is
built with `std` off. Lead default 2 (the checked-IR path is the in-memory closure, the wire stays
on the `std` side, and no new QSL reader is added) is sound, but it needs FND-002's answer. Lead
defaults 1 (`std` feature, no `alloc` feature, `thumbv7em-none-eabi`, `make build-no-std` in
`CI_CHECKS`, `no-std` target directory) and 3 (no re-export) are sound. The S3 contract for
`qsl-foundation` is accurate and buildable. The minors can be fixed in the same round.

## New findings (disposition pass 1)

Round 1 reviewed QSL #1223 at c6e3c941e, with main a89136bfa merged in.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | minor: the surface section says "`bind_abstraction` and `bind_abstraction_at` keep their behaviour; with `std` enabled they also clear the remembered emission". On main only `bind_abstraction` resets `emission`. `bind_abstraction_at` (`qsl-package/src/checked.rs:473-482`) sets the relation and leaves the remembered emission in place. As written, the sentence either requires a behaviour change for `bind_abstraction_at` with `std` on, which contradicts "keep their behaviour" and the Scope's "behaves as it does with no `std` feature declared", or it is wrong. Say that only `bind_abstraction` clears it. If clearing it in `bind_abstraction_at` is the intended fix for a stale `emitted_package_id`, state that as its own behaviour with an AC. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:164-165 |
| FND-007 | low | minor: NFR-015 lines 130-133 and AD-001's Risks bullet say a `no_std` constructor of a dependency-bearing package "needs a `no_std` reader of the `quire.checked-package-id/v2` identity preimage". To recompute a dependency's `package_id` from an in-memory `CheckedPackage`, it builds and encodes the preimage. It reads nothing. What it lacks is IR's preimage types without `std`, encoded through `quire-canonical`. A reader is only needed for a dependency that arrives as bytes. Name both, or name the preimage types. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:130-133, spec/assurance/AD-001-shared-evaluation-core-interface.md:257-259 |

## New findings (disposition pass 2)

Round 2 reviewed QSL #1223 at 70b6296de, delta `c6e3c941e..70b6296de`.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | low | minor: NFR-015-AC-9's second clause makes it a requirement that `bind_abstraction_at` leaves `emitted_package_id` "equal to the emitted `package_id`" after it changes the abstraction relation. The emitter writes one `declaration` occurrence of the relation node at each site (`emit.rs:73`, `:1173`, QSpec FR-451), so after `bind_abstraction_at` that remembered id no longer identifies the package's content. On main it is a stale value, and the AC pins it. NFR-015 is a build contract, so drop the `bind_abstraction_at` half of AC-9 and TC-1111 step 7, and keep the prose stating today's behaviour. Then open a Linear ticket on whether `bind_abstraction_at` should clear the emission the way `bind_abstraction` does. | spec/non-functional/NFR-015-build-evaluator-crates-no-std-behind-a-default-std-feature.md:221, spec/test-cases/TC-1111-qsl-package-gates-the-checked-package-wire-behind-std.md:46-48,63-64 |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
| FND-006 | fixed | - |
| FND-007 | fixed | - |
