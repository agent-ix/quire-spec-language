---
id: SR-8715
title: "Code review of quire-spec-language PR #807 (QSL-737): typestate scans after child composites"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@ff905e16d51205409fddb3834f869343bad65eed; qsl-replay/src/execute/argument.rs, xtask/src/typestate_scan.rs, qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/tree.rs, qsl-replay/src/execute/tests/child_composite.rs, qsl-replay/src/execute/tests/composite.rs"
review_set: subset
---
# Code review of quire-spec-language PR #807

## Summary

Ticket: QSL-737. Method: code-review with its rust-review lane. PR #807 (base `impl/business-model-meanings` at 998761fff) fixes the two xtask typestate scan failures that #803 caused. `Converter::node` in qsl-replay no longer builds a `NodeKey` from a wire declaration digest (TC-255). It finds the checked composite whose key bytes equal the digest instead. The TC-260 scan now allows exactly `(observation.rs, RawRecord, type_identity)`, and a new negative test covers that allowance. Reviewer-only: no build or test run. The gate claims were checked against the coder's logs.

Examined:
- `Converter::node`, the `Composite`/`Record` arm, argument.rs:429-479 (examined)
- `TypeEnvironment::composites`/`composite` in quire-semantic-value: the composites are an ordered index keyed by `NodeKey` (context_only)
- `RawRecord`, `RawNode::Record`, `SnapshotValue::record`, observation.rs:526-725 (examined)
- every consumer of `RawRecord::type_identity`: observation.rs:752 (`PartialEq`) and document.rs:1296 (`references.record_type`, a lookup to a checked `NodeKey`) (examined)
- `read_raw_value`'s `Open::Record`, tree.rs:121-138 (context_only)
- `bare_type_fields`, `tc_260_scan_rejects_bare_type_fields_beyond_the_raw_wire_allowance`, `tc_260_value_type_ref_is_the_two_member_union`, typestate_scan.rs:1334-1454 (examined)
- `tc_255_no_node_key_is_minted_outside_check_or_from_a_wire_id`, typestate_scan.rs:1023-1061 (context_only, unchanged)
- record-conversion tests in child_composite.rs and composite.rs:258-330 (examined)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test covers the unknown-declaration branch of the changed lookup. Every record witness in child_composite.rs and composite.rs names a declaration the package declares. ("an Inner named for Outer's declaration" names a known one that does not conform.) If `.ok_or_else(refuse)?` at argument.rs:446 fell back to the slot's own declaration, every test would still pass. The PR body says "an unknown declaration still refuses", but nothing proves it. Fix: add a case to child_composite.rs, for example `witness("Nope", &[("amount_minor", 1)])` at `composite("Money")` giving `Err(0)`. | qsl-replay/src/execute/argument.rs:443-446; qsl-replay/src/execute/tests/child_composite.rs:145-166 |
| FND-002 | low | Each record node now does a linear scan over every composite in the package: O(records x composites) comparisons of 32 bytes. Before, it was one map lookup. The conversion runs on an unlimited meter, and the caller only checks occurrence and work-unit limits after conversion, so this cost is not charged. Fix: build a by-bytes index once per `Converter`, as `enums` already is, or add a by-digest lookup to `TypeEnvironment`. Either way no key is minted. | qsl-replay/src/execute/argument.rs:443-446 |
| FND-003 | low | The new test's doc comment calls it "TC-260 step 4". It is a self-test of the scanner for step 3's rule. TC-260 step 4 is the adverse test that no code path accepts a bare `NodeKey` field type, and the FR-088 note in this same PR still says "TC-260 step 4 is not backed". Fix: call it "TC-260 step 3 (scan self-test)". | xtask/src/typestate_scan.rs:1370-1372; spec/functional/FR-088-clause-name-and-type-identity.md:255-256 |

## Verdict

**CONDITIONAL**: no high findings. The two fixes are right, and each side was judged correctly.

TC-255 (production code was wrong). FR-087-AC-6 (FR-087 line 596) says `replay`'s E9 lookup makes "zero `NodeKey`-constructor calls, resolving a `WireNodeId` to a `NodeKey` only by lookup against an already-minted `NodeKey` in an already-checked package". FR-087-CON-3 (line 584) says "the only conforming behavior at E4 and E9 is a lookup against an already-minted `NodeKey` in a checked package, never a constructor call". `WitnessValue::Record.declaration` is a wire-read `WireNodeId`, so `NodeKey::from_digest(*declaration.as_bytes())` broke both. The new code compares bytes against the checked keys. That is the pattern the same function already uses for enums (argument.rs:387-390) and references (argument.rs:406). The change keeps the old behaviour:
- `composites` is keyed by `NodeKey`, so at most one candidate matches and the lookup cannot be ambiguous.
- An unknown digest refuses with the same `refuse()` (`WrongValueKind` naming the parameter). The only difference is that it now refuses before the conformance check rather than after.
- A known digest yields the same declaration as before, and conformance is still checked against the slot.

The known paths are tested by name and type: a child, a grandchild, the same type, a non-conforming type, an unrelated type, and a missing field, at the top level, as an `Option` payload and as each kind of collection element (child_composite.rs). The unknown path is not tested (FND-001).

TC-260 (the scan was too broad). The coder is right and the planner's preference for a typed identity does not fit:
- TC-260 step 3 scopes the rule to "the places that record a field's type for a domain-model declaration". `RawRecord` is a borrowed `Copy` view (`arena: &'v SnapshotValue`) of a runtime snapshot value's `record` member. It holds the value's own type claim from the wire, not a declared field type.
- The string never gets past intake. Its only consumers are structural equality on the raw value (observation.rs:752) and FR-106 check 6, where `references.record_type(record.type_identity())` (document.rs:1296) resolves it by lookup to a checked `NodeKey`. An unknown string is refused as `wrong-value-kind` at its path before any model value is built (`Admit::Record { own: NodeKey }`).
- A typed identity would mean resolving at read time (`read_raw_value`), before check 6's views exist. That moves a check-6 refusal out of FR-106's check order.
- A newtype around `&str` would only rename the string to get past the scan.

The allowance matches the exact triple (file, struct, field). The negative test shows that a bare `String`/`NodeKey` type field, `RawRecord` in another file, and the same field name on another struct in the same file still fail.

Rust lane: no new `unwrap`/`expect`/indexing on a production path. No `as` casts, no `unsafe`, no `#[allow]`. The new test uses a `tempfile` dir with no fixed path. `bare_type_fields` replaces the inline loop, so the rule lives in one place.

Gates, from the coder's logs (worktree `xtask-scans-fix`, now at ff905e16d):
- `cargo test -p xtask --lib`: "test result: ok. 182 passed; 0 failed". The four TC-255/TC-260 tests pass. The fixture compile error in the log is a test that expects a build failure (`a_build_failure_returns_cargos_own_exit_code ... ok`).
- clippy: "Finished" with no warnings. The log does not show the command line, so `-D warnings --all-features` is taken from the PR body.
- qsl-replay: lib "489 passed", `it` "13 passed", and the other binaries pass (6, 1, 12, 9).
- `make check-spec-validation`: only pre-existing warnings, no errors.
- fmt: the log is empty (clean).

The tests ran on 73c1e2216. The final commit ff905e16d only reformats typestate_scan.rs, and the fmt run came after it.
