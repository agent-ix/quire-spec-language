---
id: SR-947
title: "QSL-358 slice 0 gap analysis of PR 555: stack safety, MSRV 1.82 and its gate against the slice goal"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0983d60a6353a5b6a00db258a59fa089cad8a264; QSL-358 slice 0 items (stack-safe drop, Debug and admits; rust-version 1.82; make ci MSRV check); quire-exact/src/value.rs; quire-exact/src/collection.rs; quire-exact/Cargo.toml; Makefile; spec/decisions/ADR-011-stage-dag-and-dependency-architecture.md (K is a leaf); spec/functional/FR-062-implement-checked-family-contract.md (context: native-stack rule for stages)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: reviews
---
## Summary

Ticket: QSL-358, slice 0. PR: quire-spec-language#555 at 0983d60a.

Each slice item against the change:

- **Drop is stack-safe at any depth: done.** Worklist `Drop` on the three node structs.
  It is tested at depth 100_000 on a 2 MiB thread, and the test fails without the fix
  (measured, SR-946).
- **Debug is stack-safe at any depth: done.** Iterative `Debug` with a depth test and a
  byte-for-byte parity test against captured derive output. The oracle gaps are in
  SR-946 FND-003 and FND-004.
- **admits is stack-safe: done, and needed no change.** `ValueType::admits` reads only
  the outermost value's carried type. The depth test documents this.
- **rust-version 1.98 to 1.82: done.** `quire-exact/Cargo.toml` is edition 2021 with no
  workspace inheritance.
- **A make ci check stops MSRV rot: done for the compiler.** `quire-exact-msrv` builds
  the crate and its dependencies for thumbv7em with the 1.82 rustc, which `RUSTC`
  selects for every unit, build scripts and proc macros included. It is wired into
  `ci`. It does not touch a workflow file.
- **RT can consume it: measured.** I made a scratch crate shaped like RT: `#![no_std]`,
  depending on `quire-exact = { git = "file:///home/peter/dev/quire-spec-language", rev = "0983d60a..." }`.
  `cargo +1.82 build --target thumbv7em-none-eabi` succeeded, with cargo 1.82 itself
  resolving fresh versions. Cargo 1.82 skips the repo's other manifests when it reads a
  git source, so the edition 2024 manifests elsewhere in the repo do not matter. The
  scratch crate is deleted.
- **Untraced tests: allowed by repo practice.** AGENTS.md and CLAUDE.md have no rule
  that every test carries a trace. Before this PR, 6 of the 11 tests in value.rs were
  traced, and most quire-exact tests are untraced. Not a finding. ADR-011's "K is a
  leaf" bullet (`#![no_std]`, "a no_std consumer depends on it directly") is the natural
  home if the guarantee is ever written down. FR-062's native-stack rule covers check
  stages, not kernel values.
- **Main:** no conflict with 3260a231.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new `Value` doc promises that no walk uses the host stack in proportion to depth, and names equality (`plan_equality` / `planned_equality`) and `compare_keys`. Both are worklists, which I read at equality.rs:132 and key.rs:58. But no test runs either one on the deep value. RT's parity goal includes them, and the `deep_value` and `on_small_stack` helpers already exist. Add `plan_equality(&v, &v.clone())` and `compare_keys` depth tests so a future recursive rewrite fails. | quire-exact/src/value.rs:282-295, quire-exact/src/equality.rs:132, quire-exact/src/key.rs:58 |
| FND-002 | low | `quire-exact-msrv` checks the 1.82 compiler against this workspace's `Cargo.lock` with the workspace's cargo. It does not check what RT does: cargo 1.82 reading quire-exact's manifest and resolving its dependency tree fresh. I measured that path and it passes today, so there is no defect now. A later cargo-level regression would still pass the gate, for example `workspace = true` inheritance added to quire-exact/Cargo.toml, or a `=` pin moved to a crate whose manifest needs edition 2024. Either note this limit in the Makefile comment, or add the consumer-shaped build to the target. | Makefile:195-207 |

## Verdict

Every slice-0 item is delivered, and the RT goal (consume with cargo and rustc 1.82 on
thumbv7em, stack-safe drop and `Debug`) is met, which I measured directly. Both findings
are low gaps in what the tests and gate prove, not defects in delivered behaviour.
Mergeable.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | e9e462d7: `deep_values_compare_equal_on_a_small_stack` (plan_equality plus planned_equality returns `Some(true)`) and `deep_values_compare_keys_on_a_small_stack` (`compare_keys` returns `Equal`) run on two separately built `deep_value()`s on a 2 MiB thread. Separately built values cannot take an `Arc` pointer-equality shortcut. Both pass in the e9e462d7 ci log. |
| FND-002 | fixed | e9e462d7: `quire-exact-msrv` now builds a generated no_std consumer (path dependency, empty `[workspace]`) with `cargo +1.82` itself. So cargo 1.82 reads quire-exact's manifest and its dependencies' manifests, and a cargo-level regression fails the gate. The copied lock pins the workspace's versions. I checked: cargo keeps the locked syn 3.0.5 and tinyvec 1.13.2, adds only the consumer, and prunes the rest. That is deliberate, deterministic, and stated in the Makefile comment. A fresh RT resolve picks newer patches (syn 3.0.6 and tinyvec 1.13.3 earlier today), and RT's own lock and CI cover those. The noisy first offline run is SR-946 FND-006. |
