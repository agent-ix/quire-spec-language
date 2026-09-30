---
id: SR-709
title: "Code and Rust review of the string-edge sweep and detector widening"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language; xtask/src/string_edge.rs; Makefile; src/protocol_artifact/validate.rs; src/state/evaluation.rs; src/temporal.rs; src/protocol_artifact/native_temporal/request.rs; qsl-semantics/src/check/claims.rs; src/linking.rs; src/mapped.rs; src/protocol_artifact/handoff/writer.rs; src/protocol_artifact/intake.rs; src/protocol_artifact/v2/intake.rs; src/protocol_artifact/v3/intake.rs; src/protocol_artifact/number.rs; src/package/intake.rs; src/cli.rs; src/linking/composed/models.rs; qsl-semantics/src/model/intake.rs; qsl-semantics/src/model/domain_package.rs; qsl-semantics/src/check/identity.rs; qsl-semantics/src/check/type_form.rs; qsl-semantics/src/library/bundle.rs; qsl-cst/src/lexer.rs; qsl-cst/src/parser.rs; qsl-foundation/src/digest.rs; qsl-foundation/src/selection.rs; qsl-replay/src/witness.rs; xtask/src/definition_scan.rs; xtask/src/import_graph.rs; xtask/src/main.rs; xtask/src/route_lint.rs; xtask/src/typestate_scan.rs; tests/it/family_outcome_layering.rs; qsl-*/Cargo.toml; Cargo.lock"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-064
    type: reviews
---
## Summary

Ticket: QSL-145 (cross-reference QSL-268). PR: quire-spec-language#485.
Code review with the rust-review lane, scoped to
`git diff origin/main...HEAD` (44 files).

Measured by the reviewer, in a separate detached worktree:

- `cargo xtask string-edge` exits 0 ("no unmarked, unlisted string
  comparison or string match found"). `string-edge` is a prerequisite of
  `ci:` (Makefile:116), and its recipe is `cargo xtask string-edge`.
- `cargo test -p xtask --lib string_edge`: 17 passed, 0 ignored. The
  real-site test is no longer `#[ignore]`d.
- The allow-list is empty (`allow_list()` returns `Vec::new()`,
  xtask/src/string_edge.rs:99-101).
- Mutation checks, each reverted afterwards:
  - `&&`/`||` widening off: the real-site test and the combinator fixture
    both fail.
  - Arm-value widening off, plus the real-site test scanning with marks
    honoured: both fail ("Profile::classify ... no longer found by the
    scan").
  - `strip_prefix` forced branch-gating off: only the synthetic fixture
    fails. The real-site test does not cover `strip_prefix`, because the
    `clock:` site was dropped from it (FND-002).
- Test-module skipping hides no production code today. All 15 non-`tests/`
  files named `tests.rs`/`*_tests.rs` are declared `#[cfg(test)] mod x;` by
  their parent. The name rule is still a latent hole (FND-006).
- Layering: `qsl-attrs` is a proc-macro crate with no dependencies. Adding it
  under layer three pulls in nothing. `tests/it/family_outcome_layering.rs`
  was updated to allow it.
- Behaviour is preserved by `Profile::classify` (same identity sets for all
  five `Family` arms), the `Role` comparison (a byte-equal `String` newtype),
  `SymbolName::new("self")` (valid under the IR identifier grammar, so it is
  `Some`), and `clock_binding_name` (the same `strip_prefix`).

Of the ~50 new marks, most are genuine one-conversion edges:

- CST lexing and parsing
- typed wire readers: digest and selection `FromStr`, `CheckedClauseKind`,
  `NativeValueType`, `SelectedDigest`, witness transcript
- intake readers: `model::intake` `read_*`, `protocol_artifact::intake`
  headers, `reference` and `definitions` (which resolves to the closed
  `RegisteredDefinition`), `package::intake::visit_map`, `number`
- CLI parsing and xtask tooling

`read_connection`, `read_relationship`, `read_population` and
`read_field_member` each convert wire strings to closed enums
(`RelationshipDirection`, `Extent`, `Presence`) or refuse. They are genuine
edges. The marks listed in the findings are the ones that silence dispatch
instead of converting it.

## Verdict

**Changes requested.** Five medium findings. Three of the ADR-010 §4.3
dispatch sites (`valid_digest`, `valid_adapter`, `clock:`) are silenced by
`#[string_edge]` without the conversion ADR-012 §9 decided. The profile site
is converted, but to a second decoder that duplicates the intake enum. Two
further conversions only move the string out of the literal-only scanner's
sight. The rest are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `valid_digest` and `valid_adapter` are marked `#[string_edge]` whole, not converted. ADR-012 §9's table decides "typed canonicalization enum decoded at state-input intake" and "typed adapter-kind enum decoded at intake". FR-064 Behavior says a site "not yet converted by its owning family's ticket" SHALL "continue to be reported as an unresolved violation". The mark makes the gate green with the string still selecting validity. These are two of the named ADR-010 §4.3 sites. The mark launders them rather than closing them. | src/state/evaluation.rs:2477-2481; src/state/evaluation.rs:2781-2785 |
| FND-002 | medium | The `clock:` site is still in the tree but is now invisible to the scan. `clock_binding_name` wraps `strip_prefix(CLOCK_PREFIX)` (a named const, which the literal-only scanner cannot see) and is also marked. It is re-parsed at three runtime sites, where ADR-012 §9 decides "a typed clock-role variant ... the prefix is parsed once, at lexing". The real-site test dropped the site rather than covering it, so the `strip_prefix` widening has no real-site coverage (the mutation removing it fails only the synthetic fixture). The rewrite in request.rs:838 turned a visible literal site into an invisible one. No other hidden site was introduced: the three callers are a name match and two refusals. | src/temporal.rs:50-58; src/temporal.rs:401; src/temporal.rs:598; src/protocol_artifact/native_temporal/request.rs:838 |
| FND-003 | medium | `Profile::classify` adds a second string decoder for the definition-identity vocabulary. `intake::definitions` already resolves each identity to the closed `RegisteredDefinition` enum (StateCore/StateQueries/StateGraph/EventPosition/FixedSample/TimestampedWindow/Protocol) and discards the result. `validate::package` runs immediately afterwards (intake.rs:536-537, v2/intake.rs:493-494, v3/intake.rs:236-237) and re-classifies `definition.identity` from the string. ADR-012 §9 says "decoded once at protocol-artifact intake". Two decoders can drift. Carry the `RegisteredDefinition` through (or classify from it) instead of re-reading the string. | src/protocol_artifact/validate.rs:277; src/protocol_artifact/validate.rs:399-433; src/protocol_artifact/intake.rs:419-470; src/linking/composed/definition_source.rs:22-51 |
| FND-004 | medium | `*origin.role() == Role::new("expression")` is the same post-edge string dispatch, reworded so the literal-only scanner cannot see it. `Role` is an open `String` newtype ("The kernel does not enumerate roles", quire-exact/src/location.rs:36-39), not a closed enum or registry-resolved identity. The comparison still selects which occurrences key claims. It also allocates a `String` per call. Either convert role to a closed enum at the edge that attaches it, or mark the function honestly. | qsl-semantics/src/check/claims.rs:620-622 |
| FND-005 | low | `SymbolName::new("self").ok()` uses the same newtype pattern. This one is a declared-name lookup, which is legitimate. But `.ok()` swallows a construction error into "no `self` value", which then refuses the formal binding with no cause naming the real fault. Propagate the error instead of discarding it. | src/linking.rs:620-625 |
| FND-006 | medium | `source_files` skips every file named `tests.rs` or `*_tests.rs` by name, whether or not it is `#[cfg(test)]`. The new fixture asserts this for a plain `mod plain_tests;` with no `cfg(test)`, so a production module named `*_tests.rs` is exempt from the gate. FR-064 Inputs scopes out only `tests/` directories and `#[cfg(test)]` modules. `cfg_test_module_paths` already covers every real test file measured (all 15 are `#[cfg(test)] mod x;`), so the name rule adds only the hole. Drop it and make the fixture expect `plain_tests.rs` scanned. | xtask/src/string_edge.rs:423-428; xtask/src/string_edge.rs:1170-1197 |
| FND-007 | low | Some whole-function marks cover much more than the one edge compare. `mapped::compile` marks a public compile entry point, including its whole pipeline closure, for one `language != "ix:native"` check. `OperationSelection::new` is in the handoff writer, an emitter that is not on ADR-012 §9's edge list. `validate::features` gates on an internal `&'static str` set derived from the `Body` enum (`families.contains("family.protocol")`), which is internal string dispatch, not wire intake. Any future compare added to these bodies is silenced too. Extract the compare into a small marked conversion function, or match on `Body` directly. | src/mapped.rs:164-176; src/protocol_artifact/handoff/writer.rs:861-872; src/protocol_artifact/validate.rs:1433-1462 |
| FND-008 | low | The module doc now misstates the scanner's coverage: "ADR-010 §4.3's own named violation sites ... are all literal-based, so this scope covers the real violations". The `clock:` site is now const-based (FND-002). | xtask/src/string_edge.rs:21-25 |

## Rust review

- Panic surface: new `expect`/`panic!` calls are test-only.
  `cfg_test_module_paths` propagates I/O and parse errors through the crate
  `Error`.
- `cfg_test_module_paths` reads and parses every file a second time (the scan
  parses it again). This is correct but double work. Not a finding.
- It walks only top-level `parsed.items`, so a `#[cfg(test)] mod x;` nested in
  an inline module is not excluded. That causes over-reporting, not hiding.
  Not a finding.
- `has_cfg_test` does not match `cfg(any(test, ...))` or `cfg(not(test))`,
  which is the conservative direction.
- Layering: `qsl-attrs` has no dependencies and is a proc-macro, so it adds no
  edge to the check core.
- Gates run by the reviewer: `cargo xtask string-edge`,
  `cargo test -p xtask --lib string_edge`, and
  `cargo test --test it family_outcome_layering`. The full gates were not
  re-run.

## Dispositions

Disposition pass.

| FND | Outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | valid_digest/valid_adapter deleted; CanonicalizationDomain (FromStr) + ByteDigest-typed CanonicalDigest, AdapterArtifact via TryFrom, each the one marked conversion. ByteDigest::from_hex keeps the old 64-lowercase-hex rule. |
| FND-002 | fixed | clock: read once per admission into ClockNames (v1/v2 intake, temporal_v2 admit; v3 has no temporal evaluator); the three runtime re-parses are gone; clock_binding_name compares the literal and is back in the real-site test. The ADR-012 typed clock-role variant stays unbuilt and is now stated as partial in ADR-012 (SR-722 FND-003 asks for an owner). |
| FND-003 | fixed | Profile::classify deleted; definitions() returns Vec<RegisteredDefinition> (native path uses metadata's registered list) and Graph::profile matches it. Same identity sets per Family arm. |
| FND-004 | fixed | Closed OccurrenceRole enum (alphabetical order preserved, same kernel spellings); key_claims uses iter_role(Expression); the one spelling read is the marked OccurrenceRole::of. |
| FND-005 | fixed | SymbolName::new("self") error now propagated as an InvalidModelBinding failure. |
| FND-006 | fixed | Filename skip removed; only tests/ dirs and #[cfg(test)] mod x; files skipped. Reviewer mutation: a plain `mod plain_tests;` file with a string compare is reported by cargo xtask string-edge. |
| FND-007 | fixed | mapped::compile, OperationSelection::new and features are unmarked; features uses closed FamilyFeature/RequiredFeature with marked from_wire. Reviewer mutation: a new compare in features and in mapped::compile is reported. is_workflow_apply is a narrow marked predicate (SR-722 FND-002). |
| FND-008 | fixed | Module doc now states the literal-only scope and that named-constant dispatch is not detected. |
