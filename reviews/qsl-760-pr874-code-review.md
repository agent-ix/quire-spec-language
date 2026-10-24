---
id: SR-11403
title: "Code and Rust review — QSL-760/QSL-788 PR #874 native scalar replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@cd94cfe81f8ef3cde74e35141fdb2e4fc6cc40df; QSL-760; QSL-788; Cargo.lock; qsl-replay/src/execute/tests.rs; qsl-replay/src/execute/tests/native_scalar.rs; qsl-package/src/emit/tests/admission_corpus.rs"
review_set: subset
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-056, type: references }
  - { target: ix://agent-ix/quire-spec-language/TC-1028, type: references }
---

## Summary

Ticket: QSL-760, QSL-788. Reviewed public PR #874 at head `cd94cfe81f8ef3cde74e35141fdb2e4fc6cc40df` against `origin/main` (three-dot diff), with the `rust-review` lane folded in. No build was run by this reviewer.

The `Cargo.lock` diff is one line: the `quire-contract-model` source moves from IR `f7dd613` to `a47f973`. It is the only package sourced from quire-contract-ir. Its dependency list is unchanged, and no other package, git rev or pin (quire-exact, quire-verification-contracts, quire-semantic-value) moves. The IR range `f7dd613..a47f973` holds IR-800 (#403, #405), IR-474 (#402) and IR-795 (#401). IR-800 adds `ScalarTypeForm::{String, Uuid, Timestamp}` to IR's closed vocabulary. QSL has no `match` on `ScalarTypeForm` outside the admission corpus, so no exhaustive match is left unupdated. `emit::form_is_supported` is `CheckedNodeKind::decode`, so the UUID and Timestamp nodes are now written without a production change. No other omission list excludes them. IR-474 reclassifies an unknown member inside an enum, dimension or unit preimage as `unknown_member`. No QSpec adverse or frame mutation that QSL's `checked_v2` tests read targets a preimage member, so no existing expectation flips.

## Verdict

PASS. The tests drive `compose` (parse, select, check, emit with omission refused) and then the public `replay()` with `ReplaySource::Input`. Each asserts typed fields. A constant `true` or `false`, a broken `=` or `!=`, a dropped argument or a missing refusal would each fail at least one vector. The expected values are TC-1028's literal table, not derived from the code. The admission-corpus rows add three `ScalarType` classifications backed by a new `NativeScalars` fixture. They assert that IR's checked-package/v2 reader admits the emitted package at its own `package_id` with a node of each family. They pin no digest, and no existing row or the `NOT_EMITTED_BY_QSL` list changes.

## Examined Scope

```yaml
scope:
  - id: Cargo.lock
    path: Cargo.lock
    role: examined
    excerpt: "source = \"git+https://github.com/agent-ix/quire-contract-ir?branch=main#a47f9731b5b25e3e0ad007a00d6882f6d7da5fd8\""
  - id: qsl-replay/src/execute/tests/native_scalar.rs
    path: qsl-replay/src/execute/tests/native_scalar.rs
    role: examined
    excerpt: "replay(call(function, a, b), ReplayLimits::default()) ... assert_eq!(evaluates(\"equ\", uuid(a), uuid(b)), Some(EvaluatedValue::Boolean(equal)))"
  - id: qsl-package/src/emit/tests/admission_corpus.rs
    path: qsl-package/src/emit/tests/admission_corpus.rs
    role: examined
    excerpt: "row(K::ScalarType(S::String), F::NativeScalars), row(K::ScalarType(S::Uuid), F::NativeScalars), row(K::ScalarType(S::Timestamp), F::NativeScalars)"
  - id: qsl-package/src/emit.rs
    path: qsl-package/src/emit.rs
    role: context_only
    excerpt: "fn form_is_supported(&self) -> bool { CheckedNodeKind::decode(self.tag, self.node.semantic_form()).is_some() }"
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Review Notes

- Rust lane: no `unwrap` or `expect` outside test code. The `let ... else panic!` arms name the unexpected value, and assertions compare enums and codes, not message text. The `Category` check in `evaluates` follows the value, but the value itself is asserted against the table, so the check is not the oracle.
- The full `make ci` gate was not run by the coder (focused tests and clippy only). It is still due before merge.
