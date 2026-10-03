---
id: SR-1269
title: "Code review of quire-spec-language PR #621: lock selections bind by identity, no version (QSL-395, IR-535)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@56049f4cfcfc43c1d5b91c9aaab291a7b91497cb; PR #621 diff 8244dd2b4...56049f4c: Cargo.lock; qsl-package/src/emit.rs; qsl-package/src/emit/tests.rs; qsl-package/src/checked_v2/tests.rs; qsl-replay/src/spine/dependency_tests.rs; tests/it/compile_command.rs; context: qsl-semantics/src/check/node_key/mod.rs, qsl-semantics/src/check/mod.rs, quire-contract-ir@c5fa773 crates/quire-contract-model/src/checked_package/v2/mod.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-099
    type: reviews
---
# Code review of quire-spec-language PR #621

## Summary

Ticket: QSL-395 (IR-535). PR: quire-spec-language#621, head 56049f4cf on
main 8244dd2b4, building on quire-contract-ir main c5fa773 (IR #259 merged).
The Rust lane (rust-review) is folded into this file. AC coverage is in
SR-1270.

What the PR does, checked against the code:

- **Emit.** `dependency_selections` (emit.rs:837-846) builds
  `CheckedDependencySelection {identity, package_id}` and `model_selections`
  (emit.rs:974-982) builds `CheckedDomainPackageRef {identity, digest_domain,
  digest}`. At IR c5fa773 both structs have exactly those fields and
  `#[serde(deny_unknown_fields)]` (checked_package/v2/mod.rs:150-172). A
  version cannot come back on the emit side without a compile error, and a
  wire entry carrying `version` is refused by IR's reader.
- **No compat path.** Nothing reads or writes a versioned selection shape.
  `ResolvedImport.selection.version` and `DomainPackageRef.version` stay;
  they are still used by the FR-321 intake preimage, import conflict
  detection and native protocol artifacts, which are out of scope by
  ruling. Clippy with `--all-targets -D warnings` is clean, so no field went
  dead.
- **No hand-edited ids.** The diff changes no digest or `package_id` literal.
  Every expected `package_id` is computed: `emitted_id(&dependency())`,
  `package_id(&geometry, ..)`, `PackageId::of_preimage(..)`. The conformance
  test recomputes QSpec's recorded `package_id` (`0606043a...`) from QSpec
  main 396493c's version-free vectors and matches it. That is an oracle QSL
  cannot edit.
- **Oracles catch a returning version.** The five edited assertions compare
  the whole `dependency_selections` or `model_selections` array with
  `assert_eq!` on `serde_json::Value` (emit/tests.rs:2300-2306, 2656-2667;
  dependency_tests.rs:139-152; compile_command.rs:723-730). An extra
  `version` member makes each one fail.
- **Cargo.lock** moves quire-contract-model to c5fa773 and quire-canonical to
  b4bb97a. Cargo.toml still tracks `branch = "main"` for both. The temporary
  IR-#259 pin from fdafeee4c is gone at the head, and the squash merge drops
  it from history.
- **Rust surface.** The change only removes fields: no new panics, integer
  conversions, unsafe code, locks or allocation.

## Verdict

PASS with three low findings. Each one is a doc comment that still says the
lock carries a version. The code, the IR contract and the test oracles are
correct. The PR can merge once the three comments are corrected.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The doc comment of `the_dependency_closure_is_written_in_the_lock_and_the_preimage` still says the closure is written as one `{identity, version, package_id}` entry. This PR changed the test's assertion to expect no version. | qsl-package/src/emit/tests.rs:2632-2633 |
| FND-002 | low | The doc comment of `a_model_bearing_unit_emits_its_model_selection_and_reads_back_verified` says the lock and the preimage select the domain package "by identity, version and the `sha256-jcs` digest". The assertion this PR edited expects no version. | qsl-package/src/emit/tests.rs:2263-2266 |
| FND-003 | low | The `ModelOwner` doc comment says "the version is selection evidence in the lock's `model_selections`". The lock no longer carries a version. | qsl-semantics/src/check/node_key/mod.rs:179-181 |

## Dispositions

Round 1, reviewed at 536d437865d523ababa33941d8363bfbcfead278 (fix commit
536d43786 over 56049f4cf). The focused check
`cargo test -p qsl-package --lib conformance_dependency_selection_vectors`
compiled and passed (logs/qsl-395-sel-d1-check.log).

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 536d43786 |
| FND-002 | fixed | 536d43786 |
| FND-003 | fixed | 536d43786 |
